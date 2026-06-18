use super::super::*;

impl<'a> Builder<'a> {
    /// An ELEMENT-FREE effect under a loop is keyed by WHAT is iterated: wrap it
    /// with the loop's canonical element source (`Elem(iterable)` — already
    /// canonical across loop shapes, so `while i < len(xs)` and `for x in xs`
    /// still converge; a bare `while cond` keys by its condition). Without this,
    /// `for k in obj.keys(): log(MSG)` and `for v of arr: log(MSG)` fingerprint
    /// identically — same effect, different iteration — the #210 for-in/for-of
    /// false merge. Element-REFERENCING effects already differ via `Elem`, and
    /// builder contributions never reach raw effect sinks, so the celebrated
    /// builder ↔ comprehension convergences are untouched.
    fn guard_element_free_loop_effects(
        &mut self,
        kind: LoopKind,
        kids: &[NodeId],
        pattern_bindings: &[(u32, ValueId)],
        env: &mut FxHashMap<u32, ValueId>,
        sink_start: usize,
    ) {
        // (`env` is the loop's OUTER frame, untouched by the body's env clone, so
        // evaluating the guard here matches the loop-entry view.)
        let guard = pattern_bindings.first().map(|&(_, v)| v).or_else(|| {
            (kind != LoopKind::ForEach && kids.len() >= 2).then(|| self.eval(kids[0], env))
        });
        let Some(guard) = guard else { return };
        let bot = self.sentinel_const(sentinel::BOTTOM);
        for i in sink_start..self.sinks.len() {
            let sink = self.sinks[i];
            if !matches!(sink.kind, SinkKind::Effect) || self.refs_elem(sink.value) {
                continue;
            }
            self.sinks[i].value = self.mk(ValOp::Phi, vec![guard, sink.value, bot]);
        }
    }

    pub(in crate::value_graph) fn process_loop(
        &mut self,
        stmt: NodeId,
        env: &mut FxHashMap<u32, ValueId>,
    ) {
        // Bracket the whole loop processing in a depth counter so an inline return
        // capture can tell a return executed *inside* a callee loop (poison) from one
        // merely written after it (fine).
        // A loop is an effect/aliasing barrier for indexed read-forwarding (#337): a forward
        // valid before the loop is invalid inside it (the array may be rewritten each
        // iteration), and a forward installed inside is invalid after. Clear on both edges.
        self.index_env.clear();
        self.loop_depth += 1;
        self.process_loop_inner(stmt, env);
        self.loop_depth -= 1;
        self.index_env.clear();
    }

    fn process_loop_inner(&mut self, stmt: NodeId, env: &mut FxHashMap<u32, ValueId>) {
        let kids = self.il.children(stmt).to_vec();
        let kind = match self.il.node(stmt).payload {
            Payload::Loop(k) => k,
            _ => LoopKind::While,
        };
        let body = match kids.last() {
            Some(&b) => b,
            None => return,
        };
        if kind == LoopKind::While
            && kids.len() == 2
            && self.loop_entry_condition_is_proven_false(kids[0], env)
        {
            return;
        }

        // Discover the loop's *element source* so per-element computations converge
        // across loop shapes (the §AH representation axis):
        //   • `for pat in iterable`        → element is the pattern variable;
        //   • `while i < len(xs) { … i+=1 }` → element is `xs[i]` for the induction
        //     variable `i` (index bookkeeping is iteration mechanics, not a result).
        // Both bind a single canonical `Elem(iterable)` value; an indexed `while`
        // additionally rewrites `xs[i]` → `Elem(xs)` and drops the induction variable.
        // Bindings applied to the loop's pattern/index variables (cid → value), and the
        // set of values that play the role of an *index*. Any `C[idx]` for such an
        // `idx` is the element of `C`, so indexed iteration converges with value
        // iteration. Iteration variables are not accumulators.
        let mut pattern_bindings: Vec<(u32, ValueId)> = Vec::new();
        let mut index_vals: FxHashSet<ValueId> = FxHashSet::default();
        let mut induction: FxHashSet<u32> = FxHashSet::default();

        match kind {
            LoopKind::ForEach if kids.len() >= 3 => {
                self.bind_foreach_loop_elements(&kids, env, &mut pattern_bindings, &mut index_vals);
            }
            _ => {
                self.bind_conditional_loop_elements(
                    &kids,
                    body,
                    env,
                    &mut pattern_bindings,
                    &mut index_vals,
                    &mut induction,
                );
            }
        }

        let (builder_cands, carried) = self.activate_loop_builders(body, env);

        // Seed each loop-carried variable with a symbolic "previous iteration" value
        // so the body expresses its update as a *recurrence* over `Loop(cid)`.
        let mut body_env = env.clone();
        let (loop_vals, loop_keys, loop_key_set) = self.seed_loop_carried(&carried, &mut body_env);
        // Pattern/index bindings override the `Loop` placeholder for iteration vars.
        let iter_vars: FxHashSet<u32> = pattern_bindings.iter().map(|&(c, _)| c).collect();
        for &(cid, v) in &pattern_bindings {
            body_env.insert(cid, v);
        }

        // Every `C[idx]` for an index value `idx` is morally `Elem(C)`. Rewrite it
        // everywhere the body deposits values — the sinks it pushes (guard conditions,
        // effects) and the carried recurrences — so indexed iteration (`while i<len`,
        // `for i in range(len)`, multi-collection `a[i]*b[i]`) matches value iteration,
        // even when the accumulation is conditional (filter+reduce) not a clean fold.
        // (List-builder vars were activated above, before `carried`, so a Go functional-append
        // builder is excluded from numeric recurrence seeding.)
        let sink_start = self.sinks.len();
        let outer_recurrence = self.loop_recurrence.replace(LoopRecurrenceScope {
            loop_values: loop_vals.clone(),
            loop_keys,
            loop_key_set,
        });
        let pre_body_env = body_env.clone();
        self.process_stmt(body, &mut body_env);
        self.guard_element_free_loop_effects(kind, &kids, &pattern_bindings, env, sink_start);
        self.loop_recurrence = outer_recurrence;
        self.rewrite_loop_index_sinks(sink_start, &index_vals, &carried, &mut body_env);
        let flag_break_reduction = self.detect_flag_break_reduction(
            body,
            &carried,
            env,
            &pre_body_env,
            &index_vals,
            sink_start,
        );
        self.finalize_loop_builders(
            &builder_cands,
            &index_vals,
            &body_env,
            &pattern_bindings,
            env,
        );

        // For each loop-carried accumulator, recognize an associative-commutative
        // reduction `acc = acc ⊕ contrib` and canonicalize it to a `Reduce` value —
        // so sum/product/min/max/count loops converge regardless of loop shape,
        // accumulator name, or operand grouping (the §AH representation axis). The
        // per-element `contrib` keys the value, so a `+`-loop and a `*`-loop (or
        // `a[i]*b[i]` vs `a[i]+b[i]`) stay distinct (the behavior axis). When the
        // update is not a clean reduction, thread the raw recurrence (still better
        // than a bare opaque `Loop` value reaching the sinks).
        let mut reduction_cache = ReductionCache::default();
        for &cid in &carried {
            let init = env.get(&cid).copied();
            if let Some(v) = self.carried_special_value(
                body,
                cid,
                flag_break_reduction,
                init,
                &pre_body_env,
                &index_vals,
            ) {
                env.insert(cid, v);
                continue;
            }
            if iter_vars.contains(&cid) || induction.contains(&cid) {
                continue; // iteration mechanics, not an accumulator
            }
            let Some(&newv) = body_env.get(&cid) else {
                continue;
            };
            let loopv = loop_vals[&cid];
            let v = self.carried_recurrence_value(
                init,
                newv,
                loopv,
                &pattern_bindings,
                &mut reduction_cache,
            );
            env.insert(cid, v);
        }
    }

    fn activate_loop_builders(
        &mut self,
        body: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> (Vec<BuilderCandidate>, Vec<u32>) {
        let mut assigned = FxHashSet::default();
        collect_assigned(self.il, body, &mut assigned);
        // List-builder vars (incl. Go's `r = append(r, …)`, which makes `r` assigned) are
        // activated as builders, NOT seeded as numeric loop-carried recurrences — otherwise a
        // Go builder var would be both a `Loop` placeholder and a `Map` build and collapse. The
        // seed (empty-collection) check reads the PRE-loop `env` (the real `[]` seed; the body's
        // reassignment would otherwise hide it). Builders are excluded from `carried`.
        let builder_cands = self.builder_candidates(body, env);
        let builder_set: FxHashSet<u32> = builder_cands
            .iter()
            .map(|candidate| candidate.cid)
            .collect();
        for candidate in &builder_cands {
            self.building.insert(candidate.cid, None);
            self.building_kind.insert(candidate.cid, candidate.kind);
        }
        let mut carried: Vec<u32> = assigned
            .iter()
            .copied()
            .filter(|c| !builder_set.contains(c))
            .collect();
        carried.sort_unstable();
        (builder_cands, carried)
    }

    fn seed_loop_carried(
        &mut self,
        carried: &[u32],
        body_env: &mut FxHashMap<u32, ValueId>,
    ) -> (FxHashMap<u32, ValueId>, FxHashMap<u32, u32>, FxHashSet<u32>) {
        let mut loop_vals: FxHashMap<u32, ValueId> = FxHashMap::default();
        let loop_key_base = self.next_loop_key_base;
        let loop_key_count = u32::try_from(carried.len()).unwrap_or(u32::MAX);
        self.next_loop_key_base = self
            .next_loop_key_base
            .wrapping_add(loop_key_count.saturating_add(1));
        let mut loop_keys: FxHashMap<u32, u32> = FxHashMap::default();
        let mut loop_key_set: FxHashSet<u32> = FxHashSet::default();
        for (slot, &cid) in carried.iter().enumerate() {
            let key = loop_key_base.wrapping_add(slot as u32);
            loop_keys.insert(cid, key);
            loop_key_set.insert(key);
            let lv = self.mk(ValOp::Loop(key), vec![]);
            loop_vals.insert(cid, lv);
            body_env.insert(cid, lv);
        }
        (loop_vals, loop_keys, loop_key_set)
    }

    fn bind_foreach_loop_elements(
        &mut self,
        kids: &[NodeId],
        env: &FxHashMap<u32, ValueId>,
        pattern_bindings: &mut Vec<(u32, ValueId)>,
        index_vals: &mut FxHashSet<ValueId>,
    ) {
        let pat = kids[0];
        let it = kids[1];
        if let Some(c) = self.range_len_collection(it) {
            // `for i in range(len(C))`: `i` is a canonical index into `C`.
            let cv = self.eval(c, env);
            let ix = self.idx(cv);
            index_vals.insert(ix);
            for cid in self.pattern_cids(pat) {
                pattern_bindings.push((cid, ix));
            }
        } else if self.il.kind(it) == NodeKind::Call
            && matches!(
                self.il.node(it).payload,
                Payload::Builtin(Builtin::Enumerate)
            )
            && self.admitted_builtin_call(it, Builtin::Enumerate)
        {
            // `for i, x in enumerate(C)`: `i` is the index, `x` the element.
            let cids = self.pattern_cids(pat);
            if let Some(&cnode) = self.il.children(it).first() {
                let cv = self.eval(cnode, env);
                let ix = self.idx(cv);
                let el = self.elem(cv);
                index_vals.insert(ix);
                if cids.len() >= 2 {
                    pattern_bindings.push((cids[0], ix));
                    pattern_bindings.push((cids[1], el));
                } else if let Some(&only) = cids.first() {
                    pattern_bindings.push((only, el));
                }
            }
        } else {
            // Value iteration `for x in C` (or `for i in range(n)`): the
            // pattern var is an element of the iterable. NOTE: we do *not* treat
            // this element as a collection index — only a provably-*full* range
            // (`range_len_collection`, above) licenses `C[i] → Elem(C)`. A bare
            // `range(n)` or partial `range(1, len)` indexing `C[i]` must keep its
            // `Index(C, …)` so it stays distinct from the full-range loop (else a
            // subset sum merges with the full sum — a soundness bug).
            let iv = self.eval(it, env);
            let e = self.elem(iv);
            for cid in self.pattern_cids(pat) {
                pattern_bindings.push((cid, e));
            }
        }
    }

    fn bind_conditional_loop_elements(
        &mut self,
        kids: &[NodeId],
        body: NodeId,
        env: &FxHashMap<u32, ValueId>,
        pattern_bindings: &mut Vec<(u32, ValueId)>,
        index_vals: &mut FxHashSet<ValueId>,
        induction: &mut FxHashSet<u32>,
    ) {
        let cond = kids
            .first()
            .filter(|&&c| self.il.kind(c) != NodeKind::Block);
        // A genuine loop counter both steps by a constant *and* governs the
        // loop condition. An accumulator updated by `acc = acc + 1` (a counting
        // reduction) matches the `i = i ± c` shape too, so `induction_vars`
        // alone misclassifies it as iteration mechanics — which binds it to the
        // index and destroys the reduction (it would never reach a `Reduce`).
        // Intersect with the variables the condition actually mentions so only
        // the real counter(s) are treated as indices; the accumulator stays an
        // accumulator. (Sum loops were spared by luck — `sum += xs[i]` has a
        // non-literal operand, so `is_increment` already rejected them; counting
        // loops like `if p: count += 1` were the ones that collapsed.)
        let cond_cids = cond
            .map(|&c| mentioned_cids(self.il, c))
            .unwrap_or_default();
        *induction = induction_vars(self.il, body)
            .intersection(&cond_cids)
            .copied()
            .collect();
        let iter_node = cond.and_then(|&c| self.loop_iterable(c, &*induction).map(|it| (it, None)));
        let indexed_bound_loop = iter_node.or_else(|| {
            cond.and_then(|&c| {
                self.indexed_bound_loop_iterable(c, body, &*induction)
                    .map(|(it, bound, cmp)| (it, Some((bound, cmp))))
            })
        });
        match indexed_bound_loop {
            // Indexed `while i < len(C)`: the raw `i < len(C)` guard is iteration
            // mechanics. A counter that steps by +1 from 0 visits *every* index
            // in order, so `C[i]` is the canonical `Elem(C)` (converges with `for
            // x in C`). A non-unit stride (`i += 2`) or non-zero start (`i = 1`)
            // visits a SUBSET — `C[i]` is NOT `Elem(C)` — so bind `i` to a strided
            // index that encodes start+step (distinct strides stay distinct) and
            // do NOT license the `C[i] → Elem(C)` rewrite (else a strided sum
            // merges with the full sum — the while-loop analog of the range bug).
            Some((it, bound_guard)) if !induction.is_empty() => {
                let cv = self.eval(it, env);
                if let Some((bound, cmp)) = bound_guard {
                    let bv = self.eval(bound, env);
                    if !self.full_pointer_length_contract(cmp, cv, bv) {
                        let gv = self.indexed_bound_guard(cmp, bv);
                        self.sinks.push(Sink::new(SinkKind::Cond, gv));
                    } else if let (Some(arr), Some(len)) = (self.input_key(cv), self.input_key(bv))
                    {
                        // The bound `n` was dropped as "length of the array" — record
                        // (array_pos, length_pos) so the oracle interprets under n=len.
                        self.contracts.push((arr, len));
                    }
                }
                let zero = self.int_const(0);
                for &i in &*induction {
                    let step = induction_step(self.il, body, i);
                    let start_zero = env.get(&i).is_some_and(|&s| s == zero);
                    if step == Some(1) && start_zero {
                        let ix = self.idx(cv);
                        index_vals.insert(ix);
                        pattern_bindings.push((i, ix));
                    } else {
                        let start_val = env.get(&i).copied().unwrap_or(zero);
                        let step_val = self.int_const(step.unwrap_or(0).rem_euclid(1 << 24) as u32);
                        let base = self.idx(cv);
                        let h = combine(
                            self.vhash[base as usize],
                            combine(
                                self.vhash[start_val as usize],
                                self.vhash[step_val as usize],
                            ),
                        );
                        let strided = self.mk(ValOp::Idx(h), vec![base, start_val, step_val]);
                        pattern_bindings.push((i, strided));
                    }
                }
            }
            // Plain `while`/other: keep the raw condition; no element model.
            _ => {
                induction.clear();
                if let Some(&c) = cond {
                    let cv = self.eval(c, env);
                    self.sinks.push(Sink::new(SinkKind::Cond, cv));
                }
            }
        }
    }

    fn rewrite_loop_index_sinks(
        &mut self,
        sink_start: usize,
        index_vals: &FxHashSet<ValueId>,
        carried: &[u32],
        body_env: &mut FxHashMap<u32, ValueId>,
    ) {
        if !index_vals.is_empty() {
            let mut memo = FxHashMap::default();
            for idx in sink_start..self.sinks.len() {
                let v = self.sinks[idx].value;
                self.sinks[idx].value = self.rewrite_indices(v, index_vals, &mut memo);
            }
            for &cid in carried {
                if let Some(&v) = body_env.get(&cid) {
                    let nv = self.rewrite_indices(v, index_vals, &mut memo);
                    body_env.insert(cid, nv);
                }
            }
        }
    }

    fn detect_flag_break_reduction(
        &mut self,
        body: NodeId,
        carried: &[u32],
        env: &FxHashMap<u32, ValueId>,
        pre_body_env: &FxHashMap<u32, ValueId>,
        index_vals: &FxHashSet<ValueId>,
        sink_start: usize,
    ) -> Option<(u32, ValueId)> {
        let mut flag_break_reduction = None;
        for &cid in carried {
            if let Some(&init) = env.get(&cid) {
                if let Some(v) =
                    self.flag_break_reduction(body, cid, init, pre_body_env, index_vals)
                {
                    flag_break_reduction = Some((cid, v));
                    self.sinks.truncate(sink_start);
                    break;
                }
            }
        }
        flag_break_reduction
    }

    fn finalize_loop_builders(
        &mut self,
        builder_cands: &[BuilderCandidate],
        index_vals: &FxHashSet<ValueId>,
        body_env: &FxHashMap<u32, ValueId>,
        pattern_bindings: &[(u32, ValueId)],
        env: &mut FxHashMap<u32, ValueId>,
    ) {
        // Finalize list builders: `r = []; for x: r.append(f(x))` → `r = Map(elem, f(x))`,
        // converging the loop with the comprehension `[f(x) for x in xs]` / `.map`. A
        // guarded append (`if cond: r.append(f(x))`) becomes the filtered map `Map(_, pred)`.
        // If the append happens inside a nested loop, the inner loop has already produced a
        // `Map`/`FlatMap`; wrap that per-outer-iteration collection in a `FlatMap` so
        // `for x: for y: r.append(f(x,y))` converges with `[f(x,y) for x in xs for y in ys]`.
        for candidate in builder_cands {
            let c = candidate.cid;
            if let Some(Some((mut contrib, guard))) = self.building.remove(&c) {
                let map = if !index_vals.is_empty() {
                    let mut memo = FxHashMap::default();
                    contrib = self.rewrite_indices(contrib, index_vals, &mut memo);
                    match guard {
                        Some(g) => {
                            let g = self.rewrite_indices(g, index_vals, &mut memo);
                            self.mk(ValOp::Hof(HoFKind::Map as u32), vec![contrib, g])
                        }
                        None => self.mk(ValOp::Hof(HoFKind::Map as u32), vec![contrib]),
                    }
                } else {
                    match guard {
                        Some(g) => self.mk(ValOp::Hof(HoFKind::Map as u32), vec![contrib, g]),
                        None => self.mk(ValOp::Hof(HoFKind::Map as u32), vec![contrib]),
                    }
                };
                env.insert(c, map);
            } else if let Some(&nested) = body_env.get(&c) {
                if let Some(flat) = self.flat_map_builder_value(nested, pattern_bindings) {
                    env.insert(c, flat);
                }
                self.building.remove(&c);
            } else {
                self.building.remove(&c);
            }
            self.building_kind.remove(&c);
        }
    }

    fn carried_special_value(
        &mut self,
        body: NodeId,
        cid: u32,
        flag_break_reduction: Option<(u32, ValueId)>,
        init: Option<ValueId>,
        pre_body_env: &FxHashMap<u32, ValueId>,
        index_vals: &FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if let Some((flag_cid, v)) = flag_break_reduction {
            if flag_cid == cid {
                return Some(v);
            }
        }
        if let Some(init) = init {
            if let Some(v) =
                self.ordered_string_concat_loop(body, cid, init, pre_body_env, index_vals)
            {
                return Some(v);
            }
        }
        None
    }

    fn carried_recurrence_value(
        &mut self,
        init: Option<ValueId>,
        newv: ValueId,
        loopv: ValueId,
        pattern_bindings: &[(u32, ValueId)],
        reduction_cache: &mut ReductionCache,
    ) -> ValueId {
        let loop_context: Vec<ValueId> = pattern_bindings.iter().map(|&(_, v)| v).collect();
        match (
            init,
            self.as_loop_reduction_step(newv, loopv, &loop_context, reduction_cache),
        ) {
            (Some(init), Some((op, contrib))) => {
                // Every loop reduction carries its seed. For folds the seed is
                // the init operand; for selections (min/max) the seed CLAMPS the
                // result — `best = 0; …max…` returns 0 on all-negative input,
                // which true `max(…)` does not — so it is behavior-defining and
                // must reach the fingerprint. The builtins build a seedless
                // 1-arg `Reduce`, so they can never merge with a seeded loop.
                self.mk(ValOp::Reduce(op), vec![init, contrib])
            }
            (init, _) => {
                // A non-reduction loop-carried value still depends on its pre-loop
                // SEED. The compact `Recurrence` key is the per-iteration update
                // expression ONLY, so `acc = a` (a parameter seed, the loop returning
                // `a + Σ`) collapsed onto `acc = 0` (returning `Σ`) — a false merge,
                // since the two differ exactly by that seed. Re-key the recurrence on
                // the seed as well so the seed reaches the fingerprint. (Clean
                // reductions above already carry their `init`.)
                match (init, self.nodes[newv as usize].op.clone()) {
                    (Some(init), ValOp::Recurrence(h)) => self.mk(ValOp::Recurrence(h), vec![init]),
                    _ => newv,
                }
            }
        }
    }
}

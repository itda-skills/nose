use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn eval(
        &mut self,
        expr: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        // Track the enclosing source expression so EVERY node created while evaluating it (the top
        // node AND the intermediate nodes a reduce/map unfolds via `mk`) is stamped with its span
        // at creation — those intermediates are exactly what a heavy sub-DAG anchor points at.
        let prev = self.cur_span;
        self.cur_span = Some(self.il.node(expr).span);
        // Mirror `cur_span` for the opaque census (#391 probe): the save/restore makes the
        // current kind the node whose handler mints an `Opaque`, not a just-evaluated child.
        let prev_kind = self.cur_il_kind;
        self.cur_il_kind = Some(self.il.node(expr).kind);
        let v = self.eval_inner(expr, env);
        self.cur_span = prev;
        self.cur_il_kind = prev_kind;
        v
    }
    /// Whether a `Raw` node is the `await` protocol boundary (the one `Raw` we model as a
    /// value-preserving wrapper, see the `NodeKind::Raw` arm in `eval_inner`).
    fn is_await_raw(&self, payload: &Payload) -> bool {
        matches!(payload, Payload::Name(s) if self.interner.resolve(*s) == "await")
    }
    fn eval_inner(&mut self, expr: NodeId, env: &FxHashMap<u32, ValueId>) -> ValueId {
        let node = *self.il.node(expr);
        match node.kind {
            NodeKind::Var => self.eval_var_expr(expr, node.payload, env),
            NodeKind::Lit => self.eval_lit_expr(node.payload),
            NodeKind::BinOp => self.eval_binop_expr(expr, node.payload, env),
            NodeKind::UnOp => {
                let kids = self.il.children(expr).to_vec();
                let mut a: Vec<ValueId> = kids.iter().map(|&k| self.eval(k, env)).collect();
                let op = op_code(node.payload);
                // JS `~x` is `~ToInt32(x)`, an int32 — narrow the operand so it fingerprints
                // distinctly from arbitrary-precision `~` (#283-D).
                if op == Op::BitNot as u32 {
                    for v in a.iter_mut() {
                        *v = self.js_int32_narrow(*v);
                    }
                }
                self.mk(ValOp::Un(op), a)
            }
            NodeKind::Field => self.eval_field_expr(expr, node.payload, env),
            NodeKind::Index => self.eval_index_expr(expr, env),
            NodeKind::Call => self.eval_call_expr(expr, node.payload, env),
            NodeKind::HoF => self.eval_hof_expr(expr, node.payload, env),
            NodeKind::Seq => self.eval_seq_expr(expr, node.payload, env),
            NodeKind::If => self.eval_if_expr(expr, env),
            NodeKind::Lambda => {
                let hash = self.valued_subtree_hash(expr);
                self.mk(ValOp::Lambda(hash), vec![])
            }
            // A keyword call argument `name=value` evaluated outside a call's own
            // kwarg handling (e.g. a kwarg that survived into an opaque position): key
            // it by the keyword name so `a=p` ≠ `b=p` and ≠ positional `p`.
            NodeKind::KwArg => {
                let name_hash = match node.payload {
                    Payload::Name(s) => self.interner.symbol_hash(s),
                    _ => 0,
                };
                let value = self
                    .il
                    .children(expr)
                    .first()
                    .map(|&v| self.eval(v, env))
                    .unwrap_or_else(|| self.mk(ValOp::Opaque(0), vec![]));
                self.mk(ValOp::KwArg(name_hash), vec![value])
            }
            // `await e` is the ONE `Raw` we key as a wrapper that KEEPS its operand as a
            // child — `Opaque(VG_PROTOCOL_AWAIT, [eval(e)])` — so the near/graded witness can
            // align an async fn with its sync twin through the wrapper (the `async-mirror`
            // pattern). The wrapper still makes `await e` ≠ `e` and ≠ `await f`, so the EXACT
            // channel never merges a Future with its resolved value — and async units are
            // non-`exact_safe` anyway (a `Raw` in the IL ⇒ `strict_exact` returns false), the
            // load-bearing guard. Every OTHER `Raw` stays on the childless arm below.
            NodeKind::Raw if self.is_await_raw(&node.payload) => {
                let operand = self
                    .il
                    .children(expr)
                    .first()
                    .map(|&v| self.eval(v, env))
                    .unwrap_or_else(|| self.mk(ValOp::Opaque(VG_PROTOCOL_AWAIT), vec![]));
                if self.await_transparent {
                    // Fingerprint build: `await e` ≡ `e`'s value, so the operand identity flows
                    // downstream and an async fn's fingerprint matches its sync twin (vj↑).
                    operand
                } else {
                    // Witness build: keep the wrapper so `value_dag`'s graded anti-unification can
                    // see the await and label the difference `async-mirror` (a transformation, not
                    // a behavioral equivalence).
                    self.mk(ValOp::Opaque(VG_PROTOCOL_AWAIT), vec![operand])
                }
            }
            // Any other unlowered / unhandled construct — notably `Raw`, which wraps a
            // macro, C compound literal, `#ifdef`, parse-ERROR, etc. Key it by its full
            // subtree hash (surface kind + lowered children), exactly like `Lambda`, so
            // behaviorally-different unlowered constructs produce DIFFERENT fingerprints.
            // A positional opaque counter collapsed them (e.g. two distinct C compound
            // literals → one fingerprint = an unsound false merge the interpreter oracle
            // can't catch, since `Raw` is uninterpretable). Identical constructs converge.
            _ => {
                let hash = self.subtree_hash(expr);
                self.mk(ValOp::Opaque(hash), vec![])
            }
        }
    }
}

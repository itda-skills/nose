use super::*;

/// The synthesized transparent-identifier parameter name. `<>` cannot occur in
/// a C# identifier (the compiler uses the same trick for its own generated
/// names), so user code can never reference — or be captured by — it; the
/// value graph canonicalizes lambda-parameter names, so the choice does not
/// affect convergence with hand-written method chains.
const TID: &str = "<>t";

/// What "the current element" of the desugared chain is, between clauses: a
/// plain range variable, or a transparent identifier — an anonymous object
/// threading every still-visible range variable through the chain, each
/// reachable by a member-projection path (the C# spec's translation device for
/// `let`, `join`, and a second `from`).
enum Binding {
    Simple(String),
    Transparent(Vec<(String, Vec<Symbol>)>),
}

impl Binding {
    fn names(&self) -> Vec<&str> {
        match self {
            Binding::Simple(v) => vec![v.as_str()],
            Binding::Transparent(env) => env.iter().map(|(n, _)| n.as_str()).collect(),
        }
    }

    fn binds(&self, name: &str) -> bool {
        self.names().contains(&name)
    }

    fn param_name(&self) -> &str {
        match self {
            Binding::Simple(v) => v,
            Binding::Transparent(_) => TID,
        }
    }
}

/// LINQ query syntax → the method-syntax chain the C# spec prescribes
/// (`from x in xs where p select e` ≡ `xs.Where(x => p).Select(x => e)`), so
/// the two spellings converge on one IL shape. Clauses that introduce
/// transparent identifiers (`let`, `join`, a second `from`) desugar with the
/// spec's anonymous-object translation: the pair object uses the same
/// `object`/`pair` shape as a hand-written `new { x, y = e }`, and later
/// clause bodies have each still-visible range variable rewritten to its
/// member projection. Any shape the rewrite cannot prove (a shadowing lambda
/// parameter, an assignment to a range variable, a join source or `equals`
/// right key that mentions an outer range variable) returns `None` so the
/// whole query stays fail-closed `Raw` rather than desugar unsoundly.
pub(super) fn lower_query(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let clauses = named(node);
    let (from, rest) = clauses.split_first()?;
    if from.kind() != "from_clause" {
        return None;
    }
    let mut binding = Binding::Simple(lo.text(from.child_by_field_name("name")?).to_string());
    let source = *named(*from).last()?;
    let mut acc = lower_expr(lo, source);
    let mut operations = 0usize;
    let mut idx = 0usize;
    while idx < rest.len() {
        let clause = rest[idx];
        match clause.kind() {
            "where_clause" => {
                let pred = clause.named_child(0)?;
                let lambda = binding_lambda(lo, span, &binding, pred)?;
                let call_span = call_span(lo, clause);
                acc = method_call(lo, call_span, acc, "Where", &[lambda]);
                operations += 1;
            }
            "order_by_clause" => {
                acc = lower_orderings(lo, span, acc, &binding, clause)?;
                operations += 1;
            }
            "group_clause" => {
                // `group e by k` → `.GroupBy(x => k)` (identity element) or
                // `.GroupBy(x => k, x => e)`.
                let kids = named(clause);
                let [elem, key] = kids[..] else {
                    return None;
                };
                let key_lambda = binding_lambda(lo, span, &binding, key)?;
                let mut args = vec![key_lambda];
                if !is_identity(lo, elem, &binding) {
                    args.push(binding_lambda(lo, span, &binding, elem)?);
                }
                let call_span = call_span(lo, clause);
                acc = method_call(lo, call_span, acc, "GroupBy", &args);
                operations += 1;
            }
            "select_clause" => {
                // A final identity `select x` after another operation elides,
                // as in the spec's translation; a degenerate `from x in xs
                // select x` keeps its `.Select(x => x)`.
                let result = clause.named_child(0)?;
                if operations == 0 || !is_identity(lo, result, &binding) {
                    let lambda = binding_lambda(lo, span, &binding, result)?;
                    let call_span = call_span(lo, clause);
                    acc = method_call(lo, call_span, acc, "Select", &[lambda]);
                }
                operations += 1;
            }
            "let_clause" => {
                let kids = named(clause);
                let [name_node, value] = kids[..] else {
                    return None;
                };
                let clause_span = lo.span(clause);
                binding = wrap_let(
                    lo,
                    span,
                    clause_span,
                    &mut acc,
                    binding,
                    lo.text(name_node),
                    value,
                )?;
                operations += 1;
            }
            "from_clause" => {
                let name = lo.text(clause.child_by_field_name("name")?).to_string();
                let src = *named(clause).last()?;
                if binding.binds(&name) {
                    return None; // C# rejects redeclaring a range variable
                }
                let source_lambda = binding_lambda(lo, span, &binding, src)?;
                let sm_span = call_span(lo, clause);
                if let Some(sel) = immediate_select(rest, idx) {
                    let result = two_param_result(lo, span, &binding, &name, sel)?;
                    acc = method_call(lo, sm_span, acc, "SelectMany", &[source_lambda, result]);
                    idx += 1; // the final select is consumed by this translation
                } else {
                    let clause_span = lo.span(clause);
                    let result = pair_result(lo, span, clause_span, &binding, &name);
                    acc = method_call(lo, sm_span, acc, "SelectMany", &[source_lambda, result]);
                    binding = Binding::Transparent(extended_env(lo, &binding, &name));
                }
                operations += 1;
            }
            "join_clause" => {
                let consumed = lower_join(lo, span, &mut acc, &mut binding, clause, rest, idx)?;
                idx += usize::from(consumed);
                operations += 1;
            }
            // `into g` — the query so far becomes the source of a fresh query
            // over `g` (the spec's `from g in ( … ) …` continuation). A
            // trailing translation-introduced identity `select g` then elides
            // through the `operations > 0` rule above, matching the compiler.
            "identifier" => {
                binding = Binding::Simple(lo.text(clause).to_string());
            }
            _ => return None,
        }
        idx += 1;
    }
    Some(acc)
}

/// `join b in src on k1 equals k2 [into g]` → `.Join(src, cur => k1, b => k2,
/// result)` / `.GroupJoin(…)`. Returns whether the translation consumed a
/// final `select` clause.
fn lower_join(
    lo: &mut Lowering,
    span: Span,
    acc: &mut NodeId,
    binding: &mut Binding,
    clause: TsNode,
    rest: &[TsNode],
    idx: usize,
) -> Option<bool> {
    let kids = named(clause);
    let (var_node, src, k1, k2, group) = match kids[..] {
        [v, s, a, b] => (v, s, a, b, None),
        [v, s, a, b, g] => (v, s, a, b, Some(g)),
        _ => return None,
    };
    let join_var = lo.text(var_node).to_string();
    let result_var = group.map_or_else(|| join_var.clone(), |g| lo.text(g).to_string());
    if binding.binds(&join_var) || binding.binds(&result_var) {
        return None;
    }
    // The join source and the `equals` right key are evaluated outside the
    // outer element's scope in the spec's translation (the source once, the
    // right key under its own one-parameter lambda); an outer range variable
    // there has no sound rewrite — C# rejects it, and malformed input stays Raw.
    if mentions(lo, src, &binding.names()) || mentions(lo, k2, &binding.names()) {
        return None;
    }
    let source_il = lower_expr(lo, src);
    let outer_key = binding_lambda(lo, span, binding, k1)?;
    let inner_key = range_lambda(lo, span, &join_var, k2);
    let method = if group.is_some() { "GroupJoin" } else { "Join" };
    let join_span = call_span(lo, clause);
    if let Some(sel) = immediate_select(rest, idx) {
        let result = two_param_result(lo, span, binding, &result_var, sel)?;
        *acc = method_call(
            lo,
            join_span,
            *acc,
            method,
            &[source_il, outer_key, inner_key, result],
        );
        Some(true)
    } else {
        let clause_span = lo.span(clause);
        let result = pair_result(lo, span, clause_span, binding, &result_var);
        *acc = method_call(
            lo,
            join_span,
            *acc,
            method,
            &[source_il, outer_key, inner_key, result],
        );
        *binding = Binding::Transparent(extended_env(lo, binding, &result_var));
        Some(false)
    }
}

/// `let name = value` → `.Select(cur => new { cur, name = value' })`, the
/// element becoming the pair that carries both.
fn wrap_let(
    lo: &mut Lowering,
    span: Span,
    clause_span: Span,
    acc: &mut NodeId,
    binding: Binding,
    name: &str,
    value: TsNode,
) -> Option<Binding> {
    if binding.binds(name) {
        return None; // C# rejects redeclaring a range variable
    }
    let value_il = clause_body(lo, &binding, value)?;
    let (carry_name, carry_value) = carry_member(lo, span, &binding);
    let obj = object_literal(
        lo,
        synthetic_span(clause_span, 0),
        &[
            (&carry_name, carry_value, synthetic_span(clause_span, 1)),
            (name, value_il, synthetic_span(clause_span, 2)),
        ],
    );
    let param = param(lo, span, binding.param_name());
    let lambda = lo.add(NodeKind::Lambda, Payload::None, span, &[param, obj]);
    *acc = method_call(
        lo,
        synthetic_span(clause_span, 3),
        *acc,
        "Select",
        &[lambda],
    );
    Some(Binding::Transparent(extended_env(lo, &binding, name)))
}

/// The environment after threading `binding` and a newly introduced variable
/// through a pair object: previously visible paths gain the carried member as
/// a prefix, and the new variable projects by its own name.
fn extended_env(lo: &Lowering, binding: &Binding, name: &str) -> Vec<(String, Vec<Symbol>)> {
    let mut env = Vec::new();
    match binding {
        Binding::Simple(v) => env.push((v.clone(), vec![lo.sym(v)])),
        Binding::Transparent(old) => {
            let tid = lo.sym(TID);
            for (k, path) in old {
                let mut p = Vec::with_capacity(path.len() + 1);
                p.push(tid);
                p.extend(path.iter().copied());
                env.push((k.clone(), p));
            }
        }
    }
    env.push((name.to_string(), vec![lo.sym(name)]));
    env
}

/// The pair-object member that carries the current element forward: the range
/// variable itself, or the whole previous transparent identifier.
fn carry_member(lo: &mut Lowering, span: Span, binding: &Binding) -> (String, NodeId) {
    let name = binding.param_name().to_string();
    let value = lo.var(&name, span);
    (name, value)
}

/// `(cur, name) => sel'` — the two-parameter result selector for a `from`/
/// `join` immediately followed by the final `select`.
fn two_param_result(
    lo: &mut Lowering,
    span: Span,
    binding: &Binding,
    name: &str,
    sel: TsNode,
) -> Option<NodeId> {
    let body = clause_body(lo, binding, sel)?;
    let p1 = param(lo, span, binding.param_name());
    let p2 = param(lo, span, name);
    Some(lo.add(NodeKind::Lambda, Payload::None, span, &[p1, p2, body]))
}

/// `(cur, name) => new { cur, name }` — the pair-building result selector for
/// a `from`/`join` with more clauses to come.
fn pair_result(
    lo: &mut Lowering,
    span: Span,
    clause_span: Span,
    binding: &Binding,
    name: &str,
) -> NodeId {
    let (carry_name, carry_value) = carry_member(lo, span, binding);
    let intro = lo.var(name, span);
    let obj = object_literal(
        lo,
        synthetic_span(clause_span, 0),
        &[
            (&carry_name, carry_value, synthetic_span(clause_span, 1)),
            (name, intro, synthetic_span(clause_span, 2)),
        ],
    );
    let p1 = param(lo, span, binding.param_name());
    let p2 = param(lo, span, name);
    lo.add(NodeKind::Lambda, Payload::None, span, &[p1, p2, obj])
}

/// A clause body under the current binding: lowered as-is over a plain range
/// variable, or lowered then rewritten so every visible range variable becomes
/// its member projection off the transparent identifier.
fn clause_body(lo: &mut Lowering, binding: &Binding, node: TsNode) -> Option<NodeId> {
    let raw = lower_expr(lo, node);
    match binding {
        Binding::Simple(_) => Some(raw),
        Binding::Transparent(env) => {
            let tid = lo.sym(TID);
            substitute(lo, raw, env, tid)
        }
    }
}

/// Rebuild a lowered subtree with every mapped range-variable read replaced by
/// its projection chain off the transparent identifier. Untouched subtrees are
/// shared, not copied; rebuilt nodes go through the raw builder (their spans
/// already carry the evidence the first lowering recorded). Returns `None` —
/// failing the whole query closed — on any shape whose binding structure the
/// rewrite cannot prove: a lambda parameter or an assignment target naming a
/// mapped variable (both invalid C# in a query body, so nothing real is lost).
fn substitute(
    lo: &mut Lowering,
    id: NodeId,
    env: &[(String, Vec<Symbol>)],
    tid: Symbol,
) -> Option<NodeId> {
    let kind = lo.b.kind(id);
    let payload = lo.b.payload(id);
    let node_span = lo.b.node(id).span;
    if kind == NodeKind::Var {
        if let Payload::Name(s) = payload {
            if let Some((_, path)) = env.iter().find(|(k, _)| lo.sym(k) == s) {
                let path = path.clone();
                let mut cur = lo.b.add(NodeKind::Var, Payload::Name(tid), node_span, &[]);
                for seg in path {
                    cur =
                        lo.b.add(NodeKind::Field, Payload::Name(seg), node_span, &[cur]);
                }
                return Some(cur);
            }
        }
        return Some(id);
    }
    if kind == NodeKind::Lambda {
        let shadows = lo.b.children(id).iter().any(|&c| {
            lo.b.kind(c) == NodeKind::Param
                && matches!(lo.b.payload(c), Payload::Name(p) if env.iter().any(|(k, _)| lo.sym(k) == p))
        });
        if shadows {
            return None;
        }
    }
    if kind == NodeKind::Assign {
        if let [target, _] = lo.b.children(id) {
            let rebinds = lo.b.kind(*target) == NodeKind::Var
                && matches!(lo.b.payload(*target), Payload::Name(t) if env.iter().any(|(k, _)| lo.sym(k) == t));
            if rebinds {
                return None;
            }
        }
    }
    let children = lo.b.children(id).to_vec();
    let mut rebuilt = Vec::with_capacity(children.len());
    let mut changed = false;
    for c in children {
        let n = substitute(lo, c, env, tid)?;
        changed |= n != c;
        rebuilt.push(n);
    }
    if !changed {
        return Some(id);
    }
    Some(lo.b.add(kind, payload, node_span, &rebuilt))
}

/// A zero-width span keyed off a clause's start byte. Synthesized `object`/
/// `pair` sequences need spans that collide neither with each other nor with
/// any real sequence's span (per-span evidence resolution treats mixed kinds
/// as ambiguous); no lowered node has a zero-width span, and clause start
/// bytes keep different clauses' synthesized spans apart.
/// The synthesized span for one desugared method call, keyed off the CST node
/// (clause or ordering key) that motivated it. Library-API call evidence is
/// anchored per call span, and two different contracts on one span reject each
/// other — so every synthesized call needs its own. Offset 3 stays clear of
/// the `object`/`pair` sequence offsets (0..=2) on the same clause.
fn call_span(lo: &Lowering, node: TsNode) -> Span {
    synthetic_span(lo.span(node), 3)
}

fn synthetic_span(base: Span, offset: u32) -> Span {
    let at = base.start_byte.saturating_add(offset);
    Span::new(base.file, at, at, base.start_line, base.start_line)
}

/// `orderby k1, k2 descending, …` → `.OrderBy[Descending](x => k1)
/// .ThenBy[Descending](x => k2)…`. Keys are the clause's named children; each
/// may be followed by an anonymous `ascending`/`descending` token.
fn lower_orderings(
    lo: &mut Lowering,
    span: Span,
    mut acc: NodeId,
    binding: &Binding,
    clause: TsNode,
) -> Option<NodeId> {
    let mut first = true;
    let mut pending: Option<TsNode> = None;
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if child.is_named() {
            if let Some(key) = pending.take() {
                acc = ordering_call(lo, span, acc, binding, key, first, false)?;
                first = false;
            }
            pending = Some(child);
        } else if matches!(child.kind(), "ascending" | "descending") {
            let key = pending.take()?;
            let descending = child.kind() == "descending";
            acc = ordering_call(lo, span, acc, binding, key, first, descending)?;
            first = false;
        }
    }
    if let Some(key) = pending {
        acc = ordering_call(lo, span, acc, binding, key, first, false)?;
    }
    Some(acc)
}

fn ordering_call(
    lo: &mut Lowering,
    span: Span,
    recv: NodeId,
    binding: &Binding,
    key: TsNode,
    first: bool,
    descending: bool,
) -> Option<NodeId> {
    let method = match (first, descending) {
        (true, false) => "OrderBy",
        (true, true) => "OrderByDescending",
        (false, false) => "ThenBy",
        (false, true) => "ThenByDescending",
    };
    let lambda = binding_lambda(lo, span, binding, key)?;
    Some(method_call(lo, call_span(lo, key), recv, method, &[lambda]))
}

/// `cur => <body'>` over the current binding — the shape `lower_lambda` gives
/// a source-level lambda, so the desugared chain converges with hand-written
/// method syntax.
fn binding_lambda(
    lo: &mut Lowering,
    span: Span,
    binding: &Binding,
    body: TsNode,
) -> Option<NodeId> {
    let body = clause_body(lo, binding, body)?;
    let param = param(lo, span, binding.param_name());
    Some(lo.add(NodeKind::Lambda, Payload::None, span, &[param, body]))
}

/// `x => <body>` over a plain named variable (the `equals` right key's own
/// scope).
fn range_lambda(lo: &mut Lowering, span: Span, range_var: &str, body: TsNode) -> NodeId {
    let param = param(lo, span, range_var);
    let body = lower_expr(lo, body);
    lo.add(NodeKind::Lambda, Payload::None, span, &[param, body])
}

fn param(lo: &mut Lowering, span: Span, name: &str) -> NodeId {
    let sym = lo.sym(name);
    lo.add(NodeKind::Param, Payload::Name(sym), span, &[])
}

/// `recv.Method(args…)` — the `Call(Field(Method, recv), args…)` shape
/// `lower_call` gives a source-level member invocation.
fn method_call(
    lo: &mut Lowering,
    span: Span,
    recv: NodeId,
    method: &str,
    args: &[NodeId],
) -> NodeId {
    let sym = lo.sym(method);
    let callee = lo.add(NodeKind::Field, Payload::Name(sym), span, &[recv]);
    let mut kids = vec![callee];
    kids.extend_from_slice(args);
    lo.add(NodeKind::Call, Payload::None, span, &kids)
}

/// Is the `from`/`join` at `rest[idx]` immediately followed by the query's
/// final `select` (possibly with an `into` continuation after it)? If so the
/// spec translates through the operator's own result selector, with no
/// transparent identifier.
fn immediate_select<'t>(rest: &[TsNode<'t>], idx: usize) -> Option<TsNode<'t>> {
    let next = rest.get(idx + 1)?;
    if next.kind() != "select_clause" {
        return None;
    }
    let ends_query = match rest.get(idx + 2) {
        None => true,
        Some(after) => after.kind() == "identifier",
    };
    if !ends_query {
        return None;
    }
    next.named_child(0)
}

fn is_identity(lo: &Lowering, node: TsNode, binding: &Binding) -> bool {
    matches!(binding, Binding::Simple(v)
        if node.kind() == "identifier" && lo.text(node) == v)
}

/// Does the CST subtree mention any of these identifiers? Over-approximates
/// (member names count as mentions) — used only to fail closed.
fn mentions(lo: &Lowering, node: TsNode, names: &[&str]) -> bool {
    if node.kind() == "identifier" && names.contains(&lo.text(node)) {
        return true;
    }
    Lowering::named_children(node)
        .into_iter()
        .any(|c| mentions(lo, c, names))
}

/// Named children with comments filtered out (a comment between clauses or
/// members must not shift positional reads).
fn named(node: TsNode) -> Vec<TsNode> {
    Lowering::named_children(node)
        .into_iter()
        .filter(|c| c.kind() != "comment")
        .collect()
}

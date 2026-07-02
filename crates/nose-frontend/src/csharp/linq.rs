use super::*;

/// LINQ query syntax → the method-syntax chain the C# spec prescribes
/// (`from x in xs where p select e` ≡ `xs.Where(x => p).Select(x => e)`), so
/// the two spellings converge on one IL shape. Only the simple pipeline
/// (`from`/`where`/`orderby`/`select`/`group by`) desugars; clauses that
/// introduce transparent identifiers (`let`, `join`, a second `from`) or an
/// `into` continuation return `None` so the whole query stays fail-closed
/// `Raw` rather than desugar unsoundly.
pub(super) fn lower_query(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let clauses = Lowering::named_children(node);
    let (from, rest) = clauses.split_first()?;
    if from.kind() != "from_clause" {
        return None;
    }
    let range_var = lo.text(from.child_by_field_name("name")?);
    let source = *Lowering::named_children(*from).last()?;
    if rest.iter().any(|c| {
        !matches!(
            c.kind(),
            "where_clause" | "order_by_clause" | "select_clause" | "group_clause"
        )
    }) {
        return None;
    }
    let mut acc = lower_expr(lo, source);
    let mut operations = 0usize;
    for clause in rest {
        match clause.kind() {
            "where_clause" => {
                let pred = clause.named_child(0)?;
                let lambda = range_lambda(lo, span, range_var, pred);
                acc = method_call(lo, span, acc, "Where", &[lambda]);
                operations += 1;
            }
            "order_by_clause" => {
                acc = lower_orderings(lo, span, acc, range_var, *clause)?;
                operations += 1;
            }
            "group_clause" => {
                // `group e by k` → `.GroupBy(x => k)` (identity element) or
                // `.GroupBy(x => k, x => e)`.
                let kids = Lowering::named_children(*clause);
                let [elem, key] = kids[..] else {
                    return None;
                };
                let key_lambda = range_lambda(lo, span, range_var, key);
                let mut args = vec![key_lambda];
                if !is_range_identity(lo, elem, range_var) {
                    args.push(range_lambda(lo, span, range_var, elem));
                }
                acc = method_call(lo, span, acc, "GroupBy", &args);
            }
            "select_clause" => {
                let result = clause.named_child(0)?;
                // A final identity `select x` after another operation elides,
                // as in the spec's translation; a degenerate `from x in xs
                // select x` keeps its `.Select(x => x)`.
                if operations == 0 || !is_range_identity(lo, result, range_var) {
                    let lambda = range_lambda(lo, span, range_var, result);
                    acc = method_call(lo, span, acc, "Select", &[lambda]);
                }
            }
            _ => return None,
        }
    }
    Some(acc)
}

/// `orderby k1, k2 descending, …` → `.OrderBy[Descending](x => k1)
/// .ThenBy[Descending](x => k2)…`. Keys are the clause's named children; each
/// may be followed by an anonymous `ascending`/`descending` token.
fn lower_orderings(
    lo: &mut Lowering,
    span: Span,
    mut acc: NodeId,
    range_var: &str,
    clause: TsNode,
) -> Option<NodeId> {
    let mut first = true;
    let mut pending: Option<TsNode> = None;
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if child.is_named() {
            if let Some(key) = pending.take() {
                acc = ordering_call(lo, span, acc, range_var, key, first, false);
                first = false;
            }
            pending = Some(child);
        } else if matches!(child.kind(), "ascending" | "descending") {
            let key = pending.take()?;
            let descending = child.kind() == "descending";
            acc = ordering_call(lo, span, acc, range_var, key, first, descending);
            first = false;
        }
    }
    if let Some(key) = pending {
        acc = ordering_call(lo, span, acc, range_var, key, first, false);
    }
    Some(acc)
}

fn ordering_call(
    lo: &mut Lowering,
    span: Span,
    recv: NodeId,
    range_var: &str,
    key: TsNode,
    first: bool,
    descending: bool,
) -> NodeId {
    let method = match (first, descending) {
        (true, false) => "OrderBy",
        (true, true) => "OrderByDescending",
        (false, false) => "ThenBy",
        (false, true) => "ThenByDescending",
    };
    let lambda = range_lambda(lo, span, range_var, key);
    method_call(lo, span, recv, method, &[lambda])
}

/// `x => <body>` over the query's range variable — the shape `lower_lambda`
/// gives a source-level lambda, so the desugared chain converges with
/// hand-written method syntax.
fn range_lambda(lo: &mut Lowering, span: Span, range_var: &str, body: TsNode) -> NodeId {
    let sym = lo.sym(range_var);
    let param = lo.add(NodeKind::Param, Payload::Name(sym), span, &[]);
    let body = lower_expr(lo, body);
    lo.add(NodeKind::Lambda, Payload::None, span, &[param, body])
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

fn is_range_identity(lo: &Lowering, node: TsNode, range_var: &str) -> bool {
    node.kind() == "identifier" && lo.text(node) == range_var
}

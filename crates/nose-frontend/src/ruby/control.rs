use super::*;

pub(super) fn lower_if(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node
        .child_by_field_name("condition")
        .map(|c| lower_expr(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    let then = node
        .child_by_field_name("consequence")
        .map(|c| block_of(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    let mut kids = vec![cond, then];
    if let Some(alt) = node.child_by_field_name("alternative") {
        // `else`/`elsif` clause
        let e = if alt.kind() == "else" {
            block_of(lo, alt)
        } else {
            lower_if(lo, alt)
        };
        kids.push(e);
    }
    lo.add(NodeKind::If, Payload::None, span, &kids)
}
pub(super) fn lower_while(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::while_loop(lo, node, lower_expr, block_of)
}
pub(super) fn lower_for(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let pat = node
        .child_by_field_name("pattern")
        .map(|p| lower_expr(lo, p))
        .unwrap_or_else(|| lo.empty_block(span));
    // tree-sitter-ruby wraps the iterable in an `in` node (`for x in xs` →
    // value: (in (identifier))); lower the wrapped expression, not the wrapper,
    // or the iterable becomes an exact-unsafe `Raw("in")`.
    let iter = node
        .child_by_field_name("value")
        .map(|v| {
            let target = if v.kind() == "in" {
                Lowering::named_children(v).into_iter().next().unwrap_or(v)
            } else {
                v
            };
            lower_expr(lo, target)
        })
        .unwrap_or_else(|| lo.empty_block(span));
    let body = node
        .child_by_field_name("body")
        .map(|b| block_of(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::ForEach),
        span,
        &[pat, iter, body],
    )
}
/// `case x when a … when b …` → an `if`/`else` chain (the bodies are the signal).
pub(super) fn lower_case(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let scrutinee = node.child_by_field_name("value").map(|v| lower_expr(lo, v));
    let whens: Vec<TsNode> = Lowering::named_children(node)
        .into_iter()
        .filter(|c| c.kind() == "when" || c.kind() == "else")
        .collect();
    let mut acc = lo.empty_block(span);
    for w in whens.iter().rev() {
        let body = lower_case_arm_body(lo, *w, span);
        if w.kind() == "else" {
            acc = body;
        } else {
            // `when p1, p2 …` → `scrutinee == p1 || scrutinee == p2 || …`. Each `when`
            // carries one or more `pattern` fields wrapping the match expression; lower
            // those so the pattern's computation is in the IL (previously the condition
            // was `scrutinee == scrutinee`, dropping the patterns entirely).
            let cmps: Vec<NodeId> = Lowering::named_children(*w)
                .into_iter()
                .filter(|c| c.kind() == "pattern")
                .map(|p| {
                    let pv = p
                        .named_child(0)
                        .map(|e| lower_expr(lo, e))
                        .unwrap_or_else(|| lo.empty_block(span));
                    match scrutinee {
                        Some(subject) => {
                            lo.add(NodeKind::BinOp, Payload::Op(Op::Eq), span, &[subject, pv])
                        }
                        None => pv,
                    }
                })
                .collect();
            let cond = cmps
                .into_iter()
                .reduce(|a, b| lo.add(NodeKind::BinOp, Payload::Op(Op::Or), span, &[a, b]))
                .unwrap_or_else(|| match scrutinee {
                    Some(subject) => lo.add(
                        NodeKind::BinOp,
                        Payload::Op(Op::Eq),
                        span,
                        &[subject, subject],
                    ),
                    None => lo.empty_block(span),
                });
            acc = lo.add(NodeKind::If, Payload::None, span, &[cond, body, acc]);
        }
    }
    acc
}
pub(super) fn lower_case_arm_body(lo: &mut Lowering, arm: TsNode, span: Span) -> NodeId {
    arm.child_by_field_name("body")
        .map(|body| block_of(lo, body))
        .unwrap_or_else(|| {
            if arm.kind() == "else" {
                lower_clause_body(lo, arm)
            } else {
                lo.empty_block(span)
            }
        })
}
/// `begin … rescue … ensure … else … end` → `Try(body, handler-blocks…)`, converging
/// with try/catch in other languages. Exception-type lists are skipped (data, not behavior).
pub(super) fn lower_begin(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut body = Vec::new();
    let mut handlers = Vec::new();
    for c in Lowering::named_children(node) {
        match c.kind() {
            "rescue" | "ensure" => handlers.push(lower_clause_body(lo, c)),
            // `else` runs exactly when the body completes without raising — the
            // success path's tail, not a handler. Handler position let the
            // no-throw fingerprint convention erase it entirely (#210).
            "else" => {
                for s in Lowering::named_children(c) {
                    if matches!(s.kind(), "exceptions" | "exception_variable" | "then") {
                        extend_clause_statements(lo, s, &mut body);
                    } else if let Some(id) = lower_stmt(lo, s) {
                        body.push(id);
                    }
                }
            }
            _ => {
                if let Some(s) = lower_stmt(lo, c) {
                    body.push(s);
                }
            }
        }
    }
    let body_block = lo.add(NodeKind::Block, Payload::None, span, &body);
    let mut kids = vec![body_block];
    kids.extend(handlers);
    lo.add(NodeKind::Try, Payload::None, span, &kids)
}
/// A `rescue`/`ensure`/`else` clause → a `Block` of its statements (its exception-type
/// list and exception variable carry no behavior; a `then` wrapper is erased while
/// preserving its body.
pub(super) fn lower_clause_body(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut stmts = Vec::new();
    for c in Lowering::named_children(node) {
        extend_clause_statements(lo, c, &mut stmts);
    }
    lo.add(NodeKind::Block, Payload::None, span, &stmts)
}
pub(super) fn extend_clause_statements(lo: &mut Lowering, node: TsNode, out: &mut Vec<NodeId>) {
    match node.kind() {
        "exceptions" | "exception_variable" => {}
        "then" => {
            for child in Lowering::named_children(node) {
                extend_clause_statements(lo, child, out);
            }
        }
        _ => {
            if let Some(stmt) = lower_stmt(lo, node) {
                out.push(stmt);
            }
        }
    }
}
/// `body if cond` / `body unless cond` → `If(cond, Block[body])`, matching the block
/// `if`/`unless` form. Used from both statement and expression (tail) position.
pub(super) fn lower_modifier(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let body = node
        .child_by_field_name("body")
        .and_then(|b| lower_stmt(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    let then = lo.add(NodeKind::Block, Payload::None, span, &[body]);
    let mut cond = node
        .child_by_field_name("condition")
        .map(|c| lower_expr(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    if node.kind() == "unless_modifier" {
        cond = lo.add(NodeKind::UnOp, Payload::Op(Op::Not), span, &[cond]);
    }
    lo.add(NodeKind::If, Payload::None, span, &[cond, then])
}
/// `body while cond` / `body until cond` → `Loop(While, cond, Block[body])`.
pub(super) fn lower_loop_modifier(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let body_node = node.child_by_field_name("body");
    let body = body_node
        .and_then(|b| lower_stmt(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    let block = lo.add(NodeKind::Block, Payload::None, span, &[body]);
    let mut cond = node
        .child_by_field_name("condition")
        .map(|c| lower_expr(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    if node.kind() == "until_modifier" {
        cond = lo.add(NodeKind::UnOp, Payload::Op(Op::Not), span, &[cond]);
    }
    let loop_node = lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::While),
        span,
        &[cond, block],
    );
    if body_node.is_some_and(|body| matches!(body.kind(), "begin" | "do")) {
        lo.add(NodeKind::Block, Payload::None, span, &[body, loop_node])
    } else {
        loop_node
    }
}
pub(super) fn raw_kids(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .map(|c| lower_expr(lo, c))
        .collect();
    lo.raw(node.kind(), span, &kids)
}

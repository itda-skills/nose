use super::*;

pub(super) fn lower_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Block, lower_stmt)
}
pub(super) fn lower_stmt(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    match node.kind() {
        "compound_statement" => Some(lower_block(lo, node)),
        "declaration" => Some(lower_decl(lo, node)),
        "expression_statement" => {
            let c = node.named_child(0)?;
            match c.kind() {
                "assignment_expression" | "update_expression" => Some(lower_expr(lo, c)),
                _ => {
                    let e = lower_expr(lo, c);
                    Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
                }
            }
        }
        "if_statement" => Some(lower_if(lo, node)),
        "for_statement" => Some(lower_for(lo, node)),
        "while_statement" | "do_statement" => Some(lower_while(lo, node)),
        "switch_statement" => Some(lower_switch(lo, node)),
        "return_statement" => {
            let mut kids = Vec::new();
            if let Some(v) = node.named_child(0) {
                kids.push(lower_expr(lo, v));
            }
            Some(lo.add(NodeKind::Return, Payload::None, span, &kids))
        }
        "break_statement" => Some(lo.add(NodeKind::Break, Payload::None, span, &[])),
        "continue_statement" => Some(lo.add(NodeKind::Continue, Payload::None, span, &[])),
        // `label: stmt` (goto target) — lower the inner statement, drop the label.
        "labeled_statement" => Lowering::named_children(node)
            .into_iter()
            .next_back()
            .and_then(|s| lower_stmt(lo, s)),
        // `goto label` — a jump; model as Break (drop the label so it doesn't leak).
        "goto_statement" => Some(lo.add(NodeKind::Break, Payload::None, span, &[])),
        // `#if`/`#ifdef`/… conditional compilation: lower the guarded statements as a
        // Block (skip the condition), so the code inside doesn't fall through to Raw.
        "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" | "preproc_elifdef" => {
            Some(lower_preproc(lo, node))
        }
        ";"
        | "comment"
        | "preproc_call"
        | "preproc_def"
        | "preproc_function_def"
        | "preproc_include" => None,
        _ => {
            let e = lower_expr(lo, node);
            Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
        }
    }
}
/// `#if COND … #else … #endif` and friends → a `Block` of the guarded statements,
/// skipping the condition/macro name (which carry no runtime behavior).
pub(super) fn lower_preproc(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node
        .child_by_field_name("condition")
        .or_else(|| node.child_by_field_name("name"));
    let mut kids = Vec::new();
    for c in Lowering::named_children(node) {
        if Some(c) == cond {
            continue;
        }
        if let Some(s) = lower_stmt(lo, c) {
            kids.push(s);
        }
    }
    lo.add(NodeKind::Block, Payload::None, span, &kids)
}
pub(super) fn lower_if(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::if_stmt(lo, node, lower_expr, stmt_as_block, |lo, alt| {
        // `else` clause wraps the alternative statement.
        let inner = alt.named_child(0).unwrap_or(alt);
        stmt_as_block(lo, inner)
    })
}
pub(super) fn stmt_as_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::stmt_as_block(lo, node, "compound_statement", lower_block, lower_stmt)
}
pub(super) fn lower_for(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::c_style_for(
        lo,
        node,
        "initializer",
        "update",
        lower_stmt,
        lower_expr,
        lower_expr,
        stmt_as_block,
    )
}
pub(super) fn lower_while(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::while_loop(lo, node, lower_expr, stmt_as_block)
}
pub(super) fn lower_switch(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::switch_to_if_chain(lo, node, |k| k == "case_statement", lower_expr, lower_stmt)
}

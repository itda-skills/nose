use super::*;

pub(super) fn is_tail_expr(kind: &str) -> bool {
    is_expr_kind(kind)
        && !matches!(
            kind,
            "assignment"
                | "if_statement"
                | "switch_statement"
                | "for_statement"
                | "while_statement"
        )
}
pub(super) fn lower_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    let block = Lowering::named_children(node)
        .into_iter()
        .find(|child| child.kind() == "statements")
        .unwrap_or(node);
    crate::lower::collect_into(lo, block, NodeKind::Block, lower_stmt)
}
pub(super) fn lower_stmt(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    match node.kind() {
        "statements" | "function_body" => Some(lower_block(lo, node)),
        "function_declaration" => Some(lower_function(lo, node, false)),
        "protocol_function_declaration" => Some(lower_function(lo, node, true)),
        "class_declaration"
        | "struct_declaration"
        | "enum_declaration"
        | "protocol_declaration" => Some(lower_type(lo, node)),
        "extension_declaration" => Some(lower_extension(lo, node)),
        "property_declaration"
        | "protocol_property_declaration"
        | "protocol_property_requirements" => Some(lower_property(lo, node)),
        "assignment" => Some(lower_assignment(lo, node)),
        "control_transfer_statement" => lower_control_transfer(lo, node),
        "if_statement" | "guard_statement" => Some(lower_if(lo, node)),
        "switch_statement" => Some(lower_switch(lo, node)),
        "for_statement" => Some(lower_for(lo, node)),
        "while_statement" => Some(lower_while(lo, node)),
        "repeat_while_statement" => Some(lower_repeat_while(lo, node)),
        "do_statement" => Some(lower_do(lo, node)),
        "directive" => Some(lower_directive(lo, node)),
        "discard_statement" => None,
        "line_comment" | "multiline_comment" => None,
        k if is_expr_kind(k) => {
            let expr = lower_expr(lo, node);
            Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[expr]))
        }
        _ => {
            let kids: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .map(|child| lower_expr(lo, child))
                .collect();
            Some(lo.raw(node.kind(), span, &kids))
        }
    }
}
pub(super) fn lower_directive(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let tag = swift_directive_tag(lo.text(node));
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), "line_comment" | "multiline_comment"))
        .map(|child| lower_expr(lo, child))
        .collect();
    lo.add(NodeKind::Seq, Payload::Name(lo.sym(tag)), span, &kids)
}
fn swift_directive_tag(text: &str) -> &'static str {
    let trimmed = text.trim_start();
    if trimmed.starts_with("#elseif") {
        "swift_directive_elseif"
    } else if trimmed.starts_with("#else") {
        "swift_directive_else"
    } else if trimmed.starts_with("#endif") {
        "swift_directive_endif"
    } else if trimmed.starts_with("#if") {
        "swift_directive_if"
    } else if trimmed.starts_with("#warning") {
        "swift_directive_warning"
    } else if trimmed.starts_with("#error") {
        "swift_directive_error"
    } else if trimmed.starts_with("#sourceLocation") {
        "swift_directive_source_location"
    } else {
        "swift_directive"
    }
}
pub(super) fn lower_control_transfer(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let text = lo.text(node).trim_start();
    if text.starts_with("return") {
        let kids: Vec<NodeId> = node
            .child_by_field_name("result")
            .into_iter()
            .map(|value| lower_expr(lo, value))
            .collect();
        return Some(lo.add(NodeKind::Return, Payload::None, span, &kids));
    }
    if text.starts_with("throw") {
        let kids: Vec<NodeId> = node
            .child_by_field_name("result")
            .into_iter()
            .map(|value| lower_expr(lo, value))
            .collect();
        return Some(lo.add(NodeKind::Throw, Payload::None, span, &kids));
    }
    if text.starts_with("break") {
        return Some(lo.add(NodeKind::Break, Payload::None, span, &[]));
    }
    if text.starts_with("continue") {
        return Some(lo.add(NodeKind::Continue, Payload::None, span, &[]));
    }
    None
}
pub(super) fn lower_assignment(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let lhs_node = node.child_by_field_name("target");
    let rhs_node = node.child_by_field_name("result");
    let op = node
        .child_by_field_name("operator")
        .map(|op| lo.text(op).trim().to_string())
        .unwrap_or_else(|| "=".to_string());
    let lhs = lhs_node
        .map(|target| lower_store_target(lo, target))
        .unwrap_or_else(|| lo.empty_block(span));
    let rhs = rhs_node
        .map(|value| lower_expr(lo, value))
        .unwrap_or_else(|| lo.empty_block(span));
    if op == "=" {
        return lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs]);
    }
    let read_lhs = lhs_node
        .map(|target| lower_expr(lo, target))
        .unwrap_or_else(|| lo.empty_block(span));
    let compound_base = swift_compound_assignment_base(&op).or_else(|| {
        op.strip_suffix('=')
            .filter(|base| common_bin_op(base).is_some())
    });
    let value = compound_base
        .map(|base| {
            if let Some(op) = swift_bin_op(base) {
                lo.add(NodeKind::BinOp, Payload::Op(op), span, &[read_lhs, rhs])
            } else {
                lower_swift_specific_infix(lo, span, base, &[read_lhs, rhs])
            }
        })
        .unwrap_or_else(|| lo.raw(&format!("assignment {op}"), span, &[read_lhs, rhs]));
    lo.add(NodeKind::Assign, Payload::None, span, &[lhs, value])
}
pub(super) fn lower_if(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node
        .child_by_field_name("condition")
        .map(|condition| lower_condition(lo, condition))
        .unwrap_or_else(|| lo.empty_block(span));
    let then = first_statements_child(node)
        .map(|body| lower_block(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    let mut kids = vec![cond, then];
    if let Some(else_node) = Lowering::named_children(node)
        .into_iter()
        .find(|child| child.kind() == "else")
    {
        let alt = Lowering::named_children(else_node)
            .into_iter()
            .find(|child| !matches!(child.kind(), "line_comment" | "multiline_comment"));
        if let Some(alt) = alt {
            let lowered = if alt.kind() == "if_statement" {
                lower_if(lo, alt)
            } else {
                lower_block(lo, alt)
            };
            kids.push(lowered);
        }
    }
    lo.add(NodeKind::If, Payload::None, span, &kids)
}
pub(super) fn lower_condition(lo: &mut Lowering, node: TsNode) -> NodeId {
    match node.kind() {
        "condition" | "pattern" => {
            let exprs: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .filter(|child| !is_type_level(child.kind()))
                .map(|child| lower_expr(lo, child))
                .collect();
            fold_and(lo, lo.span(node), exprs)
        }
        _ => lower_expr(lo, node),
    }
}
pub(super) fn fold_and(lo: &mut Lowering, span: Span, mut values: Vec<NodeId>) -> NodeId {
    if values.is_empty() {
        return lo.empty_block(span);
    }
    let mut acc = values.remove(0);
    for value in values {
        acc = lo.add(NodeKind::BinOp, Payload::Op(Op::And), span, &[acc, value]);
    }
    acc
}
pub(super) fn lower_while(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node
        .child_by_field_name("condition")
        .map(|condition| lower_condition(lo, condition))
        .unwrap_or_else(|| lo.empty_block(span));
    let body = first_statements_child(node)
        .map(|body| lower_block(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::While),
        span,
        &[cond, body],
    )
}
pub(super) fn lower_repeat_while(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node
        .child_by_field_name("condition")
        .map(|condition| lower_condition(lo, condition))
        .unwrap_or_else(|| lo.empty_block(span));
    let body = first_statements_child(node)
        .map(|body| lower_block(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::While),
        span,
        &[cond, body],
    )
}
pub(super) fn lower_for(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let pattern = node
        .child_by_field_name("item")
        .map(|item| binding_var(lo, item, lo.span(item)))
        .unwrap_or_else(|| lo.empty_block(span));
    let iterable = node
        .child_by_field_name("collection")
        .map(|collection| lower_expr(lo, collection))
        .unwrap_or_else(|| lo.empty_block(span));
    let body = first_statements_child(node)
        .map(|body| lower_block(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::ForEach),
        span,
        &[pattern, iterable, body],
    )
}
pub(super) fn lower_do(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let body = first_statements_child(node)
        .map(|body| lower_block(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    let mut kids = vec![body];
    for catch in Lowering::named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "catch_block")
    {
        kids.push(lower_block(lo, catch));
    }
    lo.add(NodeKind::Try, Payload::None, span, &kids)
}
pub(super) fn lower_switch(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let scrutinee = node
        .child_by_field_name("expr")
        .map(|expr| lower_expr(lo, expr));
    let mut arms = Vec::new();
    for entry in Lowering::named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "switch_entry")
    {
        let (test, body) = lower_switch_entry(lo, entry, scrutinee, span);
        arms.push((test, body));
    }
    let mut acc: Option<NodeId> = None;
    for (test, body) in arms.into_iter().rev() {
        match test {
            None => acc = Some(body),
            Some(test) => {
                let mut kids = vec![test, body];
                if let Some(else_node) = acc {
                    kids.push(else_node);
                }
                acc = Some(lo.add(NodeKind::If, Payload::None, span, &kids));
            }
        }
    }
    acc.unwrap_or_else(|| lo.empty_block(span))
}
pub(super) fn lower_switch_entry(
    lo: &mut Lowering,
    entry: TsNode,
    scrutinee: Option<NodeId>,
    switch_span: Span,
) -> (Option<NodeId>, NodeId) {
    let span = lo.span(entry);
    let text = lo.text(entry).trim_start();
    let mut labels = Vec::new();
    let mut stmts = Vec::new();
    for child in Lowering::named_children(entry) {
        match child.kind() {
            "switch_pattern" | "pattern" if !text.starts_with("default") => {
                labels.push(lower_switch_label(lo, child, scrutinee, span));
            }
            "statements" => {
                for stmt in Lowering::named_children(child) {
                    if let Some(id) = lower_stmt(lo, stmt) {
                        stmts.push(id);
                    }
                }
            }
            _ if is_expr_kind(child.kind()) && !text.starts_with("default") => {
                labels.push(lower_switch_label(lo, child, scrutinee, span));
            }
            _ => {}
        }
    }
    let body = lo.add(NodeKind::Block, Payload::None, span, &stmts);
    if text.starts_with("default") {
        return (None, body);
    }
    let test = if labels.is_empty() {
        Some(lo.raw("switch_case", switch_span, &[]))
    } else {
        fold_or(lo, span, labels)
    };
    (test, body)
}
pub(super) fn lower_switch_label(
    lo: &mut Lowering,
    label: TsNode,
    scrutinee: Option<NodeId>,
    span: Span,
) -> NodeId {
    let value = match label.kind() {
        "switch_pattern" | "pattern" => lower_pattern_value(lo, label),
        _ => lower_expr(lo, label),
    };
    match scrutinee {
        Some(subject) => lo.add(
            NodeKind::BinOp,
            Payload::Op(Op::Eq),
            span,
            &[subject, value],
        ),
        None => value,
    }
}
pub(super) fn fold_or(lo: &mut Lowering, span: Span, mut values: Vec<NodeId>) -> Option<NodeId> {
    if values.is_empty() {
        return None;
    }
    let mut acc = values.remove(0);
    for value in values {
        acc = lo.add(NodeKind::BinOp, Payload::Op(Op::Or), span, &[acc, value]);
    }
    Some(acc)
}

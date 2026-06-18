use super::*;

pub(super) fn block_of(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Block, lower_stmt)
}
/// The statement body of a `{ |x| … }` / `do |x| … end` block — its `body`
/// field's statements only (excluding the `parameters` node).
pub(super) fn block_body(lo: &mut Lowering, block: TsNode) -> NodeId {
    let span = lo.span(block);
    match block.child_by_field_name("body") {
        Some(b) => block_of(lo, b),
        // some grammars inline statements directly under the block
        None => {
            let stmts: Vec<NodeId> = Lowering::named_children(block)
                .into_iter()
                .filter(|c| c.kind() != "block_parameters")
                .filter_map(|c| lower_stmt(lo, c))
                .collect();
            lo.add(NodeKind::Block, Payload::None, span, &stmts)
        }
    }
}
/// A method/lambda body: wrap the trailing expression in `Return` (Ruby's implicit
/// return), so it converges with explicit-return languages.
pub(super) fn body_with_return(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let children = Lowering::named_children(node);
    let n = children.len();
    let mut stmts = Vec::new();
    for (idx, c) in children.into_iter().enumerate() {
        if idx + 1 == n && is_tail_expr(c.kind()) {
            let e = lower_expr(lo, c);
            stmts.push(lo.add(NodeKind::Return, Payload::None, lo.span(c), &[e]));
        } else if let Some(id) = lower_stmt(lo, c) {
            stmts.push(id);
        }
    }
    lo.add(NodeKind::Block, Payload::None, span, &stmts)
}
pub(super) fn is_tail_expr(k: &str) -> bool {
    !matches!(
        k,
        "comment" | "return" | "if" | "unless" | "while" | "until" | "case" | "for"
    )
}
pub(super) fn lower_stmt(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    match node.kind() {
        "method" | "singleton_method" => Some(lower_method(lo, node)),
        "class" | "module" | "singleton_class" => Some(lower_class(lo, node)),
        "comment" | "call" if node.kind() == "comment" => None,
        "if" | "unless" => Some(lower_if(lo, node)),
        "while" | "until" => Some(lower_while(lo, node)),
        "for" => Some(lower_for(lo, node)),
        "case" => Some(lower_case(lo, node)),
        "return" => {
            let mut kids = Vec::new();
            if let Some(v) = node.named_child(0) {
                kids.push(lower_return_value(lo, v));
            }
            Some(lo.add(NodeKind::Return, Payload::None, span, &kids))
        }
        "break" => Some(lo.add(NodeKind::Break, Payload::None, span, &[])),
        "next" => Some(lo.add(NodeKind::Continue, Payload::None, span, &[])),
        // `begin … rescue … ensure … end` → Try (body + handler/ensure blocks).
        "begin" | "do" => Some(lower_begin(lo, node)),
        // `alias new old` carries no behavior to dedupe.
        "alias" | "undef" => None,
        // Guard-clause modifiers: `stmt if cond` / `stmt unless cond` → `If` so they
        // converge with the block forms and other languages' guards.
        "if_modifier" | "unless_modifier" => Some(lower_modifier(lo, node)),
        "assignment" | "operator_assignment" => Some(lower_assign(lo, node)),
        "call" | "method_call" => {
            let e = lower_call(lo, node);
            Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
        }
        _ => {
            let e = lower_expr(lo, node);
            Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
        }
    }
}
pub(super) fn lower_method(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let mut kids = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for p in Lowering::named_children(params) {
            let pspan = lo.span(p);
            let sym = param_name(lo, p);
            kids.push(lo.add(
                NodeKind::Param,
                sym.map(Payload::Name).unwrap_or(Payload::None),
                pspan,
                &[],
            ));
        }
    }
    let body = node
        .child_by_field_name("body")
        .map(|b| body_with_return(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    kids.push(body);
    let func = lo.add(NodeKind::Func, Payload::None, span, &kids);
    lo.push_unit(func, UnitKind::Method, name);
    func
}
pub(super) fn param_name(lo: &Lowering, p: TsNode) -> Option<Symbol> {
    match p.kind() {
        "identifier" => Some(lo.sym(lo.text(p))),
        _ => p.named_child(0).map(|n| lo.sym(lo.text(n))),
    }
}
pub(super) fn lower_return_value(lo: &mut Lowering, node: TsNode) -> NodeId {
    if node.kind() == "argument_list" && node.named_child_count() == 1 {
        if let Some(v) = node.named_child(0) {
            return lower_expr(lo, v);
        }
    }
    lower_expr(lo, node)
}
pub(super) fn lower_class(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let body = node
        .child_by_field_name("body")
        .map(|b| block_of(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.push_unit(body, UnitKind::Class, name);
    body
}

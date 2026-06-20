use super::*;

pub(super) fn lower_func(lo: &mut Lowering, node: TsNode, method: bool) -> NodeId {
    crate::lower::function_unit(lo, node, method, lower_params, lower_block)
}
pub(super) fn lower_params(lo: &mut Lowering, params: TsNode, out: &mut Vec<NodeId>) {
    for decl in Lowering::named_children(params) {
        // a parameter_declaration may bind several names sharing one type
        let mut cur = decl.walk();
        let names: Vec<TsNode> = decl.children_by_field_name("name", &mut cur).collect();
        if names.is_empty() {
            // unnamed parameter (type only) or variadic — emit one anonymous Param
            let span = lo.span(decl);
            out.push(lo.add(NodeKind::Param, Payload::None, span, &[]));
        } else {
            let domain = lo.type_domain_from_text_with_dependencies(lo.text(decl));
            for n in names {
                let span = lo.span(n);
                let sym = lo.sym(lo.text(n));
                if let Some(domain) = &domain {
                    lo.record_param_domain_resolution(span, domain.clone());
                }
                out.push(lo.add(NodeKind::Param, Payload::Name(sym), span, &[]));
            }
        }
    }
}
pub(super) fn lower_var_decl(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut assigns = Vec::new();
    let is_const = node.kind() == "const_declaration";
    let mut const_ordinal = 0usize;
    let mut previous_const_value: Option<TsNode> = None;
    for spec in Lowering::named_children(node) {
        if !matches!(spec.kind(), "var_spec" | "const_spec" | "var_spec_list") {
            continue;
        }
        let sspan = lo.span(spec);
        let mut cur = spec.walk();
        let names: Vec<TsNode> = spec.children_by_field_name("name", &mut cur).collect();
        let value_node = spec
            .child_by_field_name("value")
            .and_then(|v| v.named_child(0));
        let rhs_node = if is_const {
            value_node.or(previous_const_value)
        } else {
            value_node
        };
        for n in names {
            let nspan = lo.span(n);
            let lhs = lo.var(lo.text(n), nspan);
            let r = rhs_node
                .map(|value| {
                    if is_const {
                        lower_expr_with_iota(lo, value, Some(const_ordinal))
                    } else {
                        lower_expr(lo, value)
                    }
                })
                .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), nspan, &[]));
            assigns.push(lo.add(NodeKind::Assign, Payload::None, sspan, &[lhs, r]));
        }
        if is_const {
            if value_node.is_some() {
                previous_const_value = value_node;
            }
            const_ordinal += 1;
        }
    }
    if assigns.len() == 1 {
        assigns[0]
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}

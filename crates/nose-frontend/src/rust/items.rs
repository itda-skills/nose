use super::*;

pub(super) fn lower_items(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Module, lower_item)
}
/// Lower a top-level / module-level item.
pub(super) fn lower_item(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    match node.kind() {
        "function_item" => Some(lower_func(lo, node, false)),
        "impl_item" | "trait_item" => Some(lower_type_block(lo, node, true)),
        "struct_item" | "enum_item" | "union_item" => Some(lower_type_block(lo, node, false)),
        "mod_item" => Some(lower_mod_item(lo, node)),
        "const_item" | "static_item" => Some(lower_value_item(lo, node)),
        "use_declaration" | "extern_crate_declaration" => Some(
            lower_static_import(lo, node).unwrap_or_else(|| crate::lower::import_tokens(lo, node)),
        ),
        "macro_definition" => Some(lower_macro_definition_shadow(lo, node)),
        // type aliases, attributes: no behavior to model
        "type_item"
        | "attribute_item"
        | "inner_attribute_item"
        // trait method/const declarations without a body: no behavior to model
        | "function_signature_item"
        | "associated_type"
        | "line_comment"
        | "block_comment" => None,
        _ => lower_stmt(lo, node),
    }
}
pub(super) fn lower_static_import(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let text = lo.text(node).trim().trim_end_matches(';').trim();
    let path = text.strip_prefix("use ")?.trim();
    if path.contains('*') || path.contains('{') {
        return None;
    }
    let (path, local) = if let Some((path, local)) = path.split_once(" as ") {
        (path.trim(), local.trim())
    } else {
        let local = path.rsplit("::").next()?.trim();
        (path, local)
    };
    let (module, exported) = path.rsplit_once("::")?;
    Some(crate::lower::import_binding(
        lo,
        span,
        local,
        module.trim(),
        exported.trim(),
    ))
}
/// `impl`/`trait`/`struct`/`enum` → a `Class` unit whose body holds methods (each
/// also a unit) or field declarations, so similar types/impls cluster.
pub(super) fn lower_type_block(lo: &mut Lowering, node: TsNode, methods: bool) -> NodeId {
    let span = lo.span(node);
    let name = rust_item_name(lo, node);
    let body = node.child_by_field_name("body");
    let mut kids = Vec::new();
    if let Some(body) = body {
        for c in Lowering::named_children(body) {
            if methods {
                if let Some(id) = lower_item(lo, c) {
                    kids.push(id);
                }
            } else if let Some(id) = lower_field_decl(lo, c) {
                kids.push(id);
            }
        }
    }
    let payload = name.map(Payload::Name).unwrap_or(Payload::None);
    let block = lo.add(NodeKind::Block, payload, span, &kids);
    // Only `impl`/`trait` blocks (which carry behavior) are detection units. Pure
    // data-type definitions (struct/enum/union) are NOT unit-ified: they have no
    // call/control/value signal, so any two same-arity types look "similar" and
    // flood the candidate report with false positives (see docs/dogfooding.md).
    if methods {
        lo.push_unit_with_origin(block, UnitKind::Class, name, rust_type_origin(node));
    }
    block
}
pub(super) fn rust_type_origin(node: TsNode) -> UnitOrigin {
    match node.kind() {
        "trait_item" => {
            let has_body = rust_node_has_function_body(node);
            UnitOrigin::new(
                UnitDomains::of(UnitDomain::TypeContract).union(if has_body {
                    UnitDomains::of(UnitDomain::ImplementationType)
                } else {
                    UnitDomains::empty()
                }),
                UnitSubkind::InterfaceTraitProtocol,
                if has_body {
                    UnitBodyKind::Mixed
                } else {
                    UnitBodyKind::DeclarationOnly
                },
                SourceGranularity::WholeUnit,
                RegionKind::Code,
            )
            .with_evidence(if has_body {
                UnitEvidenceFlag::HasDefaultBody
            } else {
                UnitEvidenceFlag::DeclarationOnly
            })
            .with_evidence(UnitEvidenceFlag::TypeOnly)
        }
        "impl_item" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::ImplementationType),
            UnitSubkind::ImplBlock,
            UnitBodyKind::Implementation,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::HasReusableBody),
        _ => UnitOrigin::unknown(),
    }
}
pub(super) fn rust_node_has_function_body(node: TsNode) -> bool {
    Lowering::named_children(node).into_iter().any(|child| {
        if rust_is_nested_item_boundary(child.kind()) {
            return false;
        }
        child.kind() == "function_item" && child.child_by_field_name("body").is_some()
            || rust_node_has_function_body(child)
    })
}
pub(super) fn rust_is_nested_item_boundary(kind: &str) -> bool {
    matches!(
        kind,
        "impl_item" | "trait_item" | "struct_item" | "enum_item" | "union_item" | "mod_item"
    )
}
pub(super) fn lower_mod_item(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let payload = rust_item_name(lo, node)
        .map(Payload::Name)
        .unwrap_or(Payload::None);
    let kids = node
        .child_by_field_name("body")
        .map(|body| {
            Lowering::named_children(body)
                .into_iter()
                .filter_map(|child| lower_item(lo, child))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    lo.add(NodeKind::Module, payload, span, &kids)
}
pub(super) fn rust_item_name(lo: &mut Lowering, node: TsNode) -> Option<Symbol> {
    node.child_by_field_name("name")
        .or_else(|| {
            Lowering::named_children(node)
                .into_iter()
                .find(|child| matches!(child.kind(), "identifier" | "type_identifier"))
        })
        .map(|n| lo.sym(lo.text(n)))
}
/// A struct/enum field or enum variant → an `Assign(name, type-as-literal)` so the
/// shape of the data structure is captured.
pub(super) fn lower_field_decl(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    match node.kind() {
        "field_declaration" | "enum_variant" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| lo.var(lo.text(n), span))
                .unwrap_or_else(|| lo.empty_block(span));
            let ty = lo.add(NodeKind::Lit, Payload::Lit(LitClass::Other), span, &[]);
            Some(lo.add(NodeKind::Assign, Payload::None, span, &[name, ty]))
        }
        _ => None,
    }
}
pub(super) fn lower_value_item(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let lhs = node
        .child_by_field_name("name")
        .map(|n| lo.var(lo.text(n), span))
        .unwrap_or_else(|| lo.empty_block(span));
    let rhs = node
        .child_by_field_name("value")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]));
    lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs])
}

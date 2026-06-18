use super::*;

pub(super) fn lower_items(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Module, lower_item)
}
pub(super) fn lower_item(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration"
        | "annotation_type_declaration" => Some(lower_type(lo, node)),
        "method_declaration" | "constructor_declaration" => Some(lower_method(lo, node)),
        "field_declaration" => Some(lower_field(lo, node)),
        "import_declaration" => {
            Some(lower_import(lo, node).unwrap_or_else(|| crate::lower::import_tokens(lo, node)))
        }
        "package_declaration" => Some(crate::lower::import_tokens(lo, node)),
        "line_comment" | "block_comment" => None,
        _ => lower_stmt(lo, node),
    }
}
pub(super) fn lower_import(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let text = lo.text(node).trim().trim_end_matches(';').trim();
    if let Some(path) = text.strip_prefix("import static ") {
        let path = path.trim();
        if path.ends_with(".*") {
            return None;
        }
        let (module, exported) = path.rsplit_once('.')?;
        return Some(crate::lower::import_binding(
            lo,
            span,
            exported.trim(),
            module.trim(),
            exported.trim(),
        ));
    }
    let path = text.strip_prefix("import ")?.trim();
    if path.ends_with(".*") {
        let module = path.trim_end_matches(".*").trim();
        lo.record_evidence(
            EvidenceAnchor::source_span(span),
            EvidenceKind::Import(ImportEvidenceKind::Wildcard {
                module_hash: stable_symbol_hash(module),
            }),
            "java_wildcard_import",
        );
        return None;
    }
    let (module, exported) = path.rsplit_once('.')?;
    Some(crate::lower::import_binding(
        lo,
        span,
        exported.trim(),
        module.trim(),
        exported.trim(),
    ))
}
/// `class`/`interface`/`enum` → a `Class` unit; its methods become units too.
pub(super) fn lower_type(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let mut kids = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        for c in Lowering::named_children(body) {
            if let Some(id) = lower_item(lo, c) {
                kids.push(id);
            }
        }
    }
    let block = lo.add(NodeKind::Block, Payload::None, span, &kids);
    lo.push_unit_with_origin(block, UnitKind::Class, name, java_type_origin(node));
    block
}
pub(super) fn lower_method(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let mut kids = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for p in Lowering::named_children(params) {
            let pspan = lo.span(p);
            let sym = p.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
            if let Some(domain) =
                nose_semantics::type_domain_from_source_text(Lang::Java, lo.text(p))
            {
                lo.record_param_domain(pspan, domain);
            }
            kids.push(lo.add(
                NodeKind::Param,
                sym.map(Payload::Name).unwrap_or(Payload::None),
                pspan,
                &[],
            ));
        }
    }
    let body_node = node.child_by_field_name("body");
    let body = body_node
        .map(|b| lower_block(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    kids.push(body);
    let func = lo.add(NodeKind::Func, Payload::None, span, &kids);
    lo.push_unit_with_origin(
        func,
        UnitKind::Method,
        name,
        java_method_origin(node, body_node.is_some()),
    );
    func
}
pub(super) fn java_type_origin(node: TsNode) -> UnitOrigin {
    let has_body = java_node_has_method_body(node);
    match node.kind() {
        "interface_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract),
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
            UnitEvidenceFlag::InterfaceDefaultMethod
        } else {
            UnitEvidenceFlag::DeclarationOnly
        })
        .with_evidence(UnitEvidenceFlag::TypeOnly),
        // An annotation type (`@interface`) is a declaration-only type contract, not an
        // implementation-inheritance candidate — it must not read as `extract-base-class`.
        "annotation_type_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract),
            UnitSubkind::DefinedType,
            UnitBodyKind::DeclarationOnly,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::DeclarationOnly)
        .with_evidence(UnitEvidenceFlag::TypeOnly),
        // A `record` is a data/struct contract (its canonical body is the component header);
        // it gains an implementation facet only when it carries real method bodies.
        "record_declaration" => UnitOrigin::new(
            if has_body {
                UnitDomains::of(UnitDomain::TypeContract)
                    .with(UnitDomain::Data)
                    .with(UnitDomain::ImplementationType)
            } else {
                UnitDomains::of(UnitDomain::TypeContract).with(UnitDomain::Data)
            },
            UnitSubkind::StructRecord,
            if has_body {
                UnitBodyKind::Mixed
            } else {
                UnitBodyKind::DeclarativeDenotation
            },
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::RecordHeader)
        .with_evidence(if has_body {
            UnitEvidenceFlag::HasReusableBody
        } else {
            UnitEvidenceFlag::DataShapeOnly
        }),
        "enum_declaration" => UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract).with(UnitDomain::Data),
            UnitSubkind::Enum,
            if has_body {
                UnitBodyKind::Mixed
            } else {
                UnitBodyKind::DeclarativeDenotation
            },
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_domain(if has_body {
            UnitDomain::ImplementationType
        } else {
            UnitDomain::Unknown
        })
        .with_evidence(if has_body {
            UnitEvidenceFlag::HasReusableBody
        } else {
            UnitEvidenceFlag::DataShapeOnly
        }),
        // class_declaration — the only remaining type construct routed here.
        _ => UnitOrigin::new(
            UnitDomains::of(UnitDomain::ImplementationType),
            UnitSubkind::Class,
            if has_body {
                UnitBodyKind::Implementation
            } else {
                UnitBodyKind::DeclarationOnly
            },
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(if has_body {
            UnitEvidenceFlag::HasReusableBody
        } else {
            UnitEvidenceFlag::DeclarationOnly
        }),
    }
}
pub(super) fn java_method_origin(node: TsNode, has_body: bool) -> UnitOrigin {
    if !has_body {
        return UnitOrigin::new(
            UnitDomains::of(UnitDomain::TypeContract),
            UnitSubkind::FunctionPrototype,
            UnitBodyKind::DeclarationOnly,
            SourceGranularity::Member,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::DeclarationOnly)
        .with_evidence(UnitEvidenceFlag::TypeOnly);
    }
    crate::lower::imperative_callable_origin(
        if node.kind() == "constructor_declaration" {
            UnitSubkind::Constructor
        } else {
            UnitSubkind::Method
        },
        true,
    )
}
pub(super) fn java_node_has_method_body(node: TsNode) -> bool {
    Lowering::named_children(node).into_iter().any(|child| {
        if java_is_nested_type_decl(child.kind()) {
            return false;
        }
        matches!(
            child.kind(),
            "method_declaration" | "constructor_declaration"
        ) && child.child_by_field_name("body").is_some()
            || java_node_has_method_body(child)
    })
}
pub(super) fn java_is_nested_type_decl(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}
pub(super) fn lower_field(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    // a field_declaration has one or more variable_declarators
    let mut assigns = Vec::new();
    for d in Lowering::named_children(node) {
        if d.kind() != "variable_declarator" {
            continue;
        }
        let dspan = lo.span(d);
        let lhs = d
            .child_by_field_name("name")
            .map(|n| lo.var(lo.text(n), dspan))
            .unwrap_or_else(|| lo.empty_block(dspan));
        let rhs = d
            .child_by_field_name("value")
            .map(|v| lower_expr(lo, v))
            .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), dspan, &[]));
        assigns.push(lo.add(NodeKind::Assign, Payload::None, dspan, &[lhs, rhs]));
    }
    if assigns.len() == 1 {
        assigns.pop().unwrap()
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}

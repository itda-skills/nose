use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::library_api) fn java_constructor_dependencies_match(
    il: &Il,
    interner: &Interner,
    record: &EvidenceRecord,
    callee_node: NodeId,
    call_span: Span,
    simple_type: &str,
    qualified_type: &str,
    module: &str,
    requires_import_for_simple_type: bool,
    requires_no_local_type_shadow: bool,
) -> bool {
    let Some(actual) = node_name(il, interner, callee_node) else {
        return false;
    };
    java_constructor_dependencies_match_for_name(
        il,
        interner,
        record,
        actual,
        Some(callee_node),
        il.node(callee_node).span,
        call_span,
        simple_type,
        qualified_type,
        module,
        requires_import_for_simple_type,
        requires_no_local_type_shadow,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::library_api) fn java_constructor_dependencies_match_at_span(
    il: &Il,
    interner: &Interner,
    record: &EvidenceRecord,
    callee_span: Span,
    call_span: Span,
    simple_type: &str,
    qualified_type: &str,
    module: &str,
    requires_import_for_simple_type: bool,
    requires_no_local_type_shadow: bool,
) -> bool {
    let Some(callee_node) = node_at_span_with_kind(il, callee_span, NodeKind::Var) else {
        return false;
    };
    java_constructor_dependencies_match(
        il,
        interner,
        record,
        callee_node,
        call_span,
        simple_type,
        qualified_type,
        module,
        requires_import_for_simple_type,
        requires_no_local_type_shadow,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::library_api) fn java_constructor_dependencies_match_for_name(
    il: &Il,
    interner: &Interner,
    record: &EvidenceRecord,
    actual: &str,
    callee_node: Option<NodeId>,
    callee_span: Span,
    call_span: Span,
    simple_type: &str,
    qualified_type: &str,
    module: &str,
    requires_import_for_simple_type: bool,
    requires_no_local_type_shadow: bool,
) -> bool {
    if actual == qualified_type {
        return true;
    }
    if actual != simple_type {
        return false;
    }
    if requires_no_local_type_shadow
        && unit_defines_hash_visible_at(il, interner, stable_symbol_hash(simple_type), callee_span)
    {
        return false;
    }
    if !requires_import_for_simple_type {
        return true;
    }
    let explicit_import = callee_node.is_some_and(|node| {
        dependency_has_imported_binding_node(il, interner, record, node, module, simple_type)
    });
    explicit_import
        || dependency_has_java_wildcard_import_before(
            il,
            interner,
            record,
            module,
            simple_type,
            call_span,
        )
}

pub(in crate::library_api) fn dependency_has_java_wildcard_import_before(
    il: &Il,
    interner: &Interner,
    record: &EvidenceRecord,
    module: &str,
    simple_type: &str,
    call_span: Span,
) -> bool {
    let expected = EvidenceKind::Import(ImportEvidenceKind::Wildcard {
        module_hash: stable_symbol_hash(module),
    });
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && dependency.kind == expected
            && matches!(
                dependency.anchor,
                EvidenceAnchor::SourceSpan(span)
                    if span.file == call_span.file && span.end_byte <= call_span.start_byte
            )
            && !java_explicit_import_conflicts(il, interner, module, simple_type)
    })
}

pub(in crate::library_api) fn java_explicit_import_conflicts(
    il: &Il,
    _interner: &Interner,
    module: &str,
    simple_type: &str,
) -> bool {
    let local_hash = stable_symbol_hash(simple_type);
    let expected = SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash(module),
        exported_hash: stable_symbol_hash(simple_type),
    };
    il.evidence_binding_anchored(local_hash).any(|record| {
        matches!(record.kind, EvidenceKind::Symbol(actual) if actual != expected)
            && record.status == EvidenceStatus::Asserted
    })
}

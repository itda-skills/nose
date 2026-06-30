use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceEmitter, EvidenceKind, EvidenceStatus, Interner,
    NodeId, NodeKind, SymbolEvidenceKind,
};

pub(super) fn rust_imported_binding_evidence_only_symbol(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    module: &str,
    exported: &str,
) -> bool {
    if il.kind(callee) != NodeKind::Var {
        return false;
    }
    let Some(local_name) = super::super::super::node_exact_name(il, interner, callee) else {
        return false;
    };
    let local_hash = stable_symbol_hash(local_name);
    let module_hash = stable_symbol_hash(module);
    let exported_hash = stable_symbol_hash(exported);
    let occurrence_span = il.node(callee).span;
    let mut found_matching = false;
    for record in il.evidence_binding_anchored(local_hash) {
        let EvidenceAnchor::Binding {
            span,
            local_hash: anchor_hash,
        } = record.anchor
        else {
            continue;
        };
        if span.file != occurrence_span.file || anchor_hash != local_hash {
            continue;
        }
        if !rust_import_binding_span_visible_at_call(il, span, occurrence_span) {
            continue;
        }
        let EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: actual_module,
            exported_hash: actual_exported,
        }) = record.kind
        else {
            continue;
        };
        if record.provenance.emitter != EvidenceEmitter::Builtin
            || record.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(record)
        {
            return false;
        }
        if actual_module == module_hash && actual_exported == exported_hash {
            found_matching = true;
        } else {
            return false;
        }
    }
    found_matching
}

fn rust_import_binding_span_visible_at_call(
    il: &nose_il::Il,
    binding_span: nose_il::Span,
    occurrence_span: nose_il::Span,
) -> bool {
    if binding_span.file != occurrence_span.file {
        return false;
    }
    if rust_span_inside_non_module_block_scope(il, binding_span) {
        return false;
    }
    rust_nearest_module_scope_containing_span(il, binding_span)
        == rust_nearest_module_scope_containing_span(il, occurrence_span)
}

fn rust_span_inside_non_module_block_scope(il: &nose_il::Il, span: nose_il::Span) -> bool {
    il.nodes.iter().any(|node| {
        matches!(
            node.kind,
            NodeKind::Block | NodeKind::Func | NodeKind::Lambda
        ) && node.span.file == span.file
            && node.span.start_byte <= span.start_byte
            && span.end_byte <= node.span.end_byte
            && (node.span.start_byte < span.start_byte || span.end_byte < node.span.end_byte)
    })
}

fn rust_nearest_module_scope_containing_span(
    il: &nose_il::Il,
    span: nose_il::Span,
) -> Option<NodeId> {
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.kind == NodeKind::Module
                && node.span.file == span.file
                && node.span.start_byte <= span.start_byte
                && span.end_byte <= node.span.end_byte
        })
        .min_by_key(|(_, node)| node.span.end_byte.saturating_sub(node.span.start_byte))
        .map(|(idx, _)| NodeId(idx as u32))
}

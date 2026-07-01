use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceKind, EvidenceStatus, Span, SymbolEvidenceKind,
};

pub(super) fn type_import_conflicted_at_span(
    il: &nose_il::Il,
    span: Span,
    type_name: &str,
) -> bool {
    let local_hash = stable_symbol_hash(type_name);
    let expected = SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash(super::JAVA_CONCURRENT_MODULE),
        exported_hash: stable_symbol_hash(type_name),
    };
    il.evidence.iter().any(|record| {
        matches!(
            record.anchor,
            EvidenceAnchor::Binding { span: import_span, local_hash: actual }
                if actual == local_hash
                    && import_span.file == span.file
                    && import_span.end_byte <= span.start_byte
        ) && matches!(record.kind, EvidenceKind::Symbol(actual) if actual != expected)
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

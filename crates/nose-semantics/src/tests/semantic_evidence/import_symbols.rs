use super::*;

fn import_fact_evidence(
    id: u32,
    lang: Lang,
    span: Span,
    kind: EvidenceKind,
    status: EvidenceStatus,
) -> EvidenceRecord {
    import_fact_evidence_with_provenance(
        id,
        span,
        kind,
        language_core_provenance(lang),
        status,
        Vec::new(),
    )
}

fn import_fact_evidence_with_provenance(
    id: u32,
    span: Span,
    kind: EvidenceKind,
    provenance: EvidenceProvenance,
    status: EvidenceStatus,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::sequence(span),
        kind,
        provenance,
        dependencies,
        status,
    }
}

fn language_core_provenance(lang: Lang) -> EvidenceProvenance {
    let (pack_id, producer_id) = language_core_evidence_provenance(lang);
    EvidenceProvenance {
        emitter: EvidenceEmitter::Builtin,
        pack_hash: Some(stable_symbol_hash(pack_id)),
        rule_hash: Some(stable_symbol_hash(producer_id)),
    }
}

fn binding_import_fact(module: &str, exported: &str) -> EvidenceKind {
    EvidenceKind::Import(ImportEvidenceKind::Binding {
        module_hash: stable_symbol_hash(module),
        exported_hash: stable_symbol_hash(exported),
    })
}

fn namespace_import_fact(module: &str) -> EvidenceKind {
    EvidenceKind::Import(ImportEvidenceKind::Namespace {
        module_hash: stable_symbol_hash(module),
    })
}

fn imported_literal_evidence_with_provenance(
    id: u32,
    span: Span,
    kind: EvidenceKind,
    provenance: EvidenceProvenance,
    status: EvidenceStatus,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(span, NodeKind::Seq),
        kind,
        provenance,
        dependencies,
        status,
    }
}

mod core;
mod occurrences;

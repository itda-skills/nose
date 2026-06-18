use super::*;

mod own_property;
mod qualified_global;
mod record_shape;

fn unshadowed_global_source_dependency(
    id: u32,
    span: Span,
    name: &str,
    status: EvidenceStatus,
) -> EvidenceRecord {
    evidence(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash(name),
        }),
        status,
    )
}

fn qualified_global_dependency(
    id: u32,
    span: Span,
    path: &str,
    status: EvidenceStatus,
    root_dependency: Option<u32>,
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal {
            path_hash: stable_symbol_hash(path),
        }),
        status,
        root_dependency
            .into_iter()
            .map(EvidenceId)
            .collect::<Vec<_>>(),
    )
}

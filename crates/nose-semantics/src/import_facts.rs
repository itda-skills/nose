//! Import evidence facts and imported literal provenance.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImportFactKind {
    Binding,
    Namespace,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ImportFactContract {
    pub kind: ImportFactKind,
    pub channel: ChannelEligibility,
}

pub fn import_fact_contract(kind: ImportFactKind) -> ImportFactContract {
    match kind {
        ImportFactKind::Binding => ImportFactContract {
            kind,
            channel: ChannelEligibility::ExactProven,
        },
        ImportFactKind::Namespace => ImportFactContract {
            kind,
            channel: ChannelEligibility::ExactProven,
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ImportFact {
    pub kind: ImportFactKind,
    pub module_hash: u64,
    pub exported_hash: Option<u64>,
}

pub(super) fn import_fact_evidence_at_sequence_span(
    il: &Il,
    span: Span,
) -> EvidenceResolution<ImportFact> {
    unique_evidence_at(
        il,
        span,
        |anchor| matches!(anchor, EvidenceAnchor::Sequence { span: anchor_span } if anchor_span == span),
        |evidence| match evidence {
            EvidenceKind::Import(ImportEvidenceKind::Binding {
                module_hash,
                exported_hash,
            }) => Some(ImportFact {
                kind: ImportFactKind::Binding,
                module_hash,
                exported_hash: Some(exported_hash),
            }),
            EvidenceKind::Import(ImportEvidenceKind::Namespace { module_hash }) => {
                Some(ImportFact {
                    kind: ImportFactKind::Namespace,
                    module_hash,
                    exported_hash: None,
                })
            }
            _ => None,
        },
    )
}

/// Evidence-only import fact resolution for semantic consumers. Import proof is
/// intentionally not encoded in the lowered `Seq` payload; callers rely on a
/// provider-owned evidence record, not on tag spelling.
pub fn import_fact_evidence_rhs(il: &Il, rhs: NodeId) -> Option<ImportFact> {
    if il.kind(rhs) != NodeKind::Seq {
        return None;
    }
    match import_fact_evidence_at_sequence_span(il, il.node(rhs).span) {
        EvidenceResolution::Found(fact) => Some(fact),
        EvidenceResolution::Ambiguous | EvidenceResolution::Missing => None,
    }
}

/// Prove that `span/kind` is a first-party imported-literal producer or copied
/// snapshot whose recorded dependencies are all asserted. This proof preserves a
/// provider-scope literal producer after cross-file replacement; consumers must
/// still check the expression shape/result contract they are about to build.
pub fn imported_literal_producer_evidence_at_span(il: &Il, span: Span, kind: NodeKind) -> bool {
    il.evidence_anchored_at(span).any(|record| {
        record.status == EvidenceStatus::Asserted
            && first_party_record(record)
            && record.anchor == EvidenceAnchor::node(span, kind)
            && matches!(
                record.kind,
                EvidenceKind::Import(
                    ImportEvidenceKind::ImmutableLiteralExport {
                        root_kind,
                        ..
                    } | ImportEvidenceKind::ImportedLiteralSnapshot {
                        root_kind,
                        ..
                    }
                ) if root_kind == kind
            )
            && il.evidence_dependencies_asserted(record)
    })
}

pub fn imported_literal_snapshot_evidence_at_span(il: &Il, span: Span, kind: NodeKind) -> bool {
    il.evidence_anchored_at(span).any(|record| {
        record.status == EvidenceStatus::Asserted
            && first_party_record(record)
            && record.anchor == EvidenceAnchor::node(span, kind)
            && matches!(
                record.kind,
                EvidenceKind::Import(ImportEvidenceKind::ImportedLiteralSnapshot {
                    root_kind,
                    ..
                }) if root_kind == kind
            )
            && il.evidence_dependencies_asserted(record)
    })
}

pub fn imported_literal_producer_evidence_for_node(il: &Il, node: NodeId) -> bool {
    imported_literal_producer_evidence_at_span(il, il.node(node).span, il.kind(node))
}

pub(super) fn first_party_record(record: &EvidenceRecord) -> bool {
    record.provenance.emitter == EvidenceEmitter::FirstParty
        && record.provenance.pack_hash == Some(stable_symbol_hash(FIRST_PARTY_PACK_ID))
}

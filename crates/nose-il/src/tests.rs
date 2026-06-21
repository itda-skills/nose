use super::*;

fn leaf_il() -> Il {
    let mut b = IlBuilder::new(FileId(0));
    let span = Span::new(FileId(0), 0, 1, 1, 1);
    let root = b.add(NodeKind::Module, Payload::None, span, &[]);
    b.finish(
        root,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn well_formed_il_validates() {
    assert!(leaf_il().validate().is_ok());
}

#[test]
fn dangling_child_is_caught() {
    let mut il = leaf_il();
    il.edges.push(NodeId(999)); // child id past the arena
    il.nodes[0].child_len = 1;
    assert!(il.validate().is_err(), "a dangling child id must fail");
}

#[test]
fn out_of_bounds_root_is_caught() {
    let mut il = leaf_il();
    il.root = NodeId(42);
    assert!(il.validate().is_err(), "an invalid root must fail");
}

#[test]
fn child_range_past_edges_is_caught() {
    let mut il = leaf_il();
    il.nodes[0].child_len = 5; // claims children that don't exist
    assert!(
        il.validate().is_err(),
        "an out-of-range child span must fail"
    );
}

#[test]
fn builtin_evidence_dedupe_preserves_provenance_boundary() {
    let mut il = leaf_il();
    let anchor = EvidenceAnchor::node(il.node(il.root).span, NodeKind::Module);
    let kind = EvidenceKind::Domain(DomainEvidence::Collection);
    il.evidence.push(EvidenceRecord {
        id: EvidenceId(0),
        anchor,
        kind,
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::External,
            pack_hash: Some(stable_symbol_hash("external.pack")),
            rule_hash: Some(stable_symbol_hash("external.rule")),
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    });

    let first =
        il.find_or_push_builtin_evidence(anchor, kind, "nose.first_party", "rule.a", Vec::new());
    let duplicate =
        il.find_or_push_builtin_evidence(anchor, kind, "nose.first_party", "rule.a", Vec::new());
    let different_rule =
        il.find_or_push_builtin_evidence(anchor, kind, "nose.first_party", "rule.b", Vec::new());

    assert_eq!(first, EvidenceId(1));
    assert_eq!(duplicate, first);
    assert_eq!(different_rule, EvidenceId(2));
}

#[test]
fn legacy_first_party_evidence_helper_alias_matches_builtin_helper() {
    let mut il = leaf_il();
    let anchor = EvidenceAnchor::node(il.node(il.root).span, NodeKind::Module);
    let kind = EvidenceKind::Domain(DomainEvidence::Collection);

    let builtin =
        il.find_or_push_builtin_evidence(anchor, kind, "nose.first_party", "rule.a", Vec::new());
    let legacy = il.find_or_push_first_party_evidence(
        anchor,
        kind,
        "nose.first_party",
        "rule.a",
        Vec::new(),
    );

    assert_eq!(legacy, builtin);
}

#[test]
fn builtin_evidence_emitter_keeps_legacy_wire_name() {
    assert_eq!(EvidenceEmitter::FirstParty, EvidenceEmitter::Builtin);
    assert_eq!(
        serde_json::to_string(&EvidenceEmitter::Builtin).unwrap(),
        "\"FirstParty\""
    );
    assert_eq!(
        serde_json::from_str::<EvidenceEmitter>("\"FirstParty\"").unwrap(),
        EvidenceEmitter::Builtin
    );
    assert_eq!(
        serde_json::from_str::<EvidenceEmitter>("\"Builtin\"").unwrap(),
        EvidenceEmitter::Builtin
    );
}

/// `clear()` + re-push rewrites the indexed prefix without shrinking below
/// the indexed length — the staleness sentinel must trigger a rebuild, not
/// serve buckets for records that no longer exist.
#[test]
fn evidence_index_survives_clear_and_repush() {
    let mut il = leaf_il();
    let span = il.node(il.root).span;
    let record = |id: u32, anchor| EvidenceRecord {
        id: EvidenceId(id),
        anchor,
        kind: EvidenceKind::Domain(DomainEvidence::Collection),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::Builtin,
            pack_hash: None,
            rule_hash: None,
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    };

    il.evidence
        .push(record(0, EvidenceAnchor::node(span, NodeKind::Module)));
    // Build the index, then invalidate it the rude way.
    assert_eq!(il.evidence_anchored_at(span).count(), 1);
    il.evidence.clear();
    il.evidence
        .push(record(0, EvidenceAnchor::binding(span, 7)));
    il.evidence
        .push(record(1, EvidenceAnchor::node(span, NodeKind::Module)));

    assert_eq!(il.evidence_anchored_at(span).count(), 2);
    assert_eq!(il.evidence_binding_anchored(7).count(), 1);
}

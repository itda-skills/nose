use super::*;

#[test]
fn sequence_surface_contracts_keep_value_and_exact_axes_separate() {
    let array = seq_surface_contract(Lang::JavaScript, Some("array")).unwrap();
    assert_eq!(array.value_tag, SEQ_VALUE_COLLECTION);
    assert!(array.exact_tree_safe);
    assert!(array.membership_collection);

    let untagged = seq_surface_contract(Lang::JavaScript, None).unwrap();
    assert_eq!(untagged.value_tag, SEQ_VALUE_UNTAGGED);
    assert!(!untagged.exact_tree_safe);
    assert!(!untagged.membership_collection);

    let object = seq_surface_contract(Lang::JavaScript, Some("object")).unwrap();
    assert_eq!(object.value_tag, SEQ_VALUE_MAP);
    assert!(object.exact_tree_safe);
    assert!(!object.membership_collection);
    assert!(object.imported_literal);
}

#[test]
fn go_sequence_surface_contracts_stay_language_scoped() {
    let go_map = seq_surface_contract(Lang::Go, Some("composite_literal")).unwrap();
    assert_eq!(
        go_map.value_tag,
        stable_symbol_hash("go_composite_map_literal")
    );
    assert!(!go_map.exact_tree_safe);
    assert!(!go_map.membership_collection);
    assert!(!go_map.imported_literal);

    let go_entry = seq_surface_contract(Lang::Go, Some("keyed_element")).unwrap();
    assert_eq!(go_entry.value_tag, stable_symbol_hash("keyed_element"));
    assert!(!go_entry.exact_tree_safe);
    assert!(!go_entry.membership_collection);

    assert!(seq_surface_contract(Lang::Python, Some("composite_literal")).is_none());
    assert!(seq_surface_contract(Lang::Python, Some("keyed_element")).is_none());
    assert!(imported_literal_seq_tag_safe(Lang::Python, "dictionary"));
    assert!(!imported_literal_seq_tag_safe(Lang::Ruby, "hash"));
}

#[test]
fn sequence_surface_evidence_must_match_the_lowered_surface() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let array = interner.intern("array");
    let seq = b.add(NodeKind::Seq, Payload::Name(array), sp(5), &[]);
    let root = b.add(NodeKind::Block, Payload::None, sp(5), &[seq]);
    let mut il = finish_il(b, root, Lang::JavaScript);

    assert_eq!(
        seq_surface_contract_for_node(&il, &interner, seq),
        None,
        "raw sequence tags do not prove semantic surfaces without evidence"
    );

    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(5)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    ));
    assert!(seq_surface_contract_for_node(&il, &interner, seq)
        .is_some_and(|contract| contract.membership_collection));

    il.evidence.push(language_core_evidence(
        1,
        EvidenceAnchor::sequence(sp(5)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    ));
    assert_eq!(seq_surface_contract_for_node(&il, &interner, seq), None);
}

#[test]
fn sequence_surface_evidence_requires_matching_language_core_provenance() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let array = interner.intern("array");
    let seq = b.add(NodeKind::Seq, Payload::Name(array), sp(15), &[]);
    let root = b.add(NodeKind::Block, Payload::None, sp(15), &[seq]);
    let mut il = finish_il(b, root, Lang::JavaScript);

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::sequence(sp(15)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        EvidenceStatus::Asserted,
    ));
    assert_eq!(
        seq_surface_contract_for_node(&il, &interner, seq),
        None,
        "legacy broad provenance must not prove a sequence surface"
    );

    il.evidence.clear();
    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(15)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        EvidenceStatus::Asserted,
        Lang::Python,
    ));
    assert_eq!(
        seq_surface_contract_for_node(&il, &interner, seq),
        None,
        "wrong-language builtin provenance must not prove a sequence surface"
    );

    il.evidence.clear();
    let mut external = language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(15)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    );
    external.provenance.emitter = EvidenceEmitter::External;
    il.evidence.push(external);
    assert_eq!(
        seq_surface_contract_for_node(&il, &interner, seq),
        None,
        "external provenance must not prove a builtin sequence surface"
    );

    il.evidence.clear();
    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(15)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    ));
    assert!(seq_surface_contract_for_node(&il, &interner, seq)
        .is_some_and(|contract| contract.membership_collection));
}

#[test]
fn imported_literal_export_safety_requires_sequence_evidence() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let object = interner.intern("object");
    let key = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("ready")),
        sp(6),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(6), &[]);
    let entry = b.add(NodeKind::Seq, Payload::Name(object), sp(6), &[key, value]);
    let root = b.add(NodeKind::Block, Payload::None, sp(6), &[entry]);
    let mut il = finish_il(b, root, Lang::JavaScript);

    assert!(!imported_literal_export_safe(&il, &interner, entry));

    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(6)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    ));
    assert!(imported_literal_export_safe(&il, &interner, entry));
}

#[test]
fn imported_literal_export_safety_rejects_import_coordinate_children() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let object = interner.intern("object");
    let imported = b.add(NodeKind::Seq, Payload::None, sp(7), &[]);
    let root_value = b.add(NodeKind::Seq, Payload::Name(object), sp(8), &[imported]);
    let root = b.add(NodeKind::Block, Payload::None, sp(8), &[root_value]);
    let mut il = finish_il(b, root, Lang::JavaScript);
    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(8)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map),
        EvidenceStatus::Asserted,
        Lang::JavaScript,
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::sequence(sp(7)),
        EvidenceKind::Import(ImportEvidenceKind::Binding {
            module_hash: stable_symbol_hash("provider"),
            exported_hash: stable_symbol_hash("VALUE"),
        }),
        EvidenceStatus::Asserted,
    ));

    assert!(!imported_literal_export_safe(&il, &interner, root_value));
}

#[test]
fn go_zero_map_surface_helpers_require_evidence() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let key = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("ready")),
        sp(32),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(32), &[]);
    let entry = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("keyed_element")),
        sp(32),
        &[key, value],
    );
    let map = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("composite_literal")),
        sp(31),
        &[entry],
    );
    let root = b.add(NodeKind::Block, Payload::None, sp(31), &[map]);
    let mut il = finish_il(b, root, Lang::Go);

    assert!(go_zero_map_literal_contract_for_node(&il, &interner, map).is_none());
    assert!(go_zero_map_entry_contract_for_node(&il, &interner, entry).is_none());

    il.evidence.push(language_core_evidence(
        0,
        EvidenceAnchor::sequence(sp(31)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::GoCompositeMapLiteral),
        EvidenceStatus::Asserted,
        Lang::Go,
    ));
    assert!(go_zero_map_literal_contract_for_node(&il, &interner, map).is_some());
    assert!(go_zero_map_entry_contract_for_node(&il, &interner, entry).is_none());

    il.evidence.push(language_core_evidence(
        1,
        EvidenceAnchor::sequence(sp(32)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::GoMapEntry),
        EvidenceStatus::Asserted,
        Lang::Go,
    ));
    assert!(go_zero_map_entry_contract_for_node(&il, &interner, entry).is_some());
}

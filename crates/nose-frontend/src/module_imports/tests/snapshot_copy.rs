use super::super::bindings::assignment_rhs;
use super::super::resolve_imported_immutable_bindings;
use super::super::snapshot::{append_snapshot, snapshot_subtree};
use super::support::{
    coordinate_import_binding_assignment, lookup_dict_provider, lookup_import_consumer,
    provider_with_lookup_export_evidence, test_provenance,
};
use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceId, EvidenceKind, EvidenceStatus, FileId, FileMeta,
    IlBuilder, ImportEvidenceKind, Interner, Lang, NodeKind, Payload, SequenceSurfaceKind, Span,
};

#[test]
fn snapshot_append_does_not_mint_import_or_symbol_evidence_from_coordinates() {
    let (provider, _interner, _span, assign, _rhs) =
        coordinate_import_binding_assignment(FileId(0), Lang::Java);
    let snapshot = snapshot_subtree(&provider, assign);

    let mut b = IlBuilder::new(FileId(1));
    let root_span = Span::new(FileId(1), 0, 0, 1, 1);
    let root = b.add(NodeKind::Module, Payload::None, root_span, &[]);
    let mut importer = b.finish(
        root,
        FileMeta {
            path: "consumer.java".into(),
            lang: Lang::Java,
        },
        Vec::new(),
        Vec::new(),
    );

    let appended = append_snapshot(&mut importer, &snapshot);
    assert!(
        appended.evidence.is_empty(),
        "snapshot append must copy provider evidence, not synthesize import facts from raw tags"
    );
    assert_eq!(importer.kind(appended.root), NodeKind::Assign);
}

#[test]
fn snapshot_append_copies_relevant_evidence_with_source_origin_spans() {
    let interner = Interner::new();
    let (provider, assign) = provider_with_lookup_export_evidence(&interner);
    let snapshot = snapshot_subtree(&provider, assign);

    let mut b = IlBuilder::new(FileId(1));
    let root_span = Span::new(FileId(1), 0, 0, 1, 1);
    let root = b.add(NodeKind::Module, Payload::None, root_span, &[]);
    let mut importer = b.finish(
        root,
        FileMeta {
            path: "consumer.py".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    let appended = append_snapshot(&mut importer, &snapshot);

    assert_eq!(
        importer.node(appended.root).span.file,
        FileId(0),
        "copied provider nodes keep provider source origin so importer-local scopes do not shadow them"
    );
    assert_eq!(appended.evidence.len(), 3);
    assert!(
        importer
            .evidence
            .iter()
            .all(|record| record.status == EvidenceStatus::Asserted),
        "snapshot append must not copy ambiguous evidence into asserted provenance dependencies"
    );
    let copied_surface = importer
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map)
            )
        })
        .unwrap();
    assert_eq!(
        copied_surface.anchor,
        EvidenceAnchor::sequence(Span::new(FileId(0), 4, 12, 1, 1))
    );
    assert_eq!(copied_surface.provenance, test_provenance("surface"));

    let copied_export = importer
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Import(ImportEvidenceKind::ImmutableLiteralExport { .. })
            )
        })
        .unwrap();
    assert_eq!(copied_export.dependencies, vec![copied_surface.id]);
}

#[test]
fn resolve_imported_literal_records_snapshot_provenance_dependencies() {
    let interner = Interner::new();
    let lookup = interner.intern("LOOKUP");
    let provider = lookup_dict_provider(&interner, lookup);
    let (importer, import_assign) = lookup_import_consumer(lookup);

    let mut files = vec![provider, importer];
    resolve_imported_immutable_bindings(&mut files, &interner);
    let replaced_rhs = assignment_rhs(&files[1], import_assign).unwrap();

    assert_eq!(files[1].kind(replaced_rhs), NodeKind::Seq);
    let provenance = files[1]
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Import(ImportEvidenceKind::ImportedLiteralSnapshot {
                    module_hash,
                    exported_hash,
                    root_kind: NodeKind::Seq,
                }) if module_hash == stable_symbol_hash("tables")
                    && exported_hash == stable_symbol_hash("LOOKUP")
            )
        })
        .unwrap();
    assert!(
        provenance.dependencies.contains(&EvidenceId(0)),
        "snapshot provenance must depend on the importer static import proof"
    );
    assert!(
        provenance.dependencies.iter().any(|id| {
            files[1].evidence.get(id.0 as usize).is_some_and(|record| {
                matches!(
                    record.kind,
                    EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map)
                )
            })
        }),
        "snapshot provenance must depend on copied provider surface evidence"
    );
    assert!(
        provenance.dependencies.iter().any(|id| {
            files[1].evidence.get(id.0 as usize).is_some_and(|record| {
                matches!(
                    record.kind,
                    EvidenceKind::Import(ImportEvidenceKind::ImmutableLiteralExport {
                        module_hash,
                        exported_hash,
                        root_kind: NodeKind::Seq,
                    }) if module_hash == stable_symbol_hash("tables")
                        && exported_hash == stable_symbol_hash("LOOKUP")
                )
            })
        }),
        "snapshot provenance must depend on copied provider export evidence"
    );
}

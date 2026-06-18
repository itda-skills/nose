use super::super::bindings::{assignment_name, assignment_rhs, collect_top_level_statements};
use super::support::{
    java_provider_and_importer, java_provider_and_importer_src,
    remove_library_api_evidence_by_rule, resolve_importer, resolve_snapshot_count, snapshot_count,
};
use nose_il::{
    stable_symbol_hash, EvidenceKind, FileId, Interner, Lang, NodeKind, SymbolEvidenceKind,
};
use nose_semantics::{
    library_api_contract_evidence_for_call, library_java_map_factory_contract,
    LibraryApiEvidenceStatus,
};

#[test]
fn java_map_provider_requires_library_api_evidence_for_snapshot() {
    let interner = Interner::new();
    let provider_src = "import java.util.Map;\nclass Tables { static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2); }\n";
    let (provider, importer) = java_provider_and_importer(provider_src, &interner);
    assert_eq!(
        resolve_snapshot_count(provider.clone(), importer.clone(), &interner),
        1
    );

    let mut missing_api = provider;
    remove_library_api_evidence_by_rule(&mut missing_api, "library_api_java_map_factory");
    assert_eq!(
        resolve_snapshot_count(missing_api, importer, &interner),
        0,
        "java.util import/symbol proof must not prove provider Map.of without LibraryApi evidence"
    );
}

#[test]
fn java_map_provider_snapshot_copies_library_api_dependency_closure() {
    let interner = Interner::new();
    let provider_src = "import java.util.Map;\nclass Tables { static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2); }\n";
    let (provider, importer) = java_provider_and_importer(provider_src, &interner);
    let importer = resolve_importer(provider, importer, &interner);
    let api_rule = stable_symbol_hash("library_api_java_map_factory");
    let api = importer
        .evidence
        .iter()
        .find(|record| {
            matches!(record.kind, EvidenceKind::LibraryApi(_))
                && record.provenance.rule_hash == Some(api_rule)
        })
        .expect("copied Java Map.of snapshot should retain LibraryApi evidence");

    let occurrence = api
        .dependencies
        .iter()
        .filter_map(|id| importer.evidence.get(id.0 as usize))
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
                    module_hash,
                    exported_hash,
                }) if module_hash == stable_symbol_hash("java.util")
                    && exported_hash == stable_symbol_hash("Map")
            )
        })
        .expect("copied LibraryApi evidence should keep its imported-symbol dependency");
    assert!(
        occurrence.dependencies.iter().any(|id| {
            importer.evidence.get(id.0 as usize).is_some_and(|record| {
                matches!(
                    record.kind,
                    EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
                        module_hash,
                        exported_hash,
                    }) if module_hash == stable_symbol_hash("java.util")
                        && exported_hash == stable_symbol_hash("Map")
                )
            })
        }),
        "copied occurrence proof should keep the provider binding-anchor dependency"
    );
}

#[test]
fn java_map_provider_snapshot_replaces_import_used_by_lookup_method() {
    let interner = Interner::new();
    let provider_src = "import java.util.Map;\nclass Tables { static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2); }\n";
    let importer_src = "import static Tables.LOOKUP;\nclass Consumer { static int lookup(String key, String other) { return LOOKUP.getOrDefault(key, 0); } }\n";
    let (provider, importer) =
        java_provider_and_importer_src(provider_src, importer_src, &interner);
    let importer = resolve_importer(provider, importer, &interner);
    assert_eq!(snapshot_count(&importer), 1);
    let import_stmt = collect_top_level_statements(&importer)
        .into_iter()
        .find(|&stmt| {
            assignment_name(&importer, stmt).is_some_and(|name| interner.resolve(name) == "LOOKUP")
        })
        .expect("static import assignment should remain as replacement anchor");
    let rhs = assignment_rhs(&importer, import_stmt).expect("import assignment rhs");
    assert_eq!(importer.kind(rhs), NodeKind::Call);
    assert_eq!(
            importer.node(rhs).span.file,
            FileId(0),
            "copied provider RHS keeps provider source origin so importer-local scopes cannot shadow it"
        );
    let contract =
        library_java_map_factory_contract(Lang::Java, "Map", "of").expect("Map.of contract");
    assert_eq!(
        library_api_contract_evidence_for_call(
            &importer,
            &interner,
            rhs,
            contract.id,
            contract.callee,
            4
        ),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn java_map_of_entries_provider_requires_outer_and_entry_library_api_evidence() {
    let interner = Interner::new();
    let provider_src = "import java.util.Map;\nclass Tables { static final Map<String, Integer> LOOKUP = Map.ofEntries(Map.entry(\"red\", 1), Map.entry(\"blue\", 2)); }\n";
    let (provider, importer) = java_provider_and_importer(provider_src, &interner);
    assert_eq!(
        resolve_snapshot_count(provider.clone(), importer.clone(), &interner),
        1
    );

    let mut missing_outer = provider.clone();
    remove_library_api_evidence_by_rule(&mut missing_outer, "library_api_java_map_factory");
    assert_eq!(
        resolve_snapshot_count(missing_outer, importer.clone(), &interner),
        0,
        "Map.ofEntries provider proof must require the outer LibraryApi evidence"
    );

    let mut missing_entry = provider;
    remove_library_api_evidence_by_rule(&mut missing_entry, "library_api_java_map_entry_factory");
    assert_eq!(
        resolve_snapshot_count(missing_entry, importer, &interner),
        0,
        "Map.ofEntries provider proof must require every nested Map.entry LibraryApi evidence"
    );
}

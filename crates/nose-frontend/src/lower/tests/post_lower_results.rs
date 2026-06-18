use super::*;

#[test]
fn post_lowering_emits_result_domains_for_supported_factories() {
    let interner = Interner::new();
    assert_python_factory_result_domains(&interner);
    assert_rust_and_ruby_factory_result_domains(&interner);
}

fn assert_python_factory_result_domains(interner: &Interner) {
    let py_list = lower_fixture(
        "builtin_list.py",
        b"def f(values):\n    return list(values)\n",
        Lang::Python,
        interner,
    );
    let list_contract =
        library_free_name_collection_factory_contract(Lang::Python, "list").unwrap();
    let list_api = contract_api_ids(&py_list.evidence, list_contract.id, list_contract.callee);
    assert!(result_domain_depends_on_any_api(
        &py_list.evidence,
        DomainEvidence::Collection,
        &list_api,
    ));

    let py_set = lower_fixture(
        "builtin_set.py",
        b"def f(values):\n    return set(values)\n",
        Lang::Python,
        interner,
    );
    let set_contract = library_free_name_collection_factory_contract(Lang::Python, "set").unwrap();
    let set_api = contract_api_ids(&py_set.evidence, set_contract.id, set_contract.callee);
    assert!(result_domain_depends_on_any_api(
        &py_set.evidence,
        DomainEvidence::Set,
        &set_api,
    ));

    let shadowed_py = lower_fixture(
        "shadowed.py",
        b"def f(list, values):\n    return list(values)\n",
        Lang::Python,
        interner,
    );
    assert_eq!(
        result_domain_record_count(&shadowed_py.evidence, DomainEvidence::Collection),
        0,
        "shadowed list(...) must not emit result-domain evidence"
    );
}

fn assert_rust_and_ruby_factory_result_domains(interner: &Interner) {
    let rust_vec = lower_fixture(
        "vec.rs",
        b"fn f() { let xs = Vec::new(); }",
        Lang::Rust,
        interner,
    );
    let vec_contract = library_rust_vec_new_factory_contract(Lang::Rust, "Vec::new").unwrap();
    let vec_api = contract_api_ids(&rust_vec.evidence, vec_contract.id, vec_contract.callee);
    assert!(result_domain_depends_on_any_api(
        &rust_vec.evidence,
        DomainEvidence::Collection,
        &vec_api,
    ));

    let rust_map = lower_fixture(
        "hash_map.rs",
        b"fn f() { let xs = std::collections::HashMap::from([(\"red\", 1)]); }",
        Lang::Rust,
        interner,
    );
    let map_contract =
        library_free_name_map_factory_contract(Lang::Rust, "std::collections::HashMap::from")
            .unwrap();
    let map_api = contract_api_ids(&rust_map.evidence, map_contract.id, map_contract.callee);
    assert!(result_domain_depends_on_any_api(
        &rust_map.evidence,
        DomainEvidence::Map,
        &map_api,
    ));

    let ruby = lower_fixture(
        "set.rb",
        b"require \"set\"\n\ndef f(values)\n  Set.new(values)\nend\n",
        Lang::Ruby,
        interner,
    );
    let ruby_contract = library_ruby_set_factory_contract(Lang::Ruby, "Set", "new", 1).unwrap();
    let ruby_api = contract_api_ids(&ruby.evidence, ruby_contract.id, ruby_contract.callee);
    assert!(result_domain_depends_on_any_api(
        &ruby.evidence,
        DomainEvidence::Set,
        &ruby_api,
    ));

    let missing_require = lower_fixture(
        "set_missing_require.rb",
        b"def f(values)\n  Set.new(values)\nend\n",
        Lang::Ruby,
        interner,
    );
    assert_eq!(
        result_domain_record_count(&missing_require.evidence, DomainEvidence::Set),
        0,
        "Ruby Set.new must not emit result-domain evidence without require proof"
    );
}

#[test]
fn result_domain_evidence_requires_live_library_api_dependency() {
    let interner = Interner::new();
    let mut il = crate::lower_source(
        FileId(0),
        "set.js",
        b"function f(value) { return new Set([value]).has(value); }",
        Lang::JavaScript,
        &interner,
    )
    .expect("js lowering should succeed");
    let call = call_node_with_result_domain(&il, DomainEvidence::Set)
        .expect("new Set result should carry Set domain evidence");
    assert_eq!(
        nose_semantics::domain_evidence_for_node(&il, call),
        Some(DomainEvidence::Set)
    );

    for record in &mut il.evidence {
        if matches!(record.kind, EvidenceKind::LibraryApi(_)) {
            record.status = EvidenceStatus::Ambiguous;
        }
    }
    assert_eq!(
        nose_semantics::domain_evidence_for_node(&il, call),
        None,
        "receiver-domain proof must close when the LibraryApi dependency is ambiguous"
    );
}

#[test]
fn java_empty_collection_constructor_emits_occurrence_evidence() {
    let interner = Interner::new();
    let il = crate::lower_source(
        FileId(0),
        "C.java",
        b"import java.util.ArrayList;\nclass C { Object f() { return new ArrayList<>(); } }\n",
        Lang::Java,
        &interner,
    )
    .expect("java lowering should succeed");
    let contract =
        library_java_collection_constructor_contract(Lang::Java, "ArrayList", 0).unwrap();
    let api = library_api_evidence_ids_in_records(
        &il.evidence,
        library_api_contract_id_hash(contract.id),
        library_api_callee_contract_hash(contract.callee),
    );
    assert_eq!(api.len(), 1);
    assert!(
        il.evidence.iter().any(|record| {
            record.kind == EvidenceKind::Domain(DomainEvidence::Collection)
                && record.dependencies.len() == 1
                && api.contains(&record.dependencies[0])
        }),
        "Java constructor result-domain evidence must depend on the LibraryApi occurrence"
    );
}

#[test]
fn java_empty_collection_constructor_wildcard_import_is_dependency_backed() {
    let interner = Interner::new();
    let wildcard = crate::lower_source(
        FileId(0),
        "C.java",
        b"import java.util.*;\nclass C { Object f() { return new ArrayList<>(); } }\n",
        Lang::Java,
        &interner,
    )
    .expect("java lowering should succeed");
    let contract =
        library_java_collection_constructor_contract(Lang::Java, "ArrayList", 0).unwrap();
    let api = wildcard
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
                    contract_hash,
                    callee_hash,
                    ..
                }) if contract_hash == library_api_contract_id_hash(contract.id)
                    && callee_hash == library_api_callee_contract_hash(contract.callee)
            )
        })
        .expect("wildcard java.util import should admit supported ArrayList constructor");
    assert!(api.dependencies.iter().any(|id| {
        wildcard.evidence_record_by_id(*id).is_some_and(|record| {
            matches!(
                record.kind,
                EvidenceKind::Import(ImportEvidenceKind::Wildcard { module_hash })
                    if module_hash == stable_symbol_hash("java.util")
            )
        })
    }));

    let shadowed = crate::lower_source(
            FileId(0),
            "C.java",
            b"import java.util.*;\nclass ArrayList<T> {}\nclass C { Object f() { return new ArrayList<>(); } }\n",
            Lang::Java,
            &interner,
        )
        .expect("java lowering should succeed");
    assert_eq!(
        library_api_evidence_count_in_records(
            &shadowed.evidence,
            library_api_contract_id_hash(contract.id),
            library_api_callee_contract_hash(contract.callee),
        ),
        0,
        "local ArrayList type must close the java.util constructor occurrence"
    );

    let explicit_conflict = crate::lower_source(
            FileId(0),
            "C.java",
            b"import java.util.*;\nimport other.ArrayList;\nclass C { Object f() { return new ArrayList<>(); } }\n",
            Lang::Java,
            &interner,
        )
        .expect("java lowering should succeed");
    assert_eq!(
        library_api_evidence_count_in_records(
            &explicit_conflict.evidence,
            library_api_contract_id_hash(contract.id),
            library_api_callee_contract_hash(contract.callee),
        ),
        0,
        "explicit same-name imports must close java.util wildcard constructor proof"
    );
}

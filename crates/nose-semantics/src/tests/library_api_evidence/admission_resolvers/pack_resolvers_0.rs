use super::*;

#[test]
fn imported_namespace_receiver_dependency_rejects_conflicting_symbol_identity() {
    let (mut il, interner, call, fmt) = go_fmt_println_call_il();
    let contract =
        library_method_call_contract(Lang::Go, "Println", 1).expect("Go fmt.Println contract");
    let fmt_symbol = SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash("fmt"),
    };
    il.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::binding(sp(63), stable_symbol_hash("fmt")),
        fmt_symbol,
        EvidenceStatus::Asserted,
        &[],
        Lang::Go,
    ));
    il.evidence.push(language_core_symbol_record(
        1,
        EvidenceAnchor::node(il.node(fmt).span, NodeKind::Var),
        fmt_symbol,
        EvidenceStatus::Asserted,
        &[0],
        Lang::Go,
    ));
    assert_eq!(
        library_api_receiver_dependencies_for_call(&il, &interner, call, contract.callee),
        Some(vec![EvidenceId(1)])
    );

    il.evidence.push(language_core_symbol_record(
        9,
        EvidenceAnchor::node(il.node(fmt).span, NodeKind::Var),
        SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("log"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Go,
    ));
    assert!(
        library_api_receiver_dependencies_for_call(&il, &interner, call, contract.callee).is_none(),
        "conflicting same-anchor language-core namespace evidence must close receiver dependency generation"
    );
}

#[test]
fn admitted_library_api_call_resolvers_require_evidence() {
    let (il, interner, call, _callee) = rust_some_call_il();
    assert!(
        admitted_rust_option_some_constructor_at_call(&il, &interner, call).is_none(),
        "raw free-name call shape alone must not admit a library API occurrence"
    );

    let contract = library_rust_option_some_constructor_contract(Lang::Rust, "Some", 1)
        .expect("Rust Some constructor contract");
    let (mut missing_dependency, interner, call, _callee) = rust_some_call_il();
    missing_dependency.evidence.push(rust_stdlib_option_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        1,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span API occurrence without its callee dependency is still rejected"
    );

    let (mut wrong_pack, interner, call, callee) = rust_some_call_il();
    wrong_pack.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    wrong_pack.evidence.push(library_api_record_with_provenance(
        1,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
        BUILTIN_COMPAT_PACK_ID,
        RUST_STDLIB_OPTION_PRODUCER_ID,
    ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust Option Some evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_some_call_il();
    wrong_producer.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            1,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0],
            RUST_STDLIB_OPTION_PACK_ID,
            "wrong.rust.stdlib.option-api",
        ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust Option Some evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, callee) = rust_some_call_il();
    wrong_emitter.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_emitter.node(callee).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    let mut external_record = rust_stdlib_option_record(
        1,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        1,
        EvidenceStatus::Asserted,
        &[0],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_rust_option_some_constructor_at_call(&wrong_emitter, &interner, call).is_none(),
        "Rust Option Some evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_some_call_il();
    admitted.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    admitted.evidence.push(rust_stdlib_option_record(
        1,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        1,
        EvidenceStatus::Asserted,
        &[0],
    ));

    let occurrence =
        admitted_rust_option_some_constructor_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::RustOptionSomeConstructor
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_node_resolvers_require_api_occurrence_evidence() {
    let (il, interner, field, _receiver) = js_length_field_il();
    assert!(
        admitted_property_builtin_at_field(&il, &interner, field).is_none(),
        "raw JS length field shape alone must not admit property builtin semantics"
    );

    let contract =
        library_property_builtin_contract(Lang::JavaScript, "length").expect("length contract");
    let (mut missing_dependency, interner, field, _receiver) = js_length_field_il();
    missing_dependency
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            0,
            &missing_dependency,
            field,
            contract.id,
            contract.callee,
            0,
            &[],
            PROPERTY_BUILTIN_PROTOCOL_PACK_ID,
            PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID,
        ));
    assert!(
        admitted_property_builtin_at_field(&missing_dependency, &interner, field).is_none(),
        "property API occurrence without receiver-domain dependency is rejected"
    );

    let (mut wrong_pack, interner, field, receiver) = js_length_field_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Collection),
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(asserted_library_api_node_record(
        1,
        &wrong_pack,
        field,
        contract.id,
        contract.callee,
        0,
        &[0],
    ));
    assert!(
        admitted_property_builtin_at_field(&wrong_pack, &interner, field).is_none(),
        "legacy broad property API occurrence evidence is rejected"
    );

    let (mut admitted, interner, field, receiver) = js_length_field_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Collection),
        EvidenceStatus::Asserted,
    ));
    admitted
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            1,
            &admitted,
            field,
            contract.id,
            contract.callee,
            0,
            &[0],
            PROPERTY_BUILTIN_PROTOCOL_PACK_ID,
            PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID,
        ));
    let resolved = admitted_property_builtin_at_field(&admitted, &interner, field).unwrap();
    assert_eq!(
        resolved.contract.id,
        LibraryApiContractId::PropertyBuiltin(Builtin::Len)
    );
    assert_eq!(resolved.contract.result, Builtin::Len);
    assert_eq!(resolved.node, field);
    assert_eq!(resolved.receiver, Some(receiver));
    assert_eq!(resolved.arg_count, 0);

    let (il, interner, _call, callee) = rust_some_call_il();
    assert!(
        admitted_rust_option_some_constructor_at_node(&il, &interner, callee).is_none(),
        "raw Rust Some callee node alone must not admit constructor semantics"
    );

    let some_contract = library_rust_option_some_constructor_contract(Lang::Rust, "Some", 1)
        .expect("Rust Some constructor contract");
    let (mut missing_dependency, interner, _call, callee) = rust_some_call_il();
    missing_dependency
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            0,
            &missing_dependency,
            callee,
            some_contract.id,
            some_contract.callee,
            1,
            &[],
            RUST_STDLIB_OPTION_PACK_ID,
            RUST_STDLIB_OPTION_PRODUCER_ID,
        ));
    assert!(
        admitted_rust_option_some_constructor_at_node(&missing_dependency, &interner, callee)
            .is_none(),
        "Some constructor node occurrence without symbol dependency is rejected"
    );

    let (mut admitted, interner, _call, callee) = rust_some_call_il();
    admitted.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    admitted
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            1,
            &admitted,
            callee,
            some_contract.id,
            some_contract.callee,
            1,
            &[0],
            RUST_STDLIB_OPTION_PACK_ID,
            RUST_STDLIB_OPTION_PRODUCER_ID,
        ));
    let resolved =
        admitted_rust_option_some_constructor_at_node(&admitted, &interner, callee).unwrap();
    assert_eq!(
        resolved.contract.id,
        LibraryApiContractId::RustOptionSomeConstructor
    );
    assert_eq!(resolved.node, callee);
    assert_eq!(resolved.receiver, None);
    assert_eq!(resolved.arg_count, 1);
}

#[test]
fn admitted_rust_option_none_sentinel_resolver_requires_pack_provenance() {
    let (il, interner, none) = rust_none_node_il();
    assert!(
        admitted_rust_option_none_sentinel_at_node(&il, &interner, none).is_none(),
        "raw Rust None node alone must not admit Option sentinel semantics"
    );

    let contract = library_rust_option_none_sentinel_contract(Lang::Rust, "None")
        .expect("Rust None sentinel contract");
    let (mut wrong_pack, interner, none) = rust_none_node_il();
    wrong_pack.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_pack.node(none).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    wrong_pack
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            1,
            &wrong_pack,
            none,
            contract.id,
            contract.callee,
            0,
            &[0],
            BUILTIN_COMPAT_PACK_ID,
            RUST_STDLIB_OPTION_PRODUCER_ID,
        ));
    assert!(
        admitted_rust_option_none_sentinel_at_node(&wrong_pack, &interner, none).is_none(),
        "Rust Option None evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, none) = rust_none_node_il();
    wrong_producer.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_producer.node(none).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    wrong_producer
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            1,
            &wrong_producer,
            none,
            contract.id,
            contract.callee,
            0,
            &[0],
            RUST_STDLIB_OPTION_PACK_ID,
            "wrong.rust.stdlib.option-api",
        ));
    assert!(
        admitted_rust_option_none_sentinel_at_node(&wrong_producer, &interner, none).is_none(),
        "Rust Option None evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, none) = rust_none_node_il();
    wrong_emitter.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(wrong_emitter.node(none).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    let mut external_record = asserted_library_api_node_record_with_provenance(
        1,
        &wrong_emitter,
        none,
        contract.id,
        contract.callee,
        0,
        &[0],
        RUST_STDLIB_OPTION_PACK_ID,
        RUST_STDLIB_OPTION_PRODUCER_ID,
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_rust_option_none_sentinel_at_node(&wrong_emitter, &interner, none).is_none(),
        "Rust Option None evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, none) = rust_none_node_il();
    admitted.evidence.push(language_core_symbol_record(
        0,
        EvidenceAnchor::node(admitted.node(none).span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        },
        EvidenceStatus::Asserted,
        &[],
        Lang::Rust,
    ));
    admitted
        .evidence
        .push(asserted_library_api_node_record_with_provenance(
            1,
            &admitted,
            none,
            contract.id,
            contract.callee,
            0,
            &[0],
            RUST_STDLIB_OPTION_PACK_ID,
            RUST_STDLIB_OPTION_PRODUCER_ID,
        ));
    assert_eq!(
        admitted_rust_option_none_sentinel_at_node(&admitted, &interner, none)
            .expect("Rust None should admit")
            .id,
        LibraryApiContractId::RustOptionNoneSentinel
    );
}

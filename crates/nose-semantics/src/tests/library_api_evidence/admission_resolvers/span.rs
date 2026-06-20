use super::*;

fn rust_map_get_call_il() -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(72), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("get")),
        sp(73),
        &[receiver],
    );
    let key = b.add(NodeKind::Var, Payload::Cid(1), sp(74), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(75), &[callee, key]);
    let root = b.add(NodeKind::Func, Payload::None, sp(76), &[call]);
    (
        finish_il(b, root, Lang::Rust),
        interner,
        call,
        callee,
        receiver,
    )
}

#[test]
fn admitted_span_resolver_requires_api_occurrence_evidence() {
    let (il, interner, call, callee, receiver) = rust_map_get_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(il.node(call).span),
        callee_span: Some(il.node(callee).span),
        receiver_span: Some(il.node(receiver).span),
        arg_count: 1,
    };
    assert!(
        admitted_map_get_at_call_span(&il, &interner, occurrence, stable_symbol_hash("get"))
            .is_none(),
        "raw Rust map.get(...) value-level span shape alone must not admit map-get semantics"
    );

    let contract = library_map_get_contract(Lang::Rust, "get", 1).expect("Rust map get contract");
    let (mut missing_dependency, interner, call, callee, receiver) = rust_map_get_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(missing_dependency.node(call).span),
        callee_span: Some(missing_dependency.node(callee).span),
        receiver_span: Some(missing_dependency.node(receiver).span),
        arg_count: 1,
    };
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_map_get_at_call_span(
            &missing_dependency,
            &interner,
            occurrence,
            stable_symbol_hash("get")
        )
        .is_none(),
        "span-backed map-get API occurrence without receiver-domain dependency is rejected"
    );

    let (mut admitted, interner, call, callee, receiver) = rust_map_get_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(admitted.node(call).span),
        callee_span: Some(admitted.node(callee).span),
        receiver_span: Some(admitted.node(receiver).span),
        arg_count: 1,
    };
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Map),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(library_api_record(
        1,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    ));

    let resolved =
        admitted_map_get_at_call_span(&admitted, &interner, occurrence, stable_symbol_hash("get"))
            .unwrap();
    assert_eq!(resolved.contract.id, LibraryApiContractId::MapGet);
    assert_eq!(resolved.call_span, Some(admitted.node(call).span));
    assert_eq!(resolved.callee_span, Some(admitted.node(callee).span));
    assert_eq!(resolved.receiver_span, Some(admitted.node(receiver).span));
    assert_eq!(resolved.arg_count, 1);
}

#[test]
fn admitted_span_factory_resolver_requires_import_backed_api_occurrence() {
    let interner = Interner::new();
    let (mut raw, call, _root, _local, _contract) =
        java_list_of_import_evidence_il(&interner, true);
    let callee = raw.children(call)[0];
    let receiver = raw.children(callee)[0];
    let occurrence = LibraryApiSpanCall {
        call_span: Some(raw.node(call).span),
        callee_span: Some(raw.node(callee).span),
        receiver_span: Some(raw.node(receiver).span),
        arg_count: 1,
    };
    raw.evidence.clear();
    assert!(
        admitted_java_collection_factory_at_call_span(
            &raw,
            &interner,
            occurrence,
            stable_symbol_hash("of")
        )
        .is_none(),
        "raw Java List.of(...) value-level span shape alone must not admit factory semantics"
    );

    let (mut missing_dependency, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    let callee = missing_dependency.children(call)[0];
    let receiver = missing_dependency.children(callee)[0];
    let occurrence = LibraryApiSpanCall {
        call_span: Some(missing_dependency.node(call).span),
        callee_span: Some(missing_dependency.node(callee).span),
        receiver_span: Some(missing_dependency.node(receiver).span),
        arg_count: 1,
    };
    missing_dependency.evidence.clear();
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_java_collection_factory_at_call_span(
            &missing_dependency,
            &interner,
            occurrence,
            stable_symbol_hash("of")
        )
        .is_none(),
        "span-backed Java List.of API occurrence without import dependency is rejected"
    );

    let (admitted, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    let callee = admitted.children(call)[0];
    let receiver = admitted.children(callee)[0];
    let occurrence = LibraryApiSpanCall {
        call_span: Some(admitted.node(call).span),
        callee_span: Some(admitted.node(callee).span),
        receiver_span: Some(admitted.node(receiver).span),
        arg_count: 1,
    };
    let resolved = admitted_java_collection_factory_at_call_span(
        &admitted,
        &interner,
        occurrence,
        stable_symbol_hash("of"),
    )
    .unwrap();
    assert_eq!(resolved.contract.id, contract.id);
    assert_eq!(resolved.contract.callee, contract.callee);
    assert_eq!(resolved.call_span, Some(admitted.node(call).span));
    assert_eq!(resolved.callee_span, Some(admitted.node(callee).span));
    assert_eq!(resolved.receiver_span, Some(admitted.node(receiver).span));
    assert_eq!(resolved.arg_count, 1);
}

#[test]
fn admitted_span_rust_std_map_factory_requires_pack_provenance() {
    let (mut wrong_pack, interner, call, callee) = rust_std_map_factory_call_il();
    let contract =
        library_free_name_map_factory_contract(Lang::Rust, "std::collections::HashMap::from")
            .expect("Rust HashMap::from contract");
    let occurrence = LibraryApiSpanCall {
        call_span: Some(wrong_pack.node(call).span),
        callee_span: Some(wrong_pack.node(callee).span),
        receiver_span: None,
        arg_count: 1,
    };
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashMap::from"),
        }),
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(library_api_record_with_provenance(
        1,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
        FIRST_PARTY_PACK_ID,
        RUST_STDLIB_MAP_FACTORY_PRODUCER_ID,
    ));
    assert!(
        admitted_free_name_map_factory_at_call_span(&wrong_pack, &interner, occurrence, |name| {
            name == "std::collections::HashMap::from"
        })
        .is_none(),
        "span-backed Rust map factory evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_std_map_factory_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(wrong_producer.node(call).span),
        callee_span: Some(wrong_producer.node(callee).span),
        receiver_span: None,
        arg_count: 1,
    };
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashMap::from"),
        }),
        EvidenceStatus::Asserted,
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
            RUST_STDLIB_MAP_FACTORY_PACK_ID,
            "wrong.rust.stdlib.map-factory-api",
        ));
    assert!(
        admitted_free_name_map_factory_at_call_span(
            &wrong_producer,
            &interner,
            occurrence,
            |name| name == "std::collections::HashMap::from",
        )
        .is_none(),
        "span-backed Rust map factory evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_std_map_factory_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(admitted.node(call).span),
        callee_span: Some(admitted.node(callee).span),
        receiver_span: None,
        arg_count: 1,
    };
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashMap::from"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(rust_stdlib_map_factory_record(
        1,
        admitted.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[0],
    ));
    let resolved =
        admitted_free_name_map_factory_at_call_span(&admitted, &interner, occurrence, |name| {
            name == "std::collections::HashMap::from"
        })
        .unwrap();
    assert_eq!(
        resolved.contract.id,
        LibraryApiContractId::RustStdMapFactory
    );
    assert_eq!(resolved.call_span, Some(admitted.node(call).span));
    assert_eq!(resolved.callee_span, Some(admitted.node(callee).span));
    assert_eq!(resolved.receiver_span, None);
    assert_eq!(resolved.arg_count, 1);
}

#[test]
fn admitted_span_java_map_factory_requires_pack_provenance() {
    let (mut wrong_pack, interner, call, callee, receiver) = java_map_factory_call_il();
    let contract =
        library_java_map_factory_contract(Lang::Java, "Map", "of").expect("Map.of contract");
    let occurrence = LibraryApiSpanCall {
        call_span: Some(wrong_pack.node(call).span),
        callee_span: Some(wrong_pack.node(callee).span),
        receiver_span: Some(wrong_pack.node(receiver).span),
        arg_count: 2,
    };
    push_java_map_import_dependencies(&mut wrong_pack, receiver);
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            2,
            EvidenceStatus::Asserted,
            &[1],
            FIRST_PARTY_PACK_ID,
            JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID,
        ));
    assert!(
        admitted_java_map_factory_at_call_span(
            &wrong_pack,
            &interner,
            occurrence,
            stable_symbol_hash("of"),
        )
        .is_none(),
        "span-backed Java Map.of evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee, receiver) = java_map_factory_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(wrong_producer.node(call).span),
        callee_span: Some(wrong_producer.node(callee).span),
        receiver_span: Some(wrong_producer.node(receiver).span),
        arg_count: 2,
    };
    push_java_map_import_dependencies(&mut wrong_producer, receiver);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            2,
            EvidenceStatus::Asserted,
            &[1],
            JAVA_STDLIB_MAP_FACTORY_PACK_ID,
            "wrong.java.stdlib.map-factory-api",
        ));
    assert!(
        admitted_java_map_factory_at_call_span(
            &wrong_producer,
            &interner,
            occurrence,
            stable_symbol_hash("of"),
        )
        .is_none(),
        "span-backed Java Map.of evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee, receiver) = java_map_factory_call_il();
    let occurrence = LibraryApiSpanCall {
        call_span: Some(admitted.node(call).span),
        callee_span: Some(admitted.node(callee).span),
        receiver_span: Some(admitted.node(receiver).span),
        arg_count: 2,
    };
    push_java_map_import_dependencies(&mut admitted, receiver);
    admitted.evidence.push(java_stdlib_map_factory_record(
        2,
        admitted.node(call).span,
        contract,
        2,
        EvidenceStatus::Asserted,
        &[1],
    ));
    let resolved = admitted_java_map_factory_at_call_span(
        &admitted,
        &interner,
        occurrence,
        stable_symbol_hash("of"),
    )
    .unwrap();
    assert_eq!(
        resolved.contract.id,
        LibraryApiContractId::JavaMapFactory(JavaMapFactoryKind::Of)
    );
    assert_eq!(resolved.call_span, Some(admitted.node(call).span));
    assert_eq!(resolved.callee_span, Some(admitted.node(callee).span));
    assert_eq!(resolved.receiver_span, Some(admitted.node(receiver).span));
    assert_eq!(resolved.arg_count, 2);
}

#[test]
fn admitted_span_imported_collection_factory_rejects_namespace_dependency_for_bare_callee() {
    let (mut il, interner, call, callee) = python_deque_factory_call_il();
    let namespace_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash("collections"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(61), stable_symbol_hash("Values")),
        namespace_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Var),
        namespace_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    let contract =
        library_imported_collection_factory_contract(Lang::Python, "collections", "deque")
            .expect("Python collections.deque factory contract");
    il.evidence.push(python_stdlib_collection_factory_record(
        2,
        il.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence = LibraryApiSpanCall {
        call_span: Some(il.node(call).span),
        callee_span: Some(il.node(callee).span),
        receiver_span: None,
        arg_count: 1,
    };
    assert!(
        admitted_imported_collection_factory_at_call_span(&il, &interner, occurrence).is_none(),
        "bare imported collection factory calls require an imported-binding dependency, not a namespace dependency"
    );
}

#[test]
fn admitted_span_imported_collection_factory_rejects_receiver_span_without_field_callee() {
    let (mut il, interner, call, callee) = python_deque_factory_call_il();
    let namespace_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash("collections"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(61), stable_symbol_hash("Values")),
        namespace_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Var),
        namespace_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    let contract =
        library_imported_collection_factory_contract(Lang::Python, "collections", "deque")
            .expect("Python collections.deque factory contract");
    il.evidence.push(python_stdlib_collection_factory_record(
        2,
        il.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence = LibraryApiSpanCall {
        call_span: Some(il.node(call).span),
        callee_span: Some(il.node(callee).span),
        receiver_span: Some(il.node(callee).span),
        arg_count: 1,
    };
    assert!(
        admitted_imported_collection_factory_at_call_span(&il, &interner, occurrence).is_none(),
        "namespace dependency admission requires a field callee span for the exported member"
    );
}

#[test]
fn admitted_span_imported_collection_factory_rejects_unrelated_namespace_receiver_span() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let other = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("other")),
        sp(70),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("deque")),
        sp(71),
        &[other],
    );
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(72), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(73), &[callee, arg]);
    let unrelated_namespace = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("collections")),
        sp(74),
        &[],
    );
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(75),
        &[unrelated_namespace, call],
    );
    let mut il = finish_il(b, root, Lang::Python);

    let namespace_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash("collections"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(69), stable_symbol_hash("collections")),
        namespace_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(unrelated_namespace).span, NodeKind::Var),
        namespace_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    let contract =
        library_imported_collection_factory_contract(Lang::Python, "collections", "deque")
            .expect("Python collections.deque factory contract");
    il.evidence.push(python_stdlib_collection_factory_record(
        2,
        il.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence = LibraryApiSpanCall {
        call_span: Some(il.node(call).span),
        callee_span: Some(il.node(callee).span),
        receiver_span: Some(il.node(unrelated_namespace).span),
        arg_count: 1,
    };
    assert!(
        admitted_imported_collection_factory_at_call_span(&il, &interner, occurrence).is_none(),
        "namespace dependency admission requires the queried receiver span to be the field callee receiver"
    );
}

use super::receiver_method_result_domains::receiver_method_result_domain_il;
use super::*;
use nose_semantics::{
    library_free_name_collection_factory_contract, library_free_name_map_factory_contract,
    library_imported_promise_factory_contract, library_map_get_contract,
    library_method_call_contract, library_rust_result_ok_constructor_contract,
    library_rust_result_predicate_contract, library_swift_map_factory_contract,
    JS_NODE_TIMERS_PROMISES_PACK_ID, JS_NODE_TIMERS_PROMISES_PRODUCER_ID,
    PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID, RUST_STDLIB_MAP_FACTORY_PRODUCER_ID,
    RUST_STDLIB_RESULT_PRODUCER_ID, SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
};

fn free_name_call_il(lang: Lang, name: &str, arg_count: usize) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut builder = IlBuilder::new(FileId(0));
    let callee = builder.add(
        NodeKind::Var,
        Payload::Name(interner.intern(name)),
        sp(10),
        &[],
    );
    let mut children = vec![callee];
    for idx in 0..arg_count {
        children.push(builder.add(
            NodeKind::Var,
            Payload::Cid((idx + 1) as u32),
            sp(11 + idx as u32),
            &[],
        ));
    }
    let call = builder.add(NodeKind::Call, Payload::None, sp(20), &children);
    let root = builder.add(NodeKind::Func, Payload::None, sp(21), &[call]);
    (
        builder.finish(
            root,
            FileMeta {
                path: "call-result-domain".into(),
                lang,
            },
            Vec::new(),
            Vec::new(),
        ),
        interner,
        call,
        callee,
    )
}

fn push_unshadowed_symbol(il: &mut Il, node: NodeId, name: &str) -> EvidenceId {
    let (pack_id, producer_id) = language_core_evidence_provenance(il.meta.lang);
    il.find_or_push_builtin_evidence(
        EvidenceAnchor::node(il.node(node).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash(name),
        }),
        pack_id,
        producer_id,
        Vec::new(),
    )
}

fn call_domain_records(il: &Il, call: NodeId) -> Vec<&EvidenceRecord> {
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    il.evidence_anchored_at(anchor.span())
        .filter(|record| record.anchor == anchor && matches!(record.kind, EvidenceKind::Domain(_)))
        .collect()
}

fn imported_promise_factory_call_il(
    local: &str,
    arg_count: usize,
) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut builder = IlBuilder::new(FileId(0));
    let callee = builder.add(
        NodeKind::Var,
        Payload::Name(interner.intern(local)),
        sp(50),
        &[],
    );
    let mut children = vec![callee];
    for idx in 0..arg_count {
        children.push(builder.add(
            NodeKind::Var,
            Payload::Cid(idx as u32),
            sp(51 + idx as u32),
            &[],
        ));
    }
    let call = builder.add(NodeKind::Call, Payload::None, sp(60), &children);
    let root = builder.add(NodeKind::Func, Payload::None, sp(61), &[call]);
    (
        builder.finish(
            root,
            FileMeta {
                path: "node-timers-promises".into(),
                lang: Lang::TypeScript,
            },
            Vec::new(),
            Vec::new(),
        ),
        interner,
        call,
        callee,
    )
}

fn push_imported_binding_symbol(
    il: &mut Il,
    local: &str,
    module: &str,
    exported: &str,
) -> EvidenceId {
    let (pack_id, producer_id) = language_core_evidence_provenance(il.meta.lang);
    il.find_or_push_builtin_evidence(
        EvidenceAnchor::binding(sp(49), stable_symbol_hash(local)),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash(module),
            exported_hash: stable_symbol_hash(exported),
        }),
        pack_id,
        producer_id,
        Vec::new(),
    )
}

#[test]
fn preexisting_collection_api_records_materialize_result_domains() {
    for (name, expected_domain) in [
        ("list", DomainEvidence::Collection),
        ("set", DomainEvidence::Set),
    ] {
        let (mut il, interner, call, callee) = free_name_call_il(Lang::Python, name, 1);
        let contract = library_free_name_collection_factory_contract(Lang::Python, name).unwrap();
        let symbol = push_unshadowed_symbol(&mut il, callee, name);
        let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
        let api = upsert_builtin_evidence_with_pack_id(
            &mut il,
            anchor,
            EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
                contract_hash: library_api_contract_id_hash(contract.id),
                callee_hash: library_api_callee_contract_hash(contract.callee),
                arity: 1,
            }),
            contract.pack_id,
            PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
            vec![symbol],
        );

        run(&mut il, &interner);

        let result_domains = node_domain_records(&il, call, expected_domain);
        assert_eq!(result_domains.len(), 1, "{name} should materialize domain");
        assert_eq!(
            result_domains[0].provenance,
            language_core_provenance(Lang::Python)
        );
        assert_eq!(result_domains[0].dependencies, vec![api]);
    }
}

#[test]
fn preexisting_map_api_records_materialize_result_domains() {
    let name = "std::collections::HashMap::from";
    let (mut il, interner, call, callee) = free_name_call_il(Lang::Rust, name, 1);
    let contract = library_free_name_map_factory_contract(Lang::Rust, name).unwrap();
    let symbol = push_unshadowed_symbol(&mut il, callee, name);
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    let api = upsert_builtin_evidence_with_pack_id(
        &mut il,
        anchor,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 1,
        }),
        contract.pack_id,
        RUST_STDLIB_MAP_FACTORY_PRODUCER_ID,
        vec![symbol],
    );

    run(&mut il, &interner);

    let result_domains = node_domain_records(&il, call, DomainEvidence::Map);
    assert_eq!(result_domains.len(), 1);
    assert_eq!(
        result_domains[0].provenance,
        language_core_provenance(Lang::Rust)
    );
    assert_eq!(result_domains[0].dependencies, vec![api]);
}

#[test]
fn imported_node_timers_promise_factory_materializes_promise_like_domain() {
    let (mut il, interner, call, callee) = imported_promise_factory_call_il("delay", 2);
    let binding =
        push_imported_binding_symbol(&mut il, "delay", "node:timers/promises", "setTimeout");
    let contract = library_imported_promise_factory_contract(
        Lang::TypeScript,
        "node:timers/promises",
        "setTimeout",
        2,
    )
    .expect("node:timers/promises setTimeout contract");

    run(&mut il, &interner);

    let api = library_api_records(&il, call)
        .into_iter()
        .find(|record| record.status == EvidenceStatus::Asserted)
        .expect("node timers Promise factory API evidence");
    assert_eq!(
        api.provenance,
        pack_provenance(
            JS_NODE_TIMERS_PROMISES_PACK_ID,
            JS_NODE_TIMERS_PROMISES_PRODUCER_ID,
        )
    );
    assert_eq!(
        api.kind,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 2,
        })
    );
    let occurrence = il
        .evidence_record_by_id(api.dependencies[0])
        .expect("imported binding occurrence dependency");
    assert_eq!(
        occurrence.anchor,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Var)
    );
    assert_eq!(occurrence.dependencies, vec![binding]);

    let result_domains = node_domain_records(&il, call, DomainEvidence::PromiseLike);
    assert_eq!(result_domains.len(), 1);
    assert_eq!(
        result_domains[0].provenance,
        language_core_provenance(Lang::TypeScript)
    );
    assert_eq!(result_domains[0].dependencies, vec![api.id]);
}

#[test]
fn imported_node_timers_promise_factory_requires_import_proof_and_supported_arity() {
    let (mut raw, interner, call, _) = imported_promise_factory_call_il("delay", 2);
    run(&mut raw, &interner);
    assert!(
        library_api_records(&raw, call).is_empty(),
        "raw delay(...) spelling must not prove node timers Promise factory"
    );
    assert!(
        node_domain_records(&raw, call, DomainEvidence::PromiseLike).is_empty(),
        "PromiseLike domain requires admitted imported API evidence"
    );

    let (mut too_many_args, interner, call, _) = imported_promise_factory_call_il("delay", 4);
    push_imported_binding_symbol(
        &mut too_many_args,
        "delay",
        "node:timers/promises",
        "setTimeout",
    );
    run(&mut too_many_args, &interner);
    assert!(
        library_api_records(&too_many_args, call).is_empty(),
        "unsupported setTimeout arity must stay closed"
    );
    assert!(
        node_domain_records(&too_many_args, call, DomainEvidence::PromiseLike).is_empty(),
        "unsupported arity must not materialize PromiseLike domain"
    );
}

#[test]
fn preexisting_swift_map_api_records_do_not_materialize_arity_only_domains() {
    let interner = Interner::new();
    let mut builder = IlBuilder::new(FileId(0));
    let callee = builder.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Dictionary")),
        sp(10),
        &[],
    );
    let values = builder.add(
        NodeKind::Var,
        Payload::Name(interner.intern("values")),
        sp(11),
        &[],
    );
    let kwarg = builder.add(
        NodeKind::KwArg,
        Payload::Name(interner.intern("uniqueKeysWithValues")),
        sp(12),
        &[values],
    );
    let call = builder.add(NodeKind::Call, Payload::None, sp(20), &[callee, kwarg]);
    let root = builder.add(NodeKind::Func, Payload::None, sp(21), &[call]);
    let mut il = builder.finish(
        root,
        FileMeta {
            path: "swift-call-result-domain".into(),
            lang: Lang::Swift,
        },
        Vec::new(),
        Vec::new(),
    );
    let contract =
        library_swift_map_factory_contract(Lang::Swift, "Dictionary", "uniqueKeysWithValues")
            .unwrap();
    let symbol = push_unshadowed_symbol(&mut il, callee, "Dictionary");
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    upsert_builtin_evidence_with_pack_id(
        &mut il,
        anchor,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 1,
        }),
        contract.pack_id,
        SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
        vec![symbol],
    );

    run(&mut il, &interner);

    assert!(
        node_domain_records(&il, call, DomainEvidence::Map).is_empty(),
        "Swift Dictionary result domains require call-shape proof beyond arity"
    );
}

#[test]
fn result_domain_materialization_excludes_hof_and_map_get_lanes() {
    let (mut hof_il, hof_interner, hof_call, _) = receiver_method_result_domain_il(
        Lang::JavaScript,
        "receiver",
        "map",
        1,
        Some(DomainEvidence::Array),
    );

    run(&mut hof_il, &hof_interner);

    assert!(
        asserted(library_api_records(&hof_il, hof_call)).len() == 1,
        "HOF API occurrence should still be admitted for downstream protocol consumers"
    );
    assert!(
        call_domain_records(&hof_il, hof_call).is_empty(),
        "HOF compatibility fallback must not materialize call result DomainEvidence"
    );

    let (mut get_il, get_interner, get_call, _) = receiver_method_result_domain_il(
        Lang::Rust,
        "receiver",
        "get",
        1,
        Some(DomainEvidence::Map),
    );

    run(&mut get_il, &get_interner);

    assert!(
        asserted(library_api_records(&get_il, get_call)).len() == 1,
        "Map.get API occurrence should still be admitted"
    );
    assert!(
        call_domain_records(&get_il, get_call).is_empty(),
        "Map.get value semantics must not materialize a map/option result domain"
    );
}

#[test]
fn result_domain_materialization_requires_admitted_api_evidence() {
    let (mut il, interner, call, callee) = free_name_call_il(Lang::Python, "list", 1);
    let contract = library_free_name_collection_factory_contract(Lang::Python, "list").unwrap();
    let symbol = push_unshadowed_symbol(&mut il, callee, "list");
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    upsert_builtin_evidence_with_pack_id(
        &mut il,
        anchor,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 2,
        }),
        contract.pack_id,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
        vec![symbol],
    );

    run(&mut il, &interner);

    assert!(
        node_domain_records(&il, call, DomainEvidence::Collection).is_empty(),
        "wrong-arity LibraryApi evidence must not materialize result-domain evidence"
    );
}

#[test]
fn locally_recorded_result_constructor_domain_closes_on_conflicting_api_evidence() {
    let (mut il, interner, call, callee) = free_name_call_il(Lang::Rust, "Ok", 1);
    let contract = library_rust_result_ok_constructor_contract(Lang::Rust, "Ok", 1).unwrap();
    let symbol = push_unshadowed_symbol(&mut il, callee, "Ok");
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    upsert_builtin_evidence_with_pack_id(
        &mut il,
        anchor,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 2,
        }),
        contract.pack_id,
        RUST_STDLIB_RESULT_PRODUCER_ID,
        vec![symbol],
    );

    run(&mut il, &interner);

    assert_eq!(
        asserted(library_api_records(&il, call)).len(),
        2,
        "the local recorder still records the correct constructor occurrence"
    );
    assert!(
        node_domain_records(&il, call, DomainEvidence::Result).is_empty(),
        "conflicting same-anchor LibraryApi evidence must keep result-domain materialization closed"
    );
}

#[test]
fn locally_recorded_receiver_method_domain_closes_on_conflicting_api_evidence() {
    let (mut il, interner, call, _) = receiver_method_result_domain_il(
        Lang::Rust,
        "result",
        "is_ok",
        0,
        Some(DomainEvidence::Result),
    );
    let conflict = library_rust_result_predicate_contract(Lang::Rust, "is_err", 0).unwrap();
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    upsert_builtin_evidence_with_pack_id(
        &mut il,
        anchor,
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(conflict.id),
            callee_hash: library_api_callee_contract_hash(conflict.callee),
            arity: 0,
        }),
        conflict.pack_id,
        conflict.rule,
        Vec::new(),
    );

    run(&mut il, &interner);

    assert_eq!(
        asserted(library_api_records(&il, call)).len(),
        2,
        "the local recorder still records the correct receiver-method occurrence"
    );
    assert!(
        node_domain_records(&il, call, DomainEvidence::Boolean).is_empty(),
        "conflicting same-anchor LibraryApi evidence must keep receiver result-domain materialization closed"
    );
}

#[test]
fn hard_negative_contracts_have_no_materialized_result_domain_mapping() {
    let hof = library_method_call_contract(Lang::JavaScript, "map", 1).unwrap();
    assert_eq!(
        nose_semantics::library_api_materialized_result_domain_for_arity(hof.id, hof.callee, 1),
        None
    );
    let get = library_map_get_contract(Lang::Rust, "get", 1).unwrap();
    assert_eq!(
        nose_semantics::library_api_materialized_result_domain_for_arity(get.id, get.callee, 1),
        None
    );
}

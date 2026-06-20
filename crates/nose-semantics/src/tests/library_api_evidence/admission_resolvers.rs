use super::*;

mod promise;
mod span;

fn asserted_library_api_node_record(
    id: u32,
    il: &Il,
    node: NodeId,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: &[u32],
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::node(il.node(node).span, il.kind(node)),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract_id),
            callee_hash: library_api_callee_contract_hash(callee),
            arity,
        }),
        EvidenceStatus::Asserted,
        dependencies.iter().copied().map(EvidenceId).collect(),
    )
}

fn js_length_field_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(42), &[]);
    let field = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("length")),
        sp(43),
        &[receiver],
    );
    let root = b.add(NodeKind::Func, Payload::None, sp(44), &[field]);
    (
        finish_il(b, root, Lang::JavaScript),
        interner,
        field,
        receiver,
    )
}

fn rust_some_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Some")),
        sp(45),
        &[],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(46), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(47), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(48), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, callee)
}

fn rust_vec_new_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Vec::new")),
        sp(49),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(50), &[callee]);
    let root = b.add(NodeKind::Func, Payload::None, sp(51), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, callee)
}

fn rust_vec_macro_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("vec")),
        sp(52),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(53), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(54), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(55), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, callee)
}

fn python_list_factory_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("list")),
        sp(58),
        &[],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(59), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(60), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(61), &[call]);
    (finish_il(b, root, Lang::Python), interner, call, callee)
}

fn python_deque_factory_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Values")),
        sp(62),
        &[],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(63), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(64), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(65), &[call]);
    (finish_il(b, root, Lang::Python), interner, call, callee)
}

fn ruby_set_factory_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let require = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("require")),
        sp(70),
        &[],
    );
    let require_arg = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("set")),
        sp(71),
        &[],
    );
    let require_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(72),
        &[require, require_arg],
    );
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Set")),
        sp(73),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("new")),
        sp(74),
        &[receiver],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(75), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(76), &[callee, value]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(77),
        &[require_call, call],
    );
    (finish_il(b, root, Lang::Ruby), interner, call, receiver)
}

fn push_ruby_set_require_dependencies(il: &mut Il, receiver: NodeId) {
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(il.node(receiver).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Set"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(70), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("require"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        2,
        EvidenceAnchor::source_span(span(70, 72, 1)),
        EvidenceKind::Import(ImportEvidenceKind::Require {
            module_hash: stable_symbol_hash("set"),
        }),
        EvidenceStatus::Asserted,
        vec![EvidenceId(1)],
    ));
}

fn python_len_builtin_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("len")),
        sp(66),
        &[],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(67), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(68), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(69), &[call]);
    (finish_il(b, root, Lang::Python), interner, call, callee)
}

fn java_arrays_stream_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("Arrays");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(66), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(66), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(66), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(67), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("stream")),
        sp(68),
        &[receiver],
    );
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(69), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(70), &[callee, arg]);
    let root = b.add(NodeKind::Module, Payload::None, sp(71), &[import, call]);
    (finish_il(b, root, Lang::Java), interner, call, receiver)
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
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span API occurrence without its callee dependency is still rejected"
    );

    let (mut admitted, interner, call, callee) = rust_some_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        }),
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
        .push(asserted_library_api_node_record(
            0,
            &missing_dependency,
            field,
            contract.id,
            contract.callee,
            0,
            &[],
        ));
    assert!(
        admitted_property_builtin_at_field(&missing_dependency, &interner, field).is_none(),
        "property API occurrence without receiver-domain dependency is rejected"
    );

    let (mut admitted, interner, field, receiver) = js_length_field_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Collection),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(asserted_library_api_node_record(
        1,
        &admitted,
        field,
        contract.id,
        contract.callee,
        0,
        &[0],
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
        .push(asserted_library_api_node_record(
            0,
            &missing_dependency,
            callee,
            some_contract.id,
            some_contract.callee,
            1,
            &[],
        ));
    assert!(
        admitted_rust_option_some_constructor_at_node(&missing_dependency, &interner, callee)
            .is_none(),
        "Some constructor node occurrence without symbol dependency is rejected"
    );

    let (mut admitted, interner, _call, callee) = rust_some_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(asserted_library_api_node_record(
        1,
        &admitted,
        callee,
        some_contract.id,
        some_contract.callee,
        1,
        &[0],
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
fn admitted_free_function_builtin_resolver_requires_api_occurrence_evidence() {
    let (il, interner, call, _callee) = python_len_builtin_call_il();
    assert!(
        admitted_free_function_builtin_at_call(&il, &interner, call).is_none(),
        "raw Python len(...) call shape alone must not admit builtin semantics"
    );

    let contract = library_free_function_builtin_contract(Lang::Python, "len", 1)
        .expect("Python len contract");
    let (mut missing_dependency, interner, call, _callee) = python_len_builtin_call_il();
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_free_function_builtin_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span builtin API occurrence without callee dependency is rejected"
    );

    let (mut admitted, interner, call, callee) = python_len_builtin_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("len"),
        }),
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

    let occurrence = admitted_free_function_builtin_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(occurrence.contract.id, contract.id);
    assert_eq!(occurrence.contract.result.builtin, Builtin::Len);
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_collection_factory_resolver_requires_api_occurrence_evidence() {
    let (il, interner, call, _callee) = python_list_factory_call_il();
    assert!(
        admitted_free_name_collection_factory_at_call(&il, &interner, call).is_none(),
        "raw Python list(...) call shape alone must not admit collection factory semantics"
    );

    let contract = library_free_name_collection_factory_contract(Lang::Python, "list")
        .expect("Python list factory contract");
    let (mut missing_dependency, interner, call, _callee) = python_list_factory_call_il();
    missing_dependency
        .evidence
        .push(python_builtin_collection_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_free_name_collection_factory_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span collection factory evidence without callee dependency is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = python_list_factory_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("list"),
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
        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    assert!(
        admitted_free_name_collection_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Python builtin collection factory evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = python_list_factory_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("list"),
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
            PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
            "wrong.python.builtin.collection-factory-api",
        ));
    assert!(
        admitted_free_name_collection_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Python builtin collection factory evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = python_list_factory_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("list"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted
        .evidence
        .push(python_builtin_collection_factory_record(
            1,
            admitted.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[0],
        ));

    let occurrence =
        admitted_free_name_collection_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::PythonBuiltinCollectionFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_rust_vec_new_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = rust_vec_new_call_il();
    assert!(
        admitted_rust_vec_new_factory_at_call(&il, &interner, call).is_none(),
        "raw Rust Vec::new() call shape alone must not admit stdlib Vec semantics"
    );

    let contract =
        library_rust_vec_new_factory_contract(Lang::Rust, "Vec::new").expect("Vec::new contract");

    let (mut missing_dependency, interner, call, _callee) = rust_vec_new_call_il();
    missing_dependency.evidence.push(rust_stdlib_vec_record(
        0,
        missing_dependency.node(call).span,
        contract,
        0,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_rust_vec_new_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Vec::new evidence without callee dependency is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = rust_vec_new_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Vec::new"),
        }),
        EvidenceStatus::Asserted,
    ));
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            1,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            0,
            EvidenceStatus::Asserted,
            &[0],
            FIRST_PARTY_PACK_ID,
            RUST_STDLIB_VEC_PRODUCER_ID,
        ));
    assert!(
        admitted_rust_vec_new_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust Vec::new evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_vec_new_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Vec::new"),
        }),
        EvidenceStatus::Asserted,
    ));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            1,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            0,
            EvidenceStatus::Asserted,
            &[0],
            RUST_STDLIB_VEC_PACK_ID,
            "wrong.rust.stdlib.vec-factory-api",
        ));
    assert!(
        admitted_rust_vec_new_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust Vec::new evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_vec_new_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Vec::new"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(rust_stdlib_vec_record(
        1,
        admitted.node(call).span,
        contract,
        0,
        EvidenceStatus::Asserted,
        &[0],
    ));

    let occurrence = admitted_rust_vec_new_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::RustVecNewFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 0);
}

#[test]
fn admitted_rust_vec_macro_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = rust_vec_macro_call_il();
    assert!(
        admitted_rust_vec_macro_factory_at_call(&il, &interner, call).is_none(),
        "raw Rust vec! macro call shape alone must not admit stdlib Vec semantics"
    );

    let contract =
        library_rust_vec_macro_factory_contract(Lang::Rust, "vec").expect("vec! contract");

    let (mut missing_dependency, interner, call, _callee) = rust_vec_macro_call_il();
    missing_dependency.evidence.push(rust_stdlib_vec_record(
        0,
        missing_dependency.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_rust_vec_macro_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span vec! evidence without macro/source dependencies is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = rust_vec_macro_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(wrong_pack.node(call).span),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::MacroInvocation)),
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(evidence(
        1,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("vec"),
        }),
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(library_api_record_with_provenance(
        2,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 1],
        FIRST_PARTY_PACK_ID,
        RUST_STDLIB_VEC_PRODUCER_ID,
    ));
    assert!(
        admitted_rust_vec_macro_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust vec! evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_vec_macro_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(wrong_producer.node(call).span),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::MacroInvocation)),
        EvidenceStatus::Asserted,
    ));
    wrong_producer.evidence.push(evidence(
        1,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("vec"),
        }),
        EvidenceStatus::Asserted,
    ));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 1],
            RUST_STDLIB_VEC_PACK_ID,
            "wrong.rust.stdlib.vec-factory-api",
        ));
    assert!(
        admitted_rust_vec_macro_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust vec! evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_vec_macro_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(admitted.node(call).span),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::MacroInvocation)),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(evidence(
        1,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("vec"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(rust_stdlib_vec_record(
        2,
        admitted.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[0, 1],
    ));

    let occurrence = admitted_rust_vec_macro_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::RustVecMacroFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_imported_collection_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = python_deque_factory_call_il();
    assert!(
        admitted_imported_collection_factory_at_call(&il, &interner, call).is_none(),
        "raw imported deque(...) call shape alone must not admit collection factory semantics"
    );

    let contract =
        library_imported_collection_factory_contract(Lang::Python, "collections", "deque")
            .expect("Python collections.deque factory contract");
    let imported_binding = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("collections"),
        exported_hash: stable_symbol_hash("deque"),
    });

    let (mut missing_dependency, interner, call, _callee) = python_deque_factory_call_il();
    missing_dependency
        .evidence
        .push(python_stdlib_collection_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_imported_collection_factory_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span collections.deque evidence without import dependency is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = python_deque_factory_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(61), stable_symbol_hash("Values")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    wrong_pack.evidence.push(library_api_record_with_provenance(
        2,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1],
        FIRST_PARTY_PACK_ID,
        PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    assert!(
        admitted_imported_collection_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Python stdlib collection factory evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = python_deque_factory_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(61), stable_symbol_hash("Values")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_producer.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[1],
            PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
            "wrong.python.stdlib.collection-factory-api",
        ));
    assert!(
        admitted_imported_collection_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Python stdlib collection factory evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = python_deque_factory_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(61), stable_symbol_hash("Values")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    admitted
        .evidence
        .push(python_stdlib_collection_factory_record(
            2,
            admitted.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[1],
        ));

    let occurrence =
        admitted_imported_collection_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::PythonImportedCollectionFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_ruby_set_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _receiver) = ruby_set_factory_call_il();
    assert!(
        admitted_ruby_set_factory_at_call(&il, &interner, call).is_none(),
        "raw Ruby Set.new(...) call shape alone must not admit stdlib Set semantics"
    );

    let contract =
        library_ruby_set_factory_contract(Lang::Ruby, "Set", "new", 1).expect("Set.new contract");

    let (mut missing_dependency, interner, call, _receiver) = ruby_set_factory_call_il();
    missing_dependency.evidence.push(ruby_stdlib_set_record(
        0,
        missing_dependency.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_ruby_set_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Ruby Set.new evidence without Set/require dependencies is rejected"
    );

    let (mut wrong_pack, interner, call, receiver) = ruby_set_factory_call_il();
    push_ruby_set_require_dependencies(&mut wrong_pack, receiver);
    wrong_pack.evidence.push(library_api_record_with_provenance(
        3,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 2],
        FIRST_PARTY_PACK_ID,
        RUBY_STDLIB_SET_PRODUCER_ID,
    ));
    assert!(
        admitted_ruby_set_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Ruby Set.new evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = ruby_set_factory_call_il();
    push_ruby_set_require_dependencies(&mut wrong_producer, receiver);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            3,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 2],
            RUBY_STDLIB_SET_PACK_ID,
            "wrong.ruby.stdlib.set-factory-api",
        ));
    assert!(
        admitted_ruby_set_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Ruby Set.new evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, receiver) = ruby_set_factory_call_il();
    push_ruby_set_require_dependencies(&mut admitted, receiver);
    admitted.evidence.push(ruby_stdlib_set_record(
        3,
        admitted.node(call).span,
        contract,
        EvidenceStatus::Asserted,
        &[0, 2],
    ));

    let occurrence = admitted_ruby_set_factory_at_call(&admitted, &interner, call).unwrap();
    let field_callee = admitted.children(call)[0];
    assert_eq!(occurrence.contract.id, LibraryApiContractId::RubySetFactory);
    assert_eq!(occurrence.callee, field_callee);
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_static_collection_adapter_resolver_requires_import_backed_api_occurrence_evidence() {
    let (il, interner, call, _receiver) = java_arrays_stream_call_il();
    assert!(
        admitted_static_collection_adapter_at_call(&il, &interner, call).is_none(),
        "raw Java Arrays.stream(...) call shape alone must not admit adapter semantics"
    );

    let contract = library_static_collection_adapter_contract(Lang::Java, "Arrays", "stream", 1)
        .expect("Java Arrays.stream contract");
    let (mut missing_dependency, interner, call, _receiver) = java_arrays_stream_call_il();
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_static_collection_adapter_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Java static adapter evidence without import dependency is rejected"
    );

    let (mut admitted, interner, call, receiver) = java_arrays_stream_call_il();
    let imported_binding = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("Arrays"),
    });
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("Arrays")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    admitted.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(admitted.node(receiver).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    admitted.evidence.push(library_api_record(
        2,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence =
        admitted_static_collection_adapter_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(occurrence.contract.id, contract.id);
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 1);
}

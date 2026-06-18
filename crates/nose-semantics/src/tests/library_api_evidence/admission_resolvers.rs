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

fn python_len_builtin_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("len")),
        sp(62),
        &[],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(63), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(64), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(65), &[call]);
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
    missing_dependency.evidence.push(library_api_record(
        0,
        missing_dependency.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_free_name_collection_factory_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span collection factory evidence without callee dependency is rejected"
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
    admitted.evidence.push(library_api_record(
        1,
        admitted.node(call).span,
        contract.id,
        contract.callee,
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

use super::*;

mod array;
mod promise;
mod regex;
mod span;
mod static_index;

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

fn asserted_library_api_node_record_with_provenance(
    id: u32,
    il: &Il,
    node: NodeId,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: &[u32],
    pack_id: &str,
    rule: &str,
) -> EvidenceRecord {
    let mut record =
        asserted_library_api_node_record(id, il, node, contract_id, callee, arity, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(pack_id));
    record.provenance.rule_hash = Some(stable_symbol_hash(rule));
    record
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

fn rust_none_node_il() -> (Il, Interner, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let none = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("None")),
        sp(49),
        &[],
    );
    let root = b.add(NodeKind::Func, Payload::None, sp(50), &[none]);
    (finish_il(b, root, Lang::Rust), interner, none)
}

fn rust_option_and_then_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(51), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("and_then")),
        sp(52),
        &[receiver],
    );
    let callback = b.add(NodeKind::Func, Payload::None, sp(53), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(54), &[callee, callback]);
    let root = b.add(NodeKind::Func, Payload::None, sp(55), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, receiver)
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

fn rust_std_collection_factory_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("std::collections::HashSet::from")),
        sp(56),
        &[],
    );
    let value = b.add(NodeKind::Seq, Payload::None, sp(57), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(58), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(59), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, callee)
}

fn rust_std_map_factory_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("std::collections::HashMap::from")),
        sp(66),
        &[],
    );
    let value = b.add(NodeKind::Seq, Payload::None, sp(67), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(68), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(69), &[call]);
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

fn python_math_prod_call_il() -> (Il, Interner, NodeId, NodeId) {
    python_math_prod_call_il_with_arg_count(1)
}

fn python_math_prod_call_il_with_arg_count(arg_count: usize) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let math = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("math")),
        sp(66),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("prod")),
        sp(67),
        &[math],
    );
    let mut children = vec![callee];
    for idx in 0..arg_count {
        children.push(b.add(
            NodeKind::Var,
            Payload::Cid(idx as u32),
            sp(68 + idx as u32),
            &[],
        ));
    }
    let call = b.add(NodeKind::Call, Payload::None, sp(70), &children);
    let root = b.add(NodeKind::Func, Payload::None, sp(71), &[call]);
    (finish_il(b, root, Lang::Python), interner, call, math)
}

fn push_python_math_namespace_dependencies(il: &mut Il, receiver: NodeId) {
    let namespace_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash("math"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("math")),
        namespace_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(receiver).span, NodeKind::Var),
        namespace_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
}

fn java_arrays_stream_call_il() -> (Il, Interner, NodeId, NodeId) {
    java_arrays_stream_call_il_with_arg_count(1)
}

fn java_arrays_stream_call_il_with_arg_count(arg_count: usize) -> (Il, Interner, NodeId, NodeId) {
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
    let mut children = vec![callee];
    for idx in 0..arg_count {
        children.push(b.add(
            NodeKind::Var,
            Payload::Cid(idx as u32),
            sp(69 + idx as u32),
            &[],
        ));
    }
    let call = b.add(NodeKind::Call, Payload::None, sp(70), &children);
    let root = b.add(NodeKind::Module, Payload::None, sp(71), &[import, call]);
    (finish_il(b, root, Lang::Java), interner, call, receiver)
}

fn java_map_factory_call_il() -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("Map");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(80), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(80), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(80), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(81), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("of")),
        sp(82),
        &[receiver],
    );
    let key = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("red")),
        sp(83),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(84), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(85), &[callee, key, value]);
    let root = b.add(NodeKind::Module, Payload::None, sp(86), &[import, call]);
    (
        finish_il(b, root, Lang::Java),
        interner,
        call,
        callee,
        receiver,
    )
}

fn java_map_entry_call_il() -> (Il, Interner, NodeId, NodeId, NodeId) {
    java_map_entry_call_il_with_arg_count(2)
}

fn java_map_entry_call_il_with_arg_count(
    arg_count: usize,
) -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("Map");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(80), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(80), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(80), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(81), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("entry")),
        sp(82),
        &[receiver],
    );
    let key = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("red")),
        sp(83),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(84), &[]);
    let extra = b.add(NodeKind::Lit, Payload::LitInt(2), sp(85), &[]);
    let mut children = vec![callee, key, value];
    if arg_count > 2 {
        children.push(extra);
    }
    let call = b.add(NodeKind::Call, Payload::None, sp(86), &children);
    let root = b.add(NodeKind::Module, Payload::None, sp(86), &[import, call]);
    (
        finish_il(b, root, Lang::Java),
        interner,
        call,
        callee,
        receiver,
    )
}

fn java_collection_constructor_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("ArrayList")),
        sp(87),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(88), &[callee]);
    let root = b.add(NodeKind::Module, Payload::None, sp(89), &[call]);
    (finish_il(b, root, Lang::Java), interner, call, callee)
}

fn js_global_constructor_call_il(receiver: &str) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern(receiver)),
        sp(90),
        &[],
    );
    let values = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(91),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(92), &[callee, values]);
    let root = b.add(NodeKind::Module, Payload::None, sp(93), &[call]);
    (finish_il(b, root, Lang::JavaScript), interner, call, callee)
}

fn push_java_collection_constructor_dependencies(il: &mut Il, call: NodeId, callee: NodeId) {
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(il.node(call).span),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::Construct)),
        EvidenceStatus::Asserted,
    ));
    let binding_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("ArrayList"),
    });
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::binding(sp(87), stable_symbol_hash("ArrayList")),
        binding_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        2,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Var),
        binding_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(1)],
    ));
}

fn push_js_global_constructor_dependencies(il: &mut Il, call: NodeId, callee: NodeId, name: &str) {
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(il.node(call).span),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::Construct)),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash(name),
        }),
        EvidenceStatus::Asserted,
    ));
}

fn push_java_map_import_dependencies(il: &mut Il, receiver: NodeId) {
    let binding_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("Map"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(80), stable_symbol_hash("Map")),
        binding_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(receiver).span, NodeKind::Var),
        binding_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
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
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
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
        RUST_STDLIB_OPTION_PRODUCER_ID,
    ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust Option Some evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_some_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
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
            RUST_STDLIB_OPTION_PACK_ID,
            "wrong.rust.stdlib.option-api",
        ));
    assert!(
        admitted_rust_option_some_constructor_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust Option Some evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, callee) = rust_some_call_il();
    wrong_emitter.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_emitter.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        }),
        EvidenceStatus::Asserted,
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
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        }),
        EvidenceStatus::Asserted,
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
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Some"),
        }),
        EvidenceStatus::Asserted,
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
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(none).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        }),
        EvidenceStatus::Asserted,
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
            FIRST_PARTY_PACK_ID,
            RUST_STDLIB_OPTION_PRODUCER_ID,
        ));
    assert!(
        admitted_rust_option_none_sentinel_at_node(&wrong_pack, &interner, none).is_none(),
        "Rust Option None evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, none) = rust_none_node_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(none).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        }),
        EvidenceStatus::Asserted,
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
    wrong_emitter.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_emitter.node(none).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        }),
        EvidenceStatus::Asserted,
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
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(none).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("None"),
        }),
        EvidenceStatus::Asserted,
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

#[test]
fn admitted_rust_option_and_then_resolver_requires_pack_provenance() {
    let (il, interner, call, _receiver) = rust_option_and_then_call_il();
    assert!(
        admitted_rust_option_and_then_at_call(&il, &interner, call).is_none(),
        "raw Rust and_then call shape alone must not admit Option semantics"
    );

    let contract = library_rust_option_and_then_contract(Lang::Rust, "and_then", 1)
        .expect("Rust Option and_then contract");
    let (mut wrong_pack, interner, call, receiver) = rust_option_and_then_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Option),
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
        RUST_STDLIB_OPTION_PRODUCER_ID,
    ));
    assert!(
        admitted_rust_option_and_then_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust Option and_then evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = rust_option_and_then_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Option),
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
            RUST_STDLIB_OPTION_PACK_ID,
            "wrong.rust.stdlib.option-api",
        ));
    assert!(
        admitted_rust_option_and_then_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust Option and_then evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, receiver) = rust_option_and_then_call_il();
    wrong_emitter.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_emitter.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Option),
        EvidenceStatus::Asserted,
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
        admitted_rust_option_and_then_at_call(&wrong_emitter, &interner, call).is_none(),
        "Rust Option and_then evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, receiver) = rust_option_and_then_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(receiver).span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Option),
        EvidenceStatus::Asserted,
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
    let occurrence = admitted_rust_option_and_then_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::RustOptionAndThen
    );
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 1);
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
fn admitted_imported_namespace_function_resolver_requires_pack_provenance() {
    let (il, interner, call, _receiver) = python_math_prod_call_il();
    assert!(
        admitted_imported_namespace_function_at_call(&il, &interner, call).is_none(),
        "raw Python math.prod(...) call shape alone must not admit imported namespace semantics"
    );

    let contract = library_imported_namespace_function_contract(Lang::Python, "prod", 1)
        .expect("Python math.prod contract");
    let (mut missing_dependency, interner, call, _receiver) = python_math_prod_call_il();
    missing_dependency.evidence.push(python_stdlib_math_record(
        0,
        missing_dependency.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_imported_namespace_function_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span Python math.prod evidence without namespace dependency is rejected"
    );

    let (mut wrong_pack, interner, call, receiver) = python_math_prod_call_il();
    push_python_math_namespace_dependencies(&mut wrong_pack, receiver);
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            FIRST_PARTY_PACK_ID,
            PYTHON_STDLIB_MATH_PRODUCER_ID,
        ));
    assert!(
        admitted_imported_namespace_function_at_call(&wrong_pack, &interner, call).is_none(),
        "Python math.prod evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = python_math_prod_call_il();
    push_python_math_namespace_dependencies(&mut wrong_producer, receiver);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            PYTHON_STDLIB_MATH_PACK_ID,
            "wrong.python.stdlib.math-api",
        ));
    assert!(
        admitted_imported_namespace_function_at_call(&wrong_producer, &interner, call).is_none(),
        "Python math.prod evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, receiver) = python_math_prod_call_il();
    push_python_math_namespace_dependencies(&mut wrong_emitter, receiver);
    let mut external_record = python_stdlib_math_record(
        2,
        wrong_emitter.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[1],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_imported_namespace_function_at_call(&wrong_emitter, &interner, call).is_none(),
        "Python math.prod evidence from an external emitter is rejected"
    );

    let (mut wrong_arity, interner, call, receiver) = python_math_prod_call_il_with_arg_count(3);
    push_python_math_namespace_dependencies(&mut wrong_arity, receiver);
    wrong_arity.evidence.push(python_stdlib_math_record(
        2,
        wrong_arity.node(call).span,
        contract,
        3,
        EvidenceStatus::Asserted,
        &[1],
    ));
    assert!(
        admitted_imported_namespace_function_at_call(&wrong_arity, &interner, call).is_none(),
        "Python math.prod evidence with unsupported arity is rejected"
    );

    let (mut admitted, interner, call, receiver) = python_math_prod_call_il();
    push_python_math_namespace_dependencies(&mut admitted, receiver);
    admitted.evidence.push(python_stdlib_math_record(
        2,
        admitted.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence =
        admitted_imported_namespace_function_at_call(&admitted, &interner, call).unwrap();
    let field_callee = admitted.children(call)[0];
    assert_eq!(occurrence.contract.id, contract.id);
    assert_eq!(occurrence.callee, field_callee);
    assert_eq!(occurrence.receiver, Some(receiver));
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
fn admitted_rust_std_collection_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = rust_std_collection_factory_call_il();
    assert!(
        admitted_free_name_collection_factory_at_call(&il, &interner, call).is_none(),
        "raw Rust std::collections factory shape alone must not admit stdlib semantics"
    );

    let contract = library_free_name_collection_factory_contract(
        Lang::Rust,
        "std::collections::HashSet::from",
    )
    .expect("Rust std::collections HashSet::from contract");

    let (mut missing_dependency, interner, call, _callee) = rust_std_collection_factory_call_il();
    missing_dependency
        .evidence
        .push(rust_stdlib_collection_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_free_name_collection_factory_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span Rust std::collections evidence without callee dependency is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = rust_std_collection_factory_call_il();
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_pack.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashSet::from"),
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
        RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    assert!(
        admitted_free_name_collection_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust std::collections evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_std_collection_factory_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::node(wrong_producer.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashSet::from"),
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
            RUST_STDLIB_COLLECTION_FACTORY_PACK_ID,
            "wrong.rust.stdlib.collection-factory-api",
        ));
    assert!(
        admitted_free_name_collection_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust std::collections evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_std_collection_factory_call_il();
    admitted.evidence.push(evidence(
        0,
        EvidenceAnchor::node(admitted.node(callee).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("std::collections::HashSet::from"),
        }),
        EvidenceStatus::Asserted,
    ));
    admitted
        .evidence
        .push(rust_stdlib_collection_factory_record(
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
        LibraryApiContractId::RustStdCollectionFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_rust_std_map_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = rust_std_map_factory_call_il();
    assert!(
        admitted_free_name_map_factory_at_call(&il, &interner, call).is_none(),
        "raw Rust std::collections map factory shape alone must not admit stdlib semantics"
    );

    let contract =
        library_free_name_map_factory_contract(Lang::Rust, "std::collections::HashMap::from")
            .expect("Rust std::collections HashMap::from contract");

    let (mut missing_dependency, interner, call, _callee) = rust_std_map_factory_call_il();
    missing_dependency
        .evidence
        .push(rust_stdlib_map_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_free_name_map_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Rust std::collections map evidence without callee dependency is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = rust_std_map_factory_call_il();
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
        admitted_free_name_map_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust std::collections map evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = rust_std_map_factory_call_il();
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
        admitted_free_name_map_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust std::collections map evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = rust_std_map_factory_call_il();
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

    let occurrence = admitted_free_name_map_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::RustStdMapFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_java_collection_factory_resolver_requires_pack_provenance() {
    let interner = Interner::new();
    let (mut raw, call, _root, _local, _contract) =
        java_list_of_import_evidence_il(&interner, true);
    raw.evidence.clear();
    assert!(
        admitted_java_collection_factory_at_call(&raw, &interner, call).is_none(),
        "raw Java List.of(...) shape alone must not admit stdlib collection factory semantics"
    );

    let (mut missing_dependency, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    missing_dependency.evidence.clear();
    missing_dependency
        .evidence
        .push(java_stdlib_collection_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            1,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_java_collection_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Java List.of evidence without import dependency is rejected"
    );

    let (mut wrong_pack, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    wrong_pack
        .evidence
        .retain(|record| record.id != EvidenceId(2));
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            FIRST_PARTY_PACK_ID,
            JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
        ));
    assert!(
        admitted_java_collection_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Java List.of evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    wrong_producer
        .evidence
        .retain(|record| record.id != EvidenceId(2));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID,
            "wrong.java.stdlib.collection-factory-api",
        ));
    assert!(
        admitted_java_collection_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Java List.of evidence with the wrong producer is rejected"
    );

    let (admitted, call, _root, _local, contract) =
        java_list_of_import_evidence_il(&interner, true);
    let occurrence = admitted_java_collection_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JavaCollectionFactory(JavaCollectionFactoryKind::ListOf)
    );
    assert_eq!(occurrence.contract.callee, contract.callee);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_java_map_factory_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee, _receiver) = java_map_factory_call_il();
    assert!(
        admitted_java_map_factory_at_call(&il, &interner, call).is_none(),
        "raw Java Map.of(...) shape alone must not admit stdlib map factory semantics"
    );

    let contract =
        library_java_map_factory_contract(Lang::Java, "Map", "of").expect("Map.of contract");

    let (mut missing_dependency, interner, call, _callee, _receiver) = java_map_factory_call_il();
    missing_dependency
        .evidence
        .push(java_stdlib_map_factory_record(
            0,
            missing_dependency.node(call).span,
            contract,
            2,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_java_map_factory_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Java Map.of evidence without import dependency is rejected"
    );

    let (mut wrong_pack, interner, call, _callee, receiver) = java_map_factory_call_il();
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
        admitted_java_map_factory_at_call(&wrong_pack, &interner, call).is_none(),
        "Java Map.of evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, _callee, receiver) = java_map_factory_call_il();
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
        admitted_java_map_factory_at_call(&wrong_producer, &interner, call).is_none(),
        "Java Map.of evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee, receiver) = java_map_factory_call_il();
    push_java_map_import_dependencies(&mut admitted, receiver);
    admitted.evidence.push(java_stdlib_map_factory_record(
        2,
        admitted.node(call).span,
        contract,
        2,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence = admitted_java_map_factory_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JavaMapFactory(JavaMapFactoryKind::Of)
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 2);
}

#[test]
fn admitted_java_map_entry_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee, _receiver) = java_map_entry_call_il();
    assert!(
        admitted_java_map_entry_at_call(&il, &interner, call).is_none(),
        "raw Java Map.entry(...) shape alone must not admit stdlib map-entry semantics"
    );

    let contract =
        library_java_map_entry_contract(Lang::Java, "Map", "entry").expect("Map.entry contract");

    let (mut missing_dependency, interner, call, _callee, _receiver) = java_map_entry_call_il();
    missing_dependency
        .evidence
        .push(java_stdlib_map_entry_record(
            0,
            missing_dependency.node(call).span,
            contract,
            2,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_java_map_entry_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Java Map.entry evidence without import dependency is rejected"
    );

    let (mut wrong_pack, interner, call, _callee, receiver) = java_map_entry_call_il();
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
            JAVA_STDLIB_MAP_ENTRY_PRODUCER_ID,
        ));
    assert!(
        admitted_java_map_entry_at_call(&wrong_pack, &interner, call).is_none(),
        "Java Map.entry evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, _callee, receiver) = java_map_entry_call_il();
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
            JAVA_STDLIB_MAP_ENTRY_PACK_ID,
            "wrong.java.stdlib.map-entry-api",
        ));
    assert!(
        admitted_java_map_entry_at_call(&wrong_producer, &interner, call).is_none(),
        "Java Map.entry evidence with the wrong producer is rejected"
    );

    let (mut wrong_arity, interner, call, _callee, receiver) =
        java_map_entry_call_il_with_arg_count(3);
    push_java_map_import_dependencies(&mut wrong_arity, receiver);
    wrong_arity.evidence.push(java_stdlib_map_entry_record(
        2,
        wrong_arity.node(call).span,
        contract,
        3,
        EvidenceStatus::Asserted,
        &[1],
    ));
    assert!(
        admitted_java_map_entry_at_call(&wrong_arity, &interner, call).is_none(),
        "Java Map.entry evidence with unsupported arity is rejected"
    );

    let (mut admitted, interner, call, callee, receiver) = java_map_entry_call_il();
    push_java_map_import_dependencies(&mut admitted, receiver);
    admitted.evidence.push(java_stdlib_map_entry_record(
        2,
        admitted.node(call).span,
        contract,
        2,
        EvidenceStatus::Asserted,
        &[1],
    ));

    let occurrence = admitted_java_map_entry_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JavaMapEntryFactory
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 2);
}

#[test]
fn admitted_js_set_constructor_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = js_global_constructor_call_il("Set");
    assert!(
        admitted_js_like_set_constructor_at_call(&il, &interner, call).is_none(),
        "raw JS new Set(...) shape alone must not admit builtin Set constructor semantics"
    );

    let contract =
        library_js_like_set_constructor_contract(Lang::JavaScript, "Set").expect("Set contract");

    let (mut missing_dependency, interner, call, _callee) = js_global_constructor_call_il("Set");
    missing_dependency
        .evidence
        .push(js_like_builtin_collection_constructor_record(
            0,
            missing_dependency.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_js_like_set_constructor_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Set evidence without construct/global dependencies is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = js_global_constructor_call_il("Set");
    push_js_global_constructor_dependencies(&mut wrong_pack, call, callee, "Set");
    wrong_pack.evidence.push(library_api_record_with_provenance(
        2,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 1],
        FIRST_PARTY_PACK_ID,
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    ));
    assert!(
        admitted_js_like_set_constructor_at_call(&wrong_pack, &interner, call).is_none(),
        "Set constructor evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = js_global_constructor_call_il("Set");
    push_js_global_constructor_dependencies(&mut wrong_producer, call, callee, "Set");
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 1],
            JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
            "wrong.javascript.builtins.collection-constructor-api",
        ));
    assert!(
        admitted_js_like_set_constructor_at_call(&wrong_producer, &interner, call).is_none(),
        "Set constructor evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, callee) = js_global_constructor_call_il("Set");
    push_js_global_constructor_dependencies(&mut wrong_emitter, call, callee, "Set");
    let mut external_record = js_like_builtin_collection_constructor_record(
        2,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 1],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_js_like_set_constructor_at_call(&wrong_emitter, &interner, call).is_none(),
        "Set constructor evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, callee) = js_global_constructor_call_il("Set");
    push_js_global_constructor_dependencies(&mut admitted, call, callee, "Set");
    admitted
        .evidence
        .push(js_like_builtin_collection_constructor_record(
            2,
            admitted.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 1],
        ));
    let occurrence = admitted_js_like_set_constructor_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JsLikeSetConstructor
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_js_map_constructor_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = js_global_constructor_call_il("Map");
    assert!(
        admitted_js_like_map_constructor_at_call(&il, &interner, call).is_none(),
        "raw JS new Map(...) shape alone must not admit builtin Map constructor semantics"
    );

    let contract =
        library_js_like_map_constructor_contract(Lang::JavaScript, "Map").expect("Map contract");

    let (mut missing_dependency, interner, call, _callee) = js_global_constructor_call_il("Map");
    missing_dependency
        .evidence
        .push(js_like_builtin_collection_constructor_record(
            0,
            missing_dependency.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_js_like_map_constructor_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Map evidence without construct/global dependencies is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = js_global_constructor_call_il("Map");
    push_js_global_constructor_dependencies(&mut wrong_pack, call, callee, "Map");
    wrong_pack.evidence.push(library_api_record_with_provenance(
        2,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 1],
        FIRST_PARTY_PACK_ID,
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    ));
    assert!(
        admitted_js_like_map_constructor_at_call(&wrong_pack, &interner, call).is_none(),
        "Map constructor evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = js_global_constructor_call_il("Map");
    push_js_global_constructor_dependencies(&mut wrong_producer, call, callee, "Map");
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 1],
            JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
            "wrong.javascript.builtins.collection-constructor-api",
        ));
    assert!(
        admitted_js_like_map_constructor_at_call(&wrong_producer, &interner, call).is_none(),
        "Map constructor evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, callee) = js_global_constructor_call_il("Map");
    push_js_global_constructor_dependencies(&mut wrong_emitter, call, callee, "Map");
    let mut external_record = js_like_builtin_collection_constructor_record(
        2,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 1],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_js_like_map_constructor_at_call(&wrong_emitter, &interner, call).is_none(),
        "Map constructor evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, callee) = js_global_constructor_call_il("Map");
    push_js_global_constructor_dependencies(&mut admitted, call, callee, "Map");
    admitted
        .evidence
        .push(js_like_builtin_collection_constructor_record(
            2,
            admitted.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[0, 1],
        ));
    let occurrence = admitted_js_like_map_constructor_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JsLikeMapConstructor
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 1);
}

#[test]
fn admitted_java_collection_constructor_resolver_requires_pack_provenance() {
    let (il, interner, call, _callee) = java_collection_constructor_call_il();
    assert!(
        admitted_java_collection_constructor_at_call(&il, &interner, call).is_none(),
        "raw Java new ArrayList<>() call shape alone must not admit stdlib constructor semantics"
    );

    let contract = library_java_collection_constructor_contract(Lang::Java, "ArrayList", 0)
        .expect("ArrayList constructor contract");

    let (mut missing_dependency, interner, call, _callee) = java_collection_constructor_call_il();
    missing_dependency
        .evidence
        .push(java_stdlib_collection_constructor_record(
            0,
            missing_dependency.node(call).span,
            contract,
            0,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_java_collection_constructor_at_call(&missing_dependency, &interner, call)
            .is_none(),
        "same-span Java constructor evidence without construct/import dependencies is rejected"
    );

    let (mut wrong_pack, interner, call, callee) = java_collection_constructor_call_il();
    push_java_collection_constructor_dependencies(&mut wrong_pack, call, callee);
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            3,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            0,
            EvidenceStatus::Asserted,
            &[0, 2],
            FIRST_PARTY_PACK_ID,
            JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
        ));
    assert!(
        admitted_java_collection_constructor_at_call(&wrong_pack, &interner, call).is_none(),
        "Java constructor evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee) = java_collection_constructor_call_il();
    push_java_collection_constructor_dependencies(&mut wrong_producer, call, callee);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            3,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            0,
            EvidenceStatus::Asserted,
            &[0, 2],
            JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
            "wrong.java.stdlib.collection-constructor-api",
        ));
    assert!(
        admitted_java_collection_constructor_at_call(&wrong_producer, &interner, call).is_none(),
        "Java constructor evidence with the wrong producer is rejected"
    );

    let (mut admitted, interner, call, callee) = java_collection_constructor_call_il();
    push_java_collection_constructor_dependencies(&mut admitted, call, callee);
    admitted
        .evidence
        .push(java_stdlib_collection_constructor_record(
            3,
            admitted.node(call).span,
            contract,
            0,
            EvidenceStatus::Asserted,
            &[0, 2],
        ));

    let occurrence =
        admitted_java_collection_constructor_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::JavaCollectionConstructor(JavaCollectionConstructorKind::EmptyList)
    );
    assert_eq!(occurrence.callee, callee);
    assert_eq!(occurrence.receiver, None);
    assert_eq!(occurrence.arg_count, 0);
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
    missing_dependency
        .evidence
        .push(java_stdlib_static_collection_adapter_record(
            0,
            missing_dependency.node(call).span,
            contract,
            1,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_static_collection_adapter_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Java static adapter evidence without import dependency is rejected"
    );

    let (mut wrong_pack, interner, call, receiver) = java_arrays_stream_call_il();
    let imported_binding = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("Arrays"),
    });
    wrong_pack.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("Arrays")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_pack.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_pack.node(receiver).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            FIRST_PARTY_PACK_ID,
            JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_ID,
        ));
    assert!(
        admitted_static_collection_adapter_at_call(&wrong_pack, &interner, call).is_none(),
        "Java Arrays.stream evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = java_arrays_stream_call_il();
    wrong_producer.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("Arrays")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_producer.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_producer.node(receiver).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            2,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            1,
            EvidenceStatus::Asserted,
            &[1],
            JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
            "wrong.java.stdlib.static-collection-adapter-api",
        ));
    assert!(
        admitted_static_collection_adapter_at_call(&wrong_producer, &interner, call).is_none(),
        "Java Arrays.stream evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, receiver) = java_arrays_stream_call_il();
    wrong_emitter.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("Arrays")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_emitter.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_emitter.node(receiver).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    let mut external_record = java_stdlib_static_collection_adapter_record(
        2,
        wrong_emitter.node(call).span,
        contract,
        1,
        EvidenceStatus::Asserted,
        &[1],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_static_collection_adapter_at_call(&wrong_emitter, &interner, call).is_none(),
        "Java Arrays.stream evidence from an external emitter is rejected"
    );

    let (mut wrong_arity, interner, call, receiver) = java_arrays_stream_call_il_with_arg_count(2);
    wrong_arity.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(66), stable_symbol_hash("Arrays")),
        imported_binding,
        EvidenceStatus::Asserted,
    ));
    wrong_arity.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(wrong_arity.node(receiver).span, NodeKind::Var),
        imported_binding,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    wrong_arity
        .evidence
        .push(java_stdlib_static_collection_adapter_record(
            2,
            wrong_arity.node(call).span,
            contract,
            2,
            EvidenceStatus::Asserted,
            &[1],
        ));
    assert!(
        admitted_static_collection_adapter_at_call(&wrong_arity, &interner, call).is_none(),
        "Java Arrays.stream evidence with unsupported arity is rejected"
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
    admitted
        .evidence
        .push(java_stdlib_static_collection_adapter_record(
            2,
            admitted.node(call).span,
            contract,
            1,
            EvidenceStatus::Asserted,
            &[1],
        ));

    let occurrence =
        admitted_static_collection_adapter_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(occurrence.contract.id, contract.id);
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 1);
}

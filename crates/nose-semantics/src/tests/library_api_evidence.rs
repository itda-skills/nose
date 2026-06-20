use super::*;

mod admission_resolvers;
mod callee_sources;
mod canonical_builtin;
mod resolution;

fn library_api_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_arity(id, span, contract_id, callee, 1, status, dependencies)
}

fn library_api_record_with_provenance(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
    pack_id: &str,
    rule: &str,
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract_id,
        callee,
        1,
        status,
        dependencies,
        pack_id,
        rule,
    )
}

fn library_api_record_with_provenance_and_arity(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
    pack_id: &str,
    rule: &str,
) -> EvidenceRecord {
    let mut record =
        library_api_record_with_arity(id, span, contract_id, callee, arity, status, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(pack_id));
    record.provenance.rule_hash = Some(stable_symbol_hash(rule));
    record
}

fn python_builtin_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn python_stdlib_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
        PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn python_stdlib_math_record(
    id: u32,
    span: Span,
    contract: LibraryImportedNamespaceFunctionContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        PYTHON_STDLIB_MATH_PACK_ID,
        PYTHON_STDLIB_MATH_PRODUCER_ID,
    )
}

fn js_like_builtin_promise_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract_id,
        callee,
        status,
        dependencies,
        JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
    )
}

fn js_like_builtin_array_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract_id,
        callee,
        status,
        dependencies,
        JS_LIKE_BUILTIN_ARRAY_PACK_ID,
        JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID,
    )
}

fn js_like_builtin_collection_constructor_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract_id,
        callee,
        status,
        dependencies,
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    )
}

fn ruby_stdlib_set_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        RUBY_STDLIB_SET_PACK_ID,
        RUBY_STDLIB_SET_PRODUCER_ID,
    )
}

fn rust_stdlib_vec_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    let mut record = library_api_record_with_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(RUST_STDLIB_VEC_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(RUST_STDLIB_VEC_PRODUCER_ID));
    record
}

fn rust_stdlib_option_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract_id,
        callee,
        arity,
        status,
        dependencies,
        RUST_STDLIB_OPTION_PACK_ID,
        RUST_STDLIB_OPTION_PRODUCER_ID,
    )
}

fn rust_stdlib_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        RUST_STDLIB_COLLECTION_FACTORY_PACK_ID,
        RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn rust_stdlib_map_factory_record(
    id: u32,
    span: Span,
    contract: LibraryMapFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        RUST_STDLIB_MAP_FACTORY_PACK_ID,
        RUST_STDLIB_MAP_FACTORY_PRODUCER_ID,
    )
}

fn java_stdlib_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID,
        JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn java_stdlib_collection_constructor_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    )
}

fn java_stdlib_map_factory_record(
    id: u32,
    span: Span,
    contract: LibraryMapFactoryContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        JAVA_STDLIB_MAP_FACTORY_PACK_ID,
        JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID,
    )
}

fn java_stdlib_map_entry_record(
    id: u32,
    span: Span,
    contract: LibraryMapEntryFactoryContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        JAVA_STDLIB_MAP_ENTRY_PACK_ID,
        JAVA_STDLIB_MAP_ENTRY_PRODUCER_ID,
    )
}

fn java_stdlib_static_collection_adapter_record(
    id: u32,
    span: Span,
    contract: LibraryStaticCollectionAdapterContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
        JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_ID,
    )
}

fn library_api_record_with_arity(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::node(span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract_id),
            callee_hash: library_api_callee_contract_hash(callee),
            arity,
        }),
        status,
        dependencies.iter().copied().map(EvidenceId).collect(),
    )
}

fn contract_status_for_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
) -> LibraryApiEvidenceStatus {
    library_api_contract_evidence_for_call(il, interner, call, id, callee, 1)
}

fn java_list_of_import_evidence_il(
    interner: &Interner,
    import_in_root: bool,
) -> (Il, NodeId, NodeId, Symbol, LibraryCollectionFactoryContract) {
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("List");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(30), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(30), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(30), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(31), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("of")),
        sp(32),
        &[receiver],
    );
    let arg = b.add(NodeKind::Lit, Payload::LitInt(1), sp(33), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(34), &[callee, arg]);
    let root = if import_in_root {
        b.add(NodeKind::Module, Payload::None, sp(29), &[import, call])
    } else {
        b.add(NodeKind::Func, Payload::None, sp(35), &[call])
    };
    let mut il = finish_il(b, root, Lang::Java);
    let contract = library_java_collection_factory_contract(Lang::Java, "List", "of")
        .expect("List.of contract");
    let binding_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("List"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(30), stable_symbol_hash("List")),
        binding_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(sp(31), NodeKind::Var),
        binding_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    il.evidence.push(java_stdlib_collection_factory_record(
        2,
        sp(34),
        contract,
        1,
        EvidenceStatus::Asserted,
        &[1],
    ));
    (il, call, root, local, contract)
}

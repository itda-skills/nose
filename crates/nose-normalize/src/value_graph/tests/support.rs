pub(super) use super::super::*;
pub(super) use nose_il::{
    Builtin, CallTargetEvidenceKind, EffectEvidenceKind, EvidenceAnchor, EvidenceEmitter,
    EvidenceId, EvidenceKind, EvidenceProvenance, EvidenceRecord, EvidenceStatus, FileId, FileMeta,
    GuardEvidenceKind, HoFKind, Il, IlBuilder, ImportEvidenceKind, Interner,
    JsRecordGuardComparison, JsRecordGuardNullCheck, Lang, LibraryApiEvidenceKind, NodeId,
    NodeKind, Op, ParamSemantic, Payload, SequenceSurfaceKind, SourceCallKind, SourceCastKind,
    SourceComprehensionKind, SourceFactKind, SourcePatternKind, SourceRangeKind, Span, Symbol,
    SymbolEvidenceKind, Unit, UnitKind,
};
pub(super) use nose_semantics::{
    admitted_hof_demand_effect_profile_at_node, builtin_tag, library_api_callee_contract_hash,
    library_api_contract_id_hash, library_free_function_builtin_contract,
    library_free_name_collection_factory_contract, library_imported_collection_factory_contract,
    library_java_collection_constructor_contract, library_java_collection_factory_contract,
    library_java_map_factory_contract, library_js_like_map_constructor_contract,
    library_js_like_set_constructor_contract, library_method_call_contract,
    library_promise_resolve_contract, library_promise_then_contract,
    library_rust_option_none_sentinel_contract, library_rust_option_some_constructor_contract,
    library_scalar_integer_method_contract, library_static_index_membership_contract,
    DomainEvidence, LibraryApiContractId, LibraryCollectionFactoryContract,
    LibraryMapFactoryContract, C_LANGUAGE_PACK_ID, C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID,
    FIRST_PARTY_PACK_ID, JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
    JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID, JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID,
    JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID, JAVA_STDLIB_MAP_FACTORY_PACK_ID,
    JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID, JAVA_STDLIB_MATH_PACK_ID, JAVA_STDLIB_MATH_PRODUCER_ID,
    JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
    JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID, JS_LIKE_BUILTIN_PROMISE_PACK_ID,
    JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID, JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
    JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_ID, PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
    PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID, PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
    PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID, RUST_STDLIB_INTEGER_METHOD_PACK_ID,
    RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID, RUST_STDLIB_OPTION_PACK_ID,
    RUST_STDLIB_OPTION_PRODUCER_ID,
};
pub(super) use rustc_hash::FxHashMap;

pub(super) fn sp(line: u32) -> Span {
    Span::new(FileId(0), line, line, line, line)
}

pub(super) fn finish_test_il(builder: IlBuilder, root: NodeId, lang: Lang) -> Il {
    builder.finish(
        root,
        FileMeta {
            path: "t".into(),
            lang,
        },
        Vec::new(),
        Vec::new(),
    )
}

pub(super) fn evidence(id: u32, anchor: EvidenceAnchor, kind: EvidenceKind) -> EvidenceRecord {
    evidence_with_dependencies(id, anchor, kind, Vec::new())
}

pub(super) fn evidence_with_dependencies(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor,
        kind,
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("test")),
        },
        dependencies,
        status: EvidenceStatus::Asserted,
    }
}

pub(super) fn rust_option_evidence_with_dependencies(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = evidence_with_dependencies(id, anchor, kind, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(RUST_STDLIB_OPTION_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(RUST_STDLIB_OPTION_PRODUCER_ID));
    record
}

pub(super) fn js_like_promise_evidence_with_dependencies(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = evidence_with_dependencies(id, anchor, kind, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(JS_LIKE_BUILTIN_PROMISE_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID));
    record
}

pub(super) fn imported_binding_symbol(module: &str, exported: &str) -> EvidenceKind {
    EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash(module),
        exported_hash: stable_symbol_hash(exported),
    })
}

pub(super) fn imported_namespace_symbol_kind(module: &str) -> EvidenceKind {
    EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash(module),
    })
}

pub(super) fn push_imported_binding_use(
    il: &mut Il,
    binding_id: u32,
    binding_span: Span,
    occurrence_id: u32,
    occurrence_span: Span,
    module: &str,
    exported: &str,
) {
    let symbol = imported_binding_symbol(module, exported);
    il.evidence.push(evidence(
        binding_id,
        EvidenceAnchor::binding(binding_span, stable_symbol_hash(exported)),
        symbol,
    ));
    il.evidence.push(evidence_with_dependencies(
        occurrence_id,
        EvidenceAnchor::node(occurrence_span, NodeKind::Var),
        symbol,
        vec![EvidenceId(binding_id)],
    ));
}

pub(super) fn push_imported_namespace_use(
    il: &mut Il,
    binding_id: u32,
    binding_span: Span,
    occurrence_id: u32,
    occurrence_span: Span,
    module: &str,
) {
    let symbol = imported_namespace_symbol_kind(module);
    il.evidence.push(evidence(
        binding_id,
        EvidenceAnchor::binding(binding_span, stable_symbol_hash(module)),
        symbol,
    ));
    il.evidence.push(evidence_with_dependencies(
        occurrence_id,
        EvidenceAnchor::node(occurrence_span, NodeKind::Var),
        symbol,
        vec![EvidenceId(binding_id)],
    ));
}

pub(super) fn collection_sequence_evidence(id: u32, span: Span) -> EvidenceRecord {
    evidence(
        id,
        EvidenceAnchor::sequence(span),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
    )
}

pub(super) fn identity_lambda(builder: &mut IlBuilder, param_cid: u32, span: Span) -> NodeId {
    let param = builder.add(NodeKind::Param, Payload::Cid(param_cid), span, &[]);
    let value = builder.add(NodeKind::Var, Payload::Cid(param_cid), span, &[]);
    let ret = builder.add(NodeKind::Return, Payload::None, span, &[value]);
    let block = builder.add(NodeKind::Block, Payload::None, span, &[ret]);
    builder.add(NodeKind::Lambda, Payload::None, span, &[param, block])
}

pub(super) fn const_bool_lambda(
    builder: &mut IlBuilder,
    param_cid: u32,
    value: bool,
    span: Span,
) -> NodeId {
    let param = builder.add(NodeKind::Param, Payload::Cid(param_cid), span, &[]);
    let value = builder.add(NodeKind::Lit, Payload::LitBool(value), span, &[]);
    let ret = builder.add(NodeKind::Return, Payload::None, span, &[value]);
    let block = builder.add(NodeKind::Block, Payload::None, span, &[ret]);
    builder.add(NodeKind::Lambda, Payload::None, span, &[param, block])
}

pub(super) fn div_zero_lambda(builder: &mut IlBuilder, param_cid: u32, span: Span) -> NodeId {
    let param = builder.add(NodeKind::Param, Payload::Cid(param_cid), span, &[]);
    let lhs = builder.add(NodeKind::Lit, Payload::LitInt(1), span, &[]);
    let rhs = builder.add(NodeKind::Lit, Payload::LitInt(0), span, &[]);
    let div = builder.add(NodeKind::BinOp, Payload::Op(Op::Div), span, &[lhs, rhs]);
    let ret = builder.add(NodeKind::Return, Payload::None, span, &[div]);
    let block = builder.add(NodeKind::Block, Payload::None, span, &[ret]);
    builder.add(NodeKind::Lambda, Payload::None, span, &[param, block])
}

pub(super) fn push_source_comprehension(
    il: &mut Il,
    id: u32,
    span: Span,
    kind: SourceComprehensionKind,
) {
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Source(SourceFactKind::Comprehension(kind)),
    ));
}

pub(super) fn push_source_cast(il: &mut Il, id: u32, span: Span, kind: SourceCastKind) {
    let mut record = evidence(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Source(SourceFactKind::Cast(kind)),
    );
    match kind {
        SourceCastKind::CUnsigned32 => {
            record.provenance.pack_hash = Some(stable_symbol_hash(C_LANGUAGE_PACK_ID));
            record.provenance.rule_hash =
                Some(stable_symbol_hash(C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID));
        }
    }
    il.evidence.push(record);
}

pub(super) fn push_source_range(il: &mut Il, id: u32, span: Span, kind: SourceRangeKind) {
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Source(SourceFactKind::Range(kind)),
    ));
}

pub(super) fn push_source_pattern(il: &mut Il, id: u32, span: Span, kind: SourcePatternKind) {
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::source_span(span),
        EvidenceKind::Source(SourceFactKind::Pattern(kind)),
    ));
}

pub(super) fn library_api_contract_evidence(
    id: u32,
    call_span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract_id),
            callee_hash: library_api_callee_contract_hash(callee),
            arity,
        }),
        dependencies,
    )
}

pub(super) fn js_like_builtin_collection_constructor_evidence(
    id: u32,
    call_span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record =
        library_api_contract_evidence(id, call_span, contract_id, callee, arity, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
    ));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    ));
    record
}

pub(super) fn js_like_builtin_static_index_membership_evidence(
    id: u32,
    call_span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record =
        library_api_contract_evidence(id, call_span, contract_id, callee, arity, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(
        JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
    ));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_ID,
    ));
    record
}

pub(super) fn rust_stdlib_integer_method_evidence(
    id: u32,
    call_span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record =
        library_api_contract_evidence(id, call_span, contract_id, callee, arity, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(RUST_STDLIB_INTEGER_METHOD_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID));
    record
}

pub(super) fn java_stdlib_math_evidence(
    id: u32,
    call_span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record =
        library_api_contract_evidence(id, call_span, contract_id, callee, arity, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(JAVA_STDLIB_MATH_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(JAVA_STDLIB_MATH_PRODUCER_ID));
    record
}

pub(super) fn python_builtin_collection_factory_evidence(
    id: u32,
    call_span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity,
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(
        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
    ));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    record
}

pub(super) fn python_stdlib_collection_factory_evidence(
    id: u32,
    call_span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity,
        dependencies,
    );
    record.provenance.pack_hash =
        Some(stable_symbol_hash(PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    record
}

pub(super) fn java_stdlib_map_factory_evidence(
    id: u32,
    call_span: Span,
    contract: LibraryMapFactoryContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity,
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(JAVA_STDLIB_MAP_FACTORY_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID));
    record
}

pub(super) fn java_stdlib_collection_factory_evidence(
    id: u32,
    call_span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity,
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    record
}

pub(super) fn java_stdlib_collection_constructor_evidence(
    id: u32,
    call_span: Span,
    contract: LibraryCollectionFactoryContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let mut record = library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity,
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(
        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
    ));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
    ));
    record
}

pub(super) fn push_method_call_library_api_evidence(
    il: &mut Il,
    interner: &Interner,
    id: u32,
    call: NodeId,
    method: &str,
    arity: usize,
) {
    let contract =
        library_method_call_contract(il.meta.lang, method, arity).expect("method contract");
    let dependencies = nose_semantics::library_api_receiver_dependencies_for_call(
        il,
        interner,
        call,
        contract.callee,
    )
    .expect("method receiver dependencies");
    il.evidence.push(library_api_contract_evidence(
        id,
        il.node(call).span,
        contract.id,
        contract.callee,
        arity as u16,
        dependencies,
    ));
}

pub(super) fn push_library_api_evidence_for_callee(
    il: &mut Il,
    interner: &Interner,
    id: u32,
    call: NodeId,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
) {
    let dependencies =
        nose_semantics::library_api_receiver_dependencies_for_call(il, interner, call, callee)
            .expect("library api receiver dependencies");
    let record = if matches!(contract_id, LibraryApiContractId::ScalarIntegerMethod(_))
        && matches!(
            callee,
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::ExactInteger,
                ..
            }
        ) {
        rust_stdlib_integer_method_evidence(
            id,
            il.node(call).span,
            contract_id,
            callee,
            arity,
            dependencies,
        )
    } else if matches!(contract_id, LibraryApiContractId::ScalarIntegerMethod(_))
        && matches!(
            callee,
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::UnshadowedGlobal("Math"),
                ..
            }
        )
    {
        java_stdlib_math_evidence(
            id,
            il.node(call).span,
            contract_id,
            callee,
            arity,
            dependencies,
        )
    } else {
        library_api_contract_evidence(
            id,
            il.node(call).span,
            contract_id,
            callee,
            arity,
            dependencies,
        )
    };
    il.evidence.push(record);
}

pub(super) fn eval_proven_collection_op(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<ValOp> {
    let mut builder = Builder::new(il, interner);
    let raw = builder.eval(call, &FxHashMap::default());
    builder
        .proven_collection_value(raw)
        .map(|value| builder.nodes[value as usize].op.clone())
}

pub(super) fn receiver_domain_contains_call_il() -> (Il, Interner, NodeId, Span) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver_span = sp(30);
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("xs")),
        receiver_span,
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("includes")),
        sp(31),
        &[receiver],
    );
    let item = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("item")),
        sp(32),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(33), &[callee, item]);
    let root = b.add(NodeKind::Block, Payload::None, sp(29), &[call]);
    let il = finish_test_il(b, root, Lang::TypeScript);
    (il, interner, call, receiver_span)
}

pub(super) fn eval_op(il: &Il, interner: &Interner, node: NodeId) -> ValOp {
    let mut builder = Builder::new(il, interner);
    let value = builder.eval(node, &FxHashMap::default());
    builder.nodes[value as usize].op.clone()
}

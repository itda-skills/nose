use nose_il::{
    stable_symbol_hash, Builtin, EvidenceAnchor, EvidenceEmitter, EvidenceId, EvidenceKind,
    EvidenceProvenance, EvidenceRecord, EvidenceStatus, FileId, FileMeta, Il, IlBuilder, Interner,
    Lang, LibraryApiEvidenceKind, NodeId, NodeKind, Payload, SequenceSurfaceKind, SourceCallKind,
    SourceFactKind, Span, SymbolEvidenceKind,
};
use nose_semantics::{
    library_api_callee_contract_hash, library_api_contract_id_hash, library_method_call_contract,
    LibraryApiCalleeContract, LibraryCollectionFactoryContract, FIRST_PARTY_PACK_ID,
    JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID, JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
    JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID, PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
    PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
};

pub(super) fn sp(line: u32) -> Span {
    Span::new(FileId(0), line, line, line, line)
}

pub(super) fn evidence(
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

pub(super) fn library_api_contract_evidence(
    id: u32,
    call_span: Span,
    contract_id: nose_semantics::LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    evidence(
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
    contract_id: nose_semantics::LibraryApiContractId,
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

pub(super) fn method_call_library_api_evidence(
    id: u32,
    lang: Lang,
    method: &str,
    call_span: Span,
    arity: usize,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let contract = library_method_call_contract(lang, method, arity).expect("method call contract");
    library_api_contract_evidence(
        id,
        call_span,
        contract.id,
        contract.callee,
        arity as u16,
        dependencies,
    )
}

/// Push the `List.of(…)`-shaped factory contract plus the dependent `contains`
/// method-call evidence used by the Java collection-factory tests.
pub(super) fn push_java_factory_contract_evidence(
    il: &mut Il,
    contract_id: nose_semantics::LibraryApiContractId,
    callee: LibraryApiCalleeContract,
) {
    let mut record =
        library_api_contract_evidence(2, sp(25), contract_id, callee, 2, vec![EvidenceId(1)]);
    record.provenance.pack_hash = Some(stable_symbol_hash(JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(
        JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    ));
    il.evidence.push(record);
    il.evidence.push(method_call_library_api_evidence(
        3,
        Lang::Java,
        "contains",
        sp(28),
        1,
        vec![EvidenceId(2)],
    ));
}

pub(super) fn js_new_set_il(interner: &Interner) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let set = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Set")),
        sp(10),
        &[],
    );
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(11), &[]);
    let array = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(12),
        &[one],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(13), &[set, array]);
    let root = b.add(NodeKind::Block, Payload::None, sp(13), &[call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.js".into(),
            lang: Lang::JavaScript,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(sp(13)),
        EvidenceKind::Source(SourceFactKind::Call(SourceCallKind::Construct)),
        Vec::new(),
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(10), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Set"),
        }),
        Vec::new(),
    ));
    il.evidence.push(evidence(
        2,
        EvidenceAnchor::sequence(sp(12)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        Vec::new(),
    ));
    (il, call)
}

pub(super) fn js_typeof_call_il(interner: &Interner) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("typeof")),
        sp(42),
        &[],
    );
    let arg = b.add(NodeKind::Lit, Payload::LitInt(1), sp(43), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(44), &[callee, arg]);
    let root = b.add(NodeKind::Block, Payload::None, sp(44), &[call]);
    let il = b.finish(
        root,
        FileMeta {
            path: "t.ts".into(),
            lang: Lang::TypeScript,
        },
        Vec::new(),
        Vec::new(),
    );
    (il, call)
}

pub(super) fn raw_array_seq_il(interner: &Interner) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(60), &[]);
    let seq = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(61),
        &[one],
    );
    let root = b.add(NodeKind::Block, Payload::None, sp(59), &[seq]);
    let il = b.finish(
        root,
        FileMeta {
            path: "t.js".into(),
            lang: Lang::JavaScript,
        },
        Vec::new(),
        Vec::new(),
    );
    (il, seq)
}

pub(super) fn ts_contains_call_il(interner: &Interner) -> (Il, NodeId, Span) {
    let mut b = IlBuilder::new(FileId(0));
    let receiver_span = sp(50);
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("xs")),
        receiver_span,
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("includes")),
        sp(51),
        &[receiver],
    );
    let item = b.add(NodeKind::Lit, Payload::LitInt(7), sp(52), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(53), &[callee, item]);
    let root = b.add(NodeKind::Block, Payload::None, sp(49), &[call]);
    let il = b.finish(
        root,
        FileMeta {
            path: "t.ts".into(),
            lang: Lang::TypeScript,
        },
        Vec::new(),
        Vec::new(),
    );
    (il, call, receiver_span)
}

pub(super) fn canonical_python_abs_il() -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let arg = b.add(NodeKind::Lit, Payload::LitInt(-1), sp(71), &[]);
    let call = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Abs),
        sp(72),
        &[arg],
    );
    let root = b.add(NodeKind::Block, Payload::None, sp(70), &[call]);
    let il = b.finish(
        root,
        FileMeta {
            path: "t.py".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    (il, call)
}

use super::*;

fn canonical_builtin_call_il(
    lang: Lang,
    builtin: Builtin,
    args: &[NodeId],
    builder: IlBuilder,
    root: NodeId,
) -> (Il, NodeId) {
    let mut builder = builder;
    let call = builder.add(NodeKind::Call, Payload::Builtin(builtin), sp(40), args);
    let root = builder.add(NodeKind::Func, Payload::None, sp(41), &[root, call]);
    (finish_il(builder, root, lang), call)
}

fn python_len_canonical_call_il() -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    canonical_builtin_call_il(Lang::Python, Builtin::Len, &[arg], b, arg)
}

#[test]
fn canonical_builtin_admission_requires_language_core_or_library_api_evidence() {
    let (mut il, call) = python_len_canonical_call_il();

    assert!(!admitted_builtin_semantics_at_call(&il, call, Builtin::Len));

    let contract = library_free_function_builtin_contract(Lang::Python, "len", 1)
        .expect("Python len contract");
    il.evidence.push(library_api_record(
        10,
        il.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(admitted_builtin_semantics_at_call(&il, call, Builtin::Len));
    assert!(!admitted_builtin_semantics_at_call(&il, call, Builtin::Abs));
}

#[test]
fn rust_map_get_unwrap_or_canonical_builtin_uses_map_get_dependency() {
    let mut b = IlBuilder::new(FileId(0));
    let map = b.add(NodeKind::Var, Payload::Cid(0), sp(38), &[]);
    let key = b.add(NodeKind::Var, Payload::Cid(1), sp(39), &[]);
    let default = b.add(NodeKind::Lit, Payload::LitInt(0), sp(40), &[]);
    let (mut il, call) = canonical_builtin_call_il(
        Lang::Rust,
        Builtin::GetOrDefault,
        &[map, key, default],
        b,
        map,
    );
    let map_get = library_map_get_contract(Lang::Rust, "get", 1).expect("Rust map get contract");
    let unwrap_or =
        library_method_call_contract(Lang::Rust, "unwrap_or", 1).expect("Rust unwrap_or contract");

    il.evidence.push(library_api_record(
        10,
        sp(39),
        map_get.id,
        map_get.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    il.evidence.push(library_api_record(
        11,
        il.node(call).span,
        unwrap_or.id,
        unwrap_or.callee,
        EvidenceStatus::Asserted,
        &[10],
    ));

    assert!(admitted_builtin_semantics_at_call(
        &il,
        call,
        Builtin::GetOrDefault
    ));
    assert!(!admitted_builtin_semantics_at_call(
        &il,
        call,
        Builtin::ValueOrDefault
    ));
}

#[test]
fn rust_value_default_contract_alone_does_not_admit_map_default_builtin() {
    let mut b = IlBuilder::new(FileId(0));
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(38), &[]);
    let default = b.add(NodeKind::Lit, Payload::LitInt(0), sp(39), &[]);
    let (mut il, call) = canonical_builtin_call_il(
        Lang::Rust,
        Builtin::GetOrDefault,
        &[value, default],
        b,
        value,
    );
    let unwrap_or =
        library_method_call_contract(Lang::Rust, "unwrap_or", 1).expect("Rust unwrap_or contract");

    il.evidence.push(library_api_record(
        10,
        il.node(call).span,
        unwrap_or.id,
        unwrap_or.callee,
        EvidenceStatus::Asserted,
        &[],
    ));

    assert!(!admitted_builtin_semantics_at_call(
        &il,
        call,
        Builtin::GetOrDefault
    ));
}

#[test]
fn canonical_builtin_admission_fails_closed_on_bad_library_api_evidence() {
    let contract = library_free_function_builtin_contract(Lang::Python, "len", 1)
        .expect("Python len contract");

    let (mut broken, broken_call) = python_len_canonical_call_il();
    broken.evidence.push(library_api_record(
        10,
        broken.node(broken_call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[99],
    ));
    assert!(!admitted_builtin_semantics_at_call(
        &broken,
        broken_call,
        Builtin::Len
    ));

    let (mut ambiguous, ambiguous_call) = python_len_canonical_call_il();
    ambiguous.evidence.push(library_api_record(
        10,
        ambiguous.node(ambiguous_call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Ambiguous,
        &[],
    ));
    assert!(!admitted_builtin_semantics_at_call(
        &ambiguous,
        ambiguous_call,
        Builtin::Len
    ));

    let (mut conflicting, conflicting_call) = python_len_canonical_call_il();
    let abs = library_free_function_builtin_contract(Lang::Python, "abs", 1)
        .expect("Python abs contract");
    conflicting.evidence.push(library_api_record(
        10,
        conflicting.node(conflicting_call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    conflicting.evidence.push(library_api_record(
        11,
        conflicting.node(conflicting_call).span,
        abs.id,
        abs.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(!admitted_builtin_semantics_at_call(
        &conflicting,
        conflicting_call,
        Builtin::Len
    ));
}

#[test]
fn canonical_property_builtin_admission_accepts_field_span_evidence() {
    let mut b = IlBuilder::new(FileId(0));
    let collection = b.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let call = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Len),
        sp(40),
        &[collection],
    );
    let root = b.add(NodeKind::Func, Payload::None, sp(41), &[call]);
    let mut il = finish_il(b, root, Lang::JavaScript);
    let contract =
        library_property_builtin_contract(Lang::JavaScript, "length").expect("length contract");
    il.evidence.push(evidence_with_dependencies(
        10,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Field),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 0,
        }),
        EvidenceStatus::Asserted,
        Vec::new(),
    ));

    assert!(admitted_builtin_semantics_at_call(&il, call, Builtin::Len));

    let mut swift_len = IlBuilder::new(FileId(0));
    let collection = swift_len.add(NodeKind::Var, Payload::Cid(0), sp(50), &[]);
    let count = swift_len.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Len),
        sp(51),
        &[collection],
    );
    let root = swift_len.add(NodeKind::Func, Payload::None, sp(52), &[count]);
    let mut swift_len = finish_il(swift_len, root, Lang::Swift);
    let contract = library_property_builtin_contract(Lang::Swift, "count").expect("count contract");
    swift_len.evidence.push(evidence_with_dependencies(
        11,
        EvidenceAnchor::node(swift_len.node(count).span, NodeKind::Field),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 0,
        }),
        EvidenceStatus::Asserted,
        Vec::new(),
    ));
    assert!(admitted_builtin_semantics_at_call(
        &swift_len,
        count,
        Builtin::Len
    ));

    let mut swift_empty = IlBuilder::new(FileId(0));
    let collection = swift_empty.add(NodeKind::Var, Payload::Cid(0), sp(60), &[]);
    let is_empty = swift_empty.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::IsEmpty),
        sp(61),
        &[collection],
    );
    let root = swift_empty.add(NodeKind::Func, Payload::None, sp(62), &[is_empty]);
    let mut swift_empty = finish_il(swift_empty, root, Lang::Swift);
    let contract =
        library_property_builtin_contract(Lang::Swift, "isEmpty").expect("isEmpty contract");
    swift_empty.evidence.push(evidence_with_dependencies(
        12,
        EvidenceAnchor::node(swift_empty.node(is_empty).span, NodeKind::Field),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 0,
        }),
        EvidenceStatus::Asserted,
        Vec::new(),
    ));
    assert!(admitted_builtin_semantics_at_call(
        &swift_empty,
        is_empty,
        Builtin::IsEmpty
    ));
}

#[test]
fn canonical_builtin_admission_keeps_language_core_exceptions_narrow() {
    let mut go = IlBuilder::new(FileId(0));
    let key = go.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let map = go.add(NodeKind::Var, Payload::Cid(1), sp(40), &[]);
    let (go_il, contains) =
        canonical_builtin_call_il(Lang::Go, Builtin::Contains, &[key, map], go, map);
    assert!(admitted_builtin_semantics_at_call(
        &go_il,
        contains,
        Builtin::Contains
    ));

    let mut py = IlBuilder::new(FileId(0));
    let dict_key = py.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let dict_value = py.add(NodeKind::Var, Payload::Cid(1), sp(40), &[]);
    let (py_il, dict_entry) = canonical_builtin_call_il(
        Lang::Python,
        Builtin::DictEntry,
        &[dict_key, dict_value],
        py,
        dict_value,
    );
    assert!(admitted_builtin_semantics_at_call(
        &py_il,
        dict_entry,
        Builtin::DictEntry
    ));

    let mut raw_len = IlBuilder::new(FileId(0));
    let arg = raw_len.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let (go_len_il, go_len) =
        canonical_builtin_call_il(Lang::Go, Builtin::Len, &[arg], raw_len, arg);
    assert!(!admitted_builtin_semantics_at_call(
        &go_len_il,
        go_len,
        Builtin::Len
    ));
}

#[test]
fn c_unsigned_cast_builtin_admission_requires_source_cast_evidence() {
    let mut b = IlBuilder::new(FileId(0));
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let (mut il, call) =
        canonical_builtin_call_il(Lang::C, Builtin::UnsignedCast32, &[arg], b, arg);

    assert!(!admitted_builtin_semantics_at_call(
        &il,
        call,
        Builtin::UnsignedCast32
    ));

    il.evidence.push(evidence(
        10,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::Source(SourceFactKind::Cast(SourceCastKind::CUnsigned32)),
        EvidenceStatus::Asserted,
    ));
    assert!(admitted_builtin_semantics_at_call(
        &il,
        call,
        Builtin::UnsignedCast32
    ));
}

fn add_with_len_rhs_il() -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let px = b.add(NodeKind::Param, Payload::Cid(0), sp(50), &[]);
    let py = b.add(NodeKind::Param, Payload::Cid(1), sp(51), &[]);
    let x = b.add(NodeKind::Var, Payload::Cid(0), sp(52), &[]);
    let y = b.add(NodeKind::Var, Payload::Cid(1), sp(53), &[]);
    let len = b.add(NodeKind::Call, Payload::Builtin(Builtin::Len), sp(54), &[y]);
    let add = b.add(NodeKind::BinOp, Payload::Op(Op::Add), sp(55), &[x, len]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(56), &[add]);
    let root = b.add(NodeKind::Func, Payload::None, sp(57), &[px, py, ret]);
    (finish_il(b, root, Lang::Python), len)
}

#[test]
fn value_domain_inference_requires_admitted_builtin_result_domains() {
    let (mut il, len) = add_with_len_rhs_il();
    assert_eq!(
        semantics(Lang::Python)
            .operators()
            .infer_param_value_domains(&il, il.root),
        vec![ValueDomain::Unknown, ValueDomain::Unknown],
        "raw canonical Len payload must not prove a numeric result domain"
    );

    let contract = library_free_function_builtin_contract(Lang::Python, "len", 1)
        .expect("Python len contract");
    il.evidence.push(library_api_record(
        10,
        il.node(len).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert_eq!(
        semantics(Lang::Python)
            .operators()
            .infer_param_value_domains(&il, il.root),
        vec![ValueDomain::Number, ValueDomain::Unknown],
        "admitted Python len can contribute its numeric result domain"
    );
}

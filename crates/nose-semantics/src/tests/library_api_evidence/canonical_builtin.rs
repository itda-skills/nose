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

fn rust_integer_canonical_builtin_call_il(builtin: Builtin, arg_count: usize) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let args = (0..arg_count)
        .map(|idx| {
            b.add(
                NodeKind::Var,
                Payload::Cid(idx as u32),
                sp(90 + idx as u32),
                &[],
            )
        })
        .collect::<Vec<_>>();
    let root = args[0];
    canonical_builtin_call_il(Lang::Rust, builtin, &args, b, root)
}

fn java_math_canonical_builtin_call_il(builtin: Builtin, arg_count: usize) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let args = (0..arg_count)
        .map(|idx| {
            b.add(
                NodeKind::Var,
                Payload::Cid(idx as u32),
                sp(120 + idx as u32),
                &[],
            )
        })
        .collect::<Vec<_>>();
    let root = args[0];
    canonical_builtin_call_il(Lang::Java, builtin, &args, b, root)
}

fn push_java_math_canonical_dependencies(il: &mut Il, call: NodeId) -> Vec<u32> {
    let call_span = il.node(call).span;
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(call_span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Math"),
        }),
        EvidenceStatus::Asserted,
    ));
    let args = il.children(call).to_vec();
    let mut dependencies = vec![0];
    for (idx, arg) in args.into_iter().enumerate() {
        let id = 1 + idx as u32;
        il.evidence.push(evidence(
            id,
            EvidenceAnchor::node(il.node(arg).span, il.kind(arg)),
            EvidenceKind::Domain(DomainEvidence::Integer),
            EvidenceStatus::Asserted,
        ));
        dependencies.push(id);
    }
    dependencies
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
fn rust_integer_canonical_builtin_requires_integer_method_pack_provenance() {
    for (builtin, method, source_arity, canonical_arg_count) in [
        (Builtin::Abs, "abs", 0, 1),
        (Builtin::Min, "min", 1, 2),
        (Builtin::Max, "max", 1, 2),
    ] {
        let contract = library_scalar_integer_method_contract(Lang::Rust, method, source_arity)
            .expect("Rust integer method contract");

        let (mut wrong_pack, call) =
            rust_integer_canonical_builtin_call_il(builtin, canonical_arg_count);
        let receiver = wrong_pack.children(call)[0];
        wrong_pack.evidence.push(evidence(
            0,
            EvidenceAnchor::node(wrong_pack.node(receiver).span, wrong_pack.kind(receiver)),
            EvidenceKind::Domain(DomainEvidence::Integer),
            EvidenceStatus::Asserted,
        ));
        wrong_pack
            .evidence
            .push(library_api_record_with_provenance_and_arity(
                1,
                wrong_pack.node(call).span,
                contract.id,
                contract.callee,
                source_arity as u16,
                EvidenceStatus::Asserted,
                &[0],
                FIRST_PARTY_PACK_ID,
                RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID,
            ));
        assert!(
            !admitted_builtin_semantics_at_call(&wrong_pack, call, builtin),
            "canonical Rust {method} builtin must reject compatibility-pack evidence"
        );

        let (mut admitted, call) =
            rust_integer_canonical_builtin_call_il(builtin, canonical_arg_count);
        let receiver = admitted.children(call)[0];
        admitted.evidence.push(evidence(
            0,
            EvidenceAnchor::node(admitted.node(receiver).span, admitted.kind(receiver)),
            EvidenceKind::Domain(DomainEvidence::Integer),
            EvidenceStatus::Asserted,
        ));
        admitted.evidence.push(rust_stdlib_integer_method_record(
            1,
            admitted.node(call).span,
            contract.id,
            contract.callee,
            source_arity as u16,
            EvidenceStatus::Asserted,
            &[0],
        ));
        assert!(
            admitted_builtin_semantics_at_call(&admitted, call, builtin),
            "canonical Rust {method} builtin should admit the builtin-pack evidence"
        );
    }
}

#[test]
fn java_math_canonical_builtin_requires_math_pack_provenance() {
    for (builtin, method, source_arity, canonical_arg_count) in [
        (Builtin::Abs, "abs", 1, 1),
        (Builtin::Min, "min", 2, 2),
        (Builtin::Max, "max", 2, 2),
    ] {
        let contract = library_scalar_integer_method_contract(Lang::Java, method, source_arity)
            .expect("Java Math integer method contract");

        let (mut missing_dependency, call) =
            java_math_canonical_builtin_call_il(builtin, canonical_arg_count);
        missing_dependency.evidence.push(java_stdlib_math_record(
            1,
            missing_dependency.node(call).span,
            contract.id,
            contract.callee,
            source_arity as u16,
            EvidenceStatus::Asserted,
            &[],
        ));
        assert!(
            !admitted_builtin_semantics_at_call(&missing_dependency, call, builtin),
            "canonical Java Math {method} builtin must reject evidence without Math/integer dependencies"
        );

        let (mut wrong_pack, call) =
            java_math_canonical_builtin_call_il(builtin, canonical_arg_count);
        let dependencies = push_java_math_canonical_dependencies(&mut wrong_pack, call);
        wrong_pack
            .evidence
            .push(library_api_record_with_provenance_and_arity(
                10,
                wrong_pack.node(call).span,
                contract.id,
                contract.callee,
                source_arity as u16,
                EvidenceStatus::Asserted,
                &dependencies,
                FIRST_PARTY_PACK_ID,
                JAVA_STDLIB_MATH_PRODUCER_ID,
            ));
        assert!(
            !admitted_builtin_semantics_at_call(&wrong_pack, call, builtin),
            "canonical Java Math {method} builtin must reject compatibility-pack evidence"
        );

        let LibraryApiCalleeContract::Method {
            method: callee_method,
            ..
        } = contract.callee
        else {
            unreachable!("Java Math contract is a method contract");
        };
        let forged_callee = LibraryApiCalleeContract::Method {
            method: callee_method,
            receiver: MethodReceiverContract::ExactInteger,
        };
        let (mut unresolved_callee_il, call) =
            java_math_canonical_builtin_call_il(builtin, canonical_arg_count);
        let dependencies = push_java_math_canonical_dependencies(&mut unresolved_callee_il, call);
        unresolved_callee_il.evidence.push(java_stdlib_math_record(
            10,
            unresolved_callee_il.node(call).span,
            contract.id,
            forged_callee,
            source_arity as u16,
            EvidenceStatus::Asserted,
            &dependencies,
        ));
        assert!(
            !admitted_builtin_semantics_at_call(&unresolved_callee_il, call, builtin),
            "canonical Java Math {method} builtin must reject unresolved callee hashes"
        );

        let (mut admitted, call) =
            java_math_canonical_builtin_call_il(builtin, canonical_arg_count);
        let dependencies = push_java_math_canonical_dependencies(&mut admitted, call);
        admitted.evidence.push(java_stdlib_math_record(
            10,
            admitted.node(call).span,
            contract.id,
            contract.callee,
            source_arity as u16,
            EvidenceStatus::Asserted,
            &dependencies,
        ));
        assert!(
            admitted_builtin_semantics_at_call(&admitted, call, builtin),
            "canonical Java Math {method} builtin should admit the math-pack evidence"
        );
    }
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
    assert!(
        !admitted_builtin_semantics_at_call(&il, call, Builtin::UnsignedCast32),
        "wrong-pack source facts must not admit C unsigned-cast semantics"
    );

    let mut b = IlBuilder::new(FileId(0));
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(39), &[]);
    let (mut il, call) =
        canonical_builtin_call_il(Lang::C, Builtin::UnsignedCast32, &[arg], b, arg);
    il.evidence.push(c_unsigned_32_source_cast_evidence(
        10,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceStatus::Asserted,
        Vec::new(),
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

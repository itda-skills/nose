use super::*;

fn rust_integer_method_call_il(method: &str, arg_count: usize) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(120), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(121),
        &[receiver],
    );
    let args = (0..arg_count)
        .map(|idx| {
            b.add(
                NodeKind::Lit,
                Payload::LitInt(idx as i64),
                sp(122 + idx as u32),
                &[],
            )
        })
        .collect::<Vec<_>>();
    let mut children = Vec::with_capacity(args.len() + 1);
    children.push(callee);
    children.extend(args);
    let call = b.add(NodeKind::Call, Payload::None, sp(130), &children);
    let root = b.add(NodeKind::Func, Payload::None, sp(131), &[call]);
    (finish_il(b, root, Lang::Rust), interner, call, receiver)
}

fn push_integer_receiver_dependency(il: &mut Il, receiver: NodeId) {
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(il.node(receiver).span, il.kind(receiver)),
        EvidenceKind::Domain(DomainEvidence::Integer),
        EvidenceStatus::Asserted,
    ));
}

fn java_math_call_il(method: &str) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let math = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Math")),
        sp(140),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(141),
        &[math],
    );
    let arg = b.add(NodeKind::Var, Payload::Cid(0), sp(142), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(143), &[callee, arg]);
    let root = b.add(NodeKind::Func, Payload::None, sp(144), &[call]);
    (finish_il(b, root, Lang::Java), interner, call, math)
}

#[test]
fn admitted_rust_scalar_integer_method_requires_integer_method_builtin_pack_provenance() {
    let (mut raw_shape, interner, call, receiver) = rust_integer_method_call_il("clamp", 2);
    push_integer_receiver_dependency(&mut raw_shape, receiver);
    assert!(
        admitted_scalar_integer_method_at_call(&raw_shape, &interner, call).is_none(),
        "raw Rust integer method shape plus integer receiver proof is not enough"
    );

    let contract =
        library_scalar_integer_method_contract(Lang::Rust, "clamp", 2).expect("clamp contract");

    let (mut missing_dependency, interner, call, _receiver) =
        rust_integer_method_call_il("clamp", 2);
    missing_dependency
        .evidence
        .push(rust_stdlib_integer_method_record(
            1,
            missing_dependency.node(call).span,
            contract.id,
            contract.callee,
            2,
            EvidenceStatus::Asserted,
            &[],
        ));
    assert!(
        admitted_scalar_integer_method_at_call(&missing_dependency, &interner, call).is_none(),
        "same-span Rust integer method evidence without integer receiver proof is rejected"
    );

    let (mut wrong_pack, interner, call, receiver) = rust_integer_method_call_il("clamp", 2);
    push_integer_receiver_dependency(&mut wrong_pack, receiver);
    wrong_pack
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            1,
            wrong_pack.node(call).span,
            contract.id,
            contract.callee,
            2,
            EvidenceStatus::Asserted,
            &[0],
            FIRST_PARTY_PACK_ID,
            RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID,
        ));
    assert!(
        admitted_scalar_integer_method_at_call(&wrong_pack, &interner, call).is_none(),
        "Rust integer method evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = rust_integer_method_call_il("clamp", 2);
    push_integer_receiver_dependency(&mut wrong_producer, receiver);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance_and_arity(
            1,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            2,
            EvidenceStatus::Asserted,
            &[0],
            RUST_STDLIB_INTEGER_METHOD_PACK_ID,
            "wrong.rust.stdlib.integer-method-api",
        ));
    assert!(
        admitted_scalar_integer_method_at_call(&wrong_producer, &interner, call).is_none(),
        "Rust integer method evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, receiver) = rust_integer_method_call_il("clamp", 2);
    push_integer_receiver_dependency(&mut wrong_emitter, receiver);
    let mut external_record = rust_stdlib_integer_method_record(
        1,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        2,
        EvidenceStatus::Asserted,
        &[0],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_scalar_integer_method_at_call(&wrong_emitter, &interner, call).is_none(),
        "Rust integer method evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, receiver) = rust_integer_method_call_il("clamp", 2);
    push_integer_receiver_dependency(&mut admitted, receiver);
    admitted.evidence.push(rust_stdlib_integer_method_record(
        1,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        2,
        EvidenceStatus::Asserted,
        &[0],
    ));
    let occurrence = admitted_scalar_integer_method_at_call(&admitted, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::ScalarIntegerMethod(ScalarIntegerMethod::Clamp)
    );
    assert_eq!(occurrence.receiver, Some(receiver));
    assert_eq!(occurrence.arg_count, 2);
}

#[test]
fn admitted_java_scalar_integer_method_keeps_first_party_compatibility_provenance() {
    let (mut il, interner, call, math) = java_math_call_il("abs");
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(il.node(math).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Math"),
        }),
        EvidenceStatus::Asserted,
    ));
    let contract = library_scalar_integer_method_contract(Lang::Java, "abs", 1)
        .expect("Java Math.abs contract");
    il.evidence.push(library_api_record_with_provenance(
        1,
        il.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
        FIRST_PARTY_PACK_ID,
        "library_api_scalar_integer_method",
    ));
    let occurrence = admitted_scalar_integer_method_at_call(&il, &interner, call).unwrap();
    assert_eq!(
        occurrence.contract.id,
        LibraryApiContractId::ScalarIntegerMethod(ScalarIntegerMethod::Abs)
    );
    assert_eq!(occurrence.receiver, Some(math));
    assert_eq!(occurrence.arg_count, 1);
}

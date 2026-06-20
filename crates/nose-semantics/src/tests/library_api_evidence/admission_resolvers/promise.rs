use super::*;

fn js_promise_then_call_il() -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(77), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(78),
        &[receiver],
    );
    let callback = b.add(NodeKind::Lambda, Payload::None, sp(79), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(80), &[callee, callback]);
    let root = b.add(NodeKind::Func, Payload::None, sp(81), &[call]);
    (
        finish_il(b, root, Lang::JavaScript),
        interner,
        call,
        receiver,
    )
}

fn js_promise_resolve_call_il() -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let promise = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Promise")),
        sp(82),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("resolve")),
        sp(83),
        &[promise],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(84), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(85), &[callee, value]);
    let root = b.add(NodeKind::Func, Payload::None, sp(86), &[call]);
    (
        finish_il(b, root, Lang::JavaScript),
        interner,
        call,
        callee,
        promise,
    )
}

fn push_promise_resolve_dependencies(il: &mut Il, callee: NodeId, promise: NodeId) {
    il.evidence.push(evidence_with_dependencies(
        0,
        EvidenceAnchor::source_span(il.node(callee).span),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Promise"),
        }),
        EvidenceStatus::Asserted,
        vec![],
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(il.node(callee).span, NodeKind::Field),
        EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal {
            path_hash: stable_symbol_hash("Promise.resolve"),
        }),
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    il.evidence.push(evidence_with_dependencies(
        2,
        EvidenceAnchor::node(il.node(promise).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Promise"),
        }),
        EvidenceStatus::Asserted,
        vec![],
    ));
}

#[test]
fn admitted_promise_then_resolver_requires_future_receiver_proof() {
    let (il, interner, call, _receiver) = js_promise_then_call_il();
    assert!(
        admitted_promise_then_at_call(&il, &interner, call).is_none(),
        "raw JS-like .then(...) shape alone must not admit promise continuation semantics"
    );

    let contract =
        library_promise_then_contract(Lang::JavaScript, "then", 1).expect("Promise.then contract");
    let (mut api_only, interner, call, _receiver) = js_promise_then_call_il();
    api_only.evidence.push(library_api_record(
        0,
        api_only.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[],
    ));
    assert!(
        admitted_promise_then_at_call(&api_only, &interner, call).is_none(),
        "Promise.then API occurrence remains closed until Promise-like receiver proof exists"
    );

    let (mut wrong_pack, interner, call, receiver) = js_promise_then_call_il();
    wrong_pack.evidence.push(evidence_with_dependencies(
        0,
        EvidenceAnchor::node(wrong_pack.node(receiver).span, wrong_pack.kind(receiver)),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        EvidenceStatus::Asserted,
        vec![],
    ));
    wrong_pack.evidence.push(library_api_record_with_provenance(
        1,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
        FIRST_PARTY_PACK_ID,
        JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
    ));
    assert!(
        admitted_promise_then_at_call(&wrong_pack, &interner, call).is_none(),
        "Promise.then evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, receiver) = js_promise_then_call_il();
    wrong_producer.evidence.push(evidence_with_dependencies(
        0,
        EvidenceAnchor::node(
            wrong_producer.node(receiver).span,
            wrong_producer.kind(receiver),
        ),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        EvidenceStatus::Asserted,
        vec![],
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
            JS_LIKE_BUILTIN_PROMISE_PACK_ID,
            "wrong.javascript.builtins.promise-api",
        ));
    assert!(
        admitted_promise_then_at_call(&wrong_producer, &interner, call).is_none(),
        "Promise.then evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, receiver) = js_promise_then_call_il();
    wrong_emitter.evidence.push(evidence_with_dependencies(
        0,
        EvidenceAnchor::node(
            wrong_emitter.node(receiver).span,
            wrong_emitter.kind(receiver),
        ),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        EvidenceStatus::Asserted,
        vec![],
    ));
    let mut external_record = js_like_builtin_promise_record(
        1,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_promise_then_at_call(&wrong_emitter, &interner, call).is_none(),
        "Promise.then evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, receiver) = js_promise_then_call_il();
    admitted.evidence.push(evidence_with_dependencies(
        0,
        EvidenceAnchor::node(admitted.node(receiver).span, admitted.kind(receiver)),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        EvidenceStatus::Asserted,
        vec![],
    ));
    admitted.evidence.push(js_like_builtin_promise_record(
        1,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    ));
    let resolved = admitted_promise_then_at_call(&admitted, &interner, call)
        .expect("PromiseLike receiver dependency admits Promise.then");
    assert_eq!(resolved.contract.id, LibraryApiContractId::PromiseThen);
    assert_eq!(resolved.receiver, Some(receiver));
}

#[test]
fn admitted_promise_resolve_resolver_requires_qualified_global_proof() {
    let (il, interner, call, _callee, _promise) = js_promise_resolve_call_il();
    assert!(
        admitted_promise_resolve_at_call(&il, &interner, call).is_none(),
        "raw Promise.resolve(...) shape alone must not admit promise factory semantics"
    );

    let contract = library_promise_resolve_contract(Lang::JavaScript, "Promise", "resolve", 1)
        .expect("Promise.resolve contract");

    let (mut wrong_pack, interner, call, callee, promise) = js_promise_resolve_call_il();
    push_promise_resolve_dependencies(&mut wrong_pack, callee, promise);
    wrong_pack.evidence.push(library_api_record_with_provenance(
        3,
        wrong_pack.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1, 2],
        FIRST_PARTY_PACK_ID,
        JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
    ));
    assert!(
        admitted_promise_resolve_at_call(&wrong_pack, &interner, call).is_none(),
        "Promise.resolve evidence under the compatibility pack is rejected"
    );

    let (mut wrong_producer, interner, call, callee, promise) = js_promise_resolve_call_il();
    push_promise_resolve_dependencies(&mut wrong_producer, callee, promise);
    wrong_producer
        .evidence
        .push(library_api_record_with_provenance(
            3,
            wrong_producer.node(call).span,
            contract.id,
            contract.callee,
            EvidenceStatus::Asserted,
            &[1, 2],
            JS_LIKE_BUILTIN_PROMISE_PACK_ID,
            "wrong.javascript.builtins.promise-api",
        ));
    assert!(
        admitted_promise_resolve_at_call(&wrong_producer, &interner, call).is_none(),
        "Promise.resolve evidence with the wrong producer is rejected"
    );

    let (mut wrong_emitter, interner, call, callee, promise) = js_promise_resolve_call_il();
    push_promise_resolve_dependencies(&mut wrong_emitter, callee, promise);
    let mut external_record = js_like_builtin_promise_record(
        3,
        wrong_emitter.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1, 2],
    );
    external_record.provenance.emitter = EvidenceEmitter::External;
    wrong_emitter.evidence.push(external_record);
    assert!(
        admitted_promise_resolve_at_call(&wrong_emitter, &interner, call).is_none(),
        "Promise.resolve evidence from an external emitter is rejected"
    );

    let (mut admitted, interner, call, callee, promise) = js_promise_resolve_call_il();
    push_promise_resolve_dependencies(&mut admitted, callee, promise);
    admitted.evidence.push(js_like_builtin_promise_record(
        3,
        admitted.node(call).span,
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1, 2],
    ));
    let resolved = admitted_promise_resolve_at_call(&admitted, &interner, call)
        .expect("qualified global and unshadowed receiver admit Promise.resolve");
    assert_eq!(
        resolved.contract.id,
        LibraryApiContractId::PromiseFactory(PromiseFactoryKind::Resolve)
    );
    assert_eq!(resolved.receiver, Some(promise));
}

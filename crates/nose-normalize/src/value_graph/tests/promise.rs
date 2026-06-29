use super::support::*;

#[test]
fn promise_then_over_resolve_reduces_behind_promise_boundary() {
    let (mut il, interner, then_call, sync_add) = promise_resolve_then_call_il(true);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(then_call, &FxHashMap::default());
    let payload = {
        let node = &builder.nodes[promise_value as usize];
        assert!(
            matches!(
                node.op,
                ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
            ),
            "expected resolved Promise boundary, got {}",
            val_op_name(&node.op)
        );
        *node
            .args
            .first()
            .expect("Promise boundary wraps one payload")
    };
    assert!(matches!(
        builder.nodes[payload as usize].op,
        ValOp::Bin(op) if op == Op::Add as u32
    ));

    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
    assert_ne!(
        promise_value, sync_value,
        "Promise-returning continuation must not converge with a synchronous payload"
    );
}

#[test]
fn promise_then_returning_resolve_flattens_into_single_promise_boundary() {
    let (mut il, interner, then_call) = promise_then_returning_factory_il("resolve");
    let resolve_call = il.children(il.children(then_call)[0])[0];
    let callback = il.children(then_call)[1];
    let returned_resolve_call = il.children(callback)[1];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_resolve_evidence(&mut il, returned_resolve_call, 10);
    push_promise_then_evidence(&mut il, &interner, then_call, 20);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(then_call, &FxHashMap::default());
    let node = &builder.nodes[promise_value as usize];
    assert!(matches!(
        node.op,
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
    let payload = *node.args.first().expect("Promise boundary wraps payload");
    assert!(
        !matches!(
            builder.nodes[payload as usize].op,
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
        ),
        "handler-returned Promise.resolve must be assimilated rather than nested"
    );
    assert!(matches!(
        builder.nodes[payload as usize].op,
        ValOp::Bin(op) if op == Op::Add as u32
    ));
}

#[test]
fn promise_reject_catch_recovers_rejection_to_fulfilled_boundary() {
    let (mut il, interner, catch_call, sync_add) = promise_reject_catch_call_il();
    let reject_call = il.children(il.children(catch_call)[0])[0];
    push_promise_reject_evidence(&mut il, reject_call, 0);
    push_promise_catch_evidence(&mut il, &interner, catch_call, 5);
    assert!(
        nose_semantics::admitted_promise_resolve_at_call(&il, &interner, reject_call).is_some(),
        "Promise.reject factory evidence should admit the rejected channel"
    );
    assert!(
        nose_semantics::admitted_promise_catch_at_call(&il, &interner, catch_call).is_some(),
        "Promise.catch continuation evidence should admit the recovery channel"
    );

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(catch_call, &FxHashMap::default());
    let payload = {
        let node = &builder.nodes[promise_value as usize];
        assert!(
            matches!(
                node.op,
                ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
            ),
            "expected resolved Promise boundary, got {}",
            val_op_name(&node.op)
        );
        *node.args.first().expect("Promise boundary wraps payload")
    };

    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
    assert_ne!(
        promise_value, sync_value,
        "recovered catch result must remain behind a Promise boundary"
    );
}

#[test]
fn promise_reject_then_rejection_handler_recovers_like_catch() {
    let (mut il, interner, then_call) = promise_reject_then_rejection_call_il();
    let reject_call = il.children(il.children(then_call)[0])[0];
    push_promise_reject_evidence(&mut il, reject_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn promise_then_returning_reject_preserves_rejection_channel() {
    let (mut il, interner, then_call) = promise_then_returning_factory_il("reject");
    let resolve_call = il.children(il.children(then_call)[0])[0];
    let callback = il.children(then_call)[1];
    let returned_reject_call = il.children(callback)[1];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_reject_evidence(&mut il, returned_reject_call, 10);
    push_promise_then_evidence(&mut il, &interner, then_call, 20);

    assert!(matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_REJECTED_CODE
    ));
}

#[test]
fn promise_then_returning_possible_thenable_stays_opaque() {
    let (mut il, interner, then_call) = promise_then_returning_unknown_il();
    let resolve_call = il.children(il.children(then_call)[0])[0];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE || code == PROMISE_REJECTED_CODE
    ));
}

#[test]
fn promise_then_over_possible_thenable_resolve_arg_stays_opaque() {
    let (mut il, interner, then_call, _sync_add) = promise_resolve_then_call_il(false);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn promise_then_over_explicit_thenable_resolve_arg_stays_opaque() {
    let (mut il, interner, then_call, _sync_add) = promise_resolve_then_call_il(false);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    let resolve_arg = il.children(resolve_call)[1];
    push_domain_evidence(&mut il, resolve_arg, 20, DomainEvidence::PromiseLike);
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn promise_like_receiver_without_supported_settled_producer_stays_opaque() {
    let (mut il, interner, then_call) = promise_like_receiver_then_call_il();
    let then_callee = il.children(then_call)[0];
    let receiver = il.children(then_callee)[0];
    push_domain_evidence(&mut il, receiver, 0, DomainEvidence::PromiseLike);
    push_promise_then_evidence(&mut il, &interner, then_call, 1);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn imported_promise_then_with_fulfilled_contract_recovers_payload_boundary() {
    let ImportedPromiseFixture {
        mut il,
        interner,
        producer_call,
        producer_payload,
        continuation_call,
        sync_add,
    } = imported_promise_then_call_il(true);
    push_imported_function_promise_settlement_evidence(
        &mut il,
        &interner,
        producer_call,
        producer_payload,
        PromiseSettlementChannel::Fulfilled,
        100,
    );
    push_promise_then_evidence(&mut il, &interner, continuation_call, 110);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(continuation_call, &FxHashMap::default());
    let payload = assert_resolved_promise_boundary(&builder, promise_value);
    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
    assert_ne!(
        promise_value, sync_value,
        "imported Promise recovery must preserve the async boundary"
    );
}

#[test]
fn imported_promise_catch_with_rejected_contract_recovers_payload_boundary() {
    let ImportedPromiseFixture {
        mut il,
        interner,
        producer_call,
        producer_payload,
        continuation_call,
        sync_add,
    } = imported_promise_catch_call_il();
    push_imported_function_promise_settlement_evidence(
        &mut il,
        &interner,
        producer_call,
        producer_payload,
        PromiseSettlementChannel::Rejected,
        100,
    );
    push_promise_catch_evidence(&mut il, &interner, continuation_call, 110);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(continuation_call, &FxHashMap::default());
    let payload = assert_resolved_promise_boundary(&builder, promise_value);
    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
}

#[test]
fn imported_promise_then_without_settled_contract_stays_opaque() {
    let ImportedPromiseFixture {
        mut il,
        interner,
        producer_call,
        continuation_call,
        ..
    } = imported_promise_then_call_il(true);
    push_domain_evidence(&mut il, producer_call, 100, DomainEvidence::PromiseLike);
    push_promise_then_evidence(&mut il, &interner, continuation_call, 110);

    assert!(!matches!(
        eval_op(&il, &interner, continuation_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE || code == PROMISE_REJECTED_CODE
    ));
}

#[test]
fn imported_promise_fulfilled_contract_with_possible_thenable_payload_stays_opaque() {
    let ImportedPromiseFixture {
        mut il,
        interner,
        producer_call,
        producer_payload,
        continuation_call,
        ..
    } = imported_promise_then_call_il(false);
    push_imported_function_promise_settlement_evidence(
        &mut il,
        &interner,
        producer_call,
        producer_payload,
        PromiseSettlementChannel::Fulfilled,
        100,
    );
    push_promise_then_evidence(&mut il, &interner, continuation_call, 110);

    assert!(!matches!(
        eval_op(&il, &interner, continuation_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE || code == PROMISE_REJECTED_CODE
    ));
}

#[test]
fn direct_method_promise_return_then_recovers_without_sync_erasure() {
    let DirectMethodPromiseFixture {
        mut il,
        interner,
        method,
        method_call,
        method_root,
        resolve_call,
        sync_add,
        then_call,
    } = direct_method_promise_then_fixture(false);
    il.evidence.push(language_core_evidence(
        100,
        Lang::TypeScript,
        EvidenceAnchor::node(il.node(method_call).span, NodeKind::Call),
        EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectMethod {
            target_span: il.node(method_root).span,
            receiver_type_hash: stable_symbol_hash("Worker"),
            method_hash: interner.symbol_hash(method),
        }),
    ));
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    crate::call_target_evidence::run(&mut il, &interner);
    assert_eq!(
        nose_semantics::domain_evidence_for_receiver(&il, &interner, method_call),
        Some(DomainEvidence::PromiseLike),
        "direct method call result should gain PromiseLike receiver proof"
    );
    push_promise_then_evidence(&mut il, &interner, then_call, 200);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(then_call, &FxHashMap::default());
    let payload = {
        let node = &builder.nodes[promise_value as usize];
        assert!(
            matches!(
                node.op,
                ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
            ),
            "expected resolved Promise boundary, got {}",
            val_op_name(&node.op)
        );
        *node.args.first().expect("Promise boundary wraps payload")
    };
    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
    assert_ne!(
        promise_value, sync_value,
        "direct method Promise return recovery must preserve the Promise boundary"
    );
}

#[test]
fn direct_method_promise_return_stays_closed_when_return_uses_receiver_context() {
    let DirectMethodPromiseFixture {
        mut il,
        interner,
        method,
        method_call,
        method_root,
        resolve_call,
        then_call,
        ..
    } = direct_method_promise_then_fixture(true);
    il.evidence.push(language_core_evidence(
        100,
        Lang::TypeScript,
        EvidenceAnchor::node(il.node(method_call).span, NodeKind::Call),
        EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectMethod {
            target_span: il.node(method_root).span,
            receiver_type_hash: stable_symbol_hash("Worker"),
            method_hash: interner.symbol_hash(method),
        }),
    ));
    let resolve_arg = il.children(resolve_call)[1];
    push_domain_evidence(&mut il, resolve_arg, 10, DomainEvidence::Number);
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    crate::call_target_evidence::run(&mut il, &interner);
    push_promise_then_evidence(&mut il, &interner, then_call, 200);

    assert!(
        !matches!(
            eval_op(&il, &interner, then_call),
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
        ),
        "DirectMethod return recovery must not evaluate methods that depend on receiver context"
    );
}

#[test]
fn direct_function_branching_promise_returns_recover_fulfilled_channel() {
    let BranchingPromiseFixture {
        mut il,
        interner,
        resolve_calls,
        then_call,
    } = direct_function_branching_promise_then_fixture(false);
    assert_branch_resolve_evidence_admits(&mut il, &interner, &resolve_calls);
    crate::call_target_evidence::run(&mut il, &interner);
    assert_branch_resolve_calls_remain_admitted(&il, &interner, &resolve_calls);
    let receiver = il.children(il.children(then_call)[0])[0];
    assert_eq!(
        nose_semantics::domain_evidence_for_receiver(&il, &interner, receiver),
        Some(DomainEvidence::PromiseLike),
        "branching direct function call result should gain PromiseLike receiver proof"
    );
    push_promise_then_evidence(&mut il, &interner, then_call, 200);
    assert!(
        nose_semantics::admitted_promise_then_at_call(&il, &interner, then_call).is_some(),
        "branching direct function receiver should admit Promise.then evidence"
    );

    assert_branching_direct_body_evaluates_to_resolved_phi(&il, &interner, receiver);
    assert_then_call_recovers_resolved_add_boundary(&il, &interner, then_call);
}

#[test]
fn direct_function_mixed_fulfilled_rejected_branch_stays_closed() {
    let BranchingPromiseFixture {
        mut il,
        interner,
        resolve_calls,
        then_call,
    } = direct_function_branching_promise_then_fixture(true);
    push_promise_resolve_evidence(&mut il, resolve_calls[0], 100);
    push_promise_reject_evidence(&mut il, resolve_calls[1], 110);
    crate::call_target_evidence::run(&mut il, &interner);
    push_promise_then_evidence(&mut il, &interner, then_call, 200);

    assert!(
        !matches!(
            eval_op(&il, &interner, then_call),
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE || code == PROMISE_REJECTED_CODE
        ),
        "mixed fulfilled/rejected producer branches need channel-specific control-flow proof"
    );
}

fn assert_branch_resolve_evidence_admits(
    il: &mut Il,
    interner: &Interner,
    resolve_calls: &[NodeId; 2],
) {
    for (idx, &resolve_call) in resolve_calls.iter().enumerate() {
        push_promise_resolve_evidence(il, resolve_call, 100 + 10 * idx as u32);
        assert!(
            nose_semantics::admitted_promise_resolve_at_call(il, interner, resolve_call).is_some(),
            "branch Promise.resolve call should admit factory evidence"
        );
    }
}

fn assert_branch_resolve_calls_remain_admitted(
    il: &Il,
    interner: &Interner,
    resolve_calls: &[NodeId; 2],
) {
    for &resolve_call in resolve_calls {
        assert!(
            nose_semantics::admitted_promise_resolve_at_call(il, interner, resolve_call).is_some(),
            "branch Promise.resolve call should remain admitted after call-target evidence"
        );
    }
}

fn assert_branching_direct_body_evaluates_to_resolved_phi(
    il: &Il,
    interner: &Interner,
    receiver: NodeId,
) {
    let mut builder = Builder::new(il, interner);
    let receiver_value = builder
        .eval_direct_function_return_call(receiver, &FxHashMap::default())
        .expect("branching direct function body should evaluate behind the sink fence");
    assert!(
        matches!(builder.nodes[receiver_value as usize].op, ValOp::Phi),
        "branching direct function producer should evaluate to a Phi of Promise boundaries"
    );
    let branch_values = builder.nodes[receiver_value as usize].args.clone();
    assert_resolved_promise_boundary(&builder, branch_values[1]);
    assert_resolved_promise_boundary(&builder, branch_values[2]);
}

fn assert_then_call_recovers_resolved_add_boundary(
    il: &Il,
    interner: &Interner,
    then_call: NodeId,
) {
    let mut builder = Builder::new(il, interner);
    let promise_value = builder.eval(then_call, &FxHashMap::default());
    let payload = assert_resolved_promise_boundary(&builder, promise_value);
    assert!(
        matches!(builder.nodes[payload as usize].op, ValOp::Bin(op) if op == Op::Add as u32),
        "fulfilled branch payloads should flow through the continuation"
    );
    assert_ne!(
        promise_value, payload,
        "branching Promise continuation recovery must preserve the Promise boundary"
    );
}

struct DirectMethodPromiseFixture {
    il: Il,
    interner: Interner,
    method: Symbol,
    method_root: NodeId,
    method_call: NodeId,
    resolve_call: NodeId,
    then_call: NodeId,
    sync_add: NodeId,
}

fn direct_method_promise_then_fixture(uses_receiver_context: bool) -> DirectMethodPromiseFixture {
    let interner = Interner::new();
    let method = interner.intern("load");
    let worker = interner.intern("worker");
    let mut b = IlBuilder::new(FileId(0));

    let promise = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Promise")),
        sp(210),
        &[],
    );
    let resolve_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("resolve")),
        sp(211),
        &[promise],
    );
    let resolve_arg = if uses_receiver_context {
        let this_value = b.add(
            NodeKind::Var,
            Payload::Name(interner.intern("this")),
            sp(212),
            &[],
        );
        b.add(
            NodeKind::Field,
            Payload::Name(interner.intern("value")),
            sp(213),
            &[this_value],
        )
    } else {
        b.add(NodeKind::Lit, Payload::LitInt(1), sp(212), &[])
    };
    let resolve_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(214),
        &[resolve_callee, resolve_arg],
    );
    let method_ret = b.add(NodeKind::Return, Payload::None, sp(215), &[resolve_call]);
    let method_body = b.add(NodeKind::Block, Payload::None, sp(216), &[method_ret]);
    let method_root = b.add(NodeKind::Func, Payload::None, sp(217), &[method_body]);

    let receiver = b.add(NodeKind::Var, Payload::Name(worker), sp(220), &[]);
    let method_callee = b.add(NodeKind::Field, Payload::Name(method), sp(221), &[receiver]);
    let method_call = b.add(NodeKind::Call, Payload::None, sp(222), &[method_callee]);
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(223),
        &[method_call],
    );
    let callback = add_increment_lambda(&mut b, 224, 1);
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(229),
        &[then_callee, callback],
    );
    let sync_add = add_sync_add(&mut b, 230);
    let root = b.add(
        NodeKind::Block,
        Payload::None,
        sp(233),
        &[method_root, then_call, sync_add],
    );
    let il = b.finish(
        root,
        FileMeta {
            path: "t".into(),
            lang: Lang::TypeScript,
        },
        vec![Unit {
            root: method_root,
            kind: UnitKind::Method,
            name: Some(method),
            origin: Default::default(),
        }],
        Vec::new(),
    );
    DirectMethodPromiseFixture {
        il,
        interner,
        method,
        method_root,
        method_call,
        resolve_call,
        then_call,
        sync_add,
    }
}

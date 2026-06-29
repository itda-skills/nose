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
        assert!(matches!(
            node.op,
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
        ));
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
        assert!(matches!(
            node.op,
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
        ));
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

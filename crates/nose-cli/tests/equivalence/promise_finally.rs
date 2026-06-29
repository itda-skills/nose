use super::*;

#[test]
fn proven_promise_finally_recovers_settlement_passthrough_without_sync_erasure() {
    let i = Interner::new();
    let finally_form =
        "function f() {\n  return Promise.resolve(1).finally(() => 9).then(x => x + 1);\n}\n";
    let direct_form = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let rejected_finally =
        "function f() {\n  return Promise.reject(1).finally(() => 9).catch(e => e + 1);\n}\n";
    let rejected_direct = "function f() {\n  return Promise.reject(1).catch(e => e + 1);\n}\n";
    let finally_rejects = "function f() {\n  return Promise.resolve(1).finally(() => Promise.reject(2)).catch(e => e + 1);\n}\n";
    let direct_reject = "function f() {\n  return Promise.reject(2).catch(e => e + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_eq!(
        value_fp(&i, finally_form, Lang::TypeScript),
        value_fp(&i, direct_form, Lang::TypeScript),
        "safe Promise.finally handlers should preserve fulfilled settlement values"
    );
    assert_eq!(
        value_fp(&i, rejected_finally, Lang::TypeScript),
        value_fp(&i, rejected_direct, Lang::TypeScript),
        "safe Promise.finally handlers should preserve rejected settlement values"
    );
    assert_eq!(
        value_fp(&i, finally_rejects, Lang::TypeScript),
        value_fp(&i, direct_reject, Lang::TypeScript),
        "Promise.finally handlers returning Promise.reject should override the channel"
    );
    assert_ne!(
        value_fp(&i, finally_form, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "Promise.finally recovery must preserve the Promise boundary"
    );
}

#[test]
fn promise_finally_recovery_stays_closed_for_unsafe_handlers() {
    let i = Interner::new();
    let unsafe_finally = "function f() {\n  return Promise.resolve(1).finally(() => maybeThenable).then(x => x + 1);\n}\n";
    let direct_form = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let parameterized_finally =
        "function f() {\n  return Promise.resolve(1).finally((x) => 9).then(x => x + 1);\n}\n";

    assert_ne!(
        value_fp(&i, unsafe_finally, Lang::TypeScript),
        value_fp(&i, direct_form, Lang::TypeScript),
        "Promise.finally handlers with possible thenables stay closed"
    );
    assert_ne!(
        value_fp(&i, parameterized_finally, Lang::TypeScript),
        value_fp(&i, direct_form, Lang::TypeScript),
        "Promise.finally handlers with parameters stay closed until undefined-argument semantics are modeled"
    );
}

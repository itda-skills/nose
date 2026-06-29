use super::*;

#[test]
fn promise_executor_keeps_promise_boundary_and_settlement_precedence() {
    let i = Interner::new();
    let executor_resolve = "function f() {\n  return new Promise(resolve => resolve(1));\n}\n";
    let sync_value = "function f() {\n  return 1;\n}\n";
    let multiple_settlement =
        "function f() {\n  return new Promise(resolve => { resolve(1); resolve(2); });\n}\n";
    let later_settlement = "function f() {\n  return Promise.resolve(2);\n}\n";
    let throw_after_settlement =
        "function f() {\n  return new Promise((resolve, reject) => { resolve(1); throw 2; });\n}\n";
    let thrown_rejection = "function f() {\n  return Promise.reject(2);\n}\n";

    assert_ne!(
        value_fp(&i, executor_resolve, Lang::TypeScript),
        value_fp(&i, sync_value, Lang::TypeScript),
        "new Promise settlement recovery must never erase the Promise boundary into a synchronous payload"
    );
    assert_ne!(
        value_fp(&i, multiple_settlement, Lang::TypeScript),
        value_fp(&i, later_settlement, Lang::TypeScript),
        "multiple settlement calls must not recover the later resolve value"
    );
    assert_ne!(
        value_fp(&i, throw_after_settlement, Lang::TypeScript),
        value_fp(&i, thrown_rejection, Lang::TypeScript),
        "throwing after a prior settlement must not overwrite the observed settlement channel"
    );
}

#[test]
fn promise_executor_scheduling_and_thenable_boundaries_stay_closed() {
    let i = Interner::new();
    let timer_executor =
        "function f() {\n  return new Promise(resolve => setTimeout(() => resolve(1), 0));\n}\n";
    let direct_resolve = "function f() {\n  return Promise.resolve(1);\n}\n";
    let possible_thenable_executor =
        "function f(x) {\n  return new Promise(resolve => resolve(x));\n}\n";
    let possible_thenable_factory = "function f(x) {\n  return Promise.resolve(x);\n}\n";

    assert_ne!(
        value_fp(&i, timer_executor, Lang::TypeScript),
        value_fp(&i, direct_resolve, Lang::TypeScript),
        "timer-backed executor settlement needs scheduler/liveness proof before it can converge with direct Promise.resolve"
    );
    assert_ne!(
        value_fp(&i, possible_thenable_executor, Lang::TypeScript),
        value_fp(&i, possible_thenable_factory, Lang::TypeScript),
        "constructor resolve over an untyped value must stay closed until executor and thenable-assimilation proof are both explicit"
    );
}

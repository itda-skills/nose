use super::*;

#[test]
fn promise_all_literal_fulfilled_aggregate_recovers_without_sync_erasure() {
    let i = Interner::new();
    let direct_all =
        "function f() {\n  return Promise.all([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let equivalent_all = "function f() {\n  return Promise.all([Promise.resolve(1).then(x => x), Promise.resolve(2)]);\n}\n";
    let sync_array = "function f() {\n  return [1, 2];\n}\n";

    assert_eq!(
        value_fp(&i, direct_all, Lang::TypeScript),
        value_fp(&i, equivalent_all, Lang::TypeScript),
        "fulfilled-only Promise.all over a literal aggregate should recover the ordered fulfilled payloads"
    );
    assert_ne!(
        value_fp(&i, direct_all, Lang::TypeScript),
        value_fp(&i, sync_array, Lang::TypeScript),
        "Promise.all recovery must preserve the Promise boundary"
    );
}

#[test]
fn promise_all_literal_aggregate_stays_closed_for_rejection_and_dynamic_iterables() {
    let i = Interner::new();
    let fulfilled_all =
        "function f() {\n  return Promise.all([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let rejected_all =
        "function f() {\n  return Promise.all([Promise.resolve(1), Promise.reject(2)]);\n}\n";
    let dynamic_iterable = "function f(xs) {\n  return Promise.all(xs);\n}\n";
    let all_settled = "function f() {\n  return Promise.allSettled([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let race =
        "function f() {\n  return Promise.race([Promise.resolve(1), Promise.resolve(2)]);\n}\n";

    assert_ne!(
        value_fp(&i, fulfilled_all, Lang::TypeScript),
        value_fp(&i, rejected_all, Lang::TypeScript),
        "Promise.all with a rejected input must stay closed until rejection ordering is modeled"
    );
    assert_ne!(
        value_fp(&i, fulfilled_all, Lang::TypeScript),
        value_fp(&i, dynamic_iterable, Lang::TypeScript),
        "Promise.all over a dynamic iterable must stay closed until iterable lifecycle is modeled"
    );
    assert_ne!(
        value_fp(&i, fulfilled_all, Lang::TypeScript),
        value_fp(&i, all_settled, Lang::TypeScript),
        "first/all fulfilled and all-settled aggregate semantics must remain separate"
    );
    assert_ne!(
        value_fp(&i, fulfilled_all, Lang::TypeScript),
        value_fp(&i, race, Lang::TypeScript),
        "all-fulfilled and first-settled aggregate semantics must remain separate"
    );
}

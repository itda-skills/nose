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
fn promise_all_literal_raw_non_thenable_inputs_assimilate_without_sync_erasure() {
    let i = Interner::new();
    let raw_all = "function f() {\n  return Promise.all([1, 2]);\n}\n";
    let wrapped_all =
        "function f() {\n  return Promise.all([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let sync_array = "function f() {\n  return [1, 2];\n}\n";
    let possible_thenable = "function f(x) {\n  return Promise.all([x, 2]);\n}\n";
    let wrapped_possible = "function f(x) {\n  return Promise.all([Promise.resolve(x), 2]);\n}\n";

    assert_eq!(
        value_fp(&i, raw_all, Lang::TypeScript),
        value_fp(&i, wrapped_all, Lang::TypeScript),
        "Promise.all should assimilate literal non-thenable scalar inputs as fulfilled elements"
    );
    assert_ne!(
        value_fp(&i, raw_all, Lang::TypeScript),
        value_fp(&i, sync_array, Lang::TypeScript),
        "raw input assimilation must preserve the Promise boundary"
    );
    assert_ne!(
        value_fp(&i, possible_thenable, Lang::TypeScript),
        value_fp(&i, wrapped_possible, Lang::TypeScript),
        "untyped aggregate inputs stay closed because they may be thenables"
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

#[test]
fn promise_all_settled_literal_aggregate_recovers_ordered_settlement_records() {
    let i = Interner::new();
    let direct = "function f() {\n  return Promise.allSettled([Promise.resolve(1), Promise.reject(2)]);\n}\n";
    let equivalent = "function f() {\n  return Promise.allSettled([Promise.resolve(1).then(x => x), Promise.reject(2).then(x => x)]);\n}\n";
    let sync_records = "function f() {\n  return [{ status: \"fulfilled\", value: 1 }, { status: \"rejected\", reason: 2 }];\n}\n";

    assert_eq!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, equivalent, Lang::TypeScript),
        "Promise.allSettled over a literal aggregate should recover ordered fulfilled/rejected records"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, sync_records, Lang::TypeScript),
        "Promise.allSettled recovery must preserve the Promise boundary"
    );
}

#[test]
fn promise_all_settled_literal_raw_non_thenable_inputs_become_fulfilled_records() {
    let i = Interner::new();
    let raw = "function f() {\n  return Promise.allSettled([1, Promise.reject(2)]);\n}\n";
    let wrapped =
        "function f() {\n  return Promise.allSettled([Promise.resolve(1), Promise.reject(2)]);\n}\n";
    let sync_records = "function f() {\n  return [{ status: \"fulfilled\", value: 1 }, { status: \"rejected\", reason: 2 }];\n}\n";
    let possible_thenable =
        "function f(x) {\n  return Promise.allSettled([x, Promise.reject(2)]);\n}\n";
    let wrapped_possible =
        "function f(x) {\n  return Promise.allSettled([Promise.resolve(x), Promise.reject(2)]);\n}\n";

    assert_eq!(
        value_fp(&i, raw, Lang::TypeScript),
        value_fp(&i, wrapped, Lang::TypeScript),
        "Promise.allSettled should assimilate literal non-thenable scalar inputs as fulfilled records"
    );
    assert_ne!(
        value_fp(&i, raw, Lang::TypeScript),
        value_fp(&i, sync_records, Lang::TypeScript),
        "raw input assimilation must preserve the Promise boundary and settled-record channel"
    );
    assert_ne!(
        value_fp(&i, possible_thenable, Lang::TypeScript),
        value_fp(&i, wrapped_possible, Lang::TypeScript),
        "untyped allSettled inputs stay closed because they may be thenables"
    );
}

#[test]
fn promise_all_settled_literal_aggregate_preserves_channels_and_closed_inputs() {
    let i = Interner::new();
    let mixed = "function f() {\n  return Promise.allSettled([Promise.resolve(1), Promise.reject(2)]);\n}\n";
    let swapped = "function f() {\n  return Promise.allSettled([Promise.reject(2), Promise.resolve(1)]);\n}\n";
    let fulfilled = "function f() {\n  return Promise.allSettled([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let dynamic_iterable = "function f(xs) {\n  return Promise.allSettled(xs);\n}\n";
    let raw_values = "function f() {\n  return Promise.allSettled([1, 2]);\n}\n";
    let all = "function f() {\n  return Promise.all([Promise.resolve(1), Promise.reject(2)]);\n}\n";

    assert_ne!(
        value_fp(&i, mixed, Lang::TypeScript),
        value_fp(&i, swapped, Lang::TypeScript),
        "Promise.allSettled recovery must preserve aggregate element order"
    );
    assert_ne!(
        value_fp(&i, mixed, Lang::TypeScript),
        value_fp(&i, fulfilled, Lang::TypeScript),
        "Promise.allSettled fulfilled/rejected record channels must remain distinct"
    );
    assert_ne!(
        value_fp(&i, mixed, Lang::TypeScript),
        value_fp(&i, dynamic_iterable, Lang::TypeScript),
        "Promise.allSettled over a dynamic iterable must stay closed until iterable lifecycle is modeled"
    );
    assert_ne!(
        value_fp(&i, mixed, Lang::TypeScript),
        value_fp(&i, raw_values, Lang::TypeScript),
        "fulfilled raw inputs must not erase rejected allSettled channels"
    );
    assert_ne!(
        value_fp(&i, mixed, Lang::TypeScript),
        value_fp(&i, all, Lang::TypeScript),
        "all-settled and all-fulfilled aggregate semantics must remain separate"
    );
}

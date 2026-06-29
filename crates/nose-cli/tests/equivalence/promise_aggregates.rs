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
fn promise_race_literal_aggregate_recovers_first_settled_state() {
    let i = Interner::new();
    let direct =
        "function f() {\n  return Promise.race([Promise.resolve(1), Promise.resolve(2)]);\n}\n";
    let equivalent = "function f() {\n  return Promise.race([Promise.resolve(1).then(x => x), Promise.resolve(2)]);\n}\n";
    let rejected_first =
        "function f() {\n  return Promise.race([Promise.reject(1), Promise.resolve(2)]);\n}\n";
    let rejected_equivalent = "function f() {\n  return Promise.race([Promise.reject(1).then(x => x), Promise.resolve(2)]);\n}\n";
    let swapped =
        "function f() {\n  return Promise.race([Promise.resolve(2), Promise.resolve(1)]);\n}\n";
    let sync_value = "function f() {\n  return 1;\n}\n";
    let dynamic_iterable = "function f(xs) {\n  return Promise.race(xs);\n}\n";
    let empty_race = "function f() {\n  return Promise.race([]);\n}\n";
    let possible_thenable = "function f(x) {\n  return Promise.race([1, x]);\n}\n";
    let wrapped_possible =
        "function f(x) {\n  return Promise.race([Promise.resolve(1), Promise.resolve(x)]);\n}\n";

    assert_eq!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, equivalent, Lang::TypeScript),
        "Promise.race should recover first-settled literal aggregates when every element is closed"
    );
    assert_eq!(
        value_fp(&i, rejected_first, Lang::TypeScript),
        value_fp(&i, rejected_equivalent, Lang::TypeScript),
        "Promise.race should preserve a first rejected settlement channel"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, swapped, Lang::TypeScript),
        "Promise.race recovery must preserve first-settled element order"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, sync_value, Lang::TypeScript),
        "Promise.race recovery must preserve the Promise boundary"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, dynamic_iterable, Lang::TypeScript),
        "Promise.race over a dynamic iterable must stay closed until iterable lifecycle is modeled"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, empty_race, Lang::TypeScript),
        "Promise.race([]) stays closed because it never settles"
    );
    assert_ne!(
        value_fp(&i, possible_thenable, Lang::TypeScript),
        value_fp(&i, wrapped_possible, Lang::TypeScript),
        "Promise.race with possible thenables stays closed despite an earlier literal input"
    );
}

#[test]
fn promise_any_literal_aggregate_recovers_first_fulfilled_state() {
    let i = Interner::new();
    let direct = "function f() {\n  return Promise.any([Promise.reject(1), Promise.resolve(2), Promise.resolve(3)]);\n}\n";
    let equivalent = "function f() {\n  return Promise.any([Promise.reject(1).then(x => x), Promise.resolve(2).then(x => x), Promise.resolve(3)]);\n}\n";
    let raw = "function f() {\n  return Promise.any([Promise.reject(1), 2]);\n}\n";
    let wrapped =
        "function f() {\n  return Promise.any([Promise.reject(1), Promise.resolve(2)]);\n}\n";
    let swapped = "function f() {\n  return Promise.any([Promise.reject(1), Promise.resolve(3), Promise.resolve(2)]);\n}\n";
    let sync_value = "function f() {\n  return 2;\n}\n";
    let all_rejected =
        "function f() {\n  return Promise.any([Promise.reject(1), Promise.reject(2)]);\n}\n";
    let aggregate_error_substitute = "function f() {\n  return Promise.reject([1, 2]);\n}\n";
    let dynamic_iterable = "function f(xs) {\n  return Promise.any(xs);\n}\n";
    let possible_thenable = "function f(x) {\n  return Promise.any([x, 2]);\n}\n";
    let wrapped_possible = "function f(x) {\n  return Promise.any([Promise.resolve(x), 2]);\n}\n";

    assert_eq!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, equivalent, Lang::TypeScript),
        "Promise.any should recover first-fulfilled literal aggregates when every element is closed"
    );
    assert_eq!(
        value_fp(&i, raw, Lang::TypeScript),
        value_fp(&i, wrapped, Lang::TypeScript),
        "Promise.any should assimilate non-thenable raw inputs as fulfilled candidates"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, swapped, Lang::TypeScript),
        "Promise.any recovery must preserve first-fulfilled element order"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, sync_value, Lang::TypeScript),
        "Promise.any recovery must preserve the Promise boundary"
    );
    assert_ne!(
        value_fp(&i, all_rejected, Lang::TypeScript),
        value_fp(&i, aggregate_error_substitute, Lang::TypeScript),
        "all-rejected Promise.any stays closed until AggregateError payloads are modeled"
    );
    assert_ne!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, dynamic_iterable, Lang::TypeScript),
        "Promise.any over a dynamic iterable must stay closed until iterable lifecycle is modeled"
    );
    assert_ne!(
        value_fp(&i, possible_thenable, Lang::TypeScript),
        value_fp(&i, wrapped_possible, Lang::TypeScript),
        "Promise.any with possible thenables stays closed despite a later literal input"
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

use super::*;

#[test]
fn abort_signal_timer_options_do_not_recover_fulfilled_payload() {
    let i = Interner::new();
    let timer_with_signal = "import { setTimeout } from 'node:timers/promises';\nfunction f(signal: AbortSignal) {\n  return setTimeout(10, 1, { signal });\n}\n";
    let resolved_payload = "function f(signal: AbortSignal) {\n  return Promise.resolve(1);\n}\n";
    let timer_without_signal = "import { setTimeout } from 'node:timers/promises';\nfunction f(signal: AbortSignal) {\n  return setTimeout(10, 1);\n}\n";

    assert_ne!(
        value_fp(&i, timer_with_signal, Lang::TypeScript),
        value_fp(&i, resolved_payload, Lang::TypeScript),
        "timer options with AbortSignal can reject and must not recover as a fulfilled Promise payload"
    );
    assert_ne!(
        value_fp(&i, timer_with_signal, Lang::TypeScript),
        value_fp(&i, timer_without_signal, Lang::TypeScript),
        "option-bearing timer calls remain distinct from safe no-options payload recovery"
    );
}

#[test]
fn abort_signal_lifecycle_surfaces_stay_closed() {
    let i = Interner::new();
    let timeout_signal = "function f() {\n  return AbortSignal.timeout(10);\n}\n";
    let fresh_signal = "function f() {\n  return new AbortController().signal;\n}\n";
    let composed_signal =
        "function f(a: AbortSignal, b: AbortSignal) {\n  return AbortSignal.any([a, b]);\n}\n";
    let first_signal = "function f(a: AbortSignal, b: AbortSignal) {\n  return a;\n}\n";
    let scheduler_with_signal =
        "function f(scheduler, signal) {\n  return scheduler.wait(10, { signal });\n}\n";
    let scheduler_without_signal =
        "function f(scheduler, signal) {\n  return scheduler.wait(10);\n}\n";

    assert_ne!(
        value_fp(&i, timeout_signal, Lang::TypeScript),
        value_fp(&i, fresh_signal, Lang::TypeScript),
        "timeout-created signals have scheduling/liveness behavior and must not merge with fresh controller signals"
    );
    assert_ne!(
        value_fp(&i, composed_signal, Lang::TypeScript),
        value_fp(&i, first_signal, Lang::TypeScript),
        "AbortSignal.any has composition and liveness semantics, not first-signal identity"
    );
    assert_ne!(
        value_fp(&i, scheduler_with_signal, Lang::TypeScript),
        value_fp(&i, scheduler_without_signal, Lang::TypeScript),
        "scheduler waits with cancellation signals must stay distinct from uncancelable waits"
    );
}

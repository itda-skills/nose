use super::*;

#[test]
fn interval_liveness_and_cancellation_surfaces_stay_closed() {
    let i = Interner::new();
    let interval_handle = "function f(cb) {\n  return setInterval(cb, 10);\n}\n";
    let timeout_handle = "function f(cb) {\n  return setTimeout(cb, 10);\n}\n";
    let clear_interval = "function f(handle) {\n  return clearInterval(handle);\n}\n";
    let clear_timeout = "function f(handle) {\n  return clearTimeout(handle);\n}\n";

    assert_ne!(
        value_fp(&i, interval_handle, Lang::TypeScript),
        value_fp(&i, timeout_handle, Lang::TypeScript),
        "interval streams have repeated-emission liveness and must not merge with one-shot timers"
    );
    assert_ne!(
        value_fp(&i, clear_interval, Lang::TypeScript),
        value_fp(&i, clear_timeout, Lang::TypeScript),
        "interval cancellation is not the same lifecycle contract as timeout cancellation"
    );
}

#[test]
fn scheduler_and_microtask_ordering_surfaces_stay_closed() {
    let i = Interner::new();
    let scheduler_yield = "function f(scheduler, cb) {\n  return scheduler.yield().then(cb);\n}\n";
    let promise_resolve = "function f(scheduler, cb) {\n  return Promise.resolve().then(cb);\n}\n";
    let queued_callback = "function f(cb) {\n  return queueMicrotask(cb);\n}\n";
    let direct_callback = "function f(cb) {\n  return cb();\n}\n";
    let animation_frame = "function f(cb) {\n  return requestAnimationFrame(cb);\n}\n";
    let timeout_frame = "function f(cb) {\n  return setTimeout(cb, 16);\n}\n";

    assert_ne!(
        value_fp(&i, scheduler_yield, Lang::TypeScript),
        value_fp(&i, promise_resolve, Lang::TypeScript),
        "scheduler.yield has explicit microtask/order semantics and must not collapse into Promise.resolve"
    );
    assert_ne!(
        value_fp(&i, queued_callback, Lang::TypeScript),
        value_fp(&i, direct_callback, Lang::TypeScript),
        "queueMicrotask defers callback observation and must not merge with direct callback invocation"
    );
    assert_ne!(
        value_fp(&i, animation_frame, Lang::TypeScript),
        value_fp(&i, timeout_frame, Lang::TypeScript),
        "animation-frame scheduling is a separate lifecycle boundary from timeout scheduling"
    );
}

#[test]
fn scheduler_wait_stays_distinct_from_timer_and_sync_payloads() {
    let i = Interner::new();
    let scheduler_wait = "function f(scheduler) {\n  return scheduler.wait(10);\n}\n";
    let global_timeout = "function f(scheduler) {\n  return setTimeout(() => {}, 10);\n}\n";
    let sync_payload = "function f(scheduler) {\n  return 10;\n}\n";

    assert_ne!(
        value_fp(&i, scheduler_wait, Lang::TypeScript),
        value_fp(&i, global_timeout, Lang::TypeScript),
        "scheduler.wait needs timing and cancellation/liveness proof before timer convergence"
    );
    assert_ne!(
        value_fp(&i, scheduler_wait, Lang::TypeScript),
        value_fp(&i, sync_payload, Lang::TypeScript),
        "scheduler.wait returns an async protocol boundary, not a synchronous payload"
    );
}

use super::{
    assert_missing_evidence_not_contains, missing_evidence_for_lang_call,
    python_runtime_with_path_visible_local_asyncio, runtime_boundary_evidence_for_lang_call,
    rust_runtime_with_same_file_local_tokio,
};
use nose_il::Lang;

#[test]
fn non_js_async_runtime_imported_bindings_report_shared_obligations() {
    let py_imported_task = missing_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import create_task as spawn_task\nasync def main():\n    return spawn_task(work())\n",
        Lang::Python,
        "spawn_task",
    );
    let py_imported_sleep = missing_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import sleep as delay\nasync def main():\n    return await delay(1)\n",
        Lang::Python,
        "delay",
    );
    let py_imported_gather = missing_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import gather as all_done\nasync def main(task):\n    return await all_done(task)\n",
        Lang::Python,
        "all_done",
    );
    let py_imported_wait = missing_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import wait\nasync def main(task):\n    return await wait([task])\n",
        Lang::Python,
        "wait",
    );
    let rust_brace_spawn = missing_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::{spawn};\nfn run() { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_brace_join_alias = missing_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::{join as tokio_join};\nasync fn run() { tokio_join!(work(), other()); }\n",
        Lang::Rust,
        "tokio_join",
    );
    let rust_brace_select_alias = missing_evidence_for_lang_call(
        "runtime.rs",
        "use futures::{select as fut_select};\nasync fn run() { fut_select!(a = work() => a); }\n",
        Lang::Rust,
        "fut_select",
    );

    assert!(py_imported_task.contains(&"task-spawn-scheduling-contract"));
    assert!(py_imported_task.contains(&"task-handle-lifecycle-contract"));
    assert!(py_imported_task.contains(&"task-cancellation-liveness-contract"));
    assert!(py_imported_sleep.contains(&"timer-scheduling-contract"));
    assert!(py_imported_gather.contains(&"async-aggregate-all-completion-contract"));
    assert!(py_imported_gather.contains(&"async-aggregate-result-channel-contract"));
    assert!(py_imported_wait.contains(&"async-aggregate-completion-contract"));
    assert!(py_imported_wait.contains(&"async-aggregate-cancellation-liveness-contract"));
    assert!(py_imported_wait.contains(&"async-aggregate-result-channel-contract"));
    assert!(rust_brace_spawn.contains(&"task-spawn-scheduling-contract"));
    assert!(rust_brace_spawn.contains(&"task-handle-lifecycle-contract"));
    assert!(rust_brace_spawn.contains(&"task-cancellation-liveness-contract"));
    assert!(rust_brace_join_alias.contains(&"async-aggregate-all-completion-contract"));
    assert!(rust_brace_join_alias.contains(&"async-aggregate-result-channel-contract"));
    assert!(rust_brace_select_alias.contains(&"async-aggregate-first-completion-contract"));
    assert!(rust_brace_select_alias.contains(&"async-aggregate-cancellation-liveness-contract"));
    assert!(rust_brace_select_alias.contains(&"async-aggregate-result-channel-contract"));
}

#[test]
fn non_js_async_runtime_imported_bindings_reject_local_shadows() {
    let py_imported_gather_param_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import gather as all_done\nasync def main(all_done, task):\n    return await all_done(task)\n",
        Lang::Python,
        "all_done",
    );
    let py_imported_sleep_assign_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import sleep as delay\nasync def main():\n    delay = local_sleep\n    return await delay(1)\n",
        Lang::Python,
        "delay",
    );
    let py_nested_imported_binding = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "async def helper():\n    from asyncio import sleep as delay\nasync def main():\n    return await delay(1)\n",
        Lang::Python,
        "delay",
    );

    assert_missing_evidence_not_contains(
        py_imported_gather_param_shadow,
        "async-aggregate-all-completion-contract",
        "Python imported asyncio binding shadowed by parameter",
    );
    assert_missing_evidence_not_contains(
        py_imported_sleep_assign_shadow,
        "timer-scheduling-contract",
        "Python imported asyncio binding shadowed by assignment",
    );
    assert_missing_evidence_not_contains(
        py_nested_imported_binding,
        "timer-scheduling-contract",
        "Python imported asyncio binding imported only in another local scope",
    );
}

#[test]
fn non_js_async_runtime_imported_bindings_reject_rust_shadows_and_scopes() {
    let rust_brace_spawn_param_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::{spawn};\nfn run<F>(spawn: F) { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_brace_spawn_let_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::{spawn};\nfn run() { let spawn = local; spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_brace_join_macro_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::{join};\nmacro_rules! join { ($a:expr, $b:expr) => { ($a, $b) }; }\nasync fn run() { join!(work(), other()); }\n",
        Lang::Rust,
        "join",
    );
    let rust_brace_spawn_nested_use = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "fn helper() { use tokio::{spawn}; }\nfn run() { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_direct_spawn_nested_use = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "fn helper() { use tokio::spawn; }\nfn run() { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_brace_spawn_other_module = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "fn run() { spawn(work()); }\nmod helper { use tokio::{spawn}; fn helper() { spawn(work()); } }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_brace_spawn_const_block_use = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "const INIT: () = { use tokio::{spawn}; };\nfn run() { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );

    for (labels, surface) in [
        (
            rust_brace_spawn_param_shadow,
            "Rust brace spawn parameter shadow",
        ),
        (rust_brace_spawn_let_shadow, "Rust brace spawn let shadow"),
    ] {
        assert_missing_evidence_not_contains(labels, "task-spawn-scheduling-contract", surface);
    }
    assert_missing_evidence_not_contains(
        rust_brace_join_macro_shadow,
        "async-aggregate-all-completion-contract",
        "Rust brace join shadowed by local macro",
    );
    for (labels, surface) in [
        (
            rust_brace_spawn_nested_use,
            "Rust brace spawn imported only in another function",
        ),
        (
            rust_direct_spawn_nested_use,
            "Rust direct spawn imported only in another function",
        ),
        (
            rust_brace_spawn_other_module,
            "Rust brace spawn imported only in another module",
        ),
        (
            rust_brace_spawn_const_block_use,
            "Rust brace spawn imported only in a const initializer block",
        ),
    ] {
        assert_missing_evidence_not_contains(labels, "task-spawn-scheduling-contract", surface);
    }
}

#[test]
fn non_js_async_runtime_context_rejects_project_local_imported_bindings() {
    let py_local_asyncio_imported_task = python_runtime_with_path_visible_local_asyncio(
        "from asyncio import create_task as spawn_task\nasync def main():\n    return spawn_task(work())\n",
        "spawn_task",
    );
    let rust_local_tokio_brace_spawn = rust_runtime_with_same_file_local_tokio(
        "use tokio::{spawn};\nfn run() { spawn(work()); }\n",
        "spawn",
    );

    assert_missing_evidence_not_contains(
        py_local_asyncio_imported_task,
        "task-spawn-scheduling-contract",
        "project-local Python asyncio module through imported binding",
    );
    assert_missing_evidence_not_contains(
        rust_local_tokio_brace_spawn,
        "task-spawn-scheduling-contract",
        "project-local Rust tokio root for brace imported spawn",
    );
}

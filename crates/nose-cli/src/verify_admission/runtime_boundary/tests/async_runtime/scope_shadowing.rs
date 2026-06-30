use super::{
    assert_missing_evidence_contains, assert_missing_evidence_not_contains,
    runtime_boundary_evidence_for_lang_call,
};
use nose_il::Lang;

#[test]
fn non_js_async_runtime_ignores_unrelated_local_shadows() {
    let py_alias_shadowed_elsewhere = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "import asyncio as aio\n\
         def helper(aio):\n    return aio\n\
         async def main():\n    return await aio.sleep(1)\n",
        Lang::Python,
        "aio.sleep",
    );
    let py_binding_shadowed_elsewhere = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import sleep\n\
         def helper(sleep):\n    return sleep\n\
         async def main():\n    return await sleep(1)\n",
        Lang::Python,
        "sleep",
    );
    let py_alias_assignment_elsewhere = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "import asyncio as aio\n\
         def helper():\n    aio = local\n    return aio\n\
         async def main():\n    return await aio.sleep(1)\n",
        Lang::Python,
        "aio.sleep",
    );
    let rust_spawn_shadowed_elsewhere = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::spawn;\nfn helper<F>(spawn: F) { let _ = spawn; }\nfn run() { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_join_shadowed_elsewhere = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::join;\nfn helper() { let join = local; let _unused = join; }\nasync fn run() { join!(work(), other()); }\n",
        Lang::Rust,
        "join",
    );

    assert_missing_evidence_contains(
        py_alias_shadowed_elsewhere,
        "timer-scheduling-contract",
        "Python asyncio alias shadowed only in another function",
    );
    assert_missing_evidence_contains(
        py_binding_shadowed_elsewhere,
        "timer-scheduling-contract",
        "Python asyncio imported binding shadowed only in another function",
    );
    assert_missing_evidence_contains(
        py_alias_assignment_elsewhere,
        "timer-scheduling-contract",
        "Python asyncio alias assigned only in another function",
    );
    assert_missing_evidence_contains(
        rust_spawn_shadowed_elsewhere,
        "task-spawn-scheduling-contract",
        "Rust imported spawn shadowed only in another function",
    );
    assert_missing_evidence_contains(
        rust_join_shadowed_elsewhere,
        "async-aggregate-all-completion-contract",
        "Rust imported join shadowed only in another function",
    );
}

#[test]
fn non_js_async_runtime_keeps_real_local_shadows_closed() {
    let py_module_reassignment = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "import asyncio as aio\n\
         aio = local_runtime\n\
         async def main():\n    return await aio.sleep(1)\n",
        Lang::Python,
        "aio.sleep",
    );
    let py_same_scope_binding_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import sleep\n\
         async def main(sleep):\n    return await sleep(1)\n",
        Lang::Python,
        "sleep",
    );
    let rust_same_scope_spawn_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::spawn;\nfn run<F>(spawn: F) { spawn(work()); }\n",
        Lang::Rust,
        "spawn",
    );
    let rust_enclosing_closure_spawn_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.rs",
        "use tokio::spawn;\nfn outer<F>(spawn: F) { let inner = || spawn(work()); inner(); }\n",
        Lang::Rust,
        "spawn",
    );
    let py_enclosing_param_binding_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "from asyncio import sleep\n\
         def outer(sleep):\n    async def inner():\n        return await sleep(1)\n    return inner\n",
        Lang::Python,
        "sleep",
    );
    let py_enclosing_param_alias_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.py",
        "import asyncio as aio\n\
         def outer(aio):\n    async def inner():\n        return await aio.sleep(1)\n    return inner\n",
        Lang::Python,
        "aio.sleep",
    );

    assert_missing_evidence_not_contains(
        py_module_reassignment,
        "timer-scheduling-contract",
        "Python module-level asyncio alias reassignment",
    );
    assert_missing_evidence_not_contains(
        py_same_scope_binding_shadow,
        "timer-scheduling-contract",
        "Python same-scope asyncio imported binding shadow",
    );
    assert_missing_evidence_not_contains(
        rust_same_scope_spawn_shadow,
        "task-spawn-scheduling-contract",
        "Rust same-scope imported spawn parameter shadow",
    );
    assert_missing_evidence_not_contains(
        rust_enclosing_closure_spawn_shadow,
        "task-spawn-scheduling-contract",
        "Rust enclosing-scope imported spawn closure shadow",
    );
    assert_missing_evidence_not_contains(
        py_enclosing_param_binding_shadow,
        "timer-scheduling-contract",
        "Python enclosing-scope asyncio imported binding shadow",
    );
    assert_missing_evidence_not_contains(
        py_enclosing_param_alias_shadow,
        "timer-scheduling-contract",
        "Python enclosing-scope asyncio alias shadow",
    );
}

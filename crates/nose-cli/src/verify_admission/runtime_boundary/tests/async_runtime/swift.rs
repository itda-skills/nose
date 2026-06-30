use super::{
    assert_missing_evidence_not_contains, missing_evidence_for_lang_call,
    runtime_boundary_evidence_for_corpus_call, runtime_boundary_evidence_for_lang_call,
};
use nose_il::Lang;

#[test]
fn swift_structured_concurrency_reports_shared_obligations() {
    let swift_sleep = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async {\n  await Task.sleep(nanoseconds: 1)\n}\n",
        Lang::Swift,
        "Task.sleep",
    );
    let swift_yield = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async {\n  await Task.yield()\n}\n",
        Lang::Swift,
        "Task.yield",
    );
    let swift_task_group = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async {\n  await withTaskGroup(of: Int.self) { group in\n    group.addTask { 1 }\n  }\n}\n",
        Lang::Swift,
        "withTaskGroup",
    );
    let swift_throwing_task_group = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async throws {\n  try await withThrowingTaskGroup(of: Int.self) { group in\n    group.addTask { 1 }\n  }\n}\n",
        Lang::Swift,
        "withThrowingTaskGroup",
    );
    let swift_discarding_task_group = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async {\n  await withDiscardingTaskGroup { group in\n    group.addTask { work() }\n  }\n}\n",
        Lang::Swift,
        "withDiscardingTaskGroup",
    );
    let swift_throwing_discarding_task_group = missing_evidence_for_lang_call(
        "runtime.swift",
        "func run() async throws {\n  try await withThrowingDiscardingTaskGroup { group in\n    group.addTask { try work() }\n  }\n}\n",
        Lang::Swift,
        "withThrowingDiscardingTaskGroup",
    );

    assert!(swift_sleep.contains(&"timer-scheduling-contract"));
    assert!(swift_sleep.contains(&"task-cancellation-liveness-contract"));
    assert!(swift_yield.contains(&"task-yield-scheduling-contract"));
    for labels in [&swift_task_group, &swift_throwing_task_group] {
        assert!(labels.contains(&"async-aggregate-all-completion-contract"));
        assert!(labels.contains(&"async-aggregate-cancellation-liveness-contract"));
        assert!(labels.contains(&"async-aggregate-result-channel-contract"));
    }
    assert!(swift_throwing_task_group.contains(&"exception-channel-contract"));
    assert!(swift_discarding_task_group.contains(&"async-aggregate-all-completion-contract"));
    assert!(swift_discarding_task_group.contains(&"async-aggregate-cancellation-liveness-contract"));
    assert!(!swift_discarding_task_group.contains(&"async-aggregate-result-channel-contract"));
    assert!(
        swift_throwing_discarding_task_group.contains(&"async-aggregate-all-completion-contract")
    );
    assert!(swift_throwing_discarding_task_group
        .contains(&"async-aggregate-cancellation-liveness-contract"));
    assert!(swift_throwing_discarding_task_group.contains(&"exception-channel-contract"));
    assert!(
        !swift_throwing_discarding_task_group.contains(&"async-aggregate-result-channel-contract")
    );
}

#[test]
fn swift_structured_concurrency_rejects_local_runtime_shadows() {
    let swift_shadowed_sleep = runtime_boundary_evidence_for_lang_call(
        "runtime.swift",
        "let Task = makeTask\nfunc run() async {\n  await Task.sleep(nanoseconds: 1)\n}\n",
        Lang::Swift,
        "Task.sleep",
    );
    let swift_shadowed_group = runtime_boundary_evidence_for_lang_call(
        "runtime.swift",
        "func withTaskGroup(_ body: () -> Void) { body() }\nfunc run() async {\n  await withTaskGroup { work() }\n}\n",
        Lang::Swift,
        "withTaskGroup",
    );
    let swift_project_task_group = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "TaskGroup.swift",
                "func withTaskGroup(_ body: () -> Void) { body() }\n",
                Lang::Swift,
            ),
            (
                "run.swift",
                "func run() async {\n  await withTaskGroup { work() }\n}\n",
                Lang::Swift,
            ),
        ],
        "run.swift",
        "withTaskGroup",
    );
    let swift_project_task_extension_sleep = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "TaskExtension.swift",
                "extension Task {\n  static func sleep(nanoseconds: Int) {}\n}\n",
                Lang::Swift,
            ),
            (
                "run.swift",
                "func run() async {\n  await Task.sleep(nanoseconds: 1)\n}\n",
                Lang::Swift,
            ),
        ],
        "run.swift",
        "Task.sleep",
    );
    let swift_task_extension_sleep = runtime_boundary_evidence_for_lang_call(
        "runtime.swift",
        "extension Task {\n  static func sleep(nanoseconds: Int) {}\n}\nfunc run() async {\n  await Task.sleep(nanoseconds: 1)\n}\n",
        Lang::Swift,
        "Task.sleep",
    );

    assert_missing_evidence_not_contains(
        swift_shadowed_sleep,
        "timer-scheduling-contract",
        "shadowed Swift Task.sleep",
    );
    assert_missing_evidence_not_contains(
        swift_shadowed_group,
        "async-aggregate-all-completion-contract",
        "shadowed Swift withTaskGroup",
    );
    assert_missing_evidence_not_contains(
        swift_project_task_group,
        "async-aggregate-all-completion-contract",
        "project-visible Swift withTaskGroup function",
    );
    assert_missing_evidence_not_contains(
        swift_project_task_extension_sleep,
        "timer-scheduling-contract",
        "project-visible Swift Task.sleep extension member",
    );
    assert_missing_evidence_not_contains(
        swift_task_extension_sleep,
        "timer-scheduling-contract",
        "Swift Task.sleep extension member",
    );
}

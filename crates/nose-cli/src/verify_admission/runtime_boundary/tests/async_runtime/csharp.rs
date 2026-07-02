use super::super::missing_evidence_for_protocol;
use super::{
    assert_missing_evidence_not_contains, missing_evidence_for_lang_call,
    runtime_boundary_evidence_for_corpus_call, runtime_boundary_evidence_for_lang_call,
};
use nose_il::{Lang, SourceProtocolKind};

#[test]
fn csharp_task_runtime_reports_shared_obligations() {
    let spawn = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  async Task M() { await Task.Run(() => Work()); }\n}\n",
        Lang::CSharp,
        "Task.Run",
    );
    let factory_spawn = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  void M() { Task.Factory.StartNew(() => Work()); }\n}\n",
        Lang::CSharp,
        "Task.Factory.StartNew",
    );
    let delay = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  async Task M() { await Task.Delay(100); }\n}\n",
        Lang::CSharp,
        "Task.Delay",
    );
    let task_yield = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  async Task M() { await Task.Yield(); }\n}\n",
        Lang::CSharp,
        "Task.Yield",
    );
    let when_all = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  async Task M(Task a, Task b) { await Task.WhenAll(a, b); }\n}\n",
        Lang::CSharp,
        "Task.WhenAll",
    );
    let when_any = missing_evidence_for_lang_call(
        "runtime.cs",
        "class C {\n  async Task M(Task a, Task b) { await Task.WhenAny(a, b); }\n}\n",
        Lang::CSharp,
        "Task.WhenAny",
    );

    for labels in [&spawn, &factory_spawn] {
        assert!(labels.contains(&"task-spawn-scheduling-contract"));
        assert!(labels.contains(&"task-handle-lifecycle-contract"));
        assert!(labels.contains(&"task-cancellation-liveness-contract"));
    }
    assert!(delay.contains(&"timer-scheduling-contract"));
    assert!(delay.contains(&"task-cancellation-liveness-contract"));
    assert!(task_yield.contains(&"task-yield-scheduling-contract"));
    assert!(when_all.contains(&"async-aggregate-all-completion-contract"));
    assert!(when_all.contains(&"async-aggregate-result-channel-contract"));
    assert!(when_any.contains(&"async-aggregate-first-completion-contract"));
    assert!(when_any.contains(&"async-aggregate-cancellation-liveness-contract"));
    assert!(when_any.contains(&"async-aggregate-result-channel-contract"));
}

#[test]
fn csharp_async_surfaces_report_shared_protocol_obligations() {
    let awaited = missing_evidence_for_protocol(
        "runtime.cs",
        "class C {\n  async Task M() { var x = await F(); }\n}\n",
        Lang::CSharp,
        SourceProtocolKind::Await,
    );
    assert!(awaited.contains(&"async-await-scheduling-contract"));

    let async_fn = missing_evidence_for_protocol(
        "runtime.cs",
        "class C {\n  async Task M() { Work(); }\n}\n",
        Lang::CSharp,
        SourceProtocolKind::AsyncFunction,
    );
    assert!(async_fn.contains(&"async-function-scheduling-contract"));

    let iteration = missing_evidence_for_protocol(
        "runtime.cs",
        "class C {\n  async Task M(IAsyncEnumerable<int> xs) { await foreach (var x in xs) { Use(x); } }\n}\n",
        Lang::CSharp,
        SourceProtocolKind::AsyncIteration,
    );
    assert!(iteration.contains(&"async-iteration-lifecycle-contract"));
    assert!(iteration.contains(&"async-iteration-value-channel-contract"));
    assert!(iteration.contains(&"async-await-scheduling-contract"));

    let context = missing_evidence_for_protocol(
        "runtime.cs",
        "class C {\n  async Task M() { await using (var r = Open()) { Use(r); } }\n}\n",
        Lang::CSharp,
        SourceProtocolKind::AsyncContext,
    );
    assert!(context.contains(&"async-context-lifecycle-contract"));
    assert!(context.contains(&"async-context-cleanup-contract"));
    assert!(context.contains(&"exception-channel-contract"));
    assert!(context.contains(&"async-await-scheduling-contract"));

    let generator = missing_evidence_for_protocol(
        "runtime.cs",
        "class C {\n  IEnumerable<int> M() { yield return 1; }\n}\n",
        Lang::CSharp,
        SourceProtocolKind::Yield,
    );
    assert!(generator.contains(&"generator-yield-lifecycle-contract"));
    assert!(generator.contains(&"generator-yield-protocol-contract"));
}

#[test]
fn csharp_task_runtime_rejects_local_and_project_shadows() {
    let file_local_shadow = runtime_boundary_evidence_for_lang_call(
        "runtime.cs",
        "class Task {\n  public static Task Delay(int ms) { return null; }\n}\nclass C {\n  async Task M() { await Task.Delay(100); }\n}\n",
        Lang::CSharp,
        "Task.Delay",
    );
    assert_missing_evidence_not_contains(
        file_local_shadow,
        "timer-scheduling-contract",
        "file-local C# Task shadow",
    );

    let project_shadow = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "Task.cs",
                "class Task {\n  public static Task Delay(int ms) { return null; }\n}\n",
                Lang::CSharp,
            ),
            (
                "run.cs",
                "class C {\n  async Task M() { await Task.Delay(100); }\n}\n",
                Lang::CSharp,
            ),
        ],
        "run.cs",
        "Task.Delay",
    );
    assert_missing_evidence_not_contains(
        project_shadow,
        "timer-scheduling-contract",
        "project-level C# Task shadow",
    );
}

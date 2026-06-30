use super::{
    assert_missing_evidence_contains, assert_missing_evidence_not_contains,
    missing_evidence_for_lang_call, runtime_boundary_evidence_for_corpus_call,
    runtime_boundary_evidence_for_lang_call,
};
use nose_il::Lang;

#[test]
fn java_completable_future_static_calls_report_shared_future_obligations() {
    let supply_async = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run() { return CompletableFuture.supplyAsync(() -> work()); } }\n",
        Lang::Java,
        "CompletableFuture.supplyAsync",
    );
    let run_async = missing_evidence_for_lang_call(
        "Runtime.java",
        "class Runtime { Object run() { return java.util.concurrent.CompletableFuture.runAsync(() -> work()); } }\n",
        Lang::Java,
        "java.util.concurrent.CompletableFuture.runAsync",
    );
    let wildcard_completed = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.*;\nclass Runtime { Object run() { return CompletableFuture.completedFuture(value()); } }\n",
        Lang::Java,
        "CompletableFuture.completedFuture",
    );
    let failed = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(Throwable error) { return CompletableFuture.failedFuture(error); } }\n",
        Lang::Java,
        "CompletableFuture.failedFuture",
    );
    let all_of = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(CompletableFuture<?> a, CompletableFuture<?> b) { return CompletableFuture.allOf(a, b); } }\n",
        Lang::Java,
        "CompletableFuture.allOf",
    );
    let any_of = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(CompletableFuture<?> a, CompletableFuture<?> b) { return CompletableFuture.anyOf(a, b); } }\n",
        Lang::Java,
        "CompletableFuture.anyOf",
    );

    for labels in [&supply_async, &run_async] {
        assert!(labels.contains(&"task-spawn-scheduling-contract"));
        assert!(labels.contains(&"task-handle-lifecycle-contract"));
        assert!(labels.contains(&"task-cancellation-liveness-contract"));
        assert!(labels.contains(&"future-settled-value-channel-contract"));
        assert!(labels.contains(&"future-callback-demand-effect-contract"));
    }
    assert!(wildcard_completed.contains(&"future-settled-value-channel-contract"));
    assert!(failed.contains(&"future-settled-value-channel-contract"));
    assert!(failed.contains(&"exception-channel-contract"));
    assert!(all_of.contains(&"async-aggregate-all-completion-contract"));
    assert!(all_of.contains(&"async-aggregate-result-channel-contract"));
    assert!(any_of.contains(&"async-aggregate-first-completion-contract"));
    assert!(any_of.contains(&"async-aggregate-result-channel-contract"));
}

#[test]
fn java_completion_stage_receiver_methods_require_future_like_receiver_proof() {
    let then_apply = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(CompletableFuture<String> future) { return future.thenApply(value -> value.trim()); } }\n",
        Lang::Java,
        "future.thenApply",
    );
    let exceptionally = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(CompletableFuture<String> future) { return future.exceptionally(error -> fallback()); } }\n",
        Lang::Java,
        "future.exceptionally",
    );
    let completion_stage = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletionStage;\nclass Runtime { Object run(CompletionStage<String> stage) { return stage.handle((value, error) -> value); } }\n",
        Lang::Java,
        "stage.handle",
    );
    let either = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(CompletableFuture<String> a, CompletableFuture<String> b) { return a.applyToEither(b, value -> value); } }\n",
        Lang::Java,
        "a.applyToEither",
    );
    let untyped = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "class Runtime { Object run(Object future) { return future.thenApply(value -> value); } }\n",
        Lang::Java,
        "future.thenApply",
    );

    assert!(then_apply.contains(&"future-fulfillment-continuation-contract"));
    assert!(then_apply.contains(&"future-settled-value-channel-contract"));
    assert!(then_apply.contains(&"future-callback-demand-effect-contract"));
    assert!(exceptionally.contains(&"future-exception-continuation-contract"));
    assert!(exceptionally.contains(&"exception-channel-contract"));
    assert!(completion_stage.contains(&"future-settlement-continuation-contract"));
    assert!(either.contains(&"async-aggregate-first-completion-contract"));
    assert_missing_evidence_not_contains(
        untyped,
        "future-fulfillment-continuation-contract",
        "untyped Java thenApply receiver",
    );
}

#[test]
fn java_completion_stage_receiver_methods_require_import_backed_type_domain() {
    let local_completable_future = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "class CompletableFuture<T> { Object thenApply(Object callback) { return this; } }\nclass Runtime { Object run(CompletableFuture<String> future) { return future.thenApply(value -> value); } }\n",
        Lang::Java,
        "future.thenApply",
    );
    let custom_completion_stage_import = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "import example.CompletionStage;\nclass Runtime { Object run(CompletionStage<String> stage) { return stage.handle((value, error) -> value); } }\n",
        Lang::Java,
        "stage.handle",
    );
    let wildcard_only = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.*;\nclass Runtime { Object run(CompletableFuture<String> future) { return future.thenApply(value -> value); } }\n",
        Lang::Java,
        "future.thenApply",
    );

    for (labels, surface) in [
        (
            local_completable_future,
            "local Java CompletableFuture receiver",
        ),
        (
            custom_completion_stage_import,
            "custom Java CompletionStage receiver import",
        ),
        (
            wildcard_only,
            "wildcard-only Java CompletableFuture receiver type",
        ),
    ] {
        for label in [
            "future-fulfillment-continuation-contract",
            "future-settlement-continuation-contract",
        ] {
            assert_missing_evidence_not_contains(labels.clone(), label, surface);
        }
    }
}

#[test]
fn java_completable_future_static_attribution_requires_type_identity() {
    let unimported = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "class Runtime { Object run() { return CompletableFuture.supplyAsync(() -> work()); } }\n",
        Lang::Java,
        "CompletableFuture.supplyAsync",
    );
    let local_type = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "class CompletableFuture { static Object supplyAsync(Object work) { return work; } }\nclass Runtime { Object run() { return CompletableFuture.supplyAsync(work()); } }\n",
        Lang::Java,
        "CompletableFuture.supplyAsync",
    );
    let local_variable = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run(Object CompletableFuture) { return CompletableFuture.supplyAsync(work()); } }\n",
        Lang::Java,
        "CompletableFuture.supplyAsync",
    );
    let conflicting_import = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.*;\nimport example.CompletableFuture;\nclass Runtime { Object run() { return CompletableFuture.supplyAsync(work()); } }\n",
        Lang::Java,
        "CompletableFuture.supplyAsync",
    );

    for (labels, surface) in [
        (unimported, "unimported Java CompletableFuture"),
        (local_type, "local Java CompletableFuture type"),
        (local_variable, "local Java CompletableFuture variable"),
        (
            conflicting_import,
            "conflicting Java CompletableFuture import",
        ),
    ] {
        assert_missing_evidence_not_contains(labels, "task-spawn-scheduling-contract", surface);
    }
}

#[test]
fn java_completable_future_wildcard_import_remains_open_without_conflicts() {
    let wildcard = runtime_boundary_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.*;\nclass Runtime { Object run() { return CompletableFuture.runAsync(() -> work()); } }\n",
        Lang::Java,
        "CompletableFuture.runAsync",
    );

    assert_missing_evidence_contains(
        wildcard,
        "task-spawn-scheduling-contract",
        "Java CompletableFuture wildcard import",
    );
}

#[test]
fn java_completable_future_wildcard_import_is_not_blocked_by_other_file_imports() {
    let wildcard = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "Other.java",
                "import java.util.concurrent.CompletableFuture;\nclass Other {}\n",
                Lang::Java,
            ),
            (
                "Runtime.java",
                "import java.util.concurrent.*;\nclass Runtime { Object run() { return CompletableFuture.runAsync(() -> work()); } }\n",
                Lang::Java,
            ),
        ],
        "Runtime.java",
        "CompletableFuture.runAsync",
    );

    assert_missing_evidence_contains(
        wildcard,
        "task-spawn-scheduling-contract",
        "Java CompletableFuture wildcard import with unrelated file import",
    );
}

#[test]
fn java_completable_future_wildcard_import_is_not_blocked_by_other_file_conflicts() {
    let wildcard = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "Other.java",
                "import example.CompletableFuture;\nclass Other {}\n",
                Lang::Java,
            ),
            (
                "Runtime.java",
                "import java.util.concurrent.*;\nclass Runtime { Object run() { return CompletableFuture.runAsync(() -> work()); } }\n",
                Lang::Java,
            ),
        ],
        "Runtime.java",
        "CompletableFuture.runAsync",
    );

    assert_missing_evidence_contains(
        wildcard,
        "task-spawn-scheduling-contract",
        "Java CompletableFuture wildcard import with unrelated file conflict",
    );
}

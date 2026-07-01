use super::{
    assert_missing_evidence_contains, assert_missing_evidence_not_contains,
    missing_evidence_for_lang_call, runtime_boundary_evidence_for_corpus_call,
};
use nose_il::Lang;

#[test]
fn java_completable_future_constructor_reports_manual_settlement_obligations() {
    let exact_import = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\nclass Runtime { Object run() { return new CompletableFuture<String>(); } }\n",
        Lang::Java,
        "CompletableFuture",
    );
    let wildcard_import = missing_evidence_for_lang_call(
        "Runtime.java",
        "import java.util.concurrent.*;\nclass Runtime { Object run() { return new CompletableFuture<String>(); } }\n",
        Lang::Java,
        "CompletableFuture",
    );
    let qualified = missing_evidence_for_lang_call(
        "Runtime.java",
        "class Runtime { Object run() { return new java.util.concurrent.CompletableFuture<String>(); } }\n",
        Lang::Java,
        "java.util.concurrent.CompletableFuture",
    );

    for labels in [&exact_import, &wildcard_import, &qualified] {
        assert!(labels.contains(&"future-settled-value-channel-contract"));
        assert!(labels.contains(&"exception-channel-contract"));
        assert!(labels.contains(&"task-handle-lifecycle-contract"));
        assert!(labels.contains(&"task-cancellation-liveness-contract"));
        assert!(!labels.contains(&"task-spawn-scheduling-contract"));
        assert!(!labels.contains(&"future-callback-demand-effect-contract"));
    }
}

#[test]
fn java_completable_future_constructor_wildcard_import_respects_same_package_type_shadow() {
    let same_package_shadow = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "p/CompletableFuture.java",
                "package p;\nclass CompletableFuture<T> {}\n",
                Lang::Java,
            ),
            (
                "p/Runtime.java",
                "package p;\nimport java.util.concurrent.*;\nclass Runtime { Object run() { return new CompletableFuture<String>(); } }\n",
                Lang::Java,
            ),
        ],
        "p/Runtime.java",
        "CompletableFuture",
    );
    let unrelated_package_shadow = runtime_boundary_evidence_for_corpus_call(
        &[
            (
                "p/CompletableFuture.java",
                "package p;\nclass CompletableFuture<T> {}\n",
                Lang::Java,
            ),
            (
                "q/Runtime.java",
                "package q;\nimport java.util.concurrent.*;\nclass Runtime { Object run() { return new CompletableFuture<String>(); } }\n",
                Lang::Java,
            ),
        ],
        "q/Runtime.java",
        "CompletableFuture",
    );

    for label in [
        "future-settled-value-channel-contract",
        "exception-channel-contract",
        "task-handle-lifecycle-contract",
        "task-cancellation-liveness-contract",
    ] {
        assert_missing_evidence_not_contains(
            same_package_shadow.clone(),
            label,
            "same-package Java CompletableFuture wildcard shadow",
        );
        assert_missing_evidence_contains(
            unrelated_package_shadow.clone(),
            label,
            "unrelated-package Java CompletableFuture wildcard shadow",
        );
    }
}

use super::missing_evidence_for_lang_call;
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

use super::*;

#[test]
fn recall_loss_report_keeps_java_completable_future_constructors_reporting_only() {
    let project = TempProject::new("recall_loss_java_completable_future_constructor");
    project.write(
        "Runtime.java",
        "import java.util.concurrent.CompletableFuture;\n\
class Runtime {\n\
  boolean a() { return new CompletableFuture<String>() == null; }\n\
  boolean b() { return new CompletableFuture<String>() == null; }\n\
}\n",
    );
    let report_path = project.path().join("recall-loss.json");
    let out = run_raw(&[
        "verify",
        project.path().to_str().unwrap(),
        "--max-violations",
        "0",
        "--recall-loss-report",
        report_path.to_str().unwrap(),
    ]);
    assert!(out.contains("GATE: 0"));

    let report_text = fs::read_to_string(&report_path).expect("recall-loss report");
    let report: serde_json::Value =
        serde_json::from_str(&report_text).expect("recall-loss report JSON");
    let rejections = report["admission_rejections"]
        .as_array()
        .expect("admission_rejections should be an array");
    assert!(
        rejections.iter().any(|item| item["reason"] == "unsupported-runtime-boundary"
            && item["capability_id"] == "runtime-boundary-model"
            && item["missing_evidence"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|value| value == "future-settled-value-channel-contract")
                    && items
                        .iter()
                        .any(|value| value == "task-cancellation-liveness-contract")
            })),
        "CompletableFuture constructors must remain reporting-only runtime boundaries, not exact-safe calls: {report}"
    );
}

#[test]
fn recall_loss_report_rejects_same_package_java_completable_future_wildcard_shadow() {
    let project =
        TempProject::new("recall_loss_java_completable_future_constructor_wildcard_shadow");
    fs::create_dir_all(project.path().join("src1/p")).expect("package dir");
    fs::create_dir_all(project.path().join("src2/p")).expect("package dir");
    project.write(
        "src1/p/CompletableFuture.java",
        "package p;\nclass CompletableFuture<T> {}\n",
    );
    project.write(
        "src2/p/Runtime.java",
        "package p;\n\
import java.util.concurrent.*;\n\
class Runtime {\n\
  boolean a() { return new CompletableFuture<String>() == null; }\n\
  boolean b() { return new CompletableFuture<String>() == null; }\n\
}\n",
    );
    let report_path = project.path().join("recall-loss.json");
    let out = run_raw(&[
        "verify",
        project.path().to_str().unwrap(),
        "--max-violations",
        "0",
        "--recall-loss-report",
        report_path.to_str().unwrap(),
    ]);
    assert!(out.contains("GATE: 0"));

    let report_text = fs::read_to_string(&report_path).expect("recall-loss report");
    let report: serde_json::Value =
        serde_json::from_str(&report_text).expect("recall-loss report JSON");
    let rejections = report["admission_rejections"]
        .as_array()
        .expect("admission_rejections should be an array");
    for rejected in rejections {
        let missing = rejected["missing_evidence"]
            .as_array()
            .expect("missing_evidence should be an array");
        for forbidden in [
            "future-settled-value-channel-contract",
            "exception-channel-contract",
            "task-handle-lifecycle-contract",
            "task-cancellation-liveness-contract",
        ] {
            assert!(
                !missing.iter().any(|value| value == forbidden),
                "same-package local CompletableFuture should not receive stdlib future obligation {forbidden}: {report}"
            );
        }
    }
}

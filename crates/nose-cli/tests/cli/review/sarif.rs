use super::*;

#[test]
fn review_sarif_top_records_truncation_for_review_and_query_base() {
    let dir = make_project("review_sarif_top");
    add_distinct_clone_family(&dir);
    init_git_repo(&dir);

    let a = dir.join("a/f.py");
    let src = fs::read_to_string(&a).unwrap();
    fs::write(
        &a,
        src.replace(
            "    return total",
            "    total = total + 1\n    return total",
        ),
    )
    .unwrap();
    let fresh = dir.join("new/fresh_a.py");
    let fresh_src = fs::read_to_string(&fresh).unwrap();
    fs::write(
        &fresh,
        fresh_src.replace(
            "    return total",
            "    total = total + 2\n    return total",
        ),
    )
    .unwrap();

    assert_sarif_truncated(
        nose_review(&dir, &["--format", "sarif", "--top", "1"]),
        "review",
        "--top 0",
    );
    assert_sarif_truncated(
        nose_query_in(&dir, &["base=main", "--format", "sarif", "top=1"]),
        "query base",
        "top=0",
    );
    assert_review_json_truncated(nose_review(&dir, &["--format", "json", "--top", "1"]));
    assert_query_base_json_truncated(nose_query_in(
        &dir,
        &["base=main", "--format", "json", "top=1"],
    ));

    let _ = fs::remove_dir_all(&dir);
}

fn assert_sarif_truncated(out: std::process::Output, label: &str, top_zero_hint: &str) {
    assert!(
        out.status.success(),
        "{label} should emit SARIF: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("review SARIF JSON");
    let run = &doc["runs"][0];
    assert_eq!(
        run["results"].as_array().map(Vec::len),
        Some(1),
        "{label} --top 1 emits one result: {doc}"
    );
    let props = &run["properties"];
    assert!(
        props["total_families"].as_u64().unwrap_or(0) >= 2,
        "{label} records the full divergent-family count: {doc}"
    );
    assert_eq!(props["shown_families"], 1, "{label} records shown count");
    assert!(
        run["invocations"][0]["toolExecutionNotifications"][0]["message"]["text"]
            .as_str()
            .is_some_and(|m| m.contains(top_zero_hint)),
        "{label} explains how to emit the full SARIF set: {doc}"
    );
}

fn assert_review_json_truncated(out: std::process::Output) {
    assert!(
        out.status.success(),
        "review JSON should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("review JSON");
    assert_eq!(
        doc["findings"].as_array().map(Vec::len),
        Some(1),
        "review JSON top=1 emits one finding: {doc}"
    );
    assert!(
        doc["total_inconsistent_families"].as_u64().unwrap_or(0) >= 2,
        "review JSON records the total before truncation: {doc}"
    );
    assert_eq!(doc["shown_inconsistent_families"], 1);
}

fn assert_query_base_json_truncated(out: std::process::Output) {
    assert!(
        out.status.success(),
        "query base JSON should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("query base JSON");
    assert_eq!(
        doc["items"].as_array().map(Vec::len),
        Some(1),
        "query base JSON top=1 emits one item: {doc}"
    );
    assert!(
        doc["summary"]["divergences"].as_u64().unwrap_or(0) >= 2,
        "query base JSON records the total before truncation: {doc}"
    );
    assert_eq!(doc["summary"]["shown_divergences"], 1);
    assert_eq!(doc["summary"]["limit"], 1);
}

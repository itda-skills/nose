use super::*;

#[test]
fn baseline_hides_accepted_families() {
    // Write a baseline accepting today's families, then a re-run reports nothing new.
    let dir = make_project("baseline");
    let p = dir.to_str().unwrap();
    let bl = std::env::temp_dir().join(format!("nose_bl_{}.json", std::process::id()));
    let bls = bl.to_str().unwrap();

    // Sanity: without a baseline there IS a family.
    assert!(run(&["query", p, "--min-size", "12"]).contains("copies"));

    // Accept current state…
    let _ = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bls,
            "--write-baseline",
        ])
        .output()
        .expect("run");
    assert!(bl.exists(), "baseline file should be written");
    let baseline_text = fs::read_to_string(&bl).expect("read baseline");
    assert!(
        baseline_text.contains("\"members\""),
        "baseline should record member identities for changed/resolved comparison: {baseline_text}"
    );
    assert!(
        baseline_text.contains("\"start_line\"") && baseline_text.contains("\"kind\""),
        "baseline member identities should include location fields for unique family ids: {baseline_text}"
    );

    // …then a re-run shows no *new* families.
    let after = run(&["query", p, "--min-size", "12", "--baseline", bls]);
    assert!(
        !after.contains("sites"),
        "baselined families must be hidden, got: {after}"
    );
    let _ = fs::remove_file(&bl);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_baseline_file_fails_clearly() {
    let dir = make_project("baseline_missing");
    let p = dir.to_str().unwrap();
    let bl =
        std::env::temp_dir().join(format!("nose_missing_baseline_{}.json", std::process::id()));
    let _ = fs::remove_file(&bl);

    let out = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bl.to_str().unwrap(),
        ])
        .output()
        .expect("run query");

    assert!(
        !out.status.success(),
        "an explicitly requested missing baseline must fail, not behave like an empty baseline"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("reading baseline"),
        "stderr should identify the missing baseline: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn malformed_baseline_file_fails_clearly() {
    let dir = make_project("baseline_malformed");
    let p = dir.to_str().unwrap();
    let bl = std::env::temp_dir().join(format!(
        "nose_malformed_baseline_{}.json",
        std::process::id()
    ));
    fs::write(&bl, "{ not json\n").unwrap();

    let out = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bl.to_str().unwrap(),
        ])
        .output()
        .expect("run query");

    assert!(
        !out.status.success(),
        "a malformed baseline must fail, not behave like an empty baseline"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("parsing baseline"),
        "stderr should identify the malformed baseline: {stderr}"
    );
    let _ = fs::remove_file(&bl);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn new_only_json_marks_new_families_against_baseline() {
    let dir = make_project("new_only");
    let p = dir.to_str().unwrap();
    let bl = std::env::temp_dir().join(format!("nose_new_only_bl_{}.json", std::process::id()));
    let bls = bl.to_str().unwrap();

    let baseline = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bls,
            "--write-baseline",
        ])
        .output()
        .expect("write baseline");
    assert!(baseline.status.success());

    add_distinct_clone_family(&dir);
    let out = run(&[
        "query",
        p,
        "--min-size",
        "12",
        "--baseline",
        bls,
        "--format",
        "json",
        "top=0",
    ]);
    let json = query_json(&out);
    let families = query_families(&json);
    assert!(
        !families.is_empty(),
        "new-only query should report the introduced family: {out}"
    );
    assert!(
        out.contains("fresh_a.py") && out.contains("fresh_b.py") && !out.contains("a/f.py"),
        "new-only JSON should include new sites, not accepted baseline sites: {out}"
    );
    assert_eq!(json["view"], "list");

    let _ = fs::remove_file(&bl);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fail_on_new_fails_for_changed_family_and_passes_when_clean() {
    let dir = make_project("fail_on_new");
    let p = dir.to_str().unwrap();
    let bl = std::env::temp_dir().join(format!("nose_fail_on_new_bl_{}.json", std::process::id()));
    let bls = bl.to_str().unwrap();

    let baseline = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bls,
            "--write-baseline",
        ])
        .output()
        .expect("write baseline");
    assert!(baseline.status.success());

    let clean = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bls,
            "--fail-on",
            "new",
        ])
        .output()
        .expect("clean run");
    assert!(
        clean.status.success(),
        "--fail-on-new should pass when every family is accepted"
    );

    add_member_to_existing_family(&dir);
    let changed = Command::new(bin())
        .args([
            "query",
            p,
            "--min-size",
            "12",
            "--baseline",
            bls,
            "--fail-on",
            "new",
            "--format",
            "json",
            "top=0",
        ])
        .output()
        .expect("changed run");
    assert!(
        !changed.status.success(),
        "--fail-on-new should fail when a family changes"
    );
    let stdout = String::from_utf8(changed.stdout).unwrap();
    let stderr = String::from_utf8(changed.stderr).unwrap();
    let json = query_json(&stdout);
    assert_eq!(query_families(&json).len(), 1);
    assert!(
        stderr.contains("--fail-on new"),
        "stderr should name the explicit gate: {stderr}"
    );

    let _ = fs::remove_file(&bl);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fail_on_new_requires_a_baseline() {
    let dir = make_project("fail_on_new_nobaseline");
    let p = dir.to_str().unwrap();
    // `--fail-on new` compares against a baseline; with no --baseline the gate can never
    // fire, so it must error rather than silently pass (a CI gate that always passes).
    let out = Command::new(bin())
        .args(["query", p, "--min-size", "12", "--fail-on", "new"])
        .output()
        .expect("run query");
    assert!(
        !out.status.success(),
        "`--fail-on new` without --baseline must error, not silently pass: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let _ = fs::remove_dir_all(&dir);
}

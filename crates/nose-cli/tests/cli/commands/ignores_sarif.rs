use super::*;

#[test]
fn inline_nose_ignore_suppresses_a_site() {
    // A `nose-ignore` marker above a function drops that site; with only one copy
    // left there's no family.
    let dir = std::env::temp_dir().join(format!("nose_sup_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let body = "def f(items):\n    t = 0\n    for x in items:\n        if x > 0:\n            t = t + x * x\n    return t\n";
    fs::create_dir_all(dir.join("a")).unwrap();
    fs::create_dir_all(dir.join("b")).unwrap();
    fs::write(dir.join("a/f.py"), body).unwrap();
    fs::write(dir.join("b/f.py"), format!("# nose-ignore\n{body}")).unwrap();
    let p = dir.to_str().unwrap();
    assert!(
        run(&["scan", p, "--min-size", "12"]).contains("no clone families found"),
        "the marked copy must be suppressed, leaving no family"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn structured_ignore_suppresses_family_id_with_metadata() {
    let dir = make_project("structured_ignore_id");
    let p = dir.to_str().unwrap();
    let before = run(&[
        "scan",
        p,
        "--mode",
        "semantic",
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let before_json = scan_json(&before);
    let family_id = scan_families(&before_json)[0]["family_id"]
        .as_str()
        .expect("family_id should be exposed")
        .to_string();
    let ignore_file = std::env::temp_dir().join(format!(
        "nose_structured_ignore_{}.json",
        std::process::id()
    ));
    fs::write(
        &ignore_file,
        format!(
            "{{\"ignores\":[{{\"family_id\":\"{family_id}\",\"reason\":\"generated-code\",\"note\":\"Generated from the same template.\",\"owner\":\"platform\",\"expires_at\":\"2099-01-01\"}}]}}\n"
        ),
    )
    .unwrap();

    let after = run(&[
        "scan",
        p,
        "--mode",
        "semantic",
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "0",
        "--ignore-file",
        ignore_file.to_str().unwrap(),
    ]);
    let after_json = scan_json(&after);
    assert!(
        scan_families(&after_json).is_empty(),
        "the ignored family should be absent from active findings: {after}"
    );
    assert_eq!(after_json["ignore"]["active_entries"], 1);
    assert_eq!(after_json["ignore"]["ignored_families"], 1);
    assert_eq!(after_json["ignored_families"][0]["family_id"], family_id);
    assert_eq!(
        after_json["ignored_families"][0]["ignore"]["reason"],
        "generated-code"
    );
    assert_eq!(
        after_json["ignored_families"][0]["ignore"]["owner"],
        "platform"
    );

    let _ = fs::remove_file(&ignore_file);
    let _ = fs::remove_dir_all(&dir);
}

/// Coevo S3-C4 packet c4-path-oversuppress: an entry covering only ONE member
/// must not hide the family — the other copies are first-party duplication the
/// `--fail-on` gate exists to catch. Selector semantics are ALL-members.
#[test]
fn partial_path_ignore_must_not_suppress_the_family() {
    let dir = make_project("ignore_partial_paths");
    fs::write(
        dir.join("nose.ignore.json"),
        "{\"ignores\":[{\"paths\":[\"a/**\"],\"reason\":\"vendored\"}]}\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args([
            "scan",
            ".",
            "--mode",
            "semantic",
            "--min-size",
            "12",
            "--format",
            "json",
            "--top",
            "0",
            "--fail-on",
            "any",
        ])
        .current_dir(&dir)
        .output()
        .expect("run");
    assert!(
        !out.status.success(),
        "the gate must still fire: a partially-covered family is not suppressed"
    );
    let json = scan_json(&String::from_utf8(out.stdout).unwrap());
    assert!(
        !scan_families(&json).is_empty(),
        "family with uncovered members stays reported: {json}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn default_structured_ignore_file_matches_paths() {
    let dir = make_project("structured_ignore_paths");
    fs::write(
        dir.join("nose.ignore.json"),
        "{\"ignores\":[{\"paths\":[\"a/**\",\"b/**\",\"tests/**\"],\"reason\":\"template-copy\",\"note\":\"a/ is generated.\"}]}\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args([
            "scan",
            ".",
            "--mode",
            "semantic",
            "--min-size",
            "12",
            "--format",
            "json",
            "--top",
            "0",
        ])
        .current_dir(&dir)
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "scan should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = scan_json(&stdout);
    assert!(
        scan_families(&json).is_empty(),
        "path ignore should suppress the family: {stdout}"
    );
    assert_eq!(json["ignore"]["active_entries"], 1);
    assert_eq!(json["ignore"]["ignored_families"], 1);
    assert_eq!(
        json["ignored_families"][0]["ignore"]["matched_paths"][0],
        "./a/f.py"
    );
    assert_eq!(
        json["ignored_families"][0]["ignore"]["selectors"]["paths"][0],
        "a/**"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn expired_structured_ignore_is_reported_but_not_applied() {
    let dir = make_project("structured_ignore_expired");
    fs::write(
        dir.join("nose.ignore.json"),
        "{\"ignores\":[{\"paths\":[\"a/**\"],\"reason\":\"temporary-waiver\",\"owner\":\"platform\",\"expires_at\":\"2000-01-01\"}]}\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args([
            "scan",
            ".",
            "--mode",
            "semantic",
            "--min-size",
            "12",
            "--format",
            "json",
            "--top",
            "0",
        ])
        .current_dir(&dir)
        .output()
        .expect("run");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let json = scan_json(&stdout);
    assert!(
        !scan_families(&json).is_empty(),
        "expired ignore must not suppress the family: {stdout}"
    );
    assert_eq!(json["ignore"]["active_entries"], 0);
    assert_eq!(json["ignore"]["expired_entries"], 1);
    assert_eq!(json["ignore"]["ignored_families"], 0);
    assert_eq!(json["ignore"]["expired"][0]["reason"], "temporary-waiver");
    assert!(
        stderr.contains("expired on 2000-01-01") && stderr.contains("not applied"),
        "stderr should explain the expired entry: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn malformed_structured_ignore_file_fails_clearly() {
    let dir = make_project("structured_ignore_bad");
    let ignore_file = dir.join("bad-ignore.json");
    fs::write(
        &ignore_file,
        "{\"ignores\":[{\"paths\":[\"a/**\"],\"note\":\"missing reason\"}]}\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args([
            "scan",
            dir.to_str().unwrap(),
            "--min-size",
            "12",
            "--ignore-file",
            ignore_file.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    assert!(!out.status.success(), "malformed ignore files must fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("parsing ignore file") || stderr.contains("validating ignore file"),
        "error should name the ignore file problem: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sarif_output_is_well_formed() {
    let dir = make_project("sarif");
    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--format",
        "sarif",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("SARIF must be valid JSON");
    assert_eq!(v["version"], "2.1.0");
    assert!(v["runs"][0]["tool"]["driver"]["name"] == "nose");
    assert!(
        !v["runs"][0]["results"].as_array().unwrap().is_empty(),
        "should have at least one result: {out}"
    );
    // The run records the full family count, so a consumer can tell a complete upload
    // from a truncated one. This single-family project is not truncated (default --top 30),
    // so total == shown and there is no truncation notification.
    let props = &v["runs"][0]["properties"];
    let total = props["total_families"].as_u64().expect("total_families");
    let shown = props["shown_families"].as_u64().expect("shown_families");
    assert_eq!(shown, total, "untruncated run: shown == total ({out})");
    assert_eq!(
        shown as usize,
        v["runs"][0]["results"].as_array().unwrap().len()
    );
    assert!(
        v["runs"][0].get("invocations").is_none(),
        "no truncation note when nothing is hidden: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A project with several behaviorally-distinct duplicated functions, so `nose scan`
/// reports multiple clone families. `--top N` can then truncate the report.
fn make_multi_family_project(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nose_cli_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    // Four distinct computations (distinct value fingerprints, so they don't merge into
    // one family), each duplicated across two directories → four families.
    let logics = [
        ("sq", "def f(items):\n    a = 0\n    for x in items:\n        if x > 0:\n            a = a + x * x\n    return a\n"),
        ("prod", "def f(items):\n    a = 1\n    for x in items:\n        a = a * x\n    return a\n"),
        ("cnt", "def f(items):\n    a = 0\n    for x in items:\n        if x < 0:\n            a = a + 1\n    return a\n"),
        ("join", "def f(items):\n    a = ''\n    for x in items:\n        a = a + x + ','\n    return a\n"),
    ];
    for sub in ["x", "y"] {
        for (name, src) in logics {
            let d = dir.join(sub);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("{name}.py")), src).unwrap();
        }
    }
    dir
}

#[test]
fn sarif_records_and_notes_top_truncation() {
    let dir = make_multi_family_project("sarif_trunc");

    // --top 1: only one family is emitted, but the run must record the true total and
    // carry an explicit truncation note pointing at --top 0.
    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--format",
        "sarif",
        "--top",
        "1",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("SARIF must be valid JSON");
    let props = &v["runs"][0]["properties"];
    let total = props["total_families"].as_u64().expect("total_families");
    let shown = props["shown_families"].as_u64().expect("shown_families");
    assert!(total >= 2, "fixture should yield multiple families: {out}");
    assert_eq!(shown, 1, "--top 1 shows exactly one family: {out}");
    assert_eq!(v["runs"][0]["results"].as_array().unwrap().len(), 1);
    let note = v["runs"][0]["invocations"][0]["toolExecutionNotifications"][0].clone();
    assert_eq!(note["level"], "note", "truncation note present: {out}");
    assert!(
        note["message"]["text"]
            .as_str()
            .unwrap()
            .contains("--top 0"),
        "note points the reader at --top 0: {out}"
    );

    // --top 0: the whole set is emitted, so there is no truncation note.
    let full = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--format",
        "sarif",
        "--top",
        "0",
    ]);
    let fv: serde_json::Value = serde_json::from_str(&full).expect("SARIF must be valid JSON");
    let fp = &fv["runs"][0]["properties"];
    assert_eq!(
        fp["shown_families"], fp["total_families"],
        "--top 0 emits every family: {full}"
    );
    assert!(
        fv["runs"][0].get("invocations").is_none(),
        "no truncation note with --top 0: {full}"
    );
    let _ = fs::remove_dir_all(&dir);
}

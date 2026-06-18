use super::*;

#[test]
fn scan_reports_the_clone_family() {
    let dir = make_project("fam");
    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--top",
        "10",
    ]);
    assert!(out.contains("clone"), "has a header: {out}");
    assert!(out.contains("copies"), "lists a family: {out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exclude_drops_matching_paths() {
    let dir = make_project("excl");
    let p = dir.to_str().unwrap();
    // The family spans a/, b/, tests/. Excluding `tests` must shrink it.
    let with = run(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "1",
    ]);
    let without = run(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "1",
        "--exclude",
        "tests",
    ]);
    let count = |s: &str| s.matches("\"file\"").count();
    assert!(
        count(&with) > count(&without),
        "excluding tests removes a site: {} vs {}",
        count(&with),
        count(&without)
    );
    assert!(
        !without.contains("/tests/"),
        "no tests/ path in excluded output"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exclude_supports_globs() {
    // `--exclude` takes gitignore-style globs, not just substrings: `a/**` excludes
    // the a/ directory specifically, leaving the family in b/ and tests/.
    let dir = make_project("glob");
    let p = dir.to_str().unwrap();
    let out = run(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--exclude",
        "a/**",
    ]);
    assert!(
        !out.contains("/a/f.py"),
        "glob `a/**` must exclude a/f.py: {out}"
    );
    assert!(out.contains("/b/f.py"), "b/ must remain: {out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn invalid_exclude_glob_fails_clearly() {
    let dir = make_project("bad_exclude_glob");
    let out = Command::new(bin())
        .args([
            "scan",
            dir.to_str().unwrap(),
            "--min-size",
            "12",
            "--exclude",
            "[",
        ])
        .output()
        .expect("run");
    assert!(
        !out.status.success(),
        "invalid exclude glob should fail instead of being ignored"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid exclude glob"),
        "stderr should explain the bad exclude glob: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn verify_json_reports_battery_bail_exclusions() {
    let dir = std::env::temp_dir().join(format!("nose_verify_bail_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let mut src = String::from("int huge(int x) {\n  int s = 0;\n");
    for i in 0..700 {
        src.push_str(&format!("  s = s + x + {i};\n"));
    }
    src.push_str("  return s;\n}\n\nint tiny(int x) {\n  return x + 1;\n}\n");
    fs::write(dir.join("huge.c"), src).unwrap();

    let out = run(&["verify", dir.to_str().unwrap(), "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("verify JSON");
    assert_eq!(
        json["exclusions"]["battery-bail"], 1,
        "oversized unit should fail closed into battery-bail: {out}"
    );
    assert!(
        json["excluded_units"]
            .as_array()
            .expect("excluded_units")
            .iter()
            .any(|unit| unit["reason"] == "battery-bail"
                && unit["file"]
                    .as_str()
                    .is_some_and(|file| file.ends_with("huge.c"))),
        "battery-bail unit should be named in excluded_units: {out}"
    );
    assert!(
        json["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["file"]
                .as_str()
                .is_some_and(|file| file.ends_with("huge.c"))),
        "small sibling function in the same file should still be verified: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn verify_max_violations_zero_accepts_sound_fixture() {
    let dir = make_project("verify_gate_green");
    let out = run(&["verify", dir.to_str().unwrap(), "--max-violations", "0"]);
    assert!(
        out.contains("PRESERVED: every canon-changed unit computes the same thing"),
        "verify gate should report canon preservation: {out}"
    );
    assert!(
        out.contains("GATE: 0"),
        "zero-violation gate should pass a sound fixture: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn min_value_filters_low_value_families() {
    let dir = make_project("minval");
    let p = dir.to_str().unwrap();
    // A value floor above any family's value hides them all; zero keeps them.
    let all = run(&["scan", p, "--min-size", "12", "--min-value", "0"]);
    let none = run(&["scan", p, "--min-size", "12", "--min-value", "100000"]);
    assert!(all.contains("copies"), "unfiltered shows a family: {all}");
    assert!(
        !none.contains("copies"),
        "a high value floor hides every family: {none}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn min_value_rejects_nan() {
    let dir = make_project("minval_nan");
    let out = Command::new(bin())
        .args([
            "scan",
            dir.to_str().unwrap(),
            "--min-size",
            "12",
            "--min-value",
            "NaN",
            "--fail-on",
            "any",
        ])
        .output()
        .expect("run scan");

    assert!(
        !out.status.success(),
        "--min-value NaN must be rejected instead of filtering every family out"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("min-value must be a finite non-negative number"),
        "stderr should explain the invalid value: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn hotspots_lists_modules() {
    let dir = make_project("hot");
    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--show",
        "hotspots",
    ]);
    assert!(
        out.contains("duplication hotspots"),
        "hotspots section present: {out}"
    );
    assert!(out.contains("dup lines"), "hotspot rows present: {out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn empty_corpus_warns_on_stderr() {
    // A path with no supported source must not silently look like "no duplication".
    let dir = std::env::temp_dir().join(format!("nose_empty_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("README.txt"), "no code here\n").unwrap();
    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .expect("run nose");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("no supported source files found"),
        "expected an empty-corpus warning, got: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn output_is_independent_of_thread_count() {
    // Parallel lowering/detection must not affect output: a single-threaded run and
    // a multi-threaded run must produce byte-identical JSON. A release invariant.
    let dir = make_project("threads");
    let p = dir.to_str().unwrap();
    let run_with_threads = |n: &str| {
        let out = Command::new(bin())
            .args(["scan", p, "--min-size", "12", "--format", "json"])
            .env("RAYON_NUM_THREADS", n)
            .output()
            .expect("run nose");
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap()
    };
    assert_eq!(
        run_with_threads("1"),
        run_with_threads("8"),
        "1-thread and 8-thread runs must produce identical output"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cache_output_matches_uncached_cold_and_warm() {
    // --cache-dir must not change results: cold (populating) and warm (reusing)
    // runs must both equal the non-cached output. This is the cache's correctness
    // contract — a stale/incorrect entry would diverge here.
    let dir = make_project("cache");
    // make_project's clones are *renamed* copies — matched only by the value-graph
    // channel. Add files with a *verbatim* duplicated block so the contiguous
    // (copy-paste) channel also produces a family: the cache once stored only units and
    // silently dropped that channel, so without the streams cached this run would diverge
    // here. (Identical variable names → identical raw token stream → a contiguous match,
    // unlike the renamed value-graph clones.)
    let block = "def handle(events):\n    out = []\n    for e in events:\n        if e.kind == 1:\n            out.append(e.payload)\n            record(e.id, e.kind)\n    return out\n";
    for name in ["dup_a", "dup_b", "dup_c"] {
        fs::write(dir.join(format!("{name}.py")), block).unwrap();
    }
    let p = dir.to_str().unwrap();
    let cache = std::env::temp_dir().join(format!("nose_cachedir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&cache);
    let cd = cache.to_str().unwrap();

    let uncached = run(&["scan", p, "--min-size", "12", "--format", "json"]);
    let cold = run(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--cache-dir",
        cd,
    ]);
    let warm = run(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--cache-dir",
        cd,
    ]);

    assert_eq!(
        uncached, cold,
        "cold cache run must match the uncached output"
    );
    assert_eq!(cold, warm, "warm cache run must match the cold run");
    let _ = fs::remove_dir_all(&cache);
    let _ = fs::remove_dir_all(&dir);
}

/// #275: the cached path must reproduce cross-file imported-immutable-literal
/// convergence. An inline `{…}.get(k)` and an `imported LOOKUP.get(k)` resolving
/// to the same literal form one family; the old source-content cache key skipped
/// the corpus resolve pass and under-merged. Editing the provider's literal must
/// also bust the importer's cached entry.
#[test]
fn cache_reproduces_cross_file_imported_literal_resolution() {
    let dir = std::env::temp_dir().join(format!("nose_c275_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("a")).unwrap();
    fs::create_dir_all(dir.join("b")).unwrap();
    fs::write(
        dir.join("a/local.py"),
        "def lookup(key, other):\n    return {\"red\": 1, \"blue\": 2}.get(key, 0)\n",
    )
    .unwrap();
    fs::write(
        dir.join("tables.py"),
        "LOOKUP = {\"red\": 1, \"blue\": 2}\n",
    )
    .unwrap();
    fs::write(
        dir.join("b/imported.py"),
        "from tables import LOOKUP\n\ndef lookup(key, other):\n    return LOOKUP.get(key, 0)\n",
    )
    .unwrap();
    let p = dir.to_str().unwrap();
    let cache = dir.join(".cache");
    let cd = cache.to_str().unwrap();
    let args = |extra: &[&str]| {
        let mut v = vec![
            "scan",
            p,
            "--mode",
            "semantic",
            "--min-size",
            "1",
            "--min-lines",
            "1",
            "--format",
            "json",
            "--top",
            "0",
        ];
        v.extend_from_slice(extra);
        run(&v)
    };
    let fams = |out: &str| scan_families(&scan_json(out)).len();

    let uncached = args(&[]);
    let cold = args(&["--cache-dir", cd]);
    let warm = args(&["--cache-dir", cd]);
    assert_eq!(fams(&uncached), 1, "uncached: imported literal converges");
    assert_eq!(uncached, cold, "cold cache must match uncached (#275)");
    assert_eq!(cold, warm, "warm cache must match cold (#275)");

    // Edit the provider's literal — the importer now resolves to a different
    // literal and must no longer merge; the cached entry must invalidate.
    fs::write(
        dir.join("tables.py"),
        "LOOKUP = {\"red\": 9, \"blue\": 2}\n",
    )
    .unwrap();
    let edited_uncached = args(&[]);
    let edited_cached = args(&["--cache-dir", cd]);
    assert_eq!(fams(&edited_uncached), 0, "edited: literals now differ");
    assert_eq!(
        edited_uncached, edited_cached,
        "cache must invalidate on provider literal change (#275)"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fail_flag_sets_exit_code_as_ci_gate() {
    let dir = make_project("fail");
    let p = dir.to_str().unwrap();
    // A family exists → --fail must exit non-zero.
    let found = Command::new(bin())
        .args(["scan", p, "--min-size", "12", "--fail-on", "any"])
        .output()
        .expect("run");
    assert!(
        !found.status.success(),
        "--fail must fail when a family is found"
    );
    // Filter everything out → nothing to fail on → success.
    let clean = Command::new(bin())
        .args([
            "scan",
            p,
            "--min-size",
            "12",
            "--min-value",
            "1e9",
            "--fail-on",
            "any",
        ])
        .output()
        .expect("run");
    assert!(
        clean.status.success(),
        "--fail must pass when no family survives filters"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn output_paths_are_relative_to_cwd() {
    // Even when scanned via an absolute path, reported paths should be relative to
    // the working directory (readable in CI logs / reviews).
    let dir = make_project("relpath");
    let abs = dir.canonicalize().unwrap();
    let out = Command::new(bin())
        .args([
            "scan",
            abs.to_str().unwrap(),
            "--min-size",
            "12",
            "--format",
            "json",
        ])
        .current_dir(&abs)
        .output()
        .expect("run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"a/f.py\"") || stdout.contains("\"b/f.py\""),
        "paths should be relative to cwd, got: {stdout}"
    );
    assert!(
        !stdout.contains(abs.to_str().unwrap()),
        "no absolute path should leak: {stdout}"
    );
    let _ = fs::remove_dir_all(&dir);
}

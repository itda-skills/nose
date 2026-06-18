use super::*;

#[test]
fn sort_keys_label_the_ranking_and_reject_garbage() {
    let dir = make_project("sort");
    let p = dir.to_str().unwrap();
    // Default ranking is extractability — the header says so, in plain language.
    let def = run(&["scan", p, "--min-size", "12"]);
    assert!(
        def.contains("ranked by extractability"),
        "default header names the ranking: {def}"
    );
    // --sort hazard is available and switches the header.
    let byhaz = run(&["scan", p, "--min-size", "12", "--sort", "hazard"]);
    assert!(
        byhaz.contains("ranked by divergent-edit hazard"),
        "--sort hazard names the ranking: {byhaz}"
    );
    // Families are described in plain language (copies + removable lines), not a
    // wall of internal metrics.
    assert!(
        def.contains("copies") && def.contains("lines removable"),
        "family summary is plain-language: {def}"
    );
    // --sort value switches the header.
    let byval = run(&["scan", p, "--min-size", "12", "--sort", "value"]);
    assert!(
        byval.contains("raw duplicated volume"),
        "--sort value names the ranking: {byval}"
    );
    // An unknown sort key is rejected by clap.
    let bad = Command::new(bin())
        .args(["scan", p, "--sort", "bogus"])
        .output()
        .expect("run nose");
    assert!(!bad.status.success(), "unknown --sort value is rejected");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn scan_reports_what_it_scanned() {
    let dir = make_project("scope");
    let p = dir.to_str().unwrap();
    // The scope line states the file count and per-language breakdown (make_project
    // writes four Python files), so a `.gitignore`-pruned scope is visible, not silent.
    let out = run(&["scan", p, "--min-size", "12"]);
    assert!(
        out.contains("scanned 4 files") && out.contains("python 4"),
        "header reports scanned count and languages: {out}"
    );
    // The scope line must not corrupt machine-readable output.
    let json = run(&["scan", p, "--min-size", "12", "--format", "json"]);
    let report = scan_json(&json);
    assert!(
        json.trim_start().starts_with('{') && !json.contains("scanned"),
        "json output stays a pure object without human text: {json}"
    );
    assert_eq!(report["scope"]["files"], 4);
    assert_eq!(report["scope"]["languages"][0]["language"], "python");
    assert_eq!(report["scope"]["languages"][0]["files"], 4);
    let _ = fs::remove_dir_all(&dir);
}

/// A determinism guard with teeth: the toy `make_project` is too small to exercise the
/// order-sensitive paths (shared-line IDF summation, RANSAC offset ties, cross-family
/// ordering) where byte-identical output has actually broken before. Generate a project
/// with many same-language near-duplicate functions — several clone families whose copies
/// differ in a few lines — and assert the JSON report is identical across a range of
/// thread counts (each a distinct process, so this also covers `HashMap` seed variation).
#[test]
fn output_is_byte_identical_across_thread_counts_on_a_rich_project() {
    let dir = std::env::temp_dir().join(format!("nose_determinism_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // 8 families × 5 near-duplicate copies. Each copy renames variables and tweaks a
    // couple of lines, so families form, RANSAC alignment runs, and the honest
    // shared-line counter compares several members.
    for fam in 0..8 {
        for copy in 0..5 {
            let v = format!("v{copy}");
            let src = format!(
                "def family{fam}_{copy}(items):\n    \
                 {v}_total = {copy}\n    \
                 {v}_seen = []\n    \
                 for {v}_x in items:\n        \
                 if {v}_x > {fam}:\n            \
                 {v}_total = {v}_total + {v}_x * {fam}\n            \
                 {v}_seen.append({v}_x)\n        \
                 else:\n            \
                 {v}_total = {v}_total - {copy}\n    \
                 return ({v}_total, {v}_seen)\n"
            );
            fs::write(dir.join(format!("f{fam}_{copy}.py")), src).unwrap();
        }
    }
    let p = dir.to_str().unwrap();
    let run_threads = |n: &str| {
        let out = Command::new(bin())
            .args([
                "scan",
                p,
                "--mode",
                "near:0.5",
                "--min-size",
                "12",
                "--format",
                "json",
                "--top",
                "1000",
            ])
            .env("RAYON_NUM_THREADS", n)
            .output()
            .expect("run nose");
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap()
    };
    let baseline = run_threads("1");
    assert!(
        baseline.contains("\"members\""),
        "the fixture forms families: {baseline}"
    );
    for n in ["2", "3", "4", "8"] {
        assert_eq!(
            run_threads(n),
            baseline,
            "{n}-thread output must be byte-identical to the 1-thread run"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn top_zero_shows_all_families() {
    // `--top 0` is documented as "no limit" (docs/usage.md). It must return every
    // family, not an empty set. Regression: `.take(0)` used to silently drop all.
    let dir = std::env::temp_dir().join(format!("nose_top0_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    // Build several *structurally distinct* clone families so "all" is clearly more
    // than "top 1". nose finds semantic (Type-4) clones, so each family must have a
    // different shape (statement count / control flow) or they collapse into one.
    let bodies = [
        "def f0(items):\n    acc = 0\n    for x in items:\n        acc = acc + x\n    return acc\n",
        "def f1(items):\n    acc = 1\n    for x in items:\n        if x > 0:\n            acc = acc * x\n    return acc\n",
        "def f2(s):\n    out = []\n    for c in s:\n        out.append(c)\n        out.append(c)\n    return out\n",
        "def f3(a, b):\n    r = 0\n    while a < b:\n        r = r + a\n        a = a + 1\n    return r\n",
        "def f4(d):\n    keys = []\n    for k in d:\n        if k is not None:\n            keys.append(k)\n    keys.sort()\n    return keys\n",
        "def f5(n):\n    total = 0\n    i = 0\n    while i < n:\n        total = total + i * i\n        i = i + 1\n    return total\n",
    ];
    for (i, body) in bodies.iter().enumerate() {
        for sub in ["a", "b"] {
            let d = dir.join(sub);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("f{i}.py")), body).unwrap();
        }
    }
    let p = dir.to_str().unwrap();
    let count = |args: &[&str]| -> usize {
        let out = run(args);
        let v = scan_json(&out);
        scan_families(&v).len()
    };

    let all = count(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let one = count(&[
        "scan",
        p,
        "--min-size",
        "12",
        "--format",
        "json",
        "--top",
        "1",
    ]);
    assert!(all > 1, "--top 0 must show all families, got {all}");
    assert_eq!(one, 1, "--top 1 must still cap at one family, got {one}");
    assert!(
        all >= 6,
        "--top 0 should include every distinct family (expected >=6, got {all})"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn scan_errors_on_nonexistent_path() {
    // A typo'd path in a CI gate must fail loudly, not pass on an empty report.
    let out = Command::new(bin())
        .args(["scan", "/nonexistent/nose-test-path"])
        .output()
        .expect("run nose");
    assert!(
        !out.status.success(),
        "scan on a missing path must exit non-zero"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("path does not exist"),
        "stderr names the problem: {err}"
    );
}

#[test]
fn scan_human_report_ends_with_show_hint() {
    let dir = make_project("hint");
    let p = dir.to_str().unwrap();
    let plain = run(&["scan", p, "--min-size", "12"]);
    assert!(
        plain.contains("hint: `--show diff`"),
        "default report points at the next step: {plain}"
    );
    let with_diff = run(&["scan", p, "--min-size", "12", "--show", "diff"]);
    assert!(
        !with_diff.contains("hint: `--show diff`"),
        "hint disappears once a view is requested: {with_diff}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The field shape (craken-agents audit, family b2d89110599bb1dd): an
/// imports-only module and the same import block at a different offset inside
/// a larger module form a clone family whose every member span is pure
/// declarations — it must leave the default surface (mechanically
/// non-actionable, design.md §2b) while a real function family stays.
#[test]
fn declaration_runs_leave_the_default_surface() {
    let dir = std::env::temp_dir().join(format!("nose_decl_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let imports = "import { aleph } from './aleph';\nimport { beth } from './beth';\nimport { gimel } from './gimel';\nimport { dalet } from './dalet';\nimport { hewav } from './hewav';\nimport { zayin } from './zayin';\nimport { hethx } from './hethx';\nimport { tethy } from './tethy';\n";
    let shared_fn = "export function totalPositive(items: number[]): number {\n    let total = 0;\n    for (const x of items) {\n        if (x > 0) {\n            total = total + x * x;\n        }\n    }\n    return total;\n}\n";
    for sub in ["a", "b", "c", "d"] {
        fs::create_dir_all(dir.join(sub)).unwrap();
    }
    fs::write(dir.join("a/index.ts"), imports).unwrap();
    fs::write(
        dir.join("b/index.ts"),
        format!(
            "export const seedB = 7;\nexport const labelB = 'b';\n\n{imports}\nconst namedB = new Map();\nnamedB.set('gimel', gimel);\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("c/f.ts"),
        format!("import {{ qoph }} from './qoph';\n\n{shared_fn}"),
    )
    .unwrap();
    fs::write(
        dir.join("d/f.ts"),
        format!("import {{ resh }} from './resh';\n\n{shared_fn}"),
    )
    .unwrap();
    let p = dir.to_str().unwrap();

    let human = run(&["scan", p, "--min-size", "8", "--min-lines", "3"]);
    assert!(
        human.contains("declaration-run"),
        "omission note names the declaration run: {human}"
    );
    assert!(
        !human.contains("a/index.ts"),
        "the import block is not a default finding: {human}"
    );
    assert!(
        human.contains("totalPositive"),
        "the real function family still reports: {human}"
    );

    let json = run(&[
        "scan",
        p,
        "--min-size",
        "8",
        "--min-lines",
        "3",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let v = scan_json(&json);
    let decl: Vec<_> = scan_families(&v)
        .iter()
        .filter(|f| f["recommended_surface"] == "declaration")
        .cloned()
        .collect();
    assert!(
        !decl.is_empty(),
        "JSON keeps the declaration family for diagnostics: {json}"
    );
    assert!(
        v["ranking"]["surface_counts"]["declaration"].as_u64() >= Some(1),
        "surface_counts reports the declaration class: {json}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Fail-open guard: a span that mixes imports with one real statement is NOT a
/// declaration run — misclassifying a real finding is the error class the
/// filter must never make.
#[test]
fn mixed_import_and_code_span_stays_on_the_default_surface() {
    let dir = std::env::temp_dir().join(format!("nose_declmix_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let block = "import { aleph } from './aleph';\nimport { beth } from './beth';\nimport { gimel } from './gimel';\nexport const registry = { aleph, beth, gimel };\n";
    fs::create_dir_all(dir.join("a")).unwrap();
    fs::create_dir_all(dir.join("b")).unwrap();
    fs::write(dir.join("a/f.ts"), block).unwrap();
    fs::write(dir.join("b/f.ts"), block).unwrap();
    let p = dir.to_str().unwrap();

    let json = run(&[
        "scan",
        p,
        "--min-size",
        "8",
        "--min-lines",
        "3",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let v = scan_json(&json);
    assert!(
        scan_families(&v)
            .iter()
            .all(|f| f["recommended_surface"] != "declaration"),
        "a span with real code must not be classified declaration: {json}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `--scope prod` drops all-test families (issue #264: read production
/// findings first); `--scope test` keeps only them; the default shows both.
#[test]
fn scope_flag_filters_by_test_boundary() {
    let dir = std::env::temp_dir().join(format!("nose_scope_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let prod_fn = "def total_positive(items):\n    total = 0\n    for x in items:\n        if x > 0:\n            total = total + x * x\n    return total\n";
    let test_fn = "def check_round_trip(payload):\n    encoded = encode(payload)\n    decoded = decode(encoded)\n    assert decoded == payload\n    assert len(encoded) > 0\n    return decoded\n";
    fs::create_dir_all(dir.join("src/a")).unwrap();
    fs::create_dir_all(dir.join("src/b")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(dir.join("src/a/calc.py"), prod_fn).unwrap();
    fs::write(dir.join("src/b/calc.py"), prod_fn).unwrap();
    fs::write(dir.join("tests/test_one.py"), test_fn).unwrap();
    fs::write(dir.join("tests/test_two.py"), test_fn).unwrap();
    let p = dir.to_str().unwrap();

    let all = run(&["scan", p, "--min-size", "8", "--min-lines", "3"]);
    assert!(
        all.contains("src/a/calc.py") && all.contains("tests/test_one.py"),
        "default shows both scopes: {all}"
    );
    let prod = run(&[
        "scan",
        p,
        "--min-size",
        "8",
        "--min-lines",
        "3",
        "--scope",
        "prod",
    ]);
    assert!(
        prod.contains("src/a/calc.py") && !prod.contains("tests/test_one.py"),
        "--scope prod drops the all-test family: {prod}"
    );
    let test = run(&[
        "scan",
        p,
        "--min-size",
        "8",
        "--min-lines",
        "3",
        "--scope",
        "test",
    ]);
    assert!(
        test.contains("tests/test_one.py") && !test.contains("src/a/calc.py"),
        "--scope test keeps only the all-test family: {test}"
    );
    let _ = fs::remove_dir_all(&dir);
}

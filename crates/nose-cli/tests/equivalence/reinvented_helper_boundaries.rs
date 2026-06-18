use super::*;

#[test]
fn reinvented_helper_containment_fires_and_excludes_callers() {
    // The containment channel: a function that REIMPLEMENTS an existing pure helper
    // inline (without calling it) is reported, with the helper's whole-body value hash
    // matched as an interior sub-DAG anchor of the container. A function that CALLS the
    // helper — whose fingerprint contains the same hash via interprocedural inlining —
    // must NOT be reported: calling is the fix, not the smell.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    // File 0: a straight-line pure helper big enough for the helper floor, plus a
    // caller of it (inlined → contains the helper's value graph → must be excluded).
    let helper_and_caller = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n\nfunction use(x, y) {\n    return big(x, y) + 1\n}\n";
    // File 1: a function that reimplements `big`'s computation inline and does more.
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let il0 = nose_frontend::lower_source(
        FileId(0),
        "a.js",
        helper_and_caller.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let il1 = nose_frontend::lower_source(
        FileId(1),
        "b.js",
        reinventor.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    let findings = nose_detect::reinvented_helpers(&units);
    assert_eq!(
        findings.len(),
        1,
        "exactly the reimplementation (not the caller) must be reported",
    );
    let f = &findings[0];
    assert_eq!(
        (f.container_file.as_str(), f.container_name.as_deref()),
        ("b.js", Some("manual")),
        "the container is the function reimplementing the helper",
    );
    assert_eq!(
        (f.helper_file.as_str(), f.helper_name.as_deref()),
        ("a.js", Some("big")),
        "the helper is the existing function being reinvented",
    );
}

#[test]
fn reinvented_helper_skips_effectful_and_guard_mismatched_helpers() {
    // An effectful helper (its sink profile is not pure-single-return) never becomes a
    // containment helper — replacing inline code with a call would ADD its effect.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let eff_helper = "function bigLog(x, y, log) {\n    log.push(x)\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let il0 = nose_frontend::lower_source(
        FileId(0),
        "a.js",
        eff_helper.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let il1 = nose_frontend::lower_source(
        FileId(1),
        "b.js",
        reinventor.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    assert!(
        nose_detect::reinvented_helpers(&units).is_empty(),
        "an effectful helper must not produce containment findings",
    );
}

#[test]
fn decorated_function_callers_fail_closed_coevo_s6_s2a() {
    // coevo series 6, S2-A: Python decorators are dropped in lowering, so a decorated
    // helper's runtime binding is `decorator(f)`, not the bare body. The fix records a
    // SourceFactKind::Binding(DecoratedDefinition) fact so the decorated def gets no
    // DirectFunction evidence — its callers fall back to an opaque call and are NOT
    // admitted to the exact `semantic` channel (exact_safe=false), so they can never be
    // reported as an "exact behavior match" that hides the decorator's effect. A caller
    // of a PLAIN helper still inlines and stays exact_safe.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let caller_exact_safe = |src: &str| -> bool {
        let il = nose_frontend::lower_source(FileId(0), "t.py", src.as_bytes(), Lang::Python, &i)
            .unwrap();
        nose_detect::units_of_file(&il, &i, &opts)
            .iter()
            .find(|u| u.name.as_deref() == Some("caller"))
            .expect("caller unit")
            .exact_safe
    };
    let decorated = "def double(f):\n    return lambda x: f(x) * 2\n\n@double\ndef helper(x):\n    return x * 5 + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    let plain =
        "def helper(x):\n    return x * 5 + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    assert!(
        !caller_exact_safe(decorated),
        "a caller of a DECORATED helper must not be exact-safe (no inline, fail closed)",
    );
    assert!(
        caller_exact_safe(plain),
        "a caller of a PLAIN helper still inlines and stays exact-safe (no recall loss)",
    );
}

#[test]
fn reinvented_helper_excludes_caller_via_inlined_span_coevo_s6_s2() {
    // coevo series 6, S3-2: a pure caller of a function that inline-reinvents the helper
    // must NOT itself be reported (the called-helper record is one call level deep). The
    // span-containment gate rejects it: the matched anchor's real span lies outside the
    // caller's own line range (it belongs to the inlined callee).
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    // `reinventor` reimplements `big` inline; `passThrough` merely CALLS reinventor.
    let src = "function reinventor(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) + 1\n}\n\nfunction passThrough(x, y) {\n    return reinventor(x, y) + 2\n}\n";
    let il0 =
        nose_frontend::lower_source(FileId(0), "h.js", helper.as_bytes(), Lang::JavaScript, &i)
            .unwrap();
    let il1 = nose_frontend::lower_source(FileId(1), "r.js", src.as_bytes(), Lang::JavaScript, &i)
        .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    let findings = nose_detect::reinvented_helpers(&units);
    assert!(
        findings
            .iter()
            .all(|f| f.container_name.as_deref() != Some("passThrough")),
        "a caller of an inline-reinventor must not be reported as reinventing the helper, got {:?}",
        findings
            .iter()
            .map(|f| f.container_name.clone())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn reinvented_helper_rejects_bound_blind_fold_coevo_s6_s3() {
    // coevo series 6, S3-3: an indexed `while i < n` fold absorbs the bound into a
    // pointer-length contract, dropping it from the value hash, so a fold over a
    // DIFFERENT bound must NOT be reported as containment (it computes a different value).
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "def poly_sum(xs, n):\n    total = 0\n    i = 0\n    while i < n:\n        total = total + xs[i] * xs[i] + 3 * xs[i] + 7\n        i = i + 1\n    return total\n";
    let container = "def poly_partial(xs, n, k):\n    total = 0\n    i = 0\n    while i < n - 1:\n        total = total + xs[i] * xs[i] + 3 * xs[i] + 7\n        i = i + 1\n    return total * k + 9\n";
    let il0 = nose_frontend::lower_source(FileId(0), "a.py", helper.as_bytes(), Lang::Python, &i)
        .unwrap();
    let il1 =
        nose_frontend::lower_source(FileId(1), "b.py", container.as_bytes(), Lang::Python, &i)
            .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    assert!(
        nose_detect::reinvented_helpers(&units).is_empty(),
        "a fold whose bound the value hash drops must not match a different-bound fold",
    );
}

#[test]
fn reinvented_helper_flags_test_container_for_default_exclusion() {
    // The promotion field audit (2026-06-13) excludes test-container findings from the
    // bare-default surface (a test asserting the helper's value as a literal is circular
    // to "fix"). The `container_in_test` flag drives that — set when the container file
    // is a test path, regardless of the helper's location.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let run = |container_path: &str| -> Option<bool> {
        let il0 = nose_frontend::lower_source(
            FileId(0),
            "helper.js",
            helper.as_bytes(),
            Lang::JavaScript,
            &i,
        )
        .unwrap();
        let il1 = nose_frontend::lower_source(
            FileId(1),
            container_path,
            reinventor.as_bytes(),
            Lang::JavaScript,
            &i,
        )
        .unwrap();
        let mut units = nose_detect::units_of_file(&il0, &i, &opts);
        units.extend(nose_detect::units_of_file(&il1, &i, &opts));
        nose_detect::reinvented_helpers(&units)
            .first()
            .map(|f| f.container_in_test)
    };
    assert_eq!(
        run("src/math.js"),
        Some(false),
        "a prod-path container is not flagged in_test"
    );
    assert_eq!(
        run("test/math.test.js"),
        Some(true),
        "a test-path container is flagged in_test"
    );
}

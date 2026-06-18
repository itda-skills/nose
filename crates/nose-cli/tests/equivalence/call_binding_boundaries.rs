use super::*;

#[test]
fn keyword_argument_mapping_is_by_name_not_position_issue_301() {
    // #301 (coevo series 6, S1): Python keyword arguments lower to a KwArg node carrying
    // the name, so a call's argument identity is BY NAME. Two callers passing the SAME
    // (name -> value) mapping in different orders converge; two passing DIFFERENT
    // mappings do not — the inline binds `helper(b=p, a=q)` to the right parameters.
    let i = Interner::new();
    let helper = "def helper(a, b):\n    base = a - b\n    return base * 3 + a\n";
    let call_ab = "def run(p, q):\n    return helper(a=p, b=q)\n";
    let call_ba_same = "def run(p, q):\n    return helper(b=q, a=p)\n"; // same mapping, reordered
    let call_ba_diff = "def run(p, q):\n    return helper(b=p, a=q)\n"; // different mapping
    let fp = |c: &str| value_fp_named(&i, &format!("{helper}\n{c}"), Lang::Python, "run");
    assert_eq!(
        fp(call_ab),
        fp(call_ba_same),
        "same keyword->value mapping in different order must converge",
    );
    assert_ne!(
        fp(call_ab),
        fp(call_ba_diff),
        "different keyword->value mapping must NOT converge (the #301 false merge)",
    );
}

#[test]
fn keyword_argument_oracle_binds_by_name_issue_301() {
    // The verify oracle must bind keyword args by name too, so it neither mis-binds a
    // reordered keyword call (a silent false merge) nor needlessly excludes it. The
    // differently-mapped callers compute different values on the same inputs.
    use nose_normalize::{run_unit, Value};
    let i = Interner::new();
    let src_diff = "def helper(a, b):\n    return (a - b) * 3 + a\n\ndef run(p, q):\n    return helper(b=p, a=q)\n";
    let il = nose_frontend::lower_source(FileId(0), "t.py", src_diff.as_bytes(), Lang::Python, &i)
        .unwrap();
    let n = normalize(&il, &i, &NormalizeOptions::default());
    let run = n
        .units
        .iter()
        .find(|u| u.name.is_some_and(|s| i.resolve(s) == "run"))
        .map(|u| u.root)
        .expect("run unit");
    // run(p=1, q=2) calls helper(b=1, a=2) → (a-b)*3+a = (2-1)*3+2 = 5.
    assert_eq!(
        run_unit(&n, &i, run, &[Value::Int(1), Value::Int(2)])
            .expect("interpretable")
            .ret,
        Value::Int(5),
        "the oracle must bind helper(b=p, a=q) by name: a=q=2, b=p=1",
    );
}

#[test]
fn global_reassigned_function_fails_closed_issue_302() {
    // #302: a module function reassigned via `global name; name = ...` from inside another
    // function no longer binds its `def` body, so its callers must NOT inline that body
    // (they would false-merge across files that reassign it differently). A LOCAL
    // assignment to the same name (no `global`) is a different binding and must NOT gate
    // the function — the precise distinction the series-6 reassigned-anywhere predicate
    // could not draw (it over-fired). Measured via exact-safety: an inlined caller is
    // exact-safe; an opaque (un-inlined) one is not.
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
    let reassigned = "def helper(x):\n    return x * 5 + 1\n\ndef setup():\n    global helper\n    helper = other\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    let local_shadow = "def helper(x):\n    return x * 5 + 1\n\ndef elsewhere():\n    helper = 5\n    return helper + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    assert!(
        !caller_exact_safe(reassigned),
        "a caller of a `global`-reassigned function must not be exact-safe (fail closed)",
    );
    assert!(
        caller_exact_safe(local_shadow),
        "a local `helper = 5` (no `global`) must NOT gate the function — caller still inlines",
    );
}

#[test]
fn splat_argument_is_distinct_from_plain_argument_coevo_s7_s1() {
    // coevo series 7, S1: `f(*args)` unpacks an iterable into positional params and
    // `f(**d)` unpacks a mapping into keywords — neither is `f(arg)`. The frontend used
    // to strip the splat, so `stats(*xs)` lowered identically to `stats(xs)` and the two
    // false-merged (different on `[[1,2,3]]`: len 3 vs 1). A `Splat` node now keeps them
    // distinct.
    let i = Interner::new();
    let helper = "def stats(a):\n    total = len(a)\n    return total * total + total\n";
    let via_splat = "def via(xs):\n    return stats(*xs)\n";
    let via_plain = "def via(xs):\n    return stats(xs)\n";
    let fp = |c: &str| value_fp_named(&i, &format!("{helper}\n{c}"), Lang::Python, "via");
    assert_ne!(
        fp(via_splat),
        fp(via_plain),
        "a `*args` spread must not fingerprint as a plain positional argument",
    );
    let via_kwsplat = "def via(d):\n    return stats(**d)\n";
    assert_ne!(
        fp(via_kwsplat),
        fp(via_plain),
        "a `**kwargs` spread must not fingerprint as a plain positional argument",
    );
    assert_ne!(
        fp(via_splat),
        fp(via_kwsplat),
        "`*args` and `**kwargs` spreads must stay distinct from each other",
    );
}

#[test]
fn global_rebind_recorded_for_all_assignment_forms_coevo_s7_s2() {
    // coevo series 7, S2: the #302 fix recorded `ModuleRebind` only for a plain
    // single-identifier `global helper; helper = x`. Tuple-unpack, aug-assign, and walrus
    // all lower to an `Assign` too and must also withhold the rebound function. Measured
    // via exact-safety: a caller of a rebound function is opaque (not exact-safe).
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
    let helper = "def helper(x):\n    return x * 5 + 1\n";
    let caller = "def caller(a):\n    return helper(a) * 10 + a\n";
    let tuple = format!(
        "{helper}\ndef setup():\n    global helper\n    helper, spare = other, 0\n\n{caller}"
    );
    let aug = format!("{helper}\ndef setup():\n    global helper\n    helper += 1\n\n{caller}");
    let walrus =
        format!("{helper}\ndef setup():\n    global helper\n    (helper := other)\n\n{caller}");
    for (label, src) in [
        ("tuple-unpack", &tuple),
        ("aug-assign", &aug),
        ("walrus", &walrus),
    ] {
        assert!(
            !caller_exact_safe(src),
            "a caller of a `global`-rebound function ({label}) must not be exact-safe",
        );
    }
    // Precise: a LOCAL `helper = 5` (no `global`) still leaves the function inlinable.
    let local =
        format!("{helper}\ndef elsewhere():\n    helper = 5\n    return helper + 1\n\n{caller}");
    assert!(
        caller_exact_safe(&local),
        "a local shadow (no `global`) must NOT withhold the function",
    );
}

#[test]
fn effectful_keyword_reorder_stays_distinct_coevo_s7_s3() {
    // coevo series 7, S3: the keyword name-sort (#304) is sound only for effect-free
    // values — Python evaluates args in SOURCE order, so reordering effectful keyword
    // values changes the effect/exception order. With a call-valued keyword the two
    // orderings must stay distinct; with pure values they still converge.
    let i = Interner::new();
    let eff_a = "def use(x, y):\n    return combine(a=sideA(x), b=sideB(y))\n";
    let eff_b = "def use(x, y):\n    return combine(b=sideB(y), a=sideA(x))\n";
    assert_ne!(
        value_fp_named(&i, eff_a, Lang::Python, "use"),
        value_fp_named(&i, eff_b, Lang::Python, "use"),
        "reordered EFFECTFUL keyword values must not converge (eval order differs)",
    );
    let pure_a = "def use(p, q):\n    return combine(a=p, b=q)\n";
    let pure_b = "def use(p, q):\n    return combine(b=q, a=p)\n";
    assert_eq!(
        value_fp_named(&i, pure_a, Lang::Python, "use"),
        value_fp_named(&i, pure_b, Lang::Python, "use"),
        "reordered PURE keyword values still converge (no observable order)",
    );
}

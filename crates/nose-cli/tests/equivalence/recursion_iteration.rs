use super::*;

// ---------------------------------------------------------------------------
// Recursion ↔ iteration (recursion.rs). Tail recursion and numeric structural
// recursion are rewritten to the loop a programmer would have written, so they
// converge with hand-written iteration and with each other — cross-language too.
// The negatives guard the soundness boundary: a different op, scale, or base must
// keep a distinct value fingerprint (no false merge).
// ---------------------------------------------------------------------------

#[test]
fn tail_recursion_converges_with_while_loop() {
    let i = Interner::new();
    let rec = "def f(n, acc):\n    if n == 0:\n        return acc\n    return f(n - 1, acc + n)\n";
    let loopv = "def g(n, acc):\n    while n != 0:\n        acc = acc + n\n        n = n - 1\n    return acc\n";
    assert_eq!(
        value_fp(&i, rec, Lang::Python),
        value_fp(&i, loopv, Lang::Python),
        "tail-accumulator recursion should converge with the equivalent while loop"
    );
}

#[test]
fn tail_recursion_converges_cross_language() {
    // Python accumulator recursion ≡ a typed TypeScript while loop — the shared IL makes the
    // recursion→iteration rewrite cross-language for free.
    let i = Interner::new();
    let py = "def f(n, acc):\n    if n == 0:\n        return acc\n    return f(n - 1, acc + n)\n";
    let ts = "function g(n: number, acc: number): number { while (n !== 0) { acc = acc + n; n = n - 1; } return acc; }";
    assert_eq!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, ts, Lang::TypeScript),
        "Python tail recursion and typed TypeScript while loop should converge"
    );
}

#[test]
fn structural_recursion_sum_converges_with_loop() {
    // `n + f(n-1)` is a `+`-monoid fold (identity 0) → accumulator loop.
    let i = Interner::new();
    let rec = "def f(n):\n    if n == 0:\n        return 0\n    return n + f(n - 1)\n";
    let loopv = "def g(n):\n    acc = 0\n    while n != 0:\n        acc = acc + n\n        n = n - 1\n    return acc\n";
    assert_eq!(
        value_fp(&i, rec, Lang::Python),
        value_fp(&i, loopv, Lang::Python),
        "structural sum recursion should converge with the accumulator loop"
    );
}

#[test]
fn structural_recursion_consumes_parameter_domain_evidence() {
    let i = Interner::new();
    let rec =
        "def f(n: int, x: int):\n    if n == 0:\n        return 0\n    return x + f(n - 1, x)\n";
    let loopv = "def g(n: int, x: int):\n    acc = 0\n    while n != 0:\n        acc = acc + x\n        n = n - 1\n    return acc\n";
    assert_eq!(
        value_fp(&i, rec, Lang::Python),
        value_fp(&i, loopv, Lang::Python),
        "structural fold law must consume annotation-derived ValueDomain evidence"
    );
}

#[test]
fn structural_recursion_factorial_converges_with_loop() {
    // `n * f(n-1)` is a `*`-monoid fold (identity 1) → accumulator loop.
    let i = Interner::new();
    let rec = "def f(n):\n    if n == 0:\n        return 1\n    return n * f(n - 1)\n";
    let loopv = "def g(n):\n    acc = 1\n    while n != 0:\n        acc = acc * n\n        n = n - 1\n    return acc\n";
    assert_eq!(
        value_fp(&i, rec, Lang::Python),
        value_fp(&i, loopv, Lang::Python),
        "structural factorial recursion should converge with the accumulator loop"
    );
}

#[test]
fn two_structural_recursions_converge() {
    // Independent of any loop: two same-shape recursions must share a fingerprint.
    let i = Interner::new();
    let f = "def f(n):\n    if n == 0:\n        return 0\n    return n + f(n - 1)\n";
    let g = "def h(m):\n    if m == 0:\n        return 0\n    return m + h(m - 1)\n";
    assert_eq!(value_fp(&i, f, Lang::Python), value_fp(&i, g, Lang::Python),);
}

#[test]
fn recursion_does_not_falsely_merge() {
    // The soundness boundary: a different combine op / scale / base case must NOT collapse
    // onto the sum. (Subtraction is not an associative monoid, so it is never rewritten.)
    let i = Interner::new();
    let sum = "def f(n):\n    if n == 0:\n        return 0\n    return n + f(n - 1)\n";
    let product = "def g(n):\n    if n == 0:\n        return 1\n    return n * g(n - 1)\n";
    let scaled = "def g(n):\n    if n == 0:\n        return 0\n    return 2 * n + g(n - 1)\n";
    let subtract = "def g(n):\n    if n == 0:\n        return 0\n    return n - g(n - 1)\n";
    let base5 = "def g(n):\n    if n == 0:\n        return 5\n    return n + g(n - 1)\n";
    let fp = value_fp(&i, sum, Lang::Python);
    for (label, other) in [
        ("product", product),
        ("scaled", scaled),
        ("subtraction", subtract),
        ("non-identity base", base5),
    ] {
        assert_ne!(
            fp,
            value_fp(&i, other, Lang::Python),
            "sum recursion must not merge with {label}"
        );
    }
}

#[test]
fn interp_executes_self_recursion() {
    // The oracle form keeps recursion un-rewritten (it stops before the recursion pass), so
    // this exercises the interpreter's self-call support directly: factorial must evaluate.
    use nose_normalize::{run_unit, Value};
    let i = Interner::new();
    let src = "def fact(n):\n    if n <= 0:\n        return 1\n    return n * fact(n - 1)\n";
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), Lang::Python, &i).unwrap();
    let oracle = normalize(
        &il,
        &i,
        &NormalizeOptions {
            oracle: true,
            ..NormalizeOptions::default()
        },
    );
    let root = first_func(&oracle);
    let beh = run_unit(&oracle, &i, root, &[Value::Int(5)]).expect("recursion should interpret");
    assert_eq!(
        beh.ret,
        Value::Int(120),
        "5! = 120 via interpreted recursion"
    );
}

#[test]
fn loop_accumulator_seed_is_not_abstracted() {
    // A loop-carried accumulator that is not a clean collection reduction (a numeric
    // countdown fold) still depends on its pre-loop SEED. Regression: the compact
    // `Recurrence` value keyed only on the per-iteration update, so a parameter-seeded
    // accumulator (`acc=a` → returns `a + Σ`) collapsed onto a zero-seeded one
    // (`total=0` → returns `Σ`) — a false merge. They must now stay distinct.
    let i = Interner::new();
    let param_seed = "def f(n, acc):\n    while n > 0:\n        acc = acc + n\n        n = n - 1\n    return acc\n";
    let zero_seed = "def g(n):\n    total = 0\n    while n > 0:\n        total = total + n\n        n = n - 1\n    return total\n";
    assert_ne!(
        value_fp(&i, param_seed, Lang::Python),
        value_fp(&i, zero_seed, Lang::Python),
        "a parameter-seeded accumulator must not merge with a zero-seeded one"
    );
    // Same seed (both 0) and same update still converge — the fix only adds the seed to the
    // key, it does not over-split.
    let zero_seed2 = "def h(m):\n    s = 0\n    while m > 0:\n        s = s + m\n        m = m - 1\n    return s\n";
    assert_eq!(
        value_fp(&i, zero_seed, Lang::Python),
        value_fp(&i, zero_seed2, Lang::Python),
        "two zero-seeded countdown sums must still converge"
    );
}

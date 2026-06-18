use super::*;

#[test]
fn sound_recall_rules_converge_with_hard_negatives() {
    // #284 (coevo §CE / S5-C4): behaviorally-equal forms that nose now converges.
    // The abs law is integer-gated; the bitwise laws preserve error behavior on
    // both sides for non-integer inputs.
    let i = Interner::new();

    // abs(abs x) ≡ abs x for integer-proven operands.
    let abs_nested =
        "def f(x: int):\n    a = x if x >= 0 else -x\n    return a if a >= 0 else -a\n";
    let abs_once = "def g(x: int):\n    return x if x >= 0 else -x\n";
    assert_eq!(
        value_fp(&i, abs_nested, Lang::Python),
        value_fp(&i, abs_once, Lang::Python),
        "abs(abs x) must converge with abs x"
    );

    // ~(a&b) ≡ ~a|~b — bitwise De Morgan; a non-integer Errs on both.
    let demorgan_l = "def f(a, b):\n    return ~(a & b)\n";
    let demorgan_r = "def g(a, b):\n    return (~a) | (~b)\n";
    assert_eq!(
        value_fp(&i, demorgan_l, Lang::Python),
        value_fp(&i, demorgan_r, Lang::Python),
        "~(a&b) must converge with ~a|~b"
    );
    // Hard negative: ~(a|b) ≡ ~a&~b, and must NOT collide with the AND form.
    let demorgan_or = "def h(a, b):\n    return ~(a | b)\n";
    assert_ne!(
        value_fp(&i, demorgan_l, Lang::Python),
        value_fp(&i, demorgan_or, Lang::Python),
        "~(a&b) and ~(a|b) are different functions"
    );

    // max(max(a,b),c) ≡ max(a,max(b,c)) — associative on the ternary semantics
    // (total for all inputs, incl. NaN). Hard negative: min vs max stays distinct.
    let max_l = "def f(a, b, c):\n    m = a if a > b else b\n    return m if m > c else c\n";
    let max_r = "def g(a, b, c):\n    n = b if b > c else c\n    return a if a > n else n\n";
    assert_eq!(
        value_fp(&i, max_l, Lang::Python),
        value_fp(&i, max_r, Lang::Python),
        "nested max must flatten and converge"
    );
    let min_l = "def h(a, b, c):\n    m = a if a < b else b\n    return m if m < c else c\n";
    assert_ne!(
        value_fp(&i, max_l, Lang::Python),
        value_fp(&i, min_l, Lang::Python),
        "max and min chains must stay distinct"
    );
}

#[test]
fn floored_mod_distinguishes_python_ruby_from_c_family() {
    // #283-D: Python/Ruby `%` is FLOORED (remainder takes the divisor's sign);
    // C/Go/Java/JS/Rust `%` is TRUNCATED (dividend's sign). They differ on
    // sign-disagreeing operands (`-1 % 3 == 2` vs `== -1`), so a single `Op::Mod`
    // for all languages was a false merge the interpreter was blind to.
    let i = Interner::new();
    let py = "def rem(a, b):\n    return a % b\n";
    let rb = "def rem(a, b)\n  a % b\nend\n";
    let js = "function rem(a, b){ return a % b; }";
    let go = "package p\nfunc Rem(a int, b int) int { return a % b }\n";

    // Floored ≠ truncated: Python must NOT converge with JS.
    assert_ne!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, js, Lang::JavaScript),
        "Python floored % must not merge with JS truncated %"
    );
    // Same semantics still converge: Python ≡ Ruby (both floored).
    assert_eq!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, rb, Lang::Ruby),
        "Python and Ruby % are both floored — must converge"
    );
    // JS ≡ Go (both truncated).
    assert_eq!(
        value_fp(&i, js, Lang::JavaScript),
        value_fp(&i, go, Lang::Go),
        "JS and Go % are both truncated — must converge"
    );
}

#[test]
fn double_negation_cancels_only_for_proven_numeric() {
    // #283-B: `-(-a) → a` is sound ONLY when `a` is a number — on a list `-a` Errs, so
    // `-(-a)` Errs while `a` does not. The value graph used to infer `a: Num` from the very
    // `-` it was about to delete (optimistic), and the algebra pass cancelled `-(-x)`
    // unconditionally. Both are fixed: an UNTYPED param keeps `-(-a)` distinct from `a`; a
    // genuinely-typed (annotated) param still cancels, preserving sound recall.
    let i = Interner::new();
    let negneg_untyped = "def f(a):\n    return -(-a)\n";
    let ident_untyped = "def f(a):\n    return a\n";
    let negneg_typed = "def f(a: int):\n    return -(-a)\n";
    let ident_typed = "def f(a: int):\n    return a\n";

    assert_ne!(
        value_fp(&i, negneg_untyped, Lang::Python),
        value_fp(&i, ident_untyped, Lang::Python),
        "untyped -(-a) must NOT merge with a (it Errs on a list; a does not)"
    );
    assert_eq!(
        value_fp(&i, negneg_typed, Lang::Python),
        value_fp(&i, ident_typed, Lang::Python),
        "int-annotated -(-a) is provably numeric — must still cancel to a"
    );
}

#[test]
fn bitwise_self_idempotence_gates_on_proven_numeric() {
    // #283-B: `a & a → a` / `a | a → a` is sound only for integers (`[1] & [1]` Errs in
    // Python while `[1]` does not). The optimistic value domain inferred `a: Num` from the
    // `&`/`|` itself; now an untyped param stays distinct, an annotated one still folds.
    let i = Interner::new();
    let untyped_and = "def f(a):\n    return a & a\n";
    let untyped_id = "def f(a):\n    return a\n";
    let typed_and = "def f(a: int):\n    return a & a\n";
    let typed_id = "def f(a: int):\n    return a\n";

    assert_ne!(
        value_fp(&i, untyped_and, Lang::Python),
        value_fp(&i, untyped_id, Lang::Python),
        "untyped a & a must NOT merge with a"
    );
    assert_eq!(
        value_fp(&i, typed_and, Lang::Python),
        value_fp(&i, typed_id, Lang::Python),
        "int-annotated a & a is provably numeric — must still fold to a"
    );
}

#[test]
fn untyped_add_commute_gates_on_proven_numeric() {
    // #283-C: `a + b → b + a` (commuting the operands of `+`) is sound only when both are
    // numbers — for strings/lists `+` is ORDERED concat (`"x"+"y" != "y"+"x"`). The detector
    // reordered untyped `+` optimistically. Now the reorder gates on proven-numeric operands:
    // an untyped `a+b` stays distinct from `b+a`, while an int-annotated one still converges.
    let i = Interner::new();
    let fwd_untyped = "def f(a, b):\n    return a + b\n";
    let rev_untyped = "def f(a, b):\n    return b + a\n";
    let fwd_typed = "def f(a: int, b: int):\n    return a + b\n";
    let rev_typed = "def f(a: int, b: int):\n    return b + a\n";

    assert_ne!(
        value_fp(&i, fwd_untyped, Lang::Python),
        value_fp(&i, rev_untyped, Lang::Python),
        "untyped a + b must NOT merge with b + a (string concat is ordered)"
    );
    assert_eq!(
        value_fp(&i, fwd_typed, Lang::Python),
        value_fp(&i, rev_typed, Lang::Python),
        "int-annotated a + b is provably numeric — commuting to b + a must still converge"
    );
}

#[test]
fn js_int32_bitwise_distinguished_from_arbitrary_precision() {
    // #283-D: JS bitwise coerces operands to int32 (`a & b` is `ToInt32(a) & ToInt32(b)`),
    // while Python/Ruby bitwise is arbitrary-precision. They differ for operands outside
    // int32 range (`2^40 & 2^40` is `0` in JS, `2^40` in Python), so one `Bin(BitAnd)` for
    // both was a false merge. JS bitwise leaves now carry a `ToInt32` wrap → distinct
    // fingerprint; within-JS `&` still commutes; the De Morgan canon still fires.
    let i = Interner::new();
    let js = "function f(a, b){ return a & b; }";
    let py = "def f(a, b):\n    return a & b\n";
    let js_swapped = "function g(a, b){ return b & a; }";
    let js_demorgan_a = "function f(a, b){ return ~(a & b); }";
    let js_demorgan_b = "function g(a, b){ return (~a) | (~b); }";

    assert_ne!(
        value_fp(&i, js, Lang::JavaScript),
        value_fp(&i, py, Lang::Python),
        "JS int32 `&` must not merge with Python arbitrary-precision `&`"
    );
    assert_eq!(
        value_fp(&i, js, Lang::JavaScript),
        value_fp(&i, js_swapped, Lang::JavaScript),
        "within JS, `a & b` still commutes with `b & a`"
    );
    assert_eq!(
        value_fp(&i, js_demorgan_a, Lang::JavaScript),
        value_fp(&i, js_demorgan_b, Lang::JavaScript),
        "De Morgan `~(a&b) ≡ ~a|~b` still holds for JS int32 bitwise"
    );
}

#[test]
fn true_div_distinguishes_three_way_division() {
    // #283-D: `/` is three-way — TRUE-float in Python/JS (`7/2 == 3.5`), FLOORED-int in
    // Ruby (`7/2 == 3`, like Python `//`), TRUNCATED-int in C/Go/Java/Rust (`-7/2 == -3`).
    // One `Op::Div` for all was a false merge; Python/JS `/` now lower to `Op::TrueDiv`,
    // Ruby `/` to `Op::FloorDiv`, C-family stays `Op::Div`.
    let i = Interner::new();
    let py = "def f(a, b):\n    return a / b\n";
    let js = "function f(a, b){ return a / b; }";
    let rb = "def f(a, b)\n  a / b\nend\n";
    let c = "int f(int a, int b) { return a / b; }";
    let py_floor = "def f(a, b):\n    return a // b\n";

    // True-float (py/js) ≠ truncated (c) ≠ floored (ruby).
    assert_ne!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, c, Lang::C),
        "Python true-float / must not merge with C truncated /"
    );
    assert_ne!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, rb, Lang::Ruby),
        "Python true-float / must not merge with Ruby floored /"
    );
    assert_ne!(
        value_fp(&i, rb, Lang::Ruby),
        value_fp(&i, c, Lang::C),
        "Ruby floored / must not merge with C truncated /"
    );
    // Same semantics still converge: Python ≡ JS (true-float); Ruby ≡ Python `//` (floored).
    assert_eq!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, js, Lang::JavaScript),
        "Python and JS / are both true-float — must converge"
    );
    assert_eq!(
        value_fp(&i, rb, Lang::Ruby),
        value_fp(&i, py_floor, Lang::Python),
        "Ruby / and Python // are both floored — must converge"
    );
}

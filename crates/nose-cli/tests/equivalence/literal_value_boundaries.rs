use super::*;

#[test]
fn string_literal_plus_does_not_commute_issue_308() {
    // #308: a string literal's value-graph `Const` key must stay inside the `String`
    // class range so `proven_non_concat` classifies it correctly. The old
    // `0x2000_0000.wrapping_add(hash)` carried a high-bit hash OUT of range, where the
    // string read as non-concat and `"p" + "q"` wrongly commuted with `"q" + "p"`
    // (different values "pq" vs "qp"). The masked key keeps strings in range.
    let i = Interner::new();
    let pq = "def f():\n    return \"p\" + \"q\"\n";
    let qp = "def f():\n    return \"q\" + \"p\"\n";
    assert_ne!(
        value_fp(&i, pq, Lang::Python),
        value_fp(&i, qp, Lang::Python),
        "string concatenation is ordered — `\"p\"+\"q\"` must not merge with `\"q\"+\"p\"`",
    );
    // The masked key still discriminates distinct strings (no class collision).
    let pr = "def f():\n    return \"p\" + \"r\"\n";
    assert_ne!(
        value_fp(&i, pq, Lang::Python),
        value_fp(&i, pr, Lang::Python),
        "distinct string literals must stay distinct under the masked key",
    );
    // And numeric `+` still commutes (the fix is string-specific, recall preserved).
    let ab = "def f(a, b):\n    return a + b + 1\n";
    let ba = "def f(a, b):\n    return b + a + 1\n";
    assert_eq!(
        value_fp(&i, ab, Lang::Python),
        value_fp(&i, ba, Lang::Python),
        "numeric `+` still commutes — the string fix must not regress it",
    );
}

#[test]
fn literal_const_kind_is_separate_from_value_coevo_s8() {
    // coevo series 8: the value-graph `Const` carries its KIND explicitly and the FULL
    // value/hash in `bits`, so a literal can never wrap its class boundary or truncate.
    // Three false merges the old packed u32 key produced are gone:
    let i = Interner::new();
    // S1-1 — an int whose old key collided with the boolean-true tag.
    let int_big = "def f(x):\n    return x + 536870914\n";
    let bool_true = "def f(x):\n    return x + True\n";
    assert_ne!(
        value_fp(&i, int_big, Lang::Python),
        value_fp(&i, bool_true, Lang::Python),
        "an int literal must not share a fingerprint with the boolean `True`",
    );
    // S1-2 — two ints differing by exactly 2^32 (old `v as u32` truncation collided).
    let int_a = "def f(x):\n    return x + 4294967301\n";
    let int_b = "def f(x):\n    return x + 5\n";
    assert_ne!(
        value_fp(&i, int_a, Lang::Python),
        value_fp(&i, int_b, Lang::Python),
        "ints differing by 2^32 must not collide (full i64 retained)",
    );
    // S2-1 — two short strings whose hashes collide in the old 28-bit mask.
    let s_geu = "def f():\n    return \"geU\"\n";
    let s_aaha = "def f():\n    return \"aaha\"\n";
    assert_ne!(
        value_fp(&i, s_geu, Lang::Python),
        value_fp(&i, s_aaha, Lang::Python),
        "distinct strings must not collide (full 64-bit hash retained)",
    );
    // Recall preserved: equal literals still converge, numeric `+` still commutes.
    assert_eq!(
        value_fp(&i, int_b, Lang::Python),
        value_fp(&i, "def f(x):\n    return x + 5\n", Lang::Python),
        "equal int literals still converge",
    );
    assert_eq!(
        value_fp(&i, "def f(a, b):\n    return a + b + 1\n", Lang::Python),
        value_fp(&i, "def f(a, b):\n    return b + a + 1\n", Lang::Python),
        "numeric `+` still commutes",
    );
}

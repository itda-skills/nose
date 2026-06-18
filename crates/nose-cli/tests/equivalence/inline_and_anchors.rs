use super::*;

#[test]
fn interprocedural_inline_handles_multi_statement_pure_helper() {
    // A pure helper with LOCAL temporaries (`sub = …; disc = …; return sub - disc`) inlines just
    // like a single-`return` one — the multi-statement body is a pure value computation, so a
    // caller of it converges with the same logic written inline. An effectful (field-writing)
    // helper still does NOT inline (its effect can't be dropped).
    let i = Interner::new();
    let helper = "def base(item):\n    sub = item.price * item.qty\n    disc = sub * 0.1\n    return sub - disc\n\ndef total(item):\n    return base(item) + 5\n";
    let inline = "def total(item):\n    sub = item.price * item.qty\n    disc = sub * 0.1\n    return (sub - disc) + 5\n";
    assert_eq!(
        value_fp_named(&i, helper, Lang::Python, "total"),
        value_fp_named(&i, inline, Lang::Python, "total"),
        "a multi-statement pure helper must inline like a single-return one",
    );
    let eff = "def bump(box, x):\n    box.count = box.count + 1\n    return x * 2\n\ndef use(box, x):\n    return bump(box, x) + 5\n";
    let eff_free = "def use(box, x):\n    return x * 2 + 5\n";
    assert_ne!(
        value_fp_named(&i, eff, Lang::Python, "use"),
        value_fp_named(&i, eff_free, Lang::Python, "use"),
        "an effectful (field-writing) helper must not be inlined",
    );
}

#[test]
fn interprocedural_pure_inline_converges_extract_method() {
    // A function whose body inlines a computation converges with one that calls a PURE extracted
    // helper for it — `f(args)` is β-reduced to the helper's body (interprocedural summary), the
    // extract-method equivalence. An EFFECTFUL helper (a field write the value-only inline would
    // drop) is NOT inlined, so its caller stays distinct from the effect-free version.
    let i = Interner::new();
    let inline = "def price(item):\n    return item.price * item.qty * (1 + 0.1)\n";
    let helper = "def base(item):\n    return item.price * item.qty\n\ndef price(item):\n    return base(item) * (1 + 0.1)\n";
    assert_eq!(
        value_fp_named(&i, inline, Lang::Python, "price"),
        value_fp_named(&i, helper, Lang::Python, "price"),
        "calling a pure extracted helper must converge with the inlined computation"
    );
    let eff_helper = "def bump(box, x):\n    box.count = box.count + 1\n    return x * 2\n\ndef use(box, x):\n    return bump(box, x) + 5\n";
    let eff_free = "def use(box, x):\n    return x * 2 + 5\n";
    assert_ne!(
        value_fp_named(&i, eff_helper, Lang::Python, "use"),
        value_fp_named(&i, eff_free, Lang::Python, "use"),
        "an effectful (field-writing) helper must not be inlined — its effect can't be dropped"
    );
}

#[test]
fn generalized_inline_loop_accumulator_helper_converges() {
    // The flagship interprocedural case: `foo` calling a LOOP-ACCUMULATOR helper must
    // converge with `foo` written with the loop inline. The straight-line-only inline
    // whitelist could never admit `bar`; the generalized admission evaluates the body
    // through the ordinary statement processor (the loop becomes the same `Reduce`)
    // behind the sink fence.
    let i = Interner::new();
    let helper = "function foo(values) {\n    const total = bar(values)\n    return total / values.length\n}\n\nfunction bar(values) {\n    let result = 0\n    for(let i = 0; i < values.length; i++) {\n        result += values[i]\n    }\n    return result\n}\n";
    let inline = "function foo(values) {\n    let total = 0\n    for(let i = 0; i < values.length; i++) {\n        total += values[i]\n    }\n    return total / values.length\n}\n";
    assert_eq!(
        value_fp_named(&i, helper, Lang::JavaScript, "foo"),
        value_fp_named(&i, inline, Lang::JavaScript, "foo"),
        "a pure loop-accumulator helper must inline and converge with the inline form",
    );
}

#[test]
fn generalized_inline_builder_loop_helper_converges_with_comprehension_caller() {
    // A helper whose body is a pure list-BUILDER loop inlines into its caller and
    // converges with a caller using the comprehension form directly — composing the
    // interprocedural axis with the loop↔comprehension Type-4 axis.
    let i = Interner::new();
    let helper = "def squares(xs):\n    out = []\n    for x in xs:\n        out.append(x * x)\n    return out\n\ndef use(xs):\n    return len(squares(xs))\n";
    let comp = "def use(xs):\n    return len([x * x for x in xs])\n";
    assert_eq!(
        value_fp_named(&i, helper, Lang::Python, "use"),
        value_fp_named(&i, comp, Lang::Python, "use"),
        "a pure builder-loop helper must inline and converge with the comprehension",
    );
}

#[test]
fn generalized_inline_guard_clause_helper_converges_with_ternary() {
    // A guard-clause helper (`if c: return a` then an unconditional return) folds its
    // captured returns to the same `Phi` a ternary builds, so the caller converges with
    // the ternary form written inline.
    let i = Interner::new();
    let helper = "def clamp0(x):\n    if x < 0:\n        return 0\n    return x\n\ndef use(x):\n    return clamp0(x) * 2\n";
    let ternary = "def use(x):\n    return (0 if x < 0 else x) * 2\n";
    assert_eq!(
        value_fp_named(&i, helper, Lang::Python, "use"),
        value_fp_named(&i, ternary, Lang::Python, "use"),
        "a guard-clause helper must fold to the ternary's Phi and converge",
    );
}

#[test]
fn generalized_inline_exhaustive_if_else_tail_converges() {
    // A body ending in an exhaustive `if/else` (both arms return) is admitted: the two
    // captured guarded returns are complementary and fold to one `Phi`.
    let i = Interner::new();
    let helper = "def pick(a, b, flag):\n    if flag:\n        return a\n    else:\n        return b\n\ndef use(a, b, flag):\n    return pick(a, b, flag) + 1\n";
    let ternary = "def use(a, b, flag):\n    return (a if flag else b) + 1\n";
    assert_eq!(
        value_fp_named(&i, helper, Lang::Python, "use"),
        value_fp_named(&i, ternary, Lang::Python, "use"),
        "an exhaustive if/else tail must fold to the ternary's Phi and converge",
    );
}

#[test]
fn generalized_inline_nested_pure_helpers_converge() {
    // Pure helpers calling pure helpers inline transitively (bounded by the inline
    // stack), so a two-hop composition converges with the flat form.
    let i = Interner::new();
    let nested = "def double(x):\n    return x * 2\n\ndef double_sum(xs):\n    t = 0\n    for x in xs:\n        t += double(x)\n    return t\n\ndef use(xs):\n    return double_sum(xs) + 1\n";
    let flat = "def use(xs):\n    t = 0\n    for x in xs:\n        t += x * 2\n    return t + 1\n";
    assert_eq!(
        value_fp_named(&i, nested, Lang::Python, "use"),
        value_fp_named(&i, flat, Lang::Python, "use"),
        "nested pure helpers must inline transitively",
    );
}

#[test]
fn generalized_inline_congruence_callers_of_equal_helpers_converge() {
    // Two callers of two DIFFERENT-NAMED but body-identical pure loop helpers converge:
    // inlining keys the call by the callee's semantics, never its name.
    let i = Interner::new();
    let src = "def sum_a(xs):\n    t = 0\n    for x in xs:\n        t += x\n    return t\n\ndef sum_b(ys):\n    t = 0\n    for y in ys:\n        t += y\n    return t\n\ndef use_a(xs):\n    return sum_a(xs) / len(xs)\n\ndef use_b(xs):\n    return sum_b(xs) / len(xs)\n";
    assert_eq!(
        value_fp_named(&i, src, Lang::Python, "use_a"),
        value_fp_named(&i, src, Lang::Python, "use_b"),
        "callers of behaviorally-equal helpers must converge regardless of helper name",
    );
}

#[test]
fn generalized_inline_rejects_effectful_loop_helper() {
    // A loop helper that ALSO appends to a caller-supplied list is NOT pure — inlining
    // it value-only would drop the append. The sink fence rejects it, so the caller must
    // NOT converge with the pure loop written inline (which is exactly the fingerprint a
    // broken fence would produce).
    let i = Interner::new();
    let eff = "def log_sum(xs, log):\n    t = 0\n    for x in xs:\n        log.append(x)\n        t += x\n    return t\n\ndef use(xs, log):\n    return log_sum(xs, log) / len(xs)\n";
    let pure_inline =
        "def use(xs, log):\n    t = 0\n    for x in xs:\n        t += x\n    return t / len(xs)\n";
    assert_ne!(
        value_fp_named(&i, eff, Lang::Python, "use"),
        value_fp_named(&i, pure_inline, Lang::Python, "use"),
        "an effectful loop helper must not inline — dropping its append is a false merge",
    );
}

#[test]
fn generalized_inline_rejects_recursive_helpers_without_hanging() {
    // Self- and mutual recursion fail closed (the inline stack excludes re-entry) and
    // must neither hang nor panic; the recursive caller stays distinct from a
    // non-recursive one.
    let i = Interner::new();
    let self_rec = "def fact(n):\n    if n <= 1:\n        return 1\n    return fact(n - 1) * n\n\ndef use(n):\n    return fact(n) + 1\n";
    let mutual = "def even(n):\n    if n == 0:\n        return True\n    return odd(n - 1)\n\ndef odd(n):\n    if n == 0:\n        return False\n    return even(n - 1)\n\ndef use(n):\n    return even(n)\n";
    let trivial = "def use(n):\n    return n + 1\n";
    assert_ne!(
        value_fp_named(&i, self_rec, Lang::Python, "use"),
        value_fp_named(&i, trivial, Lang::Python, "use"),
        "a self-recursive helper caller stays distinct (fail-closed, no hang)",
    );
    assert_ne!(
        value_fp_named(&i, mutual, Lang::Python, "use"),
        value_fp_named(&i, trivial, Lang::Python, "use"),
        "a mutually-recursive helper caller stays distinct (fail-closed, no hang)",
    );
}

#[test]
fn sub_dag_anchor_shared_when_units_share_a_heavy_computation() {
    // Two functions that share a large sub-computation (subtotal/tax/shipping/grand) but differ
    // elsewhere are a PARTIAL / sub-DAG clone — they share a heavy anchor (an extractable common
    // computation) even though whole-unit fingerprints differ. If the shared computation itself
    // diverges (different shipping rule / sign), no anchor is shared.
    let i = Interner::new();
    let a = "function a(items) {\n  const subtotal = items.map(x => x.price * x.qty).reduce((s, x) => s + x, 0);\n  const tax = subtotal * rate;\n  const ship = subtotal > 100 ? 0 : 15;\n  const grand = subtotal + tax + ship;\n  renderInvoice(grand);\n  return grand;\n}\n";
    let b = "function b(items) {\n  const subtotal = items.map(x => x.price * x.qty).reduce((s, x) => s + x, 0);\n  const tax = subtotal * rate;\n  const ship = subtotal > 100 ? 0 : 15;\n  const grand = subtotal + tax + ship;\n  saveOrder(grand);\n  notify(grand);\n}\n";
    let c = "function c(items) {\n  const subtotal = items.map(x => x.price * x.qty).reduce((s, x) => s + x, 0);\n  const tax = subtotal * rate;\n  const ship = subtotal > 200 ? 0 : 25;\n  const grand = subtotal - tax + ship;\n  saveOrder(grand);\n  notify(grand);\n}\n";
    let aa = value_anchors(&i, a, Lang::TypeScript);
    assert!(
        !aa.is_empty(),
        "a heavy shared computation must yield an anchor"
    );
    assert!(
        shares_any(&aa, &value_anchors(&i, b, Lang::TypeScript)),
        "units sharing a heavy computation must share an anchor (partial clone)"
    );
    assert!(
        !shares_any(&aa, &value_anchors(&i, c, Lang::TypeScript)),
        "when the shared computation diverges, no anchor is shared"
    );
}

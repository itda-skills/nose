use super::*;

#[test]
fn value_graph_ignores_statement_order() {
    // x and y are each used twice → NOT inlined; only the value graph (not the
    // AST) makes the two statement orders converge.
    let i = Interner::new();
    let a = "def f(a, b):\n    x = a + 1\n    y = b + 1\n    return x * y + x\n";
    let b = "def g(a, b):\n    y = b + 1\n    x = a + 1\n    return x * y + x\n";
    assert_eq!(value_fp(&i, a, Lang::Python), value_fp(&i, b, Lang::Python));
}

#[test]
fn value_graph_cse_temp_vs_repeated() {
    let i = Interner::new();
    let temp = "def f(a, b):\n    t = a + b\n    return t + t\n";
    let repeated = "def g(a, b):\n    return (a + b) + (a + b)\n";
    assert_eq!(
        value_fp(&i, temp, Lang::Python),
        value_fp(&i, repeated, Lang::Python)
    );
}

#[test]
fn value_graph_distinguishes_different_ops() {
    let i = Interner::new();
    let add = "def f(a, b):\n    return a + b\n";
    let sub = "def g(a, b):\n    return a - b\n";
    assert_ne!(
        value_fp(&i, add, Lang::Python),
        value_fp(&i, sub, Lang::Python)
    );
}

#[test]
fn value_graph_distinguishes_range_start_offset() {
    // `range(len(a))` sums every element; `range(1, len(a))` skips a[0]. They are
    // behaviorally DIFFERENT, so their value fingerprints must differ. Abstracting
    // `a[i]` → `Elem(a)` for a *partial* range (dropping the start bound) is a false
    // merge — a genuine soundness bug.
    let i = Interner::new();
    let full =
        "def f(a):\n    s = 0\n    for i in range(len(a)):\n        s += a[i]\n    return s\n";
    let skip =
        "def g(a):\n    s = 0\n    for i in range(1, len(a)):\n        s += a[i]\n    return s\n";
    assert_ne!(
        value_fp(&i, full, Lang::Python),
        value_fp(&i, skip, Lang::Python),
        "a partial range (skipping a[0]) must not fingerprint identically to the full range"
    );
}

#[test]
fn value_graph_distinguishes_constants() {
    // Behavior-defining numeric constants must stay distinct in the value graph
    // (the §AT axis-split): `x%7` ≢ `x%11`, `return 100` ≢ `return 200`. Large ints
    // were abstracted to one `Int` class — a latent false merge.
    let i = Interner::new();
    let m7 = "def f(x):\n    return x % 7\n";
    let m11 = "def f(x):\n    return x % 11\n";
    assert_ne!(
        value_fp(&i, m7, Lang::Python),
        value_fp(&i, m11, Lang::Python),
        "x%7 and x%11 are behaviorally different"
    );
    let a = "def f(x):\n    return x + 100\n";
    let b = "def f(x):\n    return x + 200\n";
    assert_ne!(
        value_fp(&i, a, Lang::Python),
        value_fp(&i, b, Lang::Python),
        "x+100 and x+200 are behaviorally different"
    );
}

#[test]
fn value_graph_distinguishes_for_in_vs_of() {
    // JS `for (x of it)` iterates VALUES, `for (x in it)` iterates KEYS — different.
    let i = Interner::new();
    let of = "function f(a){ let s = 0; for (const x of a) { s += x; } return s; }";
    let in_ = "function f(a){ let s = 0; for (const x in a) { s += x; } return s; }";
    assert_ne!(
        value_fp(&i, of, Lang::JavaScript),
        value_fp(&i, in_, Lang::JavaScript),
        "for-of (values) must differ from for-in (keys)"
    );
}

#[test]
fn value_graph_distinguishes_conditional_early_return() {
    // A conditional early `return;` (void) changes which later code runs — it must not
    // be invisible. Two loops differing only in an early-exit guard must differ.
    let i = Interner::new();
    let early = "def f(xs, x):\n    for v in xs:\n        if x > 0:\n            return\n        g(v)\n    h()\n";
    let always = "def f(xs, x):\n    for v in xs:\n        return\n        g(v)\n    h()\n";
    assert_ne!(
        value_fp(&i, early, Lang::Python),
        value_fp(&i, always, Lang::Python),
        "a conditional early return must differ from an unconditional one"
    );
}

#[test]
fn value_graph_distinguishes_throw_from_expression_effect() {
    let i = Interner::new();
    let thrown = "function f() { throw \"x\"; }";
    let expr = "function f() { \"x\"; }";
    assert_ne!(
        value_fp(&i, thrown, Lang::JavaScript),
        value_fp(&i, expr, Lang::JavaScript),
        "throw is terminal error behavior, not a plain expression effect"
    );
}

#[test]
fn value_graph_reads_field_written_in_unit() {
    let i = Interner::new();
    let read_field = "def f(self):\n    self.x = 7\n    return self.x\n";
    let return_value = "def f(self):\n    self.x = 7\n    return 7\n";
    let java_read_field = "class C { int x; int f() { this.x = 7; return this.x; } }";
    let java_return_value = "class C { int x; int f() { this.x = 7; return 7; } }";
    let read_other_receiver = "def f(a, b):\n    a.x = 7\n    return b.x\n";
    let read_written_receiver = "def f(a, b):\n    a.x = 7\n    return a.x\n";
    let unknown_alias_receiver = "def f(a):\n    r = receiver(a)\n    r.x = 7\n    return a.x\n";
    let computed_receiver = "def f(a):\n    receiver(a).x = 7\n    return receiver(a).x\n";
    assert_ne!(
        value_fp(&i, read_field, Lang::Python),
        value_fp(&i, return_value, Lang::Python),
        "raw Python attribute spelling is not enough proof for exact field-state readback"
    );
    assert_eq!(
        value_fp(&i, java_read_field, Lang::Java),
        value_fp(&i, java_return_value, Lang::Java),
        "a Java this.field read after an effect-proven same-unit write should resolve to the written value"
    );
    assert_ne!(
        value_fp(&i, read_other_receiver, Lang::Python),
        value_fp(&i, read_written_receiver, Lang::Python),
        "a same-named field write on one receiver must not satisfy a read on another receiver"
    );
    assert_ne!(
        value_fp(&i, unknown_alias_receiver, Lang::Python),
        value_fp(&i, return_value, Lang::Python),
        "field-state readback must not assume call-result aliasing without receiver-place proof"
    );
    assert_ne!(
        value_fp(&i, computed_receiver, Lang::Python),
        value_fp(&i, return_value, Lang::Python),
        "computed field receivers must not enter same-unit field-state caching"
    );
}

#[test]
fn value_graph_field_state_is_receiver_aware() {
    let i = Interner::new();
    let same_receiver_order_a = "def f(self):\n    self.x = 1\n    self.y = 2\n";
    let same_receiver_order_b = "def f(self):\n    self.y = 2\n    self.x = 1\n";
    let java_same_receiver_order_a =
        "class C { int x; int y; void f() { this.x = 1; this.y = 2; } }";
    let java_same_receiver_order_b =
        "class C { int x; int y; void f() { this.y = 2; this.x = 1; } }";
    let crossed_receivers_a = "def f(a, b):\n    a.x = 1\n    b.x = 2\n";
    let crossed_receivers_b = "def f(a, b):\n    b.x = 1\n    a.x = 2\n";
    assert_ne!(
        value_fp(&i, same_receiver_order_a, Lang::Python),
        value_fp(&i, same_receiver_order_b, Lang::Python),
        "raw Python attribute writes stay ordered because property/setter effects are not proven"
    );
    assert_eq!(
        value_fp(&i, java_same_receiver_order_a, Lang::Java),
        value_fp(&i, java_same_receiver_order_b, Lang::Java),
        "effect-proven Java final writes to distinct this.fields should commute"
    );
    assert_ne!(
        value_fp(&i, crossed_receivers_a, Lang::Python),
        value_fp(&i, crossed_receivers_b, Lang::Python),
        "same-named field writes on different receivers must preserve receiver identity"
    );
}

#[test]
fn value_graph_distinguishes_membership_and_negation() {
    // `in` is directional membership, not equality: `a in b` ≠ `b in a` ≠ `a == b`.
    // And `not in` / `is not` must keep their negation (`x is not None` ≢ `x is None`).
    let i = Interner::new();
    let inb = "def f(a, b):\n    return a in b\n";
    let bin = "def f(a, b):\n    return b in a\n";
    let eqb = "def f(a, b):\n    return a == b\n";
    assert_ne!(
        value_fp(&i, inb, Lang::Python),
        value_fp(&i, bin, Lang::Python),
        "a in b must differ from b in a (membership is directional)"
    );
    assert_ne!(
        value_fp(&i, inb, Lang::Python),
        value_fp(&i, eqb, Lang::Python),
        "a in b must differ from a == b"
    );
    let isn = "def f(a):\n    return a is None\n";
    let isnot = "def f(a):\n    return a is not None\n";
    assert_ne!(
        value_fp(&i, isn, Lang::Python),
        value_fp(&i, isnot, Lang::Python),
        "a is None must differ from a is not None (negation)"
    );
    let notin = "def f(a, b):\n    return a not in b\n";
    assert_ne!(
        value_fp(&i, inb, Lang::Python),
        value_fp(&i, notin, Lang::Python),
        "a in b must differ from a not in b (negation)"
    );
}

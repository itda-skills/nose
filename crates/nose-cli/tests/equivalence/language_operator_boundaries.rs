use super::*;

#[test]
fn ruby_star_repetition_is_ordered_but_other_multiply_commutes() {
    // series 9: `*` is string/array REPETITION in Ruby and asymmetric — `"ab" * 3` →
    // "ababab" but `3 * "ab"` raises (`Integer#*` rejects a String). Reordering its
    // operands (the algebra pass folded a constant to the end; the value graph sorted
    // by hash) false-merged the two. Only Ruby is gated: Python repetition commutes and
    // JS/Java/Go/C `*` is numeric.
    let i = Interner::new();
    let rb_str_first = "def a\n  \"ab\" * 3\nend\n";
    let rb_int_first = "def b\n  3 * \"ab\"\nend\n";
    assert_ne!(
        value_fp(&i, rb_str_first, Lang::Ruby),
        value_fp(&i, rb_int_first, Lang::Ruby),
        "Ruby `\"ab\" * 3` (repeats) must not merge with `3 * \"ab\"` (raises)"
    );
    let rb_arr_first = "def a\n  [1, 2] * 3\nend\n";
    let rb_arr_int_first = "def b\n  3 * [1, 2]\nend\n";
    assert_ne!(
        value_fp(&i, rb_arr_first, Lang::Ruby),
        value_fp(&i, rb_arr_int_first, Lang::Ruby),
        "Ruby `[1,2] * 3` (repeats) must not merge with `3 * [1,2]` (raises)"
    );
    // Largest-sound-generalization guard: only Ruby is gated.
    let js_xy = "function p(x, y) { return x * y; }";
    let js_yx = "function q(x, y) { return y * x; }";
    assert_eq!(
        value_fp(&i, js_xy, Lang::JavaScript),
        value_fp(&i, js_yx, Lang::JavaScript),
        "JS `x * y` is numeric and must still commute with `y * x`"
    );
    let py_sx = "def p(s):\n    return s * 3\n";
    let py_xs = "def q(s):\n    return 3 * s\n";
    assert_eq!(
        value_fp(&i, py_sx, Lang::Python),
        value_fp(&i, py_xs, Lang::Python),
        "Python `s * 3` repetition commutes (`3 * s` is equal) and must still converge"
    );
}

#[test]
fn js_nullish_assignment_desugars_to_nullish_coalescing() {
    // `x ??= y` is `x = x ?? y` — and is NOT `x += y` (the old unmapped-operator
    // fallback silently defaulted compound assignments to Add).
    let i = Interner::new();
    let compound = "function f(x, y) {\n  x ??= y;\n  return x;\n}";
    let spelled = "function g(x, y) {\n  x = x ?? y;\n  return x;\n}";
    let add = "function h(x, y) {\n  x += y;\n  return x;\n}";
    assert_eq!(
        value_fp(&i, compound, Lang::JavaScript),
        value_fp(&i, spelled, Lang::JavaScript),
        "`x ??= y` should converge with `x = x ?? y`"
    );
    assert_ne!(
        value_fp(&i, compound, Lang::JavaScript),
        value_fp(&i, add, Lang::JavaScript),
        "`x ??= y` must not merge with `x += y`"
    );
}

#[test]
fn dataflow_does_not_unsoundly_inline_a_temp_past_a_write_or_into_a_lambda() {
    // series 9 oracle residue: the copy-propagation inliner must not move a temp's
    // (possibly-raising) read into a position evaluated under a different condition.
    // Two cases, both verified to keep the temp's `Var` binding after normalization:
    //   - `t = a[i]; a[i] = a[j]; a[j] = t` — inlining `t` past the indexed write that
    //     clobbers `a[i]` would silently turn a swap into "set both to a[j]".
    //   - `ind = nodes[k]; [x for x in d if nodes[x] == ind]` — inlining `ind` into the
    //     filter lambda elides its `Err` when `d` is empty (the lambda never runs).
    use nose_il::NodeKind;
    let i = Interner::new();
    let binds_a_var_temp = |il: &nose_il::Il| -> bool {
        let mut stack = vec![first_func(il)];
        while let Some(n) = stack.pop() {
            if il.kind(n) == NodeKind::Assign {
                if let Some(&lhs) = il.children(n).first() {
                    if il.kind(lhs) == NodeKind::Var {
                        return true;
                    }
                }
            }
            stack.extend(il.children(n).iter().copied());
        }
        false
    };
    let normalized = |src: &str, lang: Lang| {
        let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, &i).unwrap();
        normalize(&il, &i, &NormalizeOptions::default())
    };
    let swap = "def swap(a, i, j):\n    t = a[i]\n    a[i] = a[j]\n    a[j] = t\n";
    assert!(
        binds_a_var_temp(&normalized(swap, Lang::Python)),
        "swap's `t = a[i]` must survive — inlining it past `a[i] = a[j]` is unsound",
    );
    let comp =
        "def f(d, nodes, k):\n    ind = nodes[k]\n    return [x for x in d if nodes[x] == ind]\n";
    assert!(
        binds_a_var_temp(&normalized(comp, Lang::Python)),
        "comprehension's `ind = nodes[k]` must not inline into the filter lambda",
    );
}

#[test]
fn js_strict_null_ternary_stays_distinct_from_nullish_coalescing() {
    // `x ?? d` and `x == null ? d : x` both default null AND undefined — they are
    // the same computation. `x === null ? d : x` passes undefined through, so it
    // must NOT join that family (it differs on every undefined input).
    let i = Interner::new();
    let nullish = "function f(x, d) {\n  return x ?? d;\n}";
    let loose = "function g(x, d) {\n  return x == null ? d : x;\n}";
    let strict = "function h(x, d) {\n  return x === null ? d : x;\n}";
    assert_eq!(
        value_fp(&i, nullish, Lang::JavaScript),
        value_fp(&i, loose, Lang::JavaScript),
        "`??` should converge with the loose-equality ternary"
    );
    assert_ne!(
        value_fp(&i, nullish, Lang::JavaScript),
        value_fp(&i, strict, Lang::JavaScript),
        "`??` must not merge with the strict-null ternary"
    );
    // Strict checks still converge with the same strict spelling…
    let strict2 = "function k(v, fb) {\n  return v === null ? fb : v;\n}";
    assert_eq!(
        value_fp(&i, strict, Lang::JavaScript),
        value_fp(&i, strict2, Lang::JavaScript),
        "alpha-renamed strict-null ternaries must still converge"
    );
    // …but `=== null` and `=== undefined` are different checks.
    let strict_undef = "function m(x, d) {\n  return x === undefined ? d : x;\n}";
    assert_ne!(
        value_fp(&i, strict, Lang::JavaScript),
        value_fp(&i, strict_undef, Lang::JavaScript),
        "`=== null` and `=== undefined` must not share a fingerprint"
    );
}

#[test]
fn java_unsigned_shift_assignment_keeps_its_operator() {
    // Java `x >>>= y` used to fall through the unmapped-compound path and lower
    // as a plain `x = y` — merging it with reassignment.
    let i = Interner::new();
    let ushift = "class C { static int f(int x, int y) { x >>>= y; return x; } }";
    let assign = "class D { static int g(int x, int y) { x = y; return x; } }";
    let add = "class E { static int h(int x, int y) { x += y; return x; } }";
    assert_ne!(
        value_fp(&i, ushift, Lang::Java),
        value_fp(&i, assign, Lang::Java),
        "`x >>>= y` must not merge with `x = y`"
    );
    assert_ne!(
        value_fp(&i, ushift, Lang::Java),
        value_fp(&i, add, Lang::Java),
        "`x >>>= y` must not merge with `x += y`"
    );
}

#[test]
fn ruby_exponent_converges_with_python_pow() {
    // Ruby `**` was unmapped (raw); it is the same exponentiation Python spells `**`.
    let i = Interner::new();
    let rb = "def area(base, exp)\n  base ** exp\nend\n";
    let py = "def area(base, exp):\n    return base ** exp\n";
    assert_eq!(
        value_fp(&i, rb, Lang::Ruby),
        value_fp(&i, py, Lang::Python),
        "Ruby `**` should converge with Python `**`"
    );
}

#[test]
fn two_argument_min_max_interpret_as_two_way_selection() {
    // `min(a, b)` (the 2-way selection `[a, b].min()` also canonicalizes to) used to
    // evaluate to Err in the oracle — leaving exactly the convergences the value
    // graph claims for it unverifiable.
    let i = Interner::new();
    let il = nose_frontend::lower_source(
        FileId(0),
        "t",
        b"def f(a, b):\n    return min(a, b), max(a, b)\n",
        Lang::Python,
        &i,
    )
    .unwrap();
    let n = normalize(&il, &i, &NormalizeOptions::default());
    let f = first_func(&n);
    use nose_normalize::{run_unit, Value};
    let out = run_unit(&n, &i, f, &[Value::Int(3), Value::Int(1)])
        .expect("two-scalar min/max is interpretable")
        .ret;
    assert_eq!(
        out,
        Value::List(vec![Value::Int(1), Value::Int(3)]),
        "min(3, 1) is 1 and max(3, 1) is 3"
    );
}

/// Ruby `for x in xs … out << e` is the same list build as a Python comprehension:
/// `for..in` is a language construct (no receiver proof needed, unlike `each`), and
/// the shovel is admitted as an append ONLY through the active-builder seed proof
/// (`out = []`). The shovel operator alone proves nothing — an integer-seeded `<<`
/// stays a shift, and a parameter receiver (no seed) never becomes a builder.
#[test]
fn ruby_for_in_shovel_builder_converges_with_comprehension() {
    let i = Interner::new();
    let comp = value_fp(
        &i,
        "def f(xs):\n    return [x * x for x in xs]\n",
        Lang::Python,
    );
    let ruby_for = value_fp(
        &i,
        "def f(xs)\n  out = []\n  for x in xs\n    out << x * x\n  end\n  out\nend\n",
        Lang::Ruby,
    );
    assert_eq!(comp, ruby_for, "ruby for-in shovel builder ≡ comprehension");

    // Adjacent hard negative: a different per-element contribution stays distinct.
    let ruby_diff = value_fp(
        &i,
        "def f(xs)\n  out = []\n  for x in xs\n    out << x + 1\n  end\n  out\nend\n",
        Lang::Ruby,
    );
    assert_ne!(
        ruby_for, ruby_diff,
        "different contribution must stay distinct"
    );

    // Hard negative: an integer-seeded `<<` is a SHIFT — must not become a builder
    // (and must stay distinct from a doubling accumulator, which it behaviorally is not
    // for non-trivial seeds; here the point is it must not merge with the list build).
    let ruby_shift = value_fp(
        &i,
        "def f(xs)\n  acc = 1\n  for x in xs\n    acc = acc << 1\n  end\n  acc\nend\n",
        Lang::Ruby,
    );
    assert_ne!(
        ruby_for, ruby_shift,
        "integer shovel is a shift, not an append"
    );

    // Hard negative: a parameter receiver has no empty-list seed proof, so its
    // shovel never builds — the loop keeps its opaque per-element effect.
    let ruby_param = value_fp(
        &i,
        "def f(xs, out)\n  for x in xs\n    out << x * x\n  end\n  out\nend\n",
        Lang::Ruby,
    );
    assert_ne!(
        comp, ruby_param,
        "shovel to an unproven (parameter) receiver must stay closed"
    );
}

/// The bare Ruby `for x in xs` loop converges with Python's: tree-sitter-ruby wraps
/// the iterable in an `in` node, which must lower to the iterable itself, not an
/// exact-unsafe `Raw("in")`.
#[test]
fn ruby_for_in_loop_converges_with_python_for() {
    let i = Interner::new();
    let rb = value_fp(
        &i,
        "def f(xs)\n  for x in xs\n    y = x\n  end\n  0\nend\n",
        Lang::Ruby,
    );
    let py = value_fp(
        &i,
        "def f(xs):\n    for x in xs:\n        y = x\n    return 0\n",
        Lang::Python,
    );
    assert_eq!(rb, py, "ruby for-in ≡ python for (no Raw iterable wrapper)");
}

/// #244 — bounded symbolic-condition path exploration. Branching on an opaque
/// call's symbolic result no longer bails the unit under `run_unit_paths`: both
/// arms run, each path's trace records its assumption as a Sym marker (so the
/// behaviors stay symbolic → advisory lane), and the strict `run_unit` contract
/// is unchanged (still bails).
#[test]
fn symbolic_condition_paths_explore_both_arms() {
    use nose_normalize::{run_unit, Value};
    let i = Interner::new();
    let src = "def f(x):\n    if g(x):\n        return 1\n    return 2\n";
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

    // Strict contract unchanged: a symbolic condition bails run_unit.
    assert!(
        run_unit(&oracle, &i, root, &[Value::Int(3)]).is_none(),
        "strict run_unit must still bail on a symbolic condition"
    );

    let mut cap = false;
    let paths = nose_normalize::run_unit_paths(&oracle, &i, root, &[Value::Int(3)], &mut cap)
        .expect("two-arm exploration interprets the unit");
    assert!(!cap, "one site is within the exploration cap");
    assert_eq!(paths.len(), 2, "one symbolic site forks exactly two paths");
    assert_eq!(
        paths[0].ret,
        Value::Int(1),
        "true arm first (deterministic)"
    );
    assert_eq!(paths[1].ret, Value::Int(2), "false arm second");
    for p in &paths {
        assert!(
            nose_normalize::behavior_has_sym(p),
            "every explored path carries its Sym assumption marker: {p:?}"
        );
    }
    assert_ne!(
        paths[0].effects, paths[1].effects,
        "the two arms record different assumptions"
    );

    // Differential alignment: the SAME shape over the same opaque call agrees…
    let twin = nose_normalize::run_unit_paths(&oracle, &i, root, &[Value::Int(3)], &mut cap)
        .expect("twin run");
    assert_eq!(paths, twin, "deterministic across runs");
    // …while branching on a DIFFERENT opaque call yields different assumptions.
    let src_other = "def f(x):\n    if h(x):\n        return 1\n    return 2\n";
    let il2 = nose_frontend::lower_source(FileId(0), "t", src_other.as_bytes(), Lang::Python, &i)
        .unwrap();
    let oracle2 = normalize(
        &il2,
        &i,
        &NormalizeOptions {
            oracle: true,
            ..NormalizeOptions::default()
        },
    );
    let other = nose_normalize::run_unit_paths(
        &oracle2,
        &i,
        first_func(&oracle2),
        &[Value::Int(3)],
        &mut cap,
    )
    .expect("other unit");
    assert_ne!(
        paths, other,
        "a different opaque condition must not align (different assumption markers)"
    );
}

/// #244 fail-closed: more symbolic decision sites than the cap → path-bail,
/// reported via the out-flag, never guessed.
#[test]
fn symbolic_condition_paths_fail_closed_past_the_cap() {
    use nose_normalize::Value;
    let i = Interner::new();
    // 4 sequential symbolic decisions > MAX_SYM_BRANCH_SITES (3).
    let src = "def f(x):\n    a = 1 if g(x) else 2\n    b = 1 if h(x) else 2\n    c = 1 if p(x) else 2\n    d = 1 if q(x) else 2\n    return a + b + c + d\n";
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
    let mut cap = false;
    let out = nose_normalize::run_unit_paths(&oracle, &i, root, &[Value::Int(3)], &mut cap);
    assert!(out.is_none(), "past the site cap the unit fails closed");
    assert!(
        cap,
        "the bail is reported as a path-cap bail for the census"
    );
}

#[test]
fn effectful_commutative_operands_do_not_reorder() {
    // coevo §CE / #283-A: `print(a) + print(b)` commutes by VALUE but the
    // interpreter observes effect order, so reordering it to `print(b) + print(a)`
    // is a false merge. Effect-bearing operands must hold their position; only
    // effect-free numeric operands reorder.
    let i = Interner::new();
    let fwd = "def f(a, b):\n    return print(a) + print(b)\n";
    let rev = "def g(a, b):\n    return print(b) + print(a)\n";
    assert_ne!(
        value_fp(&i, fwd, Lang::Python),
        value_fp(&i, rev, Lang::Python),
        "effectful commutative operands must not reorder into one fingerprint"
    );
    let chain_fwd = "def f(a, b, c):\n    return print(a) + print(b) + print(c)\n";
    let chain_rev = "def g(a, b, c):\n    return print(c) + print(b) + print(a)\n";
    assert_ne!(
        value_fp(&i, chain_fwd, Lang::Python),
        value_fp(&i, chain_rev, Lang::Python),
        "effectful AC chains must not sort into one fingerprint"
    );
    // Effect-FREE operands still COMMUTE within the same grouping (`a+b+1` ≡ `b+a+1`) — float
    // `+` is commutative, only its associativity is held (#342). (A REGROUPING like `1+b+a`
    // does NOT converge now: `(a+b)+1` vs `(1+b)+a` differ for floats.)
    let pure_fwd = "def f(a, b):\n    return a + b + 1\n";
    let pure_rev = "def g(a, b):\n    return b + a + 1\n";
    assert_eq!(
        value_fp(&i, pure_fwd, Lang::Python),
        value_fp(&i, pure_rev, Lang::Python),
        "effect-free commutative operands (same grouping) must still converge"
    );
}

use super::*;

#[test]
fn pointer_length_contract_is_exposed() {
    // A C function `f(int *xs, int n)` whose loop bound is `n` records the pointer-length
    // contract (array_pos=0, length_pos=1) so the behavioral oracle interprets it under
    // `n = len(xs)` — the same convention the value graph used to drop `n` and merge it with
    // the `len`-based form. A function that does NOT use a length param records none.
    let i = Interner::new();
    let c = "int sum_small(int *xs, int n) {\n int t=0;\n for (int i=0;i<n;i++){ if (xs[i]<3){ t+=xs[i]; } }\n return t;\n}\n";
    let lowered = nose_frontend::lower_source(FileId(0), "a.c", c.as_bytes(), Lang::C, &i).unwrap();
    let n = normalize(&lowered, &i, &NormalizeOptions::default());
    let contracts = nose_normalize::value_fingerprint_contracts(&n, n.units[0].root, &i);
    assert_eq!(
        contracts,
        vec![(0, 1)],
        "C (xs, n) must record contract (0,1)"
    );

    // The aligned two-array form `f(a, b, n)` shares `n` as the length of both.
    let dot = "int dot(int *a, int *b, int n) {\n int t=0;\n for (int i=0;i<n;i++){ t+=a[i]*b[i]; }\n return t;\n}\n";
    let ld = nose_frontend::lower_source(FileId(0), "d.c", dot.as_bytes(), Lang::C, &i).unwrap();
    let nd = normalize(&ld, &i, &NormalizeOptions::default());
    let dc = nose_normalize::value_fingerprint_contracts(&nd, nd.units[0].root, &i);
    assert!(
        dc.contains(&(0, 2)) || dc.contains(&(1, 2)),
        "aligned (a, b, n) must record a shared length contract at pos 2, got {dc:?}"
    );

    // A `len`-based form (no length param) records no contract.
    let py = "def sum_small(xs):\n    t=0\n    for x in xs:\n        if x<3:\n            t+=x\n    return t\n";
    let lp =
        nose_frontend::lower_source(FileId(0), "a.py", py.as_bytes(), Lang::Python, &i).unwrap();
    let np = normalize(&lp, &i, &NormalizeOptions::default());
    assert!(
        nose_normalize::value_fingerprint_contracts(&np, np.units[0].root, &i).is_empty(),
        "a len-based form uses no pointer-length contract"
    );
}

#[test]
fn lattice_strict_comparison_converges_and_separates() {
    // SOUND lattice canon on a total order: `(x ≤ y) ∧ (x ≠ y) ≡ x < y` and the dual
    // `(x < y) ∨ (x = y) ≡ x ≤ y`. Declaring the one `∧` rule composes through the
    // recursive `mk` fixpoint (De Morgan + comparison-direction canon) to also close
    // `not (a > b or a == b) ≡ a < b` for integer-proven operands.
    let i = Interner::new();
    let lt = value_fp(&i, "def f(a,b):\n    return a<b\n", Lang::Python);
    assert_eq!(
        lt,
        value_fp(&i, "def g(a,b):\n    return a<=b and a!=b\n", Lang::Python),
        "(a<=b) and (a!=b) must converge with a<b"
    );
    assert_eq!(
        lt,
        value_fp(&i, "def g(a,b):\n    return a!=b and a<=b\n", Lang::Python),
        "operand order of the conjunction must not matter"
    );
    let lt_int = value_fp(&i, "def f(a: int,b: int):\n    return a<b\n", Lang::Python);
    assert_eq!(
        lt_int,
        value_fp(
            &i,
            "def g(a: int,b: int):\n    return not (a>b or a==b)\n",
            Lang::Python
        ),
        "De Morgan + comparison-direction must compose into the lattice canon"
    );
    assert_ne!(
        lt,
        value_fp(
            &i,
            "def g(a,b):\n    return not (a>b or a==b)\n",
            Lang::Python
        ),
        "untyped De Morgan plus order negation must keep the NaN boundary closed"
    );
    // Cross-language: a typed TS strict-less written as the conjunction.
    assert_eq!(
        lt,
        value_fp(
            &i,
            "function g(a: number, b: number): boolean { return a <= b && a !== b; }",
            Lang::TypeScript
        ),
        "the lattice canon is language-agnostic"
    );
    let le = value_fp(&i, "def f(a,b):\n    return a<=b\n", Lang::Python);
    assert_eq!(
        le,
        value_fp(&i, "def g(a,b):\n    return a<b or a==b\n", Lang::Python),
        "(a<b) or (a==b) must converge with a<=b"
    );

    // HARD NEGATIVES — the rule must not over-fire (these are different computations):
    assert_ne!(
        lt,
        value_fp(&i, "def g(a,b):\n    return a<=b\n", Lang::Python),
        "a<b must NOT merge with a<=b"
    );
    assert_ne!(
        lt,
        value_fp(
            &i,
            "def g(a,b,c):\n    return a<=b and a!=c\n",
            Lang::Python
        ),
        "the inequality must be over the SAME operands, not a third variable"
    );
    assert_ne!(
        lt,
        value_fp(&i, "def g(a,b):\n    return a<=b or a!=b\n", Lang::Python),
        "the connective matters: (a<=b) OR (a!=b) is not a<b"
    );
}

#[test]
fn swift_total_order_absorption_is_integer_domain_gated() {
    let i = Interner::new();
    let strict = r#"
func f(_ x: Int, _ y: Int) -> Bool {
    return x < y
}
"#;
    let nonstrict_and_ne = r#"
func f(_ x: Int, _ y: Int) -> Bool {
    return x <= y && x != y
}
"#;
    let unparenthesized = r#"
func f(_ x: Int, _ y: Int) -> Bool {
    return x < y && x <= y
}
"#;
    let parenthesized = r#"
func f(_ x: Int, _ y: Int) -> Bool {
    return (x < y) && (x <= y)
}
"#;
    let wrong_connective = r#"
func f(_ x: Int, _ y: Int) -> Bool {
    return x < y || x <= y
}
"#;
    let string_and = r#"
func f(_ x: String, _ y: String) -> Bool {
    return x < y && x <= y
}
"#;
    let string_strict = r#"
func f(_ x: String, _ y: String) -> Bool {
    return x < y
}
"#;

    let fp = value_fp(&i, strict, Lang::Swift);
    assert_eq!(
        fp,
        value_fp(&i, nonstrict_and_ne, Lang::Swift),
        "Swift Int total-order lattice facts should bridge <= plus !="
    );
    assert_eq!(
        fp,
        value_fp(&i, unparenthesized, Lang::Swift),
        "Swift Int strict/non-strict conjunction should absorb to the strict comparison"
    );
    assert_eq!(
        fp,
        value_fp(&i, parenthesized, Lang::Swift),
        "parentheses should not affect the same total-order absorption"
    );
    assert_ne!(
        fp,
        value_fp(&i, wrong_connective, Lang::Swift),
        "OR broadens the predicate and must stay distinct"
    );
    assert_ne!(
        value_fp(&i, string_and, Lang::Swift),
        value_fp(&i, string_strict, Lang::Swift),
        "Swift overloaded/String comparisons stay closed without integer-domain proof"
    );
}

#[test]
fn detection_smoke_groups_clones_excludes_decoy() {
    // Two clones (a sum loop in Python and TS) plus an unrelated decoy.
    let interner = Interner::new();
    let py = "def sum_list(items):\n    total = 0\n    i = 0\n    while i < len(items):\n        total += items[i]\n        i = i + 1\n    return total\n";
    let ts = "function total(xs: number[]): number {\n  let acc = 0;\n  for (const x of xs) {\n    acc += x;\n  }\n  return acc;\n}\n";
    let decoy = "def greet(name):\n    msg = 'hello ' + name\n    print(msg)\n    print(name)\n    return msg\n";

    let files = vec![
        nose_frontend::lower_source(FileId(0), "a.py", py.as_bytes(), Lang::Python, &interner)
            .unwrap(),
        nose_frontend::lower_source(
            FileId(1),
            "b.ts",
            ts.as_bytes(),
            Lang::TypeScript,
            &interner,
        )
        .unwrap(),
        nose_frontend::lower_source(FileId(2), "c.py", decoy.as_bytes(), Lang::Python, &interner)
            .unwrap(),
    ];
    let corpus = Corpus::new(interner, files);

    let opts = DetectOptions {
        min_lines: 2,
        min_tokens: 12,
        ..Default::default()
    };
    let detector = StructuralDetector::strict(opts.jaccard_weight);
    let report = detect(&corpus, &opts, &detector);

    // Multi-granularity units may cluster the clone at both function and block
    // level, so assert by content rather than group count: the two sum files
    // appear together, the decoy never does, and a cross-language pair is found.
    assert!(
        !report.groups.is_empty(),
        "expected at least one clone group"
    );
    let files_in_groups: std::collections::HashSet<&str> = report
        .groups
        .iter()
        .flat_map(|g| g.members.iter().map(|m| m.file.as_str()))
        .collect();
    assert!(
        files_in_groups.contains("a.py"),
        "py clone should be grouped"
    );
    assert!(
        files_in_groups.contains("b.ts"),
        "ts clone should be grouped"
    );
    assert!(
        !files_in_groups.contains("c.py"),
        "decoy must not be grouped"
    );
    assert!(
        report.duplicates.iter().any(|d| d.cross_language),
        "cross-language pair expected"
    );
}

/// Normalization must be **idempotent** — a canonicalizing pipeline should reach a
/// fixpoint, so re-normalizing already-canonical IL changes nothing. A pass that
/// fails this is a smell (it hasn't converged) and would make detection sensitive
/// to how many times IL was processed. We compare the whole-file root hash.
#[test]
fn normalization_is_idempotent() {
    let i = Interner::new();
    let samples = [
        ("def f(items):\n    t = 0\n    for x in items:\n        if x > 0:\n            t = t + x * 2\n    return t\n", Lang::Python),
        ("function g(a,b){ let r = a ? b : a + 1; while(a < b){ a = a + 1; } return r; }", Lang::JavaScript),
        ("fn h(xs: &[i32]) -> i32 { let mut s = 0; for x in xs { s += *x; } s }", Lang::Rust),
        ("def k(a,b,c):\n    t = (a + b) + c\n    if not (a and b):\n        return t\n    return c\n", Lang::Python),
    ];
    for (src, lang) in samples {
        let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, &i).unwrap();
        let once = normalize(&il, &i, &NormalizeOptions::default());
        let twice = normalize(&once, &i, &NormalizeOptions::default());
        let h1 = subtree_hashes(&once, &i)[once.root.0 as usize];
        let h2 = subtree_hashes(&twice, &i)[twice.root.0 as usize];
        assert_eq!(h1, h2, "normalize not idempotent for {lang:?}: {src}");
    }
}

/// DISTRIBUTION / FACTORING (domain-gated): `a*c + b*c` ≡ `(a+b)*c`. The value graph factors
/// a shared multiplicand out of a sum of products when every leaf is proven numeric
/// (`value_graph/rules/factor_distribute.rs`, Lean obligation
/// `normalize.value_graph.factor_distribute`). The `*`
/// operands here are statically typed integers, so the factoring fires and the two forms converge.
#[test]
fn distribution_factors_common_multiplicand() {
    let i = Interner::new();
    assert_eq!(
        value_fp(
            &i,
            "fn f(a: i64, b: i64, c: i64) -> i64 { a*c+b*c }\n",
            Lang::Rust
        ),
        value_fp(
            &i,
            "fn g(a: i64, b: i64, c: i64) -> i64 { (a+b)*c }\n",
            Lang::Rust
        ),
        "a*c+b*c should factor to (a+b)*c on proven-numeric leaves"
    );
    // Three-term chain factors transitively: `a*c + b*c + d*c` ≡ `(a+b+d)*c`.
    assert_eq!(
        value_fp(
            &i,
            "fn f(a: i64, b: i64, c: i64, d: i64) -> i64 { a*c+b*c+d*c }\n",
            Lang::Rust
        ),
        value_fp(
            &i,
            "fn g(a: i64, b: i64, c: i64, d: i64) -> i64 { (a+b+d)*c }\n",
            Lang::Rust
        ),
        "a*c+b*c+d*c should factor to (a+b+d)*c"
    );
}

#[test]
fn value_law_domains_keep_string_concat_order_but_numeric_add_commutes() {
    let i = Interner::new();
    let numeric_a_b = "class C { static int f(int a, int b) { return a + b; } }";
    let numeric_b_a = "class C { static int g(int a, int b) { return b + a; } }";
    assert_eq!(
        value_fp(&i, numeric_a_b, Lang::Java),
        value_fp(&i, numeric_b_a, Lang::Java),
        "integer-domain add should still commute under the value-law contract"
    );

    let string_a_b = "class C { static String f(String a, String b) { return a + b; } }";
    let string_b_a = "class C { static String g(String a, String b) { return b + a; } }";
    assert_ne!(
        value_fp(&i, string_a_b, Lang::Java),
        value_fp(&i, string_b_a, Lang::Java),
        "string-domain concat is ordered and must not inherit numeric add laws"
    );
}

#[test]
fn value_law_domains_keep_python_string_repetition_ordered() {
    let i = Interner::new();
    let left_then_right = "def f(s: str, t: str):\n    return s * 2 + t * 2\n";
    let right_then_left = "def g(s: str, t: str):\n    return t * 2 + s * 2\n";
    assert_ne!(
        value_fp(&i, left_then_right, Lang::Python),
        value_fp(&i, right_then_left, Lang::Python),
        "typed Python str repetition feeds ordered concat, not numeric add sorting"
    );
}

#[test]
fn class_value_law_domains_use_method_parameter_evidence() {
    let i = Interner::new();
    let string_a_b = "class C { static String f(String a, String b) { return a + b; } }";
    let string_b_a = "class C { static String f(String a, String b) { return b + a; } }";
    assert_ne!(
        class_value_fp(&i, string_a_b, Lang::Java, "C"),
        class_value_fp(&i, string_b_a, Lang::Java, "C"),
        "class-unit fingerprints must preserve method string-concat order"
    );
}

/// FILTER FUSION: `filter(q, filter(p, xs))` ≡ `filter(p∧q, xs)`. The value graph carries a
/// filter's element so nested filters fuse (`value_graph.rs` `HoFKind::Filter` arm, Lean
/// `normalize.value_graph.functor`). A two-filter comprehension and an explicitly nested one
/// converge; TS `number[]` callbacks stay closed until element-domain proof exists.
#[test]
fn filter_fusion_converges() {
    let i = Interner::new();
    assert_eq!(
        value_fp(
            &i,
            "def f(xs):\n    return [x for x in xs if x>0 if x<10]\n",
            Lang::Python
        ),
        value_fp(
            &i,
            "def g(xs):\n    return [x for x in [y for y in xs if y>0] if x<10]\n",
            Lang::Python
        ),
        "two stacked filters should fuse with an explicitly nested filter"
    );
    assert_ne!(
        value_fp(
            &i,
            "def f(xs):\n    return [x for x in xs if x>0 if x<10]\n",
            Lang::Python
        ),
        value_fp(
            &i,
            "function g(xs: number[]): number[] { return xs.filter(x=>x>0).filter(x=>x<10); }",
            Lang::TypeScript
        ),
        "TypeScript number[] callback elements are not yet numeric proof for relational predicates"
    );
}

/// DICT-BUILDER: `{k: v for x in xs}` ≡ `d={}; for x in xs: d[k]=v`. The dict-building loop
/// is recognized as a builder of `DictEntry`s, the same node the comprehension produces.
#[test]
fn dict_comprehension_converges_with_building_loop() {
    let i = Interner::new();
    assert_eq!(
        value_fp(
            &i,
            "def f(xs):\n    return {x: x*x for x in xs}\n",
            Lang::Python
        ),
        value_fp(
            &i,
            "def g(xs):\n    d={}\n    for x in xs:\n        d[x]=x*x\n    return d\n",
            Lang::Python
        ),
        "dict comprehension should converge with the dict-building loop"
    );
}

/// SOUNDNESS GUARD for the dict-builder: a dict comprehension must stay DISTINCT from a list
/// of tuples — `{k: v for x in xs}` and `[(k, v) for x in xs]` build different values, so a
/// `DictEntry` must not collide with a tuple `Seq`. (Dicts are not oracle-modeled, so this
/// representational distinctness is what prevents the false merge.)
#[test]
fn dict_comprehension_distinct_from_tuple_list() {
    let i = Interner::new();
    assert_ne!(
        value_fp(
            &i,
            "def f(xs):\n    return {x: x*x for x in xs}\n",
            Lang::Python
        ),
        value_fp(
            &i,
            "def g(xs):\n    return [(x, x*x) for x in xs]\n",
            Lang::Python
        ),
        "a dict comprehension must NOT merge with a list of tuples (different behavior)"
    );
}

/// REDUCE-LAMBDA SELECTION: `reduce(λa,b. a if a>b else b, xs)` ≡ `max(xs)` (and the `<`
/// form ≡ `min`). The explicit fold's selection lambda is recognized as a min/max selection
/// reduction (which carries no accumulator seed), so it converges with the builtin.
#[test]
fn reduce_lambda_selection_converges() {
    let i = Interner::new();
    assert_eq!(
        value_fp(
            &i,
            "import functools\n\ndef f(xs):\n    return functools.reduce(lambda a,b: a if a>b else b, xs)\n",
            Lang::Python
        ),
        value_fp(&i, "def g(xs):\n    return max(xs)\n", Lang::Python),
        "reduce(λ. a if a>b else b) should converge with max()"
    );
    assert_eq!(
        value_fp(
            &i,
            "import functools\n\ndef f(xs):\n    return functools.reduce(lambda a,b: a if a<b else b, xs)\n",
            Lang::Python
        ),
        value_fp(&i, "def g(xs):\n    return min(xs)\n", Lang::Python),
        "reduce(λ. a if a<b else b) should converge with min()"
    );
}

/// COUNT of a filtered comprehension equals the sum of 1s: `len([x for x in xs if p])` ≡
/// `sum(1 for x in xs if p)` ≡ (cross-language) a Rust `xs.iter().filter(p).count()`.
#[test]
fn len_of_filtered_comprehension_is_count() {
    let i = Interner::new();
    let count_loop = value_fp(
        &i,
        "def g(xs):\n    return sum(1 for x in xs if x>0)\n",
        Lang::Python,
    );
    assert_eq!(
        value_fp(
            &i,
            "def f(xs):\n    return len([x for x in xs if x>0])\n",
            Lang::Python
        ),
        count_loop,
        "len of a filtered comprehension should equal sum(1 …)"
    );
    assert_eq!(
        value_fp(
            &i,
            "fn h(xs:&[i64])->usize{ xs.iter().filter(|x| **x>0).count() }",
            Lang::Rust
        ),
        count_loop,
        "Rust .filter(p).count() should converge with Python sum(1 for x if p)"
    );
}

/// Cross-language METHOD-FORM iterator reductions: a Rust `xs.iter().filter(p).sum()`
/// converges with the Python generator `sum(x for x in xs if p)` (method-form `.sum()`
/// canonicalizes to the same `Builtin::Sum` over the filtered stream).
#[test]
fn rust_iterator_reductions_converge() {
    let i = Interner::new();
    assert_eq!(
        value_fp(
            &i,
            "def f(xs):\n    return sum(x for x in xs if x>0)\n",
            Lang::Python
        ),
        value_fp(
            &i,
            "fn g(xs:&[i64])->i64{ xs.iter().filter(|x| **x>0).sum() }",
            Lang::Rust
        ),
        "Python filtered sum generator should converge with Rust .iter().filter().sum()"
    );
}

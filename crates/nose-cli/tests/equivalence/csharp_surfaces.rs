//! C# lowers into the shared IL like the other C-family frontends, so equivalent
//! imperative code is found as a clone *across* C# and Java (and desugars the same
//! way within C#). These are the convergence tests that keep the C# lowering honest.

use super::*;

#[test]
fn csharp_arithmetic_converges_with_java_exact() {
    // With per-parameter `int` domain evidence on both sides, `a + b` is proven
    // numeric and the two accumulators share a value-graph fingerprint (exact).
    let i = Interner::new();
    let cs = "public class A { public int Add(int a, int b) { int t = a; t += b; return t; } }";
    let java = "class A { int Add(int a, int b) { int t = a; t += b; return t; } }";
    assert_eq!(
        value_fp(&i, cs, Lang::CSharp),
        value_fp(&i, java, Lang::Java),
        "C# and Java int accumulators must share a value-graph fingerprint",
    );
}

#[test]
fn csharp_foreach_sum_converges_with_java_enhanced_for() {
    let i = Interner::new();
    let cs = "public class S { public int Sum(int[] xs) { int t = 0; foreach (var x in xs) { t += x; } return t; } }";
    let java =
        "class S { int Sum(int[] xs) { int t = 0; for (int x : xs) { t += x; } return t; } }";
    assert_eq!(
        unit_hash(&i, cs, Lang::CSharp),
        unit_hash(&i, java, Lang::Java),
        "C# foreach-sum and Java enhanced-for-sum must converge structurally",
    );
}

#[test]
fn csharp_compound_assignment_desugars() {
    let i = Interner::new();
    let a = "public class C { public int F(int n) { int t = n; t += 1; return t; } }";
    let b = "public class C { public int F(int n) { int t = n; t = t + 1; return t; } }";
    assert_eq!(
        unit_hash(&i, a, Lang::CSharp),
        unit_hash(&i, b, Lang::CSharp),
        "`t += 1` desugars to `t = t + 1`",
    );
}

#[test]
fn csharp_expression_bodied_converges_with_block_body() {
    let i = Interner::new();
    let expr = "public class C { public int G(int a, int b) => a + b; }";
    let block = "public class C { public int G(int a, int b) { return a + b; } }";
    assert_eq!(
        value_fp(&i, expr, Lang::CSharp),
        value_fp(&i, block, Lang::CSharp),
        "expression-bodied => a + b converges with a block-bodied return a + b",
    );
}

#[test]
fn csharp_switch_statement_converges_with_if_chain() {
    let i = Interner::new();
    let sw = "public class C { public int F(int x) { switch (x) { case 1: return 10; default: return 0; } } }";
    let if_chain =
        "public class C { public int F(int x) { if (x == 1) { return 10; } else { return 0; } } }";
    assert_eq!(
        unit_hash(&i, sw, Lang::CSharp),
        unit_hash(&i, if_chain, Lang::CSharp),
        "a constant `switch` lowers to the equivalent if/else chain",
    );
}

#[test]
fn csharp_different_operator_does_not_merge() {
    let i = Interner::new();
    let add = "public class C { public int F(int a, int b) { return a + b; } }";
    let sub = "public class C { public int F(int a, int b) { return a - b; } }";
    assert_ne!(
        value_fp(&i, add, Lang::CSharp),
        value_fp(&i, sub, Lang::CSharp),
        "a + b must not merge with a - b (soundness control)",
    );
}

#[test]
fn csharp_preproc_wrapped_method_converges_with_plain() {
    // tree-sitter nests `#if`-guarded members inside the `preproc_if` node; the
    // method must still register as a unit and hash like its unguarded twin.
    let i = Interner::new();
    let guarded = "public class C {\n#if FEATURE_X\n    public int F(int a, int b) { return a + b; } \n#endif\n}";
    let plain = "public class C { public int F(int a, int b) { return a + b; } }";
    assert_eq!(
        unit_hash(&i, guarded, Lang::CSharp),
        unit_hash(&i, plain, Lang::CSharp),
        "an `#if`-wrapped method must lower identically to the unguarded one",
    );
}

#[test]
fn csharp_null_coalescing_converges_with_swift() {
    // Both lower `a ?? b` to the ValueOrDefault builtin call.
    let i = Interner::new();
    let cs = "public class C { public int F(int? a, int b) { return a ?? b; } }";
    let swift = "class C { func F(a: Int?, b: Int) -> Int { return a ?? b } }";
    assert_eq!(
        unit_hash(&i, cs, Lang::CSharp),
        unit_hash(&i, swift, Lang::Swift),
        "C# `??` and Swift `??` must share the ValueOrDefault shape",
    );
}

#[test]
fn csharp_coalescing_assignment_desugars() {
    let i = Interner::new();
    let compound = "public class C { public string F(string x, string y) { x ??= y; return x; } }";
    let explicit =
        "public class C { public string F(string x, string y) { x = x ?? y; return x; } }";
    assert_eq!(
        unit_hash(&i, compound, Lang::CSharp),
        unit_hash(&i, explicit, Lang::CSharp),
        "`x ??= y` desugars to `x = x ?? y`",
    );
}

#[test]
fn csharp_relational_switch_pattern_converges_with_if_chain() {
    let i = Interner::new();
    let sw = "public class C { public int F(int x) { switch (x) { case > 5: return 1; default: return 0; } } }";
    let if_chain =
        "public class C { public int F(int x) { if (x > 5) { return 1; } else { return 0; } } }";
    assert_eq!(
        unit_hash(&i, sw, Lang::CSharp),
        unit_hash(&i, if_chain, Lang::CSharp),
        "a relational `case > 5:` lowers to the equivalent if/else chain",
    );
}

#[test]
fn csharp_switch_expression_pattern_converges_with_ternary() {
    let i = Interner::new();
    let sw = "public class C { public int F(int x) { return x switch { > 5 => 1, _ => 0 }; } }";
    let ternary = "public class C { public int F(int x) { return x > 5 ? 1 : 0; } }";
    assert_eq!(
        unit_hash(&i, sw, Lang::CSharp),
        unit_hash(&i, ternary, Lang::CSharp),
        "a relational switch-expression arm lowers to the equivalent ternary",
    );
}

#[test]
fn csharp_is_not_null_converges_with_explicit_null_check() {
    let i = Interner::new();
    let pat =
        "public class C { public int F(object o) { if (o is not null) { return 1; } return 0; } }";
    let expl =
        "public class C { public int F(object o) { if (!(o == null)) { return 1; } return 0; } }";
    assert_eq!(
        unit_hash(&i, pat, Lang::CSharp),
        unit_hash(&i, expl, Lang::CSharp),
        "`o is not null` lowers to `!(o == null)`",
    );
}

#[test]
fn csharp_declaration_pattern_converges_with_java_instanceof() {
    // Both erase the type test to the value under test.
    let i = Interner::new();
    let cs =
        "public class C { public int F(object o) { if (o is string s) { return 1; } return 0; } }";
    let java = "class C { int F(Object o) { if (o instanceof String s) { return 1; } return 0; } }";
    assert_eq!(
        unit_hash(&i, cs, Lang::CSharp),
        unit_hash(&i, java, Lang::Java),
        "C# `is` type test and Java `instanceof` must converge",
    );
}

#[test]
fn csharp_collection_expression_converges_with_array_initializer() {
    let i = Interner::new();
    let collection = "public class C { public int F() { int[] a = [1, 2]; return a[0]; } }";
    let array = "public class C { public int F() { int[] a = new int[] {1, 2}; return a[0]; } }";
    assert_eq!(
        unit_hash(&i, collection, Lang::CSharp),
        unit_hash(&i, array, Lang::CSharp),
        "the C#12 collection expression `[1, 2]` converges with `new int[] {{1, 2}}`",
    );
}

#[test]
fn csharp_conditional_access_converges_with_plain_access() {
    // The null check is type-erased (like `instanceof`): `s?.Trim()` ≡ `s.Trim()`.
    let i = Interner::new();
    let conditional = "public class C { public string F(string s) { return s?.Trim(); } }";
    let plain = "public class C { public string F(string s) { return s.Trim(); } }";
    assert_eq!(
        unit_hash(&i, conditional, Lang::CSharp),
        unit_hash(&i, plain, Lang::CSharp),
        "`s?.Trim()` converges with `s.Trim()`",
    );
}

#[test]
fn csharp_typeof_converges_with_java_class_literal() {
    let i = Interner::new();
    let cs = "public class C { public object F() { return typeof(String); } }";
    let java = "class C { Object F() { return String.class; } }";
    assert_eq!(
        unit_hash(&i, cs, Lang::CSharp),
        unit_hash(&i, java, Lang::Java),
        "`typeof(String)` and `String.class` must share the Field(class) shape",
    );
}

#[test]
fn csharp_different_patterns_do_not_merge() {
    let i = Interner::new();
    let gt = "public class C { public int F(int x) { switch (x) { case > 5: return 1; default: return 0; } } }";
    let lt = "public class C { public int F(int x) { switch (x) { case < 5: return 1; default: return 0; } } }";
    assert_ne!(
        unit_hash(&i, gt, Lang::CSharp),
        unit_hash(&i, lt, Lang::CSharp),
        "`case > 5:` must not merge with `case < 5:` (soundness control)",
    );
}

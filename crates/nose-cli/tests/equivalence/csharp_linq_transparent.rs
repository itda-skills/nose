//! The LINQ transparent-identifier translation (`let`/`join`/a second
//! `from`/`into`) and the anonymous-object shape it synthesizes. Each positive
//! asserts the query spelling converges with the spec's method-syntax chain;
//! the hard negatives keep the anonymous-object shape and the desugar sound.

use super::*;

#[test]
fn csharp_linq_let_converges_with_transparent_identifier_chain() {
    // `let` threads the range variable and the new binding through an
    // anonymous pair, per the spec: `from x in xs let y = x * 2 where y > 3
    // select y + x` is `xs.Select(x => new { x, y = x * 2 })
    // .Where(t => t.y > 3).Select(t => t.y + t.x)`.
    let i = Interner::new();
    let query = "public class C { public object F(int[] xs) { return from x in xs let y = x * 2 where y > 3 select y + x; } }";
    let method = "public class C { public object F(int[] xs) { return xs.Select(x => new { x, y = x * 2 }).Where(t => t.y > 3).Select(t => t.y + t.x); } }";
    assert_eq!(
        unit_hash(&i, query, Lang::CSharp),
        unit_hash(&i, method, Lang::CSharp),
        "a `let` query must converge with its transparent-identifier chain",
    );
}

#[test]
fn csharp_linq_join_converges_with_method_syntax() {
    // A `join` immediately followed by the final `select` translates through
    // `Join`'s own result selector — no transparent identifier.
    let i = Interner::new();
    let query = "public class C { public object F(int[] xs, int[] ys) { return from a in xs join b in ys on a % 3 equals b % 3 select a + b; } }";
    let method = "public class C { public object F(int[] xs, int[] ys) { return xs.Join(ys, a => a % 3, b => b % 3, (a, b) => a + b); } }";
    assert_eq!(
        unit_hash(&i, query, Lang::CSharp),
        unit_hash(&i, method, Lang::CSharp),
        "`join … select` must converge with the Join call",
    );
}

#[test]
fn csharp_linq_second_from_converges_with_select_many() {
    let i = Interner::new();
    let query = "public class C { public object F(int[][] xss) { return from xs in xss from x in xs select x + 1; } }";
    let method = "public class C { public object F(int[][] xss) { return xss.SelectMany(xs => xs, (xs, x) => x + 1); } }";
    assert_eq!(
        unit_hash(&i, query, Lang::CSharp),
        unit_hash(&i, method, Lang::CSharp),
        "a second `from` with a final select must converge with SelectMany",
    );
}

#[test]
fn csharp_linq_group_into_continuation_elides_translation_select() {
    // `into g select g` is the continuation the translation itself introduces;
    // its degenerate select elides, so the query is just the GroupBy.
    let i = Interner::new();
    let query = "public class C { public object F(int[] xs) { return from x in xs group x by x % 2 into g select g; } }";
    let method = "public class C { public object F(int[] xs) { return xs.GroupBy(x => x % 2); } }";
    assert_eq!(
        unit_hash(&i, query, Lang::CSharp),
        unit_hash(&i, method, Lang::CSharp),
        "`group … into g select g` must elide the continuation's identity select",
    );
}

#[test]
fn csharp_linq_where_select_chain_converges_alpha_renamed() {
    // The LINQ adapters carry the sequence-HOF demand profile (deferred,
    // pull-per-element), so chained `Where`/`Select` model as Filter/Map and
    // lambda parameter names cannot split the fingerprint.
    let i = Interner::new();
    let a = "public class C { public object F(int[] xs) { return xs.Where(a => a > 3).Select(b => b * 2); } }";
    let b = "public class C { public object F(int[] ys) { return ys.Where(p => p > 3).Select(q => q * 2); } }";
    assert_eq!(
        unit_hash(&i, a, Lang::CSharp),
        unit_hash(&i, b, Lang::CSharp),
        "alpha-renamed Where/Select chains must converge",
    );
}

#[test]
fn csharp_linq_different_let_expressions_do_not_merge() {
    let i = Interner::new();
    let mul = "public class C { public object F(int[] xs) { return from x in xs let y = x * 2 where y > 3 select y + x; } }";
    let add = "public class C { public object F(int[] xs) { return from x in xs let y = x + 2 where y > 3 select y + x; } }";
    assert_ne!(
        unit_hash(&i, mul, Lang::CSharp),
        unit_hash(&i, add, Lang::CSharp),
        "different `let` bindings must not merge (soundness control)",
    );
}

#[test]
fn csharp_linq_join_does_not_merge_with_second_from() {
    // A join keeps only key-equal pairs; a cross `from` enumerates every pair.
    let i = Interner::new();
    let join = "public class C { public object F(int[] xs, int[] ys) { return from a in xs join b in ys on a % 3 equals b % 3 select a + b; } }";
    let cross = "public class C { public object F(int[] xs, int[] ys) { return from a in xs from b in ys select a + b; } }";
    assert_ne!(
        unit_hash(&i, join, Lang::CSharp),
        unit_hash(&i, cross, Lang::CSharp),
        "`join` must not merge with a cross `from` (soundness control)",
    );
}

#[test]
fn csharp_anonymous_object_converges_with_ts_object_literal() {
    // Both lower to the shared `object`/`pair` shape — a record literal is a
    // record literal across languages. (TypeScript, so both sides carry the
    // numeric parameter domains the operand ordering canon keys on.)
    let i = Interner::new();
    let cs =
        "public class C { public object F(int p, int q) { return new { a = p * 3, b = q + 5 }; } }";
    let ts = "function f(p: number, q: number) { return { a: p * 3, b: q + 5 }; }";
    assert_eq!(
        unit_hash(&i, cs, Lang::CSharp),
        unit_hash(&i, ts, Lang::TypeScript),
        "a C# anonymous object must converge with the TS object literal",
    );
}

#[test]
fn csharp_anonymous_object_does_not_merge_with_tuple() {
    let i = Interner::new();
    let object =
        "public class C { public object F(int p, int q) { return new { a = p * 3, b = q + 5 }; } }";
    let tuple = "public class C { public object F(int p, int q) { return (p * 3, q + 5); } }";
    assert_ne!(
        unit_hash(&i, object, Lang::CSharp),
        unit_hash(&i, tuple, Lang::CSharp),
        "an anonymous object exposes named members a tuple does not (soundness control)",
    );
}

#[test]
fn csharp_anonymous_object_member_order_does_not_merge() {
    // Member order is observable (`ToString`, anonymous-type identity).
    let i = Interner::new();
    let ab =
        "public class C { public object F(int p, int q) { return new { a = p * 3, b = q + 5 }; } }";
    let ba =
        "public class C { public object F(int p, int q) { return new { b = q + 5, a = p * 3 }; } }";
    assert_ne!(
        unit_hash(&i, ab, Lang::CSharp),
        unit_hash(&i, ba, Lang::CSharp),
        "anonymous-object member order must not merge (soundness control)",
    );
}

#[test]
fn csharp_anonymous_object_shorthand_converges_with_named_member() {
    // `new { x }` is `new { x = x }` — the shorthand infers the same member.
    let i = Interner::new();
    let shorthand =
        "public class C { public object F(int x, int q) { return new { x, b = q + 5 }; } }";
    let named =
        "public class C { public object F(int x, int q) { return new { x = x, b = q + 5 }; } }";
    assert_eq!(
        unit_hash(&i, shorthand, Lang::CSharp),
        unit_hash(&i, named, Lang::CSharp),
        "a shorthand member must converge with its explicit spelling",
    );
}

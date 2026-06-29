use super::*;

#[test]
fn rust_recursion_converges_with_iteration_via_return_unwrap() {
    // Numeric structural recursion `fac(n) = n*fac(n-1)` (base 1) converges with its
    // accumulator loop — now in Rust too. Rust lowers `return e;` wrapped in `ExprStmt`, which
    // used to hide the bare-`Return` shape `recursion::recognize` matches on; desugar now
    // unwraps `ExprStmt(Return|Throw)`, so the recursion→iteration canon fires uniformly and
    // converges cross-language with the Python loop. The sum monoid stays a hard negative.
    let i = Interner::new();
    let py_loop = "def fac(n):\n    acc = 1\n    while n != 0:\n        acc = acc * n\n        n = n - 1\n    return acc\n";
    let rust_rec = "pub fn fac(n: i64) -> i64 { if n == 0 { return 1; } return n * fac(n - 1); }";
    let rust_loop = "pub fn fac(mut n: i64) -> i64 { let mut acc = 1; while n != 0 { acc = acc * n; n = n - 1; } return acc; }";
    let sum_loop = "def g(n):\n    acc = 0\n    while n != 0:\n        acc = acc + n\n        n = n - 1\n    return acc\n";
    let fold_fp = value_fp(&i, py_loop, Lang::Python);
    assert_eq!(
        fold_fp,
        value_fp(&i, rust_rec, Lang::Rust),
        "rust recursion must converge cross-language with the python accumulator loop"
    );
    assert_eq!(
        value_fp(&i, rust_rec, Lang::Rust),
        value_fp(&i, rust_loop, Lang::Rust),
        "rust recursion must converge with the rust accumulator loop"
    );
    assert_ne!(
        fold_fp,
        value_fp(&i, sum_loop, Lang::Python),
        "the sum monoid (acc + n, base 0) must stay a hard negative"
    );
}

#[test]
fn float_valued_head_structural_fold_stays_closed() {
    // The recursion→accumulator-fold canon (`normalize.recursion.structural_fold`) is sound only
    // over an associative monoid — and float `+` is NOT associative, so a float-VALUED head must
    // not fold (right-fold recursion → left-fold loop would change the result). The coarse
    // `ValueDomain::Number` does not separate int from float, so the recursion gate excludes a
    // float head via `head_possibly_float`. Proven boundary: structural_fold/Counterexamples.lean.
    let i = Interner::new();
    // INT control: the head `n + 1` is integer-valued — the fold fires and converges with the loop.
    let int_rec = "def g(n):\n    if n == 0:\n        return 0\n    return (n + 1) + g(n - 1)\n";
    let int_loop =
        "def g(n):\n    acc = 0\n    while n != 0:\n        acc = acc + (n + 1)\n        n = n - 1\n    return acc\n";
    assert_eq!(
        value_fp(&i, int_rec, Lang::Python),
        value_fp(&i, int_loop, Lang::Python),
        "an integer-valued head must still fold to its accumulator loop"
    );
    // FLOAT: the head `n + 1.0` is float-valued — the fold must NOT fire, so the recursion does NOT
    // converge with the left-fold loop (they differ for floats under reassociation).
    let flt_rec = "def g(n):\n    if n == 0:\n        return 0\n    return (n + 1.0) + g(n - 1)\n";
    let flt_loop =
        "def g(n):\n    acc = 0\n    while n != 0:\n        acc = acc + (n + 1.0)\n        n = n - 1\n    return acc\n";
    assert_ne!(
        value_fp(&i, flt_rec, Lang::Python),
        value_fp(&i, flt_loop, Lang::Python),
        "a float-valued head must NOT fold (float + is non-associative)"
    );
}

#[test]
fn ruby_shovel_builder_each_stays_closed_without_receiver_proof() {
    // Ruby `xs.each { ... }` stays an ordinary block call until a pack supplies receiver/protocol
    // proof for `xs`. The Ruby `<<` builder signal is still retained inside the opaque call body,
    // but the default analyzer must not infer Enumerable semantics from the method name alone.
    let i = Interner::new();
    let py_comp = "def f(xs):\n    return [x * x for x in xs]\n";
    let ruby_build = "def f(xs)\n  out = []\n  xs.each { |x| out << x * x }\n  out\nend\n";
    let ruby_diff = "def f(xs)\n  out = []\n  xs.each { |x| out << x + 1 }\n  out\nend\n";
    let comp_fp = value_fp(&i, py_comp, Lang::Python);
    assert_ne!(
        comp_fp,
        value_fp(&i, ruby_build, Lang::Ruby),
        "ruby each builder must stay closed without receiver/protocol proof"
    );
    assert_ne!(
        value_fp(&i, ruby_build, Lang::Ruby),
        value_fp(&i, ruby_diff, Lang::Ruby),
        "a different per-element contribution must stay distinct"
    );
}

#[test]
fn java_arraylist_add_builder_loop_converges_with_comprehension() {
    // Java builds a list with `List<T> out = new ArrayList<>(); for (…) out.add(e); return out`.
    // Modeling `new ArrayList<>()` as the empty `array` Seq and `out.add(e)` as the per-element
    // build (scoped by the empty-Seq seed — so overloaded `.add` on a Set/BigInteger never
    // enters) makes the Java builder loop converge with the Python comprehension. A different
    // contribution stays a hard negative.
    let i = Interner::new();
    let py_comp = "def f(xs):\n    return [x * x for x in xs]\n";
    let java_build = "import java.util.*;\nclass C { static List<Integer> f(int[] xs) { List<Integer> out = new ArrayList<>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let java_qualified_build = "class C { static java.util.List<Integer> f(int[] xs) { java.util.List<Integer> out = new java.util.ArrayList<>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let java_build_diff = "import java.util.*;\nclass C { static List<Integer> f(int[] xs) { List<Integer> out = new ArrayList<>(); for (int x : xs) { out.add(x + 1); } return out; } }\n";
    let java_unimported_arraylist = "class C { static Object f(int[] xs) { var out = new ArrayList<Integer>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let java_shadowed_arraylist = "import java.util.*;\nclass ArrayList<T> { void add(T value) {} }\nclass C { static ArrayList<Integer> f(int[] xs) { ArrayList<Integer> out = new ArrayList<>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let java_conflicting_arraylist_import = "import other.ArrayList;\nimport java.util.*;\nclass C { static Object f(int[] xs) { var out = new ArrayList<Integer>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let java_conflicting_exact_arraylist_import = "import other.ArrayList;\nimport java.util.ArrayList;\nclass C { static Object f(int[] xs) { var out = new ArrayList<Integer>(); for (int x : xs) { out.add(x * x); } return out; } }\n";
    let comp_fp = value_fp(&i, py_comp, Lang::Python);
    assert_eq!(
        comp_fp,
        value_fp(&i, java_build, Lang::Java),
        "java ArrayList+add builder loop must converge with the python comprehension"
    );
    assert_eq!(
        comp_fp,
        value_fp(&i, java_qualified_build, Lang::Java),
        "fully-qualified java.util.ArrayList must not require a separate import proof"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, java_build_diff, Lang::Java),
        "a different per-element contribution must stay distinct"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, java_unimported_arraylist, Lang::Java),
        "simple ArrayList constructors need a java.util import proof before exact builder seeding"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, java_shadowed_arraylist, Lang::Java),
        "a local ArrayList type must not mint the java.util empty-list builder seed"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, java_conflicting_arraylist_import, Lang::Java),
        "a conflicting explicit ArrayList import must override java.util wildcard proof"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, java_conflicting_exact_arraylist_import, Lang::Java),
        "a conflicting explicit ArrayList import must close even an exact java.util import proof"
    );
}

#[test]
fn java_static_final_map_field_converges_with_inline_factory_lookup() {
    let i = Interner::new();
    let inline = "import java.util.Map;\n\nclass JavaMapOf {\n  static int lookup(String key, String other) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(key, 0);\n  }\n}\n";
    let field = "import java.util.Map;\n\nclass JavaModuleMap {\n  static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2);\n\n  static int lookup(String key, String other) {\n    return LOOKUP.getOrDefault(key, 0);\n  }\n}\n";
    assert_eq!(
        value_fp(&i, inline, Lang::Java),
        value_fp(&i, field, Lang::Java),
        "Java static final literal map fields must seed method-level map proof"
    );
}

#[test]
fn go_functional_append_builder_loop_converges_with_comprehension() {
    // Go builds a list with `out := []T{}; for … { out = append(out, e) }` — a FUNCTIONAL
    // append (reassignment), not the effect-form `out.append(e)` of Python/JS. Recognizing the
    // `r = append(r, e)` reassign as the same per-element `Map` build (and excluding the builder
    // var from numeric loop-carried seeding) makes the Go builder loop converge with the Python
    // comprehension. The changed-contribution form stays a hard negative.
    let i = Interner::new();
    let py_comp = "def f(xs):\n    return [x * x for x in xs]\n";
    let go_build = "package p\nfunc f(xs []int) []int {\n\tout := []int{}\n\tfor _, x := range xs {\n\t\tout = append(out, x*x)\n\t}\n\treturn out\n}\n";
    let go_build_diff = "package p\nfunc f(xs []int) []int {\n\tout := []int{}\n\tfor _, x := range xs {\n\t\tout = append(out, x+1)\n\t}\n\treturn out\n}\n";
    let comp_fp = value_fp(&i, py_comp, Lang::Python);
    assert_eq!(
        comp_fp,
        value_fp(&i, go_build, Lang::Go),
        "go functional-append builder loop must converge with the python comprehension"
    );
    assert_ne!(
        comp_fp,
        value_fp(&i, go_build_diff, Lang::Go),
        "a different per-element contribution must stay distinct"
    );
}

#[test]
fn promise_then_chain_stays_opaque_without_receiver_proof() {
    // A `.then` name is not itself proof of Promise/thenable semantics. Until the frontend or a
    // semantic pack carries a resolved Promise-like receiver proof, exact value fingerprints must
    // keep `.then` opaque rather than beta-reducing an arbitrary user method.
    let i = Interner::new();
    let await_form = "function f(id) {\n  const r = await db.get(id);\n  return r.x + 1;\n}\n";
    let then_form = "function f(id) {\n  return db.get(id).then(r => r.x + 1);\n}\n";
    let then_diff = "function f(id) {\n  return db.get(id).then(r => r.y - 1);\n}\n";
    let av = value_fp(&i, await_form, Lang::TypeScript);
    assert_ne!(
        av,
        value_fp(&i, then_form, Lang::TypeScript),
        "a `.then` continuation must not converge with await without receiver proof"
    );
    assert_ne!(
        av,
        value_fp(&i, then_diff, Lang::TypeScript),
        "a different continuation expression must stay distinct"
    );
}

#[test]
fn member_call_return_promise_receiver_stays_closed_without_target_proof() {
    let i = Interner::new();
    let member_return = "function f(service) {\n  return service.load().then(x => x + 1);\n}\n";
    let direct_promise = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_ne!(
        value_fp(&i, member_return, Lang::TypeScript),
        value_fp(&i, direct_promise, Lang::TypeScript),
        "member call-return receivers must stay closed without direct method target and return-domain proof"
    );
    assert_ne!(
        value_fp(&i, member_return, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "closed member Promise receiver candidates must not erase into sync payloads"
    );
}

#[test]
fn imported_member_call_return_promise_receiver_stays_closed_without_settled_value_proof() {
    let i = Interner::new();
    let imported_member_return =
        "import * as service from './service';\nfunction f() {\n  return service.load().then(x => x + 1);\n}\n";
    let direct_promise = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_ne!(
        value_fp(&i, imported_member_return, Lang::TypeScript),
        value_fp(&i, direct_promise, Lang::TypeScript),
        "import-backed member target identity alone must not recover a Promise receiver without settled-value proof"
    );
    assert_ne!(
        value_fp(&i, imported_member_return, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "closed imported member Promise receiver candidates must not erase into sync payloads"
    );
}

#[test]
fn proven_promise_then_chains_converge_without_sync_erasure() {
    let i = Interner::new();
    let chained =
        "function f() {\n  return Promise.resolve(1).then(x => x + 1).then(z => z * 2);\n}\n";
    let single = "function f() {\n  return Promise.resolve(1).then(x => (x + 1) * 2);\n}\n";
    assert_eq!(
        value_fp(&i, chained, Lang::TypeScript),
        value_fp(&i, single, Lang::TypeScript),
        "supported Promise.resolve(...).then(...) chains should converge through proven Promise receiver evidence"
    );

    let promise_return = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";
    assert_ne!(
        value_fp(&i, promise_return, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "Promise continuations keep a Promise boundary and must not converge with synchronous payloads"
    );
}

#[test]
fn proven_promise_then_flattens_returned_promise_without_sync_erasure() {
    let i = Interner::new();
    let direct = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let returned_promise =
        "function f() {\n  return Promise.resolve(1).then(x => Promise.resolve(x + 1));\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";
    assert_eq!(
        value_fp(&i, direct, Lang::TypeScript),
        value_fp(&i, returned_promise, Lang::TypeScript),
        "handler-returned Promise.resolve should flatten through the local Promise continuation model"
    );
    assert_ne!(
        value_fp(&i, returned_promise, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "flattening a handler-returned Promise must still preserve the outer Promise boundary"
    );
}

#[test]
fn proven_promise_rejection_recovery_converges_without_channel_erasure() {
    let i = Interner::new();
    let catch_form = "function f() {\n  return Promise.reject(1).catch(e => e + 1);\n}\n";
    let then_reject_form =
        "function f() {\n  return Promise.reject(1).then(undefined, e => e + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";
    let still_rejected = "function f() {\n  return Promise.reject(1);\n}\n";
    assert_eq!(
        value_fp(&i, catch_form, Lang::TypeScript),
        value_fp(&i, then_reject_form, Lang::TypeScript),
        "Promise.catch and then(undefined, onRejected) should converge for a proven rejected producer"
    );
    assert_ne!(
        value_fp(&i, catch_form, Lang::TypeScript),
        value_fp(&i, sync_return, Lang::TypeScript),
        "recovered Promise rejection must not erase the Promise boundary"
    );
    assert_ne!(
        value_fp(&i, catch_form, Lang::TypeScript),
        value_fp(&i, still_rejected, Lang::TypeScript),
        "recovery to fulfillment must stay distinct from an unrecovered rejected channel"
    );
}

#[test]
fn same_file_async_function_then_recovers_without_sync_erasure() {
    let i = Interner::new();
    let async_then = "async function load() {\n  return 1;\n}\n\
function f() {\n  return load().then(x => x + 1);\n}\n";
    let promise_then = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_eq!(
        value_fp_named(&i, async_then, Lang::TypeScript, "f"),
        value_fp(&i, promise_then, Lang::TypeScript),
        "same-file async function calls with non-thenable returns should recover as Promise producers"
    );
    assert_ne!(
        value_fp_named(&i, async_then, Lang::TypeScript, "f"),
        value_fp(&i, sync_return, Lang::TypeScript),
        "async function continuation recovery must preserve the Promise boundary"
    );
}

#[test]
fn same_file_async_function_recovery_stays_closed_for_await_and_throw_paths() {
    let i = Interner::new();
    let awaited = "async function load() {\n  return await Promise.resolve(1);\n}\n\
function f() {\n  return load().then(x => x + 1);\n}\n";
    let fulfilled = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let throwing = "async function load() {\n  throw 1;\n}\n\
function f() {\n  return load().catch(e => e + 1);\n}\n";
    let rejected = "function f() {\n  return Promise.reject(1).catch(e => e + 1);\n}\n";

    assert_ne!(
        value_fp_named(&i, awaited, Lang::TypeScript, "f"),
        value_fp(&i, fulfilled, Lang::TypeScript),
        "awaited async bodies stay closed until scheduling/thenable assimilation is modeled"
    );
    assert_ne!(
        value_fp_named(&i, throwing, Lang::TypeScript, "f"),
        value_fp(&i, rejected, Lang::TypeScript),
        "throwing async bodies must not be recovered as Promise rejection without rejection-channel proof"
    );
}

#[test]
fn direct_function_promise_return_then_recovers_without_sync_erasure() {
    let i = Interner::new();
    let local_return = "function load() {\n  return Promise.resolve(1);\n}\n\
function f() {\n  return load().then(x => x + 1);\n}\n";
    let promise_then = "function f() {\n  return Promise.resolve(1).then(x => x + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_eq!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, promise_then, Lang::TypeScript),
        "direct functions returning proven PromiseLike producers should recover as Promise receivers"
    );
    assert_ne!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, sync_return, Lang::TypeScript),
        "direct function Promise return recovery must preserve the Promise boundary"
    );
}

#[test]
fn direct_function_promise_return_recovery_stays_closed_for_thenables() {
    let i = Interner::new();
    let local_return = "function load(x) {\n  return Promise.resolve(x);\n}\n\
function f(x) {\n  return load(x).then(v => v);\n}\n";
    let direct_promise = "function f(x) {\n  return Promise.resolve(x).then(v => v);\n}\n";

    assert_ne!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, direct_promise, Lang::TypeScript),
        "possible thenable assimilation must remain closed across direct function return recovery"
    );
}

#[test]
fn direct_function_promise_return_recovers_typed_non_thenable_parameter() {
    let i = Interner::new();
    let local_return = "function load(x: number) {\n  return Promise.resolve(x);\n}\n\
function f(x: number) {\n  return load(x).then(v => v + 1);\n}\n";
    let direct_promise =
        "function f(x: number) {\n  return Promise.resolve(x).then(v => v + 1);\n}\n";
    let sync_return = "function f(x: number) {\n  return x + 1;\n}\n";

    assert_eq!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, direct_promise, Lang::TypeScript),
        "direct functions returning Promise.resolve over proven non-thenable parameters should recover"
    );
    assert_ne!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, sync_return, Lang::TypeScript),
        "typed parameter recovery must keep the result behind a Promise boundary"
    );
}

#[test]
fn direct_function_promise_rejection_return_recovers_catch_channel() {
    let i = Interner::new();
    let local_return = "function load() {\n  return Promise.reject(1);\n}\n\
function f() {\n  return load().catch(e => e + 1);\n}\n";
    let direct_reject = "function f() {\n  return Promise.reject(1).catch(e => e + 1);\n}\n";
    let sync_return = "function f() {\n  return 1 + 1;\n}\n";

    assert_eq!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, direct_reject, Lang::TypeScript),
        "direct functions returning proven rejected Promise producers should recover catch continuations"
    );
    assert_ne!(
        value_fp_named(&i, local_return, Lang::TypeScript, "f"),
        value_fp(&i, sync_return, Lang::TypeScript),
        "direct function rejection recovery must keep the result behind a Promise boundary"
    );
}

#[test]
fn go_slice_literal_converges_with_array_but_struct_stays_distinct() {
    // A Go slice literal `[]int{1,2,3}` is an ordered sequence — it converges with a Python
    // list / JS array. A Go STRUCT literal `Point{1,2,3}` is a record, NOT a collection, and
    // must stay distinct (no `Point{1,2,3}` ≡ `[1,2,3]` false merge). Tagging composite
    // literals by type (slice/array → `array`, map → `composite_literal`, struct → `go_struct`)
    // removes the old blanket tag that collapsed all three to one value.
    let i = Interner::new();
    let py_list = "def f():\n    return [1, 2, 3]\n";
    let go_slice = "package p\nfunc f() []int { return []int{1, 2, 3} }\n";
    let go_struct =
        "package p\ntype Point struct{ x, y, z int }\nfunc f() Point { return Point{1, 2, 3} }\n";
    let list_fp = value_fp(&i, py_list, Lang::Python);
    assert_eq!(
        list_fp,
        value_fp(&i, go_slice, Lang::Go),
        "go slice literal must converge with the python list literal"
    );
    assert_ne!(
        list_fp,
        value_fp(&i, go_struct, Lang::Go),
        "a go struct literal must stay distinct from a list (it is a record, not a collection)"
    );
}

#[test]
fn seq_surface_contracts_keep_maps_out_of_collection_membership() {
    let i = Interner::new();
    let py_membership = "def f(x):\n    return x in [\"red\", \"blue\"]\n";
    let go_slice_membership = "package p\nimport \"slices\"\nfunc f(x string) bool { return slices.Contains([]string{\"red\", \"blue\"}, x) }\n";
    let go_map_as_slice_membership = "package p\nimport \"slices\"\nfunc f(x string) bool { return slices.Contains(map[string]int{\"red\": 0, \"blue\": 0}, x) }\n";
    let go_zero_map_lookup =
        "package p\nfunc f(x string) int { return map[string]int{\"red\": 0, \"blue\": 0}[x] }\n";
    let go_empty_map = "package p\nfunc f() map[string]int { return map[string]int{} }\n";
    let go_empty_slice = "package p\nfunc f() []int { return []int{} }\n";

    let membership = value_fp(&i, py_membership, Lang::Python);
    assert_eq!(
        membership,
        value_fp(&i, go_slice_membership, Lang::Go),
        "Go slices.Contains over a slice literal is a proven collection membership"
    );
    assert_ne!(
        membership,
        value_fp(&i, go_map_as_slice_membership, Lang::Go),
        "Go map composite literals must not leak into collection membership semantics"
    );
    assert_ne!(
        value_fp(&i, go_zero_map_lookup, Lang::Go),
        value_fp(&i, go_map_as_slice_membership, Lang::Go),
        "the supported Go zero-map lookup contract is separate from collection membership"
    );
    assert_ne!(
        value_fp(&i, go_empty_map, Lang::Go),
        value_fp(&i, go_empty_slice, Lang::Go),
        "empty Go map literals must not fall back to the empty collection value tag"
    );
}

#[test]
fn js_object_length_and_computed_keys_stay_outside_exact_collection_contracts() {
    let i = Interner::new();
    let py_dict_len = "def f():\n    return len({\"length\": 99, \"a\": 0})\n";
    let js_object_length = "function f() { return ({ length: 99, a: 0 }).length; }";
    let py_object = "def f():\n    return {\"red\": 1, \"blue\": 2}\n";
    let js_static_object = "function f() { return { red: 1, blue: 2 }; }";
    let js_computed_object = "function f(k) { return { [k]: 1, blue: 2 }; }";

    assert_ne!(
        value_fp(&i, py_dict_len, Lang::Python),
        value_fp(&i, js_object_length, Lang::JavaScript),
        "JS object `.length` is a property read, not map/dict cardinality"
    );
    assert_eq!(
        value_fp(&i, py_object, Lang::Python),
        value_fp(&i, js_static_object, Lang::JavaScript),
        "static JS object keys remain an exact map/object literal surface"
    );
    assert_ne!(
        value_fp(&i, js_static_object, Lang::JavaScript),
        value_fp(&i, js_computed_object, Lang::JavaScript),
        "computed object keys need a future key-evaluation contract before exact map semantics"
    );
}

#[test]
fn method_recursion_requires_explicit_call_target_evidence() {
    // Method bodies are not admitted from a bare same-name call. Java static/private/final
    // dispatch and Ruby top-level method lookup need exact source/pack target evidence before
    // recursion→iteration can treat `fac(...)` as direct self-recursion. Free-function recursion
    // remains covered by `rust_recursion_converges_with_iteration_via_return_unwrap`.
    let i = Interner::new();
    let py_loop = "def fac(n):\n    acc = 1\n    while n != 0:\n        acc = acc * n\n        n = n - 1\n    return acc\n";
    let java_rec =
        "class C { static int fac(int n) { if (n == 0) { return 1; } return n * fac(n - 1); } }";
    let ruby_rec = "def fac(n)\n  return 1 if n == 0\n  n * fac(n - 1)\nend\n";
    let sum_loop = "def g(n):\n    acc = 0\n    while n != 0:\n        acc = acc + n\n        n = n - 1\n    return acc\n";
    let fold = value_fp(&i, py_loop, Lang::Python);
    assert_ne!(
        fold,
        value_fp_named(&i, java_rec, Lang::Java, "fac"),
        "java method recursion must stay closed without direct call-target evidence"
    );
    assert_ne!(
        fold,
        value_fp_named(&i, ruby_rec, Lang::Ruby, "fac"),
        "ruby method recursion must stay closed without direct call-target evidence"
    );
    assert_ne!(
        fold,
        value_fp(&i, sum_loop, Lang::Python),
        "the sum monoid (acc + n, base 0) must stay a hard negative"
    );
}

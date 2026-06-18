//! IL-equivalence tests — the heart of correctness. Semantically-equivalent
//! snippets must normalize to the same structural hash; genuinely different code
//! must not. Also covers provenance and an end-to-end detection smoke test.

use nose_detect::{detect, DetectOptions, StructuralDetector};
use nose_il::{Corpus, FileId, Interner, Lang, NodeId, UnitKind};
use nose_normalize::{normalize, subtree_hashes, NormalizeOptions};

/// Normalize `src` and return the structural hash of its first function/method
/// unit. A shared `interner` keeps field-name symbols comparable across calls.
fn unit_hash(interner: &Interner, src: &str, lang: Lang) -> u64 {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    let hashes = subtree_hashes(&n, interner);
    let root = first_func(&n);
    hashes[root.0 as usize]
}

fn first_func(il: &nose_il::Il) -> NodeId {
    il.units
        .iter()
        .find(|u| matches!(u.kind, UnitKind::Function | UnitKind::Method))
        .map(|u| u.root)
        .expect("expected a function unit")
}

fn count_nodes(il: &nose_il::Il, root: NodeId, kind: Option<nose_il::NodeKind>) -> usize {
    let own = match kind {
        Some(kind) => usize::from(il.node(root).kind == kind),
        None => 1,
    };
    own + il
        .children(root)
        .iter()
        .map(|child| count_nodes(il, *child, kind))
        .sum::<usize>()
}

#[path = "equivalence/algebra_laws.rs"]
mod algebra_laws;
#[path = "equivalence/branch_selection.rs"]
mod branch_selection;
#[path = "equivalence/collection_builders.rs"]
mod collection_builders;
#[path = "equivalence/collection_membership.rs"]
mod collection_membership;
#[path = "equivalence/collection_streams.rs"]
mod collection_streams;
#[path = "equivalence/inline_and_anchors.rs"]
mod inline_and_anchors;
#[path = "equivalence/iteration_contracts.rs"]
mod iteration_contracts;
#[path = "equivalence/language_surfaces.rs"]
mod language_surfaces;
#[path = "equivalence/literal_map_defaults.rs"]
mod literal_map_defaults;
#[path = "equivalence/loops_and_reductions.rs"]
mod loops_and_reductions;
#[path = "equivalence/map_default_boundaries.rs"]
mod map_default_boundaries;
#[path = "equivalence/map_key_membership.rs"]
mod map_key_membership;
#[path = "equivalence/numeric_scalars.rs"]
mod numeric_scalars;
#[path = "equivalence/protocol_boundaries.rs"]
mod protocol_boundaries;
#[path = "equivalence/syntax_surfaces.rs"]
mod syntax_surfaces;
#[path = "equivalence/value_graph_core.rs"]
mod value_graph_core;
#[path = "equivalence/value_graph_try_effects.rs"]
mod value_graph_try_effects;

#[test]
fn array_element_swap_does_not_merge_with_clobber() {
    // #337: in-place element mutation. `swap` (`t=a[i]; a[i]=a[j]; a[j]=t`) and `clobber`
    // (`a[i]=a[j]; a[j]=a[i]`) differ: clobber's second `a[i]` read observes the FIRST write,
    // while swap captured the pre-write value in `t`. The value graph forwards a post-write
    // read of `a[i]` to the written value (`index_env`), so the two element-write traces — and
    // thus the fingerprints — differ. They previously SHARED a fingerprint (the no-mutation
    // store model treated every `a[i]` read as the pre-write value): a false merge.
    let i = Interner::new();
    let swap = "def swap(a, i, j):\n    t = a[i]\n    a[i] = a[j]\n    a[j] = t\n";
    let clobber = "def clobber(a, i, j):\n    a[i] = a[j]\n    a[j] = a[i]\n";
    assert_ne!(
        value_fp(&i, swap, Lang::Python),
        value_fp(&i, clobber, Lang::Python),
        "swap must not merge with clobber (post-write reads forward to the written value)"
    );
    // Control — two structurally-identical swaps still converge (read-forwarding is structure-
    // sensitive, not a blanket exclusion of indexed stores, so genuine clones are unaffected).
    let swap2 = "def s2(b, m, n):\n    t = b[m]\n    b[m] = b[n]\n    b[n] = t\n";
    assert_eq!(
        value_fp(&i, swap, Lang::Python),
        value_fp(&i, swap2, Lang::Python),
        "identical swaps still converge"
    );
}

/// Value-graph fingerprint of the first function unit.
fn value_fp(interner: &Interner, src: &str, lang: Lang) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    nose_normalize::value_fingerprint(&n, first_func(&n), interner)
}

fn return_fp(interner: &Interner, src: &str, lang: Lang) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    nose_normalize::value_fingerprint_lits(&n, first_func(&n), interner).2
}

fn value_anchors(interner: &Interner, src: &str, lang: Lang) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    nose_normalize::value_anchors(&n, first_func(&n), interner)
        .into_iter()
        .map(|anchor| anchor.hash)
        .collect()
}

fn shares_any(a: &[u64], b: &[u64]) -> bool {
    a.iter().any(|x| b.contains(x))
}

fn full_anchors(interner: &Interner, src: &str, lang: Lang) -> Vec<nose_normalize::Anchor> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    nose_normalize::value_anchors(&n, first_func(&n), interner)
}

#[test]
fn sub_dag_anchor_carries_source_line_range_of_the_shared_computation() {
    // A heavy sub-DAG anchor records WHERE its computation lives (line range), so a partial clone
    // can report the shared lines. The SAME computation placed on different lines in two functions
    // yields the same anchor hash but each unit's own line range.
    let i = Interner::new();
    let body = |head: &str| {
        format!(
            "{head}\n  const totals = items.map(i => i.price * i.qty).reduce((s, x) => s + x, 0);\n  const tax = totals * 0.1;\n  const shipping = totals > 100 ? 0 : 15;\n  const grand = totals + tax + shipping;\n  log(grand);\n  return grand;\n}}\n"
        )
    };
    // Same heavy computation, but two extra lines push it down in `g`.
    let a = body("function f(items) {");
    let b = body("function g(items) {\n  log(1);\n  log(2);");
    let aa = full_anchors(&i, &a, Lang::TypeScript);
    let bb = full_anchors(&i, &b, Lang::TypeScript);
    // At least one anchor carries a real (non-zero) line range.
    assert!(
        aa.iter().any(|x| x.line_start > 0),
        "an anchor should record its source line range, got {aa:?}",
    );
    // The shared computation produces the same hash in both, on each unit's own line.
    let sh = aa
        .iter()
        .filter(|x| bb.iter().any(|y| y.hash == x.hash))
        .max_by_key(|x| x.weight)
        .expect("the shared computation should produce a common anchor hash");
    let in_b = bb.iter().find(|y| y.hash == sh.hash).unwrap();
    // Each unit reports the shared computation at ITS OWN location: the two leading `log` lines in
    // `g` shift every line down by exactly 2.
    assert!(
        sh.line_start > 0 && in_b.line_start > 0,
        "both carry a real line: {sh:?} / {in_b:?}"
    );
    assert_eq!(
        in_b.line_start,
        sh.line_start + 2,
        "g's shared computation is 2 lines below f's (the two extra `log` lines)",
    );
}

fn value_fp_named(interner: &Interner, src: &str, lang: Lang, name: &str) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    let root = n
        .units
        .iter()
        .find(|unit| {
            unit.name
                .is_some_and(|symbol| interner.resolve(symbol) == name)
        })
        .map(|unit| unit.root)
        .unwrap_or_else(|| panic!("expected function unit named {name}"));
    nose_normalize::value_fingerprint(&n, root, interner)
}

fn class_value_fp(interner: &Interner, src: &str, lang: Lang, name: &str) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t", src.as_bytes(), lang, interner).unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    let root = n
        .units
        .iter()
        .find(|unit| {
            matches!(unit.kind, UnitKind::Class)
                && unit
                    .name
                    .is_some_and(|symbol| interner.resolve(symbol) == name)
        })
        .map(|unit| unit.root)
        .unwrap_or_else(|| panic!("expected class unit named {name}"));
    nose_normalize::value_fingerprint(&n, root, interner)
}

fn corpus_value_fp(corpus: &Corpus, path_suffix: &str, name: &str) -> Vec<u64> {
    let il = corpus
        .files
        .iter()
        .find(|il| il.meta.path.ends_with(path_suffix))
        .unwrap_or_else(|| panic!("expected corpus file ending with {path_suffix}"));
    let n = normalize(il, &corpus.interner, &NormalizeOptions::default());
    let root = n
        .units
        .iter()
        .find(|unit| {
            unit.name
                .is_some_and(|symbol| corpus.interner.resolve(symbol) == name)
        })
        .map(|unit| unit.root)
        .unwrap_or_else(|| panic!("expected function unit named {name} in {path_suffix}"));
    nose_normalize::value_fingerprint(&n, root, &corpus.interner)
}

/// Write `files` into a fresh per-process temp dir named after `tag` and lower
/// them together as one corpus. Callers remove the returned dir when done.
fn lower_temp_corpus(tag: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, Corpus) {
    let dir = std::env::temp_dir().join(format!("{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, contents) in files {
        std::fs::write(dir.join(name), contents).unwrap();
    }
    let corpus = nose_frontend::lower_corpus_many(&[dir.as_path()]);
    (dir, corpus)
}

#[test]
fn python_docstrings_are_function_semantic_noops() {
    let i = Interner::new();
    let plain = "def f(i, j):\n    if i == j:\n        return 1\n    return 0\n";
    let docstring = "def g(i, j):\n    \"\"\"Return one when the indexes match.\"\"\"\n    if i == j:\n        return 1\n    else:\n        return 0\n";
    let other_docstring = "def h(i, j):\n    \"\"\"Different documentation text.\"\"\"\n    if i == j:\n        return 1\n    return 0\n";

    assert_eq!(
        value_fp(&i, plain, Lang::Python),
        value_fp(&i, docstring, Lang::Python),
        "a Python function docstring must not change call behavior"
    );
    assert_eq!(
        value_fp(&i, plain, Lang::Python),
        value_fp(&i, other_docstring, Lang::Python),
        "docstring text is metadata, not function return behavior"
    );

    let returned_red = "def f():\n    return \"red\"\n";
    let returned_blue = "def g():\n    return \"blue\"\n";
    assert_ne!(
        value_fp(&i, returned_red, Lang::Python),
        value_fp(&i, returned_blue, Lang::Python),
        "returned strings are behavior-defining values"
    );

    let f_string = "def f(x):\n    f\"{x}\"\n    return 1\n";
    let no_effect = "def g(x):\n    return 1\n";
    assert_ne!(
        value_fp(&i, f_string, Lang::Python),
        value_fp(&i, no_effect, Lang::Python),
        "a leading f-string expression is not a static docstring proof"
    );
}

#[test]
fn import_named_and_namespace_member_coordinates_converge() {
    let i = Interner::new();
    let js_named = "import { helper } from \"./shared-math\";\nfunction f(value) { return helper(value + 1); }\n";
    let js_namespace = "import * as mathOps from \"./shared-math\";\nfunction f(value) { return mathOps.helper(value + 1); }\n";
    let js_wrong_member = "import * as mathOps from \"./shared-math\";\nfunction f(value) { return mathOps.otherHelper(value + 1); }\n";
    let ts_named = "import { helper } from \"./shared-math\";\nfunction f(value: number): number { return helper(value + 1); }\n";
    let ts_namespace = "import * as mathOps from \"./shared-math\";\nfunction f(value: number): number { return mathOps.helper(value + 1); }\n";
    let ts_type_only =
        "import type { helper } from \"./shared-math\";\nfunction f(value: number): number { return helper(value + 1); }\n";
    let ts_mixed_type_only = "import { helper, type otherHelper } from \"./shared-math\";\nfunction f(value: number): number { return otherHelper(value + 1); }\n";
    let py_named =
        "from shared_math import helper\n\ndef f(value):\n    return helper(value + 1)\n";
    let py_namespace =
        "import shared_math as math_ops\n\ndef f(value):\n    return math_ops.helper(value + 1)\n";
    let py_wrong_member =
        "import shared_math as math_ops\n\ndef f(value):\n    return math_ops.other_helper(value + 1)\n";

    let fp = value_fp(&i, js_named, Lang::JavaScript);
    assert_eq!(fp, value_fp(&i, js_namespace, Lang::JavaScript));
    assert_ne!(fp, value_fp(&i, js_wrong_member, Lang::JavaScript));

    let ts_fp = value_fp(&i, ts_named, Lang::TypeScript);
    assert_eq!(ts_fp, value_fp(&i, ts_namespace, Lang::TypeScript));
    assert_ne!(fp, ts_fp);
    assert_ne!(fp, value_fp(&i, ts_type_only, Lang::TypeScript));
    assert_ne!(fp, value_fp(&i, ts_mixed_type_only, Lang::TypeScript));

    let py_fp = value_fp(&i, py_named, Lang::Python);
    assert_eq!(py_fp, value_fp(&i, py_namespace, Lang::Python));
    assert_ne!(py_fp, value_fp(&i, py_wrong_member, Lang::Python));
}

#[test]
fn js_namespace_imports_ignore_parameter_shadow_mutations_only() {
    let i = Interner::new();
    let plain = r#"
import * as path from "node:path";

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let shadowed_param = r#"
import * as path from "node:path";

export const escapeGlobCharacters = (path: string): string =>
  path.replaceAll(/([!()*?[\\\]{}])/g, "\\$1");

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let unshadowed_non_mutating_js_method = r#"
import * as path from "node:path";

export const touchPath = (): void => {
  path.replaceAll("x", "y");
};

export const replaceRootDirInPath = (rootDir: string, filePath: string): string => {
  if (!filePath.startsWith("<rootDir>")) {
    return filePath;
  }

  return path.resolve(
    rootDir,
    path.normalize(`./${filePath.slice("<rootDir>".length)}`),
  );
};
"#;
    let fp = value_fp_named(&i, plain, Lang::TypeScript, "replaceRootDirInPath");
    assert_eq!(
        fp,
        value_fp_named(&i, shadowed_param, Lang::TypeScript, "replaceRootDirInPath"),
        "a parameter named like the namespace import must not taint the module binding"
    );
    assert_eq!(
        fp,
        value_fp_named(
            &i,
            unshadowed_non_mutating_js_method,
            Lang::TypeScript,
            "replaceRootDirInPath"
        ),
        "a Java-only mutation-like method name must not taint a TypeScript namespace import"
    );
}

#[test]
fn java_arrays_aslist_single_argument_respects_array_provenance() {
    let i = Interner::new();
    let array_membership = "import java.util.Arrays;\n\nclass C { static boolean f(String[] values, String value) { return Arrays.asList(values).contains(value); } }\n";
    let list_membership = "import java.util.Arrays;\nimport java.util.List;\n\nclass C { static boolean f(List<String> values, String value) { return Arrays.asList(values).contains(value); } }\n";
    let singleton_list_membership = "import java.util.List;\n\nclass C { static boolean f(String[] values, String value) { return List.of(values).contains(value); } }\n";
    let parameter_shadowed_arrays = "import java.util.Arrays;\n\nclass FakeArrays { java.util.List<String> asList(String[] values) { return java.util.List.of(\"green\"); } }\nclass C { static boolean f(FakeArrays Arrays, String[] values, String value) { return Arrays.asList(values).contains(value); } }\n";

    let array_fp = value_fp(&i, array_membership, Lang::Java);
    assert_ne!(array_fp, value_fp(&i, list_membership, Lang::Java));
    assert_ne!(
        array_fp,
        value_fp(&i, singleton_list_membership, Lang::Java)
    );
    assert_ne!(
        array_fp,
        value_fp(&i, parameter_shadowed_arrays, Lang::Java)
    );
}

#[test]
fn typed_empty_checks_keep_array_collection_and_string_domains_distinct() {
    let i = Interner::new();
    let java_list_size =
        "class C { static boolean f(java.util.List<Integer> values) { return values == null || values.size() == 0; } }\n";
    let java_list_named =
        "class C { static boolean f(java.util.List<Integer> values) { return values == null || values.isEmpty(); } }\n";
    let java_queue_named = "import java.util.Queue;\n\nclass C { static boolean f(Queue<String> values) { return values == null || values.isEmpty(); } }\n";
    let java_array_length =
        "class C { static boolean f(Object[] values) { return values == null || values.length == 0; } }\n";
    let java_string_named =
        "class C { static boolean f(String value) { return value == null || value.isEmpty(); } }\n";

    let list_fp = value_fp(&i, java_list_size, Lang::Java);
    assert_eq!(list_fp, value_fp(&i, java_list_named, Lang::Java));
    assert_eq!(list_fp, value_fp(&i, java_queue_named, Lang::Java));
    assert_ne!(list_fp, value_fp(&i, java_array_length, Lang::Java));
    assert_ne!(list_fp, value_fp(&i, java_string_named, Lang::Java));
    assert_ne!(
        value_fp(&i, java_array_length, Lang::Java),
        value_fp(&i, java_string_named, Lang::Java)
    );
}

#[test]
fn swift_import_identity_uses_module_and_export_coordinates() {
    let i = Interner::new();
    let imported = r#"
import Shared

func f(_ value: Int) -> Int {
    return Shared.helper(value + 1)
}
"#;
    let renamed = r#"
import Shared

func g(_ input: Int) -> Int {
    return Shared.helper(input + 1)
}
"#;
    let no_import = r#"
func f(_ value: Int) -> Int {
    return Shared.helper(value + 1)
}
"#;
    let wrong_module = r#"
import Other

func f(_ value: Int) -> Int {
    return Other.helper(value + 1)
}
"#;
    let wrong_member = r#"
import Shared

func f(_ value: Int) -> Int {
    return Shared.other(value + 1)
}
"#;

    let fp = value_fp(&i, imported, Lang::Swift);
    assert_eq!(
        fp,
        value_fp(&i, renamed, Lang::Swift),
        "Swift imported namespace calls should be stable under local alpha-renaming"
    );
    assert_ne!(
        fp,
        value_fp(&i, no_import, Lang::Swift),
        "an imported module coordinate must require the import statement"
    );
    assert_ne!(
        fp,
        value_fp(&i, wrong_module, Lang::Swift),
        "changing the imported module coordinate changes the callee identity"
    );
    assert_ne!(
        fp,
        value_fp(&i, wrong_member, Lang::Swift),
        "changing the imported export/member coordinate changes the callee identity"
    );
}

#[test]
fn rust_constructor_pattern_variant_test_stays_distinct() {
    let i = Interner::new();
    // #390: a binding constructor pattern's variant test lowers to the constructor PATH (the
    // discriminant), not the whole pattern as an opaque Raw node. The discriminant must still
    // discriminate — matching `Some(_)` vs `Ok(_)` are different variants and stay distinct, even
    // now that binding extraction makes the *bodies* converge (see
    // `rust_constructor_pattern_binding_extraction_converges`): the arm conditions still differ.
    let some = "pub fn f(x: Option<i32>) -> i32 { match x { Some(a) => a + 1, None => 0 } }\n";
    let ok = "pub fn f(x: Result<i32, i32>) -> i32 { match x { Ok(a) => a + 1, Err(_) => 0 } }\n";
    assert_ne!(
        value_fp(&i, some, Lang::Rust),
        value_fp(&i, ok, Lang::Rust),
        "Some(_) and Ok(_) are different variants — must stay distinct"
    );
}

#[test]
fn rust_constructor_pattern_binding_extraction_converges() {
    // #390 follow-up: a match arm projects its payload binding (`Some(v)` → `v = x.0`) ahead of
    // the body so the body's uses of it alpha-canonicalize. Two copies that differ ONLY in the
    // bound name now converge — closing the split the #390 lowering left open.
    let i = Interner::new();
    let some_a =
        "pub fn f(x: Option<i32>) -> i32 { match x { Some(a) => a * 2 + 1, None => 0 } }\n";
    let some_b =
        "pub fn g(x: Option<i32>) -> i32 { match x { Some(b) => b * 2 + 1, None => 0 } }\n";
    assert_eq!(
        value_fp(&i, some_a, Lang::Rust),
        value_fp(&i, some_b, Lang::Rust),
        "`Some(a) => a*2+1` and `Some(b) => b*2+1` differ only in the bound name — must converge"
    );
    // The body still gates: a different arm computation must NOT merge (no false merge).
    let some_c =
        "pub fn h(x: Option<i32>) -> i32 { match x { Some(c) => c * 3 + 1, None => 0 } }\n";
    assert_ne!(
        value_fp(&i, some_a, Lang::Rust),
        value_fp(&i, some_c, Lang::Rust),
        "different arithmetic in the arm body must stay distinct"
    );
    // Cross-variant stays distinct even though both bodies are now `v = x.0; …` (the arm
    // *condition* — `x == Some` vs `x == Ok` — keeps them apart).
    let ok_a =
        "pub fn k(x: Result<i32, i32>) -> i32 { match x { Ok(a) => a * 2 + 1, Err(_) => 0 } }\n";
    assert_ne!(
        value_fp(&i, some_a, Lang::Rust),
        value_fp(&i, ok_a, Lang::Rust),
        "Some and Ok are different variants — must stay distinct after binding extraction"
    );
}

#[test]
fn option_defaulting_converges_with_nullish_default_boundaries() {
    let i = Interner::new();
    let js = "function f(value, fallback, other, otherDefault) { return value ?? fallback; }";
    let js_guard = "function f(value, fallback, other, otherDefault) { if (value == null) { return fallback; } return value; }";
    let ts_guard = "function f(value: number | null | undefined, fallback: number, other: number | null | undefined, otherDefault: number): number { return value == null ? fallback : value; }";
    let rust_unwrap = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { value.unwrap_or(fallback) }\n";
    let rust_unwrap_else = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { value.unwrap_or_else(|| fallback) }\n";
    let rust_map_or = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { value.map_or(fallback, |inner| inner) }\n";
    let rust_guard = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { if value.is_some() { value.unwrap_or(fallback) } else { fallback } }\n";
    let wrong_default = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { value.unwrap_or(other_default) }\n";
    let wrong_value = "pub fn f(value: Option<i32>, fallback: i32, other: Option<i32>, other_default: i32) -> i32 { other.unwrap_or(fallback) }\n";
    let truthy_or =
        "function f(value, fallback, other, otherDefault) { return value || fallback; }";
    let shadowed_undefined = "function f(value, fallback, other, otherDefault, undefined) { return value === undefined ? fallback : value; }";

    let fp = value_fp(&i, js, Lang::JavaScript);
    assert_eq!(fp, value_fp(&i, js_guard, Lang::JavaScript));
    assert_eq!(fp, value_fp(&i, ts_guard, Lang::TypeScript));
    assert_eq!(fp, value_fp(&i, rust_unwrap, Lang::Rust));
    assert_eq!(fp, value_fp(&i, rust_unwrap_else, Lang::Rust));
    assert_eq!(fp, value_fp(&i, rust_map_or, Lang::Rust));
    assert_eq!(fp, value_fp(&i, rust_guard, Lang::Rust));
    assert_ne!(fp, value_fp(&i, wrong_default, Lang::Rust));
    assert_ne!(fp, value_fp(&i, wrong_value, Lang::Rust));
    assert_ne!(fp, value_fp(&i, truthy_or, Lang::JavaScript));
    assert_ne!(fp, value_fp(&i, shadowed_undefined, Lang::JavaScript));
}

#[test]
fn repeated_nullish_default_with_same_fallback_collapses() {
    let i = Interner::new();
    let single = "function f(value, fallback, otherDefault) { return value ?? fallback; }";
    let repeated =
        "function f(value, fallback, otherDefault) { return (value ?? fallback) ?? fallback; }";
    let different_default =
        "function f(value, fallback, otherDefault) { return (value ?? fallback) ?? otherDefault; }";
    let fp = value_fp(&i, single, Lang::JavaScript);
    assert_eq!(fp, value_fp(&i, repeated, Lang::JavaScript));
    assert_ne!(fp, value_fp(&i, different_default, Lang::JavaScript));
}

#[test]
fn rust_if_let_option_presence_converges_with_option_predicates() {
    let i = Interner::new();
    let if_some = "pub fn f(value: Option<i32>) -> bool {\n    if let Some(_) = value { true } else { false }\n}\n";
    let is_some = "pub fn g(value: Option<i32>) -> bool {\n    value.is_some()\n}\n";
    let if_none = "pub fn h(value: Option<i32>) -> bool {\n    if let None = value { true } else { false }\n}\n";
    let shadowed_some_pattern = "struct Some<T>(T);\npub fn f(value: Some<i32>) -> bool {\n    if let Some(_) = value { true } else { false }\n}\n";
    assert_eq!(
        value_fp(&i, if_some, Lang::Rust),
        value_fp(&i, is_some, Lang::Rust),
        "if let Some(_) should converge with is_some()"
    );
    assert_ne!(
        value_fp(&i, if_some, Lang::Rust),
        value_fp(&i, if_none, Lang::Rust),
        "if let Some(_) must stay distinct from if let None"
    );
    assert_ne!(
        value_fp(&i, if_some, Lang::Rust),
        value_fp_named(&i, shadowed_some_pattern, Lang::Rust, "f"),
        "a local Rust Some pattern must not be treated as Option::Some"
    );
}

#[test]
fn value_graph_distinguishes_boolean_literals() {
    // `True` and `False` are behavior-defining (like `0`≠`1`): a predicate
    // `if x>0: return True else False` and its negation (booleans swapped) compute
    // opposite results and must not collapse. The bool *value* was abstracted away.
    let i = Interner::new();
    let p = "def f(x: int):\n    if x > 0:\n        return True\n    return False\n";
    let q = "def g(x: int):\n    if x > 0:\n        return False\n    return True\n";
    assert_ne!(
        value_fp(&i, p, Lang::Python),
        value_fp(&i, q, Lang::Python),
        "a predicate and its boolean-swapped negation must not fingerprint identically"
    );
    // Cross-language: the same integer predicate in Java converges with Python.
    let java = "class C { static boolean f(int x) { if (x > 0) { return true; } return false; } }";
    assert_eq!(
        value_fp(&i, p, Lang::Python),
        value_fp(&i, java, Lang::Java),
        "same integer predicate should converge across languages"
    );
    // TypeScript `number` keeps its NaN comparison boundary closed.
    let ts = "function f(x: number): boolean { if (x > 0) { return true; } return false; }";
    assert_ne!(
        value_fp(&i, p, Lang::Python),
        value_fp(&i, ts, Lang::TypeScript),
        "TS number predicates must not merge with integer-only predicates"
    );
}

#[test]
fn value_graph_distinguishes_free_callees() {
    // Calls to DIFFERENT global functions must not collapse: alpha-renaming assigned
    // free names a positional cid by occurrence, so `foo(x)`/`bar(x)` became
    // identical IL. `max(a,b)`/`min(a,b)` must also remain distinct after their
    // scalar-choice builtin canonicalization.
    let i = Interner::new();
    let foo = "def f(x):\n    return foo(x)\n";
    let bar = "def f(x):\n    return bar(x)\n";
    assert_ne!(
        value_fp(&i, foo, Lang::Python),
        value_fp(&i, bar, Lang::Python),
        "calls to different globals foo(x) vs bar(x) must differ"
    );
    let mx = "def f(a, b):\n    return max(a, b)\n";
    let mn = "def f(a, b):\n    return min(a, b)\n";
    assert_ne!(
        value_fp(&i, mx, Lang::Python),
        value_fp(&i, mn, Lang::Python),
        "max(a,b) vs min(a,b) must differ"
    );
    // …but the SAME callee in two alpha-renamed copies must still converge.
    let g1 = "def f(items):\n    return helper(items)\n";
    let g2 = "def g(seq):\n    return helper(seq)\n";
    assert_eq!(
        value_fp(&i, g1, Lang::Python),
        value_fp(&i, g2, Lang::Python),
        "same callee with renamed locals must still converge"
    );
}

#[test]
fn value_graph_distinguishes_slice_bounds() {
    // `a[1:]` (drop first) and `a[:1]` (keep first) are different slices — collecting
    // only the slice's named children collapsed both to `Seq(1)`, losing whether the
    // `1` is the start or the stop. They must not fingerprint identically.
    let i = Interner::new();
    let drop1 = "def f(a):\n    return a[1:]\n";
    let keep1 = "def g(a):\n    return a[:1]\n";
    assert_ne!(
        value_fp(&i, drop1, Lang::Python),
        value_fp(&i, keep1, Lang::Python),
        "different slice bounds (a[1:] vs a[:1]) must not fingerprint identically"
    );
}

#[test]
fn value_graph_distinguishes_slice_bounds_go_rust() {
    // Same slice-position bug in Go's `a[1:]`/`a[:1]` and Rust's `&a[1..]`/`&a[..1]`,
    // plus Rust inclusivity `1..2` vs `1..=2`.
    let i = Interner::new();
    let g1 = "package p\nfunc f(a []int) []int {\n\treturn a[1:]\n}\n";
    let g2 = "package p\nfunc f(a []int) []int {\n\treturn a[:1]\n}\n";
    assert_ne!(
        value_fp(&i, g1, Lang::Go),
        value_fp(&i, g2, Lang::Go),
        "Go a[1:] vs a[:1] must differ"
    );
    let r1 = "fn f(a: &[i64]) -> &[i64] {\n    &a[1..]\n}\n";
    let r2 = "fn f(a: &[i64]) -> &[i64] {\n    &a[..1]\n}\n";
    let r3 = "fn f(a: &[i64]) -> &[i64] {\n    &a[1..2]\n}\n";
    let r4 = "fn f(a: &[i64]) -> &[i64] {\n    &a[1..=2]\n}\n";
    assert_ne!(
        value_fp(&i, r1, Lang::Rust),
        value_fp(&i, r2, Lang::Rust),
        "Rust &a[1..] vs &a[..1] must differ"
    );
    assert_ne!(
        value_fp(&i, r3, Lang::Rust),
        value_fp(&i, r4, Lang::Rust),
        "Rust 1..2 (exclusive) vs 1..=2 (inclusive) must differ"
    );
}

#[test]
fn value_graph_distinguishes_while_stride() {
    // `while i<len: s+=a[i]; i+=1` sums every element; `i+=2` sums every other. A
    // non-unit stride visits a subset, so `a[i]` is NOT `Elem(a)` — the two must not
    // fingerprint identically (the while-loop analog of the range-start bug).
    let i = Interner::new();
    let all = "def f(a):\n    s=0\n    i=0\n    while i < len(a):\n        s += a[i]\n        i += 1\n    return s\n";
    let evn = "def g(a):\n    s=0\n    i=0\n    while i < len(a):\n        s += a[i]\n        i += 2\n    return s\n";
    assert_ne!(
        value_fp(&i, all, Lang::Python),
        value_fp(&i, evn, Lang::Python),
        "a strided while-loop must not fingerprint identically to a unit-stride one"
    );
}

#[test]
fn value_graph_distinguishes_early_break() {
    // A loop that `break`s early (sum until acc>100) computes a PREFIX sum — a
    // different value than the full sum. They must not fingerprint identically;
    // `break` cannot be treated as a no-op.
    let i = Interner::new();
    let full = "def f(xs):\n    acc = 0\n    for x in xs:\n        acc += x\n    return acc\n";
    let brk = "def g(xs):\n    acc = 0\n    for x in xs:\n        acc += x\n        if acc > 100:\n            break\n    return acc\n";
    assert_ne!(
        value_fp(&i, full, Lang::Python),
        value_fp(&i, brk, Lang::Python),
        "an early-break loop must not fingerprint identically to a full-iteration loop"
    );
}

#[test]
fn statically_false_loop_guard_skips_dead_body() {
    let i = Interner::new();
    let exact = "class C { static long f(float[] vertex, int strideInBytes, float[] vertices, int numVertices) { final int size = strideInBytes / 4; for (int i = 0; i < numVertices; i++) { final int offset = i * size; boolean found = true; for (int j = 0; !found && j < size; j++) if (vertices[offset + j] != vertex[j]) found = false; if (found) return (long)i; } return -1; } }";
    let epsilon = "class C { static long f(float[] vertex, int strideInBytes, float[] vertices, int numVertices, float epsilon) { final int size = strideInBytes / 4; for (int i = 0; i < numVertices; i++) { final int offset = i * size; boolean found = true; for (int j = 0; !found && j < size; j++) if ((vertices[offset + j] > vertex[j] ? vertices[offset + j] - vertex[j] : vertex[j] - vertices[offset + j]) > epsilon) found = false; if (found) return (long)i; } return -1; } }";
    let executes_from_false = "class C { static long f(float[] vertex, int strideInBytes, float[] vertices, int numVertices) { final int size = strideInBytes / 4; for (int i = 0; i < numVertices; i++) { final int offset = i * size; boolean found = false; for (int j = 0; !found && j < size; j++) if (vertices[offset + j] == vertex[j]) found = true; if (found) return (long)i; } return -1; } }";
    let positive_guard = "class C { static long f(float[] vertex, int strideInBytes, float[] vertices, int numVertices) { final int size = strideInBytes / 4; for (int i = 0; i < numVertices; i++) { final int offset = i * size; boolean found = true; for (int j = 0; found && j < size; j++) if (vertices[offset + j] != vertex[j]) found = false; if (found) return (long)i; } return -1; } }";
    let reassigned_guard = "class C { static long f(float[] vertex, int strideInBytes, float[] vertices, int numVertices) { final int size = strideInBytes / 4; for (int i = 0; i < numVertices; i++) { final int offset = i * size; boolean found = true; found = vertices == vertex; for (int j = 0; !found && j < size; j++) if (vertices[offset + j] != vertex[j]) found = false; if (found) return (long)i; } return -1; } }";
    let fp = value_fp(&i, exact, Lang::Java);
    assert_eq!(
        fp,
        value_fp(&i, epsilon, Lang::Java),
        "a loop guarded by !found after found=true has an unreachable body"
    );
    assert_ne!(fp, value_fp(&i, executes_from_false, Lang::Java));
    assert_ne!(fp, value_fp(&i, positive_guard, Lang::Java));
    assert_ne!(fp, value_fp(&i, reassigned_guard, Lang::Java));
}

#[test]
fn java_low_bit_toggle_parity_converges_with_xor() {
    let i = Interner::new();
    let even_branch = "class C { static int f(int edgeId) { return edgeId % 2 == 0 ? edgeId + 1 : edgeId - 1; } }";
    let xor = "class C { static int g(int edgeKey) { return edgeKey ^ 1; } }";
    let odd_branch = "class C { static int h(int edgeId) { return edgeId % 2 != 0 ? edgeId - 1 : edgeId + 1; } }";
    let reversed = "class C { static int r(int edgeId) { return edgeId % 2 == 0 ? edgeId - 1 : edgeId + 1; } }";
    let xor_two = "class C { static int x(int edgeId) { return edgeId ^ 2; } }";
    let positive_one = "class C { static int p(int edgeId) { return edgeId % 2 == 1 ? edgeId - 1 : edgeId + 1; } }";
    let wrong_delta = "class C { static int w(int edgeId) { return edgeId % 2 == 0 ? edgeId + 1 : edgeId - 2; } }";

    let fp = value_fp(&i, even_branch, Lang::Java);
    assert_eq!(
        fp,
        value_fp(&i, xor, Lang::Java),
        "Java even/odd +/-1 reverse-edge idiom should converge with low-bit xor"
    );
    assert_eq!(
        fp,
        value_fp(&i, odd_branch, Lang::Java),
        "the equivalent != 0 branch order should also converge"
    );
    assert_ne!(fp, value_fp(&i, reversed, Lang::Java));
    assert_ne!(fp, value_fp(&i, xor_two, Lang::Java));
    assert_ne!(fp, value_fp(&i, positive_one, Lang::Java));
    assert_ne!(fp, value_fp(&i, wrong_delta, Lang::Java));
}

#[test]
fn c_u16_big_endian_byte_pack_converges_with_boundaries() {
    let i = Interner::new();
    let add_casted = r#"
typedef unsigned char u8;
unsigned int f(const u8 *a) {
    return (((unsigned int)a[0]) << 8) + ((unsigned int)a[1]);
}
"#;
    let add_uncasted = r#"
typedef unsigned char u8;
int g(u8 *p) {
    return (p[0] << 8) + p[1];
}
"#;
    let bit_or = r#"
unsigned int h(unsigned char *a) {
    return (a[0] << 8) | a[1];
}
"#;
    let uint8_or = r#"
unsigned int j(const uint8_t *a) {
    return (a[0] << 8) | a[1];
}
"#;
    let wrong_order = r#"
typedef unsigned char u8;
unsigned int r(const u8 *a) {
    return (a[1] << 8) | a[0];
}
"#;
    let overlapping_lane = r#"
typedef unsigned char u8;
unsigned int o(const u8 *a) {
    return (a[0] << 4) | a[1];
}
"#;
    let wrong_second_byte = r#"
typedef unsigned char u8;
unsigned int w(const u8 *a) {
    return (a[0] << 8) | a[2];
}
"#;
    let unproven_alias = r#"
typedef unsigned short u8;
unsigned int u(const u8 *a) {
    return (a[0] << 8) | a[1];
}
"#;
    let int_pointer = r#"
unsigned int q(const int *a) {
    return (a[0] << 8) | a[1];
}
"#;

    let fp = value_fp(&i, add_casted, Lang::C);
    assert!(
        fp.len() >= 4,
        "C u16 byte-pack fingerprint must stay large enough for exact scan buckets: {} atoms",
        fp.len()
    );
    assert_eq!(fp, value_fp(&i, add_uncasted, Lang::C));
    assert_eq!(fp, value_fp(&i, bit_or, Lang::C));
    assert_eq!(fp, value_fp(&i, uint8_or, Lang::C));
    assert_ne!(fp, value_fp(&i, wrong_order, Lang::C));
    assert_ne!(fp, value_fp(&i, overlapping_lane, Lang::C));
    assert_ne!(fp, value_fp(&i, wrong_second_byte, Lang::C));
    assert_ne!(fp, value_fp(&i, unproven_alias, Lang::C));
    assert_ne!(fp, value_fp(&i, int_pointer, Lang::C));
}

#[test]
fn c_u32_big_endian_byte_pack_requires_unsigned_high_lane() {
    let i = Interner::new();
    let add_casted_alias = r#"
typedef unsigned char u8;
typedef unsigned int u32;
u32 f(const u8 *a) {
    return (((u32)a[0]) << 24) + (((u32)a[1]) << 16) + (((u32)a[2]) << 8) + ((u32)a[3]);
}
"#;
    let or_casted_alias = r#"
typedef unsigned char u8;
typedef unsigned int u32;
u32 g(u8 *a) {
    return ((u32)a[0] << 24) | ((u32)a[1] << 16) | ((u32)a[2] << 8) | ((u32)a[3]);
}
"#;
    let unsigned_int_cast = r#"
unsigned int h(unsigned char *a) {
    return ((unsigned int)a[0] << 24) + ((unsigned int)a[1] << 16) + ((unsigned int)a[2] << 8) + (unsigned int)a[3];
}
"#;
    let high_lane_uncasted = r#"
typedef unsigned char u8;
typedef unsigned int u32;
u32 u(const u8 *a) {
    return (a[0] << 24) | (a[1] << 16) | (a[2] << 8) | a[3];
}
"#;
    let wrong_order = r#"
typedef unsigned char u8;
typedef unsigned int u32;
u32 r(const u8 *a) {
    return ((u32)a[1] << 24) | ((u32)a[0] << 16) | ((u32)a[2] << 8) | ((u32)a[3]);
}
"#;
    let wrong_alias = r#"
typedef unsigned char u8;
typedef signed int u32;
u32 s(const u8 *a) {
    return ((u32)a[0] << 24) | ((u32)a[1] << 16) | ((u32)a[2] << 8) | ((u32)a[3]);
}
"#;
    let int_pointer = r#"
typedef unsigned int u32;
u32 q(const int *a) {
    return ((u32)a[0] << 24) | ((u32)a[1] << 16) | ((u32)a[2] << 8) | ((u32)a[3]);
}
"#;

    let fp = value_fp(&i, add_casted_alias, Lang::C);
    assert!(
        fp.len() >= 4,
        "C u32 byte-pack fingerprint must stay large enough for exact scan buckets: {} atoms",
        fp.len()
    );
    assert_eq!(fp, value_fp(&i, or_casted_alias, Lang::C));
    assert_eq!(fp, value_fp(&i, unsigned_int_cast, Lang::C));
    assert_ne!(fp, value_fp(&i, high_lane_uncasted, Lang::C));
    assert_ne!(fp, value_fp(&i, wrong_order, Lang::C));
    assert_ne!(fp, value_fp(&i, wrong_alias, Lang::C));
    assert_ne!(fp, value_fp(&i, int_pointer, Lang::C));
}

#[test]
fn value_graph_cross_language_reorder() {
    // Same computation, different statement order, different language.
    let i = Interner::new();
    let py = "def f(a: int, b: int):\n    p = a * b\n    q = a + b\n    return p + q + p\n";
    let ts = "function g(a: number, b: number): number { const q = a + b; const p = a * b; return p + q + p; }";
    assert_eq!(
        value_fp(&i, py, Lang::Python),
        value_fp(&i, ts, Lang::TypeScript)
    );
}

/// Exploratory probe (research): candidate SOUND algebraic/boolean equivalences that
/// stress phase-ordering — does a single bottom-up `mk` pass reach the canonical form,
/// or would a fixpoint/saturation be needed? Not an assertion; a frontier map.
/// Run: cargo test convergence_probe5 -- --nocapture
#[test]
fn convergence_probe5() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        // Distribution in the EXPANSION direction (current code only FACTORS).
        (
            "distribute-expand",
            "def f(a,b,c):\n    return c*(a+b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return c*a+c*b\n",
            Lang::Python,
        ),
        // Factor where the shared multiplicand is on the LEFT of one product.
        (
            "factor-left-shared",
            "def f(a,b,c):\n    return c*a+b*c\n",
            Lang::Python,
            "def g(a,b,c):\n    return (a+b)*c\n",
            Lang::Python,
        ),
        // De Morgan composed with comparison-direction: needs algebra THEN compare-canon.
        (
            "demorgan+cmp",
            "def f(a,b):\n    return not (a>b or a==b)\n",
            Lang::Python,
            "def g(a,b):\n    return a<b\n",
            Lang::Python,
        ),
        // Nested distribution requiring re-canonicalization of a synthesized node.
        (
            "distribute-3term",
            "def f(a,b,d,c):\n    return a*c+b*c+d*c\n",
            Lang::Python,
            "def g(a,b,d,c):\n    return (a+b+d)*c\n",
            Lang::Python,
        ),
        // Distribution feeding AC sort: (a+b)*c + e  vs  c*b + c*a + e
        (
            "distribute-then-ac",
            "def f(a,b,c,e):\n    return c*b+c*a+e\n",
            Lang::Python,
            "def g(a,b,c,e):\n    return (a+b)*c+e\n",
            Lang::Python,
        ),
        // Double negation pushed through a comparison then re-canon.
        (
            "not-not-cmp",
            "def f(a,b):\n    return not (not (a>b))\n",
            Lang::Python,
            "def g(a,b):\n    return b<a\n",
            Lang::Python,
        ),
        // Negation distributed then factored back.
        (
            "neg-distribute-factor",
            "def f(a,b,c):\n    return -(a*c+b*c)\n",
            Lang::Python,
            "def g(a,b,c):\n    return -((a+b)*c)\n",
            Lang::Python,
        ),
        // Decompose the demorgan+cmp gap:
        // (a) lattice fact alone: (a<=b) ∧ (a!=b) ≡ a<b
        (
            "lattice-le-ne",
            "def f(a,b):\n    return a<=b and a!=b\n",
            Lang::Python,
            "def g(a,b):\n    return a<b\n",
            Lang::Python,
        ),
        // (b) De Morgan over OR alone in the value graph
        (
            "demorgan-or",
            "def f(a,b,c):\n    return not (a<b or c<b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return a>=b and c>=b\n",
            Lang::Python,
        ),
        // (c) De Morgan over AND alone
        (
            "demorgan-and",
            "def f(a,b,c):\n    return not (a<b and c<b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return a>=b or c>=b\n",
            Lang::Python,
        ),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe5: {}/{} converge", pairs.len() - gaps, pairs.len());
}

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

/// Convergence probe (research): diverse genuinely-equivalent pairs that a strong IL
/// SHOULD converge. Prints which converge vs not — the non-converging ones are IL gaps to
/// close. Not an assertion (a map of the frontier). Run: cargo test convergence_probe -- --nocapture
#[test]
fn convergence_probe() {
    let pairs: &[(&str, &str, &str)] = &[
        ("nested-if vs conjunction",
         "def f(a,b,c):\n    if a>0:\n        if b>0:\n            return c+1\n    return c+2\n",
         "def g(a,b,c):\n    if a>0 and b>0:\n        return c+1\n    return c+2\n"),
        ("else-after-return vs guard",
         "def f(a,b):\n    if a>0:\n        return b+1\n    else:\n        return b+2\n",
         "def g(a,b):\n    if a>0:\n        return b+1\n    return b+2\n"),
        ("ternary vs if-assign",
         "def f(a,b):\n    x = b+1 if a>0 else b+2\n    return x*3\n",
         "def g(a,b):\n    if a>0:\n        x = b+1\n    else:\n        x = b+2\n    return x*3\n"),
        ("filter fusion",
         "def f(xs):\n    return [x for x in xs if x>0 if x<10]\n",
         "def g(xs):\n    return [x for x in [y for y in xs if y>0] if x<10]\n"),
        ("map-filter (filter then map)",
         "def f(xs):\n    return [h(x) for x in xs if x>0]\n",
         "def g(xs):\n    return [h(y) for y in [x for x in xs if x>0]]\n"),
        ("fold-map fusion (sum of squares)",
         "def f(xs):\n    return sum(x*x for x in xs)\n",
         "def g(xs):\n    t = 0\n    for x in xs:\n        t += x*x\n    return t\n"),
        ("De Morgan",
         "def f(a,b):\n    return not (a>0 and b>0)\n",
         "def g(a,b):\n    return a<=0 or b<=0\n"),
        ("comparison swap inside",
         "def f(a,b):\n    return (a>b) and (b>0)\n",
         "def g(a,b):\n    return (0<b) and (b<a)\n"),
        ("double map then sum",
         "def f(xs):\n    return sum(g(f(x)) for x in xs)\n",
         "def g2(xs):\n    return sum(g(y) for y in [f(x) for x in xs])\n"),
        ("while vs for-range sum",
         "def f(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=xs[i]\n    return t\n",
         "def g(xs):\n    t=0\n    i=0\n    while i<len(xs):\n        t+=xs[i]\n        i+=1\n    return t\n"),
    ];
    let i = Interner::new();
    let mut gaps = 0;
    for (name, a, b) in pairs {
        let eq = value_fp(&i, a, Lang::Python) == value_fp(&i, b, Lang::Python);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!(
        "convergence probe: {}/{} converge",
        pairs.len() - gaps,
        pairs.len()
    );
}

/// Cross-language + more-construct convergence probe (research): the SAME algorithm in
/// different languages / forms should converge to one fingerprint. Maps the frontier.
#[test]
fn convergence_probe_xlang() {
    // (name, srcA, langA, srcB, langB)
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("sum-loop Py vs JS-reduce",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "function f(xs){ return xs.reduce((a,x)=>a+x, 0); }", Lang::JavaScript),
        ("sum-loop Py vs Go",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "package p\nfunc f(xs []int) int {\n\tt := 0\n\tfor _, x := range xs {\n\t\tt += x\n\t}\n\treturn t\n}\n", Lang::Go),
        ("sum-loop Py vs Rust-fold",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "fn f(xs: &[i64]) -> i64 { xs.iter().fold(0, |a, x| a + x) }", Lang::Rust),
        ("map Py vs JS",
         "def f(xs):\n    return [x*x for x in xs]\n", Lang::Python,
         "function f(xs){ return xs.map(x => x*x); }", Lang::JavaScript),
        ("guard Py vs Go",
         "def f(a,b):\n    if a>0:\n        return b+1\n    return b+2\n", Lang::Python,
         "package p\nfunc f(a,b int) int {\n\tif a>0 {\n\t\treturn b+1\n\t}\n\treturn b+2\n}\n", Lang::Go),
        ("x*2 vs x+x", "def f(x):\n    return x*2\n", Lang::Python, "def g(x):\n    return x+x\n", Lang::Python),
        ("abs idioms Py", "def f(x: int):\n    return x if x>=0 else -x\n", Lang::Python, "def g(x: int):\n    return abs(x)\n", Lang::Python),
        ("compound assign", "def f(a,b):\n    a += b\n    a *= 2\n    return a\n", Lang::Python, "def g(a,b):\n    return (a+b)*2\n", Lang::Python),
        ("min idioms", "def f(a,b):\n    return a if a<b else b\n", Lang::Python, "def g(a,b):\n    return min(a,b)\n", Lang::Python),
        ("count loop vs sum-1", "def f(xs):\n    c=0\n    for x in xs:\n        if x>0:\n            c+=1\n    return c\n", Lang::Python, "def g(xs):\n    return sum(1 for x in xs if x>0)\n", Lang::Python),
    ];
    let i = Interner::new();
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!(
        "xlang probe: {}/{} converge",
        pairs.len() - gaps,
        pairs.len()
    );
}

/// More-construct convergence probe (research, batch 2): widen the frontier map.
#[test]
fn convergence_probe2() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("chained compare", "def f(a,b,c):\n    return a<b<c\n", Lang::Python, "def g(a,b,c):\n    return a<b and b<c\n", Lang::Python),
        ("aug-sub vs assign", "def f(a,b):\n    a -= b\n    return a\n", Lang::Python, "def g(a,b):\n    return a-b\n", Lang::Python),
        ("not-eq vs !=", "def f(a,b):\n    return not (a==b)\n", Lang::Python, "def g(a,b):\n    return a!=b\n", Lang::Python),
        ("double not", "def f(a,b):\n    return not (not (a<b))\n", Lang::Python, "def g(a,b):\n    return a<b\n", Lang::Python),
        ("or-default vs ternary", "def f(a,b):\n    return a if a else b\n", Lang::Python, "def g(a,b):\n    return a or b\n", Lang::Python),
        ("nested ternary vs elif", "def f(a):\n    return 1 if a>0 else (2 if a<0 else 3)\n", Lang::Python, "def g(a):\n    if a>0:\n        return 1\n    elif a<0:\n        return 2\n    return 3\n", Lang::Python),
        ("product loop vs reduce", "def f(xs):\n    p=1\n    for x in xs:\n        p*=x\n    return p\n", Lang::Python, "def g(xs):\n    r=1\n    for x in xs:\n        r=r*x\n    return r\n", Lang::Python),
        ("max loop vs max()", "def f(xs):\n    m=xs[0]\n    for x in xs:\n        if x>m:\n            m=x\n    return m\n", Lang::Python, "def g(xs):\n    return max(xs)\n", Lang::Python),
        ("filter-map JS vs Py", "function f(xs){ return xs.filter(x=>x>0).map(x=>h(x)); }", Lang::JavaScript, "def g(xs):\n    return [h(x) for x in xs if x>0]\n", Lang::Python),
        ("early-continue vs filter", "def f(xs):\n    t=0\n    for x in xs:\n        if x<=0:\n            continue\n        t+=x\n    return t\n", Lang::Python, "def g(xs):\n    return sum(x for x in xs if x>0)\n", Lang::Python),
        ("swap temps", "def f(a,b):\n    t=a\n    a=b\n    b=t\n    return a-b\n", Lang::Python, "def g(a,b):\n    return b-a\n", Lang::Python),
        ("redundant paren-group", "def f(a,b,c):\n    return (a+b)+c\n", Lang::Python, "def g(a,b,c):\n    return a+(b+c)\n", Lang::Python),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe2: {}/{} converge", pairs.len() - gaps, pairs.len());
}

/// Convergence probe batch 3 (research): slices, enumerate, dict, recursion, more xlang.
#[test]
fn convergence_probe3() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("enumerate vs range-index", "def f(xs):\n    t=0\n    for i,x in enumerate(xs):\n        t+=i*x\n    return t\n", Lang::Python, "def g(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=i*xs[i]\n    return t\n", Lang::Python),
        ("dict-comp vs loop", "def f(xs):\n    return {x: x*x for x in xs}\n", Lang::Python, "def g(xs):\n    d={}\n    for x in xs:\n        d[x]=x*x\n    return d\n", Lang::Python),
        ("any vs or-loop", "def f(xs):\n    return any(x>0 for x in xs)\n", Lang::Python, "def g(xs):\n    for x in xs:\n        if x>0:\n            return True\n    return False\n", Lang::Python),
        ("all vs and-loop", "def f(xs):\n    return all(x>0 for x in xs)\n", Lang::Python, "def g(xs):\n    for x in xs:\n        if not (x>0):\n            return False\n    return True\n", Lang::Python),
        ("reversed-compare", "def f(a,b):\n    return a>=b\n", Lang::Python, "def g(a,b):\n    return b<=a\n", Lang::Python),
        ("neg distribute", "def f(a,b):\n    return -(a+b)\n", Lang::Python, "def g(a,b):\n    return -a-b\n", Lang::Python),
        ("mul-add factor", "def f(a,b,c):\n    return a*c+b*c\n", Lang::Python, "def g(a,b,c):\n    return (a+b)*c\n", Lang::Python),
        ("string concat order", "def f(a,b):\n    return a+b+a\n", Lang::Python, "def g(a,b):\n    return a+(b+a)\n", Lang::Python),
        ("Go for-i vs Py range", "package p\nfunc f(xs []int) int {\n\tt:=0\n\tfor i:=0;i<len(xs);i++{\n\t\tt+=xs[i]\n\t}\n\treturn t\n}\n", Lang::Go, "def g(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=xs[i]\n    return t\n", Lang::Python),
        ("Rust map vs Py comp", "fn f(xs:&[i64])->Vec<i64>{ xs.iter().map(|x| x*x).collect() }", Lang::Rust, "def g(xs):\n    return [x*x for x in xs]\n", Lang::Python),
        ("compound vs explicit", "def f(a):\n    a //= 2\n    return a\n", Lang::Python, "def g(a):\n    return a // 2\n", Lang::Python),
        ("nested-neg", "def f(x):\n    return -(-(-x))\n", Lang::Python, "def g(x):\n    return -x\n", Lang::Python),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe3: {}/{} converge", pairs.len() - gaps, pairs.len());
}

/// Convergence probe batch 4 (research): candidate Type-4 equivalences to scope which to
/// close next — negative indexing, count-of-filter, reduce-lambda selection, more idioms.
#[test]
fn convergence_probe4() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        (
            "neg-index last",
            "def f(s):\n    return s[len(s)-1]\n",
            Lang::Python,
            "def g(s):\n    return s[-1]\n",
            Lang::Python,
        ),
        (
            "neg-index k",
            "def f(s):\n    return s[len(s)-2]\n",
            Lang::Python,
            "def g(s):\n    return s[-2]\n",
            Lang::Python,
        ),
        (
            "len-count vs sum-1",
            "def f(xs):\n    return len([x for x in xs if x>0])\n",
            Lang::Python,
            "def g(xs):\n    return sum(1 for x in xs if x>0)\n",
            Lang::Python,
        ),
        (
            "reduce-lambda max vs max()",
            "def f(xs):\n    return reduce(lambda a,b: a if a>b else b, xs)\n",
            Lang::Python,
            "def g(xs):\n    return max(xs)\n",
            Lang::Python,
        ),
        (
            "reduce-lambda min vs min()",
            "def f(xs):\n    return reduce(lambda a,b: a if a<b else b, xs)\n",
            Lang::Python,
            "def g(xs):\n    return min(xs)\n",
            Lang::Python,
        ),
        (
            "not-in vs not(in)",
            "def f(a,b):\n    return a not in b\n",
            Lang::Python,
            "def g(a,b):\n    return not (a in b)\n",
            Lang::Python,
        ),
        (
            "filter Py vs JS",
            "def f(xs):\n    return [x for x in xs if x>0]\n",
            Lang::Python,
            "function g(xs){ return xs.filter(x=>x>0); }",
            Lang::JavaScript,
        ),
        (
            "sum-filter Py vs Rust",
            "def f(xs):\n    return sum(x for x in xs if x>0)\n",
            Lang::Python,
            "fn g(xs:&[i64])->i64{ xs.iter().filter(|x| **x>0).sum() }",
            Lang::Rust,
        ),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe4: {}/{} converge", pairs.len() - gaps, pairs.len());
}

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

#[test]
fn c_hex_literal_with_e_lowers_to_int_not_float() {
    // 0xE5 is a hex INTEGER (229); the 'E' is a hex digit, not a float exponent.
    let i = Interner::new();
    let il = nose_frontend::lower_source(FileId(0), "t", b"int f(){ return 0xE5; }", Lang::C, &i)
        .unwrap();
    let root = first_func(&il);
    let s = il.to_sexpr(root, &i);
    assert!(
        !s.to_lowercase().contains("float"),
        "0xE5 (hex int) must not lower to a float literal: {s}"
    );
}

#[test]
fn python_true_division_stays_distinct_from_floor_division() {
    // `5 / 2 == 2.5` but `5 // 2 == 2` — collapsing both onto one Div op merged
    // behaviorally-different functions into one semantic family (a false merge).
    let i = Interner::new();
    let true_div = "def f(xs, d):\n    out = []\n    for x in xs:\n        out.append(x / d)\n    return out\n";
    let floor_div = "def g(xs, d):\n    out = []\n    for x in xs:\n        out.append(x // d)\n    return out\n";
    assert_ne!(
        value_fp(&i, true_div, Lang::Python),
        value_fp(&i, floor_div, Lang::Python),
        "true division and floor division must not share a fingerprint"
    );
    // Floor division still converges with itself across renames.
    let floor_div2 = "def h(items, n):\n    res = []\n    for v in items:\n        res.append(v // n)\n    return res\n";
    assert_eq!(
        value_fp(&i, floor_div, Lang::Python),
        value_fp(&i, floor_div2, Lang::Python),
        "alpha-renamed floor divisions must still converge"
    );
}

#[test]
fn python_floor_division_interprets_with_floor_semantics() {
    // The oracle's FloorDiv rounds toward −∞ like Python `//` — NOT toward zero
    // like `Op::Div` — so a bad canonicalization between the two cannot hide.
    let i = Interner::new();
    let il = nose_frontend::lower_source(
        FileId(0),
        "t",
        b"def f(a, b):\n    return a // b\n",
        Lang::Python,
        &i,
    )
    .unwrap();
    let n = normalize(&il, &i, &NormalizeOptions::default());
    let f = first_func(&n);
    use nose_normalize::{run_unit, Value};
    let run = |a: i64, b: i64| {
        run_unit(&n, &i, f, &[Value::Int(a), Value::Int(b)])
            .unwrap()
            .ret
    };
    assert_eq!(run(5, 2), Value::Int(2));
    assert_eq!(run(-5, 2), Value::Int(-3), "-5 // 2 floors to -3");
    assert_eq!(run(5, -2), Value::Int(-3), "5 // -2 floors to -3");
    assert_eq!(run(-5, -2), Value::Int(2));
    assert_eq!(run(7, 0), Value::Err, "division by zero errs");
}

#[test]
fn python_matmul_stays_distinct_from_elementwise_mul() {
    // `a @ b` (matrix product) is not `a * b` (elementwise); mapping `@` onto Mul
    // merged the two. `@` keeps a raw shape keyed by its own spelling.
    let i = Interner::new();
    let mul = "def f(a, b):\n    c = a * b\n    d = a * c\n    return c, d\n";
    let matmul = "def g(a, b):\n    c = a @ b\n    d = a @ c\n    return c, d\n";
    assert_ne!(
        value_fp(&i, mul, Lang::Python),
        value_fp(&i, matmul, Lang::Python),
        "matmul must not share a fingerprint with elementwise mul"
    );
}

#[test]
fn js_unsigned_shift_stays_distinct_from_signed_shift() {
    // `-5 >> 1 == -3` (sign-extends) but `-5 >>> 1 == 2147483645` (zero-fills);
    // collapsing `>>>` onto Shr merged the two shifts.
    let i = Interner::new();
    let signed = "function f(xs, n) {\n  const out = [];\n  for (const x of xs) out.push(x >> n);\n  return out;\n}";
    let unsigned = "function g(xs, n) {\n  const out = [];\n  for (const x of xs) out.push(x >>> n);\n  return out;\n}";
    assert_ne!(
        value_fp(&i, signed, Lang::JavaScript),
        value_fp(&i, unsigned, Lang::JavaScript),
        "signed and unsigned right shift must not share a fingerprint"
    );
    let unsigned2 = "function h(ys, k) {\n  const res = [];\n  for (const y of ys) res.push(y >>> k);\n  return res;\n}";
    assert_eq!(
        value_fp(&i, unsigned, Lang::JavaScript),
        value_fp(&i, unsigned2, Lang::JavaScript),
        "alpha-renamed unsigned shifts must still converge"
    );
}

#[test]
fn js_shift_is_int32_and_distinct_from_arbitrary_precision() {
    // series 9: `& | ^` were narrowed to int32 (#283-D) but `<<`/`>>` were not, so JS
    // `a << b` (shifts ToInt32(a), 32-bit) false-merged with Python's arbitrary-precision
    // `a << b` — e.g. `1 << 31` is -2147483648 in JS but 2147483648 in Python.
    let i = Interner::new();
    let js_shl = "function f(a, b) { return a << b; }";
    let py_shl = "def f(a, b):\n    return a << b\n";
    let js_shr = "function g(a, b) { return a >> b; }";
    let py_shr = "def g(a, b):\n    return a >> b\n";
    assert_ne!(
        value_fp(&i, js_shl, Lang::JavaScript),
        value_fp(&i, py_shl, Lang::Python),
        "JS `<<` is int32; must not merge with arbitrary-precision Python `<<`"
    );
    assert_ne!(
        value_fp(&i, js_shr, Lang::JavaScript),
        value_fp(&i, py_shr, Lang::Python),
        "JS `>>` is int32; must not merge with arbitrary-precision Python `>>`"
    );
    // Recall preserved: same-language shifts (and JS-vs-JS) still converge.
    let js_shl2 = "function h(x, y) { return x << y; }";
    assert_eq!(
        value_fp(&i, js_shl, Lang::JavaScript),
        value_fp(&i, js_shl2, Lang::JavaScript),
        "two JS `<<` must still converge"
    );
    let py_shl2 = "def h(x, y):\n    return x << y\n";
    assert_eq!(
        value_fp(&i, py_shl, Lang::Python),
        value_fp(&i, py_shl2, Lang::Python),
        "two Python `<<` must still converge"
    );
}

#[test]
fn js_mixed_string_addition_keeps_grouping_ordered() {
    // JS `+` is not just numeric add or string concat: when a string participates,
    // later numeric operands are coerced to strings in left-to-right order.
    // `"a" + 2 + 3` is `"a23"`, while `"a" + (2 + 3)` / `"a" + 5` is `"a5"`.
    // Flattening/folding an untyped JS `+` chain therefore false-merges real code.
    let i = Interner::new();
    let left_assoc = "function f(x) { return x + 2 + 3; }";
    let grouped = "function g(x) { return x + (2 + 3); }";
    let folded = "function h(x) { return x + 5; }";
    assert_ne!(
        value_fp(&i, left_assoc, Lang::JavaScript),
        value_fp(&i, grouped, Lang::JavaScript),
        "untyped JS `x + 2 + 3` must not merge with `x + (2 + 3)`"
    );
    assert_ne!(
        value_fp(&i, left_assoc, Lang::JavaScript),
        value_fp(&i, folded, Lang::JavaScript),
        "untyped JS `x + 2 + 3` must not merge with `x + 5`"
    );

    let typed_left = "function f(x: number): number { return x + 2 + 3; }";
    let typed_grouped = "function g(x: number): number { return x + (2 + 3); }";
    assert_eq!(
        value_fp(&i, typed_left, Lang::TypeScript),
        value_fp(&i, typed_grouped, Lang::TypeScript),
        "TypeScript number evidence should preserve numeric associativity recall"
    );

    let sub = "function f(x) { return x - 3; }";
    let add_neg = "function g(x) { return x + (-3); }";
    assert_ne!(
        value_fp(&i, sub, Lang::JavaScript),
        value_fp(&i, add_neg, Lang::JavaScript),
        "untyped JS `x - 3` must not merge with `x + (-3)`"
    );

    let neg_grouped = "function f(x) { return -(x + 2); }";
    let distributed = "function g(x) { return -x - 2; }";
    assert_ne!(
        value_fp(&i, neg_grouped, Lang::JavaScript),
        value_fp(&i, distributed, Lang::JavaScript),
        "untyped JS `-(x + 2)` must not distribute over potentially-string `+`"
    );

    let typed_sub = "function f(x: number): number { return x - 3; }";
    let typed_add_neg = "function g(x: number): number { return x + (-3); }";
    assert_eq!(
        value_fp(&i, typed_sub, Lang::TypeScript),
        value_fp(&i, typed_add_neg, Lang::TypeScript),
        "TypeScript number evidence should preserve subtraction/add-negation recall"
    );
}

#[test]
fn js_value_returning_logical_operators_keep_operand_order() {
    // JS `||`/`&&` return one of the operand values, not a coerced Bool. With
    // `a = "left"` and `b = "right"`, `a || b` returns `a` while `b || a` returns `b`;
    // `a && b` returns `b` while `b && a` returns `a`.
    let i = Interner::new();
    let or_ab = "function f(a, b) { return a || b; }";
    let or_ba = "function g(a, b) { return b || a; }";
    let and_ab = "function h(a, b) { return a && b; }";
    let and_ba = "function k(a, b) { return b && a; }";
    assert_ne!(
        value_fp(&i, or_ab, Lang::JavaScript),
        value_fp(&i, or_ba, Lang::JavaScript),
        "untyped JS `a || b` must not merge with `b || a`"
    );
    assert_ne!(
        value_fp(&i, and_ab, Lang::JavaScript),
        value_fp(&i, and_ba, Lang::JavaScript),
        "untyped JS `a && b` must not merge with `b && a`"
    );

    let bool_or_ab = "function f(a: boolean, b: boolean): boolean { return a || b; }";
    let bool_or_ba = "function g(a: boolean, b: boolean): boolean { return b || a; }";
    let bool_and_ab = "function h(a: boolean, b: boolean): boolean { return a && b; }";
    let bool_and_ba = "function k(a: boolean, b: boolean): boolean { return b && a; }";
    assert_eq!(
        value_fp(&i, bool_or_ab, Lang::TypeScript),
        value_fp(&i, bool_or_ba, Lang::TypeScript),
        "typed boolean `||` should keep commutative recall"
    );
    assert_eq!(
        value_fp(&i, bool_and_ab, Lang::TypeScript),
        value_fp(&i, bool_and_ba, Lang::TypeScript),
        "typed boolean `&&` should keep commutative recall"
    );
}

#[test]
fn js_loose_equality_stays_distinct_from_strict_equality() {
    // JS loose equality coerces (`false == 0`, `"0" == 0`, `[] == 0`), so it is not
    // semantically interchangeable with strict equality except for the intentionally modeled
    // nullish check (`x == null`) that backs `??`.
    let i = Interner::new();
    let loose_zero = "function f(x) { return x == 0; }";
    let loose_zero_swapped = "function g(y) { return 0 == y; }";
    let strict_zero = "function h(x) { return x === 0; }";
    assert_eq!(
        value_fp(&i, loose_zero, Lang::JavaScript),
        value_fp(&i, loose_zero_swapped, Lang::JavaScript),
        "loose equality itself is symmetric and should still converge across operand order"
    );
    assert_ne!(
        value_fp(&i, loose_zero, Lang::JavaScript),
        value_fp(&i, strict_zero, Lang::JavaScript),
        "loose `x == 0` must not merge with strict `x === 0`"
    );

    let loose_ne_zero = "function f(x) { return x != 0; }";
    let strict_ne_zero = "function h(x) { return x !== 0; }";
    assert_ne!(
        value_fp(&i, loose_ne_zero, Lang::JavaScript),
        value_fp(&i, strict_ne_zero, Lang::JavaScript),
        "loose `x != 0` must not merge with strict `x !== 0`"
    );

    let nullish = "function f(x, d) { return x ?? d; }";
    let loose_null = "function g(x, d) { return x == null ? d : x; }";
    let strict_null = "function h(x, d) { return x === null ? d : x; }";
    assert_eq!(
        value_fp(&i, nullish, Lang::JavaScript),
        value_fp(&i, loose_null, Lang::JavaScript),
        "loose `== null` remains the modeled nullish check"
    );
    assert_ne!(
        value_fp(&i, loose_null, Lang::JavaScript),
        value_fp(&i, strict_null, Lang::JavaScript),
        "strict null equality must stay separate from the nullish loose check"
    );
}

#[test]
fn js_instanceof_stays_distinct_from_equality() {
    // `instanceof` tests a value's prototype chain. It is not equality:
    // `[] instanceof Array` is true, while `[] === Array` is false.
    let i = Interner::new();
    let membership = "function f(x, C) { return x instanceof C; }";
    let renamed_membership = "function h(value, Type) { return value instanceof Type; }";
    let equality = "function g(x, C) { return x === C; }";
    assert_eq!(
        value_fp(&i, membership, Lang::JavaScript),
        value_fp(&i, renamed_membership, Lang::JavaScript),
        "`instanceof` should still converge with the same directional source surface"
    );
    assert_ne!(
        value_fp(&i, membership, Lang::JavaScript),
        value_fp(&i, equality, Lang::JavaScript),
        "`x instanceof C` must not merge with `x === C`"
    );

    let not_membership = "function f(x, C) { return !(x instanceof C); }";
    let not_renamed_membership = "function h(value, Type) { return !(value instanceof Type); }";
    let strict_inequality = "function g(x, C) { return x !== C; }";
    assert_eq!(
        value_fp(&i, not_membership, Lang::JavaScript),
        value_fp(&i, not_renamed_membership, Lang::JavaScript),
        "negated `instanceof` should still converge with the same source surface"
    );
    assert_ne!(
        value_fp(&i, not_membership, Lang::JavaScript),
        value_fp(&i, strict_inequality, Lang::JavaScript),
        "`!(x instanceof C)` must not merge with `x !== C`"
    );
}

#[test]
fn js_relational_comparison_stays_distinct_from_typed_numeric_comparison() {
    // JS relational comparison is not purely numeric for untyped operands:
    // `"2" < "10"` is false because both operands are strings, while `2 < 10` is true.
    let i = Interner::new();
    let js_lt = "function f(a, b) { return a < b; }";
    let ts_lt = "function g(a: number, b: number): boolean { return a < b; }";
    let ts_gt = "function h(a: number, b: number): boolean { return b > a; }";
    assert_eq!(
        value_fp(&i, ts_lt, Lang::TypeScript),
        value_fp(&i, ts_gt, Lang::TypeScript),
        "typed numeric TS comparison should keep primitive comparison laws"
    );
    assert_ne!(
        value_fp(&i, js_lt, Lang::JavaScript),
        value_fp(&i, ts_lt, Lang::TypeScript),
        "untyped JS `<` must not merge with typed numeric `<`"
    );

    let not_lt = "function f(a, b) { return !(a < b); }";
    let ge = "function g(a, b) { return a >= b; }";
    assert_ne!(
        value_fp(&i, not_lt, Lang::JavaScript),
        value_fp(&i, ge, Lang::JavaScript),
        "JS `!(a < b)` must not merge with `a >= b` because NaN makes them differ"
    );

    let py_not_lt = "def f(a, b):\n    return not (a < b)\n";
    let py_ge = "def g(a, b):\n    return a >= b\n";
    assert_ne!(
        value_fp(&i, py_not_lt, Lang::Python),
        value_fp(&i, py_ge, Lang::Python),
        "Python `not (a < b)` must not merge with `a >= b` because NaN makes them differ"
    );
    let py_int_not_lt = "def f(a: int, b: int):\n    return not (a < b)\n";
    let py_int_ge = "def g(a: int, b: int):\n    return a >= b\n";
    assert_eq!(
        value_fp(&i, py_int_not_lt, Lang::Python),
        value_fp(&i, py_int_ge, Lang::Python),
        "integer-proven Python order negation can use total-order duals"
    );
}

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

#[test]
fn reinvented_helper_containment_fires_and_excludes_callers() {
    // The containment channel: a function that REIMPLEMENTS an existing pure helper
    // inline (without calling it) is reported, with the helper's whole-body value hash
    // matched as an interior sub-DAG anchor of the container. A function that CALLS the
    // helper — whose fingerprint contains the same hash via interprocedural inlining —
    // must NOT be reported: calling is the fix, not the smell.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    // File 0: a straight-line pure helper big enough for the helper floor, plus a
    // caller of it (inlined → contains the helper's value graph → must be excluded).
    let helper_and_caller = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n\nfunction use(x, y) {\n    return big(x, y) + 1\n}\n";
    // File 1: a function that reimplements `big`'s computation inline and does more.
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let il0 = nose_frontend::lower_source(
        FileId(0),
        "a.js",
        helper_and_caller.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let il1 = nose_frontend::lower_source(
        FileId(1),
        "b.js",
        reinventor.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    let findings = nose_detect::reinvented_helpers(&units);
    assert_eq!(
        findings.len(),
        1,
        "exactly the reimplementation (not the caller) must be reported",
    );
    let f = &findings[0];
    assert_eq!(
        (f.container_file.as_str(), f.container_name.as_deref()),
        ("b.js", Some("manual")),
        "the container is the function reimplementing the helper",
    );
    assert_eq!(
        (f.helper_file.as_str(), f.helper_name.as_deref()),
        ("a.js", Some("big")),
        "the helper is the existing function being reinvented",
    );
}

#[test]
fn reinvented_helper_skips_effectful_and_guard_mismatched_helpers() {
    // An effectful helper (its sink profile is not pure-single-return) never becomes a
    // containment helper — replacing inline code with a call would ADD its effect.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let eff_helper = "function bigLog(x, y, log) {\n    log.push(x)\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let il0 = nose_frontend::lower_source(
        FileId(0),
        "a.js",
        eff_helper.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let il1 = nose_frontend::lower_source(
        FileId(1),
        "b.js",
        reinventor.as_bytes(),
        Lang::JavaScript,
        &i,
    )
    .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    assert!(
        nose_detect::reinvented_helpers(&units).is_empty(),
        "an effectful helper must not produce containment findings",
    );
}

#[test]
fn decorated_function_callers_fail_closed_coevo_s6_s2a() {
    // coevo series 6, S2-A: Python decorators are dropped in lowering, so a decorated
    // helper's runtime binding is `decorator(f)`, not the bare body. The fix records a
    // SourceFactKind::Binding(DecoratedDefinition) fact so the decorated def gets no
    // DirectFunction evidence — its callers fall back to an opaque call and are NOT
    // admitted to the exact `semantic` channel (exact_safe=false), so they can never be
    // reported as an "exact behavior match" that hides the decorator's effect. A caller
    // of a PLAIN helper still inlines and stays exact_safe.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let caller_exact_safe = |src: &str| -> bool {
        let il = nose_frontend::lower_source(FileId(0), "t.py", src.as_bytes(), Lang::Python, &i)
            .unwrap();
        nose_detect::units_of_file(&il, &i, &opts)
            .iter()
            .find(|u| u.name.as_deref() == Some("caller"))
            .expect("caller unit")
            .exact_safe
    };
    let decorated = "def double(f):\n    return lambda x: f(x) * 2\n\n@double\ndef helper(x):\n    return x * 5 + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    let plain =
        "def helper(x):\n    return x * 5 + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    assert!(
        !caller_exact_safe(decorated),
        "a caller of a DECORATED helper must not be exact-safe (no inline, fail closed)",
    );
    assert!(
        caller_exact_safe(plain),
        "a caller of a PLAIN helper still inlines and stays exact-safe (no recall loss)",
    );
}

#[test]
fn reinvented_helper_excludes_caller_via_inlined_span_coevo_s6_s2() {
    // coevo series 6, S3-2: a pure caller of a function that inline-reinvents the helper
    // must NOT itself be reported (the called-helper record is one call level deep). The
    // span-containment gate rejects it: the matched anchor's real span lies outside the
    // caller's own line range (it belongs to the inlined callee).
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    // `reinventor` reimplements `big` inline; `passThrough` merely CALLS reinventor.
    let src = "function reinventor(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) + 1\n}\n\nfunction passThrough(x, y) {\n    return reinventor(x, y) + 2\n}\n";
    let il0 =
        nose_frontend::lower_source(FileId(0), "h.js", helper.as_bytes(), Lang::JavaScript, &i)
            .unwrap();
    let il1 = nose_frontend::lower_source(FileId(1), "r.js", src.as_bytes(), Lang::JavaScript, &i)
        .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    let findings = nose_detect::reinvented_helpers(&units);
    assert!(
        findings
            .iter()
            .all(|f| f.container_name.as_deref() != Some("passThrough")),
        "a caller of an inline-reinventor must not be reported as reinventing the helper, got {:?}",
        findings
            .iter()
            .map(|f| f.container_name.clone())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn reinvented_helper_rejects_bound_blind_fold_coevo_s6_s3() {
    // coevo series 6, S3-3: an indexed `while i < n` fold absorbs the bound into a
    // pointer-length contract, dropping it from the value hash, so a fold over a
    // DIFFERENT bound must NOT be reported as containment (it computes a different value).
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "def poly_sum(xs, n):\n    total = 0\n    i = 0\n    while i < n:\n        total = total + xs[i] * xs[i] + 3 * xs[i] + 7\n        i = i + 1\n    return total\n";
    let container = "def poly_partial(xs, n, k):\n    total = 0\n    i = 0\n    while i < n - 1:\n        total = total + xs[i] * xs[i] + 3 * xs[i] + 7\n        i = i + 1\n    return total * k + 9\n";
    let il0 = nose_frontend::lower_source(FileId(0), "a.py", helper.as_bytes(), Lang::Python, &i)
        .unwrap();
    let il1 =
        nose_frontend::lower_source(FileId(1), "b.py", container.as_bytes(), Lang::Python, &i)
            .unwrap();
    let mut units = nose_detect::units_of_file(&il0, &i, &opts);
    units.extend(nose_detect::units_of_file(&il1, &i, &opts));
    assert!(
        nose_detect::reinvented_helpers(&units).is_empty(),
        "a fold whose bound the value hash drops must not match a different-bound fold",
    );
}

#[test]
fn keyword_argument_mapping_is_by_name_not_position_issue_301() {
    // #301 (coevo series 6, S1): Python keyword arguments lower to a KwArg node carrying
    // the name, so a call's argument identity is BY NAME. Two callers passing the SAME
    // (name -> value) mapping in different orders converge; two passing DIFFERENT
    // mappings do not — the inline binds `helper(b=p, a=q)` to the right parameters.
    let i = Interner::new();
    let helper = "def helper(a, b):\n    base = a - b\n    return base * 3 + a\n";
    let call_ab = "def run(p, q):\n    return helper(a=p, b=q)\n";
    let call_ba_same = "def run(p, q):\n    return helper(b=q, a=p)\n"; // same mapping, reordered
    let call_ba_diff = "def run(p, q):\n    return helper(b=p, a=q)\n"; // different mapping
    let fp = |c: &str| value_fp_named(&i, &format!("{helper}\n{c}"), Lang::Python, "run");
    assert_eq!(
        fp(call_ab),
        fp(call_ba_same),
        "same keyword->value mapping in different order must converge",
    );
    assert_ne!(
        fp(call_ab),
        fp(call_ba_diff),
        "different keyword->value mapping must NOT converge (the #301 false merge)",
    );
}

#[test]
fn keyword_argument_oracle_binds_by_name_issue_301() {
    // The verify oracle must bind keyword args by name too, so it neither mis-binds a
    // reordered keyword call (a silent false merge) nor needlessly excludes it. The
    // differently-mapped callers compute different values on the same inputs.
    use nose_normalize::{run_unit, Value};
    let i = Interner::new();
    let src_diff = "def helper(a, b):\n    return (a - b) * 3 + a\n\ndef run(p, q):\n    return helper(b=p, a=q)\n";
    let il = nose_frontend::lower_source(FileId(0), "t.py", src_diff.as_bytes(), Lang::Python, &i)
        .unwrap();
    let n = normalize(&il, &i, &NormalizeOptions::default());
    let run = n
        .units
        .iter()
        .find(|u| u.name.is_some_and(|s| i.resolve(s) == "run"))
        .map(|u| u.root)
        .expect("run unit");
    // run(p=1, q=2) calls helper(b=1, a=2) → (a-b)*3+a = (2-1)*3+2 = 5.
    assert_eq!(
        run_unit(&n, &i, run, &[Value::Int(1), Value::Int(2)])
            .expect("interpretable")
            .ret,
        Value::Int(5),
        "the oracle must bind helper(b=p, a=q) by name: a=q=2, b=p=1",
    );
}

#[test]
fn global_reassigned_function_fails_closed_issue_302() {
    // #302: a module function reassigned via `global name; name = ...` from inside another
    // function no longer binds its `def` body, so its callers must NOT inline that body
    // (they would false-merge across files that reassign it differently). A LOCAL
    // assignment to the same name (no `global`) is a different binding and must NOT gate
    // the function — the precise distinction the series-6 reassigned-anywhere predicate
    // could not draw (it over-fired). Measured via exact-safety: an inlined caller is
    // exact-safe; an opaque (un-inlined) one is not.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let caller_exact_safe = |src: &str| -> bool {
        let il = nose_frontend::lower_source(FileId(0), "t.py", src.as_bytes(), Lang::Python, &i)
            .unwrap();
        nose_detect::units_of_file(&il, &i, &opts)
            .iter()
            .find(|u| u.name.as_deref() == Some("caller"))
            .expect("caller unit")
            .exact_safe
    };
    let reassigned = "def helper(x):\n    return x * 5 + 1\n\ndef setup():\n    global helper\n    helper = other\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    let local_shadow = "def helper(x):\n    return x * 5 + 1\n\ndef elsewhere():\n    helper = 5\n    return helper + 1\n\ndef caller(a):\n    return helper(a) * 10 + a\n";
    assert!(
        !caller_exact_safe(reassigned),
        "a caller of a `global`-reassigned function must not be exact-safe (fail closed)",
    );
    assert!(
        caller_exact_safe(local_shadow),
        "a local `helper = 5` (no `global`) must NOT gate the function — caller still inlines",
    );
}

#[test]
fn splat_argument_is_distinct_from_plain_argument_coevo_s7_s1() {
    // coevo series 7, S1: `f(*args)` unpacks an iterable into positional params and
    // `f(**d)` unpacks a mapping into keywords — neither is `f(arg)`. The frontend used
    // to strip the splat, so `stats(*xs)` lowered identically to `stats(xs)` and the two
    // false-merged (different on `[[1,2,3]]`: len 3 vs 1). A `Splat` node now keeps them
    // distinct.
    let i = Interner::new();
    let helper = "def stats(a):\n    total = len(a)\n    return total * total + total\n";
    let via_splat = "def via(xs):\n    return stats(*xs)\n";
    let via_plain = "def via(xs):\n    return stats(xs)\n";
    let fp = |c: &str| value_fp_named(&i, &format!("{helper}\n{c}"), Lang::Python, "via");
    assert_ne!(
        fp(via_splat),
        fp(via_plain),
        "a `*args` spread must not fingerprint as a plain positional argument",
    );
    let via_kwsplat = "def via(d):\n    return stats(**d)\n";
    assert_ne!(
        fp(via_kwsplat),
        fp(via_plain),
        "a `**kwargs` spread must not fingerprint as a plain positional argument",
    );
    assert_ne!(
        fp(via_splat),
        fp(via_kwsplat),
        "`*args` and `**kwargs` spreads must stay distinct from each other",
    );
}

#[test]
fn global_rebind_recorded_for_all_assignment_forms_coevo_s7_s2() {
    // coevo series 7, S2: the #302 fix recorded `ModuleRebind` only for a plain
    // single-identifier `global helper; helper = x`. Tuple-unpack, aug-assign, and walrus
    // all lower to an `Assign` too and must also withhold the rebound function. Measured
    // via exact-safety: a caller of a rebound function is opaque (not exact-safe).
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let caller_exact_safe = |src: &str| -> bool {
        let il = nose_frontend::lower_source(FileId(0), "t.py", src.as_bytes(), Lang::Python, &i)
            .unwrap();
        nose_detect::units_of_file(&il, &i, &opts)
            .iter()
            .find(|u| u.name.as_deref() == Some("caller"))
            .expect("caller unit")
            .exact_safe
    };
    let helper = "def helper(x):\n    return x * 5 + 1\n";
    let caller = "def caller(a):\n    return helper(a) * 10 + a\n";
    let tuple = format!(
        "{helper}\ndef setup():\n    global helper\n    helper, spare = other, 0\n\n{caller}"
    );
    let aug = format!("{helper}\ndef setup():\n    global helper\n    helper += 1\n\n{caller}");
    let walrus =
        format!("{helper}\ndef setup():\n    global helper\n    (helper := other)\n\n{caller}");
    for (label, src) in [
        ("tuple-unpack", &tuple),
        ("aug-assign", &aug),
        ("walrus", &walrus),
    ] {
        assert!(
            !caller_exact_safe(src),
            "a caller of a `global`-rebound function ({label}) must not be exact-safe",
        );
    }
    // Precise: a LOCAL `helper = 5` (no `global`) still leaves the function inlinable.
    let local =
        format!("{helper}\ndef elsewhere():\n    helper = 5\n    return helper + 1\n\n{caller}");
    assert!(
        caller_exact_safe(&local),
        "a local shadow (no `global`) must NOT withhold the function",
    );
}

#[test]
fn effectful_keyword_reorder_stays_distinct_coevo_s7_s3() {
    // coevo series 7, S3: the keyword name-sort (#304) is sound only for effect-free
    // values — Python evaluates args in SOURCE order, so reordering effectful keyword
    // values changes the effect/exception order. With a call-valued keyword the two
    // orderings must stay distinct; with pure values they still converge.
    let i = Interner::new();
    let eff_a = "def use(x, y):\n    return combine(a=sideA(x), b=sideB(y))\n";
    let eff_b = "def use(x, y):\n    return combine(b=sideB(y), a=sideA(x))\n";
    assert_ne!(
        value_fp_named(&i, eff_a, Lang::Python, "use"),
        value_fp_named(&i, eff_b, Lang::Python, "use"),
        "reordered EFFECTFUL keyword values must not converge (eval order differs)",
    );
    let pure_a = "def use(p, q):\n    return combine(a=p, b=q)\n";
    let pure_b = "def use(p, q):\n    return combine(b=q, a=p)\n";
    assert_eq!(
        value_fp_named(&i, pure_a, Lang::Python, "use"),
        value_fp_named(&i, pure_b, Lang::Python, "use"),
        "reordered PURE keyword values still converge (no observable order)",
    );
}

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
fn reinvented_helper_flags_test_container_for_default_exclusion() {
    // The promotion field audit (2026-06-13) excludes test-container findings from the
    // bare-default surface (a test asserting the helper's value as a literal is circular
    // to "fix"). The `container_in_test` flag drives that — set when the container file
    // is a test path, regardless of the helper's location.
    let i = Interner::new();
    let opts = DetectOptions {
        min_lines: 1,
        min_tokens: 1,
        ..Default::default()
    };
    let helper = "function big(x, y) {\n    return ((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))\n}\n";
    let reinventor = "function manual(x, y) {\n    return (((x * 2 + 3) * (x - 4)) / ((x + 5) * (y - 7) + (y * y + 11))) * 7\n}\n";
    let run = |container_path: &str| -> Option<bool> {
        let il0 = nose_frontend::lower_source(
            FileId(0),
            "helper.js",
            helper.as_bytes(),
            Lang::JavaScript,
            &i,
        )
        .unwrap();
        let il1 = nose_frontend::lower_source(
            FileId(1),
            container_path,
            reinventor.as_bytes(),
            Lang::JavaScript,
            &i,
        )
        .unwrap();
        let mut units = nose_detect::units_of_file(&il0, &i, &opts);
        units.extend(nose_detect::units_of_file(&il1, &i, &opts));
        nose_detect::reinvented_helpers(&units)
            .first()
            .map(|f| f.container_in_test)
    };
    assert_eq!(
        run("src/math.js"),
        Some(false),
        "a prod-path container is not flagged in_test"
    );
    assert_eq!(
        run("test/math.test.js"),
        Some(true),
        "a test-path container is flagged in_test"
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

// ----- CSS (declarative) exact-channel semantics -----
//
// CSS units are NOT lowered through the imperative value graph; their exact fingerprint
// is the canonical multiset of their declarations (see `nose-normalize::css`). These
// lock in the soundness-relevant behavior: a duplicated declaration block is a clone
// regardless of selector or declaration order, but a cascade-affecting or value
// difference must NOT merge.

/// The declarative value fingerprint of the first `CssRule` unit in `src`.
fn css_fp(interner: &Interner, src: &str) -> Vec<u64> {
    let il = nose_frontend::lower_source(FileId(0), "t.css", src.as_bytes(), Lang::Css, interner)
        .unwrap();
    let n = normalize(&il, interner, &NormalizeOptions::default());
    let root = n
        .units
        .iter()
        .find(|u| n.node(u.root).kind == nose_il::NodeKind::CssRule)
        .map(|u| u.root)
        .expect("expected a CssRule unit");
    nose_normalize::value_fingerprint(&n, root, interner)
}

#[test]
fn css_declaration_order_independent_and_selector_independent() {
    // A duplicated declaration block is the canonical CSS clone — same declarations,
    // different selector AND different source order, must converge.
    let i = Interner::new();
    let a = ".btn { display: flex; align-items: center; gap: 8px; padding: 12px; }";
    let b = ".cta { padding: 12px; gap: 8px; align-items: center; display: flex; }";
    assert_eq!(
        css_fp(&i, a),
        css_fp(&i, b),
        "same declarations across selectors/orders must converge",
    );
}

#[test]
fn css_differing_value_does_not_converge() {
    // One value differs (12px vs 16px) → different computed style → must NOT merge.
    let i = Interner::new();
    let a = ".btn { display: flex; align-items: center; gap: 8px; padding: 12px; }";
    let c = ".btn { display: flex; align-items: center; gap: 8px; padding: 16px; }";
    assert_ne!(
        css_fp(&i, a),
        css_fp(&i, c),
        "a differing value must not merge"
    );
}

#[test]
fn css_cascade_last_wins_is_order_sensitive_for_repeated_properties() {
    // Declaration order is irrelevant EXCEPT for a repeated property, where the cascade
    // keeps the last — so these two compute different colors and must NOT merge.
    let i = Interner::new();
    let red_then_blue = ".x { color: red; color: blue; }";
    let blue_then_red = ".x { color: blue; color: red; }";
    assert_ne!(
        css_fp(&i, red_then_blue),
        css_fp(&i, blue_then_red),
        "repeated-property cascade (last-wins) must stay order-sensitive",
    );
}

#[test]
fn css_value_order_within_a_declaration_is_significant() {
    // Across declarations order is free, but WITHIN a declaration token order matters:
    // `margin: 1px 2px` (top/bottom 1px, left/right 2px) ≠ `margin: 2px 1px`.
    let i = Interner::new();
    let a = ".a { margin: 1px 2px; color: red; display: flex; gap: 4px; }";
    let b = ".b { margin: 2px 1px; color: red; display: flex; gap: 4px; }";
    assert_ne!(
        css_fp(&i, a),
        css_fp(&i, b),
        "value-token order within a declaration must be significant",
    );
}

#[test]
fn css_at_rule_context_blocks_merge_with_unconditional_rule() {
    // A `@media`-scoped rule and an identical unconditional rule compute different
    // styles (one is conditional) — the at-rule context must keep them apart.
    let i = Interner::new();
    let scoped =
        "@media (max-width: 600px) { .btn { display: flex; align-items: center; gap: 8px; padding: 12px; } }";
    let plain = ".btn { display: flex; align-items: center; gap: 8px; padding: 12px; }";
    assert_ne!(
        css_fp(&i, scoped),
        css_fp(&i, plain),
        "@media-scoped rule must not merge with an unconditional one",
    );
}

#[test]
fn css_computed_equivalence_converges_color_shorthand_unit() {
    // The deep CSS equivalence: textually different but computed-identical rules merge.
    // #ffffff≡white, margin:0 0 0 0≡margin:0, #ff0000≡red, padding 4-val≡2-val collapse.
    let i = Interner::new();
    let a = ".btn { color: #ffffff; margin: 0 0 0 0; background: #ff0000; padding: 10px 20px 10px 20px; }";
    let b = ".cta { color: white; margin: 0; background: red; padding: 10px 20px; }";
    assert_eq!(
        css_fp(&i, a),
        css_fp(&i, b),
        "computed-equivalent rules (color/shorthand/unit) must converge",
    );
}

#[test]
fn css_distinct_colors_do_not_converge() {
    // Soundness hard negative: different colors must never merge, however close.
    let i = Interner::new();
    let red = ".a { color: #ff0000; display: block; padding: 1px; }";
    let blue = ".b { color: #0000ff; display: block; padding: 1px; }";
    assert_ne!(
        css_fp(&i, red),
        css_fp(&i, blue),
        "#f00 vs #00f must stay distinct"
    );
}

#[test]
fn css_shorthand_longhand_cascade_is_order_sensitive() {
    // Soundness: a shorthand and one of its longhands in the same rule cascade by
    // ORDER — `margin: 0; margin-top: 5px` (top=5px) ≠ `margin-top: 5px; margin: 0`
    // (top=0). The order-independent multiset would false-merge these; the
    // cascade-ambiguity guard must keep them apart.
    let i = Interner::new();
    let a = ".a { margin: 0; margin-top: 5px; display: block; color: red; }";
    let b = ".b { margin-top: 5px; margin: 0; display: block; color: red; }";
    assert_ne!(
        css_fp(&i, a),
        css_fp(&i, b),
        "shorthand/longhand cascade must stay order-sensitive",
    );
}

#[test]
fn css_at_rule_full_condition_distinguishes_blocks() {
    // Soundness (regression: a bulma `@container` false merge). The WHOLE at-rule head
    // is the context — two blocks differing only in their condition (and selectors,
    // which the fingerprint excludes) compute different styles and must NOT merge.
    let i = Interner::new();
    let a = "@container foo (max-width: 768px) { .a > .grid { --c: 1; } .a2 > .grid { --c: 2; } }";
    let b = "@container foo (min-width: 769px) { .b > .grid { --c: 1; } .b2 > .grid { --c: 2; } }";
    assert_ne!(
        css_fp(&i, a),
        css_fp(&i, b),
        "at-rule blocks with different conditions must not merge",
    );
}

#[test]
fn css_independent_non_overlapping_properties_stay_order_free() {
    // The guard must NOT over-fire: non-overlapping properties (no shorthand/longhand
    // relation) remain order-independent.
    let i = Interner::new();
    let a = ".a { color: red; display: block; padding: 4px; gap: 2px; }";
    let b = ".b { gap: 2px; padding: 4px; display: block; color: red; }";
    assert_eq!(
        css_fp(&i, a),
        css_fp(&i, b),
        "independent properties must remain order-free",
    );
}

// ----- HTML (declarative markup) exact-channel semantics -----
//
// Markup units are fingerprinted by their canonical rendered DOM (see
// `nose-normalize::html`): attribute order/boolean-form/whitespace/class-set are
// normalized, but tag/structure/text/value differences are kept distinct.

/// The declarative DOM fingerprint of the first top-level `HtmlElement` in `src`.
fn html_fp(interner: &Interner, src: &str) -> Vec<u64> {
    let ils = nose_frontend::lower_source_regions(
        FileId(0),
        "t.html",
        src.as_bytes(),
        Lang::Html,
        interner,
    );
    let markup = ils
        .iter()
        .find(|il| il.meta.lang == Lang::Html)
        .expect("a markup (html) region");
    let n = normalize(markup, interner, &NormalizeOptions::default());
    let root = n
        .children(n.root)
        .iter()
        .copied()
        .find(|&c| n.node(c).kind == nose_il::NodeKind::HtmlElement)
        .expect("a top-level HtmlElement unit");
    nose_normalize::value_fingerprint(&n, root, interner)
}

#[test]
fn html_same_dom_converges_under_attr_order_boolean_whitespace_class_set() {
    let i = Interner::new();
    let a = r#"<div class="card x"><img src="a.png" alt="p"><button type="button" disabled>Go</button></div>"#;
    // attrs reordered, boolean form `disabled=""`, class tokens reordered, extra whitespace
    let b = "<div class=\"x card\">\n  <img alt=\"p\"   src=\"a.png\">\n  <button disabled=\"\" type=\"button\">Go</button>\n</div>";
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, b),
        "same rendered DOM must converge"
    );
}

#[test]
fn html_text_and_value_differences_do_not_converge() {
    let i = Interner::new();
    let a = r#"<div class="card"><h3>Title</h3><a href="/a">Link</a></div>"#;
    let b = r#"<div class="card"><h3>Other</h3><a href="/a">Link</a></div>"#; // different text
    let c = r#"<div class="card"><h3>Title</h3><a href="/b">Link</a></div>"#; // different href
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "different text must not merge"
    );
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, c),
        "different attr value must not merge"
    );
}

#[test]
fn html_child_order_is_significant() {
    let i = Interner::new();
    let a = r#"<ul class="m"><li>one</li><li>two</li></ul>"#;
    let b = r#"<ul class="m"><li>two</li><li>one</li></ul>"#;
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "DOM child order must be significant"
    );
}

// ---- cross-dialect markup (HTML/Vue/Svelte/JSX/TSX) ---------------------------------
// The exact (semantic) fingerprint of a markup unit must stay FAITHFUL to the rendered
// DOM even as the five dialects' idioms (control flow, directives, holes) are normalized
// into the common IL — so the moat (equal fingerprint ⟹ equal rendered DOM) still holds.

/// Exact DOM fingerprint of the OUTERMOST markup element in `src`, lowered as `lang`
/// (works for `.vue`/`.svelte` markup AND JSX/TSX, whose markup lives in the JS region).
fn markup_fp(i: &Interner, path: &str, lang: Lang, src: &str) -> Vec<u64> {
    fn outer(n: &nose_il::Il, node: NodeId) -> Option<NodeId> {
        if n.node(node).kind == nose_il::NodeKind::HtmlElement {
            return Some(node);
        }
        n.children(node).iter().find_map(|&c| outer(n, c))
    }
    let ils = nose_frontend::lower_source_regions(FileId(0), path, src.as_bytes(), lang, i);
    for il in &ils {
        let n = normalize(il, i, &NormalizeOptions::default());
        if let Some(root) = outer(&n, n.root) {
            return nose_normalize::value_fingerprint(&n, root, i);
        }
    }
    panic!("no markup unit lowered from {path}");
}

#[test]
fn markup_loop_does_not_merge_with_single_element() {
    // SOUNDNESS: a `{#each}`/`v-for`/`.map` repeat renders 0..N elements — it must NEVER
    // share a fingerprint with a single static element (the HtmlControl wrapper keeps it
    // distinct), or nose would claim a loop and one element have equal rendered DOM.
    let i = Interner::new();
    let loop_ = r#"<ul class="m">{#each items as x}<li class="r">row</li>{/each}</ul>"#;
    let single = r#"<ul class="m"><li class="r">row</li></ul>"#;
    assert_ne!(
        html_fp(&i, loop_),
        html_fp(&i, single),
        "a repeat must not exact-merge with a single element"
    );
}

#[test]
fn markup_repeat_and_conditional_controls_are_distinct() {
    // A repeat (`{#each}`) and a conditional (`{#if}`) are different rendered behavior.
    let i = Interner::new();
    let each = r#"<ul class="m">{#each xs as x}<li>z</li>{/each}</ul>"#;
    let if_ = r#"<ul class="m">{#if cond}<li>z</li>{/if}</ul>"#;
    assert_ne!(
        html_fp(&i, each),
        html_fp(&i, if_),
        "repeat and conditional controls must not merge"
    );
}

#[test]
fn markup_control_converges_vue_directive_and_svelte_block() {
    // CROSS-DIALECT (sound positive): a Vue `v-for` element and the equivalent Svelte
    // `{#each}` block render the SAME DOM (a repeat of an identical template) → converge.
    let i = Interner::new();
    let vue = r#"<ul class="list"><li class="item" v-for="x in xs" :key="x.id">row text</li></ul>"#;
    let svelte =
        r#"<ul class="list">{#each xs as x (x.id)}<li class="item">row text</li>{/each}</ul>"#;
    assert_eq!(
        html_fp(&i, vue),
        html_fp(&i, svelte),
        "v-for and {{#each}} of the same template must converge"
    );
}

#[test]
fn markup_bound_attribute_keeps_its_expression_distinct() {
    // SOUNDNESS: a directive-bound attribute keeps its expression verbatim — two different
    // bound expressions must not be claimed equal (no hole-collapse on the exact channel).
    let i = Interner::new();
    let a = r#"<img class="a" :src="hero" :alt="name">"#;
    let b = r#"<img class="a" :src="thumb" :alt="name">"#;
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "different bound expressions must not merge on the exact channel"
    );
}

#[test]
fn markup_jsx_converges_with_html_on_identical_static_dom() {
    // CROSS-DIALECT (sound positive): a JSX element and a hand-written HTML element with
    // the SAME rendered DOM (className→class, static text/attrs identical) converge — and
    // JSX markup now lives in the common declarative IL, not an imperative Call-tree.
    let i = Interner::new();
    let jsx = r#"export function Nav(){return <li className="nav-item"><a className="nav-link" href="/login">Sign in</a></li>;}"#;
    let html = r#"<li class="nav-item"><a class="nav-link" href="/login">Sign in</a></li>"#;
    assert_eq!(
        markup_fp(&i, "n.jsx", Lang::JavaScript, jsx),
        markup_fp(&i, "n.html", Lang::Html, html),
        "JSX and HTML with identical rendered DOM must converge"
    );
}

#[test]
fn markup_jsx_map_does_not_merge_with_single_element() {
    // SOUNDNESS (JSX side): `{xs.map(x => <li/>)}` is a repeat — distinct from a single li.
    let i = Interner::new();
    let mapped = r#"export function L(){return <ul className="m">{xs.map(x => <li className="r">row</li>)}</ul>;}"#;
    let single =
        r#"export function L(){return <ul className="m"><li className="r">row</li></ul>;}"#;
    assert_ne!(
        markup_fp(&i, "a.jsx", Lang::JavaScript, mapped),
        markup_fp(&i, "b.jsx", Lang::JavaScript, single),
        "a JSX .map repeat must not merge with a single element"
    );
}

#[test]
fn html_inline_style_is_computed_canonicalized() {
    // Inline `style="…"` reuses the CSS computed-style canon: order-independent,
    // color/shorthand/unit normalized.
    let i = Interner::new();
    let a = r#"<div style="margin: 0 0 0 0; color: #ffffff; padding: 4px"><span>x</span></div>"#;
    let b = r#"<div style="color: white; padding: 4px; margin: 0"><span>x</span></div>"#;
    let c = r#"<div style="color: black; padding: 4px; margin: 0"><span>x</span></div>"#;
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, b),
        "computed-equal inline styles converge"
    );
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, c),
        "different inline style must not merge"
    );
}

#[test]
fn html_vue_directive_shorthand_canonicalizes() {
    // `:x` ≡ `v-bind:x` and `@x` ≡ `v-on:x` — same binding, different spelling.
    let i = Interner::new();
    let short = r#"<button :class="c" @click="f"><i class="ico"></i>Go</button>"#;
    let long = r#"<button v-bind:class="c" v-on:click="f"><i class="ico"></i>Go</button>"#;
    assert_eq!(
        html_fp(&i, short),
        html_fp(&i, long),
        "directive shorthand must canonicalize"
    );
}

#[test]
fn html_preformatted_whitespace_is_significant() {
    // Soundness: inside <pre>/<textarea> whitespace is preserved by the renderer, so
    // two blocks differing only in indentation render differently and must NOT merge.
    let i = Interner::new();
    let a = "<pre>fn f() {\n    return 1;\n    return 2;\n}</pre>";
    let b = "<pre>fn f() {\n  return 1;\n  return 2;\n}</pre>"; // 2-space indent
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "<pre> whitespace must be significant"
    );
    let a2 = "<pre>fn f() {\n    return 1;\n    return 2;\n}</pre>";
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, a2),
        "identical <pre> still converges"
    );
    // Outside <pre>, flow whitespace stays INsignificant.
    let p1 = "<p>hello world   again here friend</p>";
    let p2 = "<p>hello world\n  again here friend</p>";
    assert_eq!(
        html_fp(&i, p1),
        html_fp(&i, p2),
        "flow whitespace stays insignificant"
    );
}

#[test]
fn html_fingerprint_is_domain_disjoint_from_css_and_imperative() {
    let i = Interner::new();
    let html = html_fp(
        &i,
        r#"<div class="card"><h3>Title</h3><p>body text</p></div>"#,
    );
    let css = css_fp(&i, ".card { display: flex; gap: 8px; padding: 12px; }");
    let py = value_fp(&i, "def f(x):\n    return x + 5\n", Lang::Python);
    assert_ne!(html, css, "HTML and CSS fingerprints must be disjoint");
    assert_ne!(
        html, py,
        "HTML and imperative fingerprints must be disjoint"
    );
}

#[test]
fn css_fingerprint_is_domain_disjoint_from_imperative() {
    // Cross-domain false-merge guard: a CSS fingerprint must never equal an imperative
    // one, so the (language-blind) exact channel can never merge CSS with code.
    let i = Interner::new();
    let css = css_fp(
        &i,
        ".a { display: flex; align-items: center; gap: 8px; padding: 12px; }",
    );
    let py = value_fp(&i, "def f(x):\n    return x + 5\n", Lang::Python);
    assert_ne!(css, py, "CSS and imperative fingerprints must be disjoint");
}

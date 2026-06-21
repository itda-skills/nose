use super::*;

#[test]
fn receiver_membership_converges_cross_language() {
    let i = Interner::new();
    let py = "def f(lookup, other_lookup, key, other):\n    return key in lookup\n";
    let py_method =
        "def f(lookup, other_lookup, key, other):\n    return lookup.__contains__(key)\n";
    let py_keys_in = "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in lookup.keys()\n";
    let py_keys_contains = "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return lookup.keys().__contains__(key)\n";
    let go = "package p\n\nfunc F(lookup map[string]string, otherLookup map[string]string, key string, other string) bool { _, ok := lookup[key]; return ok }\n";
    let java = "import java.util.Map;\n\nclass C { static boolean f(Map<String, String> lookup, Map<String, String> other_lookup, String key, String other) { return lookup.containsKey(key); } }\n";
    let java_key_set = "import java.util.Map;\n\nclass C { static boolean f(Map<String, String> lookup, Map<String, String> other_lookup, String key, String other) { return lookup.keySet().contains(key); } }\n";
    let rust = "use std::collections::HashMap;\n\npub fn f(lookup: &HashMap<String, String>, other_lookup: &HashMap<String, String>, key: &str, other: &str) -> bool { lookup.contains_key(key) }\n";
    let rust_get = "use std::collections::HashMap;\n\npub fn f(lookup: &HashMap<String, String>, other_lookup: &HashMap<String, String>, key: &str, other: &str) -> bool { lookup.get(key).is_some() }\n";
    let ruby = "def f(lookup, other_lookup, key, other)\n  lookup.key?(key)\nend\n";
    let ruby_has = "def f(lookup, other_lookup, key, other)\n  lookup.has_key?(key)\nend\n";
    let ts_array_from_keys = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean { return Array.from(lookup.keys()).includes(key); }";
    let ts_direct_keys_includes = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean { return lookup.keys().includes(key); }";
    let typed_set_same_names = "function f(lookup: Set<string>, other_lookup: Set<string>, key: string, other: string): boolean { return lookup.has(key); }";

    let fp = value_fp(&i, py, Lang::Python);
    assert_ne!(fp, value_fp(&i, py_method, Lang::Python));
    assert_eq!(fp, value_fp(&i, py_keys_in, Lang::Python));
    assert_eq!(fp, value_fp(&i, py_keys_contains, Lang::Python));
    assert_eq!(fp, value_fp(&i, go, Lang::Go));
    assert_eq!(fp, value_fp(&i, java, Lang::Java));
    assert_eq!(fp, value_fp(&i, java_key_set, Lang::Java));
    assert_eq!(fp, value_fp(&i, rust, Lang::Rust));
    assert_eq!(fp, value_fp(&i, rust_get, Lang::Rust));
    assert_ne!(fp, value_fp(&i, ruby, Lang::Ruby));
    assert_ne!(fp, value_fp(&i, ruby_has, Lang::Ruby));
    assert_eq!(fp, value_fp(&i, ts_array_from_keys, Lang::TypeScript));
    assert_ne!(
        fp,
        value_fp(&i, ts_direct_keys_includes, Lang::TypeScript),
        "Map.keys() is an iterator view; direct .includes is not a proven key-view collection"
    );
    assert_ne!(fp, value_fp(&i, typed_set_same_names, Lang::TypeScript));
}

#[test]
fn receiver_membership_keeps_wrong_coordinate_boundaries() {
    let i = Interner::new();
    let py = "def f(lookup, other_lookup, key, other):\n    return key in lookup\n";
    let wrong_key =
        "def f(lookup, other_lookup, key, other):\n    return lookup.__contains__(other)\n";
    let wrong_map =
        "def f(lookup, other_lookup, key, other):\n    return other_lookup.__contains__(key)\n";
    let value_membership =
        "def f(lookup, other_lookup, key, other):\n    return key in lookup.values()\n";
    let py_keys_wrong_key = "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return other in lookup.keys()\n";
    let py_keys_wrong_map = "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in other_lookup.keys()\n";
    let py_values_view = "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in lookup.values()\n";
    let ts_array_from_keys_wrong_key = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean { return Array.from(lookup.keys()).includes(other); }";
    let ts_array_from_keys_wrong_map = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean { return Array.from(other_lookup.keys()).includes(key); }";
    let ts_array_from_values = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean { return Array.from(lookup.values()).includes(key); }";
    let ts_array_from_shadowed_array = "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string, Array: any): boolean { return Array.from(lookup.keys()).includes(key); }";

    let fp = value_fp(&i, py, Lang::Python);
    assert_ne!(fp, value_fp(&i, wrong_key, Lang::Python));
    assert_ne!(fp, value_fp(&i, wrong_map, Lang::Python));
    assert_ne!(fp, value_fp(&i, value_membership, Lang::Python));
    assert_ne!(fp, value_fp(&i, py_keys_wrong_key, Lang::Python));
    assert_ne!(fp, value_fp(&i, py_keys_wrong_map, Lang::Python));
    assert_ne!(fp, value_fp(&i, py_values_view, Lang::Python));
    assert_ne!(
        fp,
        value_fp(&i, ts_array_from_keys_wrong_key, Lang::TypeScript)
    );
    assert_ne!(
        fp,
        value_fp(&i, ts_array_from_keys_wrong_map, Lang::TypeScript)
    );
    assert_ne!(fp, value_fp(&i, ts_array_from_values, Lang::TypeScript));
    assert_ne!(
        fp,
        value_fp(&i, ts_array_from_shadowed_array, Lang::TypeScript)
    );
}

use super::*;

// Broad fixture matrix for typed TypeScript map-key membership contracts. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn query_mode_semantic_proves_typed_typescript_receiver_membership() {
    let dir = std::env::temp_dir().join(format!("nose_typed_ts_map_key_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("map_key.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in lookup\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key.java"),
        "import java.util.Map;\n\nclass C { static boolean f(Map<String, String> lookup, Map<String, String> other_lookup, String key, String other) { return lookup.containsKey(key); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return lookup.has(key);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key_python_keys_in.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in lookup.keys()\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key_python_keys_contains.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return lookup.keys().__contains__(key)\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key_ts_array_from_keys.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return Array.from(lookup.keys()).includes(key);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_key.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return lookup.has(other);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("python_keys_wrong_key.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return other in lookup.keys()\n",
    )
    .unwrap();
    fs::write(
        dir.join("python_keys_wrong_map.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in other_lookup.keys()\n",
    )
    .unwrap();
    fs::write(
        dir.join("python_values_view.py"),
        "def f(lookup: dict[str, str], other_lookup: dict[str, str], key: str, other: str) -> bool:\n    return key in lookup.values()\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_array_from_keys_wrong_key.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return Array.from(lookup.keys()).includes(other);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_array_from_keys_wrong_map.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return Array.from(other_lookup.keys()).includes(key);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_array_from_values.ts"),
        "function f(lookup: Map<string, string>, other_lookup: Map<string, string>, key: string, other: string): boolean {\n  return Array.from(lookup.values()).includes(key);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("set_negative.ts"),
        "function f(lookup: Set<string>, other_lookup: Set<string>, key: string, other: string): boolean {\n  return lookup.has(key);\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    let positive_family = semantic_families
        .iter()
        .map(serde_json::Value::to_string)
        .find(|family| {
            [
                "map_key.py",
                "map_key.java",
                "map_key.ts",
                "map_key_python_keys_in.py",
                "map_key_python_keys_contains.py",
                "map_key_ts_array_from_keys.ts",
            ]
            .iter()
            .all(|expected| family.contains(expected))
        })
        .unwrap_or_else(|| panic!("semantic mode should include typed map-key family: {semantic}"));
    for expected in [
        "map_key.py",
        "map_key.java",
        "map_key.ts",
        "map_key_python_keys_in.py",
        "map_key_python_keys_contains.py",
        "map_key_ts_array_from_keys.ts",
    ] {
        assert!(
            positive_family.contains(expected),
            "semantic mode should include typed TS Map.has map-key membership {expected}: {semantic}"
        );
    }
    for unexpected in [
        "wrong_key.ts",
        "set_negative.ts",
        "python_keys_wrong_key.py",
        "python_keys_wrong_map.py",
        "python_values_view.py",
        "ts_array_from_keys_wrong_key.ts",
        "ts_array_from_keys_wrong_map.ts",
        "ts_array_from_values.ts",
    ] {
        assert!(
            !positive_family.contains(unexpected),
            "semantic mode must preserve typed TS Map.has boundaries: {semantic}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

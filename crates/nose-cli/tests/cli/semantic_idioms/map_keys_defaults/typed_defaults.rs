use super::*;

// Broad fixture matrix for typed TypeScript map default lookup contracts. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn query_mode_semantic_proves_typed_typescript_map_default_lookup() {
    let dir =
        std::env::temp_dir().join(format!("nose_typed_ts_map_default_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("map_default.go"),
        "package p\n\nfunc F(lookup map[string]int, otherLookup map[string]int, key string, otherKey string, fallback int, otherDefault int) int { value, ok := lookup[key]; if !ok { value = fallback }; return value }\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_nullish.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  return lookup.get(key) ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_has_get.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  return lookup.has(key) ? lookup.get(key) : fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_temp_guard.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  const selected = lookup.get(key);\n  return selected === undefined ? fallback : selected;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_guard_return.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  if (lookup.has(key)) {\n    return lookup.get(key)!;\n  }\n  return fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("java_guard_return.java"),
        "import java.util.Map;\n\nclass C { static int f(Map<String, Integer> lookup, Map<String, Integer> other_lookup, String key, String other_key, int fallback, int other_default) { if (lookup.containsKey(key)) { return lookup.get(key); } return fallback; } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_wrong_key.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  return lookup.get(other_key) ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_wrong_default.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  return lookup.get(key) ?? other_default;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_wrong_map.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  return other_lookup.get(key) ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_untyped.ts"),
        "function f(lookup, other_lookup, key, other_key, fallback, other_default) {\n  return lookup.get(key) ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("ts_guard_wrong_key.ts"),
        "function f(lookup: Map<string, number>, other_lookup: Map<string, number>, key: string, other_key: string, fallback: number, other_default: number): number {\n  if (lookup.has(other_key)) {\n    return lookup.get(other_key)!;\n  }\n  return fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("java_guard_wrong_default.java"),
        "import java.util.Map;\n\nclass C { static int f(Map<String, Integer> lookup, Map<String, Integer> other_lookup, String key, String other_key, int fallback, int other_default) { if (lookup.containsKey(key)) { return lookup.get(key); } return other_default; } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_dict.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_guard_return.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    if key in lookup:\n        return lookup[key]\n    return fallback\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_mapping.py"),
        "from collections.abc import Mapping\n\ndef f(lookup: Mapping[str, int], other_lookup: Mapping[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_mutable_mapping.py"),
        "from collections.abc import MutableMapping\n\ndef f(lookup: MutableMapping[str, int], other_lookup: MutableMapping[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_mapping.py"),
        "from collections.abc import Mapping as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_mutable_mapping.py"),
        "from collections.abc import MutableMapping as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_dict.py"),
        "from typing import Dict as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_wrong_key.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(other_key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_wrong_default.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, other_default)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_wrong_map.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return other_lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_guard_wrong_map.py"),
        "def f(lookup: dict[str, int], other_lookup: dict[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    if key in other_lookup:\n        return other_lookup[key]\n    return fallback\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_untyped.py"),
        "def f(lookup, other_lookup, key, other_key, fallback, other_default):\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_wrong_key.py"),
        "from collections.abc import Mapping as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(other_key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_wrong_default.py"),
        "from collections.abc import Mapping as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, other_default)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_wrong_map.py"),
        "from collections.abc import Mapping as MapLike\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return other_lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_unresolved.py"),
        "def f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();
    fs::write(
        dir.join("py_alias_shadowed.py"),
        "from collections.abc import Mapping as MapLike\nMapLike = list\n\ndef f(lookup: MapLike[str, int], other_lookup: MapLike[str, int], key: str, other_key: str, fallback: int, other_default: int) -> int:\n    return lookup.get(key, fallback)\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    let expected = [
        "map_default.go",
        "ts_has_get.ts",
        "ts_guard_return.ts",
        "java_guard_return.java",
        "py_dict.py",
        "py_guard_return.py",
        "py_mapping.py",
        "py_mutable_mapping.py",
        "py_alias_mapping.py",
        "py_alias_mutable_mapping.py",
        "py_alias_dict.py",
    ];
    let positive_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one typed map default family: {semantic}")
        });
    let positive_text = positive_family.to_string();
    for expected in expected {
        assert!(
            positive_text.contains(expected),
            "semantic mode should include typed map default lookup {expected}: {semantic}"
        );
    }
    for unexpected in [
        // `ts_nullish` (`?? `) is nullish COALESCE and `ts_temp_guard` (`=== undefined`) is conflated
        // with `== null` — neither merges with the absence-only default family (#410, experiments §CT).
        "ts_nullish.ts",
        "ts_temp_guard.ts",
        "ts_wrong_key.ts",
        "ts_wrong_default.ts",
        "ts_wrong_map.ts",
        "ts_untyped.ts",
        "ts_guard_wrong_key.ts",
        "java_guard_wrong_default.java",
        "py_wrong_key.py",
        "py_wrong_default.py",
        "py_wrong_map.py",
        "py_guard_wrong_map.py",
        "py_untyped.py",
        "py_alias_wrong_key.py",
        "py_alias_wrong_default.py",
        "py_alias_wrong_map.py",
        "py_alias_unresolved.py",
        "py_alias_shadowed.py",
    ] {
        assert!(
            !positive_text.contains(unexpected),
            "semantic mode must preserve typed map default boundaries: {semantic}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

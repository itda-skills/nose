use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn write_literal_map_default_fixtures(dir: &Path) {
    fs::write(
        dir.join("map_default.py"),
        "def lookup(key, other):\n    return {\"red\": 1, \"blue\": 2}.get(key, 0)\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default.rb"),
        "def lookup(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key, 0)\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_block.rb"),
        "def lookup(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key) { 0 }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_inline.js"),
        "function lookup(key, other) {\n  return new Map([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_call.js"),
        "function lookup(key, other) {\n  return Map([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_map_global.js"),
        "function Map(entries) {\n  return { get: function() { return 99; }, has: function() { return true; } };\n}\nfunction lookup(key, other) {\n  return new Map([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_local.js"),
        "function lookup(key, other) {\n  const values = new Map([[\"red\", 1], [\"blue\", 2]]);\n  return values.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_has_get.js"),
        "function lookup(key, other) {\n  const values = new Map([[\"red\", 1], [\"blue\", 2]]);\n  return values.has(key) ? values.get(key) : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_inline.ts"),
        "function lookup(key: string, other: string): number {\n  return new Map<string, number>([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_java_of.java"),
        "import java.util.Map;\n\nclass JavaMapOf {\n  static int lookup(String key, String other) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_java_entries.java"),
        "import java.util.Map;\n\nclass JavaMapEntries {\n  static int lookup(String key, String other) {\n    return Map.ofEntries(Map.entry(\"red\", 1), Map.entry(\"blue\", 2)).getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_java_local.java"),
        "import java.util.Map;\n\nclass JavaMapLocal {\n  static int lookup(String key, String other) {\n    Map<String, Integer> values = Map.of(\"red\", 1, \"blue\", 2);\n    return values.getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_module.js"),
        "const LOOKUP = new Map([[\"red\", 1], [\"blue\", 2]]);\n\nfunction lookup(key, other) {\n  return LOOKUP.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_module.ts"),
        "const LOOKUP = new Map<string, number>([[\"red\", 1], [\"blue\", 2]]);\n\nfunction lookup(key: string, other: string): number {\n  return LOOKUP.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_module.java"),
        "import java.util.Map;\n\nclass JavaModuleMap {\n  static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2);\n\n  static int lookup(String key, String other) {\n    return LOOKUP.getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_rust_hashmap.rs"),
        "pub fn lookup(key: &str, other: &str) -> i32 {\n    *std::collections::HashMap::from([(\"red\", 1), (\"blue\", 2)]).get(key).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_rust_btreemap.rs"),
        "pub fn lookup(key: &str, other: &str) -> i32 {\n    *std::collections::BTreeMap::from([(\"red\", 1), (\"blue\", 2)]).get(key).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_rust_local.rs"),
        "pub fn lookup(key: &str, other: &str) -> i32 {\n    let values = std::collections::HashMap::from([(\"red\", 1), (\"blue\", 2)]);\n    *values.get(key).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_inline.go"),
        "package p\n\nfunc Lookup(key string, other string) int {\n    return map[string]int{\"red\": 1, \"blue\": 2}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_local.go"),
        "package p\n\nfunc Lookup(key string, other string) int {\n    lookup := map[string]int{\"red\": 1, \"blue\": 2}\n    return lookup[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_var.go"),
        "package p\n\nfunc Lookup(key string, other string) int {\n    var lookup = map[string]int{\"red\": 1, \"blue\": 2}\n    return lookup[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_string.py"),
        "def lookup(key, other):\n    return {\"red\": \"apple\", \"blue\": \"berry\"}.get(key, \"\")\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_string.rb"),
        "def lookup(key, other)\n  {\"red\" => \"apple\", \"blue\" => \"berry\"}.fetch(key, \"\")\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_string_block.rb"),
        "def lookup(key, other)\n  {\"red\" => \"apple\", \"blue\" => \"berry\"}.fetch(key) { \"\" }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_string_inline.go"),
        "package p\n\nfunc Lookup(key string, other string) string {\n    return map[string]string{\"red\": \"apple\", \"blue\": \"berry\"}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_string_local.go"),
        "package p\n\nfunc Lookup(key string, other string) string {\n    lookup := map[string]string{\"red\": \"apple\", \"blue\": \"berry\"}\n    return lookup[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_bool.py"),
        "def lookup(key, other):\n    return {\"red\": True, \"blue\": False}.get(key, False)\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_bool.rb"),
        "def lookup(key, other)\n  {\"red\" => true, \"blue\" => false}.fetch(key, false)\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_bool_block.rb"),
        "def lookup(key, other)\n  {\"red\" => true, \"blue\" => false}.fetch(key) { false }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_bool_inline.go"),
        "package p\n\nfunc Lookup(key string, other string) bool {\n    return map[string]bool{\"red\": true, \"blue\": false}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_float.py"),
        "def lookup(key, other):\n    return {\"red\": 1.5, \"blue\": 2.5}.get(key, 0.0)\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_float.rb"),
        "def lookup(key, other)\n  {\"red\" => 1.5, \"blue\" => 2.5}.fetch(key, 0.0)\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_float_inline.go"),
        "package p\n\nfunc Lookup(key string, other string) float64 {\n    return map[string]float64{\"red\": 1.5, \"blue\": 2.5}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_float_local.go"),
        "package p\n\nfunc Lookup(key string, other string) float64 {\n    lookup := map[string]float64{\"red\": 1.5, \"blue\": 2.5}\n    return lookup[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_nil.py"),
        "def lookup(key, other):\n    return {\"red\": None, \"blue\": None}.get(key, None)\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_nil.rb"),
        "def lookup(key, other)\n  {\"red\" => nil, \"blue\" => nil}.fetch(key, nil)\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_nil_block.rb"),
        "def lookup(key, other)\n  {\"red\" => nil, \"blue\" => nil}.fetch(key) { nil }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_default_go_nil_inline.go"),
        "package p\n\ntype Item struct{}\n\nfunc Lookup(key string, other string) *Item {\n    return map[string]*Item{\"red\": nil, \"blue\": nil}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_hasown.js"),
        "function lookup(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return Object.hasOwn(values, key) ? values[key] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_hasown_call.js"),
        "function lookup(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return Object.prototype.hasOwnProperty.call(values, key) ? values[key] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_negated.ts"),
        "function lookup(key: string, other: string): number {\n  const values: Record<string, number> = { \"red\": 1, \"blue\": 2 };\n  return !Object.hasOwn(values, key) ? 0 : values[key];\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_key.py"),
        "def wrong_key(key, other):\n    return {\"red\": 1, \"blue\": 2}.get(other, 0)\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_default.rb"),
        "def wrong_default(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key, 9)\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_default_block.rb"),
        "def wrong_default(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key) { 9 }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_map.py"),
        "def wrong_map(key, other):\n    return {\"red\": 9, \"blue\": 2}.get(key, 0)\n",
    )
    .unwrap();
    fs::write(
        dir.join("ruby_fetch_block_param.rb"),
        "def wrong(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key) { |missing| missing.to_s }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("ruby_fetch_raise_block.rb"),
        "def wrong(key, other)\n  {\"red\" => 1, \"blue\" => 2}.fetch(key) { raise KeyError }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_js_key.js"),
        "function wrong_key(key, other) {\n  return new Map([[\"red\", 1], [\"blue\", 2]]).get(other) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_js_default.js"),
        "function wrong_default(key, other) {\n  return new Map([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 9;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_js_map.js"),
        "function wrong_map(key, other) {\n  return new Map([[\"red\", 9], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("untyped_receiver.js"),
        "function untyped_receiver(values, key, other) {\n  return values.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_map.js"),
        "function shadowed_map(key, other, Map) {\n  return new Map([[\"red\", 1], [\"blue\", 2]]).get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_java_key.java"),
        "import java.util.Map;\n\nclass WrongJavaKey {\n  static int wrong(String key, String other) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(other, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_java_default.java"),
        "import java.util.Map;\n\nclass WrongJavaDefault {\n  static int wrong(String key, String other) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(key, 9);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_java_map.java"),
        "import java.util.Map;\n\nclass WrongJavaMap {\n  static int wrong(String key, String other) {\n    return Map.of(\"red\", 9, \"blue\", 2).getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_java_map.java"),
        "class ShadowedJavaMap {\n  static class MapFactory {\n    java.util.Map<String, Integer> of(Object... values) { return java.util.Map.of(); }\n  }\n  static int wrong(String key, String other, MapFactory Map) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("local_java_map_type.java"),
        "class LocalJavaMapType {\n  static int wrong(String key, String other) {\n    return Map.of(\"red\", 1, \"blue\", 2).getOrDefault(key, 0);\n  }\n}\nclass Map {\n  static java.util.Map<String, Integer> of(Object... values) { return java.util.Map.of(); }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("module_map_missing_java_import.java"),
        "class JavaModuleMapMissingImport {\n  static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2);\n\n  static int wrong(String key, String other) {\n    return LOOKUP.getOrDefault(key, 0);\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("module_map_mutated.js"),
        "const LOOKUP = new Map([[\"red\", 1], [\"blue\", 2]]);\nLOOKUP.set(\"red\", 9);\n\nfunction wrong(key, other) {\n  return LOOKUP.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("module_map_shadowed.ts"),
        "const Map: any = function(_entries: any) {\n  return { get: function() { return 9; } };\n};\nconst LOOKUP = new Map([[\"red\", 1], [\"blue\", 2]]);\n\nfunction wrong(key: string, other: string): number {\n  return LOOKUP.get(key) ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("module_map_shadowed.java"),
        "class JavaShadowedModuleMap {\n  static final Map<String, Integer> LOOKUP = Map.of(\"red\", 1, \"blue\", 2);\n\n  static int wrong(String key, String other) {\n    return LOOKUP.getOrDefault(key, 0);\n  }\n}\nclass Map {\n  static java.util.Map<String, Integer> of(Object... values) { return java.util.Map.of(); }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_rust_key.rs"),
        "pub fn wrong(key: &str, other: &str) -> i32 {\n    *std::collections::HashMap::from([(\"red\", 1), (\"blue\", 2)]).get(other).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_rust_default.rs"),
        "pub fn wrong(key: &str, other: &str) -> i32 {\n    *std::collections::HashMap::from([(\"red\", 1), (\"blue\", 2)]).get(key).unwrap_or(&9)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_rust_map.rs"),
        "pub fn wrong(key: &str, other: &str) -> i32 {\n    *std::collections::HashMap::from([(\"red\", 9), (\"blue\", 2)]).get(key).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("rust_map_mutated.rs"),
        "pub fn wrong(key: &str, other: &str) -> i32 {\n    let mut values = std::collections::HashMap::from([(\"red\", 1), (\"blue\", 2)]);\n    values.insert(\"red\", 9);\n    *values.get(key).unwrap_or(&0)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_key.go"),
        "package p\n\nfunc Wrong(key string, other string) int {\n    return map[string]int{\"red\": 1, \"blue\": 2}[other]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_map.go"),
        "package p\n\nfunc Wrong(key string, other string) int {\n    return map[string]int{\"red\": 9, \"blue\": 2}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("go_keyed_slice.go"),
        "package p\n\nfunc Wrong(key int, other int) int {\n    return []int{0: 1, 1: 2}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_string_key.go"),
        "package p\n\nfunc Wrong(key string, other string) string {\n    return map[string]string{\"red\": \"apple\", \"blue\": \"berry\"}[other]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_bool_map.go"),
        "package p\n\nfunc Wrong(key string, other string) bool {\n    return map[string]bool{\"red\": false, \"blue\": false}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_float_key.go"),
        "package p\n\nfunc Wrong(key string, other string) float64 {\n    return map[string]float64{\"red\": 1.5, \"blue\": 2.5}[other]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_go_nil_map.go"),
        "package p\n\nfunc Wrong(key string, other string) string {\n    return map[string]string{\"red\": \"apricot\", \"blue\": \"berry\"}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("go_mixed_value_map.go"),
        "package p\n\nfunc Wrong(key string, other string) interface{} {\n    return map[string]interface{}{\"red\": \"apple\", \"blue\": false}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("go_string_keyed_slice.go"),
        "package p\n\nfunc Wrong(key int, other int) string {\n    return []string{0: \"apple\", 1: \"berry\"}[key]\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_wrong_key.js"),
        "function wrong_key(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return Object.hasOwn(values, other) ? values[other] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_wrong_default.js"),
        "function wrong_default(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return Object.hasOwn(values, key) ? values[key] : 9;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_wrong_map.js"),
        "function wrong_map(key, other) {\n  const values = { \"red\": 9, \"blue\": 2 };\n  return Object.hasOwn(values, key) ? values[key] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_unguarded.js"),
        "function unguarded(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return values[key] ?? 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_in.js"),
        "function object_in(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return key in values ? values[key] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_method.js"),
        "function object_method(key, other) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return values.hasOwnProperty(key) ? values[key] : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_shadowed.js"),
        "function object_shadowed(key, other, Object) {\n  const values = { \"red\": 1, \"blue\": 2 };\n  return Object.hasOwn(values, key) ? values[key] : 0;\n}\n",
    )
    .unwrap();
}

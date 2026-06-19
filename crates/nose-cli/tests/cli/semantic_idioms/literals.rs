use super::*;

#[test]
fn query_mode_semantic_converges_cross_language_list_literals() {
    let dir = std::env::temp_dir().join(format!("nose_list_cross_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("pair.js"),
        "export function pair(a, b) {\n    return [a, b];\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("pair.py"),
        "def make_pair(x, y):\n    return [x, y]\n",
    )
    .unwrap();
    fs::write(
        dir.join("pair.rb"),
        "def build_pair(first, second)\n  [first, second]\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("tuple_negative.py"),
        "def tuple_pair(a, b):\n    return (a, b)\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report one cross-language list literal family: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("pair.js")
            && semantic_text.contains("pair.py")
            && semantic_text.contains("pair.rb")
            && !semantic_text.contains("tuple_negative.py"),
        "semantic mode must converge list-like literals without merging tuples: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_preserves_js_object_keys() {
    let dir = std::env::temp_dir().join(format!("nose_object_semantic_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("object_a.ts"),
        "export function example(command: string, description: string) {\n    return { command, description };\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_b.ts"),
        "export function makeExample(cmd: string, desc: string) {\n    return { command: cmd, description: desc };\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_key_negative.ts"),
        "export function makeParam(name: string, description: string) {\n    return { name, description };\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_computed_a.ts"),
        "const KEY = \"command\";\nexport function computed(command: string, description: string) {\n    return { [KEY]: command, description };\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("object_computed_b.ts"),
        "const FIELD = \"command\";\nexport function computedOther(cmd: string, desc: string) {\n    return { [FIELD]: cmd, description: desc };\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report only same-key object construction: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("object_a.ts")
            && semantic_text.contains("object_b.ts")
            && !semantic_text.contains("object_key_negative.ts")
            && !semantic_text.contains("object_computed_a.ts")
            && !semantic_text.contains("object_computed_b.ts"),
        "semantic mode must preserve static object keys and reject computed-key object contracts: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_converges_cross_language_map_literals() {
    let dir = std::env::temp_dir().join(format!("nose_map_cross_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("map.ts"),
        "export function example(command: string, description: string) {\n    return { command, description };\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map.py"),
        "def make_example(cmd, desc):\n    return {\"command\": cmd, \"description\": desc}\n",
    )
    .unwrap();
    fs::write(
        dir.join("map.rb"),
        "def build_example(command, description)\n  { command: command, description: description }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("map_key_negative.ts"),
        "export function makeParam(name: string, description: string) {\n    return { name, description };\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report one cross-language map literal family: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("map.ts")
            && semantic_text.contains("map.py")
            && semantic_text.contains("map.rb")
            && !semantic_text.contains("map_key_negative.ts"),
        "semantic mode must converge map-like literals without dropping key identity: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_captures_module_literal_bindings() {
    let dir = std::env::temp_dir().join(format!("nose_module_const_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("locale_a.ts"),
        "const labels = { today: \"today\", tomorrow: \"tomorrow\" };\nexport function label(token: string) {\n    return labels[token];\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("locale_b.ts"),
        "const labels = { today: \"heute\", tomorrow: \"morgen\" };\nexport function label(token: string) {\n    return labels[token];\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("locale_a_copy.ts"),
        "const labels = { today: \"today\", tomorrow: \"tomorrow\" };\nexport function relativeLabel(key: string) {\n    return labels[key];\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("locale_mutated.ts"),
        "let labels = { today: \"today\", tomorrow: \"tomorrow\" };\nlabels = { today: \"heute\", tomorrow: \"morgen\" };\nexport function mutatedLabel(key: string) {\n    return labels[key];\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report only same module-literal binding behavior: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("locale_a.ts")
            && semantic_text.contains("locale_a_copy.ts")
            && !semantic_text.contains("locale_b.ts")
            && !semantic_text.contains("locale_mutated.ts"),
        "semantic mode must include captured module literal values: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_preserves_python_dict_keys() {
    let dir = std::env::temp_dir().join(format!("nose_dict_semantic_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("dict_a.py"),
        "def example(command, description):\n    return {\"command\": command, \"description\": description}\n",
    )
    .unwrap();
    fs::write(
        dir.join("dict_b.py"),
        "def make_example(cmd, desc):\n    return {\"command\": cmd, \"description\": desc}\n",
    )
    .unwrap();
    fs::write(
        dir.join("dict_key_negative.py"),
        "def make_param(name, description):\n    return {\"name\": name, \"description\": description}\n",
    )
    .unwrap();
    fs::write(
        dir.join("dict_spread_a.py"),
        "def with_spread(base, command):\n    return {**base, \"command\": command}\n",
    )
    .unwrap();
    fs::write(
        dir.join("dict_spread_b.py"),
        "def copy_spread(other, cmd):\n    return {**other, \"command\": cmd}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report only same-key dict construction: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("dict_a.py")
            && semantic_text.contains("dict_b.py")
            && !semantic_text.contains("dict_key_negative.py")
            && !semantic_text.contains("dict_spread_a.py")
            && !semantic_text.contains("dict_spread_b.py"),
        "semantic mode must preserve dict keys and reject unproved unpacking: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_preserves_ruby_hash_keys() {
    let dir = std::env::temp_dir().join(format!("nose_hash_semantic_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("hash_a.rb"),
        "def example(command, description)\n  { command: command, description: description }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("hash_b.rb"),
        "def make_example(cmd, desc)\n  { command: cmd, description: desc }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("hash_key_negative.rb"),
        "def make_param(name, description)\n  { name: name, description: description }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("hash_splat_a.rb"),
        "def with_splat(base, command)\n  { **base, command: command }\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("hash_splat_b.rb"),
        "def copy_splat(other, cmd)\n  { **other, command: cmd }\nend\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report only same-key hash construction: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("hash_a.rb")
            && semantic_text.contains("hash_b.rb")
            && !semantic_text.contains("hash_key_negative.rb")
            && !semantic_text.contains("hash_splat_a.rb")
            && !semantic_text.contains("hash_splat_b.rb"),
        "semantic mode must preserve hash keys and reject unproved splats: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

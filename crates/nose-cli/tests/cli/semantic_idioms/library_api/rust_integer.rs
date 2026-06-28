use crate::*;

#[test]
fn query_mode_semantic_proves_rust_integer_methods() {
    let dir =
        std::env::temp_dir().join(format!("nose_rust_numeric_methods_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("abs_conditional.py"),
        "def magnitude(value: int, other: int):\n    magnitude = value if value >= 0 else -value\n    return magnitude + other\n",
    )
    .unwrap();
    fs::write(
        dir.join("abs_method.rs"),
        "pub fn magnitude(value: i64, other: i64) -> i64 {\n    let magnitude = value.abs();\n    magnitude + other\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("min_conditional.py"),
        "def select(left, right, other):\n    selected = left if left <= right else right\n    return selected + other\n",
    )
    .unwrap();
    fs::write(
        dir.join("min_method.rs"),
        "pub fn select(left: i64, right: i64, other: i64) -> i64 {\n    let selected = left.min(right);\n    selected + other\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("max_conditional.py"),
        "def select(left, right, other):\n    selected = left if left >= right else right\n    return selected + other\n",
    )
    .unwrap();
    fs::write(
        dir.join("max_method.rs"),
        "pub fn select(left: i64, right: i64, other: i64) -> i64 {\n    let selected = left.max(right);\n    selected + other\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("abs_custom_negative.rs"),
        "struct Wrap(i64);\nimpl Wrap { fn abs(&self) -> i64 { 0 } }\npub fn magnitude(value: Wrap) -> i64 {\n    let magnitude = value.abs();\n    magnitude + 1\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("min_custom_negative.rs"),
        "struct Wrap(i64);\nimpl Wrap { fn min(&self, _right: i64) -> i64 { 0 } }\npub fn select(left: Wrap, right: i64, other: i64) -> i64 {\n    let selected = left.min(right);\n    selected + other\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("max_custom_negative.rs"),
        "struct Wrap(i64);\nimpl Wrap { fn max(&self, _right: i64) -> i64 { 0 } }\npub fn select(left: Wrap, right: i64, other: i64) -> i64 {\n    let selected = left.max(right);\n    selected + other\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("min_wrong_value.rs"),
        "pub fn select(left: i64, right: i64, other: i64) -> i64 {\n    let selected = left.min(other);\n    selected + other\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    for expected_pair in [
        ["abs_conditional.py", "abs_method.rs"],
        ["min_conditional.py", "min_method.rs"],
        ["max_conditional.py", "max_method.rs"],
    ] {
        let family = semantic_families
            .iter()
            .find(|family| {
                let text = family.to_string();
                expected_pair.iter().all(|expected| text.contains(expected))
            })
            .unwrap_or_else(|| {
                panic!("semantic mode should report integer method family: {semantic}")
            });
        let text = family.to_string();
        for unexpected in [
            "abs_custom_negative.rs",
            "min_custom_negative.rs",
            "max_custom_negative.rs",
            "min_wrong_value.rs",
        ] {
            assert!(
                !text.contains(unexpected),
                "semantic mode must preserve Rust integer method boundaries: {semantic}"
            );
        }
    }

    let _ = fs::remove_dir_all(&dir);
}

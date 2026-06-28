use crate::*;

#[test]
fn query_mode_semantic_proves_extreme_type4_idioms() {
    let dir = std::env::temp_dir().join(format!("nose_extreme_type4_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("record.ts"),
        "export function recordA(value: unknown) { return value !== null && typeof value === 'object' && Array.isArray(value) === false; }\n\
         export function recordB(input: unknown) { return typeof input === 'object' && input !== null && !Array.isArray(input); }\n\
         export function recordMissingArray(value: unknown) { return typeof value === 'object' && value !== null; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("early.ts"),
        "export function anyLoop(xs: number[]) { let found = false; for (const x of xs) { if (x > 0) { found = true; break; } } return found; }\n\
         export function anySome(xs: number[]) { return xs.some(x => x > 0); }\n\
         export function anyWrongPredicate(xs: number[]) { let found = false; for (const x of xs) { if (x < 0) { found = true; break; } } return found; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("membership.ts"),
        "export function colorOr(value: string) { return value === 'red' || value === 'blue'; }\n\
         export function colorIncludes(value: string) { return ['blue', 'red'].includes(value); }\n\
         export function colorWrongLiteral(value: string) { return value === 'red' || value === 'green'; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("builder.py"),
        concat!(
            "def concat_loop(xs):\n",
            "    out = \"\"\n",
            "    for x in xs:\n",
            "        out += x\n",
            "    return out\n\n",
            "def concat_join(xs):\n",
            "    return \"\".join(xs)\n\n",
            "def concat_prepend(xs):\n",
            "    out = \"\"\n",
            "    for x in xs:\n",
            "        out = x + out\n",
            "    return out\n",
        ),
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    let family = |positives: &[&str], negatives: &[&str]| {
        semantic_families
            .iter()
            .map(serde_json::Value::to_string)
            .find(|text| {
                positives.iter().all(|name| text.contains(name))
                    && negatives.iter().all(|name| !text.contains(name))
            })
            .unwrap_or_else(|| {
                panic!(
                    "semantic mode should report {positives:?} without {negatives:?}: {semantic}"
                )
            })
    };

    family(&["recordA", "recordB"], &["recordMissingArray"]);
    family(&["anyLoop", "anySome"], &["anyWrongPredicate"]);
    family(&["colorOr", "colorIncludes"], &["colorWrongLiteral"]);
    family(&["concat_loop", "concat_join"], &["concat_prepend"]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_proves_collection_empty_checks() {
    let dir = std::env::temp_dir().join(format!("nose_collection_empty_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("rust_len.rs"),
        "pub fn empty_len(items: &[i32], other: &[i32]) -> bool {\n    items.len() == 0\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("rust_named.rs"),
        "pub fn empty_named(values: &[i32], other: &[i32]) -> bool {\n    values.is_empty()\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("rust_threshold_negative.rs"),
        "pub fn one_item(items: &[i32], other: &[i32]) -> bool {\n    items.len() == 1\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("rust_receiver_negative.rs"),
        "pub fn other_empty(items: &[i32], other: &[i32]) -> bool {\n    other.is_empty()\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("java_size.java"),
        "class JavaSize { static boolean emptySize(java.util.List<Integer> items, java.util.List<Integer> other) { return items.size() == 0; } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("java_named.java"),
        "class JavaNamed { static boolean emptyNamed(java.util.List<Integer> values, java.util.List<Integer> other) { return values.isEmpty(); } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("ruby_length.rb"),
        "def empty_length(items, other)\n  items.length == 0\nend\n",
    )
    .unwrap();
    fs::write(
        dir.join("ruby_named.rb"),
        "def empty_named(values, other)\n  values.empty?\nend\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert!(
        !semantic_families.is_empty(),
        "semantic mode should report collection emptiness families: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    assert!(
        semantic_text.contains("rust_len.rs")
            && semantic_text.contains("rust_named.rs")
            && semantic_text.contains("java_size.java")
            && semantic_text.contains("java_named.java")
            && !semantic_text.contains("ruby_length.rb")
            && !semantic_text.contains("ruby_named.rb")
            && !semantic_text.contains("rust_threshold_negative.rs")
            && !semantic_text.contains("rust_receiver_negative.rs"),
        "semantic mode must prove collection-empty checks without merging boundaries: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

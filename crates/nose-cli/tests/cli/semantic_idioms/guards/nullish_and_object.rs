use super::*;

fn family_has_location_suffix(family: &serde_json::Value, suffix: &str) -> bool {
    family["locations"]
        .as_array()
        .expect("family should contain locations")
        .iter()
        .any(|loc| {
            loc["file"]
                .as_str()
                .is_some_and(|file| file.ends_with(suffix))
        })
}

#[test]
fn query_mode_semantic_distinguishes_nullish_from_truthy_defaults() {
    let dir = std::env::temp_dir().join(format!("nose_nullish_default_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("nullish_coalesce.js"),
        "function coalesce(value, fallback) {\n  return value ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("nullish_ternary.js"),
        "function ternary(value, fallback) {\n  return value == null ? fallback : value;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("nullish_guard.js"),
        "function guard(value, fallback) {\n  if (value == null) {\n    return fallback;\n  }\n  return value;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("truthy_negative.js"),
        "function truthy(value, fallback) {\n  return value || fallback;\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report one nullish-default family: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    for expected in [
        "nullish_coalesce.js",
        "nullish_ternary.js",
        "nullish_guard.js",
    ] {
        assert!(
            semantic_text.contains(expected),
            "semantic mode should include {expected}: {semantic}"
        );
    }
    assert!(
        !semantic_text.contains("truthy_negative.js"),
        "semantic mode must not merge nullish and truthy defaults: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_pins_strict_nullish_default_boundaries() {
    let dir = std::env::temp_dir().join(format!("nose_strict_nullish_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("nullish_coalesce.js"),
        "function coalesce(value, fallback) {\n  return value ?? fallback;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("nullish_loose_ternary.js"),
        "function loose(value, fallback) {\n  return value == null ? fallback : value;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("strict_null_ternary.js"),
        "function strict(value, fallback) {\n  return value === null ? fallback : value;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("strict_null_copy.js"),
        "function strictCopy(input, fallback) {\n  return input === null ? fallback : input;\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        2,
        "semantic mode should keep loose-nullish and strict-null defaults in separate families: {semantic}"
    );

    let nullish_family = family_with_all(
        &semantic_json,
        &["nullish_coalesce.js", "nullish_loose_ternary.js"],
    )
    .unwrap_or_else(|| panic!("semantic mode should keep the loose-nullish family: {semantic}"));
    for unexpected in ["strict_null_ternary.js", "strict_null_copy.js"] {
        assert!(
            !family_has_location_suffix(nullish_family, unexpected),
            "strict-null defaults must not merge into the nullish family: {semantic}"
        );
    }

    let strict_family = family_with_all(
        &semantic_json,
        &["strict_null_ternary.js", "strict_null_copy.js"],
    )
    .unwrap_or_else(|| {
        panic!("semantic mode should still converge equivalent strict-null defaults: {semantic}")
    });
    for unexpected in ["nullish_coalesce.js", "nullish_loose_ternary.js"] {
        assert!(
            !family_has_location_suffix(strict_family, unexpected),
            "loose-nullish defaults must not merge into the strict-null family: {semantic}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_pins_js_object_guard_nullish_boundary() {
    let dir =
        std::env::temp_dir().join(format!("nose_object_guard_nullish_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("strict_object_a.js"),
        "function strictA(value) {\n  return typeof value === \"object\" && value !== null;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("strict_object_b.js"),
        "function strictB(input) {\n  return input !== null && typeof input === \"object\";\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("loose_object_negative.js"),
        "function loose(candidate) {\n  return candidate != null && typeof candidate === \"object\";\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let strict_family = family_with_all(
        &semantic_json,
        &["strict_object_a.js", "strict_object_b.js"],
    )
    .unwrap_or_else(|| {
        panic!("semantic mode should keep the strict object guard family: {semantic}")
    });
    assert!(
        !family_has_location_suffix(strict_family, "loose_object_negative.js"),
        "loose `!= null` object guards must not merge into the strict non-null object guard family: {semantic}"
    );
    assert!(
        !semantic_json.to_string().contains("loose_object_negative.js"),
        "the compact loose-nullish object guard is a hard negative for this semantic family: {semantic}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_proves_js_record_shape_guards() {
    let dir = std::env::temp_dir().join(format!("nose_record_guard_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("guard_direct.js"),
        "function direct(value) {\n  return typeof value === \"object\" && value !== null && !Array.isArray(value);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("guard_reordered.js"),
        "function reordered(candidate) {\n  return !Array.isArray(candidate) && candidate !== null && typeof candidate === \"object\";\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("guard_truthy.js"),
        "function truthy(input) {\n  return Boolean(input) && typeof input === \"object\" && !Array.isArray(input);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("boolean_shadowed_negative.js"),
        "function shadowed(Boolean, input) {\n  return Boolean(input) && typeof input === \"object\" && !Array.isArray(input);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("array_shadowed_negative.js"),
        "function shadowed(Array, input) {\n  return typeof input === \"object\" && input !== null && !Array.isArray(input);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("array_destructured_shadow_negative.js"),
        "function shadowed(scope, input) {\n  const { Array } = scope;\n  return typeof input === \"object\" && input !== null && !Array.isArray(input);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("array_allowed_negative.js"),
        "function arrayAllowed(value) {\n  return typeof value === \"object\" && value !== null;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("null_allowed_negative.js"),
        "function nullAllowed(value) {\n  return typeof value === \"object\" && !Array.isArray(value);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("wrong_typeof_literal_negative.js"),
        "function wrongLiteral(value) {\n  return typeof value === \"ob ject\" && value !== null && !Array.isArray(value);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("typeof_identifier_negative.js"),
        "function wrongIdentifier(value) {\n  return typeofvalue === \"object\" && value !== null && !Array.isArray(value);\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report one proved record-shape guard family: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    for expected in ["guard_direct.js", "guard_reordered.js", "guard_truthy.js"] {
        assert!(
            semantic_text.contains(expected),
            "semantic mode should include {expected}: {semantic}"
        );
    }
    for unexpected in [
        "array_allowed_negative.js",
        "array_destructured_shadow_negative.js",
        "array_shadowed_negative.js",
        "boolean_shadowed_negative.js",
        "null_allowed_negative.js",
        "typeof_identifier_negative.js",
        "wrong_typeof_literal_negative.js",
    ] {
        assert!(
            !semantic_text.contains(unexpected),
            "semantic mode must reject invalid or incomplete record-shape guards: {semantic}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_mode_semantic_proves_js_own_property_guards() {
    let dir = std::env::temp_dir().join(format!("nose_own_property_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("has_own.js"),
        "function hasOwn(value) {\n  return Object.hasOwn(value, \"ready\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("has_own_call.js"),
        "function hasOwnCall(candidate) {\n  return Object.prototype.hasOwnProperty.call(candidate, \"ready\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("in_operator_negative.js"),
        "function inOperator(value) {\n  return \"ready\" in value;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("direct_method_negative.js"),
        "function directMethod(value) {\n  return value.hasOwnProperty(\"ready\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("different_key_negative.js"),
        "function differentKey(value) {\n  return Object.hasOwn(value, \"enabled\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_object_negative.js"),
        "function shadowedObject(Object, value) {\n  return Object.hasOwn(value, \"ready\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_global_object_negative.js"),
        "const Object = { hasOwn() { return false; } };\nfunction shadowedGlobal(value) {\n  return Object.hasOwn(value, \"ready\");\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("shadowed_object_call_negative.js"),
        "function shadowedObjectCall(Object, value) {\n  return Object.prototype.hasOwnProperty.call(value, \"ready\");\n}\n",
    )
    .unwrap();

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    assert_eq!(
        semantic_families.len(),
        1,
        "semantic mode should report one proved own-property guard family: {semantic}"
    );
    let semantic_text = semantic_json.to_string();
    for expected in ["has_own.js", "has_own_call.js"] {
        assert!(
            semantic_text.contains(expected),
            "semantic mode should include {expected}: {semantic}"
        );
    }
    for unexpected in [
        "in_operator_negative.js",
        "direct_method_negative.js",
        "different_key_negative.js",
        "shadowed_object_negative.js",
        "shadowed_object_call_negative.js",
        "shadowed_global_object_negative.js",
    ] {
        assert!(
            !semantic_text.contains(unexpected),
            "semantic mode must reject non-own or different-key property guards: {semantic}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

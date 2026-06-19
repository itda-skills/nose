use super::*;

#[path = "literal_defaults/fixture.rs"]
mod fixture;

// Broad fixture matrix for literal map default contracts. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn query_mode_semantic_proves_literal_map_default_lookup() {
    let dir = std::env::temp_dir().join(format!("nose_map_default_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fixture::write_literal_map_default_fixtures(&dir);

    let semantic = query_min_json(&dir, "semantic");
    let semantic_json = query_json(&semantic);
    let semantic_families = query_families(&semantic_json);
    let expected = [
        "map_default.py",
        "map_default.rb",
        "map_default_block.rb",
        "map_default_java_of.java",
        "map_default_java_entries.java",
        "map_default_java_local.java",
        "map_default_module.java",
        "map_default_has_get.js",
        "map_default_rust_hashmap.rs",
        "map_default_rust_btreemap.rs",
        "map_default_go_inline.go",
        "map_default_go_local.go",
        "map_default_go_var.go",
        "object_hasown.js",
        "object_hasown_call.js",
        "object_negated.ts",
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
            panic!("semantic mode should report one literal map-default family: {semantic}")
        });
    let positive_text = positive_family.to_string();
    for expected in expected {
        assert!(
            positive_text.contains(expected),
            "semantic mode should include {expected}: {semantic}"
        );
    }
    // The nullish-coalesce `?? ` forms split into their OWN family — `m.get(k) ?? d` defaults on
    // absent OR present-null, unsound to merge with the absence-only default (#410, experiments §CT).
    let coalesce_expected = [
        "map_default_inline.js",
        "map_default_local.js",
        "map_default_inline.ts",
        "map_default_module.js",
        "map_default_module.ts",
    ];
    let coalesce_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            coalesce_expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one nullish-coalesce family: {semantic}")
        });
    let coalesce_text = coalesce_family.to_string();
    for expected in coalesce_expected {
        assert!(
            coalesce_text.contains(expected),
            "semantic mode should include nullish-coalesce {expected}: {semantic}"
        );
    }
    assert!(
        !positive_text.contains("map_default_inline.js"),
        "absence-default family must exclude the nullish-coalesce forms: {semantic}"
    );
    let string_expected = [
        "map_default_string.py",
        "map_default_string.rb",
        "map_default_string_block.rb",
        "map_default_go_string_inline.go",
        "map_default_go_string_local.go",
    ];
    let string_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            string_expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one string map-default family: {semantic}")
        });
    let string_text = string_family.to_string();
    for expected in string_expected {
        assert!(
            string_text.contains(expected),
            "semantic mode should include string map-default {expected}: {semantic}"
        );
    }

    let bool_expected = [
        "map_default_bool.py",
        "map_default_bool.rb",
        "map_default_bool_block.rb",
        "map_default_go_bool_inline.go",
    ];
    let bool_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            bool_expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one bool map-default family: {semantic}")
        });
    let bool_text = bool_family.to_string();
    for expected in bool_expected {
        assert!(
            bool_text.contains(expected),
            "semantic mode should include bool map-default {expected}: {semantic}"
        );
    }

    let float_expected = [
        "map_default_float.py",
        "map_default_float.rb",
        "map_default_go_float_inline.go",
        "map_default_go_float_local.go",
    ];
    let float_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            float_expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one float map-default family: {semantic}")
        });
    let float_text = float_family.to_string();
    for expected in float_expected {
        assert!(
            float_text.contains(expected),
            "semantic mode should include float map-default {expected}: {semantic}"
        );
    }

    let nil_expected = [
        "map_default_nil.py",
        "map_default_nil.rb",
        "map_default_nil_block.rb",
        "map_default_go_nil_inline.go",
    ];
    let nil_family = semantic_families
        .iter()
        .find(|family| {
            let family_text = family.to_string();
            nil_expected
                .iter()
                .all(|expected| family_text.contains(expected))
        })
        .unwrap_or_else(|| {
            panic!("semantic mode should report one nil map-default family: {semantic}")
        });
    let nil_text = nil_family.to_string();
    for expected in nil_expected {
        assert!(
            nil_text.contains(expected),
            "semantic mode should include nil map-default {expected}: {semantic}"
        );
    }

    let boundary_files = [
        "wrong_key.py",
        "wrong_default.rb",
        "wrong_default_block.rb",
        "wrong_map.py",
        "ruby_fetch_block_param.rb",
        "ruby_fetch_raise_block.rb",
        "map_default_call.js",
        "shadowed_map_global.js",
        "map_default_rust_local.rs",
        "wrong_js_key.js",
        "wrong_js_default.js",
        "wrong_js_map.js",
        "untyped_receiver.js",
        "shadowed_map.js",
        "wrong_java_key.java",
        "wrong_java_default.java",
        "wrong_java_map.java",
        "shadowed_java_map.java",
        "local_java_map_type.java",
        "module_map_missing_java_import.java",
        "module_map_mutated.js",
        "module_map_shadowed.ts",
        "module_map_shadowed.java",
        "wrong_rust_key.rs",
        "wrong_rust_default.rs",
        "wrong_rust_map.rs",
        "rust_map_mutated.rs",
        "wrong_go_key.go",
        "wrong_go_map.go",
        "go_keyed_slice.go",
        "wrong_go_string_key.go",
        "wrong_go_bool_map.go",
        "wrong_go_float_key.go",
        "wrong_go_nil_map.go",
        "go_mixed_value_map.go",
        "go_string_keyed_slice.go",
        "object_wrong_key.js",
        "object_wrong_default.js",
        "object_wrong_map.js",
        "object_unguarded.js",
        "object_in.js",
        "object_method.js",
        "object_shadowed.js",
    ];
    for family_text in [
        &positive_text,
        &string_text,
        &bool_text,
        &float_text,
        &nil_text,
    ] {
        for unexpected in &boundary_files {
            assert!(
                !family_text.contains(*unexpected),
                "semantic mode must preserve literal map-default boundaries: {semantic}"
            );
        }
    }

    let _ = fs::remove_dir_all(&dir);
}

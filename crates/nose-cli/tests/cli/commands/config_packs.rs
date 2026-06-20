use super::*;

#[test]
fn config_file_supplies_defaults() {
    // A nose.toml in the working dir provides defaults (here: an exclude glob and
    // min-size); a CLI flag still overrides.
    let dir = make_project("cfg");
    fs::write(
        dir.join("nose.toml"),
        "[query]\nexclude = [\"a/**\"]\nmin-size = 12\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["query", ".", "--format", "json"])
        .current_dir(&dir)
        .output()
        .expect("run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("a/f.py"),
        "config exclude a/** should drop a/f.py: {stdout}"
    );
    assert!(stdout.contains("b/f.py"), "b/ remains: {stdout}");
    let _ = fs::remove_dir_all(&dir);
}

fn semantic_pack_manifest(id: &str) -> String {
    format!(
        r#"{{
  "api_version": "nose.semantic-pack.v0",
  "pack": {{
    "id": "{id}",
    "kind": "LibraryPack",
    "version": "0.1.0",
    "display_name": "Example semantic pack",
    "trust": "external-opt-in",
    "enabled_by_default": false
  }},
  "provenance": {{
    "provider": {{ "name": "Example Packs" }},
    "license": "MIT",
    "repository": "https://example.invalid/semantic-pack"
  }},
  "compatibility": {{ "nose": ">=0.5.0 <0.6.0" }},
  "supported_languages": [{{ "id": "python" }}],
  "declares": {{
    "evidence_producers": [{{
      "id": "python.library-api.example",
      "kind": "LibraryApi.Contract",
      "anchors": ["node"],
      "channel": "exact-empirical",
      "stable_hash_inputs": ["pack.id", "producer.id", "call_span"],
      "conflict_policy": "fail-closed"
    }}],
    "contracts": [{{
      "id": "python.example.contract",
      "surface": {{ "kind": "function" }},
      "requires": [{{
        "ref": "python.library-api.example",
        "subject": "call",
        "required": true
      }}],
      "semantics": {{
        "operation": "Example",
        "demand": {{ "arguments": "eager-left-to-right" }},
        "effects": ["argument-effects-in-order"]
      }},
      "channel": "exact-empirical",
      "proof_status": "covered",
      "conformance_refs": ["positive", "negative"]
    }}],
    "value_laws": []
  }},
  "conformance": {{
    "positive_fixtures": [{{
      "id": "positive",
      "description": "positive",
      "path": "fixtures/positive.py",
      "expectation": "exact-contract-open"
    }}],
    "hard_negatives": [{{
      "id": "negative",
      "description": "negative",
      "path": "fixtures/negative.py",
      "expectation": "exact-contract-closed"
    }}],
    "known_unsupported": []
  }}
}}"#
    )
}

fn semantic_pack_by_id<'a>(json: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    json["semantic_packs"]
        .as_array()
        .expect("query JSON should report semantic_packs")
        .iter()
        .find(|pack| pack["id"] == id)
        .unwrap_or_else(|| panic!("semantic_packs should include {id}: {json}"))
}

fn assert_example_external_pack(pack: &serde_json::Value, expected_id: &str) {
    assert_eq!(pack["id"], expected_id);
    assert_eq!(pack["kind"], "LibraryPack");
    assert_eq!(pack["version"], "0.1.0");
    assert_eq!(pack["display_name"], "Example semantic pack");
    assert_eq!(pack["trust"], "external-opt-in");
    assert_eq!(pack["enabled_by_default"], false);
    assert_eq!(pack["source"], "local-manifest");
    assert_eq!(pack["influence"], "metadata-only");
    assert_eq!(pack["provider"], "Example Packs");
    assert_eq!(pack["repository"], "https://example.invalid/semantic-pack");
    assert_eq!(pack["license"], "MIT");
    assert_eq!(
        json_array_strings(pack, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(pack["counts"]["evidence_producers"], 1);
    assert_eq!(pack["counts"]["contracts"], 1);
    assert_eq!(pack["counts"]["value_laws"], 0);
    assert_eq!(pack["counts"]["positive_fixtures"], 1);
    assert_eq!(pack["counts"]["hard_negatives"], 1);
}

#[test]
fn query_json_reports_builtin_semantic_packs() {
    let dir = make_project("semantic_pack_builtin_report");
    let json = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--format",
        "json",
    ]));
    assert_eq!(
        json["semantic_packs"]
            .as_array()
            .expect("semantic_packs should be an array")
            .len(),
        21
    );

    let first_party = semantic_pack_by_id(&json, "nose.first_party");
    assert_eq!(first_party["source"], "compiled-first-party");
    assert_eq!(first_party["influence"], "evidence-and-contracts");
    assert_eq!(first_party["trust"], "default-first-party");
    assert_eq!(first_party["enabled_by_default"], true);
    assert!(first_party["path"].is_null());

    let c = semantic_pack_by_id(&json, "nose.lang.c");
    assert_eq!(c["kind"], "LanguagePack");
    assert_eq!(c["source"], "compiled-first-party");
    assert_eq!(json_array_strings(c, "supported_languages"), vec!["c"]);
    assert_eq!(c["counts"]["evidence_producers"], 1);
    assert_eq!(c["counts"]["contracts"], 0);
    assert_eq!(c["counts"]["positive_fixtures"], 2);
    assert_eq!(c["counts"]["hard_negatives"], 2);

    let builtins = semantic_pack_by_id(&json, "nose.python.builtins.collection_factories");
    assert_eq!(builtins["kind"], "StdlibPack");
    assert_eq!(
        json_array_strings(builtins, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(builtins["counts"]["evidence_producers"], 1);
    assert_eq!(builtins["counts"]["contracts"], 1);
    assert_eq!(builtins["counts"]["positive_fixtures"], 4);
    assert_eq!(builtins["counts"]["hard_negatives"], 2);

    let collections = semantic_pack_by_id(&json, "nose.python.stdlib.collection_factories");
    assert_eq!(collections["kind"], "StdlibPack");
    assert_eq!(
        json_array_strings(collections, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(collections["counts"]["evidence_producers"], 1);
    assert_eq!(collections["counts"]["contracts"], 1);
    assert_eq!(collections["counts"]["positive_fixtures"], 3);
    assert_eq!(collections["counts"]["hard_negatives"], 2);

    let math = semantic_pack_by_id(&json, "nose.python.stdlib.math");
    assert_eq!(math["hash"], "9abb9da5e7aa81e0");
    assert_eq!(math["kind"], "StdlibPack");
    assert_eq!(math["display_name"], "nose Python stdlib math pack");
    assert_eq!(math["source"], "compiled-first-party");
    assert_eq!(math["influence"], "evidence-and-contracts");
    assert_eq!(math["trust"], "default-first-party");
    assert_eq!(math["enabled_by_default"], true);
    assert_eq!(math["path"], serde_json::Value::Null);
    assert_eq!(math["provider"], "Corca, Inc.");
    assert_eq!(math["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(math["license"], "MIT");
    assert_eq!(
        json_array_strings(math, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(math["counts"]["evidence_producers"], 1);
    assert_eq!(math["counts"]["contracts"], 1);
    assert_eq!(math["counts"]["value_laws"], 0);
    assert_eq!(math["counts"]["positive_fixtures"], 1);
    assert_eq!(math["counts"]["hard_negatives"], 2);

    let ruby_set = semantic_pack_by_id(&json, "nose.ruby.stdlib.set");
    assert_eq!(ruby_set["kind"], "StdlibPack");
    assert_eq!(
        json_array_strings(ruby_set, "supported_languages"),
        vec!["ruby"]
    );
    assert_eq!(ruby_set["counts"]["evidence_producers"], 1);
    assert_eq!(ruby_set["counts"]["contracts"], 1);
    assert_eq!(ruby_set["counts"]["positive_fixtures"], 3);
    assert_eq!(ruby_set["counts"]["hard_negatives"], 3);

    let rust_vec = semantic_pack_by_id(&json, "nose.rust.stdlib.vec");
    assert_eq!(rust_vec["hash"], "cc787cbb5aa0a87c");
    assert_eq!(rust_vec["kind"], "StdlibPack");
    assert_eq!(rust_vec["display_name"], "nose Rust stdlib Vec pack");
    assert_eq!(rust_vec["source"], "compiled-first-party");
    assert_eq!(rust_vec["influence"], "evidence-and-contracts");
    assert_eq!(rust_vec["trust"], "default-first-party");
    assert_eq!(rust_vec["enabled_by_default"], true);
    assert_eq!(rust_vec["path"], serde_json::Value::Null);
    assert_eq!(rust_vec["provider"], "Corca, Inc.");
    assert_eq!(rust_vec["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(rust_vec["license"], "MIT");
    assert_eq!(
        json_array_strings(rust_vec, "supported_languages"),
        vec!["rust"]
    );
    assert_eq!(rust_vec["counts"]["evidence_producers"], 1);
    assert_eq!(rust_vec["counts"]["contracts"], 2);
    assert_eq!(rust_vec["counts"]["value_laws"], 0);
    assert_eq!(rust_vec["counts"]["positive_fixtures"], 2);
    assert_eq!(rust_vec["counts"]["hard_negatives"], 2);

    let rust_option = semantic_pack_by_id(&json, "nose.rust.stdlib.option");
    assert_eq!(rust_option["hash"], "8ffb410363be1b73");
    assert_eq!(rust_option["kind"], "StdlibPack");
    assert_eq!(rust_option["display_name"], "nose Rust stdlib Option pack");
    assert_eq!(rust_option["source"], "compiled-first-party");
    assert_eq!(rust_option["influence"], "evidence-and-contracts");
    assert_eq!(rust_option["trust"], "default-first-party");
    assert_eq!(rust_option["enabled_by_default"], true);
    assert_eq!(rust_option["path"], serde_json::Value::Null);
    assert_eq!(rust_option["provider"], "Corca, Inc.");
    assert_eq!(
        rust_option["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(rust_option["license"], "MIT");
    assert_eq!(
        json_array_strings(rust_option, "supported_languages"),
        vec!["rust"]
    );
    assert_eq!(rust_option["counts"]["evidence_producers"], 1);
    assert_eq!(rust_option["counts"]["contracts"], 3);
    assert_eq!(rust_option["counts"]["value_laws"], 0);
    assert_eq!(rust_option["counts"]["positive_fixtures"], 3);
    assert_eq!(rust_option["counts"]["hard_negatives"], 3);

    let rust_collections = semantic_pack_by_id(&json, "nose.rust.stdlib.collection_factories");
    assert_eq!(rust_collections["hash"], "c0913f2d5652c20f");
    assert_eq!(rust_collections["kind"], "StdlibPack");
    assert_eq!(
        rust_collections["display_name"],
        "nose Rust stdlib collection factory pack"
    );
    assert_eq!(rust_collections["source"], "compiled-first-party");
    assert_eq!(rust_collections["influence"], "evidence-and-contracts");
    assert_eq!(rust_collections["trust"], "default-first-party");
    assert_eq!(rust_collections["enabled_by_default"], true);
    assert_eq!(rust_collections["path"], serde_json::Value::Null);
    assert_eq!(rust_collections["provider"], "Corca, Inc.");
    assert_eq!(
        rust_collections["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(rust_collections["license"], "MIT");
    assert_eq!(
        json_array_strings(rust_collections, "supported_languages"),
        vec!["rust"]
    );
    assert_eq!(rust_collections["counts"]["evidence_producers"], 1);
    assert_eq!(rust_collections["counts"]["contracts"], 1);
    assert_eq!(rust_collections["counts"]["value_laws"], 0);
    assert_eq!(rust_collections["counts"]["positive_fixtures"], 3);
    assert_eq!(rust_collections["counts"]["hard_negatives"], 2);

    let rust_maps = semantic_pack_by_id(&json, "nose.rust.stdlib.map_factories");
    assert_eq!(rust_maps["hash"], "418077a33dc67531");
    assert_eq!(rust_maps["kind"], "StdlibPack");
    assert_eq!(
        rust_maps["display_name"],
        "nose Rust stdlib map factory pack"
    );
    assert_eq!(rust_maps["source"], "compiled-first-party");
    assert_eq!(rust_maps["influence"], "evidence-and-contracts");
    assert_eq!(rust_maps["trust"], "default-first-party");
    assert_eq!(rust_maps["enabled_by_default"], true);
    assert_eq!(rust_maps["path"], serde_json::Value::Null);
    assert_eq!(rust_maps["provider"], "Corca, Inc.");
    assert_eq!(rust_maps["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(rust_maps["license"], "MIT");
    assert_eq!(
        json_array_strings(rust_maps, "supported_languages"),
        vec!["rust"]
    );
    assert_eq!(rust_maps["counts"]["evidence_producers"], 1);
    assert_eq!(rust_maps["counts"]["contracts"], 1);
    assert_eq!(rust_maps["counts"]["value_laws"], 0);
    assert_eq!(rust_maps["counts"]["positive_fixtures"], 2);
    assert_eq!(rust_maps["counts"]["hard_negatives"], 2);

    let java_maps = semantic_pack_by_id(&json, "nose.java.stdlib.map_factories");
    assert_eq!(java_maps["hash"], "1eecb2960193782f");
    assert_eq!(java_maps["kind"], "StdlibPack");
    assert_eq!(
        java_maps["display_name"],
        "nose Java stdlib map factory pack"
    );
    assert_eq!(java_maps["source"], "compiled-first-party");
    assert_eq!(java_maps["influence"], "evidence-and-contracts");
    assert_eq!(java_maps["trust"], "default-first-party");
    assert_eq!(java_maps["enabled_by_default"], true);
    assert_eq!(java_maps["path"], serde_json::Value::Null);
    assert_eq!(java_maps["provider"], "Corca, Inc.");
    assert_eq!(java_maps["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(java_maps["license"], "MIT");
    assert_eq!(
        json_array_strings(java_maps, "supported_languages"),
        vec!["java"]
    );
    assert_eq!(java_maps["counts"]["evidence_producers"], 1);
    assert_eq!(java_maps["counts"]["contracts"], 2);
    assert_eq!(java_maps["counts"]["value_laws"], 0);
    assert_eq!(java_maps["counts"]["positive_fixtures"], 2);
    assert_eq!(java_maps["counts"]["hard_negatives"], 2);

    let java_entries = semantic_pack_by_id(&json, "nose.java.stdlib.map_entries");
    assert_eq!(java_entries["hash"], "70b8bbc16bb60219");
    assert_eq!(java_entries["kind"], "StdlibPack");
    assert_eq!(
        java_entries["display_name"],
        "nose Java stdlib map entry pack"
    );
    assert_eq!(java_entries["source"], "compiled-first-party");
    assert_eq!(java_entries["influence"], "evidence-and-contracts");
    assert_eq!(java_entries["trust"], "default-first-party");
    assert_eq!(java_entries["enabled_by_default"], true);
    assert_eq!(java_entries["path"], serde_json::Value::Null);
    assert_eq!(java_entries["provider"], "Corca, Inc.");
    assert_eq!(
        java_entries["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(java_entries["license"], "MIT");
    assert_eq!(
        json_array_strings(java_entries, "supported_languages"),
        vec!["java"]
    );
    assert_eq!(java_entries["counts"]["evidence_producers"], 1);
    assert_eq!(java_entries["counts"]["contracts"], 1);
    assert_eq!(java_entries["counts"]["value_laws"], 0);
    assert_eq!(java_entries["counts"]["positive_fixtures"], 1);
    assert_eq!(java_entries["counts"]["hard_negatives"], 2);

    let java_collections = semantic_pack_by_id(&json, "nose.java.stdlib.collection_factories");
    assert_eq!(java_collections["hash"], "e784159038ce0c8d");
    assert_eq!(java_collections["kind"], "StdlibPack");
    assert_eq!(
        java_collections["display_name"],
        "nose Java stdlib collection factory pack"
    );
    assert_eq!(java_collections["source"], "compiled-first-party");
    assert_eq!(java_collections["influence"], "evidence-and-contracts");
    assert_eq!(java_collections["trust"], "default-first-party");
    assert_eq!(java_collections["enabled_by_default"], true);
    assert_eq!(java_collections["path"], serde_json::Value::Null);
    assert_eq!(java_collections["provider"], "Corca, Inc.");
    assert_eq!(
        java_collections["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(java_collections["license"], "MIT");
    assert_eq!(
        json_array_strings(java_collections, "supported_languages"),
        vec!["java"]
    );
    assert_eq!(java_collections["counts"]["evidence_producers"], 1);
    assert_eq!(java_collections["counts"]["contracts"], 3);
    assert_eq!(java_collections["counts"]["value_laws"], 0);
    assert_eq!(java_collections["counts"]["positive_fixtures"], 3);
    assert_eq!(java_collections["counts"]["hard_negatives"], 2);

    let java_constructors = semantic_pack_by_id(&json, "nose.java.stdlib.collection_constructors");
    assert_eq!(java_constructors["hash"], "47217e0e2e1f8108");
    assert_eq!(java_constructors["kind"], "StdlibPack");
    assert_eq!(
        java_constructors["display_name"],
        "nose Java stdlib collection constructor pack"
    );
    assert_eq!(java_constructors["source"], "compiled-first-party");
    assert_eq!(java_constructors["influence"], "evidence-and-contracts");
    assert_eq!(java_constructors["trust"], "default-first-party");
    assert_eq!(java_constructors["enabled_by_default"], true);
    assert_eq!(java_constructors["path"], serde_json::Value::Null);
    assert_eq!(java_constructors["provider"], "Corca, Inc.");
    assert_eq!(
        java_constructors["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(java_constructors["license"], "MIT");
    assert_eq!(
        json_array_strings(java_constructors, "supported_languages"),
        vec!["java"]
    );
    assert_eq!(java_constructors["counts"]["evidence_producers"], 1);
    assert_eq!(java_constructors["counts"]["contracts"], 1);
    assert_eq!(java_constructors["counts"]["value_laws"], 0);
    assert_eq!(java_constructors["counts"]["positive_fixtures"], 2);
    assert_eq!(java_constructors["counts"]["hard_negatives"], 3);

    let java_static_adapters =
        semantic_pack_by_id(&json, "nose.java.stdlib.static_collection_adapters");
    assert_eq!(java_static_adapters["hash"], "6fe217885f0a8fe8");
    assert_eq!(java_static_adapters["kind"], "StdlibPack");
    assert_eq!(
        java_static_adapters["display_name"],
        "nose Java stdlib static collection adapter pack"
    );
    assert_eq!(java_static_adapters["source"], "compiled-first-party");
    assert_eq!(java_static_adapters["influence"], "evidence-and-contracts");
    assert_eq!(java_static_adapters["trust"], "default-first-party");
    assert_eq!(java_static_adapters["enabled_by_default"], true);
    assert_eq!(java_static_adapters["path"], serde_json::Value::Null);
    assert_eq!(java_static_adapters["provider"], "Corca, Inc.");
    assert_eq!(
        java_static_adapters["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(java_static_adapters["license"], "MIT");
    assert_eq!(
        json_array_strings(java_static_adapters, "supported_languages"),
        vec!["java"]
    );
    assert_eq!(java_static_adapters["counts"]["evidence_producers"], 1);
    assert_eq!(java_static_adapters["counts"]["contracts"], 1);
    assert_eq!(java_static_adapters["counts"]["value_laws"], 0);
    assert_eq!(java_static_adapters["counts"]["positive_fixtures"], 1);
    assert_eq!(java_static_adapters["counts"]["hard_negatives"], 2);

    let js_promise = semantic_pack_by_id(&json, "nose.javascript.builtins.promise");
    assert_eq!(js_promise["hash"], "db20255756aa3abc");
    assert_eq!(js_promise["kind"], "StdlibPack");
    assert_eq!(
        js_promise["display_name"],
        "nose JavaScript builtins Promise pack"
    );
    assert_eq!(js_promise["source"], "compiled-first-party");
    assert_eq!(js_promise["influence"], "evidence-and-contracts");
    assert_eq!(js_promise["trust"], "default-first-party");
    assert_eq!(js_promise["enabled_by_default"], true);
    assert_eq!(js_promise["path"], serde_json::Value::Null);
    assert_eq!(js_promise["provider"], "Corca, Inc.");
    assert_eq!(js_promise["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(js_promise["license"], "MIT");
    assert_eq!(
        json_array_strings(js_promise, "supported_languages"),
        vec!["javascript", "typescript"]
    );
    assert_eq!(js_promise["counts"]["evidence_producers"], 1);
    assert_eq!(js_promise["counts"]["contracts"], 2);
    assert_eq!(js_promise["counts"]["value_laws"], 0);
    assert_eq!(js_promise["counts"]["positive_fixtures"], 2);
    assert_eq!(js_promise["counts"]["hard_negatives"], 3);

    let js_array = semantic_pack_by_id(&json, "nose.javascript.builtins.array");
    assert_eq!(js_array["hash"], "ca9d1142025e589c");
    assert_eq!(js_array["kind"], "StdlibPack");
    assert_eq!(
        js_array["display_name"],
        "nose JavaScript builtins Array pack"
    );
    assert_eq!(js_array["source"], "compiled-first-party");
    assert_eq!(js_array["influence"], "evidence-and-contracts");
    assert_eq!(js_array["trust"], "default-first-party");
    assert_eq!(js_array["enabled_by_default"], true);
    assert_eq!(js_array["path"], serde_json::Value::Null);
    assert_eq!(js_array["provider"], "Corca, Inc.");
    assert_eq!(js_array["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(js_array["license"], "MIT");
    assert_eq!(
        json_array_strings(js_array, "supported_languages"),
        vec!["javascript", "typescript"]
    );
    assert_eq!(js_array["counts"]["evidence_producers"], 1);
    assert_eq!(js_array["counts"]["contracts"], 2);
    assert_eq!(js_array["counts"]["value_laws"], 0);
    assert_eq!(js_array["counts"]["positive_fixtures"], 2);
    assert_eq!(js_array["counts"]["hard_negatives"], 3);

    let js_boolean = semantic_pack_by_id(&json, "nose.javascript.builtins.boolean");
    assert_eq!(js_boolean["hash"], "7548f93fba013b5d");
    assert_eq!(js_boolean["kind"], "StdlibPack");
    assert_eq!(
        js_boolean["display_name"],
        "nose JavaScript builtins Boolean pack"
    );
    assert_eq!(js_boolean["source"], "compiled-first-party");
    assert_eq!(js_boolean["influence"], "evidence-and-contracts");
    assert_eq!(js_boolean["trust"], "default-first-party");
    assert_eq!(js_boolean["enabled_by_default"], true);
    assert_eq!(js_boolean["path"], serde_json::Value::Null);
    assert_eq!(js_boolean["provider"], "Corca, Inc.");
    assert_eq!(js_boolean["repository"], "https://github.com/corca-ai/nose");
    assert_eq!(js_boolean["license"], "MIT");
    assert_eq!(
        json_array_strings(js_boolean, "supported_languages"),
        vec!["javascript", "typescript"]
    );
    assert_eq!(js_boolean["counts"]["evidence_producers"], 1);
    assert_eq!(js_boolean["counts"]["contracts"], 1);
    assert_eq!(js_boolean["counts"]["value_laws"], 0);
    assert_eq!(js_boolean["counts"]["positive_fixtures"], 1);
    assert_eq!(js_boolean["counts"]["hard_negatives"], 2);

    let js_collections =
        semantic_pack_by_id(&json, "nose.javascript.builtins.collection_constructors");
    assert_eq!(js_collections["hash"], "38f71dd71d3585c5");
    assert_eq!(js_collections["kind"], "StdlibPack");
    assert_eq!(
        js_collections["display_name"],
        "nose JavaScript builtins collection constructor pack"
    );
    assert_eq!(js_collections["source"], "compiled-first-party");
    assert_eq!(js_collections["influence"], "evidence-and-contracts");
    assert_eq!(js_collections["trust"], "default-first-party");
    assert_eq!(js_collections["enabled_by_default"], true);
    assert_eq!(js_collections["path"], serde_json::Value::Null);
    assert_eq!(js_collections["provider"], "Corca, Inc.");
    assert_eq!(
        js_collections["repository"],
        "https://github.com/corca-ai/nose"
    );
    assert_eq!(js_collections["license"], "MIT");
    assert_eq!(
        json_array_strings(js_collections, "supported_languages"),
        vec!["javascript", "typescript"]
    );
    assert_eq!(js_collections["counts"]["evidence_producers"], 1);
    assert_eq!(js_collections["counts"]["contracts"], 2);
    assert_eq!(js_collections["counts"]["value_laws"], 0);
    assert_eq!(js_collections["counts"]["positive_fixtures"], 2);
    assert_eq!(js_collections["counts"]["hard_negatives"], 3);

    let stdlib = semantic_pack_by_id(&json, "nose.python.stdlib.type_domain");
    assert_eq!(stdlib["kind"], "StdlibPack");
    assert_eq!(
        json_array_strings(stdlib, "supported_languages"),
        vec!["python"]
    );
    assert_eq!(stdlib["counts"]["evidence_producers"], 1);
    assert_eq!(stdlib["counts"]["contracts"], 1);

    let laws = semantic_pack_by_id(&json, "nose.value_graph.laws");
    assert_eq!(laws["kind"], "LawPack");
    assert_eq!(laws["source"], "compiled-first-party");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_json_reports_cli_semantic_pack_metadata_without_changing_families() {
    let dir = make_project("semantic_pack_cli_report");
    let pack = dir.join("pack.json");
    fs::write(&pack, semantic_pack_manifest("com.example.cli-pack")).unwrap();

    let without_pack = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--format",
        "json",
    ]));
    let with_pack = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--semantic-pack",
        pack.to_str().unwrap(),
        "--format",
        "json",
    ]));

    assert_eq!(
        query_families(&with_pack),
        query_families(&without_pack),
        "metadata-only external packs must not change reported families"
    );
    let reported = semantic_pack_by_id(&with_pack, "com.example.cli-pack");
    assert_example_external_pack(reported, "com.example.cli-pack");
    assert_eq!(
        reported["path"].as_str(),
        Some(pack.canonicalize().unwrap().to_str().unwrap())
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn external_pack_mirroring_builtin_type_domain_vocabulary_stays_metadata_only() {
    let dir = make_project("semantic_pack_type_domain_mirror");
    let pack = dir.join("pack.json");
    let mirror = semantic_pack_manifest("com.example.python-stdlib-type-domain-mirror")
        .replace(
            "python.library-api.example",
            "python.stdlib.type-domain-alias-domain",
        )
        .replace("LibraryApi.Contract", "Domain.TypeAlias")
        .replace(
            "python.example.contract",
            "python.stdlib.type-domain-alias.contract",
        )
        .replace("Example", "PythonStdlibTypeDomainAlias");
    fs::write(&pack, mirror).unwrap();

    let without_pack = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--format",
        "json",
    ]));
    let with_pack = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--semantic-pack",
        pack.to_str().unwrap(),
        "--format",
        "json",
    ]));

    assert_eq!(
        query_families(&with_pack),
        query_families(&without_pack),
        "a local external pack mirroring builtin type-domain row ids must stay metadata-only"
    );
    let reported = semantic_pack_by_id(&with_pack, "com.example.python-stdlib-type-domain-mirror");
    assert_eq!(reported["source"], "local-manifest");
    assert_eq!(reported["influence"], "metadata-only");
    assert_eq!(reported["counts"]["evidence_producers"], 1);
    assert_eq!(reported["counts"]["contracts"], 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_pack_check_json_reports_conformance_success() {
    let dir = make_project("semantic_pack_check_ok");
    let fixture_dir = dir.join("fixtures");
    fs::create_dir_all(&fixture_dir).unwrap();
    fs::write(
        fixture_dir.join("positive.py"),
        "def positive(xs):\n    return sum(xs)\n",
    )
    .unwrap();
    fs::write(
        fixture_dir.join("negative.py"),
        "def negative(xs):\n    return list(xs)\n",
    )
    .unwrap();
    let pack = dir.join("pack.json");
    fs::write(&pack, semantic_pack_manifest("com.example.semantic-pack")).unwrap();

    let out = Command::new(bin())
        .args([
            "semantic-pack",
            "check",
            pack.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("semantic pack check");

    assert!(
        out.status.success(),
        "semantic-pack check should pass: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("check should emit JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["totals"]["manifests"], 1);
    assert_eq!(json["totals"]["positive_fixtures"], 1);
    assert_eq!(json["totals"]["hard_negatives"], 1);
    assert_eq!(json["totals"]["fixture_issues"], 0);
    assert_eq!(json["manifests"][0]["id"], "com.example.semantic-pack");
    assert_eq!(
        json["manifests"][0]["fixtures"][0]["issues"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_pack_check_fails_on_missing_fixture_files() {
    let dir = make_project("semantic_pack_check_missing");
    let pack = dir.join("pack.json");
    fs::write(&pack, semantic_pack_manifest("com.example.semantic-pack")).unwrap();

    let out = Command::new(bin())
        .args([
            "semantic-pack",
            "check",
            pack.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("semantic pack check");

    assert!(
        !out.status.success(),
        "semantic-pack check should fail when declared fixtures are missing"
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("failed check should still emit JSON");
    assert_eq!(json["status"], "failed");
    assert_eq!(json["totals"]["fixture_issues"], 2);
    assert_eq!(
        json["manifests"][0]["fixtures"][0]["issues"][0],
        "missing-file"
    );
    assert!(
        stderr.contains("semantic pack conformance failed"),
        "stderr should name the conformance failure: {stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn query_json_keeps_python_repetition_out_of_numeric_law_provenance() {
    let dir =
        std::env::temp_dir().join(format!("nose_cli_law_hard_negative_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("repetition.py"),
        "def repeated(a, b):\n    return a * 2 + b * 2\n\n\ndef grouped(a, b):\n    return (a + b) * 2\n",
    )
    .unwrap();
    let json = query_json(&run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-size",
        "1",
        "--min-lines",
        "1",
        "top=0",
        "--format",
        "json",
    ]));
    let families = json["families"].as_array().expect("families array");
    assert!(
        families
            .iter()
            .all(|family| family["semantic_laws"].is_null()),
        "Python repetition must not report numeric factor-distribution provenance: {json}"
    );
    assert!(
        families.is_empty(),
        "Python repetition must fail closed for the semantic exact channel: {json}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn config_semantic_packs_are_explicit_opt_ins() {
    let dir = make_project("semantic_pack_cfg");
    fs::write(
        dir.join("pack.json"),
        semantic_pack_manifest("com.example.config-pack"),
    )
    .unwrap();
    fs::write(
        dir.join("nose.toml"),
        "[query]\nmin-size = 12\nsemantic-packs = [\"pack.json\"]\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["query", ".", "--format", "json"])
        .current_dir(&dir)
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "query should load config semantic pack: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = query_json(&stdout);
    let reported = semantic_pack_by_id(&json, "com.example.config-pack");
    assert_example_external_pack(reported, "com.example.config-pack");
    assert_eq!(
        reported["path"].as_str(),
        Some(
            dir.join("pack.json")
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn explicit_config_semantic_pack_paths_resolve_from_config_directory() {
    let dir = make_project("semantic_pack_explicit_cfg");
    fs::write(
        dir.join("pack.json"),
        semantic_pack_manifest("com.example.explicit-config-pack"),
    )
    .unwrap();
    let config = dir.join("nose.toml");
    fs::write(
        &config,
        "[query]\nmin-size = 12\nsemantic-packs = [\"pack.json\"]\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args([
            "query",
            dir.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ])
        .current_dir(dir.parent().expect("test project has a parent"))
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "query should load config-relative semantic pack: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = query_json(&stdout);
    let reported = semantic_pack_by_id(&json, "com.example.explicit-config-pack");
    assert_example_external_pack(reported, "com.example.explicit-config-pack");
    assert_eq!(
        reported["path"].as_str(),
        Some(
            dir.join("pack.json")
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn explicit_config_ignore_file_resolves_from_config_directory() {
    let dir = make_project("ignore_explicit_cfg");
    fs::write(
        dir.join("nose.ignore.json"),
        "{\"ignores\":[{\"paths\":[\"**/a/**\",\"**/b/**\",\"**/tests/**\"],\"reason\":\"template-copy\"}]}\n",
    )
    .unwrap();
    let config = dir.join("nose.toml");
    fs::write(
        &config,
        "[query]\nmin-size = 12\nignore-file = \"nose.ignore.json\"\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args([
            "query",
            dir.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--mode",
            "semantic",
            "--format",
            "json",
            "top=0",
        ])
        .current_dir(dir.parent().expect("test project has a parent"))
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "query should load config-relative ignore file: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = query_json(&stdout);
    assert!(
        query_families(&json).is_empty(),
        "config-relative ignore file should suppress the family: {stdout}"
    );
    let _ = fs::remove_dir_all(&dir);
}

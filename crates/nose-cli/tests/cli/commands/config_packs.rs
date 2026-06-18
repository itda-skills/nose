use super::*;

#[test]
fn config_file_supplies_defaults() {
    // A nose.toml in the working dir provides defaults (here: an exclude glob and
    // min-size); a CLI flag still overrides.
    let dir = make_project("cfg");
    fs::write(
        dir.join("nose.toml"),
        "[scan]\nexclude = [\"a/**\"]\nmin-size = 12\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["scan", ".", "--format", "json"])
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

/// Compiled-in pack rows share the same provenance coordinates.
fn assert_compiled_first_party_pack(pack: &serde_json::Value, id: &str) {
    assert_eq!(pack["id"], id);
    assert_eq!(pack["source"], "compiled-first-party");
    assert_eq!(pack["influence"], "evidence-and-contracts");
}

#[test]
fn scan_json_reports_first_party_and_local_semantic_pack_provenance() {
    let dir = make_project("semantic_pack_cli");
    let pack = dir.join("pack.json");
    fs::write(&pack, semantic_pack_manifest("com.example.semantic-pack")).unwrap();

    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--format",
        "json",
        "--min-size",
        "12",
        "--semantic-pack",
        pack.to_str().unwrap(),
    ]);
    let json = scan_json(&out);
    let packs = json["semantic_packs"]
        .as_array()
        .expect("semantic_packs array");
    assert_eq!(
        packs.len(),
        4,
        "first-party packs + local opt-in pack: {json}"
    );
    assert_compiled_first_party_pack(&packs[0], "nose.first_party");
    assert_compiled_first_party_pack(&packs[1], "nose.python.stdlib.type_domain");
    assert_eq!(packs[1]["kind"], "StdlibPack");
    assert_eq!(packs[1]["counts"]["evidence_producers"], 1);
    assert_eq!(packs[1]["counts"]["contracts"], 1);
    assert_compiled_first_party_pack(&packs[2], "nose.value_graph.laws");
    assert_eq!(packs[2]["kind"], "LawPack");
    assert_eq!(packs[2]["counts"]["value_laws"], 2);
    assert_eq!(packs[3]["id"], "com.example.semantic-pack");
    assert_eq!(packs[3]["trust"], "external-opt-in");
    assert_eq!(packs[3]["source"], "local-manifest");
    assert_eq!(packs[3]["influence"], "metadata-only");
    assert_eq!(packs[3]["counts"]["contracts"], 1);
    assert!(packs[3]["hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn scan_json_reports_value_law_provenance_for_semantic_family() {
    let dir = std::env::temp_dir().join(format!("nose_cli_law_provenance_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("left.rs"),
        "fn f(a: i64, b: i64, c: i64) -> i64 {\n    a * c + b * c\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("right.rs"),
        "fn g(a: i64, b: i64, c: i64) -> i64 {\n    (a + b) * c\n}\n",
    )
    .unwrap();
    let json = scan_json(&run(&[
        "scan",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-size",
        "1",
        "--min-lines",
        "1",
        "--top",
        "0",
        "--format",
        "json",
    ]));
    let families = json["families"].as_array().expect("families array");
    assert_eq!(
        families.len(),
        1,
        "factor-distribution family expected: {json}"
    );
    let laws = families[0]["semantic_laws"]
        .as_array()
        .expect("semantic_laws array");
    assert_eq!(laws.len(), 1, "one law provenance row expected: {json}");
    assert_eq!(laws[0]["pack_id"], "nose.value_graph.laws");
    assert_eq!(
        laws[0]["law_id"],
        "value-graph.factor-distribute.numeric-common-factor"
    );
    assert_eq!(
        laws[0]["proof_obligation_id"],
        "normalize.value_graph.factor_distribute"
    );
    assert_eq!(laws[0]["proof_status"], "proven");
    assert_eq!(laws[0]["channel"], "exact-proven");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn scan_json_keeps_python_repetition_out_of_numeric_law_provenance() {
    let dir =
        std::env::temp_dir().join(format!("nose_cli_law_hard_negative_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("repetition.py"),
        "def repeated(a, b):\n    return a * 2 + b * 2\n\n\ndef grouped(a, b):\n    return (a + b) * 2\n",
    )
    .unwrap();
    let json = scan_json(&run(&[
        "scan",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-size",
        "1",
        "--min-lines",
        "1",
        "--top",
        "0",
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
fn scan_human_reports_local_semantic_pack_opt_in() {
    let dir = make_project("semantic_pack_human");
    let pack = dir.join("pack.json");
    fs::write(&pack, semantic_pack_manifest("com.example.semantic-pack")).unwrap();

    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--min-size",
        "12",
        "--semantic-pack",
        pack.to_str().unwrap(),
    ]);
    assert!(
        out.contains(
            "semantic packs: 3 first-party default · 1 local opt-in: \
             com.example.semantic-pack@0.1.0 (metadata-only)"
        ),
        "human scan output should disclose local semantic pack opt-ins: {out}"
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
        "[scan]\nmin-size = 12\nsemantic-packs = [\"pack.json\"]\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["scan", ".", "--format", "json"])
        .current_dir(&dir)
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "scan should load config semantic pack: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = scan_json(&stdout);
    let packs = json["semantic_packs"]
        .as_array()
        .expect("semantic_packs array");
    assert!(packs.iter().any(
        |pack| pack["id"] == "com.example.config-pack" && pack["influence"] == "metadata-only"
    ));
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
        "[scan]\nmin-size = 12\nsemantic-packs = [\"pack.json\"]\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args([
            "scan",
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
        "scan should load config-relative semantic pack: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = scan_json(&stdout);
    let packs = json["semantic_packs"]
        .as_array()
        .expect("semantic_packs array");
    assert!(packs
        .iter()
        .any(|pack| pack["id"] == "com.example.explicit-config-pack"));
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
        "[scan]\nmin-size = 12\nignore-file = \"nose.ignore.json\"\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args([
            "scan",
            dir.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--mode",
            "semantic",
            "--format",
            "json",
            "--top",
            "0",
        ])
        .current_dir(dir.parent().expect("test project has a parent"))
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "scan should load config-relative ignore file: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json = scan_json(&stdout);
    assert!(
        scan_families(&json).is_empty(),
        "config-relative ignore file should suppress the family: {stdout}"
    );
    assert_eq!(json["ignore"]["active_entries"], 1);
    assert_eq!(json["ignore"]["ignored_families"], 1);
    assert!(json["ignore"]["path"]
        .as_str()
        .is_some_and(|path| path.ends_with("nose.ignore.json")));
    let _ = fs::remove_dir_all(&dir);
}

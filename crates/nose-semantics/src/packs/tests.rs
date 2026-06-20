use super::*;
use std::collections::HashSet;
use std::fs;

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nose_semantic_pack_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// nose-ignore: inline semantic-pack manifest fixture; keeping the JSON shape visible matters here.
fn manifest(id: &str) -> String {
    format!(
        r#"{{
  "api_version": "nose.semantic-pack.v0",
  "pack": {{
    "id": "{id}",
    "kind": "LibraryPack",
    "version": "0.1.0",
    "display_name": "Example",
    "trust": "external-opt-in",
    "enabled_by_default": false
  }},
  "provenance": {{
    "provider": {{ "name": "Example" }},
    "license": "MIT",
    "repository": "https://example.invalid"
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
fn builtin_pack_descriptor_registry_names_current_compiled_packs() {
    let descriptors = builtin_pack_descriptors();
    assert_eq!(descriptors.len(), 6);
    let ids = descriptors
        .iter()
        .map(|descriptor| descriptor.id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            FIRST_PARTY_PACK_ID,
            C_LANGUAGE_PACK_ID,
            PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
            PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
            PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID,
            FIRST_PARTY_VALUE_LAW_PACK_ID
        ]
    );
    assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    assert!(descriptors
        .iter()
        .all(|descriptor| descriptor.trust == PackTrust::DefaultFirstParty));
    assert!(descriptors
        .iter()
        .all(|descriptor| descriptor.enabled_by_default));
}

#[test]
fn builtin_pack_descriptors_enumerate_declarations_and_conformance_refs() {
    let c = builtin_pack_descriptor(C_LANGUAGE_PACK_ID).expect("C language descriptor");
    assert_eq!(c.kind, SemanticPackKind::LanguagePack);
    assert_eq!(c.supported_languages, &["c"]);
    assert!(c.supported_packages.is_empty());
    let language = c
        .language
        .expect("C descriptor should expose language binding");
    assert_eq!(language.lang, nose_il::Lang::C);
    assert_eq!(language.file_extensions, &["c", "h"]);
    assert_eq!(language.parser, "tree-sitter-c");
    assert_eq!(language.lowering_entrypoint, "nose_frontend::c::lower");
    assert_eq!(
        c.evidence_producer_ids,
        &[C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID]
    );
    assert_eq!(
        c.source_fact_producer_ids,
        &[C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID]
    );
    assert_eq!(c.counts().evidence_producers, 1);
    assert_eq!(c.counts().contracts, 0);
    assert_eq!(c.counts().positive_fixtures, 2);
    assert_eq!(c.counts().hard_negatives, 2);
    assert!(c
        .conformance_refs()
        .contains(&"c-unsigned32-signed-cast-hard-negative"));

    let python_builtins = builtin_pack_descriptor(PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID)
        .expect("Python builtins descriptor");
    assert_eq!(python_builtins.kind, SemanticPackKind::StdlibPack);
    assert_eq!(python_builtins.supported_languages, &["python"]);
    assert_eq!(python_builtins.supported_packages, &["builtins"]);
    assert_eq!(
        python_builtins.evidence_producer_ids,
        &[PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID]
    );
    assert!(python_builtins.source_fact_producer_ids.is_empty());
    assert_eq!(
        python_builtins.contract_ids,
        &[PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_ID]
    );
    assert_eq!(python_builtins.counts().evidence_producers, 1);
    assert_eq!(python_builtins.counts().contracts, 1);
    assert_eq!(python_builtins.counts().positive_fixtures, 4);
    assert_eq!(python_builtins.counts().hard_negatives, 2);
    assert!(python_builtins
        .conformance_refs()
        .contains(&"python-builtin-list-wildcard-import-hard-negative"));

    let python_stdlib_collections =
        builtin_pack_descriptor(PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID)
            .expect("Python stdlib collection factory descriptor");
    assert_eq!(python_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(python_stdlib_collections.supported_languages, &["python"]);
    assert_eq!(
        python_stdlib_collections.supported_packages,
        &["collections"]
    );
    assert_eq!(
        python_stdlib_collections.evidence_producer_ids,
        &[PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID]
    );
    assert!(python_stdlib_collections
        .source_fact_producer_ids
        .is_empty());
    assert_eq!(
        python_stdlib_collections.contract_ids,
        &[PYTHON_STDLIB_COLLECTION_FACTORY_CONTRACT_ID]
    );
    assert_eq!(python_stdlib_collections.counts().evidence_producers, 1);
    assert_eq!(python_stdlib_collections.counts().contracts, 1);
    assert_eq!(python_stdlib_collections.counts().positive_fixtures, 3);
    assert_eq!(python_stdlib_collections.counts().hard_negatives, 2);
    assert!(python_stdlib_collections
        .conformance_refs()
        .contains(&"python-collections-deque-wrong-module-hard-negative"));

    let python = builtin_pack_descriptor(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)
        .expect("Python stdlib descriptor");
    assert_eq!(python.kind, SemanticPackKind::StdlibPack);
    assert_eq!(python.supported_languages, &["python"]);
    assert_eq!(
        python.supported_packages,
        &["typing", "collections.abc", "asyncio"]
    );
    assert_eq!(
        python.evidence_producer_ids,
        &[PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID]
    );
    assert_eq!(
        python.contract_ids,
        &["python.stdlib.type-domain-alias.contract"]
    );
    assert_eq!(
        python.type_domain_alias_contracts,
        PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS
    );
    assert!(python
        .type_domain_alias_contracts
        .iter()
        .all(|row| row.pack_id == PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID));
    assert!(python
        .type_domain_alias_contracts
        .iter()
        .all(|row| row.producer_id == PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID));
    assert!(python
        .type_domain_alias_contracts
        .iter()
        .all(|row| python.contract_ids.contains(&row.contract_id)));
    assert_eq!(python.counts().evidence_producers, 1);
    assert!(python.source_fact_producer_ids.is_empty());
    assert_eq!(python.counts().contracts, 1);
    assert_eq!(
        python.counts().positive_fixtures,
        PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len()
    );
    assert!(python
        .conformance_refs()
        .contains(&"python-typing-dict-domain-positive"));
    assert!(python
        .conformance_refs()
        .contains(&"python-typing-domain-wrong-module-hard-negative"));

    let laws =
        builtin_pack_descriptor(FIRST_PARTY_VALUE_LAW_PACK_ID).expect("value law descriptor");
    assert_eq!(laws.kind, SemanticPackKind::LawPack);
    assert_eq!(laws.counts().value_laws, pack_facing_value_laws().len());
    assert_eq!(
        laws.value_law_ids(),
        pack_facing_value_laws()
            .iter()
            .map(|law| law.law_id)
            .collect::<Vec<_>>()
    );
    assert!(laws
        .conformance_refs()
        .contains(&"clamp-float-hard-negative"));
}

#[test]
fn first_party_pack_hash_matches_evidence_provenance_hash_policy() {
    let pack = first_party_semantic_pack();
    assert_eq!(pack.id, FIRST_PARTY_PACK_ID);
    assert_eq!(pack.hash, stable_symbol_hash(FIRST_PARTY_PACK_ID));
    assert_eq!(pack.influence, SemanticPackInfluence::EvidenceAndContracts);
    let set = SemanticPackSet::first_party_only();
    let c = set
        .packs()
        .iter()
        .find(|pack| pack.id == C_LANGUAGE_PACK_ID)
        .expect("C summary");
    assert_eq!(c.id, C_LANGUAGE_PACK_ID);
    assert_eq!(c.hash, stable_symbol_hash(C_LANGUAGE_PACK_ID));
    assert_eq!(c.kind, SemanticPackKind::LanguagePack);
    assert_eq!(c.counts.evidence_producers, 1);
    let python_builtins = set
        .packs()
        .iter()
        .find(|pack| pack.id == PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID)
        .expect("Python builtins summary");
    assert_eq!(
        python_builtins.id,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID
    );
    assert_eq!(
        python_builtins.hash,
        stable_symbol_hash(PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID)
    );
    assert_eq!(python_builtins.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        python_builtins.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(python_builtins.counts.evidence_producers, 1);
    assert_eq!(python_builtins.counts.contracts, 1);
    assert_eq!(python_builtins.counts.positive_fixtures, 4);
    assert_eq!(python_builtins.counts.hard_negatives, 2);
    let python_stdlib_collections = set
        .packs()
        .iter()
        .find(|pack| pack.id == PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID)
        .expect("Python stdlib collections summary");
    assert_eq!(
        python_stdlib_collections.hash,
        stable_symbol_hash(PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID)
    );
    assert_eq!(python_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        python_stdlib_collections.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(python_stdlib_collections.counts.evidence_producers, 1);
    assert_eq!(python_stdlib_collections.counts.contracts, 1);
    assert_eq!(python_stdlib_collections.counts.positive_fixtures, 3);
    assert_eq!(python_stdlib_collections.counts.hard_negatives, 2);
    let python = python_stdlib_type_domain_pack();
    assert_eq!(python.id, PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID);
    assert_eq!(
        python.hash,
        stable_symbol_hash(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)
    );
    assert_eq!(python.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        python.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(python.counts.evidence_producers, 1);
    assert_eq!(python.counts.contracts, 1);
    assert_eq!(
        python.counts.positive_fixtures,
        PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len()
    );
    let laws = first_party_value_law_pack();
    assert_eq!(laws.id, FIRST_PARTY_VALUE_LAW_PACK_ID);
    assert_eq!(laws.hash, stable_symbol_hash(FIRST_PARTY_VALUE_LAW_PACK_ID));
    assert_eq!(laws.kind, SemanticPackKind::LawPack);
    assert_eq!(laws.counts.value_laws, pack_facing_value_laws().len());
    assert_eq!(laws.counts.positive_fixtures, 2);
    assert_eq!(laws.counts.hard_negatives, 4);
}

#[test]
fn local_manifest_loads_as_metadata_only_opt_in() {
    let dir = unique_dir("load");
    let path = dir.join("pack.json");
    fs::write(&path, manifest("com.example.pack")).unwrap();
    let set = SemanticPackSet::new_local(&[path]).expect("pack loads");
    assert_eq!(set.packs().len(), 7);
    assert_eq!(set.packs()[1].id, C_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[2].id, PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID);
    assert_eq!(set.packs()[3].id, PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID);
    assert_eq!(set.packs()[4].id, PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID);
    assert_eq!(set.packs()[5].id, FIRST_PARTY_VALUE_LAW_PACK_ID);
    let external = &set.packs()[6];
    assert_eq!(external.id, "com.example.pack");
    assert_eq!(external.hash, stable_symbol_hash("com.example.pack"));
    assert_eq!(external.trust, PackTrust::ExternalOptIn);
    assert_eq!(external.source, SemanticPackSource::LocalManifest);
    assert_eq!(external.influence, SemanticPackInfluence::MetadataOnly);
    assert_eq!(external.counts.contracts, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_reports_declared_fixture_files() {
    let dir = unique_dir("conformance_ok");
    let fixture_dir = dir.join("fixtures");
    fs::create_dir_all(&fixture_dir).unwrap();
    fs::write(
        fixture_dir.join("positive.py"),
        "import math\nmath.prod([1, 2])\n",
    )
    .unwrap();
    fs::write(
        fixture_dir.join("negative.py"),
        "math = object()\nmath.prod([1, 2])\n",
    )
    .unwrap();
    let path = dir.join("pack.json");
    fs::write(&path, manifest("com.example.pack")).unwrap();

    let report = check_semantic_pack_conformance(&[path]).expect("conformance loads");

    assert!(report.passed());
    assert_eq!(report.manifest_count(), 1);
    assert_eq!(report.positive_fixture_count(), 1);
    assert_eq!(report.hard_negative_count(), 1);
    assert_eq!(report.fixture_issue_count(), 0);
    let fixture_ids = report.manifests[0]
        .fixtures
        .iter()
        .map(|fixture| (fixture.kind.as_str(), fixture.id.as_str(), fixture.passed()))
        .collect::<Vec<_>>();
    assert_eq!(
        fixture_ids,
        vec![
            ("positive", "positive", true),
            ("hard-negative", "negative", true)
        ]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_fails_closed_on_missing_fixture_files() {
    let dir = unique_dir("conformance_missing");
    let path = dir.join("pack.json");
    fs::write(&path, manifest("com.example.pack")).unwrap();

    let report = check_semantic_pack_conformance(&[path]).expect("manifest is structurally valid");

    assert!(!report.passed());
    assert_eq!(report.fixture_issue_count(), 2);
    let issues = report.manifests[0]
        .fixtures
        .iter()
        .flat_map(|fixture| fixture.issues.iter().map(|issue| issue.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(issues, vec!["missing-file", "missing-file"]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_requires_fixture_paths_and_expectations() {
    let dir = unique_dir("conformance_metadata");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack")
            .replace(
                r#",
      "path": "fixtures/positive.py",
      "expectation": "exact-contract-open""#,
                "",
            )
            .replace(
                r#",
      "path": "fixtures/negative.py",
      "expectation": "exact-contract-closed""#,
                "",
            ),
    )
    .unwrap();

    let report = check_semantic_pack_conformance(&[path]).expect("manifest is structurally valid");

    assert!(!report.passed());
    assert_eq!(report.fixture_issue_count(), 4);
    let issues = report.manifests[0]
        .fixtures
        .iter()
        .flat_map(|fixture| fixture.issues.iter().map(|issue| issue.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        issues,
        vec![
            "missing-path",
            "missing-expectation",
            "missing-path",
            "missing-expectation"
        ]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn external_pack_enabled_by_default_is_rejected() {
    let dir = unique_dir("trust");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""trust": "external-opt-in",
    "enabled_by_default": false"#,
            r#""trust": "external-opt-in",
    "enabled_by_default": true"#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("must reject implicit external default");
    assert!(err
        .to_string()
        .contains("must be external-opt-in and disabled by default"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_claiming_first_party_trust_is_rejected() {
    let dir = unique_dir("first_party_trust");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""trust": "external-opt-in""#,
            r#""trust": "default-first-party""#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("local manifest must not claim first-party");
    assert!(err
        .to_string()
        .contains("must be external-opt-in and disabled by default"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn package_entries_must_match_manifest_shape() {
    let dir = unique_dir("package");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#"  "supported_languages": [{ "id": "python" }],
"#,
            r#"  "supported_languages": [{ "id": "python" }],
  "packages": [{ "ecosystem": "pypi", "name": "example" }],
"#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("package versions are required");
    assert!(err.to_string().contains("missing field `versions`"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn compatibility_nose_must_be_version_requirement_like() {
    let dir = unique_dir("compatibility");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""compatibility": { "nose": ">=0.5.0 <0.6.0" }"#,
            r#""compatibility": { "nose": "current stable" }"#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("version range should be comparable");
    assert!(err
        .to_string()
        .contains("unsupported version constraint `current`"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exact_capable_contracts_must_reference_positive_and_hard_negative_fixtures() {
    let dir = unique_dir("contract_fixture_refs");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""conformance_refs": ["positive", "negative"]"#,
            r#""conformance_refs": ["positive"]"#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path)
        .expect_err("exact-capable contracts need both fixture polarities");
    assert!(
        err.to_string()
            .contains("must reference at least one positive and one hard-negative"),
        "{err}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn value_law_semantics_must_be_an_object_even_when_not_exact_capable() {
    let dir = unique_dir("value_law_semantics_shape");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""value_laws": []"#,
            r#""value_laws": [{
      "id": "python.example.near-law",
      "requires": [],
      "semantics": "not an object",
      "channel": "near-only",
      "proof_status": "missing",
      "conformance_refs": []
    }]"#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("value law semantics must match schema");
    assert!(
        err.to_string().contains("semantics must be an object"),
        "{err}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exact_capable_contracts_must_have_required_evidence_requirements() {
    let dir = unique_dir("required_evidence_requirement");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(r#""required": true"#, r#""required": false"#),
    )
    .unwrap();
    let err =
        load_local_manifest(&path).expect_err("optional-only requirements must not open exact");
    assert!(
        err.to_string()
            .contains("must declare at least one required evidence requirement"),
        "{err}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn evidence_kind_must_match_schema_shape() {
    let dir = unique_dir("evidence_kind_shape");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""kind": "LibraryApi.Contract""#,
            r#""kind": "LibraryApi.""#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("empty evidence-kind suffix is invalid");
    assert!(
        err.to_string().contains("unknown kind `LibraryApi.`"),
        "{err}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_fixtures_must_use_manifest_relative_paths() {
    let dir = unique_dir("absolute_fixture_path");
    let outside = unique_dir("absolute_fixture_path_outside");
    let absolute_fixture = outside.join("positive.py");
    fs::write(&absolute_fixture, "print('external fixture')\n").unwrap();
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack")
            .replace("fixtures/positive.py", absolute_fixture.to_str().unwrap()),
    )
    .unwrap();

    let report = check_semantic_pack_conformance(&[path]).expect("manifest is structurally valid");

    assert!(!report.passed());
    let issues = report.manifests[0]
        .fixtures
        .iter()
        .flat_map(|fixture| fixture.issues.iter().map(|issue| issue.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(issues, vec!["absolute-path", "missing-file"]);
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn evidence_producer_anchors_must_be_known_anchor_names() {
    let dir = unique_dir("anchor");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack")
            .replace(r#""anchors": ["node"]"#, r#""anchors": ["raw-selector"]"#),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("unknown anchors must not load");
    assert!(err.to_string().contains("unknown variant"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn duplicate_pack_ids_fail_closed() {
    let dir = unique_dir("dupe");
    let one = dir.join("one.json");
    let two = dir.join("two.json");
    fs::write(&one, manifest("com.example.pack")).unwrap();
    fs::write(&two, manifest("com.example.pack")).unwrap();
    let err = SemanticPackSet::new_local(&[one, two]).expect_err("duplicate id");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_cannot_claim_compiled_first_party_pack_id() {
    let dir = unique_dir("compiled_first_party_id");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)).unwrap();
    let err = SemanticPackSet::new_local(&[path]).expect_err("compiled id is reserved");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_cannot_claim_compiled_first_party_pack_id() {
    let dir = unique_dir("compiled_first_party_conformance");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)).unwrap();
    let err = check_semantic_pack_conformance(&[path]).expect_err("compiled id is reserved");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn directory_discovery_sorts_json_manifests() {
    let dir = unique_dir("dir");
    fs::write(dir.join("b.json"), manifest("com.example.b")).unwrap();
    fs::write(dir.join("a.json"), manifest("com.example.a")).unwrap();
    let paths = discover_manifest_paths(std::slice::from_ref(&dir)).expect("discover");
    let names = paths
        .iter()
        .map(|path| path.file_name().unwrap().to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["a.json", "b.json"]);
    let _ = fs::remove_dir_all(dir);
}

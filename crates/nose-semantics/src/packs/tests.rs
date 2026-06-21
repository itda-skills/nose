use super::*;
use std::collections::HashSet;
use std::fs;

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nose_semantic_pack_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn assert_source_fact_language_descriptor(
    pack_id: &str,
    supported_languages: &[&str],
    langs: &[nose_il::Lang],
    file_extensions: &[&str],
    parser: &str,
    lowering_entrypoint: &str,
    core_producer_id: &str,
    source_fact_producer_id: &str,
) {
    let descriptor = builtin_pack_descriptor(pack_id).expect("language descriptor");
    assert_eq!(descriptor.kind, SemanticPackKind::LanguagePack);
    assert_eq!(descriptor.supported_languages, supported_languages);
    assert!(descriptor.supported_packages.is_empty());
    let language = descriptor
        .language
        .expect("language descriptor should expose binding metadata");
    assert_eq!(language.langs, langs);
    assert_eq!(language.file_extensions, file_extensions);
    assert_eq!(language.parser, parser);
    assert_eq!(language.lowering_entrypoint, lowering_entrypoint);
    assert_eq!(
        descriptor.evidence_producer_ids,
        &[core_producer_id, source_fact_producer_id]
    );
    assert_eq!(
        descriptor.source_fact_producer_ids,
        &[source_fact_producer_id]
    );
    assert!(descriptor.contract_ids.is_empty());
    assert_eq!(descriptor.counts().evidence_producers, 2);
    assert_eq!(descriptor.counts().contracts, 0);
    assert_eq!(descriptor.counts().positive_fixtures, 0);
    assert_eq!(descriptor.counts().hard_negatives, 0);
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

fn manifest_with_value_law(id: &str) -> String {
    manifest(id).replace(
        r#""value_laws": []"#,
        r#""value_laws": [{
      "id": "python.example.numeric-law",
      "requires": [{
        "ref": "Domain.Number",
        "subject": "operands",
        "required": true,
        "same_anchor_as": "value"
      }],
      "semantics": {
        "law": "x + 0 == x",
        "domain": "numeric-only",
        "demand": { "arguments": "preserve-original-expression-demand" },
        "effects": ["no-new-effects"]
      },
      "channel": "exact-proven",
      "proof_status": "proven",
      "conformance_refs": ["positive", "negative"]
    }]"#,
    )
}

#[test]
fn builtin_pack_descriptor_registry_names_current_compiled_packs() {
    let descriptors = builtin_pack_descriptors();
    assert_eq!(descriptors.len(), 42);
    let ids = descriptors
        .iter()
        .map(|descriptor| descriptor.id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            BUILTIN_COMPAT_PACK_ID,
            PYTHON_LANGUAGE_PACK_ID,
            JS_TS_LANGUAGE_PACK_ID,
            GO_LANGUAGE_PACK_ID,
            RUST_LANGUAGE_PACK_ID,
            JAVA_LANGUAGE_PACK_ID,
            C_LANGUAGE_PACK_ID,
            RUBY_LANGUAGE_PACK_ID,
            SWIFT_LANGUAGE_PACK_ID,
            CSS_LANGUAGE_PACK_ID,
            HTML_EMBEDDED_LANGUAGE_PACK_ID,
            PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
            PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
            PYTHON_STDLIB_MATH_PACK_ID,
            RUBY_STDLIB_SET_PACK_ID,
            RUST_STDLIB_VEC_PACK_ID,
            RUST_STDLIB_OPTION_PACK_ID,
            RUST_STDLIB_INTEGER_METHOD_PACK_ID,
            RUST_STDLIB_COLLECTION_FACTORY_PACK_ID,
            RUST_STDLIB_MAP_FACTORY_PACK_ID,
            JAVA_STDLIB_MATH_PACK_ID,
            JAVA_STDLIB_MAP_FACTORY_PACK_ID,
            JAVA_STDLIB_MAP_ENTRY_PACK_ID,
            JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID,
            JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
            JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
            MAP_GET_PROTOCOL_PACK_ID,
            MAP_GET_DEFAULT_PROTOCOL_PACK_ID,
            FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID,
            RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID,
            MAP_KEY_VIEW_PROTOCOL_PACK_ID,
            PROPERTY_BUILTIN_PROTOCOL_PACK_ID,
            BUILTIN_METHOD_CALL_PROTOCOL_PACK_ID,
            ITERATOR_IDENTITY_ADAPTER_PACK_ID,
            JS_LIKE_BUILTIN_PROMISE_PACK_ID,
            JS_LIKE_BUILTIN_ARRAY_PACK_ID,
            JS_LIKE_BUILTIN_BOOLEAN_PACK_ID,
            JS_LIKE_BUILTIN_REGEX_PACK_ID,
            JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
            JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
            PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID,
            VALUE_GRAPH_LAW_PACK_ID
        ]
    );
    assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    assert!(descriptors
        .iter()
        .all(|descriptor| descriptor.trust == PackTrust::BuiltinDefault));
    assert!(descriptors
        .iter()
        .all(|descriptor| descriptor.enabled_by_default));
}

#[test]
fn builtin_pack_descriptors_enumerate_declarations_and_conformance_refs() {
    assert_source_fact_language_descriptor(
        PYTHON_LANGUAGE_PACK_ID,
        &["python"],
        &[nose_il::Lang::Python],
        &["py", "pyi"],
        "tree-sitter-python",
        "nose_frontend::python::lower",
        PYTHON_LANGUAGE_CORE_PRODUCER_ID,
        PYTHON_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        JS_TS_LANGUAGE_PACK_ID,
        &["javascript", "typescript"],
        &[nose_il::Lang::JavaScript, nose_il::Lang::TypeScript],
        &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"],
        "tree-sitter-javascript/tree-sitter-typescript",
        "nose_frontend::js_ts::lower",
        JS_TS_LANGUAGE_CORE_PRODUCER_ID,
        JS_TS_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        GO_LANGUAGE_PACK_ID,
        &["go"],
        &[nose_il::Lang::Go],
        &["go"],
        "tree-sitter-go",
        "nose_frontend::go::lower",
        GO_LANGUAGE_CORE_PRODUCER_ID,
        GO_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        RUST_LANGUAGE_PACK_ID,
        &["rust"],
        &[nose_il::Lang::Rust],
        &["rs"],
        "tree-sitter-rust",
        "nose_frontend::rust::lower",
        RUST_LANGUAGE_CORE_PRODUCER_ID,
        RUST_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        JAVA_LANGUAGE_PACK_ID,
        &["java"],
        &[nose_il::Lang::Java],
        &["java"],
        "tree-sitter-java",
        "nose_frontend::java::lower",
        JAVA_LANGUAGE_CORE_PRODUCER_ID,
        JAVA_SOURCE_FACT_PRODUCER_ID,
    );
    let c = builtin_pack_descriptor(C_LANGUAGE_PACK_ID).expect("C language descriptor");
    assert_eq!(c.kind, SemanticPackKind::LanguagePack);
    assert_eq!(c.supported_languages, &["c"]);
    assert!(c.supported_packages.is_empty());
    let language = c
        .language
        .expect("C descriptor should expose language binding");
    assert_eq!(language.langs, &[nose_il::Lang::C]);
    assert_eq!(language.file_extensions, &["c", "h"]);
    assert_eq!(language.parser, "tree-sitter-c");
    assert_eq!(language.lowering_entrypoint, "nose_frontend::c::lower");
    assert_eq!(
        c.evidence_producer_ids,
        &[
            C_LANGUAGE_CORE_PRODUCER_ID,
            C_SOURCE_FACT_PRODUCER_ID,
            C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID
        ]
    );
    assert_eq!(
        c.source_fact_producer_ids,
        &[
            C_SOURCE_FACT_PRODUCER_ID,
            C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID
        ]
    );
    assert_eq!(c.counts().evidence_producers, 3);
    assert_eq!(c.counts().contracts, 0);
    assert_eq!(c.counts().positive_fixtures, 2);
    assert_eq!(c.counts().hard_negatives, 2);
    assert!(c
        .conformance_refs()
        .contains(&"c-unsigned32-signed-cast-hard-negative"));
    assert_source_fact_language_descriptor(
        RUBY_LANGUAGE_PACK_ID,
        &["ruby"],
        &[nose_il::Lang::Ruby],
        &["rb"],
        "tree-sitter-ruby",
        "nose_frontend::ruby::lower",
        RUBY_LANGUAGE_CORE_PRODUCER_ID,
        RUBY_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        SWIFT_LANGUAGE_PACK_ID,
        &["swift"],
        &[nose_il::Lang::Swift],
        &["swift"],
        "tree-sitter-swift",
        "nose_frontend::swift::lower",
        SWIFT_LANGUAGE_CORE_PRODUCER_ID,
        SWIFT_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        CSS_LANGUAGE_PACK_ID,
        &["css"],
        &[nose_il::Lang::Css],
        &["css"],
        "tree-sitter-css",
        "nose_frontend::css::lower",
        CSS_LANGUAGE_CORE_PRODUCER_ID,
        CSS_SOURCE_FACT_PRODUCER_ID,
    );
    assert_source_fact_language_descriptor(
        HTML_EMBEDDED_LANGUAGE_PACK_ID,
        &["html", "vue", "svelte"],
        &[
            nose_il::Lang::Html,
            nose_il::Lang::Vue,
            nose_il::Lang::Svelte,
        ],
        &["html", "htm", "vue", "svelte"],
        "tree-sitter-html + embedded JS/TS/CSS extraction",
        "nose_frontend::embedded::lower_regions",
        HTML_EMBEDDED_LANGUAGE_CORE_PRODUCER_ID,
        HTML_EMBEDDED_SOURCE_FACT_PRODUCER_ID,
    );

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

    let python_stdlib_math =
        builtin_pack_descriptor(PYTHON_STDLIB_MATH_PACK_ID).expect("Python stdlib math descriptor");
    assert_eq!(python_stdlib_math.kind, SemanticPackKind::StdlibPack);
    assert_eq!(python_stdlib_math.supported_languages, &["python"]);
    assert_eq!(python_stdlib_math.supported_packages, &["math"]);
    assert_eq!(
        python_stdlib_math.evidence_producer_ids,
        &[PYTHON_STDLIB_MATH_PRODUCER_ID]
    );
    assert!(python_stdlib_math.source_fact_producer_ids.is_empty());
    assert_eq!(
        python_stdlib_math.contract_ids,
        &[PYTHON_STDLIB_MATH_PROD_CONTRACT_ID]
    );
    assert_eq!(python_stdlib_math.counts().evidence_producers, 1);
    assert_eq!(python_stdlib_math.counts().contracts, 1);
    assert_eq!(python_stdlib_math.counts().positive_fixtures, 1);
    assert_eq!(python_stdlib_math.counts().hard_negatives, 2);
    assert!(python_stdlib_math
        .conformance_refs()
        .contains(&"python-math-prod-wrong-namespace-hard-negative"));

    let ruby_set =
        builtin_pack_descriptor(RUBY_STDLIB_SET_PACK_ID).expect("Ruby stdlib Set descriptor");
    assert_eq!(ruby_set.kind, SemanticPackKind::StdlibPack);
    assert_eq!(ruby_set.supported_languages, &["ruby"]);
    assert_eq!(ruby_set.supported_packages, &["set"]);
    assert_eq!(
        ruby_set.evidence_producer_ids,
        &[RUBY_STDLIB_SET_PRODUCER_ID]
    );
    assert!(ruby_set.source_fact_producer_ids.is_empty());
    assert_eq!(ruby_set.contract_ids, &[RUBY_STDLIB_SET_CONTRACT_ID]);
    assert_eq!(ruby_set.counts().evidence_producers, 1);
    assert_eq!(ruby_set.counts().contracts, 1);
    assert_eq!(ruby_set.counts().positive_fixtures, 3);
    assert_eq!(ruby_set.counts().hard_negatives, 3);
    assert!(ruby_set
        .conformance_refs()
        .contains(&"ruby-set-missing-require-hard-negative"));

    let rust_vec =
        builtin_pack_descriptor(RUST_STDLIB_VEC_PACK_ID).expect("Rust stdlib Vec descriptor");
    assert_eq!(rust_vec.kind, SemanticPackKind::StdlibPack);
    assert_eq!(rust_vec.supported_languages, &["rust"]);
    assert_eq!(rust_vec.supported_packages, &["std::vec", "alloc::vec"]);
    assert_eq!(
        rust_vec.evidence_producer_ids,
        &[RUST_STDLIB_VEC_PRODUCER_ID]
    );
    assert!(rust_vec.source_fact_producer_ids.is_empty());
    assert_eq!(
        rust_vec.contract_ids,
        &[
            RUST_STDLIB_VEC_MACRO_CONTRACT_ID,
            RUST_STDLIB_VEC_NEW_CONTRACT_ID
        ]
    );
    assert_eq!(rust_vec.counts().evidence_producers, 1);
    assert_eq!(rust_vec.counts().contracts, 2);
    assert_eq!(rust_vec.counts().positive_fixtures, 2);
    assert_eq!(rust_vec.counts().hard_negatives, 2);
    assert!(rust_vec
        .conformance_refs()
        .contains(&"rust-vec-new-shadowed-hard-negative"));

    let rust_option =
        builtin_pack_descriptor(RUST_STDLIB_OPTION_PACK_ID).expect("Rust stdlib Option descriptor");
    assert_eq!(rust_option.kind, SemanticPackKind::StdlibPack);
    assert_eq!(rust_option.supported_languages, &["rust"]);
    assert_eq!(
        rust_option.supported_packages,
        &["std::option", "core::option"]
    );
    assert_eq!(
        rust_option.evidence_producer_ids,
        &[RUST_STDLIB_OPTION_PRODUCER_ID]
    );
    assert!(rust_option.source_fact_producer_ids.is_empty());
    assert_eq!(
        rust_option.contract_ids,
        &[
            RUST_STDLIB_OPTION_SOME_CONTRACT_ID,
            RUST_STDLIB_OPTION_NONE_CONTRACT_ID,
            RUST_STDLIB_OPTION_AND_THEN_CONTRACT_ID,
        ]
    );
    assert_eq!(rust_option.counts().evidence_producers, 1);
    assert_eq!(rust_option.counts().contracts, 3);
    assert_eq!(rust_option.counts().positive_fixtures, 3);
    assert_eq!(rust_option.counts().hard_negatives, 3);
    assert!(rust_option
        .conformance_refs()
        .contains(&"rust-option-and-then-non-option-hard-negative"));

    let rust_integer_methods = builtin_pack_descriptor(RUST_STDLIB_INTEGER_METHOD_PACK_ID)
        .expect("Rust stdlib integer method descriptor");
    assert_eq!(rust_integer_methods.kind, SemanticPackKind::StdlibPack);
    assert_eq!(rust_integer_methods.supported_languages, &["rust"]);
    assert_eq!(
        rust_integer_methods.supported_packages,
        &["core::primitive"]
    );
    assert_eq!(
        rust_integer_methods.evidence_producer_ids,
        &[RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID]
    );
    assert!(rust_integer_methods.source_fact_producer_ids.is_empty());
    assert_eq!(
        rust_integer_methods.contract_ids,
        &[
            SCALAR_INTEGER_METHOD_ABS_CONTRACT_ID,
            SCALAR_INTEGER_METHOD_MIN_CONTRACT_ID,
            SCALAR_INTEGER_METHOD_MAX_CONTRACT_ID,
            SCALAR_INTEGER_METHOD_CLAMP_CONTRACT_ID,
        ]
    );
    assert_eq!(rust_integer_methods.counts().evidence_producers, 1);
    assert_eq!(rust_integer_methods.counts().contracts, 4);
    assert_eq!(rust_integer_methods.counts().positive_fixtures, 4);
    assert_eq!(rust_integer_methods.counts().hard_negatives, 2);
    assert!(rust_integer_methods
        .conformance_refs()
        .contains(&"rust-integer-method-non-integer-receiver-hard-negative"));

    let rust_stdlib_collections = builtin_pack_descriptor(RUST_STDLIB_COLLECTION_FACTORY_PACK_ID)
        .expect("Rust stdlib collection factory descriptor");
    assert_eq!(rust_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(rust_stdlib_collections.supported_languages, &["rust"]);
    assert_eq!(
        rust_stdlib_collections.supported_packages,
        &["std::collections"]
    );
    assert_eq!(
        rust_stdlib_collections.evidence_producer_ids,
        &[RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID]
    );
    assert!(rust_stdlib_collections.source_fact_producer_ids.is_empty());
    assert_eq!(
        rust_stdlib_collections.contract_ids,
        &[RUST_STDLIB_COLLECTION_FACTORY_CONTRACT_ID]
    );
    assert_eq!(rust_stdlib_collections.counts().evidence_producers, 1);
    assert_eq!(rust_stdlib_collections.counts().contracts, 1);
    assert_eq!(rust_stdlib_collections.counts().positive_fixtures, 3);
    assert_eq!(rust_stdlib_collections.counts().hard_negatives, 2);
    assert!(rust_stdlib_collections
        .conformance_refs()
        .contains(&"rust-std-collections-shadowed-std-hard-negative"));

    let rust_stdlib_maps = builtin_pack_descriptor(RUST_STDLIB_MAP_FACTORY_PACK_ID)
        .expect("Rust stdlib map factory descriptor");
    assert_eq!(rust_stdlib_maps.kind, SemanticPackKind::StdlibPack);
    assert_eq!(rust_stdlib_maps.supported_languages, &["rust"]);
    assert_eq!(rust_stdlib_maps.supported_packages, &["std::collections"]);
    assert_eq!(
        rust_stdlib_maps.evidence_producer_ids,
        &[RUST_STDLIB_MAP_FACTORY_PRODUCER_ID]
    );
    assert!(rust_stdlib_maps.source_fact_producer_ids.is_empty());
    assert_eq!(
        rust_stdlib_maps.contract_ids,
        &[RUST_STDLIB_MAP_FACTORY_CONTRACT_ID]
    );
    assert_eq!(rust_stdlib_maps.counts().evidence_producers, 1);
    assert_eq!(rust_stdlib_maps.counts().contracts, 1);
    assert_eq!(rust_stdlib_maps.counts().positive_fixtures, 2);
    assert_eq!(rust_stdlib_maps.counts().hard_negatives, 2);
    assert!(rust_stdlib_maps
        .conformance_refs()
        .contains(&"rust-std-map-shadowed-std-hard-negative"));

    let java_stdlib_math =
        builtin_pack_descriptor(JAVA_STDLIB_MATH_PACK_ID).expect("Java stdlib Math descriptor");
    assert_eq!(java_stdlib_math.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_stdlib_math.supported_languages, &["java"]);
    assert_eq!(java_stdlib_math.supported_packages, &["java.lang"]);
    assert_eq!(
        java_stdlib_math.evidence_producer_ids,
        &[JAVA_STDLIB_MATH_PRODUCER_ID]
    );
    assert!(java_stdlib_math.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_stdlib_math.contract_ids,
        &[
            SCALAR_INTEGER_METHOD_ABS_CONTRACT_ID,
            SCALAR_INTEGER_METHOD_MIN_CONTRACT_ID,
            SCALAR_INTEGER_METHOD_MAX_CONTRACT_ID
        ]
    );
    assert_eq!(java_stdlib_math.counts().evidence_producers, 1);
    assert_eq!(java_stdlib_math.counts().contracts, 3);
    assert_eq!(java_stdlib_math.counts().positive_fixtures, 3);
    assert_eq!(java_stdlib_math.counts().hard_negatives, 3);
    assert!(java_stdlib_math
        .conformance_refs()
        .contains(&"java-math-shadowed-math-hard-negative"));
    assert!(java_stdlib_math
        .conformance_refs()
        .contains(&"java-math-non-integer-argument-hard-negative"));
    assert!(!java_stdlib_math
        .contract_ids
        .contains(&SCALAR_INTEGER_METHOD_CLAMP_CONTRACT_ID));

    let java_stdlib_maps = builtin_pack_descriptor(JAVA_STDLIB_MAP_FACTORY_PACK_ID)
        .expect("Java stdlib map factory descriptor");
    assert_eq!(java_stdlib_maps.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_stdlib_maps.supported_languages, &["java"]);
    assert_eq!(java_stdlib_maps.supported_packages, &["java.util"]);
    assert_eq!(
        java_stdlib_maps.evidence_producer_ids,
        &[JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID]
    );
    assert!(java_stdlib_maps.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_stdlib_maps.contract_ids,
        &[
            JAVA_STDLIB_MAP_FACTORY_OF_CONTRACT_ID,
            JAVA_STDLIB_MAP_FACTORY_OF_ENTRIES_CONTRACT_ID
        ]
    );
    assert_eq!(java_stdlib_maps.counts().evidence_producers, 1);
    assert_eq!(java_stdlib_maps.counts().contracts, 2);
    assert_eq!(java_stdlib_maps.counts().positive_fixtures, 2);
    assert_eq!(java_stdlib_maps.counts().hard_negatives, 2);
    assert!(java_stdlib_maps
        .conformance_refs()
        .contains(&"java-map-missing-import-hard-negative"));
    assert!(!java_stdlib_maps
        .contract_ids
        .contains(&"java.map_entry_factory"));

    let java_stdlib_entries = builtin_pack_descriptor(JAVA_STDLIB_MAP_ENTRY_PACK_ID)
        .expect("Java stdlib map entry descriptor");
    assert_eq!(java_stdlib_entries.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_stdlib_entries.supported_languages, &["java"]);
    assert_eq!(java_stdlib_entries.supported_packages, &["java.util"]);
    assert_eq!(
        java_stdlib_entries.evidence_producer_ids,
        &[JAVA_STDLIB_MAP_ENTRY_PRODUCER_ID]
    );
    assert!(java_stdlib_entries.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_stdlib_entries.contract_ids,
        &[JAVA_STDLIB_MAP_ENTRY_CONTRACT_ID]
    );
    assert_eq!(java_stdlib_entries.counts().evidence_producers, 1);
    assert_eq!(java_stdlib_entries.counts().contracts, 1);
    assert_eq!(java_stdlib_entries.counts().positive_fixtures, 1);
    assert_eq!(java_stdlib_entries.counts().hard_negatives, 2);
    assert!(java_stdlib_entries
        .conformance_refs()
        .contains(&"java-map-entry-shadowed-map-hard-negative"));

    let java_stdlib_collections = builtin_pack_descriptor(JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID)
        .expect("Java stdlib collection factory descriptor");
    assert_eq!(java_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_stdlib_collections.supported_languages, &["java"]);
    assert_eq!(java_stdlib_collections.supported_packages, &["java.util"]);
    assert_eq!(
        java_stdlib_collections.evidence_producer_ids,
        &[JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID]
    );
    assert!(java_stdlib_collections.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_stdlib_collections.contract_ids,
        &[
            JAVA_STDLIB_COLLECTION_FACTORY_LIST_OF_CONTRACT_ID,
            JAVA_STDLIB_COLLECTION_FACTORY_SET_OF_CONTRACT_ID,
            JAVA_STDLIB_COLLECTION_FACTORY_ARRAYS_AS_LIST_CONTRACT_ID
        ]
    );
    assert_eq!(java_stdlib_collections.counts().evidence_producers, 1);
    assert_eq!(java_stdlib_collections.counts().contracts, 3);
    assert_eq!(java_stdlib_collections.counts().positive_fixtures, 3);
    assert_eq!(java_stdlib_collections.counts().hard_negatives, 2);
    assert!(java_stdlib_collections
        .conformance_refs()
        .contains(&"java-collection-missing-import-hard-negative"));
    assert!(!java_stdlib_collections
        .contract_ids
        .contains(&"java.collection_constructor.empty_list"));

    let java_stdlib_constructors =
        builtin_pack_descriptor(JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID)
            .expect("Java stdlib collection constructor descriptor");
    assert_eq!(java_stdlib_constructors.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_stdlib_constructors.supported_languages, &["java"]);
    assert_eq!(java_stdlib_constructors.supported_packages, &["java.util"]);
    assert_eq!(
        java_stdlib_constructors.evidence_producer_ids,
        &[JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID]
    );
    assert!(java_stdlib_constructors.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_stdlib_constructors.contract_ids,
        &[JAVA_STDLIB_COLLECTION_CONSTRUCTOR_EMPTY_LIST_CONTRACT_ID]
    );
    assert_eq!(java_stdlib_constructors.counts().evidence_producers, 1);
    assert_eq!(java_stdlib_constructors.counts().contracts, 1);
    assert_eq!(java_stdlib_constructors.counts().positive_fixtures, 2);
    assert_eq!(java_stdlib_constructors.counts().hard_negatives, 3);
    assert!(java_stdlib_constructors
        .conformance_refs()
        .contains(&"java-constructor-conflicting-import-hard-negative"));

    let java_static_adapters =
        builtin_pack_descriptor(JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID)
            .expect("Java stdlib static collection adapter descriptor");
    assert_eq!(java_static_adapters.kind, SemanticPackKind::StdlibPack);
    assert_eq!(java_static_adapters.supported_languages, &["java"]);
    assert_eq!(java_static_adapters.supported_packages, &["java.util"]);
    assert_eq!(
        java_static_adapters.evidence_producer_ids,
        &[JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_ID]
    );
    assert!(java_static_adapters.source_fact_producer_ids.is_empty());
    assert_eq!(
        java_static_adapters.contract_ids,
        &[JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONTRACT_ID]
    );
    assert_eq!(java_static_adapters.counts().evidence_producers, 1);
    assert_eq!(java_static_adapters.counts().contracts, 1);
    assert_eq!(java_static_adapters.counts().positive_fixtures, 1);
    assert_eq!(java_static_adapters.counts().hard_negatives, 2);
    assert!(java_static_adapters
        .conformance_refs()
        .contains(&"java-arrays-stream-shadowed-arrays-hard-negative"));

    let map_get =
        builtin_pack_descriptor(MAP_GET_PROTOCOL_PACK_ID).expect("map-get protocol descriptor");
    assert_eq!(map_get.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_get.supported_languages,
        &[
            "java",
            "rust",
            "javascript",
            "typescript",
            "vue",
            "svelte",
            "html"
        ]
    );
    assert_eq!(
        map_get.supported_packages,
        &["Map", "java.util", "std::collections"]
    );
    assert_eq!(
        map_get.evidence_producer_ids,
        &[MAP_GET_PROTOCOL_PRODUCER_ID]
    );
    assert!(map_get.source_fact_producer_ids.is_empty());
    assert_eq!(map_get.contract_ids, &[MAP_GET_CONTRACT_ID]);
    assert_eq!(map_get.counts().evidence_producers, 1);
    assert_eq!(map_get.counts().contracts, 1);
    assert_eq!(map_get.counts().positive_fixtures, 3);
    assert_eq!(map_get.counts().hard_negatives, 2);
    assert!(map_get
        .conformance_refs()
        .contains(&"map-get-non-map-receiver-hard-negative"));

    let map_get_default = builtin_pack_descriptor(MAP_GET_DEFAULT_PROTOCOL_PACK_ID)
        .expect("map-get-default protocol descriptor");
    assert_eq!(map_get_default.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_get_default.supported_languages,
        &["python", "ruby", "java"]
    );
    assert_eq!(
        map_get_default.supported_packages,
        &["dict", "Hash", "java.util"]
    );
    assert_eq!(
        map_get_default.evidence_producer_ids,
        &[MAP_GET_DEFAULT_PROTOCOL_PRODUCER_ID]
    );
    assert!(map_get_default.source_fact_producer_ids.is_empty());
    assert_eq!(map_get_default.contract_ids, &[MAP_GET_DEFAULT_CONTRACT_ID]);
    assert_eq!(map_get_default.counts().evidence_producers, 1);
    assert_eq!(map_get_default.counts().contracts, 1);
    assert_eq!(map_get_default.counts().positive_fixtures, 3);
    assert_eq!(map_get_default.counts().hard_negatives, 2);
    assert!(map_get_default
        .conformance_refs()
        .contains(&"map-get-default-non-map-receiver-hard-negative"));

    let free_function_builtin = builtin_pack_descriptor(FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID)
        .expect("free-function builtin protocol descriptor");
    assert_eq!(free_function_builtin.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        free_function_builtin.supported_languages,
        &["python", "go", "swift"]
    );
    assert_eq!(
        free_function_builtin.supported_packages,
        &["builtins", "go.predeclared", "Swift"]
    );
    assert_eq!(
        free_function_builtin.evidence_producer_ids,
        &[FREE_FUNCTION_BUILTIN_PROTOCOL_PRODUCER_ID]
    );
    assert!(free_function_builtin.source_fact_producer_ids.is_empty());
    assert_eq!(
        free_function_builtin.contract_ids,
        &[FREE_FUNCTION_BUILTIN_CONTRACT_ID]
    );
    assert_eq!(free_function_builtin.counts().evidence_producers, 1);
    assert_eq!(free_function_builtin.counts().contracts, 1);
    assert_eq!(free_function_builtin.counts().positive_fixtures, 6);
    assert_eq!(free_function_builtin.counts().hard_negatives, 4);
    assert!(free_function_builtin
        .conformance_refs()
        .contains(&"free-function-builtin-compatibility-pack-hard-negative"));

    let builtin_method_call = builtin_pack_descriptor(BUILTIN_METHOD_CALL_PROTOCOL_PACK_ID)
        .expect("builtin method-call protocol descriptor");
    assert_eq!(builtin_method_call.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        builtin_method_call.supported_languages,
        &[
            "python",
            "javascript",
            "typescript",
            "vue",
            "svelte",
            "html",
            "go",
            "rust",
            "java",
            "ruby",
            "swift"
        ]
    );
    assert_eq!(
        builtin_method_call.supported_packages,
        &[
            "Collection",
            "Option",
            "String",
            "console",
            "fmt",
            "functools",
            "slices",
            "strings"
        ]
    );
    assert_eq!(
        builtin_method_call.evidence_producer_ids,
        &[BUILTIN_METHOD_CALL_PROTOCOL_PRODUCER_ID]
    );
    assert!(builtin_method_call.source_fact_producer_ids.is_empty());
    assert_eq!(
        builtin_method_call.contract_ids,
        &[BUILTIN_METHOD_CALL_CONTRACT_ID]
    );
    assert_eq!(builtin_method_call.counts().evidence_producers, 1);
    assert_eq!(builtin_method_call.counts().contracts, 1);
    assert_eq!(builtin_method_call.counts().positive_fixtures, 9);
    assert_eq!(builtin_method_call.counts().hard_negatives, 3);
    assert!(builtin_method_call
        .conformance_refs()
        .contains(&"builtin-method-call-wrong-pack-hard-negative"));

    let property_builtin = builtin_pack_descriptor(PROPERTY_BUILTIN_PROTOCOL_PACK_ID)
        .expect("property builtin protocol descriptor");
    assert_eq!(property_builtin.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        property_builtin.supported_languages,
        &[
            "javascript",
            "typescript",
            "vue",
            "svelte",
            "html",
            "java",
            "swift"
        ]
    );
    assert_eq!(
        property_builtin.supported_packages,
        &["Array", "Collection", "Swift.Collection", "java.lang"]
    );
    assert_eq!(
        property_builtin.evidence_producer_ids,
        &[PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID]
    );
    assert!(property_builtin.source_fact_producer_ids.is_empty());
    assert_eq!(
        property_builtin.contract_ids,
        &[
            PROPERTY_BUILTIN_LEN_CONTRACT_ID,
            PROPERTY_BUILTIN_IS_EMPTY_CONTRACT_ID
        ]
    );
    assert_eq!(property_builtin.counts().evidence_producers, 1);
    assert_eq!(property_builtin.counts().contracts, 2);
    assert_eq!(property_builtin.counts().positive_fixtures, 4);
    assert_eq!(property_builtin.counts().hard_negatives, 3);
    assert!(property_builtin
        .conformance_refs()
        .contains(&"property-builtin-wrong-pack-hard-negative"));

    let receiver_membership = builtin_pack_descriptor(RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID)
        .expect("receiver-membership protocol descriptor");
    assert_eq!(receiver_membership.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        receiver_membership.supported_languages,
        &[
            "python",
            "ruby",
            "java",
            "rust",
            "swift",
            "javascript",
            "typescript",
            "vue",
            "svelte",
            "html"
        ]
    );
    assert_eq!(
        receiver_membership.supported_packages,
        &[
            "Array",
            "Collection",
            "Hash",
            "Map",
            "Set",
            "Swift.Collection",
            "dict",
            "java.util",
            "std::collections"
        ]
    );
    assert_eq!(
        receiver_membership.evidence_producer_ids,
        &[RECEIVER_MEMBERSHIP_PROTOCOL_PRODUCER_ID]
    );
    assert!(receiver_membership.source_fact_producer_ids.is_empty());
    assert_eq!(
        receiver_membership.contract_ids,
        &[RECEIVER_MEMBERSHIP_CONTRACT_ID]
    );
    assert_eq!(receiver_membership.counts().evidence_producers, 1);
    assert_eq!(receiver_membership.counts().contracts, 1);
    assert_eq!(receiver_membership.counts().positive_fixtures, 10);
    assert_eq!(receiver_membership.counts().hard_negatives, 3);
    assert!(receiver_membership
        .conformance_refs()
        .contains(&"receiver-membership-go-slices-contains-out-of-scope-hard-negative"));

    let map_key_view = builtin_pack_descriptor(MAP_KEY_VIEW_PROTOCOL_PACK_ID)
        .expect("map-key-view protocol descriptor");
    assert_eq!(map_key_view.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_key_view.supported_languages,
        &[
            "python",
            "ruby",
            "java",
            "javascript",
            "typescript",
            "vue",
            "svelte",
            "html"
        ]
    );
    assert_eq!(
        map_key_view.supported_packages,
        &["dict", "Hash", "Map", "java.util"]
    );
    assert_eq!(
        map_key_view.evidence_producer_ids,
        &[MAP_KEY_VIEW_PROTOCOL_PRODUCER_ID]
    );
    assert!(map_key_view.source_fact_producer_ids.is_empty());
    assert_eq!(
        map_key_view.contract_ids,
        &[
            MAP_KEY_VIEW_COLLECTION_CONTRACT_ID,
            MAP_KEY_VIEW_ITERATOR_CONTRACT_ID
        ]
    );
    assert_eq!(map_key_view.counts().evidence_producers, 1);
    assert_eq!(map_key_view.counts().contracts, 2);
    assert_eq!(map_key_view.counts().positive_fixtures, 4);
    assert_eq!(map_key_view.counts().hard_negatives, 2);
    assert!(map_key_view
        .conformance_refs()
        .contains(&"map-key-view-non-map-receiver-hard-negative"));

    let iterator_identity_adapters = builtin_pack_descriptor(ITERATOR_IDENTITY_ADAPTER_PACK_ID)
        .expect("iterator identity adapter protocol descriptor");
    assert_eq!(
        iterator_identity_adapters.kind,
        SemanticPackKind::ProtocolPack
    );
    assert_eq!(
        iterator_identity_adapters.supported_languages,
        &["java", "rust"]
    );
    assert_eq!(
        iterator_identity_adapters.supported_packages,
        &["core::iter", "java.util.stream"]
    );
    assert_eq!(
        iterator_identity_adapters.evidence_producer_ids,
        &[ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID]
    );
    assert!(iterator_identity_adapters
        .source_fact_producer_ids
        .is_empty());
    assert_eq!(
        iterator_identity_adapters.contract_ids,
        &[ITERATOR_IDENTITY_ADAPTER_CONTRACT_ID]
    );
    assert_eq!(iterator_identity_adapters.counts().evidence_producers, 1);
    assert_eq!(iterator_identity_adapters.counts().contracts, 1);
    assert_eq!(iterator_identity_adapters.counts().positive_fixtures, 3);
    assert_eq!(iterator_identity_adapters.counts().hard_negatives, 2);
    assert!(iterator_identity_adapters
        .conformance_refs()
        .contains(&"iterator-identity-non-iterable-receiver-hard-negative"));

    let js_promise = builtin_pack_descriptor(JS_LIKE_BUILTIN_PROMISE_PACK_ID)
        .expect("JavaScript builtins Promise descriptor");
    assert_eq!(js_promise.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_promise.supported_languages,
        &["javascript", "typescript"]
    );
    assert_eq!(js_promise.supported_packages, &["Promise"]);
    assert_eq!(
        js_promise.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID]
    );
    assert!(js_promise.source_fact_producer_ids.is_empty());
    assert_eq!(
        js_promise.contract_ids,
        &[
            JS_LIKE_BUILTIN_PROMISE_RESOLVE_CONTRACT_ID,
            JS_LIKE_BUILTIN_PROMISE_THEN_CONTRACT_ID,
        ]
    );
    assert_eq!(js_promise.counts().evidence_producers, 1);
    assert_eq!(js_promise.counts().contracts, 2);
    assert_eq!(js_promise.counts().positive_fixtures, 2);
    assert_eq!(js_promise.counts().hard_negatives, 3);
    assert!(js_promise
        .conformance_refs()
        .contains(&"js-promise-resolve-shadowed-hard-negative"));

    let js_array = builtin_pack_descriptor(JS_LIKE_BUILTIN_ARRAY_PACK_ID)
        .expect("JavaScript builtins Array descriptor");
    assert_eq!(js_array.kind, SemanticPackKind::StdlibPack);
    assert_eq!(js_array.supported_languages, &["javascript", "typescript"]);
    assert_eq!(js_array.supported_packages, &["Array"]);
    assert_eq!(
        js_array.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID]
    );
    assert!(js_array.source_fact_producer_ids.is_empty());
    assert_eq!(
        js_array.contract_ids,
        &[
            JS_LIKE_BUILTIN_ARRAY_FROM_CONTRACT_ID,
            JS_LIKE_BUILTIN_ARRAY_IS_ARRAY_CONTRACT_ID,
        ]
    );
    assert_eq!(js_array.counts().evidence_producers, 1);
    assert_eq!(js_array.counts().contracts, 2);
    assert_eq!(js_array.counts().positive_fixtures, 2);
    assert_eq!(js_array.counts().hard_negatives, 3);
    assert!(js_array
        .conformance_refs()
        .contains(&"js-array-from-shadowed-hard-negative"));

    let js_boolean = builtin_pack_descriptor(JS_LIKE_BUILTIN_BOOLEAN_PACK_ID)
        .expect("JavaScript builtins Boolean descriptor");
    assert_eq!(js_boolean.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_boolean.supported_languages,
        &["javascript", "typescript"]
    );
    assert_eq!(js_boolean.supported_packages, &["Boolean"]);
    assert_eq!(
        js_boolean.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_ID]
    );
    assert!(js_boolean.source_fact_producer_ids.is_empty());
    assert_eq!(
        js_boolean.contract_ids,
        &[JS_LIKE_BUILTIN_BOOLEAN_CONTRACT_ID]
    );
    assert_eq!(js_boolean.counts().evidence_producers, 1);
    assert_eq!(js_boolean.counts().contracts, 1);
    assert_eq!(js_boolean.counts().positive_fixtures, 1);
    assert_eq!(js_boolean.counts().hard_negatives, 2);
    assert!(js_boolean
        .conformance_refs()
        .contains(&"js-boolean-coercion-shadowed-hard-negative"));

    let js_regex = builtin_pack_descriptor(JS_LIKE_BUILTIN_REGEX_PACK_ID)
        .expect("JavaScript builtins RegExp descriptor");
    assert_eq!(js_regex.kind, SemanticPackKind::StdlibPack);
    assert_eq!(js_regex.supported_languages, &["javascript", "typescript"]);
    assert_eq!(js_regex.supported_packages, &["RegExp"]);
    assert_eq!(
        js_regex.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_REGEX_PRODUCER_ID]
    );
    assert!(js_regex.source_fact_producer_ids.is_empty());
    assert_eq!(
        js_regex.contract_ids,
        &[JS_LIKE_BUILTIN_REGEX_TEST_CONTRACT_ID]
    );
    assert_eq!(js_regex.counts().evidence_producers, 1);
    assert_eq!(js_regex.counts().contracts, 1);
    assert_eq!(js_regex.counts().positive_fixtures, 1);
    assert_eq!(js_regex.counts().hard_negatives, 2);
    assert!(js_regex
        .conformance_refs()
        .contains(&"js-regex-test-string-receiver-hard-negative"));

    let js_static_index = builtin_pack_descriptor(JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID)
        .expect("JavaScript builtins static index membership descriptor");
    assert_eq!(js_static_index.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_static_index.supported_languages,
        &["javascript", "typescript"]
    );
    assert_eq!(js_static_index.supported_packages, &["Array"]);
    assert_eq!(
        js_static_index.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_ID]
    );
    assert!(js_static_index.source_fact_producer_ids.is_empty());
    assert_eq!(
        js_static_index.contract_ids,
        &[
            JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_INDEX_OF_CONTRACT_ID,
            JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_FIND_INDEX_CONTRACT_ID,
        ]
    );
    assert_eq!(js_static_index.counts().evidence_producers, 1);
    assert_eq!(js_static_index.counts().contracts, 2);
    assert_eq!(js_static_index.counts().positive_fixtures, 2);
    assert_eq!(js_static_index.counts().hard_negatives, 2);
    assert!(js_static_index
        .conformance_refs()
        .contains(&"js-static-index-membership-non-literal-receiver-hard-negative"));

    let js_collection_constructors =
        builtin_pack_descriptor(JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID)
            .expect("JavaScript builtins collection constructor descriptor");
    assert_eq!(
        js_collection_constructors.kind,
        SemanticPackKind::StdlibPack
    );
    assert_eq!(
        js_collection_constructors.supported_languages,
        &["javascript", "typescript"]
    );
    assert_eq!(
        js_collection_constructors.supported_packages,
        &["Map", "Set"]
    );
    assert_eq!(
        js_collection_constructors.evidence_producer_ids,
        &[JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID]
    );
    assert!(js_collection_constructors
        .source_fact_producer_ids
        .is_empty());
    assert_eq!(
        js_collection_constructors.contract_ids,
        &[
            JS_LIKE_BUILTIN_SET_CONSTRUCTOR_CONTRACT_ID,
            JS_LIKE_BUILTIN_MAP_CONSTRUCTOR_CONTRACT_ID,
        ]
    );
    assert_eq!(js_collection_constructors.counts().evidence_producers, 1);
    assert_eq!(js_collection_constructors.counts().contracts, 2);
    assert_eq!(js_collection_constructors.counts().positive_fixtures, 2);
    assert_eq!(js_collection_constructors.counts().hard_negatives, 3);
    assert!(js_collection_constructors
        .conformance_refs()
        .contains(&"js-collection-constructor-missing-construct-hard-negative"));

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

    let laws = builtin_pack_descriptor(VALUE_GRAPH_LAW_PACK_ID).expect("value law descriptor");
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
fn builtin_compat_pack_hash_matches_evidence_provenance_hash_policy() {
    let pack = builtin_compat_semantic_pack();
    assert_eq!(pack.id, BUILTIN_COMPAT_PACK_ID);
    assert_eq!(pack.hash, stable_symbol_hash(BUILTIN_COMPAT_PACK_ID));
    assert_eq!(pack.influence, SemanticPackInfluence::EvidenceAndContracts);
    let set = SemanticPackSet::builtin_only();
    let c = set
        .packs()
        .iter()
        .find(|pack| pack.id == C_LANGUAGE_PACK_ID)
        .expect("C summary");
    assert_eq!(c.id, C_LANGUAGE_PACK_ID);
    assert_eq!(c.hash, stable_symbol_hash(C_LANGUAGE_PACK_ID));
    assert_eq!(c.kind, SemanticPackKind::LanguagePack);
    assert_eq!(c.counts.evidence_producers, 3);
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
    let ruby_set = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUBY_STDLIB_SET_PACK_ID)
        .expect("Ruby stdlib Set summary");
    assert_eq!(ruby_set.hash, stable_symbol_hash(RUBY_STDLIB_SET_PACK_ID));
    assert_eq!(ruby_set.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        ruby_set.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(ruby_set.counts.evidence_producers, 1);
    assert_eq!(ruby_set.counts.contracts, 1);
    assert_eq!(ruby_set.counts.positive_fixtures, 3);
    assert_eq!(ruby_set.counts.hard_negatives, 3);
    let rust_vec = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUST_STDLIB_VEC_PACK_ID)
        .expect("Rust stdlib Vec summary");
    assert_eq!(rust_vec.hash, stable_symbol_hash(RUST_STDLIB_VEC_PACK_ID));
    assert_eq!(rust_vec.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        rust_vec.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(rust_vec.counts.evidence_producers, 1);
    assert_eq!(rust_vec.counts.contracts, 2);
    assert_eq!(rust_vec.counts.positive_fixtures, 2);
    assert_eq!(rust_vec.counts.hard_negatives, 2);
    let rust_option = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUST_STDLIB_OPTION_PACK_ID)
        .expect("Rust stdlib Option summary");
    assert_eq!(
        rust_option.hash,
        stable_symbol_hash(RUST_STDLIB_OPTION_PACK_ID)
    );
    assert_eq!(rust_option.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        rust_option.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(rust_option.counts.evidence_producers, 1);
    assert_eq!(rust_option.counts.contracts, 3);
    assert_eq!(rust_option.counts.positive_fixtures, 3);
    assert_eq!(rust_option.counts.hard_negatives, 3);
    let rust_integer_methods = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUST_STDLIB_INTEGER_METHOD_PACK_ID)
        .expect("Rust stdlib integer method summary");
    assert_eq!(
        rust_integer_methods.hash,
        stable_symbol_hash(RUST_STDLIB_INTEGER_METHOD_PACK_ID)
    );
    assert_eq!(rust_integer_methods.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        rust_integer_methods.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(rust_integer_methods.counts.evidence_producers, 1);
    assert_eq!(rust_integer_methods.counts.contracts, 4);
    assert_eq!(rust_integer_methods.counts.positive_fixtures, 4);
    assert_eq!(rust_integer_methods.counts.hard_negatives, 2);
    let rust_stdlib_collections = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUST_STDLIB_COLLECTION_FACTORY_PACK_ID)
        .expect("Rust stdlib collection factory summary");
    assert_eq!(
        rust_stdlib_collections.hash,
        stable_symbol_hash(RUST_STDLIB_COLLECTION_FACTORY_PACK_ID)
    );
    assert_eq!(rust_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        rust_stdlib_collections.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(rust_stdlib_collections.counts.evidence_producers, 1);
    assert_eq!(rust_stdlib_collections.counts.contracts, 1);
    assert_eq!(rust_stdlib_collections.counts.positive_fixtures, 3);
    assert_eq!(rust_stdlib_collections.counts.hard_negatives, 2);
    let rust_stdlib_maps = set
        .packs()
        .iter()
        .find(|pack| pack.id == RUST_STDLIB_MAP_FACTORY_PACK_ID)
        .expect("Rust stdlib map factory summary");
    assert_eq!(
        rust_stdlib_maps.hash,
        stable_symbol_hash(RUST_STDLIB_MAP_FACTORY_PACK_ID)
    );
    assert_eq!(rust_stdlib_maps.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        rust_stdlib_maps.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(rust_stdlib_maps.counts.evidence_producers, 1);
    assert_eq!(rust_stdlib_maps.counts.contracts, 1);
    assert_eq!(rust_stdlib_maps.counts.positive_fixtures, 2);
    assert_eq!(rust_stdlib_maps.counts.hard_negatives, 2);
    let java_stdlib_math = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_MATH_PACK_ID)
        .expect("Java stdlib Math summary");
    assert_eq!(
        java_stdlib_math.hash,
        stable_symbol_hash(JAVA_STDLIB_MATH_PACK_ID)
    );
    assert_eq!(java_stdlib_math.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_stdlib_math.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_stdlib_math.counts.evidence_producers, 1);
    assert_eq!(java_stdlib_math.counts.contracts, 3);
    assert_eq!(java_stdlib_math.counts.positive_fixtures, 3);
    assert_eq!(java_stdlib_math.counts.hard_negatives, 3);
    let map_get = set
        .packs()
        .iter()
        .find(|pack| pack.id == MAP_GET_PROTOCOL_PACK_ID)
        .expect("map-get protocol summary");
    assert_eq!(map_get.hash, stable_symbol_hash(MAP_GET_PROTOCOL_PACK_ID));
    assert_eq!(map_get.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_get.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(map_get.counts.evidence_producers, 1);
    assert_eq!(map_get.counts.contracts, 1);
    assert_eq!(map_get.counts.positive_fixtures, 3);
    assert_eq!(map_get.counts.hard_negatives, 2);
    let map_get_default = set
        .packs()
        .iter()
        .find(|pack| pack.id == MAP_GET_DEFAULT_PROTOCOL_PACK_ID)
        .expect("map-get-default protocol summary");
    assert_eq!(
        map_get_default.hash,
        stable_symbol_hash(MAP_GET_DEFAULT_PROTOCOL_PACK_ID)
    );
    assert_eq!(map_get_default.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_get_default.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(map_get_default.counts.evidence_producers, 1);
    assert_eq!(map_get_default.counts.contracts, 1);
    assert_eq!(map_get_default.counts.positive_fixtures, 3);
    assert_eq!(map_get_default.counts.hard_negatives, 2);
    let free_function_builtin = set
        .packs()
        .iter()
        .find(|pack| pack.id == FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID)
        .expect("free-function builtin protocol summary");
    assert_eq!(
        free_function_builtin.hash,
        stable_symbol_hash(FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID)
    );
    assert_eq!(free_function_builtin.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        free_function_builtin.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(free_function_builtin.counts.evidence_producers, 1);
    assert_eq!(free_function_builtin.counts.contracts, 1);
    assert_eq!(free_function_builtin.counts.positive_fixtures, 6);
    assert_eq!(free_function_builtin.counts.hard_negatives, 4);
    let receiver_membership = set
        .packs()
        .iter()
        .find(|pack| pack.id == RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID)
        .expect("receiver-membership protocol summary");
    assert_eq!(
        receiver_membership.hash,
        stable_symbol_hash(RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID)
    );
    assert_eq!(receiver_membership.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        receiver_membership.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(receiver_membership.counts.evidence_producers, 1);
    assert_eq!(receiver_membership.counts.contracts, 1);
    assert_eq!(receiver_membership.counts.positive_fixtures, 10);
    assert_eq!(receiver_membership.counts.hard_negatives, 3);
    let map_key_view = set
        .packs()
        .iter()
        .find(|pack| pack.id == MAP_KEY_VIEW_PROTOCOL_PACK_ID)
        .expect("map-key-view protocol summary");
    assert_eq!(
        map_key_view.hash,
        stable_symbol_hash(MAP_KEY_VIEW_PROTOCOL_PACK_ID)
    );
    assert_eq!(map_key_view.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        map_key_view.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(map_key_view.counts.evidence_producers, 1);
    assert_eq!(map_key_view.counts.contracts, 2);
    assert_eq!(map_key_view.counts.positive_fixtures, 4);
    assert_eq!(map_key_view.counts.hard_negatives, 2);
    let property_builtin = set
        .packs()
        .iter()
        .find(|pack| pack.id == PROPERTY_BUILTIN_PROTOCOL_PACK_ID)
        .expect("property-builtin protocol summary");
    assert_eq!(
        property_builtin.hash,
        stable_symbol_hash(PROPERTY_BUILTIN_PROTOCOL_PACK_ID)
    );
    assert_eq!(property_builtin.kind, SemanticPackKind::ProtocolPack);
    assert_eq!(
        property_builtin.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(property_builtin.counts.evidence_producers, 1);
    assert_eq!(property_builtin.counts.contracts, 2);
    assert_eq!(property_builtin.counts.positive_fixtures, 4);
    assert_eq!(property_builtin.counts.hard_negatives, 3);
    let java_stdlib_maps = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_MAP_FACTORY_PACK_ID)
        .expect("Java stdlib map factory summary");
    assert_eq!(
        java_stdlib_maps.hash,
        stable_symbol_hash(JAVA_STDLIB_MAP_FACTORY_PACK_ID)
    );
    assert_eq!(java_stdlib_maps.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_stdlib_maps.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_stdlib_maps.counts.evidence_producers, 1);
    assert_eq!(java_stdlib_maps.counts.contracts, 2);
    assert_eq!(java_stdlib_maps.counts.positive_fixtures, 2);
    assert_eq!(java_stdlib_maps.counts.hard_negatives, 2);
    let java_stdlib_entries = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_MAP_ENTRY_PACK_ID)
        .expect("Java stdlib map entry summary");
    assert_eq!(
        java_stdlib_entries.hash,
        stable_symbol_hash(JAVA_STDLIB_MAP_ENTRY_PACK_ID)
    );
    assert_eq!(java_stdlib_entries.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_stdlib_entries.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_stdlib_entries.counts.evidence_producers, 1);
    assert_eq!(java_stdlib_entries.counts.contracts, 1);
    assert_eq!(java_stdlib_entries.counts.positive_fixtures, 1);
    assert_eq!(java_stdlib_entries.counts.hard_negatives, 2);
    let java_stdlib_collections = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID)
        .expect("Java stdlib collection factory summary");
    assert_eq!(
        java_stdlib_collections.hash,
        stable_symbol_hash(JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID)
    );
    assert_eq!(java_stdlib_collections.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_stdlib_collections.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_stdlib_collections.counts.evidence_producers, 1);
    assert_eq!(java_stdlib_collections.counts.contracts, 3);
    assert_eq!(java_stdlib_collections.counts.positive_fixtures, 3);
    assert_eq!(java_stdlib_collections.counts.hard_negatives, 2);
    let java_stdlib_constructors = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID)
        .expect("Java stdlib collection constructor summary");
    assert_eq!(
        java_stdlib_constructors.hash,
        stable_symbol_hash(JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID)
    );
    assert_eq!(java_stdlib_constructors.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_stdlib_constructors.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_stdlib_constructors.counts.evidence_producers, 1);
    assert_eq!(java_stdlib_constructors.counts.contracts, 1);
    assert_eq!(java_stdlib_constructors.counts.positive_fixtures, 2);
    assert_eq!(java_stdlib_constructors.counts.hard_negatives, 3);
    let java_static_adapters = set
        .packs()
        .iter()
        .find(|pack| pack.id == JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID)
        .expect("Java stdlib static collection adapter summary");
    assert_eq!(
        java_static_adapters.hash,
        stable_symbol_hash(JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID)
    );
    assert_eq!(java_static_adapters.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        java_static_adapters.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(java_static_adapters.counts.evidence_producers, 1);
    assert_eq!(java_static_adapters.counts.contracts, 1);
    assert_eq!(java_static_adapters.counts.positive_fixtures, 1);
    assert_eq!(java_static_adapters.counts.hard_negatives, 2);
    let iterator_identity_adapters = set
        .packs()
        .iter()
        .find(|pack| pack.id == ITERATOR_IDENTITY_ADAPTER_PACK_ID)
        .expect("iterator identity adapter protocol summary");
    assert_eq!(
        iterator_identity_adapters.hash,
        stable_symbol_hash(ITERATOR_IDENTITY_ADAPTER_PACK_ID)
    );
    assert_eq!(
        iterator_identity_adapters.kind,
        SemanticPackKind::ProtocolPack
    );
    assert_eq!(
        iterator_identity_adapters.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(iterator_identity_adapters.counts.evidence_producers, 1);
    assert_eq!(iterator_identity_adapters.counts.contracts, 1);
    assert_eq!(iterator_identity_adapters.counts.positive_fixtures, 3);
    assert_eq!(iterator_identity_adapters.counts.hard_negatives, 2);
    let js_promise = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_PROMISE_PACK_ID)
        .expect("JavaScript builtins Promise summary");
    assert_eq!(
        js_promise.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_PROMISE_PACK_ID)
    );
    assert_eq!(js_promise.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_promise.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_promise.counts.evidence_producers, 1);
    assert_eq!(js_promise.counts.contracts, 2);
    assert_eq!(js_promise.counts.positive_fixtures, 2);
    assert_eq!(js_promise.counts.hard_negatives, 3);
    let js_array = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_ARRAY_PACK_ID)
        .expect("JavaScript builtins Array summary");
    assert_eq!(
        js_array.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_ARRAY_PACK_ID)
    );
    assert_eq!(js_array.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_array.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_array.counts.evidence_producers, 1);
    assert_eq!(js_array.counts.contracts, 2);
    assert_eq!(js_array.counts.positive_fixtures, 2);
    assert_eq!(js_array.counts.hard_negatives, 3);
    let js_boolean = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_BOOLEAN_PACK_ID)
        .expect("JavaScript builtins Boolean summary");
    assert_eq!(
        js_boolean.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_BOOLEAN_PACK_ID)
    );
    assert_eq!(js_boolean.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_boolean.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_boolean.counts.evidence_producers, 1);
    assert_eq!(js_boolean.counts.contracts, 1);
    assert_eq!(js_boolean.counts.positive_fixtures, 1);
    assert_eq!(js_boolean.counts.hard_negatives, 2);
    let js_regex = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_REGEX_PACK_ID)
        .expect("JavaScript builtins RegExp summary");
    assert_eq!(
        js_regex.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_REGEX_PACK_ID)
    );
    assert_eq!(js_regex.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_regex.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_regex.counts.evidence_producers, 1);
    assert_eq!(js_regex.counts.contracts, 1);
    assert_eq!(js_regex.counts.positive_fixtures, 1);
    assert_eq!(js_regex.counts.hard_negatives, 2);
    let js_static_index = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID)
        .expect("JavaScript builtins static index membership summary");
    assert_eq!(
        js_static_index.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID)
    );
    assert_eq!(js_static_index.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        js_static_index.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_static_index.counts.evidence_producers, 1);
    assert_eq!(js_static_index.counts.contracts, 2);
    assert_eq!(js_static_index.counts.positive_fixtures, 2);
    assert_eq!(js_static_index.counts.hard_negatives, 2);
    let js_collection_constructors = set
        .packs()
        .iter()
        .find(|pack| pack.id == JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID)
        .expect("JavaScript builtins collection constructor summary");
    assert_eq!(
        js_collection_constructors.hash,
        stable_symbol_hash(JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID)
    );
    assert_eq!(
        js_collection_constructors.kind,
        SemanticPackKind::StdlibPack
    );
    assert_eq!(
        js_collection_constructors.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(js_collection_constructors.counts.evidence_producers, 1);
    assert_eq!(js_collection_constructors.counts.contracts, 2);
    assert_eq!(js_collection_constructors.counts.positive_fixtures, 2);
    assert_eq!(js_collection_constructors.counts.hard_negatives, 3);
    let python_stdlib_math = set
        .packs()
        .iter()
        .find(|pack| pack.id == PYTHON_STDLIB_MATH_PACK_ID)
        .expect("Python stdlib math summary");
    assert_eq!(
        python_stdlib_math.hash,
        stable_symbol_hash(PYTHON_STDLIB_MATH_PACK_ID)
    );
    assert_eq!(python_stdlib_math.kind, SemanticPackKind::StdlibPack);
    assert_eq!(
        python_stdlib_math.influence,
        SemanticPackInfluence::EvidenceAndContracts
    );
    assert_eq!(python_stdlib_math.counts.evidence_producers, 1);
    assert_eq!(python_stdlib_math.counts.contracts, 1);
    assert_eq!(python_stdlib_math.counts.positive_fixtures, 1);
    assert_eq!(python_stdlib_math.counts.hard_negatives, 2);
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
    let laws = value_graph_law_pack();
    assert_eq!(laws.id, VALUE_GRAPH_LAW_PACK_ID);
    assert_eq!(laws.hash, stable_symbol_hash(VALUE_GRAPH_LAW_PACK_ID));
    assert_eq!(laws.kind, SemanticPackKind::LawPack);
    assert_eq!(laws.counts.value_laws, pack_facing_value_laws().len());
    assert_eq!(laws.counts.positive_fixtures, 2);
    assert_eq!(laws.counts.hard_negatives, 4);
}

#[test]
fn legacy_first_party_pack_aliases_match_builtin_names() {
    assert_eq!(FIRST_PARTY_PACK_ID, BUILTIN_COMPAT_PACK_ID);
    assert_eq!(FIRST_PARTY_VALUE_LAW_PACK_ID, VALUE_GRAPH_LAW_PACK_ID);

    let legacy_compat = first_party_semantic_pack();
    let builtin_compat = builtin_compat_semantic_pack();
    assert_eq!(legacy_compat.id, builtin_compat.id);
    assert_eq!(legacy_compat.hash, builtin_compat.hash);
    assert_eq!(legacy_compat.kind, builtin_compat.kind);

    let legacy_laws = first_party_value_law_pack();
    let value_laws = value_graph_law_pack();
    assert_eq!(legacy_laws.id, value_laws.id);
    assert_eq!(legacy_laws.hash, value_laws.hash);
    assert_eq!(legacy_laws.kind, value_laws.kind);
}

#[test]
fn local_manifest_loads_as_metadata_only_opt_in() {
    let dir = unique_dir("load");
    let path = dir.join("pack.json");
    fs::write(&path, manifest("com.example.pack")).unwrap();
    let set = SemanticPackSet::new_local(&[path]).expect("pack loads");
    assert_eq!(set.packs().len(), 43);
    assert_eq!(set.packs()[1].id, PYTHON_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[2].id, JS_TS_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[3].id, GO_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[4].id, RUST_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[5].id, JAVA_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[6].id, C_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[7].id, RUBY_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[8].id, SWIFT_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[9].id, CSS_LANGUAGE_PACK_ID);
    assert_eq!(set.packs()[10].id, HTML_EMBEDDED_LANGUAGE_PACK_ID);
    assert_eq!(
        set.packs()[11].id,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID
    );
    assert_eq!(set.packs()[12].id, PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID);
    assert_eq!(set.packs()[13].id, PYTHON_STDLIB_MATH_PACK_ID);
    assert_eq!(set.packs()[14].id, RUBY_STDLIB_SET_PACK_ID);
    assert_eq!(set.packs()[15].id, RUST_STDLIB_VEC_PACK_ID);
    assert_eq!(set.packs()[16].id, RUST_STDLIB_OPTION_PACK_ID);
    assert_eq!(set.packs()[17].id, RUST_STDLIB_INTEGER_METHOD_PACK_ID);
    assert_eq!(set.packs()[18].id, RUST_STDLIB_COLLECTION_FACTORY_PACK_ID);
    assert_eq!(set.packs()[19].id, RUST_STDLIB_MAP_FACTORY_PACK_ID);
    assert_eq!(set.packs()[20].id, JAVA_STDLIB_MATH_PACK_ID);
    assert_eq!(set.packs()[21].id, JAVA_STDLIB_MAP_FACTORY_PACK_ID);
    assert_eq!(set.packs()[22].id, JAVA_STDLIB_MAP_ENTRY_PACK_ID);
    assert_eq!(set.packs()[23].id, JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID);
    assert_eq!(
        set.packs()[24].id,
        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID
    );
    assert_eq!(
        set.packs()[25].id,
        JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID
    );
    assert_eq!(set.packs()[26].id, MAP_GET_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[27].id, MAP_GET_DEFAULT_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[28].id, FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[29].id, RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[30].id, MAP_KEY_VIEW_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[31].id, PROPERTY_BUILTIN_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[32].id, BUILTIN_METHOD_CALL_PROTOCOL_PACK_ID);
    assert_eq!(set.packs()[33].id, ITERATOR_IDENTITY_ADAPTER_PACK_ID);
    assert_eq!(set.packs()[34].id, JS_LIKE_BUILTIN_PROMISE_PACK_ID);
    assert_eq!(set.packs()[35].id, JS_LIKE_BUILTIN_ARRAY_PACK_ID);
    assert_eq!(set.packs()[36].id, JS_LIKE_BUILTIN_BOOLEAN_PACK_ID);
    assert_eq!(set.packs()[37].id, JS_LIKE_BUILTIN_REGEX_PACK_ID);
    assert_eq!(
        set.packs()[38].id,
        JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID
    );
    assert_eq!(
        set.packs()[39].id,
        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID
    );
    assert_eq!(set.packs()[40].id, PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID);
    assert_eq!(set.packs()[41].id, VALUE_GRAPH_LAW_PACK_ID);
    let external = &set.packs()[42];
    assert_eq!(external.id, "com.example.pack");
    assert_eq!(external.hash, stable_symbol_hash("com.example.pack"));
    assert_eq!(external.trust, PackTrust::ExternalOptIn);
    assert_eq!(external.source, SemanticPackSource::LocalManifest);
    assert_eq!(external.influence, SemanticPackInfluence::MetadataOnly);
    assert_eq!(external.counts.contracts, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_registers_external_value_law_rows_as_data_only() {
    let dir = unique_dir("external_value_law_rows");
    let path = dir.join("pack.json");
    fs::write(&path, manifest_with_value_law("com.example.laws")).unwrap();

    let set = SemanticPackSet::new_local(&[path.clone()]).expect("pack loads");
    let external = set.packs().last().expect("external pack summary");
    assert_eq!(external.id, "com.example.laws");
    assert_eq!(external.influence, SemanticPackInfluence::MetadataOnly);
    assert_eq!(external.counts.value_laws, 1);

    let rows = set.external_value_law_rows();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.pack_id, "com.example.laws");
    assert_eq!(row.pack_hash, stable_symbol_hash("com.example.laws"));
    assert_eq!(row.manifest_path, path.canonicalize().unwrap());
    assert_eq!(row.law_id, "python.example.numeric-law");
    assert_eq!(
        row.law_hash,
        stable_symbol_hash("python.example.numeric-law")
    );
    assert_eq!(row.channel, SemanticPackChannel::ExactProven);
    assert_eq!(row.proof_status, SemanticPackProofStatus::Proven);
    assert_eq!(row.requirements.len(), 1);
    assert_eq!(row.requirements[0].ref_id, "Domain.Number");
    assert_eq!(row.requirements[0].subject, "operands");
    assert!(row.requirements[0].required);
    assert_eq!(row.requirements[0].same_anchor_as.as_deref(), Some("value"));
    assert_eq!(row.conformance_refs, ["positive", "negative"]);
    assert_eq!(row.semantics["law"], "x + 0 == x");

    assert!(SemanticPackSet::builtin_only()
        .external_value_law_rows()
        .is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_registers_external_producer_and_contract_rows_as_data_only() {
    let dir = unique_dir("external_contract_rows");
    let path = dir.join("pack.json");
    fs::write(&path, manifest("com.example.contracts")).unwrap();

    let set = SemanticPackSet::new_local(&[path.clone()]).expect("pack loads");
    let external = set.packs().last().expect("external pack summary");
    assert_eq!(external.id, "com.example.contracts");
    assert_eq!(external.influence, SemanticPackInfluence::MetadataOnly);
    assert_eq!(external.counts.evidence_producers, 1);
    assert_eq!(external.counts.contracts, 1);

    let producers = set.external_evidence_producer_rows();
    assert_eq!(producers.len(), 1);
    let producer = &producers[0];
    assert_eq!(producer.pack_id, "com.example.contracts");
    assert_eq!(
        producer.pack_hash,
        stable_symbol_hash("com.example.contracts")
    );
    assert_eq!(producer.manifest_path, path.canonicalize().unwrap());
    assert_eq!(producer.producer_id, "python.library-api.example");
    assert_eq!(
        producer.producer_hash,
        stable_symbol_hash("python.library-api.example")
    );
    assert_eq!(producer.kind, "LibraryApi.Contract");
    assert_eq!(producer.anchors, [SemanticPackAnchor::Node]);
    assert_eq!(producer.channel, SemanticPackChannel::ExactEmpirical);
    assert!(producer.emits.is_empty());
    assert!(producer.requirements.is_empty());
    assert_eq!(
        producer.stable_hash_inputs,
        ["pack.id", "producer.id", "call_span"]
    );
    assert_eq!(producer.conflict_policy, "fail-closed");
    assert!(producer.notes.is_none());

    let contracts = set.external_contract_rows();
    assert_eq!(contracts.len(), 1);
    let contract = &contracts[0];
    assert_eq!(contract.pack_id, "com.example.contracts");
    assert_eq!(
        contract.pack_hash,
        stable_symbol_hash("com.example.contracts")
    );
    assert_eq!(contract.manifest_path, path.canonicalize().unwrap());
    assert_eq!(contract.contract_id, "python.example.contract");
    assert_eq!(
        contract.contract_hash,
        stable_symbol_hash("python.example.contract")
    );
    assert_eq!(contract.surface["kind"], "function");
    assert_eq!(contract.requirements.len(), 1);
    assert_eq!(
        contract.requirements[0].ref_id,
        "python.library-api.example"
    );
    assert_eq!(contract.requirements[0].subject, "call");
    assert!(contract.requirements[0].required);
    assert_eq!(contract.semantics["operation"], "Example");
    assert_eq!(contract.channel, SemanticPackChannel::ExactEmpirical);
    assert_eq!(contract.proof_status, SemanticPackProofStatus::Covered);
    assert_eq!(contract.conformance_refs, ["positive", "negative"]);
    assert!(contract.known_unsupported.is_empty());
    assert!(contract.notes.is_none());

    let builtin = SemanticPackSet::builtin_only();
    assert!(builtin.external_evidence_producer_rows().is_empty());
    assert!(builtin.external_contract_rows().is_empty());
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
fn local_manifest_claiming_builtin_trust_is_rejected() {
    let dir = unique_dir("builtin_trust");
    let path = dir.join("pack.json");
    fs::write(
        &path,
        manifest("com.example.pack").replace(
            r#""trust": "external-opt-in""#,
            r#""trust": "builtin-default""#,
        ),
    )
    .unwrap();
    let err = load_local_manifest(&path).expect_err("local manifest must not claim builtin trust");
    assert!(err
        .to_string()
        .contains("must be external-opt-in and disabled by default"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_legacy_first_party_trust_aliases_are_rejected_after_parse() {
    for legacy_trust in ["default-first-party", "first-party-optional"] {
        let dir = unique_dir(&format!("legacy_{}_trust", legacy_trust.replace('-', "_")));
        let path = dir.join("pack.json");
        fs::write(
            &path,
            manifest("com.example.pack").replace(
                r#""trust": "external-opt-in""#,
                &format!(r#""trust": "{legacy_trust}""#),
            ),
        )
        .unwrap();
        let err =
            load_local_manifest(&path).expect_err("legacy alias must be reserved for builtin");
        assert!(err
            .to_string()
            .contains("must be external-opt-in and disabled by default"));
        let _ = fs::remove_dir_all(dir);
    }
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
fn local_manifest_cannot_claim_compiled_builtin_pack_id() {
    let dir = unique_dir("compiled_builtin_id");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)).unwrap();
    let err = SemanticPackSet::new_local(&[path]).expect_err("compiled id is reserved");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn local_manifest_cannot_claim_builtin_language_pack_id() {
    let dir = unique_dir("builtin_language_id");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(PYTHON_LANGUAGE_PACK_ID)).unwrap();
    let err = SemanticPackSet::new_local(&[path]).expect_err("builtin language id is reserved");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_cannot_claim_compiled_builtin_pack_id() {
    let dir = unique_dir("compiled_builtin_conformance");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)).unwrap();
    let err = check_semantic_pack_conformance(&[path]).expect_err("compiled id is reserved");
    assert!(err.to_string().contains("duplicate semantic pack id"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conformance_check_cannot_claim_builtin_language_pack_id() {
    let dir = unique_dir("builtin_language_conformance");
    let path = dir.join("pack.json");
    fs::write(&path, manifest(JS_TS_LANGUAGE_PACK_ID)).unwrap();
    let err =
        check_semantic_pack_conformance(&[path]).expect_err("builtin language id is reserved");
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

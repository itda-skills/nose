use super::*;
use nose_il::Lang;

const C_LANGUAGE: &[&str] = &["c"];
const C_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["c", "h"];
const PYTHON_BINDING_LANGS: &[Lang] = &[Lang::Python];
const JS_TS_BINDING_LANGS: &[Lang] = &[Lang::JavaScript, Lang::TypeScript];
const GO_BINDING_LANGS: &[Lang] = &[Lang::Go];
const RUST_BINDING_LANGS: &[Lang] = &[Lang::Rust];
const JAVA_BINDING_LANGS: &[Lang] = &[Lang::Java];
const C_BINDING_LANGS: &[Lang] = &[Lang::C];
const RUBY_BINDING_LANGS: &[Lang] = &[Lang::Ruby];
const SWIFT_BINDING_LANGS: &[Lang] = &[Lang::Swift];
const CSS_BINDING_LANGS: &[Lang] = &[Lang::Css];
const HTML_EMBEDDED_BINDING_LANGS: &[Lang] = &[Lang::Html, Lang::Vue, Lang::Svelte];
const PYTHON_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    PYTHON_LANGUAGE_CORE_PRODUCER_ID,
    PYTHON_SOURCE_FACT_PRODUCER_ID,
];
const PYTHON_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[PYTHON_SOURCE_FACT_PRODUCER_ID];
const JS_TS_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    JS_TS_LANGUAGE_CORE_PRODUCER_ID,
    JS_TS_SOURCE_FACT_PRODUCER_ID,
];
const JS_TS_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[JS_TS_SOURCE_FACT_PRODUCER_ID];
const GO_LANGUAGE_PRODUCER_IDS: &[&str] =
    &[GO_LANGUAGE_CORE_PRODUCER_ID, GO_SOURCE_FACT_PRODUCER_ID];
const GO_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[GO_SOURCE_FACT_PRODUCER_ID];
const RUST_LANGUAGE_PRODUCER_IDS: &[&str] =
    &[RUST_LANGUAGE_CORE_PRODUCER_ID, RUST_SOURCE_FACT_PRODUCER_ID];
const RUST_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[RUST_SOURCE_FACT_PRODUCER_ID];
const JAVA_LANGUAGE_PRODUCER_IDS: &[&str] =
    &[JAVA_LANGUAGE_CORE_PRODUCER_ID, JAVA_SOURCE_FACT_PRODUCER_ID];
const JAVA_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[JAVA_SOURCE_FACT_PRODUCER_ID];
const C_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    C_LANGUAGE_CORE_PRODUCER_ID,
    C_SOURCE_FACT_PRODUCER_ID,
    C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID,
];
const C_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[
    C_SOURCE_FACT_PRODUCER_ID,
    C_UNSIGNED_32_CAST_SOURCE_PRODUCER_ID,
];
const RUBY_LANGUAGE_PRODUCER_IDS: &[&str] =
    &[RUBY_LANGUAGE_CORE_PRODUCER_ID, RUBY_SOURCE_FACT_PRODUCER_ID];
const RUBY_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[RUBY_SOURCE_FACT_PRODUCER_ID];
const SWIFT_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    SWIFT_LANGUAGE_CORE_PRODUCER_ID,
    SWIFT_SOURCE_FACT_PRODUCER_ID,
];
const SWIFT_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[SWIFT_SOURCE_FACT_PRODUCER_ID];
const CSS_LANGUAGE_PRODUCER_IDS: &[&str] =
    &[CSS_LANGUAGE_CORE_PRODUCER_ID, CSS_SOURCE_FACT_PRODUCER_ID];
const CSS_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] = &[CSS_SOURCE_FACT_PRODUCER_ID];
const HTML_EMBEDDED_LANGUAGE_PRODUCER_IDS: &[&str] = &[
    HTML_EMBEDDED_LANGUAGE_CORE_PRODUCER_ID,
    HTML_EMBEDDED_SOURCE_FACT_PRODUCER_ID,
];
const HTML_EMBEDDED_LANGUAGE_SOURCE_FACT_PRODUCER_IDS: &[&str] =
    &[HTML_EMBEDDED_SOURCE_FACT_PRODUCER_ID];
const C_LANGUAGE_CONFORMANCE_REFS: &[&str] = &[
    "c-unsigned32-byte-lane-cast-positive",
    "c-unsigned32-alias-cast-positive",
    "c-unsigned32-signed-cast-hard-negative",
    "c-unsigned32-non-byte-lane-hard-negative",
];
const PYTHON_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["py", "pyi"];
const JS_TS_LANGUAGE_FILE_EXTENSIONS: &[&str] =
    &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"];
const GO_LANGUAGE: &[&str] = &["go"];
const GO_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["go"];
const RUST_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["rs"];
const JAVA_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["java"];
const RUBY_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["rb"];
const SWIFT_LANGUAGE: &[&str] = &["swift"];
const SWIFT_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["swift"];
const CSS_LANGUAGE: &[&str] = &["css"];
const CSS_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["css"];
const HTML_EMBEDDED_LANGUAGES: &[&str] = &["html", "vue", "svelte"];
const HTML_EMBEDDED_LANGUAGE_FILE_EXTENSIONS: &[&str] = &["html", "htm", "vue", "svelte"];
const JS_LIKE_LANGUAGE: &[&str] = &["javascript", "typescript"];
const JAVA_LANGUAGE: &[&str] = &["java"];
const JAVA_RUST_LANGUAGE: &[&str] = &["java", "rust"];
const MAP_GET_DEFAULT_PROTOCOL_LANGUAGES: &[&str] = &["python", "ruby", "java"];
const FREE_FUNCTION_BUILTIN_PROTOCOL_LANGUAGES: &[&str] = &["python", "go", "swift"];
const RECEIVER_MEMBERSHIP_PROTOCOL_LANGUAGES: &[&str] = &[
    "python",
    "ruby",
    "java",
    "rust",
    "swift",
    "javascript",
    "typescript",
    "vue",
    "svelte",
    "html",
];
const MAP_KEY_VIEW_PROTOCOL_LANGUAGES: &[&str] = &[
    "python",
    "ruby",
    "java",
    "javascript",
    "typescript",
    "vue",
    "svelte",
    "html",
];
const PROPERTY_BUILTIN_PROTOCOL_LANGUAGES: &[&str] = &[
    "javascript",
    "typescript",
    "vue",
    "svelte",
    "html",
    "java",
    "swift",
];
const BUILTIN_METHOD_CALL_PROTOCOL_LANGUAGES: &[&str] = &[
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
    "swift",
];
const MAP_GET_PROTOCOL_LANGUAGES: &[&str] = &[
    "java",
    "rust",
    "javascript",
    "typescript",
    "vue",
    "svelte",
    "html",
];
const NO_LANGUAGES: &[&str] = &[];
const PYTHON_LANGUAGE: &[&str] = &["python"];
const RUBY_LANGUAGE: &[&str] = &["ruby"];
const RUST_LANGUAGE: &[&str] = &["rust"];
const NO_PACKAGES: &[&str] = &[];
const JAVA_STDLIB_MAP_FACTORY_PACKAGES: &[&str] = &["java.util"];
const JAVA_STDLIB_MAP_ENTRY_PACKAGES: &[&str] = &["java.util"];
const JAVA_STDLIB_COLLECTION_FACTORY_PACKAGES: &[&str] = &["java.util"];
const JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACKAGES: &[&str] = &["java.util"];
const JAVA_STDLIB_MATH_PACKAGES: &[&str] = &["java.lang"];
const JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACKAGES: &[&str] = &["java.util"];
const ITERATOR_IDENTITY_ADAPTER_PACKAGES: &[&str] = &["core::iter", "java.util.stream"];
const MAP_GET_PROTOCOL_PACKAGES: &[&str] = &["Map", "java.util", "std::collections"];
const MAP_GET_DEFAULT_PROTOCOL_PACKAGES: &[&str] = &["dict", "Hash", "java.util"];
const FREE_FUNCTION_BUILTIN_PROTOCOL_PACKAGES: &[&str] = &["builtins", "go.predeclared", "Swift"];
const RECEIVER_MEMBERSHIP_PROTOCOL_PACKAGES: &[&str] = &[
    "Array",
    "Collection",
    "Hash",
    "Map",
    "Set",
    "Swift.Collection",
    "dict",
    "java.util",
    "std::collections",
];
const MAP_KEY_VIEW_PROTOCOL_PACKAGES: &[&str] = &["dict", "Hash", "Map", "java.util"];
const PROPERTY_BUILTIN_PROTOCOL_PACKAGES: &[&str] =
    &["Array", "Collection", "Swift.Collection", "java.lang"];
const BUILTIN_METHOD_CALL_PROTOCOL_PACKAGES: &[&str] = &[
    "Collection",
    "Option",
    "String",
    "console",
    "fmt",
    "functools",
    "slices",
    "strings",
];
const JS_LIKE_BUILTIN_ARRAY_PACKAGES: &[&str] = &["Array"];
const JS_LIKE_BUILTIN_BOOLEAN_PACKAGES: &[&str] = &["Boolean"];
const JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACKAGES: &[&str] = &["Map", "Set"];
const JS_LIKE_BUILTIN_PROMISE_PACKAGES: &[&str] = &["Promise"];
const JS_LIKE_BUILTIN_REGEX_PACKAGES: &[&str] = &["RegExp"];
const JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACKAGES: &[&str] = &["Array"];
const PYTHON_BUILTIN_PACKAGES: &[&str] = &["builtins"];
const PYTHON_STDLIB_COLLECTION_FACTORY_PACKAGES: &[&str] = &["collections"];
const PYTHON_STDLIB_MATH_PACKAGES: &[&str] = &["math"];
const PYTHON_STDLIB_TYPE_DOMAIN_PACKAGES: &[&str] = &["typing", "collections.abc", "asyncio"];
const RUBY_STDLIB_SET_PACKAGES: &[&str] = &["set"];
const RUST_STDLIB_COLLECTION_FACTORY_PACKAGES: &[&str] = &["std::collections"];
const RUST_STDLIB_MAP_FACTORY_PACKAGES: &[&str] = &["std::collections"];
const RUST_STDLIB_OPTION_PACKAGES: &[&str] = &["std::option", "core::option"];
const RUST_STDLIB_INTEGER_METHOD_PACKAGES: &[&str] = &["core::primitive"];
const RUST_STDLIB_VEC_PACKAGES: &[&str] = &["std::vec", "alloc::vec"];
const NO_IDS: &[&str] = &[];
const PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_IDS: &[&str] =
    &[PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID];
const PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_IDS: &[&str] =
    &[PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_ID];
const PYTHON_BUILTIN_COLLECTION_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "python-builtin-list-factory-positive",
    "python-builtin-set-factory-positive",
    "python-builtin-frozenset-factory-positive",
    "python-builtin-tuple-factory-positive",
    "python-builtin-list-shadowed-hard-negative",
    "python-builtin-list-wildcard-import-hard-negative",
];
const PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS: &[&str] =
    &[PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID];
const PYTHON_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS: &[&str] =
    &[PYTHON_STDLIB_COLLECTION_FACTORY_CONTRACT_ID];
const PYTHON_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "python-collections-deque-imported-binding-positive",
    "python-collections-deque-imported-alias-positive",
    "python-collections-deque-imported-namespace-positive",
    "python-collections-deque-missing-import-hard-negative",
    "python-collections-deque-wrong-module-hard-negative",
];
const PYTHON_STDLIB_MATH_PRODUCER_IDS: &[&str] = &[PYTHON_STDLIB_MATH_PRODUCER_ID];
const PYTHON_STDLIB_MATH_CONTRACT_IDS: &[&str] = &[PYTHON_STDLIB_MATH_PROD_CONTRACT_ID];
const PYTHON_STDLIB_MATH_CONFORMANCE_REFS: &[&str] = &[
    "python-math-prod-positive",
    "python-math-prod-local-shadow-hard-negative",
    "python-math-prod-wrong-namespace-hard-negative",
];
const JS_LIKE_BUILTIN_PROMISE_PRODUCER_IDS: &[&str] = &[JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID];
const JS_LIKE_BUILTIN_PROMISE_CONTRACT_IDS: &[&str] = &[
    JS_LIKE_BUILTIN_PROMISE_RESOLVE_CONTRACT_ID,
    JS_LIKE_BUILTIN_PROMISE_THEN_CONTRACT_ID,
];
const JS_LIKE_BUILTIN_PROMISE_CONFORMANCE_REFS: &[&str] = &[
    "js-promise-resolve-positive",
    "js-promise-then-positive",
    "js-promise-resolve-shadowed-hard-negative",
    "js-promise-then-missing-receiver-hard-negative",
    "js-promise-resolve-thenable-hard-negative",
];
const JS_LIKE_BUILTIN_ARRAY_PRODUCER_IDS: &[&str] = &[JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID];
const JS_LIKE_BUILTIN_ARRAY_CONTRACT_IDS: &[&str] = &[
    JS_LIKE_BUILTIN_ARRAY_FROM_CONTRACT_ID,
    JS_LIKE_BUILTIN_ARRAY_IS_ARRAY_CONTRACT_ID,
];
const JS_LIKE_BUILTIN_ARRAY_CONFORMANCE_REFS: &[&str] = &[
    "js-array-from-positive",
    "js-array-is-array-positive",
    "js-array-from-shadowed-hard-negative",
    "js-array-from-unsupported-arity-hard-negative",
    "js-array-is-array-shadowed-hard-negative",
];
const JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_IDS: &[&str] = &[JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_ID];
const JS_LIKE_BUILTIN_BOOLEAN_CONTRACT_IDS: &[&str] = &[JS_LIKE_BUILTIN_BOOLEAN_CONTRACT_ID];
const JS_LIKE_BUILTIN_BOOLEAN_CONFORMANCE_REFS: &[&str] = &[
    "js-boolean-coercion-positive",
    "js-boolean-coercion-shadowed-hard-negative",
    "js-boolean-coercion-unsupported-arity-hard-negative",
];
const JS_LIKE_BUILTIN_REGEX_PRODUCER_IDS: &[&str] = &[JS_LIKE_BUILTIN_REGEX_PRODUCER_ID];
const JS_LIKE_BUILTIN_REGEX_CONTRACT_IDS: &[&str] = &[JS_LIKE_BUILTIN_REGEX_TEST_CONTRACT_ID];
const JS_LIKE_BUILTIN_REGEX_CONFORMANCE_REFS: &[&str] = &[
    "js-regex-test-positive",
    "js-regex-test-string-receiver-hard-negative",
    "js-regex-test-unsupported-arity-hard-negative",
];
const JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_IDS: &[&str] =
    &[JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_ID];
const JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONTRACT_IDS: &[&str] = &[
    JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_INDEX_OF_CONTRACT_ID,
    JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_FIND_INDEX_CONTRACT_ID,
];
const JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONFORMANCE_REFS: &[&str] = &[
    "js-static-index-membership-index-of-positive",
    "js-static-index-membership-find-index-positive",
    "js-static-index-membership-non-literal-receiver-hard-negative",
    "js-static-index-membership-float-literal-hard-negative",
];
const JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_IDS: &[&str] =
    &[JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID];
const JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONTRACT_IDS: &[&str] = &[
    JS_LIKE_BUILTIN_SET_CONSTRUCTOR_CONTRACT_ID,
    JS_LIKE_BUILTIN_MAP_CONSTRUCTOR_CONTRACT_ID,
];
const JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS: &[&str] = &[
    "js-set-constructor-positive",
    "js-map-constructor-positive",
    "js-set-constructor-shadowed-hard-negative",
    "js-map-constructor-shadowed-hard-negative",
    "js-collection-constructor-missing-construct-hard-negative",
];
const RUBY_STDLIB_SET_PRODUCER_IDS: &[&str] = &[RUBY_STDLIB_SET_PRODUCER_ID];
const RUBY_STDLIB_SET_CONTRACT_IDS: &[&str] = &[RUBY_STDLIB_SET_CONTRACT_ID];
const RUBY_STDLIB_SET_CONFORMANCE_REFS: &[&str] = &[
    "ruby-set-new-include-positive",
    "ruby-set-new-member-positive",
    "ruby-set-local-positive",
    "ruby-set-missing-require-hard-negative",
    "ruby-set-shadowed-hard-negative",
    "ruby-set-mutated-hard-negative",
];
const RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS: &[&str] =
    &[RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID];
const RUST_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS: &[&str] =
    &[RUST_STDLIB_COLLECTION_FACTORY_CONTRACT_ID];
const RUST_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "rust-std-collections-hashset-from-positive",
    "rust-std-collections-btreeset-from-positive",
    "rust-std-collections-vecdeque-from-positive",
    "rust-std-collections-shadowed-std-hard-negative",
    "rust-std-collections-type-alias-std-hard-negative",
];
const RUST_STDLIB_MAP_FACTORY_PRODUCER_IDS: &[&str] = &[RUST_STDLIB_MAP_FACTORY_PRODUCER_ID];
const RUST_STDLIB_MAP_FACTORY_CONTRACT_IDS: &[&str] = &[RUST_STDLIB_MAP_FACTORY_CONTRACT_ID];
const RUST_STDLIB_MAP_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "rust-std-map-hashmap-from-positive",
    "rust-std-map-btreemap-from-positive",
    "rust-std-map-shadowed-std-hard-negative",
    "rust-std-map-type-alias-std-hard-negative",
];
const RUST_STDLIB_OPTION_PRODUCER_IDS: &[&str] = &[RUST_STDLIB_OPTION_PRODUCER_ID];
const RUST_STDLIB_OPTION_CONTRACT_IDS: &[&str] = &[
    RUST_STDLIB_OPTION_SOME_CONTRACT_ID,
    RUST_STDLIB_OPTION_NONE_CONTRACT_ID,
    RUST_STDLIB_OPTION_AND_THEN_CONTRACT_ID,
];
const RUST_STDLIB_OPTION_CONFORMANCE_REFS: &[&str] = &[
    "rust-option-some-positive",
    "rust-option-none-positive",
    "rust-option-and-then-positive",
    "rust-option-some-shadow-hard-negative",
    "rust-option-none-shadow-hard-negative",
    "rust-option-and-then-non-option-hard-negative",
];
const RUST_STDLIB_INTEGER_METHOD_PRODUCER_IDS: &[&str] = &[RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID];
const RUST_STDLIB_INTEGER_METHOD_CONTRACT_IDS: &[&str] = &[
    SCALAR_INTEGER_METHOD_ABS_CONTRACT_ID,
    SCALAR_INTEGER_METHOD_MIN_CONTRACT_ID,
    SCALAR_INTEGER_METHOD_MAX_CONTRACT_ID,
    SCALAR_INTEGER_METHOD_CLAMP_CONTRACT_ID,
];
const RUST_STDLIB_INTEGER_METHOD_CONFORMANCE_REFS: &[&str] = &[
    "rust-integer-method-abs-positive",
    "rust-integer-method-min-positive",
    "rust-integer-method-max-positive",
    "rust-integer-method-clamp-positive",
    "rust-integer-method-non-integer-receiver-hard-negative",
    "rust-integer-method-unsupported-arity-hard-negative",
];
const JAVA_STDLIB_MATH_PRODUCER_IDS: &[&str] = &[JAVA_STDLIB_MATH_PRODUCER_ID];
const JAVA_STDLIB_MATH_CONTRACT_IDS: &[&str] = &[
    SCALAR_INTEGER_METHOD_ABS_CONTRACT_ID,
    SCALAR_INTEGER_METHOD_MIN_CONTRACT_ID,
    SCALAR_INTEGER_METHOD_MAX_CONTRACT_ID,
];
const JAVA_STDLIB_MATH_CONFORMANCE_REFS: &[&str] = &[
    "java-math-abs-positive",
    "java-math-min-positive",
    "java-math-max-positive",
    "java-math-shadowed-math-hard-negative",
    "java-math-non-integer-argument-hard-negative",
    "java-math-unsupported-arity-hard-negative",
];
const MAP_GET_PROTOCOL_PRODUCER_IDS: &[&str] = &[MAP_GET_PROTOCOL_PRODUCER_ID];
const MAP_GET_PROTOCOL_CONTRACT_IDS: &[&str] = &[MAP_GET_CONTRACT_ID];
const MAP_GET_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "map-get-rust-positive",
    "map-get-java-positive",
    "map-get-js-positive",
    "map-get-non-map-receiver-hard-negative",
    "map-get-unsupported-arity-hard-negative",
];
const MAP_GET_DEFAULT_PROTOCOL_PRODUCER_IDS: &[&str] = &[MAP_GET_DEFAULT_PROTOCOL_PRODUCER_ID];
const MAP_GET_DEFAULT_PROTOCOL_CONTRACT_IDS: &[&str] = &[MAP_GET_DEFAULT_CONTRACT_ID];
const MAP_GET_DEFAULT_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "map-get-default-python-get-positive",
    "map-get-default-ruby-fetch-positive",
    "map-get-default-java-get-or-default-positive",
    "map-get-default-non-map-receiver-hard-negative",
    "map-get-default-unsupported-arity-hard-negative",
];
const FREE_FUNCTION_BUILTIN_PROTOCOL_PRODUCER_IDS: &[&str] =
    &[FREE_FUNCTION_BUILTIN_PROTOCOL_PRODUCER_ID];
const FREE_FUNCTION_BUILTIN_PROTOCOL_CONTRACT_IDS: &[&str] = &[FREE_FUNCTION_BUILTIN_CONTRACT_ID];
const FREE_FUNCTION_BUILTIN_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "free-function-builtin-python-len-positive",
    "free-function-builtin-python-range-positive",
    "free-function-builtin-python-reduction-positive",
    "free-function-builtin-go-len-positive",
    "free-function-builtin-go-append-positive",
    "free-function-builtin-swift-abs-positive",
    "free-function-builtin-missing-symbol-hard-negative",
    "free-function-builtin-compatibility-pack-hard-negative",
    "free-function-builtin-wrong-producer-hard-negative",
    "free-function-builtin-unsupported-arity-hard-negative",
];
const RECEIVER_MEMBERSHIP_PROTOCOL_PRODUCER_IDS: &[&str] =
    &[RECEIVER_MEMBERSHIP_PROTOCOL_PRODUCER_ID];
const RECEIVER_MEMBERSHIP_PROTOCOL_CONTRACT_IDS: &[&str] = &[RECEIVER_MEMBERSHIP_CONTRACT_ID];
const RECEIVER_MEMBERSHIP_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "receiver-membership-java-contains-key-positive",
    "receiver-membership-rust-contains-key-positive",
    "receiver-membership-ruby-key-positive",
    "receiver-membership-ruby-has-key-positive",
    "receiver-membership-python-contains-positive",
    "receiver-membership-js-includes-positive",
    "receiver-membership-js-has-positive",
    "receiver-membership-java-contains-positive",
    "receiver-membership-swift-contains-positive",
    "receiver-membership-ruby-member-positive",
    "receiver-membership-missing-receiver-proof-hard-negative",
    "receiver-membership-unsupported-arity-hard-negative",
    "receiver-membership-go-slices-contains-out-of-scope-hard-negative",
];
const MAP_KEY_VIEW_PROTOCOL_PRODUCER_IDS: &[&str] = &[MAP_KEY_VIEW_PROTOCOL_PRODUCER_ID];
const MAP_KEY_VIEW_PROTOCOL_CONTRACT_IDS: &[&str] = &[
    MAP_KEY_VIEW_COLLECTION_CONTRACT_ID,
    MAP_KEY_VIEW_ITERATOR_CONTRACT_ID,
];
const MAP_KEY_VIEW_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "map-key-view-python-keys-positive",
    "map-key-view-ruby-keys-positive",
    "map-key-view-java-keyset-positive",
    "map-key-view-js-keys-positive",
    "map-key-view-non-map-receiver-hard-negative",
    "map-key-view-unsupported-arity-hard-negative",
];
const PROPERTY_BUILTIN_PROTOCOL_PRODUCER_IDS: &[&str] = &[PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID];
const PROPERTY_BUILTIN_PROTOCOL_CONTRACT_IDS: &[&str] = &[
    PROPERTY_BUILTIN_LEN_CONTRACT_ID,
    PROPERTY_BUILTIN_IS_EMPTY_CONTRACT_ID,
];
const PROPERTY_BUILTIN_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "property-builtin-js-length-positive",
    "property-builtin-java-length-positive",
    "property-builtin-swift-count-positive",
    "property-builtin-swift-is-empty-positive",
    "property-builtin-missing-receiver-proof-hard-negative",
    "property-builtin-wrong-pack-hard-negative",
    "property-builtin-unsupported-property-hard-negative",
];
const BUILTIN_METHOD_CALL_PROTOCOL_PRODUCER_IDS: &[&str] =
    &[BUILTIN_METHOD_CALL_PROTOCOL_PRODUCER_ID];
const BUILTIN_METHOD_CALL_PROTOCOL_CONTRACT_IDS: &[&str] = &[BUILTIN_METHOD_CALL_CONTRACT_ID];
const BUILTIN_METHOD_CALL_PROTOCOL_CONFORMANCE_REFS: &[&str] = &[
    "builtin-method-call-python-append-positive",
    "builtin-method-call-js-push-positive",
    "builtin-method-call-rust-len-positive",
    "builtin-method-call-java-size-positive",
    "builtin-method-call-python-startswith-positive",
    "builtin-method-call-python-join-positive",
    "builtin-method-call-rust-unwrap-or-positive",
    "builtin-method-call-go-fmt-print-positive",
    "builtin-method-call-python-reduce-positive",
    "builtin-method-call-missing-receiver-proof-hard-negative",
    "builtin-method-call-wrong-pack-hard-negative",
    "builtin-method-call-unsupported-arity-hard-negative",
];
const JAVA_STDLIB_MAP_FACTORY_PRODUCER_IDS: &[&str] = &[JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID];
const JAVA_STDLIB_MAP_FACTORY_CONTRACT_IDS: &[&str] = &[
    JAVA_STDLIB_MAP_FACTORY_OF_CONTRACT_ID,
    JAVA_STDLIB_MAP_FACTORY_OF_ENTRIES_CONTRACT_ID,
];
const JAVA_STDLIB_MAP_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "java-map-of-positive",
    "java-map-of-entries-positive",
    "java-map-missing-import-hard-negative",
    "java-map-entry-boundary-hard-negative",
];
const JAVA_STDLIB_MAP_ENTRY_PRODUCER_IDS: &[&str] = &[JAVA_STDLIB_MAP_ENTRY_PRODUCER_ID];
const JAVA_STDLIB_MAP_ENTRY_CONTRACT_IDS: &[&str] = &[JAVA_STDLIB_MAP_ENTRY_CONTRACT_ID];
const JAVA_STDLIB_MAP_ENTRY_CONFORMANCE_REFS: &[&str] = &[
    "java-map-entry-positive",
    "java-map-entry-missing-import-hard-negative",
    "java-map-entry-shadowed-map-hard-negative",
];
const JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS: &[&str] =
    &[JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID];
const JAVA_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS: &[&str] = &[
    JAVA_STDLIB_COLLECTION_FACTORY_LIST_OF_CONTRACT_ID,
    JAVA_STDLIB_COLLECTION_FACTORY_SET_OF_CONTRACT_ID,
    JAVA_STDLIB_COLLECTION_FACTORY_ARRAYS_AS_LIST_CONTRACT_ID,
];
const JAVA_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS: &[&str] = &[
    "java-list-of-positive",
    "java-set-of-positive",
    "java-arrays-as-list-positive",
    "java-collection-missing-import-hard-negative",
    "java-collection-constructor-boundary-hard-negative",
];
const JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_IDS: &[&str] =
    &[JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID];
const JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONTRACT_IDS: &[&str] =
    &[JAVA_STDLIB_COLLECTION_CONSTRUCTOR_EMPTY_LIST_CONTRACT_ID];
const JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS: &[&str] = &[
    "java-arraylist-empty-constructor-positive",
    "java-linkedlist-empty-constructor-positive",
    "java-constructor-missing-import-hard-negative",
    "java-constructor-shadowed-type-hard-negative",
    "java-constructor-conflicting-import-hard-negative",
];
const JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_IDS: &[&str] =
    &[JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_ID];
const JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONTRACT_IDS: &[&str] =
    &[JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONTRACT_ID];
const JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONFORMANCE_REFS: &[&str] = &[
    "java-arrays-stream-positive",
    "java-arrays-stream-missing-import-hard-negative",
    "java-arrays-stream-shadowed-arrays-hard-negative",
];
const ITERATOR_IDENTITY_ADAPTER_PRODUCER_IDS: &[&str] = &[ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID];
const ITERATOR_IDENTITY_ADAPTER_CONTRACT_IDS: &[&str] = &[ITERATOR_IDENTITY_ADAPTER_CONTRACT_ID];
const ITERATOR_IDENTITY_ADAPTER_CONFORMANCE_REFS: &[&str] = &[
    "rust-iterator-identity-iter-positive",
    "rust-iterator-identity-collect-positive",
    "java-iterator-identity-stream-positive",
    "iterator-identity-non-iterable-receiver-hard-negative",
    "iterator-identity-unsupported-arity-hard-negative",
];
const RUST_STDLIB_VEC_PRODUCER_IDS: &[&str] = &[RUST_STDLIB_VEC_PRODUCER_ID];
const RUST_STDLIB_VEC_CONTRACT_IDS: &[&str] = &[
    RUST_STDLIB_VEC_MACRO_CONTRACT_ID,
    RUST_STDLIB_VEC_NEW_CONTRACT_ID,
];
const RUST_STDLIB_VEC_CONFORMANCE_REFS: &[&str] = &[
    "rust-vec-macro-factory-positive",
    "rust-vec-new-factory-positive",
    "rust-vec-macro-shadowed-hard-negative",
    "rust-vec-new-shadowed-hard-negative",
];
const PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS: &[&str] =
    &["python.stdlib.type-domain-alias.contract"];
const PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS: &[&str] = &[PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID];
const PYTHON_STDLIB_TYPE_DOMAIN_HARD_NEGATIVE_REFS: &[&str] =
    &["python-typing-domain-wrong-module-hard-negative"];
const NO_TYPE_DOMAIN_ALIAS_CONTRACTS: &[BuiltinTypeDomainAliasContract] = &[];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuiltinLanguageBinding {
    pub langs: &'static [Lang],
    pub file_extensions: &'static [&'static str],
    pub parser: &'static str,
    pub lowering_entrypoint: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinPackDescriptor {
    pub id: &'static str,
    pub kind: SemanticPackKind,
    pub display_name: &'static str,
    pub trust: PackTrust,
    pub enabled_by_default: bool,
    pub supported_languages: &'static [&'static str],
    pub supported_packages: &'static [&'static str],
    pub language: Option<BuiltinLanguageBinding>,
    pub evidence_producer_ids: &'static [&'static str],
    pub source_fact_producer_ids: &'static [&'static str],
    pub contract_ids: &'static [&'static str],
    pub type_domain_alias_contracts: &'static [BuiltinTypeDomainAliasContract],
    static_value_law_ids: &'static [&'static str],
    dynamic_value_law_ids: Option<fn() -> Vec<&'static str>>,
    static_conformance_refs: &'static [&'static str],
    dynamic_conformance_refs: Option<fn() -> Vec<&'static str>>,
    counts: fn() -> SemanticPackCounts,
}

impl BuiltinPackDescriptor {
    pub fn value_law_ids(self) -> Vec<&'static str> {
        let mut ids = self.static_value_law_ids.to_vec();
        if let Some(dynamic_ids) = self.dynamic_value_law_ids {
            ids.extend(dynamic_ids());
        }
        ids
    }

    pub fn conformance_refs(self) -> Vec<&'static str> {
        let mut refs = self.static_conformance_refs.to_vec();
        if let Some(dynamic_refs) = self.dynamic_conformance_refs {
            refs.extend(dynamic_refs());
        }
        refs
    }

    pub fn counts(self) -> SemanticPackCounts {
        (self.counts)()
    }

    fn summary(self) -> SemanticPackSummary {
        SemanticPackSummary {
            id: self.id.to_string(),
            hash: semantic_pack_hash(self.id),
            kind: self.kind,
            version: env!("CARGO_PKG_VERSION").to_string(),
            display_name: self.display_name.to_string(),
            trust: self.trust,
            enabled_by_default: self.enabled_by_default,
            source: SemanticPackSource::CompiledBuiltin,
            influence: SemanticPackInfluence::EvidenceAndContracts,
            manifest_path: None,
            provider: "Corca, Inc.".to_string(),
            repository: "https://github.com/corca-ai/nose".to_string(),
            license: "MIT".to_string(),
            supported_languages: self
                .supported_languages
                .iter()
                .map(|language| (*language).to_string())
                .collect(),
            counts: self.counts(),
        }
    }
}

fn empty_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: 0,
        contracts: 0,
        value_laws: 0,
        positive_fixtures: 0,
        hard_negatives: 0,
    }
}

fn c_language_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: C_LANGUAGE_PRODUCER_IDS.len(),
        contracts: 0,
        value_laws: 0,
        positive_fixtures: C_LANGUAGE_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: C_LANGUAGE_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn language_core_and_source_fact_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: 2,
        contracts: 0,
        value_laws: 0,
        positive_fixtures: 0,
        hard_negatives: 0,
    }
}

fn python_builtin_collection_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_IDS.len(),
        contracts: PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PYTHON_BUILTIN_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: PYTHON_BUILTIN_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn python_stdlib_collection_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS.len(),
        contracts: PYTHON_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PYTHON_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: PYTHON_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn python_stdlib_math_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PYTHON_STDLIB_MATH_PRODUCER_IDS.len(),
        contracts: PYTHON_STDLIB_MATH_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PYTHON_STDLIB_MATH_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: PYTHON_STDLIB_MATH_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_promise_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_PROMISE_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_PROMISE_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_PROMISE_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_PROMISE_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_array_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_ARRAY_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_ARRAY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_ARRAY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_ARRAY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_boolean_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_BOOLEAN_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_BOOLEAN_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_BOOLEAN_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_regex_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_REGEX_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_REGEX_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_REGEX_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_REGEX_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_static_index_membership_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn js_like_builtin_collection_constructor_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_IDS.len(),
        contracts: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn ruby_stdlib_set_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUBY_STDLIB_SET_PRODUCER_IDS.len(),
        contracts: RUBY_STDLIB_SET_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUBY_STDLIB_SET_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUBY_STDLIB_SET_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn rust_stdlib_collection_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS.len(),
        contracts: RUST_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUST_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUST_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn rust_stdlib_map_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUST_STDLIB_MAP_FACTORY_PRODUCER_IDS.len(),
        contracts: RUST_STDLIB_MAP_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUST_STDLIB_MAP_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUST_STDLIB_MAP_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn rust_stdlib_option_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUST_STDLIB_OPTION_PRODUCER_IDS.len(),
        contracts: RUST_STDLIB_OPTION_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUST_STDLIB_OPTION_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUST_STDLIB_OPTION_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn rust_stdlib_integer_method_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUST_STDLIB_INTEGER_METHOD_PRODUCER_IDS.len(),
        contracts: RUST_STDLIB_INTEGER_METHOD_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUST_STDLIB_INTEGER_METHOD_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUST_STDLIB_INTEGER_METHOD_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_map_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_MAP_FACTORY_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_MAP_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_MAP_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_MAP_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_math_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_MATH_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_MATH_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_MATH_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_MATH_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn map_get_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: MAP_GET_PROTOCOL_PRODUCER_IDS.len(),
        contracts: MAP_GET_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: MAP_GET_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: MAP_GET_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn map_get_default_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: MAP_GET_DEFAULT_PROTOCOL_PRODUCER_IDS.len(),
        contracts: MAP_GET_DEFAULT_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: MAP_GET_DEFAULT_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: MAP_GET_DEFAULT_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn free_function_builtin_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: FREE_FUNCTION_BUILTIN_PROTOCOL_PRODUCER_IDS.len(),
        contracts: FREE_FUNCTION_BUILTIN_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: FREE_FUNCTION_BUILTIN_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: FREE_FUNCTION_BUILTIN_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn receiver_membership_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RECEIVER_MEMBERSHIP_PROTOCOL_PRODUCER_IDS.len(),
        contracts: RECEIVER_MEMBERSHIP_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RECEIVER_MEMBERSHIP_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RECEIVER_MEMBERSHIP_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn map_key_view_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: MAP_KEY_VIEW_PROTOCOL_PRODUCER_IDS.len(),
        contracts: MAP_KEY_VIEW_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: MAP_KEY_VIEW_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: MAP_KEY_VIEW_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn property_builtin_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PROPERTY_BUILTIN_PROTOCOL_PRODUCER_IDS.len(),
        contracts: PROPERTY_BUILTIN_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PROPERTY_BUILTIN_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: PROPERTY_BUILTIN_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn builtin_method_call_protocol_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: BUILTIN_METHOD_CALL_PROTOCOL_PRODUCER_IDS.len(),
        contracts: BUILTIN_METHOD_CALL_PROTOCOL_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: BUILTIN_METHOD_CALL_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: BUILTIN_METHOD_CALL_PROTOCOL_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_map_entry_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_MAP_ENTRY_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_MAP_ENTRY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_MAP_ENTRY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_MAP_ENTRY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_collection_factory_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_collection_constructor_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn java_stdlib_static_collection_adapter_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_IDS.len(),
        contracts: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn iterator_identity_adapter_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: ITERATOR_IDENTITY_ADAPTER_PRODUCER_IDS.len(),
        contracts: ITERATOR_IDENTITY_ADAPTER_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: ITERATOR_IDENTITY_ADAPTER_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: ITERATOR_IDENTITY_ADAPTER_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn rust_stdlib_vec_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: RUST_STDLIB_VEC_PRODUCER_IDS.len(),
        contracts: RUST_STDLIB_VEC_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: RUST_STDLIB_VEC_CONFORMANCE_REFS
            .iter()
            .filter(|id| !id.contains("hard-negative"))
            .count(),
        hard_negatives: RUST_STDLIB_VEC_CONFORMANCE_REFS
            .iter()
            .filter(|id| id.contains("hard-negative"))
            .count(),
    }
}

fn python_stdlib_type_domain_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS.len(),
        contracts: PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len(),
        hard_negatives: 2,
    }
}

fn python_stdlib_type_domain_conformance_refs() -> Vec<&'static str> {
    let mut refs = Vec::with_capacity(PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len() * 2);
    for row in PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS {
        refs.push(row.positive_fixture);
        refs.push(row.hard_negative_fixture);
    }
    refs.extend(PYTHON_STDLIB_TYPE_DOMAIN_HARD_NEGATIVE_REFS);
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn value_graph_law_ids() -> Vec<&'static str> {
    pack_facing_value_laws()
        .iter()
        .map(|law| law.law_id)
        .collect()
}

fn value_graph_law_conformance_refs() -> Vec<&'static str> {
    let mut refs = pack_facing_value_laws()
        .iter()
        .flat_map(|law| law.conformance_refs.iter().copied())
        .collect::<Vec<_>>();
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn value_graph_law_counts() -> SemanticPackCounts {
    let laws = pack_facing_value_laws();
    SemanticPackCounts {
        evidence_producers: 0,
        contracts: 0,
        value_laws: laws.len(),
        positive_fixtures: laws
            .iter()
            .map(|law| {
                law.conformance_refs
                    .iter()
                    .filter(|id| !id.contains("hard-negative"))
                    .count()
            })
            .sum(),
        hard_negatives: laws
            .iter()
            .map(|law| {
                law.conformance_refs
                    .iter()
                    .filter(|id| id.contains("hard-negative"))
                    .count()
            })
            .sum(),
    }
}

static BUILTIN_PACK_DESCRIPTORS: &[BuiltinPackDescriptor] = &[
    BuiltinPackDescriptor {
        id: BUILTIN_COMPAT_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose builtin semantic compatibility facade",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: NO_LANGUAGES,
        supported_packages: NO_PACKAGES,
        language: None,
        evidence_producer_ids: NO_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: empty_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Python language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: PYTHON_BINDING_LANGS,
            file_extensions: PYTHON_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-python",
            lowering_entrypoint: "nose_frontend::python::lower",
        }),
        evidence_producer_ids: PYTHON_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: PYTHON_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: JS_TS_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose JavaScript/TypeScript language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: JS_TS_BINDING_LANGS,
            file_extensions: JS_TS_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-javascript/tree-sitter-typescript",
            lowering_entrypoint: "nose_frontend::js_ts::lower",
        }),
        evidence_producer_ids: JS_TS_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: JS_TS_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: GO_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Go language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: GO_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: GO_BINDING_LANGS,
            file_extensions: GO_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-go",
            lowering_entrypoint: "nose_frontend::go::lower",
        }),
        evidence_producer_ids: GO_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: GO_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Rust language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: RUST_BINDING_LANGS,
            file_extensions: RUST_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-rust",
            lowering_entrypoint: "nose_frontend::rust::lower",
        }),
        evidence_producer_ids: RUST_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: RUST_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Java language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: JAVA_BINDING_LANGS,
            file_extensions: JAVA_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-java",
            lowering_entrypoint: "nose_frontend::java::lower",
        }),
        evidence_producer_ids: JAVA_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: JAVA_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: C_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose C language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: C_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: C_BINDING_LANGS,
            file_extensions: C_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-c",
            lowering_entrypoint: "nose_frontend::c::lower",
        }),
        evidence_producer_ids: C_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: C_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: C_LANGUAGE_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: c_language_counts,
    },
    BuiltinPackDescriptor {
        id: RUBY_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Ruby language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUBY_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: RUBY_BINDING_LANGS,
            file_extensions: RUBY_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-ruby",
            lowering_entrypoint: "nose_frontend::ruby::lower",
        }),
        evidence_producer_ids: RUBY_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: RUBY_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: SWIFT_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose Swift language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: SWIFT_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: SWIFT_BINDING_LANGS,
            file_extensions: SWIFT_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-swift",
            lowering_entrypoint: "nose_frontend::swift::lower",
        }),
        evidence_producer_ids: SWIFT_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: SWIFT_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: CSS_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose CSS language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: CSS_LANGUAGE,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: CSS_BINDING_LANGS,
            file_extensions: CSS_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-css",
            lowering_entrypoint: "nose_frontend::css::lower",
        }),
        evidence_producer_ids: CSS_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: CSS_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: HTML_EMBEDDED_LANGUAGE_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose HTML/Vue/Svelte embedded region language pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: HTML_EMBEDDED_LANGUAGES,
        supported_packages: NO_PACKAGES,
        language: Some(BuiltinLanguageBinding {
            langs: HTML_EMBEDDED_BINDING_LANGS,
            file_extensions: HTML_EMBEDDED_LANGUAGE_FILE_EXTENSIONS,
            parser: "tree-sitter-html + embedded JS/TS/CSS extraction",
            lowering_entrypoint: "nose_frontend::embedded::lower_regions",
        }),
        evidence_producer_ids: HTML_EMBEDDED_LANGUAGE_PRODUCER_IDS,
        source_fact_producer_ids: HTML_EMBEDDED_LANGUAGE_SOURCE_FACT_PRODUCER_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: language_core_and_source_fact_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Python builtins collection factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: PYTHON_BUILTIN_PACKAGES,
        language: None,
        evidence_producer_ids: PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: PYTHON_BUILTIN_COLLECTION_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: python_builtin_collection_factory_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Python stdlib collection factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: PYTHON_STDLIB_COLLECTION_FACTORY_PACKAGES,
        language: None,
        evidence_producer_ids: PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: PYTHON_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: PYTHON_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: python_stdlib_collection_factory_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_STDLIB_MATH_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Python stdlib math pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: PYTHON_STDLIB_MATH_PACKAGES,
        language: None,
        evidence_producer_ids: PYTHON_STDLIB_MATH_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: PYTHON_STDLIB_MATH_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: PYTHON_STDLIB_MATH_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: python_stdlib_math_counts,
    },
    BuiltinPackDescriptor {
        id: RUBY_STDLIB_SET_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Ruby stdlib Set pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUBY_LANGUAGE,
        supported_packages: RUBY_STDLIB_SET_PACKAGES,
        language: None,
        evidence_producer_ids: RUBY_STDLIB_SET_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUBY_STDLIB_SET_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUBY_STDLIB_SET_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: ruby_stdlib_set_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_STDLIB_VEC_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Rust stdlib Vec pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: RUST_STDLIB_VEC_PACKAGES,
        language: None,
        evidence_producer_ids: RUST_STDLIB_VEC_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUST_STDLIB_VEC_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUST_STDLIB_VEC_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: rust_stdlib_vec_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_STDLIB_OPTION_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Rust stdlib Option pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: RUST_STDLIB_OPTION_PACKAGES,
        language: None,
        evidence_producer_ids: RUST_STDLIB_OPTION_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUST_STDLIB_OPTION_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUST_STDLIB_OPTION_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: rust_stdlib_option_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_STDLIB_INTEGER_METHOD_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Rust stdlib integer method pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: RUST_STDLIB_INTEGER_METHOD_PACKAGES,
        language: None,
        evidence_producer_ids: RUST_STDLIB_INTEGER_METHOD_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUST_STDLIB_INTEGER_METHOD_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUST_STDLIB_INTEGER_METHOD_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: rust_stdlib_integer_method_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_STDLIB_COLLECTION_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Rust stdlib collection factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: RUST_STDLIB_COLLECTION_FACTORY_PACKAGES,
        language: None,
        evidence_producer_ids: RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUST_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUST_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: rust_stdlib_collection_factory_counts,
    },
    BuiltinPackDescriptor {
        id: RUST_STDLIB_MAP_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Rust stdlib map factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RUST_LANGUAGE,
        supported_packages: RUST_STDLIB_MAP_FACTORY_PACKAGES,
        language: None,
        evidence_producer_ids: RUST_STDLIB_MAP_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RUST_STDLIB_MAP_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RUST_STDLIB_MAP_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: rust_stdlib_map_factory_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_MATH_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib Math pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_MATH_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_MATH_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_MATH_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_MATH_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_math_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_MAP_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib map factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_MAP_FACTORY_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_MAP_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_MAP_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_MAP_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_map_factory_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_MAP_ENTRY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib map entry pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_MAP_ENTRY_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_MAP_ENTRY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_MAP_ENTRY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_MAP_ENTRY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_map_entry_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib collection factory pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_COLLECTION_FACTORY_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_COLLECTION_FACTORY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_COLLECTION_FACTORY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_collection_factory_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib collection constructor pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_collection_constructor_counts,
    },
    BuiltinPackDescriptor {
        id: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Java stdlib static collection adapter pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_LANGUAGE,
        supported_packages: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACKAGES,
        language: None,
        evidence_producer_ids: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: java_stdlib_static_collection_adapter_counts,
    },
    BuiltinPackDescriptor {
        id: MAP_GET_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose map-get protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: MAP_GET_PROTOCOL_LANGUAGES,
        supported_packages: MAP_GET_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: MAP_GET_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: MAP_GET_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: MAP_GET_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: map_get_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: MAP_GET_DEFAULT_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose map-get-default protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: MAP_GET_DEFAULT_PROTOCOL_LANGUAGES,
        supported_packages: MAP_GET_DEFAULT_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: MAP_GET_DEFAULT_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: MAP_GET_DEFAULT_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: MAP_GET_DEFAULT_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: map_get_default_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: FREE_FUNCTION_BUILTIN_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose free-function builtin protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: FREE_FUNCTION_BUILTIN_PROTOCOL_LANGUAGES,
        supported_packages: FREE_FUNCTION_BUILTIN_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: FREE_FUNCTION_BUILTIN_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: FREE_FUNCTION_BUILTIN_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: FREE_FUNCTION_BUILTIN_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: free_function_builtin_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: RECEIVER_MEMBERSHIP_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose receiver-membership protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: RECEIVER_MEMBERSHIP_PROTOCOL_LANGUAGES,
        supported_packages: RECEIVER_MEMBERSHIP_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: RECEIVER_MEMBERSHIP_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: RECEIVER_MEMBERSHIP_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: RECEIVER_MEMBERSHIP_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: receiver_membership_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: MAP_KEY_VIEW_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose map-key-view protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: MAP_KEY_VIEW_PROTOCOL_LANGUAGES,
        supported_packages: MAP_KEY_VIEW_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: MAP_KEY_VIEW_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: MAP_KEY_VIEW_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: MAP_KEY_VIEW_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: map_key_view_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: PROPERTY_BUILTIN_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose property builtin protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PROPERTY_BUILTIN_PROTOCOL_LANGUAGES,
        supported_packages: PROPERTY_BUILTIN_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: PROPERTY_BUILTIN_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: PROPERTY_BUILTIN_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: PROPERTY_BUILTIN_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: property_builtin_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: BUILTIN_METHOD_CALL_PROTOCOL_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose builtin method-call protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: BUILTIN_METHOD_CALL_PROTOCOL_LANGUAGES,
        supported_packages: BUILTIN_METHOD_CALL_PROTOCOL_PACKAGES,
        language: None,
        evidence_producer_ids: BUILTIN_METHOD_CALL_PROTOCOL_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: BUILTIN_METHOD_CALL_PROTOCOL_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: BUILTIN_METHOD_CALL_PROTOCOL_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: builtin_method_call_protocol_counts,
    },
    BuiltinPackDescriptor {
        id: ITERATOR_IDENTITY_ADAPTER_PACK_ID,
        kind: SemanticPackKind::ProtocolPack,
        display_name: "nose iterator identity adapter protocol pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JAVA_RUST_LANGUAGE,
        supported_packages: ITERATOR_IDENTITY_ADAPTER_PACKAGES,
        language: None,
        evidence_producer_ids: ITERATOR_IDENTITY_ADAPTER_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: ITERATOR_IDENTITY_ADAPTER_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: ITERATOR_IDENTITY_ADAPTER_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: iterator_identity_adapter_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins Promise pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_PROMISE_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_PROMISE_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_PROMISE_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_PROMISE_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_promise_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_ARRAY_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins Array pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_ARRAY_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_ARRAY_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_ARRAY_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_ARRAY_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_array_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_BOOLEAN_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins Boolean pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_BOOLEAN_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_BOOLEAN_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_BOOLEAN_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_boolean_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_REGEX_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins RegExp pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_REGEX_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_REGEX_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_REGEX_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_REGEX_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_regex_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins static index membership pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_static_index_membership_counts,
    },
    BuiltinPackDescriptor {
        id: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose JavaScript builtins collection constructor pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: JS_LIKE_LANGUAGE,
        supported_packages: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACKAGES,
        language: None,
        evidence_producer_ids: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONTRACT_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_CONFORMANCE_REFS,
        dynamic_conformance_refs: None,
        counts: js_like_builtin_collection_constructor_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Python stdlib type-domain pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: PYTHON_STDLIB_TYPE_DOMAIN_PACKAGES,
        language: None,
        evidence_producer_ids: PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS,
        type_domain_alias_contracts: PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: Some(python_stdlib_type_domain_conformance_refs),
        counts: python_stdlib_type_domain_counts,
    },
    BuiltinPackDescriptor {
        id: VALUE_GRAPH_LAW_PACK_ID,
        kind: SemanticPackKind::LawPack,
        display_name: "nose value-graph law pack",
        trust: PackTrust::BuiltinDefault,
        enabled_by_default: true,
        supported_languages: NO_LANGUAGES,
        supported_packages: NO_PACKAGES,
        language: None,
        evidence_producer_ids: NO_IDS,
        source_fact_producer_ids: NO_IDS,
        contract_ids: NO_IDS,
        type_domain_alias_contracts: NO_TYPE_DOMAIN_ALIAS_CONTRACTS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: Some(value_graph_law_ids),
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: Some(value_graph_law_conformance_refs),
        counts: value_graph_law_counts,
    },
];

pub fn builtin_pack_descriptors() -> &'static [BuiltinPackDescriptor] {
    BUILTIN_PACK_DESCRIPTORS
}

pub fn builtin_pack_descriptor(pack_id: &str) -> Option<&'static BuiltinPackDescriptor> {
    BUILTIN_PACK_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.id == pack_id)
}

pub fn builtin_compat_semantic_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(BUILTIN_COMPAT_PACK_ID)
        .expect("builtin compatibility pack descriptor exists")
        .summary()
}

pub fn first_party_semantic_pack() -> SemanticPackSummary {
    builtin_compat_semantic_pack()
}

pub fn python_stdlib_type_domain_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)
        .expect("Python stdlib type-domain descriptor exists")
        .summary()
}

pub fn value_graph_law_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(VALUE_GRAPH_LAW_PACK_ID)
        .expect("value-graph law descriptor exists")
        .summary()
}

pub fn first_party_value_law_pack() -> SemanticPackSummary {
    value_graph_law_pack()
}

pub(super) fn compiled_builtin_packs() -> Vec<SemanticPackSummary> {
    BUILTIN_PACK_DESCRIPTORS
        .iter()
        .map(|descriptor| descriptor.summary())
        .collect()
}

pub(super) fn is_compiled_builtin_pack_id(pack_id: &str) -> bool {
    compiled_builtin_packs()
        .iter()
        .any(|pack| pack.id == pack_id)
}

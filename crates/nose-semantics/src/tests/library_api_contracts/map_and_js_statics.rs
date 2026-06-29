use super::*;

#[test]
fn map_key_view_contracts_distinguish_collection_and_iterator_views() {
    assert_eq!(
        map_key_view_contract(Lang::Python, "keys", 0),
        Some(MapKeyViewContract {
            method: "keys",
            kind: MapKeyViewKind::Collection,
        })
    );
    assert_eq!(
        map_key_view_contract(Lang::Java, "keySet", 0),
        Some(MapKeyViewContract {
            method: "keySet",
            kind: MapKeyViewKind::Collection,
        })
    );
    assert_eq!(
        map_key_view_contract(Lang::TypeScript, "keys", 0),
        Some(MapKeyViewContract {
            method: "keys",
            kind: MapKeyViewKind::Iterator,
        })
    );
    assert_eq!(map_key_view_contract(Lang::JavaScript, "keySet", 0), None);
    assert_eq!(map_key_view_contract(Lang::Python, "keys", 1), None);
    assert_eq!(
        map_key_view_wrapper_contract(Lang::JavaScript, "Array", "from", 1),
        Some(MapKeyViewWrapperContract {
            receiver: "Array",
            method: "from",
            qualified_path: "Array.from",
        })
    );
    assert_eq!(
        map_key_view_wrapper_contract(Lang::Python, "Array", "from", 1),
        None
    );
    assert_eq!(
        map_key_view_contract_by_hash(Lang::Java, stable_symbol_hash("keySet"), 0)
            .map(|contract| contract.kind),
        Some(MapKeyViewKind::Collection)
    );
    assert!(map_key_view_wrapper_contract_by_hash(
        Lang::TypeScript,
        "Array",
        stable_symbol_hash("from"),
        1,
    )
    .is_some());
}

#[test]
fn go_zero_map_contracts_are_go_surface_and_default_constrained() {
    assert_eq!(
        go_zero_map_lookup_contract(Lang::Go),
        Some(GoZeroMapLookupContract {
            map_literal_tag: "composite_literal",
            entry_tag: "keyed_element",
            canonical_value_tag: "go_literal_zero_map",
        })
    );
    assert_eq!(go_zero_map_lookup_contract(Lang::Python), None);
    assert_eq!(
        go_zero_map_default_kind(Lang::Go, Payload::LitInt(1)),
        Some(GoZeroMapDefaultKind::Int)
    );
    assert_eq!(
        go_zero_map_default_kind(Lang::Go, Payload::LitStr(stable_symbol_hash("x"))),
        Some(GoZeroMapDefaultKind::String)
    );
    assert_eq!(
        go_zero_map_default_kind(Lang::Go, Payload::Lit(LitClass::Null)),
        Some(GoZeroMapDefaultKind::Null)
    );
    assert_eq!(
        go_zero_map_default_kind(Lang::JavaScript, Payload::LitInt(1)),
        None
    );
    assert_eq!(go_zero_map_default_kind(Lang::Go, Payload::None), None);
}

#[test]
fn map_get_contracts_are_language_and_arity_constrained() {
    assert_eq!(
        map_get_contract(Lang::Rust, "get", 1),
        Some(MapGetContract {
            method: "get",
            receiver: MethodReceiverContract::ExactMap,
        })
    );
    assert_eq!(
        map_get_contract_by_hash(Lang::Java, stable_symbol_hash("get"), 1),
        Some(MapGetContract {
            method: "get",
            receiver: MethodReceiverContract::ExactMap,
        })
    );
    assert_eq!(
        map_get_contract(Lang::TypeScript, "get", 1),
        Some(MapGetContract {
            method: "get",
            receiver: MethodReceiverContract::ExactMap,
        })
    );
    assert_eq!(map_get_contract(Lang::Python, "get", 1), None);
    assert_eq!(map_get_contract(Lang::Rust, "get", 2), None);
    assert_eq!(map_get_contract(Lang::Java, "getOrDefault", 1), None);
}

#[test]
fn js_static_builtin_contracts_are_language_and_arity_constrained() {
    assert_eq!(
        static_global_symbol_contract(Lang::JavaScript, "Math"),
        Some(StaticGlobalSymbolContract {
            name: "Math",
            requires_unshadowed: true,
        })
    );
    assert_eq!(
        static_global_symbol_contract(Lang::TypeScript, "undefined"),
        Some(StaticGlobalSymbolContract {
            name: "undefined",
            requires_unshadowed: true,
        })
    );
    assert_eq!(
        static_global_symbol_contract(Lang::TypeScript, "Promise"),
        Some(StaticGlobalSymbolContract {
            name: "Promise",
            requires_unshadowed: true,
        })
    );
    assert_eq!(
        qualified_global_symbol_contract(Lang::JavaScript, "Promise.resolve"),
        Some(QualifiedGlobalSymbolContract {
            path: "Promise.resolve",
            root: "Promise",
            requires_unshadowed_root: true,
        })
    );
    assert_eq!(
        qualified_global_symbol_contract(Lang::JavaScript, "Promise.reject"),
        Some(QualifiedGlobalSymbolContract {
            path: "Promise.reject",
            root: "Promise",
            requires_unshadowed_root: true,
        })
    );
    assert_eq!(
        qualified_global_symbol_contract(Lang::JavaScript, "Promise.all"),
        Some(QualifiedGlobalSymbolContract {
            path: "Promise.all",
            root: "Promise",
            requires_unshadowed_root: true,
        })
    );
    assert_eq!(static_global_symbol_contract(Lang::Python, "Math"), None);
    assert_eq!(
        static_global_symbol_contract(Lang::JavaScript, "WeakMap"),
        None
    );
    assert_eq!(
        typeof_operator_contract(Lang::TypeScript, "typeof", 1),
        Some(TypeofOperatorContract {
            name: "typeof",
            required_source_fact: SourceFactKind::Operator(SourceOperatorKind::Typeof),
        })
    );
    assert_eq!(typeof_operator_contract(Lang::Python, "typeof", 1), None);
    assert_eq!(
        typeof_operator_contract(Lang::JavaScript, "typeof", 2),
        None
    );
    assert_eq!(
        js_array_is_array_contract(Lang::JavaScript, "Array", "isArray", 1),
        Some(StaticGlobalMethodContract {
            receiver: "Array",
            method: "isArray",
            qualified_path: "Array.isArray",
            requires_unshadowed_receiver: true,
        })
    );
    assert_eq!(
        js_array_is_array_contract(Lang::Python, "Array", "isArray", 1),
        None
    );
    assert_eq!(
        js_array_is_array_contract(Lang::TypeScript, "Array", "isArray", 2),
        None
    );
    assert_eq!(
        js_boolean_coercion_contract(Lang::JavaScript, "Boolean", 1),
        Some(StaticGlobalFunctionContract {
            function: "Boolean",
            requires_unshadowed_function: true,
        })
    );
    assert_eq!(
        js_boolean_coercion_contract(Lang::TypeScript, "Boolean", 1),
        Some(StaticGlobalFunctionContract {
            function: "Boolean",
            requires_unshadowed_function: true,
        })
    );
    assert_eq!(
        js_boolean_coercion_contract(Lang::Python, "Boolean", 1),
        None
    );
    assert_eq!(
        js_boolean_coercion_contract(Lang::JavaScript, "Boolean", 2),
        None
    );
    assert_eq!(
        regex_test_contract(Lang::JavaScript, "test", 1),
        Some(RegexTestContract {
            method: "test",
            required_receiver_fact: SourceFactKind::Literal(SourceLiteralKind::Regex),
        })
    );
    assert_eq!(regex_test_contract(Lang::Ruby, "test", 1), None);
}

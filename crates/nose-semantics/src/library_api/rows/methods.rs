use super::*;

pub fn library_map_key_view_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryMapKeyViewContract> {
    if arg_count != 0 {
        return None;
    }
    let result = match (lang, method) {
        (Lang::Python | Lang::Ruby, "keys") => MapKeyViewContract {
            method: "keys",
            kind: MapKeyViewKind::Collection,
        },
        (Lang::Java, "keySet") => MapKeyViewContract {
            method: "keySet",
            kind: MapKeyViewKind::Collection,
        },
        (Lang::JavaScript | Lang::TypeScript | Lang::Vue | Lang::Svelte | Lang::Html, "keys") => {
            MapKeyViewContract {
                method: "keys",
                kind: MapKeyViewKind::Iterator,
            }
        }
        _ => return None,
    };
    Some(LibraryMapKeyViewContract {
        pack_id: MAP_KEY_VIEW_PROTOCOL_PACK_ID,
        id: LibraryApiContractId::MapKeyView(result.kind),
        callee: LibraryApiCalleeContract::Method {
            method: result.method,
            receiver: MethodReceiverContract::ExactMap,
        },
        result,
    })
}

pub fn library_map_key_view_contract_by_hash(
    lang: Lang,
    method_hash: u64,
    arg_count: usize,
) -> Option<LibraryMapKeyViewContract> {
    ["keys", "keySet"].into_iter().find_map(|method| {
        (stable_symbol_hash(method) == method_hash)
            .then(|| library_map_key_view_contract(lang, method, arg_count))
            .flatten()
    })
}

pub fn library_map_key_view_wrapper_contract(
    lang: Lang,
    receiver: &str,
    method: &str,
    arg_count: usize,
) -> Option<LibraryMapKeyViewWrapperContract> {
    if !js_like_lang(lang) || receiver != "Array" || method != "from" || arg_count != 1 {
        return None;
    }
    let result = MapKeyViewWrapperContract {
        receiver: "Array",
        method: "from",
        qualified_path: "Array.from",
    };
    Some(LibraryMapKeyViewWrapperContract {
        pack_id: JS_LIKE_BUILTIN_ARRAY_PACK_ID,
        id: LibraryApiContractId::MapKeyViewWrapper,
        callee: LibraryApiCalleeContract::StaticGlobalMethod {
            receiver: result.receiver,
            method: result.method,
            qualified_path: result.qualified_path,
            requires_unshadowed_receiver: true,
        },
        result,
    })
}

pub fn library_map_key_view_wrapper_contract_by_hash(
    lang: Lang,
    receiver: &str,
    method_hash: u64,
    arg_count: usize,
) -> Option<LibraryMapKeyViewWrapperContract> {
    (method_hash == stable_symbol_hash("from"))
        .then(|| library_map_key_view_wrapper_contract(lang, receiver, "from", arg_count))
        .flatten()
}

pub fn library_map_get_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryMapGetContract> {
    if !matches!(
        lang,
        Lang::Java
            | Lang::Rust
            | Lang::JavaScript
            | Lang::TypeScript
            | Lang::Vue
            | Lang::Svelte
            | Lang::Html
    ) || method != "get"
        || arg_count != 1
    {
        return None;
    }
    let result = MapGetContract {
        method: "get",
        receiver: MethodReceiverContract::ExactMap,
    };
    Some(LibraryMapGetContract {
        pack_id: MAP_GET_PROTOCOL_PACK_ID,
        id: LibraryApiContractId::MapGet,
        callee: LibraryApiCalleeContract::Method {
            method: result.method,
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_map_get_contract_by_hash(
    lang: Lang,
    method_hash: u64,
    arg_count: usize,
) -> Option<LibraryMapGetContract> {
    (method_hash == stable_symbol_hash("get"))
        .then(|| library_map_get_contract(lang, "get", arg_count))
        .flatten()
}

pub fn library_js_array_is_array_contract(
    lang: Lang,
    receiver: &str,
    method: &str,
    arg_count: usize,
) -> Option<LibraryStaticGlobalMethodContract> {
    if !js_like_lang(lang) || receiver != "Array" || method != "isArray" || arg_count != 1 {
        return None;
    }
    let result = StaticGlobalMethodContract {
        receiver: "Array",
        method: "isArray",
        qualified_path: "Array.isArray",
        requires_unshadowed_receiver: true,
    };
    Some(LibraryStaticGlobalMethodContract {
        pack_id: JS_LIKE_BUILTIN_ARRAY_PACK_ID,
        id: LibraryApiContractId::JsArrayIsArray,
        callee: LibraryApiCalleeContract::StaticGlobalMethod {
            receiver: result.receiver,
            method: result.method,
            qualified_path: result.qualified_path,
            requires_unshadowed_receiver: result.requires_unshadowed_receiver,
        },
        result,
    })
}

pub fn library_js_boolean_coercion_contract(
    lang: Lang,
    function: &str,
    arg_count: usize,
) -> Option<LibraryStaticGlobalFunctionContract> {
    if !js_like_lang(lang) || function != "Boolean" || arg_count != 1 {
        return None;
    }
    let result = StaticGlobalFunctionContract {
        function: "Boolean",
        requires_unshadowed_function: true,
    };
    Some(LibraryStaticGlobalFunctionContract {
        pack_id: JS_LIKE_BUILTIN_BOOLEAN_PACK_ID,
        id: LibraryApiContractId::JsBooleanCoercion,
        callee: LibraryApiCalleeContract::StaticGlobalFunction {
            function: result.function,
            requires_unshadowed_function: result.requires_unshadowed_function,
        },
        result,
    })
}

pub fn library_regex_test_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryRegexTestContract> {
    if !js_like_lang(lang) || method != "test" || arg_count != 1 {
        return None;
    }
    let result = RegexTestContract {
        method: "test",
        required_receiver_fact: SourceFactKind::Literal(SourceLiteralKind::Regex),
    };
    Some(LibraryRegexTestContract {
        pack_id: JS_LIKE_BUILTIN_REGEX_PACK_ID,
        id: LibraryApiContractId::RegexTest,
        callee: LibraryApiCalleeContract::RegexLiteralMethod {
            method: result.method,
            required_receiver_fact: result.required_receiver_fact,
        },
        result,
    })
}

pub fn library_static_index_membership_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryStaticIndexMembershipContract> {
    let result = static_index_membership_contract(lang, method, arg_count)?;
    Some(LibraryStaticIndexMembershipContract {
        pack_id: JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
        id: LibraryApiContractId::JsLikeStaticIndexMembership(result.kind),
        callee: LibraryApiCalleeContract::StaticIndexMembershipMethod {
            method: result.method,
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_imported_namespace_function_contract(
    lang: Lang,
    function: &str,
    arg_count: usize,
) -> Option<LibraryImportedNamespaceFunctionContract> {
    let result = match (lang, function, arg_count) {
        (Lang::Python, "prod", 1 | 2) => ImportedNamespaceFunctionContract {
            module: "math",
            function: "prod",
            receiver: MethodReceiverContract::ImportedNamespace("math"),
            semantic: ImportedNamespaceFunctionSemantic::ProductReduction {
                op: Op::Mul,
                identity: 1,
            },
        },
        _ => return None,
    };
    Some(LibraryImportedNamespaceFunctionContract {
        pack_id: PYTHON_STDLIB_MATH_PACK_ID,
        id: LibraryApiContractId::ImportedNamespaceFunction(result.semantic),
        callee: LibraryApiCalleeContract::ImportedNamespaceFunction {
            module: result.module,
            function: result.function,
        },
        result,
    })
}

pub fn library_promise_then_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryPromiseThenContract> {
    if !js_like_lang(lang) || method != "then" || arg_count != 1 {
        return None;
    }
    let result = PromiseThenContract {
        receiver: AsyncReceiverContract::ExactPromiseLike,
        demand: promise_then_demand_effect_profile(),
    };
    Some(LibraryPromiseThenContract {
        pack_id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        id: LibraryApiContractId::PromiseThen,
        callee: LibraryApiCalleeContract::AsyncMethod {
            method: "then",
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_promise_resolve_contract(
    lang: Lang,
    receiver: &str,
    method: &str,
    arg_count: usize,
) -> Option<LibraryPromiseFactoryContract> {
    if !js_like_lang(lang) || receiver != "Promise" || method != "resolve" || arg_count != 1 {
        return None;
    }
    let result = PromiseFactoryContract {
        receiver: "Promise",
        method: "resolve",
        qualified_path: "Promise.resolve",
        kind: PromiseFactoryKind::Resolve,
        result_domain: DomainEvidence::PromiseLike,
    };
    Some(LibraryPromiseFactoryContract {
        pack_id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        id: LibraryApiContractId::PromiseFactory(PromiseFactoryKind::Resolve),
        callee: LibraryApiCalleeContract::StaticGlobalMethod {
            receiver: result.receiver,
            method: result.method,
            qualified_path: result.qualified_path,
            requires_unshadowed_receiver: true,
        },
        result,
    })
}

pub fn library_iterator_identity_adapter_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryIteratorIdentityAdapterContract> {
    let method = if lang == Lang::Rust && arg_count == 0 {
        match method {
            "iter" => "iter",
            "into_iter" => "into_iter",
            "iter_mut" => "iter_mut",
            "collect" => "collect",
            "to_vec" => "to_vec",
            "copied" => "copied",
            "cloned" => "cloned",
            _ => return None,
        }
    } else if lang == Lang::Java && method == "stream" && arg_count == 0 {
        "stream"
    } else {
        return None;
    };
    let result = IteratorIdentityAdapterContract {
        receiver: IteratorAdapterReceiverContract::ExactIterableValue,
    };
    Some(LibraryIteratorIdentityAdapterContract {
        pack_id: ITERATOR_IDENTITY_ADAPTER_PACK_ID,
        id: LibraryApiContractId::IteratorIdentityAdapter,
        callee: LibraryApiCalleeContract::IteratorAdapterMethod {
            method,
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_static_collection_adapter_contract(
    lang: Lang,
    receiver: &str,
    method: &str,
    arg_count: usize,
) -> Option<LibraryStaticCollectionAdapterContract> {
    if lang != Lang::Java || receiver != "Arrays" || method != "stream" || arg_count != 1 {
        return None;
    }
    let result = StaticCollectionAdapterContract {
        module: "java.util",
        exported: "Arrays",
    };
    Some(LibraryStaticCollectionAdapterContract {
        pack_id: JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
        id: LibraryApiContractId::StaticCollectionAdapter,
        callee: LibraryApiCalleeContract::JavaUtilStaticMember {
            receiver: result.exported,
            method: "stream",
        },
        result,
    })
}

pub fn library_method_call_contract(
    lang: Lang,
    name: &str,
    arg_count: usize,
) -> Option<LibraryMethodCallContract> {
    let result = method_call_contract_shape(lang, name, arg_count)?;
    let method = library_method_selector_name(name)?;
    Some(LibraryMethodCallContract {
        id: LibraryApiContractId::MethodCall(result.semantic),
        callee: LibraryApiCalleeContract::Method {
            method,
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_map_get_default_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryMethodCallContract> {
    let contract = library_method_call_contract(lang, method, arg_count)?;
    let exact_map_get_default = contract.id
        == LibraryApiContractId::MethodCall(MethodSemanticContract::Builtin(Builtin::GetOrDefault))
        && matches!(
            contract.callee,
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::ExactMap,
                ..
            }
        )
        && matches!(
            contract.result.args,
            MethodBuiltinArgs::MapGetDefault | MethodBuiltinArgs::MapGetDefaultOrZeroArgLambda
        );
    exact_map_get_default.then_some(contract)
}

pub fn library_receiver_method_api_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryReceiverMethodApiContract> {
    library_map_get_contract(lang, method, arg_count)
        .map(|contract| LibraryReceiverMethodApiContract {
            pack_id: contract.pack_id,
            id: contract.id,
            callee: contract.callee,
            rule: MAP_GET_PROTOCOL_PRODUCER_ID,
            result_domain: None,
        })
        .or_else(|| {
            library_map_key_view_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: contract.pack_id,
                    id: contract.id,
                    callee: contract.callee,
                    rule: MAP_KEY_VIEW_PROTOCOL_PRODUCER_ID,
                    result_domain: None,
                }
            })
        })
        .or_else(|| {
            library_iterator_identity_adapter_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: contract.pack_id,
                    id: contract.id,
                    callee: contract.callee,
                    rule: ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID,
                    result_domain: None,
                }
            })
        })
        .or_else(|| {
            library_scalar_integer_method_contract(lang, method, arg_count).map(|contract| {
                let rule = match contract.pack_id {
                    RUST_STDLIB_INTEGER_METHOD_PACK_ID => RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID,
                    JAVA_STDLIB_MATH_PACK_ID => JAVA_STDLIB_MATH_PRODUCER_ID,
                    _ => "library_api_scalar_integer_method",
                };
                LibraryReceiverMethodApiContract {
                    pack_id: contract.pack_id,
                    id: contract.id,
                    callee: contract.callee,
                    rule,
                    result_domain: None,
                }
            })
        })
        .or_else(|| {
            library_rust_option_and_then_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: contract.pack_id,
                    id: contract.id,
                    callee: contract.callee,
                    rule: RUST_STDLIB_OPTION_PRODUCER_ID,
                    result_domain: None,
                }
            })
        })
        .or_else(|| {
            library_promise_then_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: contract.pack_id,
                    id: contract.id,
                    callee: contract.callee,
                    rule: JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
                    result_domain: Some(DomainEvidence::PromiseLike),
                }
            })
        })
        .or_else(|| {
            library_map_get_default_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: MAP_GET_DEFAULT_PROTOCOL_PACK_ID,
                    id: contract.id,
                    callee: contract.callee,
                    rule: MAP_GET_DEFAULT_PROTOCOL_PRODUCER_ID,
                    result_domain: None,
                }
            })
        })
        .or_else(|| {
            library_method_call_contract(lang, method, arg_count).map(|contract| {
                LibraryReceiverMethodApiContract {
                    pack_id: FIRST_PARTY_PACK_ID,
                    id: contract.id,
                    callee: contract.callee,
                    rule: "library_api_method_call",
                    result_domain: None,
                }
            })
        })
}

pub(crate) fn library_method_selector_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "__contains__" => "__contains__",
        "Abs" => "Abs",
        "Contains" => "Contains",
        "HasPrefix" => "HasPrefix",
        "HasSuffix" => "HasSuffix",
        "Max" => "Max",
        "Min" => "Min",
        "Print" => "Print",
        "Printf" => "Printf",
        "Println" => "Println",
        "abs" => "abs",
        "add" => "add",
        "all" => "all",
        "all?" => "all?",
        "allMatch" => "allMatch",
        "any" => "any",
        "any?" => "any?",
        "anyMatch" => "anyMatch",
        "and_then" => "and_then",
        "append" => "append",
        "clamp" => "clamp",
        "collect" => "collect",
        "contains" => "contains",
        "containsKey" => "containsKey",
        "contains_key" => "contains_key",
        "count" => "count",
        "debug" => "debug",
        "empty?" => "empty?",
        "end_with?" => "end_with?",
        "endsWith" => "endsWith",
        "ends_with" => "ends_with",
        "endswith" => "endswith",
        "every" => "every",
        "fetch" => "fetch",
        "filter" => "filter",
        "filter_map" => "filter_map",
        "flatMap" => "flatMap",
        "flat_map" => "flat_map",
        "fold" => "fold",
        "get" => "get",
        "getOrDefault" => "getOrDefault",
        "has" => "has",
        "hasPrefix" => "hasPrefix",
        "hasSuffix" => "hasSuffix",
        "has_key?" => "has_key?",
        "include?" => "include?",
        "includes" => "includes",
        "info" => "info",
        "inject" => "inject",
        "isEmpty" => "isEmpty",
        "is_empty" => "is_empty",
        "is_none" => "is_none",
        "is_some" => "is_some",
        "join" => "join",
        "key?" => "key?",
        "len" => "len",
        "length" => "length",
        "log" => "log",
        "map" => "map",
        "map_or" => "map_or",
        "max" => "max",
        "member?" => "member?",
        "min" => "min",
        "nil?" => "nil?",
        "push" => "push",
        "reduce" => "reduce",
        "select" => "select",
        "size" => "size",
        "some" => "some",
        "start_with?" => "start_with?",
        "startsWith" => "startsWith",
        "starts_with" => "starts_with",
        "startswith" => "startswith",
        "sum" => "sum",
        "unwrap_or" => "unwrap_or",
        "unwrap_or_else" => "unwrap_or_else",
        "zip" => "zip",
        _ => return None,
    })
}

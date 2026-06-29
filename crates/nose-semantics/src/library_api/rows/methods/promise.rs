use super::*;

pub fn library_promise_then_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryPromiseThenContract> {
    if !js_like_lang(lang) || method != "then" || !matches!(arg_count, 1 | 2) {
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

pub fn library_promise_catch_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryPromiseCatchContract> {
    if !js_like_lang(lang) || method != "catch" || arg_count != 1 {
        return None;
    }
    let result = PromiseCatchContract {
        receiver: AsyncReceiverContract::ExactPromiseLike,
        demand: promise_then_demand_effect_profile(),
    };
    Some(LibraryPromiseCatchContract {
        pack_id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        id: LibraryApiContractId::PromiseCatch,
        callee: LibraryApiCalleeContract::AsyncMethod {
            method: "catch",
            receiver: result.receiver,
        },
        result,
    })
}

pub fn library_promise_finally_contract(
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> Option<LibraryPromiseFinallyContract> {
    if !js_like_lang(lang) || method != "finally" || arg_count != 1 {
        return None;
    }
    let result = PromiseFinallyContract {
        receiver: AsyncReceiverContract::ExactPromiseLike,
        demand: promise_then_demand_effect_profile(),
    };
    Some(LibraryPromiseFinallyContract {
        pack_id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        id: LibraryApiContractId::PromiseFinally,
        callee: LibraryApiCalleeContract::AsyncMethod {
            method: "finally",
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
    if !js_like_lang(lang)
        || receiver != "Promise"
        || !matches!(method, "resolve" | "reject")
        || arg_count != 1
    {
        return None;
    }
    let kind = match method {
        "resolve" => PromiseFactoryKind::Resolve,
        "reject" => PromiseFactoryKind::Reject,
        _ => return None,
    };
    let (method_name, qualified_path) = match kind {
        PromiseFactoryKind::Resolve => ("resolve", "Promise.resolve"),
        PromiseFactoryKind::Reject => ("reject", "Promise.reject"),
    };
    let result = PromiseFactoryContract {
        receiver: "Promise",
        method: method_name,
        qualified_path,
        kind,
        result_domain: DomainEvidence::PromiseLike,
    };
    Some(LibraryPromiseFactoryContract {
        pack_id: JS_LIKE_BUILTIN_PROMISE_PACK_ID,
        id: LibraryApiContractId::PromiseFactory(kind),
        callee: LibraryApiCalleeContract::StaticGlobalMethod {
            receiver: result.receiver,
            method: result.method,
            qualified_path: result.qualified_path,
            requires_unshadowed_receiver: true,
        },
        result,
    })
}

pub fn library_imported_promise_factory_contract(
    lang: Lang,
    module: &str,
    exported: &str,
    arg_count: usize,
) -> Option<LibraryImportedPromiseFactoryContract> {
    if !js_like_lang(lang) {
        return None;
    }
    let (module, exported) = js_imported_promise_factory_callee(module, exported)?;
    if !js_imported_promise_factory_arity_supported(exported, arg_count) {
        return None;
    }
    Some(LibraryImportedPromiseFactoryContract {
        pack_id: JS_NODE_TIMERS_PROMISES_PACK_ID,
        id: LibraryApiContractId::JsImportedPromiseFactory,
        callee: LibraryApiCalleeContract::ImportedBinding { module, exported },
        result_domain: DomainEvidence::PromiseLike,
    })
}

pub fn library_imported_promise_factory_contracts(
    lang: Lang,
) -> impl Iterator<Item = LibraryImportedPromiseFactoryContract> {
    [
        ("node:timers/promises", "setTimeout", 0),
        ("node:timers/promises", "setImmediate", 0),
        ("timers/promises", "setTimeout", 0),
        ("timers/promises", "setImmediate", 0),
    ]
    .into_iter()
    .filter_map(move |(module, exported, arg_count)| {
        library_imported_promise_factory_contract(lang, module, exported, arg_count)
    })
}

fn js_imported_promise_factory_callee(
    module: &str,
    exported: &str,
) -> Option<(&'static str, &'static str)> {
    match (module, exported) {
        ("node:timers/promises" | "timers/promises", "setTimeout") => Some((
            if module == "node:timers/promises" {
                "node:timers/promises"
            } else {
                "timers/promises"
            },
            "setTimeout",
        )),
        ("node:timers/promises" | "timers/promises", "setImmediate") => Some((
            if module == "node:timers/promises" {
                "node:timers/promises"
            } else {
                "timers/promises"
            },
            "setImmediate",
        )),
        _ => None,
    }
}

fn js_imported_promise_factory_arity_supported(exported: &str, arg_count: usize) -> bool {
    match exported {
        "setTimeout" => arg_count <= 3,
        "setImmediate" => arg_count <= 2,
        _ => false,
    }
}

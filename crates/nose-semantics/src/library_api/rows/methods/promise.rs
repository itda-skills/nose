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

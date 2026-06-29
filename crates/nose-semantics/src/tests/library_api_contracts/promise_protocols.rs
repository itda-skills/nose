use super::*;

#[test]
fn promise_then_contract_requires_js_like_surface_and_receiver_proof() {
    let expected = Some(PromiseThenContract {
        receiver: AsyncReceiverContract::ExactPromiseLike,
        demand: promise_then_demand_effect_profile(),
    });
    assert_eq!(promise_then_contract(Lang::TypeScript, "then", 1), expected);
    assert_eq!(promise_then_contract(Lang::TypeScript, "then", 2), expected);
    assert_eq!(promise_then_contract(Lang::TypeScript, "then", 3), None);
    assert_eq!(promise_then_contract(Lang::Python, "then", 1), None);
}

#[test]
fn promise_catch_contract_requires_js_like_surface_and_receiver_proof() {
    assert_eq!(
        promise_catch_contract(Lang::TypeScript, "catch", 1),
        Some(PromiseCatchContract {
            receiver: AsyncReceiverContract::ExactPromiseLike,
            demand: promise_then_demand_effect_profile(),
        })
    );
    assert_eq!(promise_catch_contract(Lang::TypeScript, "catch", 2), None);
    assert_eq!(promise_catch_contract(Lang::Python, "catch", 1), None);
}

#[test]
fn promise_finally_contract_requires_js_like_surface_and_receiver_proof() {
    assert_eq!(
        promise_finally_contract(Lang::TypeScript, "finally", 1),
        Some(PromiseFinallyContract {
            receiver: AsyncReceiverContract::ExactPromiseLike,
            demand: promise_then_demand_effect_profile(),
        })
    );
    assert_eq!(
        promise_finally_contract(Lang::TypeScript, "finally", 2),
        None
    );
    assert_eq!(promise_finally_contract(Lang::Python, "finally", 1), None);
}

#[test]
fn promise_factory_contract_requires_js_like_static_global_surface() {
    assert_eq!(
        promise_resolve_contract(Lang::JavaScript, "Promise", "resolve", 1),
        Some(PromiseFactoryContract {
            receiver: "Promise",
            method: "resolve",
            qualified_path: "Promise.resolve",
            kind: PromiseFactoryKind::Resolve,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_resolve_contract(Lang::TypeScript, "Promise", "resolve", 2),
        None
    );
    assert_eq!(
        promise_resolve_contract(Lang::TypeScript, "Promise", "reject", 1),
        Some(PromiseFactoryContract {
            receiver: "Promise",
            method: "reject",
            qualified_path: "Promise.reject",
            kind: PromiseFactoryKind::Reject,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_resolve_contract(Lang::Python, "Promise", "resolve", 1),
        None
    );
}

#[test]
fn promise_aggregate_contract_requires_js_like_static_global_surface() {
    assert_eq!(
        promise_aggregate_contract(Lang::TypeScript, "Promise", "all", 1),
        Some(PromiseAggregateContract {
            receiver: "Promise",
            method: "all",
            qualified_path: "Promise.all",
            kind: PromiseAggregateKind::All,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_aggregate_contract(Lang::TypeScript, "Promise", "allSettled", 1),
        Some(PromiseAggregateContract {
            receiver: "Promise",
            method: "allSettled",
            qualified_path: "Promise.allSettled",
            kind: PromiseAggregateKind::AllSettled,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_aggregate_contract(Lang::TypeScript, "Promise", "race", 1),
        Some(PromiseAggregateContract {
            receiver: "Promise",
            method: "race",
            qualified_path: "Promise.race",
            kind: PromiseAggregateKind::Race,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_aggregate_contract(Lang::TypeScript, "Promise", "any", 1),
        Some(PromiseAggregateContract {
            receiver: "Promise",
            method: "any",
            qualified_path: "Promise.any",
            kind: PromiseAggregateKind::Any,
            result_domain: DomainEvidence::PromiseLike,
        })
    );
    assert_eq!(
        promise_aggregate_contract(Lang::TypeScript, "Promise", "all", 2),
        None
    );
    assert_eq!(
        promise_aggregate_contract(Lang::Python, "Promise", "all", 1),
        None
    );
}

#[test]
fn imported_promise_factory_contract_requires_js_like_import_coordinate() {
    assert_eq!(
        library_imported_promise_factory_contract(
            Lang::TypeScript,
            "node:timers/promises",
            "setTimeout",
            2,
        ),
        Some(LibraryImportedPromiseFactoryContract {
            pack_id: JS_NODE_TIMERS_PROMISES_PACK_ID,
            id: LibraryApiContractId::JsImportedPromiseFactory,
            callee: LibraryApiCalleeContract::ImportedBinding {
                module: "node:timers/promises",
                exported: "setTimeout",
            },
            result_domain: DomainEvidence::PromiseLike,
            fulfilled_payload_arg: Some(1),
        })
    );
    assert_eq!(
        library_imported_promise_factory_contract(
            Lang::JavaScript,
            "timers/promises",
            "setImmediate",
            1,
        ),
        Some(LibraryImportedPromiseFactoryContract {
            pack_id: JS_NODE_TIMERS_PROMISES_PACK_ID,
            id: LibraryApiContractId::JsImportedPromiseFactory,
            callee: LibraryApiCalleeContract::ImportedBinding {
                module: "timers/promises",
                exported: "setImmediate",
            },
            result_domain: DomainEvidence::PromiseLike,
            fulfilled_payload_arg: Some(0),
        })
    );
    assert_eq!(
        library_imported_promise_factory_contract(
            Lang::Python,
            "node:timers/promises",
            "setTimeout",
            1,
        ),
        None
    );
    assert_eq!(
        library_imported_promise_factory_contract(
            Lang::TypeScript,
            "node:timers/promises",
            "scheduler",
            1,
        ),
        None
    );
    assert_eq!(
        library_imported_promise_factory_contract(
            Lang::TypeScript,
            "node:timers/promises",
            "setImmediate",
            3,
        ),
        None
    );
}

use super::*;
use nose_semantics::{
    admitted_promise_aggregate_at_call, admitted_promise_resolve_at_call,
    library_promise_aggregate_contract, library_promise_resolve_contract,
    JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
};

fn promise_static_method_call_il(
    method: &str,
    with_qualified_global: bool,
) -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut builder = IlBuilder::new(FileId(0));
    let receiver = builder.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Promise")),
        sp(10),
        &[],
    );
    let callee = builder.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(11),
        &[receiver],
    );
    let arg = builder.add(NodeKind::Lit, Payload::LitInt(1), sp(12), &[]);
    let call = builder.add(NodeKind::Call, Payload::None, sp(20), &[callee, arg]);
    let root = builder.add(NodeKind::Func, Payload::None, sp(21), &[call]);
    let mut il = builder.finish(
        root,
        FileMeta {
            path: "promise-static".into(),
            lang: Lang::TypeScript,
        },
        Vec::new(),
        Vec::new(),
    );
    let (pack_id, producer_id) = language_core_evidence_provenance(Lang::TypeScript);
    il.find_or_push_builtin_evidence(
        EvidenceAnchor::node(il.node(receiver).span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Promise"),
        }),
        pack_id,
        producer_id,
        Vec::new(),
    );
    if with_qualified_global {
        let root_dependency = il.find_or_push_builtin_evidence(
            EvidenceAnchor::source_span(il.node(callee).span),
            EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
                name_hash: stable_symbol_hash("Promise"),
            }),
            pack_id,
            producer_id,
            Vec::new(),
        );
        il.find_or_push_builtin_evidence(
            EvidenceAnchor::node(il.node(callee).span, NodeKind::Field),
            EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal {
                path_hash: stable_symbol_hash(&format!("Promise.{method}")),
            }),
            pack_id,
            producer_id,
            vec![root_dependency],
        );
    }
    (il, interner, call, receiver, callee)
}

fn assert_promise_static_global_result_domain(
    method: &str,
    pack_id: &'static str,
    admitted_receiver: impl Fn(&Il, &Interner, NodeId) -> Option<Option<NodeId>>,
) {
    let (mut il, interner, call, receiver, _) = promise_static_method_call_il(method, true);

    run(&mut il, &interner);

    let admitted = admitted_receiver(&il, &interner, call)
        .unwrap_or_else(|| panic!("qualified global Promise.{method} should be admitted"));
    assert_eq!(admitted, Some(receiver));
    let api = library_api_records(&il, call)
        .into_iter()
        .find(|record| record.status == EvidenceStatus::Asserted)
        .unwrap_or_else(|| panic!("Promise.{method} API evidence"));
    assert_eq!(
        api.provenance,
        pack_provenance(pack_id, JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID)
    );
    let result_domains = node_domain_records(&il, call, DomainEvidence::PromiseLike);
    assert_eq!(result_domains.len(), 1);
    assert_eq!(
        result_domains[0].provenance,
        language_core_provenance(Lang::TypeScript)
    );
    assert_eq!(result_domains[0].dependencies, vec![api.id]);

    let (mut shadow_closed, interner, call, _, _) = promise_static_method_call_il(method, false);
    run(&mut shadow_closed, &interner);
    assert!(
        admitted_receiver(&shadow_closed, &interner, call).is_none(),
        "Promise.{method} shape without qualified-global proof must stay closed"
    );
    assert!(
        node_domain_records(&shadow_closed, call, DomainEvidence::PromiseLike).is_empty(),
        "PromiseLike result domain requires admitted Promise.{method} API evidence"
    );
}

#[test]
fn promise_resolve_static_global_materializes_result_domain() {
    let contract =
        library_promise_resolve_contract(Lang::TypeScript, "Promise", "resolve", 1).unwrap();
    assert_promise_static_global_result_domain(
        "resolve",
        contract.pack_id,
        |il, interner, call| {
            admitted_promise_resolve_at_call(il, interner, call).map(|admitted| admitted.receiver)
        },
    );
}

#[test]
fn promise_all_static_global_materializes_result_domain() {
    let contract =
        library_promise_aggregate_contract(Lang::TypeScript, "Promise", "all", 1).unwrap();
    assert_promise_static_global_result_domain("all", contract.pack_id, |il, interner, call| {
        admitted_promise_aggregate_at_call(il, interner, call).map(|admitted| admitted.receiver)
    });
}

#[test]
fn promise_all_settled_static_global_materializes_result_domain() {
    let contract =
        library_promise_aggregate_contract(Lang::TypeScript, "Promise", "allSettled", 1).unwrap();
    assert_promise_static_global_result_domain(
        "allSettled",
        contract.pack_id,
        |il, interner, call| {
            admitted_promise_aggregate_at_call(il, interner, call).map(|admitted| admitted.receiver)
        },
    );
}

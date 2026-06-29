use super::*;
use nose_semantics::{
    admitted_promise_aggregate_at_call, admitted_promise_resolve_at_call,
    library_promise_aggregate_contract, library_promise_resolve_contract,
    JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
};

fn promise_resolve_call_il(with_qualified_global: bool) -> (Il, Interner, NodeId, NodeId, NodeId) {
    promise_static_method_call_il("resolve", with_qualified_global)
}

fn promise_all_call_il(with_qualified_global: bool) -> (Il, Interner, NodeId, NodeId, NodeId) {
    promise_static_method_call_il("all", with_qualified_global)
}

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

#[test]
fn promise_resolve_static_global_materializes_result_domain() {
    let (mut il, interner, call, receiver, _) = promise_resolve_call_il(true);
    let contract =
        library_promise_resolve_contract(Lang::TypeScript, "Promise", "resolve", 1).unwrap();

    run(&mut il, &interner);

    let admitted = admitted_promise_resolve_at_call(&il, &interner, call).expect(
        "qualified global Promise.resolve should be admitted after normalize evidence pass",
    );
    assert_eq!(admitted.receiver, Some(receiver));
    let api = library_api_records(&il, call)
        .into_iter()
        .find(|record| record.status == EvidenceStatus::Asserted)
        .expect("Promise.resolve API evidence");
    assert_eq!(
        api.provenance,
        pack_provenance(contract.pack_id, JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID)
    );
    let result_domains = node_domain_records(&il, call, DomainEvidence::PromiseLike);
    assert_eq!(result_domains.len(), 1);
    assert_eq!(
        result_domains[0].provenance,
        language_core_provenance(Lang::TypeScript)
    );
    assert_eq!(result_domains[0].dependencies, vec![api.id]);

    let (mut shadow_closed, interner, call, _, _) = promise_resolve_call_il(false);
    run(&mut shadow_closed, &interner);
    assert!(
        admitted_promise_resolve_at_call(&shadow_closed, &interner, call).is_none(),
        "Promise.resolve shape without qualified-global proof must stay closed"
    );
    assert!(
        node_domain_records(&shadow_closed, call, DomainEvidence::PromiseLike).is_empty(),
        "PromiseLike result domain requires admitted Promise.resolve API evidence"
    );
}

#[test]
fn promise_all_static_global_materializes_result_domain() {
    let (mut il, interner, call, receiver, _) = promise_all_call_il(true);
    let contract =
        library_promise_aggregate_contract(Lang::TypeScript, "Promise", "all", 1).unwrap();

    run(&mut il, &interner);

    let admitted = admitted_promise_aggregate_at_call(&il, &interner, call)
        .expect("qualified global Promise.all should be admitted after normalize evidence pass");
    assert_eq!(admitted.receiver, Some(receiver));
    let api = library_api_records(&il, call)
        .into_iter()
        .find(|record| record.status == EvidenceStatus::Asserted)
        .expect("Promise.all API evidence");
    assert_eq!(
        api.provenance,
        pack_provenance(contract.pack_id, JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID)
    );
    let result_domains = node_domain_records(&il, call, DomainEvidence::PromiseLike);
    assert_eq!(result_domains.len(), 1);
    assert_eq!(
        result_domains[0].provenance,
        language_core_provenance(Lang::TypeScript)
    );
    assert_eq!(result_domains[0].dependencies, vec![api.id]);

    let (mut shadow_closed, interner, call, _, _) = promise_all_call_il(false);
    run(&mut shadow_closed, &interner);
    assert!(
        admitted_promise_aggregate_at_call(&shadow_closed, &interner, call).is_none(),
        "Promise.all shape without qualified-global proof must stay closed"
    );
    assert!(
        node_domain_records(&shadow_closed, call, DomainEvidence::PromiseLike).is_empty(),
        "PromiseLike result domain requires admitted Promise.all API evidence"
    );
}

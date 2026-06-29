use super::support::*;

#[test]
fn node_timers_promise_like_factory_without_settlement_stays_opaque() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let delay = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("delay")),
        sp(300),
        &[],
    );
    let timeout = b.add(NodeKind::Lit, Payload::LitInt(0), sp(301), &[]);
    let payload = b.add(NodeKind::Lit, Payload::LitInt(1), sp(302), &[]);
    let producer_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(303),
        &[delay, timeout, payload],
    );
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(304),
        &[producer_call],
    );
    let callback = add_increment_lambda(&mut b, 305, 1);
    let continuation_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(310),
        &[then_callee, callback],
    );
    let mut il = finish_test_il(b, continuation_call, Lang::TypeScript);
    il.evidence.push(language_core_symbol_evidence(
        100,
        Lang::TypeScript,
        EvidenceAnchor::binding(sp(299), stable_symbol_hash("delay")),
        SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("node:timers/promises"),
            exported_hash: stable_symbol_hash("setTimeout"),
        },
    ));

    crate::library_api_evidence::run(&mut il, &interner);

    assert_eq!(
        nose_semantics::domain_evidence_for_receiver(&il, &interner, producer_call),
        Some(DomainEvidence::PromiseLike),
        "node:timers/promises setTimeout should prove only the PromiseLike receiver domain"
    );
    assert!(
        nose_semantics::admitted_promise_then_at_call(&il, &interner, continuation_call).is_some(),
        "Promise.then should admit after the imported producer domain is materialized"
    );
    assert!(
        !matches!(
            eval_op(&il, &interner, continuation_call),
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE || code == PROMISE_REJECTED_CODE
        ),
        "timers/promises producers need scheduling/settlement proof before payload recovery"
    );
}

#[test]
fn node_timers_commonjs_imported_binding_dependency_opens_promise_like_domain() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let delay = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("delay")),
        sp(300),
        &[],
    );
    let timeout = b.add(NodeKind::Lit, Payload::LitInt(0), sp(301), &[]);
    let producer_call = b.add(NodeKind::Call, Payload::None, sp(303), &[delay, timeout]);
    let mut il = finish_test_il(b, producer_call, Lang::TypeScript);
    il.evidence.push(language_core_symbol_evidence(
        100,
        Lang::TypeScript,
        EvidenceAnchor::node(sp(298), NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("require"),
        },
    ));
    il.evidence
        .push(language_core_symbol_evidence_with_dependencies(
            101,
            Lang::TypeScript,
            EvidenceAnchor::binding(sp(299), stable_symbol_hash("delay")),
            SymbolEvidenceKind::ImportedBinding {
                module_hash: stable_symbol_hash("timers/promises"),
                exported_hash: stable_symbol_hash("setTimeout"),
            },
            vec![EvidenceId(100)],
        ));

    crate::library_api_evidence::run(&mut il, &interner);

    assert_eq!(
        nose_semantics::domain_evidence_for_receiver(&il, &interner, producer_call),
        Some(DomainEvidence::PromiseLike),
        "dependency-closed CJS timers/promises destructuring should prove the PromiseLike domain"
    );
}

#[test]
fn node_timers_commonjs_imported_binding_requires_asserted_dependency() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let delay = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("delay")),
        sp(300),
        &[],
    );
    let timeout = b.add(NodeKind::Lit, Payload::LitInt(0), sp(301), &[]);
    let producer_call = b.add(NodeKind::Call, Payload::None, sp(303), &[delay, timeout]);
    let mut il = finish_test_il(b, producer_call, Lang::TypeScript);
    let mut require = language_core_symbol_evidence(
        100,
        Lang::TypeScript,
        EvidenceAnchor::node(sp(298), NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("require"),
        },
    );
    require.status = EvidenceStatus::Ambiguous;
    il.evidence.push(require);
    il.evidence
        .push(language_core_symbol_evidence_with_dependencies(
            101,
            Lang::TypeScript,
            EvidenceAnchor::binding(sp(299), stable_symbol_hash("delay")),
            SymbolEvidenceKind::ImportedBinding {
                module_hash: stable_symbol_hash("timers/promises"),
                exported_hash: stable_symbol_hash("setTimeout"),
            },
            vec![EvidenceId(100)],
        ));

    crate::library_api_evidence::run(&mut il, &interner);

    assert_eq!(
        nose_semantics::domain_evidence_for_receiver(&il, &interner, producer_call),
        None,
        "CJS timers/promises import proof must stay closed when require proof is ambiguous"
    );
}

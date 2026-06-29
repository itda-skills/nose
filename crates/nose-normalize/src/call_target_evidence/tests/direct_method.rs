use super::support::*;

#[test]
fn emits_promise_like_domain_for_direct_method_returning_promise_like() {
    let DirectMethodReturnFixture {
        interner,
        mut il,
        target,
        call,
        return_expr,
        method,
    } = direct_method_return_call_fixture();
    let target_evidence_id = EvidenceId(400);
    let return_domain_id = EvidenceId(500);
    il.evidence.push(EvidenceRecord {
        id: target_evidence_id,
        anchor: EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        kind: EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectMethod {
            target_span: il.node(target).span,
            receiver_type_hash: stable_symbol_hash("Worker"),
            method_hash: interner.symbol_hash(method),
        }),
        provenance: language_core_provenance(Lang::TypeScript),
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    });
    il.evidence.push(EvidenceRecord {
        id: return_domain_id,
        anchor: EvidenceAnchor::node(il.node(return_expr).span, NodeKind::Call),
        kind: EvidenceKind::Domain(DomainEvidence::PromiseLike),
        provenance: language_core_provenance(Lang::TypeScript),
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    });

    run(&mut il, &interner);

    let domain = il
        .evidence
        .iter()
        .find(|record| {
            record.anchor == EvidenceAnchor::node(il.node(call).span, NodeKind::Call)
                && record.kind == EvidenceKind::Domain(DomainEvidence::PromiseLike)
        })
        .expect("PromiseLike domain evidence for direct method call result");
    assert_eq!(
        domain.dependencies,
        vec![target_evidence_id, return_domain_id]
    );
}

#[test]
fn direct_method_return_domain_requires_return_expression_domain_proof() {
    let DirectMethodReturnFixture {
        interner,
        mut il,
        target,
        call,
        method,
        ..
    } = direct_method_return_call_fixture();
    il.evidence.push(EvidenceRecord {
        id: EvidenceId(400),
        anchor: EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        kind: EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectMethod {
            target_span: il.node(target).span,
            receiver_type_hash: stable_symbol_hash("Worker"),
            method_hash: interner.symbol_hash(method),
        }),
        provenance: language_core_provenance(Lang::TypeScript),
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    });

    run(&mut il, &interner);

    assert!(
        il.evidence
            .iter()
            .find(|record| {
                record.anchor == EvidenceAnchor::node(il.node(call).span, NodeKind::Call)
                    && record.kind == EvidenceKind::Domain(DomainEvidence::PromiseLike)
            })
            .is_none(),
        "direct method result domain requires returned expression domain evidence"
    );
}

struct DirectMethodReturnFixture {
    interner: Interner,
    il: Il,
    target: NodeId,
    call: NodeId,
    return_expr: NodeId,
    method: Symbol,
}

fn direct_method_return_call_fixture() -> DirectMethodReturnFixture {
    let interner = Interner::new();
    let method = interner.intern("load");
    let worker = interner.intern("worker");
    let mut b = IlBuilder::new(FileId(0));
    let (target, return_expr) = promise_returning_method(&mut b, &interner);
    let receiver = b.add(NodeKind::Var, Payload::Name(worker), sp(10), &[]);
    let callee = b.add(NodeKind::Field, Payload::Name(method), sp(11), &[receiver]);
    let call = b.add(NodeKind::Call, Payload::None, sp(12), &[callee]);
    let module = b.add(NodeKind::Module, Payload::None, sp(13), &[target, call]);
    DirectMethodReturnFixture {
        interner,
        il: finish_ts_method_fixture(b, module, target, method),
        target,
        call,
        return_expr,
        method,
    }
}

fn promise_returning_method(b: &mut IlBuilder, interner: &Interner) -> (NodeId, NodeId) {
    let promise_callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("producer")),
        sp(1),
        &[],
    );
    let return_expr = b.add(NodeKind::Call, Payload::None, sp(2), &[promise_callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(3), &[return_expr]);
    (
        b.add(NodeKind::Func, Payload::None, sp(4), &[ret]),
        return_expr,
    )
}

fn finish_ts_method_fixture(
    builder: IlBuilder,
    root: NodeId,
    method_root: NodeId,
    method: Symbol,
) -> Il {
    builder.finish(
        root,
        FileMeta {
            path: "t".into(),
            lang: Lang::TypeScript,
        },
        vec![Unit {
            root: method_root,
            kind: UnitKind::Method,
            name: Some(method),
            origin: Default::default(),
        }],
        Vec::new(),
    )
}

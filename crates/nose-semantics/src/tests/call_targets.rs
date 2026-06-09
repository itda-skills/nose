use super::*;

fn call_target_record(
    id: u32,
    span: Span,
    target: CallTargetEvidenceKind,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::node(span, NodeKind::Call),
        EvidenceKind::CallTarget(target),
        status,
        dependencies.iter().copied().map(EvidenceId).collect(),
    )
}

fn imported_function_call_il(interner: &Interner) -> (Il, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let local = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("prod")),
        sp(10),
        &[],
    );
    let arg = b.add(NodeKind::Lit, Payload::LitInt(3), sp(11), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(12), &[local, arg]);
    (finish_il(b, call, Lang::Python), call)
}

fn field_call_il(interner: &Interner, method: &str) -> (Il, NodeId, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("ns")),
        sp(20),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(21),
        &[receiver],
    );
    let arg = b.add(NodeKind::Lit, Payload::LitInt(4), sp(22), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(23), &[callee, arg]);
    (finish_il(b, call, Lang::Python), call, callee)
}

#[test]
fn imported_function_call_target_requires_matching_local_selector() {
    let interner = Interner::new();
    let (mut il, call) = imported_function_call_il(&interner);
    il.evidence.push(call_target_record(
        0,
        sp(12),
        CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: interner.symbol_hash(interner.intern("prod")),
        },
        EvidenceStatus::Asserted,
        &[],
    ));

    assert_eq!(
        call_target_evidence_at_call(&il, &interner, call),
        Some(CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: interner.symbol_hash(interner.intern("prod")),
        })
    );
    assert!(imported_function_call_target_at_call(&il, &interner, call));
}

#[test]
fn wrong_imported_function_selector_is_rejected_not_missing() {
    let interner = Interner::new();
    let (mut il, call) = imported_function_call_il(&interner);
    il.evidence.push(call_target_record(
        0,
        sp(12),
        CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: stable_symbol_hash("sum"),
        },
        EvidenceStatus::Asserted,
        &[],
    ));

    assert_eq!(
        call_target_evidence_status_at_call(&il, &interner, call),
        CallTargetEvidenceStatus::Rejected
    );
    assert_eq!(call_target_evidence_at_call(&il, &interner, call), None);
}

#[test]
fn dependency_broken_call_target_is_rejected() {
    let interner = Interner::new();
    let (mut il, call) = imported_function_call_il(&interner);
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(10), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
        }),
        EvidenceStatus::Ambiguous,
    ));
    il.evidence.push(call_target_record(
        1,
        sp(12),
        CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: interner.symbol_hash(interner.intern("prod")),
        },
        EvidenceStatus::Asserted,
        &[0],
    ));

    assert_eq!(
        call_target_evidence_status_at_call(&il, &interner, call),
        CallTargetEvidenceStatus::Rejected
    );
}

#[test]
fn conflicting_call_targets_stay_closed() {
    let interner = Interner::new();
    let (mut il, call) = imported_function_call_il(&interner);
    il.evidence.push(call_target_record(
        0,
        sp(12),
        CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: interner.symbol_hash(interner.intern("prod")),
        },
        EvidenceStatus::Asserted,
        &[],
    ));
    il.evidence.push(call_target_record(
        1,
        sp(12),
        CallTargetEvidenceKind::ImportedFunction {
            module_hash: stable_symbol_hash("statistics"),
            exported_hash: stable_symbol_hash("prod"),
            local_hash: interner.symbol_hash(interner.intern("prod")),
        },
        EvidenceStatus::Asserted,
        &[],
    ));

    assert_eq!(
        call_target_evidence_status_at_call(&il, &interner, call),
        CallTargetEvidenceStatus::Rejected
    );
}

#[test]
fn direct_method_target_requires_matching_selector_and_target_span() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let target_body = b.add(NodeKind::Block, Payload::None, sp(30), &[]);
    let target = b.add(NodeKind::Func, Payload::None, sp(31), &[target_body]);
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("worker")),
        sp(32),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("run")),
        sp(33),
        &[receiver],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(34), &[callee]);
    let root = b.add(NodeKind::Module, Payload::None, sp(35), &[target, call]);
    let mut il = finish_il(b, root, Lang::TypeScript);
    il.evidence.push(call_target_record(
        0,
        sp(34),
        CallTargetEvidenceKind::DirectMethod {
            target_span: il.node(target).span,
            receiver_type_hash: stable_symbol_hash("Worker"),
            method_hash: interner.symbol_hash(interner.intern("run")),
        },
        EvidenceStatus::Asserted,
        &[],
    ));

    assert!(direct_method_call_target_at_call(
        &il, &interner, call, target
    ));
}

#[test]
fn dynamic_dispatch_is_evidence_but_not_imported_member_identity() {
    let interner = Interner::new();
    let (mut il, call, _) = field_call_il(&interner, "next");
    il.evidence.push(call_target_record(
        0,
        sp(23),
        CallTargetEvidenceKind::DynamicDispatch {
            protocol_hash: stable_symbol_hash("Iterator"),
            method_hash: interner.symbol_hash(interner.intern("next")),
        },
        EvidenceStatus::Asserted,
        &[],
    ));

    assert!(matches!(
        call_target_evidence_at_call(&il, &interner, call),
        Some(CallTargetEvidenceKind::DynamicDispatch { .. })
    ));
    assert!(!imported_member_call_target_at_call(&il, &interner, call));
}

use super::{builtin_call_node, push_unique, rust_macro_invocation_call, visit_subtree};
use nose_il::{Interner, NodeId};

pub(super) fn callee_identity_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    visit_subtree(il, root, |node| {
        if il.kind(node) == nose_il::NodeKind::Call
            && !builtin_call_node(il, node)
            && !rust_macro_invocation_call(il, node)
        {
            push_unique(
                &mut labels,
                callee_identity_call_evidence(il, interner, node),
            );
        }
    });
    if labels.is_empty() {
        labels.push("import-or-call-target-proof");
    }
    labels
}

pub(super) fn callee_identity_call_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
) -> &'static str {
    match nose_semantics::call_target_evidence_status_at_call(il, interner, call) {
        nose_semantics::CallTargetEvidenceStatus::Rejected => "call-target-evidence-rejected",
        nose_semantics::CallTargetEvidenceStatus::Admitted(target) => match target {
            nose_il::CallTargetEvidenceKind::DirectFunction { .. } => {
                "direct-function-target-present-call-contract-proof"
            }
            nose_il::CallTargetEvidenceKind::DirectMethod { .. } => {
                "direct-method-target-present-call-contract-proof"
            }
            nose_il::CallTargetEvidenceKind::ImportedFunction { .. } => {
                "imported-function-target-present-call-contract-proof"
            }
            nose_il::CallTargetEvidenceKind::ImportedMember { .. } => {
                "imported-member-target-present-call-contract-proof"
            }
            nose_il::CallTargetEvidenceKind::DynamicDispatch { .. } => {
                "dynamic-dispatch-target-present-concrete-target-proof"
            }
        },
        nose_semantics::CallTargetEvidenceStatus::Missing => il
            .children(call)
            .first()
            .map_or("unknown-call-target-proof", |&callee| {
                missing_call_target_evidence(il, interner, callee)
            }),
    }
}

fn missing_call_target_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
) -> &'static str {
    match il.kind(callee) {
        nose_il::NodeKind::Var => var_call_target_evidence(il, interner, callee),
        nose_il::NodeKind::Field => field_call_target_evidence(il, interner, callee),
        _ => "unknown-call-target-proof",
    }
}

fn var_call_target_evidence(il: &nose_il::Il, interner: &Interner, callee: NodeId) -> &'static str {
    if var_name_contains_scope(il, interner, callee) {
        return "scoped-path-call-target-proof";
    }
    symbol_call_target_evidence(il, callee).unwrap_or("local-or-parameter-call-target-proof")
}

fn field_call_target_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
) -> &'static str {
    if let Some(label) = symbol_call_target_evidence(il, callee) {
        return label;
    }
    if let Some(&receiver) = il.children(callee).first() {
        if receiver_imported_member_evidence(il, receiver) {
            return "imported-member-call-target-proof";
        }
        if receiver_contains_scoped_path(il, interner, receiver) {
            return "scoped-path-call-target-proof";
        }
    }
    "member-call-target-proof"
}

fn receiver_imported_member_evidence(il: &nose_il::Il, receiver: NodeId) -> bool {
    il.evidence_anchored_at(il.node(receiver).span)
        .any(|record| {
            matches!(
                record.kind,
                nose_il::EvidenceKind::Symbol(
                    nose_il::SymbolEvidenceKind::ImportedBinding { .. }
                        | nose_il::SymbolEvidenceKind::ImportedNamespace { .. }
                )
            )
        })
}

fn receiver_contains_scoped_path(il: &nose_il::Il, interner: &Interner, node: NodeId) -> bool {
    match il.kind(node) {
        nose_il::NodeKind::Var => var_name_contains_scope(il, interner, node),
        nose_il::NodeKind::Field => il
            .children(node)
            .first()
            .is_some_and(|&child| receiver_contains_scoped_path(il, interner, child)),
        _ => false,
    }
}

fn var_name_contains_scope(il: &nose_il::Il, interner: &Interner, node: NodeId) -> bool {
    matches!(
        il.node(node).payload,
        nose_il::Payload::Name(name) if interner.resolve(name).contains("::")
    )
}

fn symbol_call_target_evidence(il: &nose_il::Il, node: NodeId) -> Option<&'static str> {
    il.evidence_anchored_at(il.node(node).span)
        .find_map(|record| match record.kind {
            nose_il::EvidenceKind::Symbol(nose_il::SymbolEvidenceKind::ImportedBinding {
                ..
            }) => Some("imported-binding-call-target-proof"),
            nose_il::EvidenceKind::Symbol(nose_il::SymbolEvidenceKind::ImportedNamespace {
                ..
            }) => Some("imported-member-call-target-proof"),
            nose_il::EvidenceKind::Symbol(nose_il::SymbolEvidenceKind::QualifiedGlobal {
                ..
            }) => Some("qualified-global-call-target-proof"),
            nose_il::EvidenceKind::Symbol(nose_il::SymbolEvidenceKind::UnshadowedGlobal {
                ..
            }) => Some("unshadowed-global-call-target-proof"),
            _ => None,
        })
}

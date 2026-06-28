use super::{
    builtin_call_node, callee_identity::callee_identity_call_evidence, push_unique,
    rust_macro_invocation_call, visit_subtree,
};
use nose_il::{HoFKind, Interner, NodeId};

pub(super) fn hof_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> Vec<&'static str> {
    let mut labels = vec!["hof-demand-effect-profile"];
    visit_subtree(il, root, |node| {
        if let nose_il::Payload::HoF(kind) = il.node(node).payload {
            push_unique(&mut labels, hof_kind_demand_effect_evidence(kind));
            if nose_semantics::admitted_hof_demand_effect_profile_at_node_with_interner(
                il,
                Some(interner),
                node,
                kind,
            )
            .is_none()
            {
                if nose_semantics::source_comprehension_at_node(il, node).is_some() {
                    push_unique(
                        &mut labels,
                        "hof-source-comprehension-demand-effect-profile",
                    );
                } else if nose_semantics::admitted_hof_api_at_node_with_interner(
                    il,
                    Some(interner),
                    node,
                    kind,
                ) {
                    push_unique(&mut labels, "hof-library-demand-effect-profile");
                } else {
                    push_unique(&mut labels, "hof-source-or-library-api-occurrence-proof");
                }
            }
            let children = il.children(node);
            match children.get(1).copied() {
                None => push_unique(&mut labels, "hof-callback-arity-shape-proof"),
                Some(callback) => {
                    if !matches!(
                        il.kind(callback),
                        nose_il::NodeKind::Func | nose_il::NodeKind::Lambda
                    ) {
                        push_unique(&mut labels, "hof-callback-identity-proof");
                    }
                    push_callback_effect_evidence_labels(il, interner, callback, &mut labels);
                }
            }
        }
    });
    labels
}

fn hof_kind_demand_effect_evidence(kind: HoFKind) -> &'static str {
    match kind {
        HoFKind::Map => "hof-map-callback-demand-effect-profile",
        HoFKind::Filter => "hof-filter-callback-demand-effect-profile",
        HoFKind::Reduce => "hof-reduce-callback-demand-effect-profile",
        HoFKind::FlatMap => "hof-flat-map-callback-demand-effect-profile",
        HoFKind::FilterMap => "hof-filter-map-callback-demand-effect-profile",
        HoFKind::Reject => "hof-reject-callback-demand-effect-profile",
    }
}

fn push_callback_effect_evidence_labels(
    il: &nose_il::Il,
    interner: &Interner,
    callback: NodeId,
    labels: &mut Vec<&'static str>,
) {
    visit_subtree(il, callback, |node| match il.kind(node) {
        nose_il::NodeKind::Call => {
            push_unique(labels, "hof-callback-effect-proof");
            push_unique(labels, "hof-callback-call-effect-proof");
            push_callback_call_effect_evidence_labels(il, interner, node, labels);
        }
        nose_il::NodeKind::Assign => {
            push_unique(labels, "hof-callback-effect-proof");
            push_unique(labels, "hof-callback-assignment-effect-proof");
        }
        nose_il::NodeKind::Throw | nose_il::NodeKind::Try | nose_il::NodeKind::Raw => {
            push_unique(labels, "hof-callback-effect-proof");
            push_unique(labels, "hof-callback-runtime-boundary-proof");
        }
        _ => {}
    });
}

fn push_callback_call_effect_evidence_labels(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    labels: &mut Vec<&'static str>,
) {
    if builtin_call_node(il, call) {
        push_unique(labels, "hof-callback-builtin-call-effect-proof");
        return;
    }
    if rust_macro_invocation_call(il, call) {
        push_unique(labels, "hof-callback-rust-macro-call-effect-proof");
        return;
    }
    push_unique(
        labels,
        callback_call_effect_evidence(callee_identity_call_evidence(il, interner, call)),
    );
}

fn callback_call_effect_evidence(call_target_label: &'static str) -> &'static str {
    match call_target_label {
        "call-target-evidence-rejected" => "hof-callback-rejected-call-target-effect-proof",
        "direct-function-target-present-call-contract-proof" => {
            "hof-callback-direct-function-call-effect-proof"
        }
        "direct-method-target-present-call-contract-proof" => {
            "hof-callback-direct-method-call-effect-proof"
        }
        "imported-function-target-present-call-contract-proof" => {
            "hof-callback-imported-function-call-effect-proof"
        }
        "imported-member-target-present-call-contract-proof" => {
            "hof-callback-imported-member-call-effect-proof"
        }
        "dynamic-dispatch-target-present-concrete-target-proof" => {
            "hof-callback-dynamic-dispatch-call-effect-proof"
        }
        "scoped-path-call-target-proof" => "hof-callback-scoped-path-call-effect-proof",
        "imported-binding-call-target-proof" => "hof-callback-imported-binding-call-effect-proof",
        "imported-member-call-target-proof" => "hof-callback-imported-member-call-effect-proof",
        "qualified-global-call-target-proof" => "hof-callback-qualified-global-call-effect-proof",
        "unshadowed-global-call-target-proof" => "hof-callback-unshadowed-global-call-effect-proof",
        "member-call-target-proof" => "hof-callback-member-call-effect-proof",
        "local-or-parameter-call-target-proof" => {
            "hof-callback-local-or-parameter-call-effect-proof"
        }
        _ => "hof-callback-unknown-call-effect-proof",
    }
}

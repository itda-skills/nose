use nose_il::{HoFKind, Interner, NodeId};

#[derive(Clone)]
pub(crate) struct ExactAdmissionRejectionDiagnostic {
    pub(crate) reason: &'static str,
    pub(crate) admission_gate: &'static str,
    pub(crate) capability_id: &'static str,
    pub(crate) pack_id: Option<&'static str>,
    pub(crate) missing_evidence: Vec<&'static str>,
}

pub(crate) fn exact_admission_rejection(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
    exact_safe: bool,
    value_len: usize,
) -> Option<ExactAdmissionRejectionDiagnostic> {
    if exact_safe {
        return (!nose_detect::exact_claim_eligible_parts(true, value_len)).then(|| {
            ExactAdmissionRejectionDiagnostic {
                reason: "value-fingerprint-too-small",
                admission_gate: "exact-claim-value-fingerprint-floor",
                capability_id: "non-degenerate-value-fingerprint",
                pack_id: None,
                missing_evidence: vec!["non-degenerate-value-fingerprint"],
            }
        });
    }

    Some(strict_exact_rejection_reason(il, interner, root))
}

fn strict_exact_rejection_reason(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> ExactAdmissionRejectionDiagnostic {
    if let Some(missing_evidence) = runtime_boundary_missing_evidence(il, interner, root) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "unsupported-runtime-boundary",
            admission_gate: "strict-exact-safety",
            capability_id: "runtime-boundary-model",
            pack_id: None,
            missing_evidence,
        };
    }

    if subtree_has(il, root, |il, node| il.kind(node) == nose_il::NodeKind::HoF) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "hof-demand-effect-proof-missing",
            admission_gate: "strict-exact-hof-demand-effect",
            capability_id: "hof-demand-effect-materialization",
            pack_id: None,
            missing_evidence: hof_missing_evidence(il, interner, root),
        };
    }

    if subtree_has(il, root, effect_boundary_node) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "mutation-effect-boundary",
            admission_gate: "strict-exact-effect-safety",
            capability_id: "effect-and-place-contract",
            pack_id: None,
            missing_evidence: vec!["effect-preserving-contract"],
        };
    }

    if subtree_has(il, root, builtin_call_node) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "library-api-occurrence-proof-missing",
            admission_gate: "strict-exact-library-api-occurrence",
            capability_id: "library-api-occurrence",
            pack_id: None,
            missing_evidence: vec!["library-api-occurrence-evidence"],
        };
    }

    if subtree_has(il, root, |il, node| {
        receiver_method_call(il, interner, node)
    }) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "receiver-domain-proof-missing",
            admission_gate: "strict-exact-receiver-domain",
            capability_id: "receiver-domain-evidence",
            pack_id: None,
            missing_evidence: vec!["receiver-domain-proof"],
        };
    }

    if subtree_has(il, root, rust_macro_invocation_call) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "source-surface-proof-missing",
            admission_gate: "strict-exact-source-surface",
            capability_id: "source-surface-evidence",
            pack_id: None,
            missing_evidence: vec!["rust-macro-expansion-contract"],
        };
    }

    if subtree_has(il, root, |il, node| {
        il.kind(node) == nose_il::NodeKind::Call
    }) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "import-symbol-callee-identity-proof-missing",
            admission_gate: "strict-exact-callee-identity",
            capability_id: "callee-identity-evidence",
            pack_id: None,
            missing_evidence: callee_identity_missing_evidence(il, interner, root),
        };
    }

    if subtree_has(il, root, source_surface_boundary_node) {
        return ExactAdmissionRejectionDiagnostic {
            reason: "source-surface-proof-missing",
            admission_gate: "strict-exact-source-surface",
            capability_id: "source-surface-evidence",
            pack_id: None,
            missing_evidence: vec!["source-surface-contract"],
        };
    }

    ExactAdmissionRejectionDiagnostic {
        reason: "unattributed-strict-exact-unsafe",
        admission_gate: "strict-exact-safety",
        capability_id: "exact-semantic-merge",
        pack_id: None,
        missing_evidence: vec!["strict-exact-safe-tree"],
    }
}

fn subtree_has(
    il: &nose_il::Il,
    root: NodeId,
    pred: impl Fn(&nose_il::Il, NodeId) -> bool,
) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if pred(il, node) {
            return true;
        }
        stack.extend(il.children(node).iter().copied());
    }
    false
}

fn callee_identity_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if il.kind(node) == nose_il::NodeKind::Call
            && !builtin_call_node(il, node)
            && !rust_macro_invocation_call(il, node)
        {
            push_unique(
                &mut labels,
                callee_identity_call_evidence(il, interner, node),
            );
        }
        stack.extend(il.children(node).iter().copied());
    }
    if labels.is_empty() {
        labels.push("import-or-call-target-proof");
    }
    labels
}

fn push_unique(labels: &mut Vec<&'static str>, label: &'static str) {
    if !labels.contains(&label) {
        labels.push(label);
    }
}

fn runtime_boundary_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> Option<Vec<&'static str>> {
    let mut labels = vec!["lowered-runtime-boundary-contract"];
    let mut found = false;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if push_runtime_node_missing_evidence(il, node, &mut labels) {
            found = true;
        }
        if il.kind(node) == nose_il::NodeKind::Call {
            found |= push_promise_protocol_call_missing_evidence(il, interner, node, &mut labels);
        }
        stack.extend(il.children(node).iter().copied());
    }
    found.then_some(labels)
}

fn push_runtime_node_missing_evidence(
    il: &nose_il::Il,
    node: NodeId,
    labels: &mut Vec<&'static str>,
) -> bool {
    match il.kind(node) {
        nose_il::NodeKind::Raw => {
            match nose_semantics::source_protocol_at_node(il, node) {
                Some(nose_il::SourceProtocolKind::Await) => {
                    push_unique(labels, "promise-await-scheduling-contract");
                }
                Some(nose_il::SourceProtocolKind::AsyncFunction) => {
                    push_unique(labels, "promise-async-function-scheduling-contract");
                }
                Some(nose_il::SourceProtocolKind::AsyncBlock) => {
                    push_unique(labels, "future-async-block-scheduling-contract");
                }
                Some(nose_il::SourceProtocolKind::Yield) => {
                    push_unique(labels, "generator-yield-protocol-contract");
                }
                Some(
                    nose_il::SourceProtocolKind::ChannelReceive
                    | nose_il::SourceProtocolKind::ChannelSelect
                    | nose_il::SourceProtocolKind::ChannelSelectCase
                    | nose_il::SourceProtocolKind::ChannelSelectDefault
                    | nose_il::SourceProtocolKind::ChannelSend,
                ) => {
                    push_unique(labels, "channel-protocol-contract");
                }
                Some(
                    nose_il::SourceProtocolKind::Defer | nose_il::SourceProtocolKind::GoRoutine,
                ) => {
                    push_unique(labels, "concurrency-scheduling-contract");
                }
                Some(nose_il::SourceProtocolKind::TryPropagation) => {
                    push_unique(labels, "exception-channel-contract");
                }
                None => {}
            }
            true
        }
        nose_il::NodeKind::Try | nose_il::NodeKind::Throw => {
            push_unique(labels, "exception-channel-contract");
            true
        }
        nose_il::NodeKind::Splat | nose_il::NodeKind::KwArg => {
            push_unique(labels, "runtime-call-shape-contract");
            true
        }
        _ => false,
    }
}

fn push_promise_protocol_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    labels: &mut Vec<&'static str>,
) -> bool {
    if !js_like_runtime_lang(il.meta.lang) {
        return false;
    }
    let Some(callee) = il.children(call).first().copied() else {
        return false;
    };
    let Some(path) = callee_path(il, interner, callee) else {
        return false;
    };
    if promise_construct_call(il, call, &path) {
        push_unique(labels, "promise-executor-callback-effect-contract");
        return true;
    }
    match path.as_str() {
        "Promise" => {
            push_unique(labels, "promise-non-construct-call-boundary-contract");
            true
        }
        "Promise.resolve" => {
            push_unique(labels, "promise-factory-settled-value-contract");
            true
        }
        "Promise.reject" => {
            push_unique(labels, "promise-reject-rejected-value-channel-contract");
            true
        }
        "Promise.all" | "Promise.allSettled" | "Promise.any" | "Promise.race" => {
            push_unique(labels, "promise-aggregate-result-channel-contract");
            true
        }
        _ if path.ends_with(".then") => {
            push_unique(labels, "promise-then-callback-demand-effect-contract");
            push_unique(labels, "promise-like-receiver-proof");
            true
        }
        _ if path.ends_with(".catch") => {
            push_unique(labels, "promise-catch-rejection-continuation-contract");
            push_unique(labels, "promise-catch-callback-demand-effect-contract");
            push_unique(labels, "promise-like-receiver-proof");
            true
        }
        _ if path.ends_with(".finally") => {
            push_unique(labels, "promise-finally-settlement-continuation-contract");
            push_unique(labels, "promise-finally-callback-demand-effect-contract");
            push_unique(labels, "promise-like-receiver-proof");
            true
        }
        _ => false,
    }
}

fn js_like_runtime_lang(lang: nose_il::Lang) -> bool {
    matches!(
        lang,
        nose_il::Lang::JavaScript
            | nose_il::Lang::TypeScript
            | nose_il::Lang::Vue
            | nose_il::Lang::Svelte
            | nose_il::Lang::Html
    )
}

fn promise_construct_call(il: &nose_il::Il, call: NodeId, callee_path: &str) -> bool {
    callee_path == "Promise"
        && nose_semantics::source_call_at_node(il, call) == Some(nose_il::SourceCallKind::Construct)
}

fn callee_path(il: &nose_il::Il, interner: &Interner, node: NodeId) -> Option<String> {
    match il.kind(node) {
        nose_il::NodeKind::Var => match il.node(node).payload {
            nose_il::Payload::Name(name) => Some(interner.resolve(name).to_string()),
            _ => None,
        },
        nose_il::NodeKind::Field => {
            let nose_il::Payload::Name(method) = il.node(node).payload else {
                return None;
            };
            let receiver = il.children(node).first().copied()?;
            let receiver = callee_path(il, interner, receiver)?;
            Some(format!("{}.{}", receiver, interner.resolve(method)))
        }
        _ => None,
    }
}

fn hof_missing_evidence(il: &nose_il::Il, interner: &Interner, root: NodeId) -> Vec<&'static str> {
    let mut labels = vec!["hof-demand-effect-profile"];
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
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
        stack.extend(il.children(node).iter().copied());
    }
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
    let mut stack = vec![callback];
    while let Some(node) = stack.pop() {
        match il.kind(node) {
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
        }
        stack.extend(il.children(node).iter().copied());
    }
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

fn callee_identity_call_evidence(
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

fn effect_boundary_node(il: &nose_il::Il, node: NodeId) -> bool {
    match il.node(node).payload {
        nose_il::Payload::Builtin(nose_il::Builtin::Append | nose_il::Builtin::Print) => true,
        _ => {
            il.kind(node) == nose_il::NodeKind::Assign
                && il.children(node).first().is_some_and(|&lhs| {
                    matches!(
                        il.kind(lhs),
                        nose_il::NodeKind::Field | nose_il::NodeKind::Index
                    )
                })
                || expression_statement_call(il, node)
        }
    }
}

fn builtin_call_node(il: &nose_il::Il, node: NodeId) -> bool {
    il.kind(node) == nose_il::NodeKind::Call
        && matches!(il.node(node).payload, nose_il::Payload::Builtin(_))
}

fn expression_statement_call(il: &nose_il::Il, node: NodeId) -> bool {
    il.kind(node) == nose_il::NodeKind::ExprStmt
        && il.children(node).first().is_some_and(|&expr| {
            subtree_has(il, expr, |il, node| {
                il.kind(node) == nose_il::NodeKind::Call
            })
        })
}

fn receiver_method_call(il: &nose_il::Il, interner: &Interner, node: NodeId) -> bool {
    if il.kind(node) != nose_il::NodeKind::Call {
        return false;
    }
    let Some(&callee) = il.children(node).first() else {
        return false;
    };
    if il.kind(callee) != nose_il::NodeKind::Field {
        return false;
    }
    let nose_il::Payload::Name(method) = il.node(callee).payload else {
        return false;
    };
    matches!(
        interner.resolve(method),
        "and_then"
            | "any"
            | "all"
            | "collect"
            | "contains"
            | "end_with?"
            | "endsWith"
            | "filter"
            | "filter_map"
            | "flatMap"
            | "flat_map"
            | "get"
            | "getOrDefault"
            | "is_empty"
            | "isEmpty"
            | "map"
            | "max"
            | "min"
            | "reduce"
            | "reject"
            | "some"
            | "start_with?"
            | "startsWith"
            | "then"
    )
}

fn source_surface_boundary_node(il: &nose_il::Il, node: NodeId) -> bool {
    if rust_macro_invocation_call(il, node) {
        return true;
    }
    matches!(
        il.kind(node),
        nose_il::NodeKind::Seq
            | nose_il::NodeKind::Lambda
            | nose_il::NodeKind::Index
            | nose_il::NodeKind::BinOp
            | nose_il::NodeKind::UnOp
    )
}

fn rust_macro_invocation_call(il: &nose_il::Il, node: NodeId) -> bool {
    il.meta.lang == nose_il::Lang::Rust
        && il.kind(node) == nose_il::NodeKind::Call
        && il.evidence_anchored_at(il.node(node).span).any(|record| {
            matches!(
                record.kind,
                nose_il::EvidenceKind::Source(nose_il::SourceFactKind::Call(
                    nose_il::SourceCallKind::MacroInvocation
                ))
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nose_il::{FileId, Lang};

    fn lowered_js(src: &str) -> (nose_il::Il, Interner) {
        let interner = Interner::new();
        let il = nose_frontend::lower_source(
            FileId(0),
            "promise.js",
            src.as_bytes(),
            Lang::JavaScript,
            &interner,
        )
        .expect("lower JavaScript fixture");
        (il, interner)
    }

    fn missing_evidence_for_call(src: &str, callee_suffix: &str) -> Vec<&'static str> {
        let (il, interner) = lowered_js(src);
        let call = (0..il.nodes.len())
            .map(|idx| NodeId(idx as u32))
            .find(|&node| {
                il.kind(node) == nose_il::NodeKind::Call
                    && il
                        .children(node)
                        .first()
                        .and_then(|&callee| callee_path(&il, &interner, callee))
                        .is_some_and(|path| path.ends_with(callee_suffix))
            })
            .unwrap_or_else(|| panic!("expected call ending in {callee_suffix}"));
        runtime_boundary_missing_evidence(&il, &interner, call)
            .unwrap_or_else(|| panic!("expected runtime boundary evidence for {callee_suffix}"))
    }

    #[test]
    fn promise_reject_missing_evidence_is_rejection_value_channel_specific() {
        let labels = missing_evidence_for_call(
            "function rejectIt(e) { return Promise.reject(e); }\n",
            "Promise.reject",
        );

        assert!(labels.contains(&"promise-reject-rejected-value-channel-contract"));
        assert!(!labels.contains(&"promise-rejection-channel-contract"));
    }

    #[test]
    fn promise_catch_missing_evidence_splits_continuation_from_callback_effect() {
        let labels =
            missing_evidence_for_call("function catchIt(p, h) { return p.catch(h); }\n", ".catch");

        assert!(labels.contains(&"promise-catch-rejection-continuation-contract"));
        assert!(labels.contains(&"promise-catch-callback-demand-effect-contract"));
        assert!(labels.contains(&"promise-like-receiver-proof"));
        assert!(!labels.contains(&"promise-rejection-continuation-contract"));
    }

    #[test]
    fn promise_finally_missing_evidence_splits_settlement_from_callback_effect() {
        let labels = missing_evidence_for_call(
            "function finallyIt(p, h) { return p.finally(h); }\n",
            ".finally",
        );

        assert!(labels.contains(&"promise-finally-settlement-continuation-contract"));
        assert!(labels.contains(&"promise-finally-callback-demand-effect-contract"));
        assert!(labels.contains(&"promise-like-receiver-proof"));
        assert!(!labels.contains(&"promise-rejection-continuation-contract"));
    }
}

use super::{callee_identity::callee_identity_call_evidence, push_unique, visit_subtree};
use nose_il::{Interner, NodeId};

pub(super) fn runtime_boundary_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    root: NodeId,
) -> Option<Vec<&'static str>> {
    let mut labels = vec!["lowered-runtime-boundary-contract"];
    let mut found = false;
    visit_subtree(il, root, |node| {
        if push_runtime_node_missing_evidence(il, node, &mut labels) {
            found = true;
        }
        if il.kind(node) == nose_il::NodeKind::Call {
            found |= push_promise_protocol_call_missing_evidence(il, interner, node, &mut labels);
        }
    });
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
    let path = callee_path(il, interner, callee);
    let method = callee_field_method(il, interner, callee);
    if path
        .as_deref()
        .is_some_and(|path| promise_construct_call(il, call, path))
    {
        push_unique(labels, "promise-executor-callback-effect-contract");
        return true;
    }
    match path.as_deref() {
        Some("Promise") => {
            push_unique(labels, "promise-non-construct-call-boundary-contract");
            true
        }
        Some("Promise.resolve") => {
            push_unique(labels, "promise-factory-settled-value-contract");
            true
        }
        Some("Promise.reject") => {
            push_unique(labels, "promise-reject-rejected-value-channel-contract");
            true
        }
        Some("Promise.all" | "Promise.allSettled" | "Promise.any" | "Promise.race") => {
            push_unique(labels, "promise-aggregate-result-channel-contract");
            true
        }
        _ if method == Some("then") => {
            push_promise_receiver_producer_missing_evidence(il, interner, callee, labels);
            push_unique(labels, "promise-then-promise-like-receiver-proof");
            push_unique(labels, "promise-then-fulfillment-continuation-contract");
            push_unique(labels, "promise-then-rejection-continuation-contract");
            if promise_then_has_callback_slot(il, call) {
                push_unique(labels, "promise-then-callback-demand-effect-contract");
            }
            true
        }
        _ if method == Some("catch") => {
            push_promise_receiver_producer_missing_evidence(il, interner, callee, labels);
            push_unique(labels, "promise-catch-rejection-continuation-contract");
            push_unique(labels, "promise-catch-callback-demand-effect-contract");
            push_unique(labels, "promise-like-receiver-proof");
            true
        }
        _ if method == Some("finally") => {
            push_promise_receiver_producer_missing_evidence(il, interner, callee, labels);
            push_unique(labels, "promise-finally-settlement-continuation-contract");
            push_unique(labels, "promise-finally-callback-demand-effect-contract");
            push_unique(labels, "promise-like-receiver-proof");
            true
        }
        _ => false,
    }
}

fn push_promise_receiver_producer_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    labels: &mut Vec<&'static str>,
) {
    let Some(receiver) = method_receiver(il, callee) else {
        return;
    };
    if receiver_is_promise_constructor_call(il, interner, receiver) {
        push_unique(labels, "promise-constructor-receiver-producer-proof");
        return;
    }
    if receiver_is_async_function_return(il, receiver) {
        push_unique(labels, "promise-async-function-return-producer-proof");
        return;
    }
    if il.kind(receiver) == nose_il::NodeKind::Call {
        push_promise_call_return_receiver_missing_evidence(il, interner, receiver, labels);
    }
}

fn push_promise_call_return_receiver_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    receiver: NodeId,
    labels: &mut Vec<&'static str>,
) {
    push_unique(labels, "promise-call-return-receiver-producer-proof");
    push_unique(
        labels,
        promise_call_return_receiver_callee_evidence(callee_identity_call_evidence(
            il, interner, receiver,
        )),
    );
}

fn promise_call_return_receiver_callee_evidence(callee_evidence: &'static str) -> &'static str {
    match callee_evidence {
        "direct-function-target-present-call-contract-proof" => {
            "promise-call-return-direct-function-return-domain-proof"
        }
        "direct-method-target-present-call-contract-proof" => {
            "promise-call-return-direct-method-return-domain-proof"
        }
        "imported-function-target-present-call-contract-proof" => {
            "promise-call-return-imported-function-return-domain-proof"
        }
        "imported-member-target-present-call-contract-proof" => {
            "promise-call-return-imported-member-return-domain-proof"
        }
        "dynamic-dispatch-target-present-concrete-target-proof" => {
            "promise-call-return-dynamic-dispatch-return-domain-proof"
        }
        "call-target-evidence-rejected" => "promise-call-return-rejected-call-target-proof",
        "scoped-path-call-target-proof" => "promise-call-return-scoped-path-callee-proof",
        "local-or-parameter-call-target-proof" => {
            "promise-call-return-local-or-parameter-callee-proof"
        }
        "imported-binding-call-target-proof" => "promise-call-return-imported-binding-callee-proof",
        "imported-member-call-target-proof" => "promise-call-return-imported-member-callee-proof",
        "qualified-global-call-target-proof" => "promise-call-return-qualified-global-callee-proof",
        "unshadowed-global-call-target-proof" => {
            "promise-call-return-unshadowed-global-callee-proof"
        }
        "member-call-target-proof" => "promise-call-return-member-callee-proof",
        _ => "promise-call-return-unknown-callee-proof",
    }
}

fn method_receiver(il: &nose_il::Il, callee: NodeId) -> Option<NodeId> {
    if il.kind(callee) != nose_il::NodeKind::Field {
        return None;
    }
    il.children(callee).first().copied()
}

fn receiver_is_promise_constructor_call(
    il: &nose_il::Il,
    interner: &Interner,
    receiver: NodeId,
) -> bool {
    if il.kind(receiver) != nose_il::NodeKind::Call {
        return false;
    }
    let Some(&callee) = il.children(receiver).first() else {
        return false;
    };
    callee_path(il, interner, callee)
        .as_deref()
        .is_some_and(|path| promise_construct_call(il, receiver, path))
}

fn receiver_is_async_function_return(il: &nose_il::Il, receiver: NodeId) -> bool {
    if subtree_has_source_protocol(il, receiver, nose_il::SourceProtocolKind::AsyncFunction) {
        return true;
    }
    if il.kind(receiver) != nose_il::NodeKind::Call {
        return false;
    }
    let Some(&callee) = il.children(receiver).first() else {
        return false;
    };
    let Some(callee_name) = callee_var_symbol(il, callee) else {
        return false;
    };
    il.units.iter().any(|unit| {
        unit.name == Some(callee_name)
            && subtree_has_source_protocol(
                il,
                unit.root,
                nose_il::SourceProtocolKind::AsyncFunction,
            )
    })
}

fn callee_var_symbol(il: &nose_il::Il, callee: NodeId) -> Option<nose_il::Symbol> {
    if il.kind(callee) != nose_il::NodeKind::Var {
        return None;
    }
    match il.node(callee).payload {
        nose_il::Payload::Name(name) => Some(name),
        _ => None,
    }
}

fn subtree_has_source_protocol(
    il: &nose_il::Il,
    root: NodeId,
    protocol: nose_il::SourceProtocolKind,
) -> bool {
    let mut found = false;
    visit_subtree(il, root, |node| {
        found |= nose_semantics::source_protocol_at_node(il, node) == Some(protocol);
    });
    found
}

fn promise_then_has_callback_slot(il: &nose_il::Il, call: NodeId) -> bool {
    il.children(call).len() > 1
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

fn callee_field_method<'a>(
    il: &nose_il::Il,
    interner: &'a Interner,
    node: NodeId,
) -> Option<&'a str> {
    if il.kind(node) != nose_il::NodeKind::Field {
        return None;
    }
    let nose_il::Payload::Name(method) = il.node(node).payload else {
        return None;
    };
    Some(interner.resolve(method))
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
                    && call_matches_callee_surface(&il, &interner, node, callee_suffix)
            })
            .unwrap_or_else(|| panic!("expected call ending in {callee_suffix}"));
        runtime_boundary_missing_evidence(&il, &interner, call)
            .unwrap_or_else(|| panic!("expected runtime boundary evidence for {callee_suffix}"))
    }

    fn call_matches_callee_surface(
        il: &nose_il::Il,
        interner: &Interner,
        call: NodeId,
        callee_suffix: &str,
    ) -> bool {
        let Some(&callee) = il.children(call).first() else {
            return false;
        };
        if callee_path(il, interner, callee).is_some_and(|path| path.ends_with(callee_suffix)) {
            return true;
        }
        callee_suffix
            .strip_prefix('.')
            .is_some_and(|method| callee_field_method(il, interner, callee) == Some(method))
    }

    #[test]
    fn promise_then_missing_evidence_splits_receiver_fulfillment_rejection_and_callback() {
        let labels = missing_evidence_for_call(
            "function thenIt(p, f, r) { return p.then(f, r); }\n",
            ".then",
        );

        assert!(labels.contains(&"promise-then-promise-like-receiver-proof"));
        assert!(labels.contains(&"promise-then-fulfillment-continuation-contract"));
        assert!(labels.contains(&"promise-then-rejection-continuation-contract"));
        assert!(labels.contains(&"promise-then-callback-demand-effect-contract"));
        assert!(!labels.contains(&"promise-like-receiver-proof"));
    }

    #[test]
    fn promise_then_on_expression_receiver_still_reports_receiver_obligation() {
        let labels = missing_evidence_for_call(
            "function thenIt(db, id, f) { return db.get(id).then(f); }\n",
            ".then",
        );

        assert!(labels.contains(&"promise-call-return-receiver-producer-proof"));
        assert!(labels.contains(&"promise-call-return-member-callee-proof"));
        assert!(labels.contains(&"promise-then-promise-like-receiver-proof"));
        assert!(labels.contains(&"promise-then-fulfillment-continuation-contract"));
        assert!(labels.contains(&"promise-then-rejection-continuation-contract"));
        assert!(labels.contains(&"promise-then-callback-demand-effect-contract"));
    }

    #[test]
    fn promise_then_on_local_call_receiver_reports_local_callee_obligation() {
        let labels = missing_evidence_for_call(
            "function thenIt(makePromise, f) { return makePromise().then(f); }\n",
            ".then",
        );

        assert!(labels.contains(&"promise-call-return-receiver-producer-proof"));
        assert!(labels.contains(&"promise-call-return-local-or-parameter-callee-proof"));
        assert!(labels.contains(&"promise-then-promise-like-receiver-proof"));
    }

    #[test]
    fn promise_then_constructor_receiver_reports_producer_obligation() {
        let labels = missing_evidence_for_call(
            "function thenIt(executor, f) { return new Promise(executor).then(f); }\n",
            ".then",
        );

        assert!(labels.contains(&"promise-constructor-receiver-producer-proof"));
        assert!(!labels.contains(&"promise-call-return-receiver-producer-proof"));
        assert!(labels.contains(&"promise-then-promise-like-receiver-proof"));
    }

    #[test]
    fn promise_then_async_function_receiver_reports_producer_obligation() {
        let labels = missing_evidence_for_call(
            "async function load() { return 1; }\nfunction thenIt(f) { return load().then(f); }\n",
            ".then",
        );

        assert!(labels.contains(&"promise-async-function-return-producer-proof"));
        assert!(!labels.contains(&"promise-call-return-receiver-producer-proof"));
        assert!(labels.contains(&"promise-then-promise-like-receiver-proof"));
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

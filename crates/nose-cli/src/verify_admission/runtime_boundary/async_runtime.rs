use super::{callee_field_method, callee_path, method_receiver, node_defines_name};
use crate::verify_admission::AdmissionContext;
use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceEmitter, EvidenceKind, EvidenceStatus, Interner,
    NodeId, NodeKind, SymbolEvidenceKind,
};

pub(super) fn push_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    let Some(callee) = il.children(call).first().copied() else {
        return false;
    };
    let Some(path) = callee_path(il, interner, callee) else {
        return false;
    };
    match il.meta.lang {
        nose_il::Lang::Python => push_python_async_runtime_call_missing_evidence(
            il, interner, callee, &path, context, labels,
        ),
        nose_il::Lang::Rust => {
            push_rust_async_runtime_call_missing_evidence(il, call, &path, context, labels)
        }
        nose_il::Lang::Swift => push_swift_async_runtime_call_missing_evidence(
            il, interner, callee, &path, context, labels,
        ),
        _ => false,
    }
}

fn push_python_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if context.python_module_is_local_for_file("asyncio", &il.meta.path) {
        return false;
    }
    let Some(receiver) = method_receiver(il, callee) else {
        return false;
    };
    if !python_asyncio_namespace_receiver_proven(il, interner, receiver) {
        return false;
    }
    match callee_field_method(il, interner, callee) {
        Some("create_task" | "ensure_future") if callee_path.starts_with("asyncio.") => {
            push_task_spawn_missing_evidence(labels);
            true
        }
        Some("sleep") if callee_path == "asyncio.sleep" => {
            super::super::push_unique(labels, "timer-scheduling-contract");
            true
        }
        Some("gather") if callee_path == "asyncio.gather" => {
            push_async_aggregate_all_missing_evidence(labels);
            true
        }
        Some("wait") if callee_path == "asyncio.wait" => {
            push_async_aggregate_completion_missing_evidence(labels);
            super::super::push_unique(labels, "async-aggregate-cancellation-liveness-contract");
            true
        }
        _ => false,
    }
}

fn python_asyncio_namespace_receiver_proven(
    il: &nose_il::Il,
    interner: &Interner,
    receiver: NodeId,
) -> bool {
    if il.kind(receiver) != NodeKind::Var || !node_defines_name(il, interner, receiver, "asyncio") {
        return false;
    }
    if nose_semantics::imported_namespace_symbol(il, interner, receiver, "asyncio") {
        return true;
    }
    let occurrence_span = il.node(receiver).span;
    let mut import_bindings = 0usize;
    let mut shadow_definitions = 0usize;
    for unit in &il.units {
        if il.node(unit.root).span.file == occurrence_span.file
            && unit
                .name
                .is_some_and(|symbol| interner.resolve(symbol) == "asyncio")
        {
            shadow_definitions += 1;
        }
    }
    for (idx, node) in il.nodes.iter().enumerate() {
        if node.span.file != occurrence_span.file {
            continue;
        }
        let node_id = NodeId(idx as u32);
        match node.kind {
            NodeKind::Assign => {
                let Some(lhs) = il.children(node_id).first().copied() else {
                    continue;
                };
                if !node_defines_name(il, interner, lhs, "asyncio") {
                    continue;
                }
                if python_asyncio_import_namespace_binding_at_span(il, node.span) {
                    import_bindings += 1;
                } else {
                    shadow_definitions += 1;
                }
            }
            NodeKind::Param if node_defines_name(il, interner, node_id, "asyncio") => {
                shadow_definitions += 1;
            }
            _ => {}
        }
    }
    import_bindings > 0 && shadow_definitions == 0
}

fn python_asyncio_import_namespace_binding_at_span(il: &nose_il::Il, span: nose_il::Span) -> bool {
    let local_hash = stable_symbol_hash("asyncio");
    let module_hash = stable_symbol_hash("asyncio");
    il.evidence_anchored_at(span).any(|record| {
        record.anchor == EvidenceAnchor::binding(span, local_hash)
            && record.kind
                == EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace { module_hash })
            && record.provenance.emitter == EvidenceEmitter::Builtin
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

fn push_rust_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    call: NodeId,
    callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if runtime_root(callee_path)
        .is_some_and(|root| context.rust_runtime_root_is_local_for_file(root, &il.meta.path))
    {
        return false;
    }
    if rust_async_spawn_path(callee_path) {
        push_task_spawn_missing_evidence(labels);
        return true;
    }
    if nose_semantics::source_call_at_node(il, call)
        != Some(nose_il::SourceCallKind::MacroInvocation)
    {
        return false;
    }
    if rust_async_join_macro_path(callee_path) {
        push_async_aggregate_all_missing_evidence(labels);
        return true;
    }
    if rust_async_select_macro_path(callee_path) {
        push_async_aggregate_first_missing_evidence(labels);
        return true;
    }
    false
}

fn push_swift_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if context.swift_name_is_visible("Task") {
        return false;
    }
    match callee_path {
        "Task" | "Task.detached" if swift_task_root_unshadowed(il, interner, callee) => {
            push_task_spawn_missing_evidence(labels);
            true
        }
        _ => false,
    }
}

fn runtime_root(callee_path: &str) -> Option<&str> {
    callee_path.split("::").next()
}

fn rust_async_spawn_path(callee_path: &str) -> bool {
    matches!(
        callee_path,
        "tokio::spawn"
            | "tokio::task::spawn"
            | "tokio::task::spawn_blocking"
            | "async_std::task::spawn"
            | "async_std::task::spawn_blocking"
    )
}

fn rust_async_join_macro_path(callee_path: &str) -> bool {
    matches!(
        callee_path,
        "tokio::join"
            | "tokio::try_join"
            | "futures::join"
            | "futures::try_join"
            | "futures_util::join"
            | "futures_util::try_join"
    )
}

fn rust_async_select_macro_path(callee_path: &str) -> bool {
    matches!(
        callee_path,
        "tokio::select" | "futures::select" | "futures_util::select"
    )
}

fn swift_task_root_unshadowed(il: &nose_il::Il, interner: &Interner, callee: NodeId) -> bool {
    let root = if il.kind(callee) == NodeKind::Field {
        method_receiver(il, callee).unwrap_or(callee)
    } else {
        callee
    };
    il.kind(root) == NodeKind::Var
        && node_defines_name(il, interner, root, "Task")
        && !nose_semantics::file_defines_name_visible_at(il, interner, "Task", il.node(root).span)
}

fn push_task_spawn_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "task-spawn-scheduling-contract");
    super::super::push_unique(labels, "task-handle-lifecycle-contract");
    super::super::push_unique(labels, "task-cancellation-liveness-contract");
}

fn push_async_aggregate_all_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "async-aggregate-all-completion-contract");
    super::super::push_unique(labels, "async-aggregate-result-channel-contract");
}

fn push_async_aggregate_first_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "async-aggregate-first-completion-contract");
    super::super::push_unique(labels, "async-aggregate-cancellation-liveness-contract");
    super::super::push_unique(labels, "async-aggregate-result-channel-contract");
}

fn push_async_aggregate_completion_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "async-aggregate-completion-contract");
    super::super::push_unique(labels, "async-aggregate-result-channel-contract");
}

use super::{callee_field_method, callee_path, method_receiver, node_defines_name};
use crate::verify_admission::AdmissionContext;
use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceEmitter, EvidenceKind, EvidenceStatus, Interner,
    NodeId, NodeKind, SymbolEvidenceKind,
};

mod rust_imports;

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
        nose_il::Lang::Rust => push_rust_async_runtime_call_missing_evidence(
            il, interner, call, callee, &path, context, labels,
        ),
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
    _callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if context.python_module_is_local_for_file("asyncio", &il.meta.path) {
        return false;
    }
    if let Some(receiver) = method_receiver(il, callee) {
        if python_asyncio_namespace_receiver_proven(il, interner, receiver) {
            return callee_field_method(il, interner, callee)
                .is_some_and(|method| push_python_asyncio_api_missing_evidence(method, labels));
        }
    }
    for exported in ["create_task", "ensure_future", "sleep", "gather", "wait"] {
        if python_asyncio_imported_binding_proven(il, interner, callee, exported) {
            return push_python_asyncio_api_missing_evidence(exported, labels);
        }
    }
    false
}

fn push_python_asyncio_api_missing_evidence(
    exported: &str,
    labels: &mut Vec<&'static str>,
) -> bool {
    match exported {
        "create_task" | "ensure_future" => {
            push_task_spawn_missing_evidence(labels);
            true
        }
        "sleep" => {
            super::super::push_unique(labels, "timer-scheduling-contract");
            true
        }
        "gather" => {
            push_async_aggregate_all_missing_evidence(labels);
            true
        }
        "wait" => {
            push_async_aggregate_completion_missing_evidence(labels);
            super::super::push_unique(labels, "async-aggregate-cancellation-liveness-contract");
            true
        }
        _ => false,
    }
}

fn python_asyncio_imported_binding_proven(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    exported: &str,
) -> bool {
    nose_semantics::imported_binding_symbol(il, interner, callee, "asyncio", exported)
        && !python_imported_binding_shadowed(il, interner, callee, "asyncio", exported)
}

fn python_asyncio_namespace_receiver_proven(
    il: &nose_il::Il,
    interner: &Interner,
    receiver: NodeId,
) -> bool {
    if il.kind(receiver) != NodeKind::Var {
        return false;
    }
    if nose_semantics::imported_namespace_symbol(il, interner, receiver, "asyncio") {
        return true;
    }
    let Some(receiver_name) = super::super::node_exact_name(il, interner, receiver) else {
        return false;
    };
    let occurrence_span = il.node(receiver).span;
    let top_level_statements = top_level_statements(il);
    let mut import_bindings = 0usize;
    let mut shadow_definitions = 0usize;
    for unit in &il.units {
        if il.node(unit.root).span.file == occurrence_span.file
            && unit
                .name
                .is_some_and(|symbol| interner.resolve(symbol) == receiver_name)
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
                if !node_defines_name(il, interner, lhs, receiver_name) {
                    continue;
                }
                if top_level_statements.contains(&node_id)
                    && python_import_namespace_binding_at_span(
                        il,
                        node.span,
                        receiver_name,
                        "asyncio",
                    )
                {
                    import_bindings += 1;
                } else {
                    shadow_definitions += 1;
                }
            }
            NodeKind::Param if node_defines_name(il, interner, node_id, receiver_name) => {
                shadow_definitions += 1;
            }
            _ => {}
        }
    }
    import_bindings > 0 && shadow_definitions == 0
}

fn python_imported_binding_shadowed(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    module: &str,
    exported: &str,
) -> bool {
    let Some(local_name) = super::super::node_exact_name(il, interner, callee) else {
        return false;
    };
    let occurrence_span = il.node(callee).span;
    let top_level_statements = top_level_statements(il);
    for unit in &il.units {
        if il.node(unit.root).span.file == occurrence_span.file
            && unit
                .name
                .is_some_and(|symbol| interner.resolve(symbol) == local_name)
        {
            return true;
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
                if !node_defines_name(il, interner, lhs, local_name) {
                    continue;
                }
                if !top_level_statements.contains(&node_id)
                    || !python_import_binding_at_span(il, node.span, local_name, module, exported)
                {
                    return true;
                }
            }
            NodeKind::Param if node_defines_name(il, interner, node_id, local_name) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn python_import_namespace_binding_at_span(
    il: &nose_il::Il,
    span: nose_il::Span,
    local: &str,
    module: &str,
) -> bool {
    let local_hash = stable_symbol_hash(local);
    let module_hash = stable_symbol_hash(module);
    il.evidence_anchored_at(span).any(|record| {
        record.anchor == EvidenceAnchor::binding(span, local_hash)
            && record.kind
                == EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace { module_hash })
            && record.provenance.emitter == EvidenceEmitter::Builtin
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

fn python_import_binding_at_span(
    il: &nose_il::Il,
    span: nose_il::Span,
    local: &str,
    module: &str,
    exported: &str,
) -> bool {
    let local_hash = stable_symbol_hash(local);
    let module_hash = stable_symbol_hash(module);
    let exported_hash = stable_symbol_hash(exported);
    il.evidence_anchored_at(span).any(|record| {
        record.anchor == EvidenceAnchor::binding(span, local_hash)
            && record.kind
                == EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
                    module_hash,
                    exported_hash,
                })
            && record.provenance.emitter == EvidenceEmitter::Builtin
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

fn top_level_statements(il: &nose_il::Il) -> Vec<NodeId> {
    il.children(il.root)
        .iter()
        .copied()
        .fold(Vec::new(), |mut statements, node| {
            if il.kind(node) == NodeKind::Block {
                statements.extend_from_slice(il.children(node));
            } else {
                statements.push(node);
            }
            statements
        })
}

fn push_rust_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    callee: NodeId,
    callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if runtime_root(callee_path)
        .is_some_and(|root| context.rust_runtime_root_is_local_for_file(root, &il.meta.path))
    {
        return false;
    }
    let is_macro_invocation = nose_semantics::source_call_at_node(il, call)
        == Some(nose_il::SourceCallKind::MacroInvocation);
    if !is_macro_invocation && rust_async_spawn_path(callee_path) {
        push_task_spawn_missing_evidence(labels);
        return true;
    }
    if !is_macro_invocation && rust_imported_async_spawn_member(il, interner, callee, context) {
        push_task_spawn_missing_evidence(labels);
        return true;
    }
    if !is_macro_invocation {
        return false;
    }
    if rust_async_join_macro_path(callee_path) {
        push_async_aggregate_all_missing_evidence(labels);
        return true;
    }
    if rust_imported_async_join_macro_member(il, interner, callee, context) {
        push_async_aggregate_all_missing_evidence(labels);
        return true;
    }
    if rust_async_select_macro_path(callee_path) {
        push_async_aggregate_first_missing_evidence(labels);
        return true;
    }
    if rust_imported_async_select_macro_member(il, interner, callee, context) {
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

fn module_root(module: &str) -> &str {
    module.split("::").next().unwrap_or(module)
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

fn rust_imported_async_spawn_member(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    context: &AdmissionContext,
) -> bool {
    rust_imported_runtime_member(il, interner, callee, "tokio", "spawn", context)
        || rust_imported_runtime_member(il, interner, callee, "tokio::task", "spawn", context)
        || rust_imported_runtime_member(
            il,
            interner,
            callee,
            "tokio::task",
            "spawn_blocking",
            context,
        )
        || rust_imported_runtime_member(il, interner, callee, "async_std::task", "spawn", context)
        || rust_imported_runtime_member(
            il,
            interner,
            callee,
            "async_std::task",
            "spawn_blocking",
            context,
        )
}

fn rust_imported_async_join_macro_member(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    context: &AdmissionContext,
) -> bool {
    rust_imported_runtime_member(il, interner, callee, "tokio", "join", context)
        || rust_imported_runtime_member(il, interner, callee, "tokio", "try_join", context)
        || rust_imported_runtime_member(il, interner, callee, "futures", "join", context)
        || rust_imported_runtime_member(il, interner, callee, "futures", "try_join", context)
        || rust_imported_runtime_member(il, interner, callee, "futures_util", "join", context)
        || rust_imported_runtime_member(il, interner, callee, "futures_util", "try_join", context)
}

fn rust_imported_async_select_macro_member(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    context: &AdmissionContext,
) -> bool {
    rust_imported_runtime_member(il, interner, callee, "tokio", "select", context)
        || rust_imported_runtime_member(il, interner, callee, "futures", "select", context)
        || rust_imported_runtime_member(il, interner, callee, "futures_util", "select", context)
}

fn rust_imported_runtime_member(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    module: &str,
    exported: &str,
    context: &AdmissionContext,
) -> bool {
    if context.rust_runtime_root_is_local_for_file(module_root(module), &il.meta.path) {
        return false;
    }
    (nose_semantics::imported_member_symbol(il, interner, callee, module, exported)
        || rust_imports::rust_imported_binding_evidence_only_symbol(
            il, interner, callee, module, exported,
        ))
        && !rust_imported_member_shadowed(il, interner, callee, module, exported)
}

fn rust_imported_member_shadowed(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    module: &str,
    exported: &str,
) -> bool {
    let Some(local_name) = super::super::node_exact_name(il, interner, callee) else {
        return false;
    };
    let occurrence_span = il.node(callee).span;
    for unit in &il.units {
        if il.node(unit.root).span.file == occurrence_span.file
            && unit
                .name
                .is_some_and(|symbol| interner.resolve(symbol) == local_name)
        {
            return true;
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
                if !node_defines_name(il, interner, lhs, local_name) {
                    continue;
                }
                if !rust_imported_binding_at_span(il, node.span, local_name, module, exported) {
                    return true;
                }
            }
            NodeKind::Block | NodeKind::Module | NodeKind::Param
                if node_defines_name(il, interner, node_id, local_name) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn rust_imported_binding_at_span(
    il: &nose_il::Il,
    span: nose_il::Span,
    local: &str,
    module: &str,
    exported: &str,
) -> bool {
    let local_hash = stable_symbol_hash(local);
    let module_hash = stable_symbol_hash(module);
    let exported_hash = stable_symbol_hash(exported);
    il.evidence_anchored_at(span).any(|record| {
        record.anchor == EvidenceAnchor::binding(span, local_hash)
            && record.kind
                == EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
                    module_hash,
                    exported_hash,
                })
            && record.provenance.emitter == EvidenceEmitter::Builtin
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
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

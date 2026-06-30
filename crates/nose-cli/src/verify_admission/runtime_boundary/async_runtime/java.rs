use nose_il::{
    stable_symbol_hash, DomainEvidence, EvidenceAnchor, EvidenceEmitter, EvidenceId, EvidenceKind,
    EvidenceRecord, EvidenceStatus, ImportEvidenceKind, Interner, NodeId, NodeKind, Payload, Span,
    Symbol, SymbolEvidenceKind,
};

const COMPLETABLE_FUTURE_MODULE: &str = "java.util.concurrent";
const COMPLETABLE_FUTURE_TYPE: &str = "CompletableFuture";
const COMPLETABLE_FUTURE_QUALIFIED: &str = "java.util.concurrent.CompletableFuture";
const COMPLETION_STAGE_TYPE: &str = "CompletionStage";

pub(super) fn push_java_future_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    callee: NodeId,
    callee_path: &str,
    _context: &crate::verify_admission::AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    if let Some(method) = completable_future_static_method(il, interner, call, callee, callee_path)
    {
        return push_completable_future_static_method_missing_evidence(method, labels);
    }

    let Some(method) = super::super::callee_field_method(il, interner, callee) else {
        return false;
    };
    if java_future_like_receiver_proven(il, interner, callee) {
        return push_completion_stage_continuation_missing_evidence(method, labels);
    }
    false
}

fn completable_future_static_method<'a>(
    il: &nose_il::Il,
    interner: &'a Interner,
    call: NodeId,
    callee: NodeId,
    callee_path: &'a str,
) -> Option<&'a str> {
    let (receiver_path, method) = callee_path.rsplit_once('.')?;
    if receiver_path == COMPLETABLE_FUTURE_QUALIFIED {
        return Some(method);
    }
    if receiver_path != COMPLETABLE_FUTURE_TYPE {
        return None;
    }
    let receiver = super::super::method_receiver(il, callee)?;
    java_completable_future_simple_receiver_proven(il, interner, call, receiver).then_some(method)
}

fn java_completable_future_simple_receiver_proven(
    il: &nose_il::Il,
    interner: &Interner,
    call: NodeId,
    receiver: NodeId,
) -> bool {
    il.kind(receiver) == NodeKind::Var
        && super::super::node_defines_name(il, interner, receiver, COMPLETABLE_FUTURE_TYPE)
        && !java_simple_type_shadowed(il, interner, receiver)
        && (nose_semantics::imported_binding_symbol(
            il,
            interner,
            receiver,
            COMPLETABLE_FUTURE_MODULE,
            COMPLETABLE_FUTURE_TYPE,
        ) || java_wildcard_import_proves_completable_future(il, call))
}

fn java_simple_type_shadowed(il: &nose_il::Il, interner: &Interner, receiver: NodeId) -> bool {
    let occurrence_span = il.node(receiver).span;
    let top_level_statements = super::top_level_statements(il);
    for unit in &il.units {
        if il.node(unit.root).span.file == occurrence_span.file
            && unit
                .name
                .is_some_and(|symbol| interner.resolve(symbol) == COMPLETABLE_FUTURE_TYPE)
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
                if !super::super::node_defines_name(il, interner, lhs, COMPLETABLE_FUTURE_TYPE) {
                    continue;
                }
                if !top_level_statements.contains(&node_id)
                    || !java_imported_completable_future_at_span(il, node.span)
                {
                    return true;
                }
            }
            NodeKind::Param
                if super::super::node_defines_name(
                    il,
                    interner,
                    node_id,
                    COMPLETABLE_FUTURE_TYPE,
                ) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn java_imported_completable_future_at_span(il: &nose_il::Il, span: Span) -> bool {
    let local_hash = stable_symbol_hash(COMPLETABLE_FUTURE_TYPE);
    let module_hash = stable_symbol_hash(COMPLETABLE_FUTURE_MODULE);
    let exported_hash = stable_symbol_hash(COMPLETABLE_FUTURE_TYPE);
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

fn java_wildcard_import_proves_completable_future(il: &nose_il::Il, call: NodeId) -> bool {
    let call_span = il.node(call).span;
    if java_explicit_completable_future_import_conflicts(il, call_span) {
        return false;
    }
    let expected = EvidenceKind::Import(ImportEvidenceKind::Wildcard {
        module_hash: stable_symbol_hash(COMPLETABLE_FUTURE_MODULE),
    });
    il.evidence.iter().any(|record| {
        record.kind == expected
            && record.status == EvidenceStatus::Asserted
            && record.provenance.emitter == EvidenceEmitter::Builtin
            && matches!(
                record.anchor,
                EvidenceAnchor::SourceSpan(span)
                    if span.file == call_span.file && span.end_byte <= call_span.start_byte
            )
            && il.evidence_dependencies_asserted(record)
    })
}

fn java_explicit_completable_future_import_conflicts(il: &nose_il::Il, call_span: Span) -> bool {
    let local_hash = stable_symbol_hash(COMPLETABLE_FUTURE_TYPE);
    let expected = SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash(COMPLETABLE_FUTURE_MODULE),
        exported_hash: stable_symbol_hash(COMPLETABLE_FUTURE_TYPE),
    };
    il.evidence.iter().any(|record| {
        matches!(
            record.anchor,
            EvidenceAnchor::Binding { span, local_hash: actual }
                if actual == local_hash
                    && span.file == call_span.file
                    && span.end_byte <= call_span.start_byte
        ) && matches!(record.kind, EvidenceKind::Symbol(actual) if actual != expected)
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

fn java_future_like_receiver_proven(il: &nose_il::Il, interner: &Interner, callee: NodeId) -> bool {
    let Some(receiver) = super::super::method_receiver(il, callee) else {
        return false;
    };
    if nose_semantics::domain_evidence_for_receiver(il, interner, receiver)
        != Some(DomainEvidence::FutureLike)
    {
        return false;
    }
    java_future_like_receiver_domain_record(il, receiver).is_some_and(|record| {
        record
            .dependencies
            .iter()
            .copied()
            .any(|dependency| java_completion_stage_type_import_dependency(il, dependency))
    })
}

fn java_future_like_receiver_domain_record(
    il: &nose_il::Il,
    receiver: NodeId,
) -> Option<&EvidenceRecord> {
    let span = java_receiver_param_span(il, receiver)?;
    il.evidence_anchored_at(span).find(|record| {
        record.anchor == EvidenceAnchor::param(span)
            && record.kind == EvidenceKind::Domain(DomainEvidence::FutureLike)
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record)
    })
}

fn java_receiver_param_span(il: &nose_il::Il, receiver: NodeId) -> Option<Span> {
    match il.node(receiver).payload {
        Payload::Name(name) => java_nearest_named_param_span(il, receiver, name),
        _ => None,
    }
}

fn java_nearest_named_param_span(il: &nose_il::Il, receiver: NodeId, name: Symbol) -> Option<Span> {
    let target = il.node(receiver).span;
    let mut best: Option<(u32, Span)> = None;
    for (idx, candidate) in il.nodes.iter().enumerate() {
        if !matches!(candidate.kind, NodeKind::Func | NodeKind::Lambda) {
            continue;
        }
        if candidate.span.file != target.file
            || candidate.span.start_byte > target.start_byte
            || target.end_byte > candidate.span.end_byte
        {
            continue;
        }
        let scope = NodeId(idx as u32);
        let Some(param) = il.children(scope).iter().copied().find(|&child| {
            il.kind(child) == NodeKind::Param && il.node(child).payload == Payload::Name(name)
        }) else {
            continue;
        };
        let width = candidate
            .span
            .end_byte
            .saturating_sub(candidate.span.start_byte);
        let span = il.node(param).span;
        if best.is_none_or(|(best_width, _)| width < best_width) {
            best = Some((width, span));
        }
    }
    best.map(|(_, span)| span)
}

fn java_completion_stage_type_import_dependency(il: &nose_il::Il, dependency: EvidenceId) -> bool {
    let Some(record) = il.evidence.get(dependency.0 as usize) else {
        return false;
    };
    let expected_module_hash = stable_symbol_hash(COMPLETABLE_FUTURE_MODULE);
    let supported_type = |exported_hash| {
        exported_hash == stable_symbol_hash(COMPLETABLE_FUTURE_TYPE)
            || exported_hash == stable_symbol_hash(COMPLETION_STAGE_TYPE)
    };
    matches!(
        record.kind,
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash,
            exported_hash,
        }) if module_hash == expected_module_hash && supported_type(exported_hash)
    ) && record.status == EvidenceStatus::Asserted
        && record.provenance.emitter == EvidenceEmitter::Builtin
        && il.evidence_dependencies_asserted(record)
}

fn push_completable_future_static_method_missing_evidence(
    method: &str,
    labels: &mut Vec<&'static str>,
) -> bool {
    match method {
        "supplyAsync" | "runAsync" => {
            super::push_task_spawn_missing_evidence(labels);
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        "completedFuture" | "completedStage" => {
            push_future_settled_value_missing_evidence(labels);
            true
        }
        "failedFuture" | "failedStage" => {
            push_future_settled_value_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        "allOf" => {
            super::push_async_aggregate_all_missing_evidence(labels);
            push_future_settled_value_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        "anyOf" => {
            super::push_async_aggregate_first_missing_evidence(labels);
            push_future_settled_value_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        _ => false,
    }
}

fn push_completion_stage_continuation_missing_evidence(
    method: &str,
    labels: &mut Vec<&'static str>,
) -> bool {
    match method {
        "thenApply" | "thenApplyAsync" | "thenAccept" | "thenAcceptAsync" | "thenRun"
        | "thenRunAsync" | "thenCompose" | "thenComposeAsync" => {
            super::super::push_unique(labels, "future-fulfillment-continuation-contract");
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            true
        }
        "thenCombine"
        | "thenCombineAsync"
        | "thenAcceptBoth"
        | "thenAcceptBothAsync"
        | "runAfterBoth"
        | "runAfterBothAsync" => {
            super::push_async_aggregate_all_missing_evidence(labels);
            super::super::push_unique(labels, "future-fulfillment-continuation-contract");
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            true
        }
        "applyToEither"
        | "applyToEitherAsync"
        | "acceptEither"
        | "acceptEitherAsync"
        | "runAfterEither"
        | "runAfterEitherAsync" => {
            super::push_async_aggregate_first_missing_evidence(labels);
            super::super::push_unique(labels, "future-fulfillment-continuation-contract");
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            true
        }
        "exceptionally"
        | "exceptionallyAsync"
        | "exceptionallyCompose"
        | "exceptionallyComposeAsync" => {
            super::super::push_unique(labels, "future-exception-continuation-contract");
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        "handle" | "handleAsync" | "whenComplete" | "whenCompleteAsync" => {
            super::super::push_unique(labels, "future-settlement-continuation-contract");
            push_future_settled_value_missing_evidence(labels);
            push_future_callback_missing_evidence(labels);
            super::super::push_unique(labels, "exception-channel-contract");
            true
        }
        _ => false,
    }
}

fn push_future_settled_value_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "future-settled-value-channel-contract");
}

fn push_future_callback_missing_evidence(labels: &mut Vec<&'static str>) {
    super::super::push_unique(labels, "future-callback-demand-effect-contract");
}

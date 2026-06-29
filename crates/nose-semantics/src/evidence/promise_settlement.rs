use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PromiseSettledValueAtCall {
    pub channel: PromiseSettlementChannel,
    pub payload: NodeId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromiseSettledValueEvidenceStatus {
    Missing,
    Admitted(PromiseSettledValueAtCall),
    Rejected,
}

pub fn promise_settled_value_evidence_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<PromiseSettledValueAtCall> {
    match promise_settled_value_evidence_status_at_call(il, interner, call) {
        PromiseSettledValueEvidenceStatus::Admitted(settled) => Some(settled),
        PromiseSettledValueEvidenceStatus::Missing
        | PromiseSettledValueEvidenceStatus::Rejected => None,
    }
}

pub fn promise_settled_value_evidence_status_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> PromiseSettledValueEvidenceStatus {
    if il.kind(call) != NodeKind::Call {
        return PromiseSettledValueEvidenceStatus::Missing;
    }
    let settled = match builtin_promise_settlement_evidence_at_call(il, call) {
        EvidenceResolution::Found(settled) => settled,
        EvidenceResolution::Ambiguous => return PromiseSettledValueEvidenceStatus::Rejected,
        EvidenceResolution::Missing => return PromiseSettledValueEvidenceStatus::Missing,
    };
    match call_target_evidence_status_at_call(il, interner, call) {
        CallTargetEvidenceStatus::Admitted(
            CallTargetEvidenceKind::ImportedFunction { .. }
            | CallTargetEvidenceKind::ImportedMember { .. },
        ) => {}
        CallTargetEvidenceStatus::Admitted(
            CallTargetEvidenceKind::DirectFunction { .. }
            | CallTargetEvidenceKind::DirectMethod { .. }
            | CallTargetEvidenceKind::DynamicDispatch { .. },
        )
        | CallTargetEvidenceStatus::Rejected
        | CallTargetEvidenceStatus::Missing => return PromiseSettledValueEvidenceStatus::Rejected,
    }
    let Some(payload) =
        node_at_exact_span_with_kind(il, settled.payload_span, settled.payload_kind)
    else {
        return PromiseSettledValueEvidenceStatus::Rejected;
    };
    PromiseSettledValueEvidenceStatus::Admitted(PromiseSettledValueAtCall {
        channel: settled.channel,
        payload,
    })
}

fn builtin_promise_settlement_evidence_at_call(
    il: &Il,
    call: NodeId,
) -> EvidenceResolution<PromiseSettledValueEvidenceKind> {
    let call_span = il.node(call).span;
    let mut found = None;
    for record in il.evidence_anchored_at(call_span) {
        if !matches!(
            record.anchor,
            EvidenceAnchor::Node { span, kind } if span == call_span && kind == NodeKind::Call
        ) {
            continue;
        }
        let EvidenceKind::PromiseSettledValue(settled) = record.kind else {
            continue;
        };
        if record.provenance.emitter != EvidenceEmitter::Builtin {
            continue;
        }
        if record.status != EvidenceStatus::Asserted || !il.evidence_dependencies_asserted(record) {
            return EvidenceResolution::Ambiguous;
        }
        match found {
            None => found = Some(settled),
            Some(existing) if existing == settled => {}
            Some(_) => return EvidenceResolution::Ambiguous,
        }
    }
    found.map_or(EvidenceResolution::Missing, EvidenceResolution::Found)
}

fn node_at_exact_span_with_kind(il: &Il, span: Span, kind: NodeKind) -> Option<NodeId> {
    let mut found = None;
    for id in il.nodes_spanning(span) {
        let node = il.node(id);
        if node.span != span || node.kind != kind {
            continue;
        }
        if found.replace(id).is_some() {
            return None;
        }
    }
    found
}

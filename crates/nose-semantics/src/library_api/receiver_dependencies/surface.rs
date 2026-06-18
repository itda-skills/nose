use super::*;

pub(in crate::library_api) fn sequence_surface_dependency_id_for_receiver(
    il: &Il,
    interner: &Interner,
    receiver: NodeId,
    contract: MethodReceiverContract,
) -> Option<EvidenceId> {
    if il.kind(receiver) != NodeKind::Seq {
        return None;
    }
    let surface = seq_surface_contract_for_node(il, interner, receiver)?;
    if !sequence_surface_satisfies_method_receiver(surface, contract) {
        return None;
    }
    let anchor = EvidenceAnchor::sequence(il.node(receiver).span);
    let mut found = None;
    for record in il.evidence_anchored_at(anchor.span()) {
        let EvidenceKind::SequenceSurface(kind) = record.kind else {
            continue;
        };
        if record.anchor != anchor
            || record.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(record)
        {
            continue;
        }
        match found {
            None => found = Some((kind, record.id)),
            Some((existing, _)) if existing == kind => {}
            Some(_) => return None,
        }
    }
    found.map(|(_, id)| id)
}

pub(in crate::library_api) fn static_index_membership_receiver_dependency_id(
    il: &Il,
    interner: &Interner,
    receiver: NodeId,
    contract: StaticIndexMembershipReceiverContract,
) -> Option<EvidenceId> {
    static_index_membership_receiver_dependency_id_at_span(
        il,
        interner,
        il.node(receiver).span,
        contract,
    )
    .filter(|_| static_index_membership_receiver_shape_matches(il, interner, receiver, contract))
}

pub(in crate::library_api) fn static_index_membership_receiver_dependency_id_at_span(
    il: &Il,
    interner: &Interner,
    span: Span,
    contract: StaticIndexMembershipReceiverContract,
) -> Option<EvidenceId> {
    let receiver = node_at_span_with_kind(il, span, NodeKind::Seq)?;
    if !static_index_membership_receiver_shape_matches(il, interner, receiver, contract) {
        return None;
    }
    let anchor = EvidenceAnchor::sequence(span);
    let mut found = None;
    for record in il.evidence_anchored_at(anchor.span()) {
        let EvidenceKind::SequenceSurface(kind) = record.kind else {
            continue;
        };
        if record.anchor != anchor
            || record.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(record)
        {
            continue;
        }
        match found {
            None => found = Some((kind, record.id)),
            Some((existing, _)) if existing == kind => {}
            Some(_) => return None,
        }
    }
    found.and_then(|(kind, id)| (kind == SequenceSurfaceKind::Collection).then_some(id))
}

pub(in crate::library_api) fn static_index_membership_receiver_shape_matches(
    il: &Il,
    interner: &Interner,
    receiver: NodeId,
    contract: StaticIndexMembershipReceiverContract,
) -> bool {
    match contract {
        StaticIndexMembershipReceiverContract::StaticNonFloatLiteralCollection => {
            if il.kind(receiver) != NodeKind::Seq {
                return false;
            }
            if !seq_surface_contract_for_node(il, interner, receiver)
                .is_some_and(|surface| surface.membership_collection)
            {
                return false;
            }
            let kids = il.children(receiver);
            !kids.is_empty()
                && kids.iter().all(|&kid| {
                    il.kind(kid) == NodeKind::Lit
                        && matches!(
                            il.node(kid).payload,
                            Payload::LitInt(_)
                                | Payload::LitBool(_)
                                | Payload::LitStr(_)
                                | Payload::Lit(LitClass::Null)
                        )
                })
        }
    }
}

pub(in crate::library_api) fn sequence_surface_satisfies_method_receiver(
    surface: SeqSurfaceContract,
    contract: MethodReceiverContract,
) -> bool {
    match contract {
        MethodReceiverContract::ExactCollection
        | MethodReceiverContract::ExactProtocol
        | MethodReceiverContract::ExactProtocolPairArgument
        | MethodReceiverContract::ExactCollectionOrJavaKeySet => surface.membership_collection,
        MethodReceiverContract::ExactMap | MethodReceiverContract::ExactMapLiteral => {
            surface.value_tag == SEQ_VALUE_MAP
        }
        MethodReceiverContract::ExactCollectionOrMap
        | MethodReceiverContract::ExactCollectionOrMapLiteral => {
            surface.membership_collection || surface.value_tag == SEQ_VALUE_MAP
        }
        MethodReceiverContract::ExactSetOrMap => surface.value_tag == SEQ_VALUE_MAP,
        _ => false,
    }
}

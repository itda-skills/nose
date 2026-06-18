use super::*;

pub(super) fn proven_map_get_call_parts(
    old: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<(NodeId, NodeId)> {
    let (map, key) = map_get_call_parts(old, interner, node)?;
    source_map_expr(old, interner, map).then_some((map, key))
}

pub(super) fn source_map_expr(old: &Il, interner: &Interner, node: NodeId) -> bool {
    map_like_literal(old, interner, node) || rust_std_map_factory_call(old, interner, node)
}

pub(super) fn rust_std_map_factory_call(old: &Il, interner: &Interner, node: NodeId) -> bool {
    if old.kind(node) != NodeKind::Call {
        return false;
    }
    let kids = old.children(node);
    if kids.len() != 2 || old.kind(kids[1]) != NodeKind::Seq {
        return false;
    }
    let Some(admitted) = admitted_free_name_map_factory_at_call(old, interner, node) else {
        return false;
    };
    let LibraryMapFactoryResult::EntrySequence { entry_seq_tag } = admitted.contract.result else {
        return false;
    };
    if seq_surface_contract_for_node(old, interner, kids[1])
        .is_none_or(|contract| contract.value_tag != SEQ_VALUE_COLLECTION)
    {
        return false;
    }
    map_factory_entries_match_surface(old, interner, kids[1], entry_seq_tag)
}

pub(super) fn map_factory_entries_match_surface(
    old: &Il,
    interner: &Interner,
    entries: NodeId,
    entry_seq_tag: u64,
) -> bool {
    old.children(entries).iter().all(|&entry| {
        old.kind(entry) == NodeKind::Seq
            && seq_surface_contract_for_node(old, interner, entry)
                .is_some_and(|contract| contract.value_tag == entry_seq_tag)
    })
}

pub(super) fn map_like_literal(old: &Il, interner: &Interner, id: NodeId) -> bool {
    if old.kind(id) != NodeKind::Seq {
        return false;
    }
    seq_surface_contract_for_node(old, interner, id)
        .is_some_and(|contract| contract.value_tag == SEQ_VALUE_MAP)
}

pub(super) fn map_get_call_parts(
    old: &Il,
    interner: &Interner,
    id: NodeId,
) -> Option<(NodeId, NodeId)> {
    if old.kind(id) != NodeKind::Call {
        return None;
    }
    let kids = old.children(id);
    if kids.len() != 2 {
        return None;
    }
    let admitted = admitted_map_get_at_call(old, interner, id)?;
    let receiver = admitted.receiver?;
    Some((receiver, kids[1]))
}

pub(super) fn key_set_receiver(old: &Il, interner: &Interner, id: NodeId) -> Option<NodeId> {
    if old.kind(id) != NodeKind::Call {
        return None;
    }
    let admitted = admitted_map_key_view_at_call(old, interner, id)?;
    let contract = admitted.contract.result;
    if contract.kind != MapKeyViewKind::Collection {
        return None;
    }
    admitted.receiver
}

pub(super) fn zero_arg_lambda_body(old: &Il, lambda: NodeId) -> Option<NodeId> {
    if old.kind(lambda) != NodeKind::Lambda {
        return None;
    }
    let kids = old.children(lambda);
    if kids.len() == 1 {
        Some(kids[0])
    } else {
        None
    }
}

pub(super) fn zero_arg_lambda_body_value(old: &Il, lambda: NodeId) -> Option<NodeId> {
    let body = zero_arg_lambda_body(old, lambda)?;
    implicit_block_value(old, body).or(Some(body))
}

pub(super) fn implicit_block_value(old: &Il, block: NodeId) -> Option<NodeId> {
    if old.kind(block) != NodeKind::Block {
        return None;
    }
    let kids = old.children(block);
    let &[stmt] = kids else {
        return None;
    };
    match old.kind(stmt) {
        NodeKind::ExprStmt | NodeKind::Return => {
            let stmt_kids = old.children(stmt);
            let &[expr] = stmt_kids else {
                return None;
            };
            Some(expr)
        }
        _ => None,
    }
}

pub(super) fn identity_lambda(old: &Il, lambda: NodeId) -> bool {
    if old.kind(lambda) != NodeKind::Lambda {
        return false;
    }
    let kids = old.children(lambda);
    if kids.len() != 2 || old.kind(kids[0]) != NodeKind::Param || old.kind(kids[1]) != NodeKind::Var
    {
        return false;
    }
    match (old.node(kids[0]).payload, old.node(kids[1]).payload) {
        (Payload::Cid(a), Payload::Cid(b)) => a == b,
        (Payload::Name(a), Payload::Name(b)) => a == b,
        _ => false,
    }
}

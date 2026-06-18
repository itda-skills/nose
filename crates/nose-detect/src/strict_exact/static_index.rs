use super::*;

pub(super) fn strict_exact_static_index_membership_safe(
    il: &Il,
    interner: &Interner,
    facts: &StrictFacts,
    node: NodeId,
) -> bool {
    let Payload::Op(op) = il.node(node).payload else {
        return false;
    };
    let kids = il.children(node);
    if kids.len() != 2 {
        return false;
    }
    if strict_exact_index_membership_threshold(il, op, false, kids[1]) {
        if let Some((element, collection)) =
            strict_exact_static_index_membership_parts(il, interner, facts, kids[0])
        {
            return strict_exact_safe_tree(il, interner, facts, element)
                && strict_exact_static_non_float_collection(il, interner, collection);
        }
    }
    if strict_exact_index_membership_threshold(il, op, true, kids[0]) {
        if let Some((element, collection)) =
            strict_exact_static_index_membership_parts(il, interner, facts, kids[1])
        {
            return strict_exact_safe_tree(il, interner, facts, element)
                && strict_exact_static_non_float_collection(il, interner, collection);
        }
    }
    false
}

fn strict_exact_static_index_membership_parts(
    il: &Il,
    interner: &Interner,
    facts: &StrictFacts,
    node: NodeId,
) -> Option<(NodeId, NodeId)> {
    if il.kind(node) != NodeKind::Call {
        return None;
    }
    let kids = il.children(node);
    if kids.len() != 2 || il.kind(kids[0]) != NodeKind::Field {
        return None;
    }
    let admitted = admitted_static_index_membership_at_call(il, interner, node)?;
    let receiver = admitted.receiver?;
    if !strict_exact_static_non_float_collection(il, interner, receiver) {
        return None;
    }
    match admitted.contract.result.kind {
        StaticIndexMembershipKind::IndexOf => Some((kids[1], receiver)),
        StaticIndexMembershipKind::FindIndex => {
            let element = strict_exact_lambda_eq_param_element(il, interner, facts, kids[1])?;
            Some((element, receiver))
        }
    }
}

fn strict_exact_index_membership_threshold(
    il: &Il,
    op: Op,
    index_call_on_right: bool,
    threshold: NodeId,
) -> bool {
    if strict_exact_minus_one_literal(il, threshold) {
        return semantics(il.meta.lang)
            .operators()
            .static_index_membership_threshold(
                op,
                index_call_on_right,
                IndexMembershipThreshold::MinusOne,
            )
            .is_some();
    }
    if matches!(il.node(threshold).payload, Payload::LitInt(0)) {
        return semantics(il.meta.lang)
            .operators()
            .static_index_membership_threshold(
                op,
                index_call_on_right,
                IndexMembershipThreshold::Zero,
            )
            .is_some();
    }
    false
}

fn strict_exact_lambda_eq_param_element(
    il: &Il,
    interner: &Interner,
    facts: &StrictFacts,
    lambda: NodeId,
) -> Option<NodeId> {
    if il.kind(lambda) != NodeKind::Lambda {
        return None;
    }
    let kids = il.children(lambda);
    let param = kids.iter().find_map(|&kid| {
        if il.kind(kid) != NodeKind::Param {
            return None;
        }
        if let Payload::Cid(cid) = il.node(kid).payload {
            Some(cid)
        } else {
            None
        }
    })?;
    let ret = strict_exact_first_return_expr(il, *kids.last()?)?;
    if il.kind(ret) != NodeKind::BinOp || !matches!(il.node(ret).payload, Payload::Op(Op::Eq)) {
        return None;
    }
    let source_operator = source_operator_at_node(il, ret)?;
    if !exact_static_membership_predicate_operator(il.meta.lang, Op::Eq, source_operator) {
        return None;
    }
    let ret_kids = il.children(ret);
    if ret_kids.len() != 2 {
        return None;
    }
    if strict_exact_lambda_param_var(il, ret_kids[0], param) {
        return strict_exact_safe_tree(il, interner, facts, ret_kids[1]).then_some(ret_kids[1]);
    }
    if strict_exact_lambda_param_var(il, ret_kids[1], param) {
        return strict_exact_safe_tree(il, interner, facts, ret_kids[0]).then_some(ret_kids[0]);
    }
    None
}

fn strict_exact_first_return_expr(il: &Il, node: NodeId) -> Option<NodeId> {
    if il.kind(node) == NodeKind::Return {
        return il.children(node).first().copied();
    }
    if il.kind(node) == NodeKind::Block {
        return il
            .children(node)
            .iter()
            .find_map(|&child| strict_exact_first_return_expr(il, child));
    }
    None
}

fn strict_exact_lambda_param_var(il: &Il, node: NodeId, param: u32) -> bool {
    il.kind(node) == NodeKind::Var
        && matches!(il.node(node).payload, Payload::Cid(cid) if cid == param)
}

fn strict_exact_minus_one_literal(il: &Il, node: NodeId) -> bool {
    if matches!(il.node(node).payload, Payload::LitInt(-1)) {
        return true;
    }
    if il.kind(node) != NodeKind::UnOp || !matches!(il.node(node).payload, Payload::Op(Op::Neg)) {
        return false;
    }
    let kids = il.children(node);
    kids.len() == 1 && matches!(il.node(kids[0]).payload, Payload::LitInt(1))
}

pub(super) fn strict_exact_static_non_float_collection(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> bool {
    if il.kind(node) != NodeKind::Seq {
        return false;
    }
    if !seq_surface_contract_for_node(il, interner, node)
        .is_some_and(|contract| contract.membership_collection)
    {
        return false;
    }
    let kids = il.children(node);
    !kids.is_empty()
        && kids.iter().all(|&kid| {
            matches!(
                il.node(kid).payload,
                Payload::LitInt(_)
                    | Payload::LitBool(_)
                    | Payload::LitStr(_)
                    | Payload::Lit(LitClass::Null)
            )
        })
}

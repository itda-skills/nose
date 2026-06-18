use super::*;

pub(super) fn exact_literal_safe(il: &Il, node: NodeId) -> bool {
    matches!(
        il.node(node).payload,
        Payload::LitInt(_)
            | Payload::LitBool(_)
            | Payload::LitStr(_)
            | Payload::LitFloat(_)
            | Payload::Lit(LitClass::Null)
    )
}

pub(super) fn strict_exact_safe_var(il: &Il, facts: &StrictFacts, node: NodeId) -> bool {
    match il.node(node).payload {
        Payload::Cid(_) => true,
        Payload::Name(name) => facts.exact_value_name(name),
        _ => false,
    }
}

pub(super) fn strict_exact_nullish_global_safe(il: &Il, interner: &Interner, node: NodeId) -> bool {
    let (NodeKind::Var, Payload::Name(name)) = (il.kind(node), il.node(node).payload) else {
        return false;
    };
    let name = interner.resolve(name);
    let Some(contract) = nullish_global_contract(il.meta.lang, name) else {
        return false;
    };
    !contract.requires_unshadowed || asserted_unshadowed_global_symbol(il, node, contract.name)
}

pub(super) fn strict_exact_rust_option_none_safe(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> bool {
    admitted_rust_option_none_sentinel_at_node(il, interner, node).is_some()
}

pub(super) fn strict_exact_safe_seq(il: &Il, interner: &Interner, node: NodeId) -> bool {
    if let Payload::Name(tag) = il.node(node).payload {
        match interner.resolve(tag) {
            "own_property_guard" => {
                return strict_exact_own_property_guard_seq_safe(il, interner, node);
            }
            "record_guard" => return record_shape_guard_for_node(il, interner, node),
            _ => {}
        }
    }
    seq_surface_contract_for_node(il, interner, node)
        .is_some_and(|contract| contract.exact_tree_safe)
}

fn strict_exact_own_property_guard_seq_safe(il: &Il, interner: &Interner, node: NodeId) -> bool {
    own_property_guard_for_node(il, interner, node)
}

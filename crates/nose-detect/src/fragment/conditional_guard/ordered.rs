//! Ordered effect-sequence recognizers for conditional guard branch blocks.

use super::*;

pub(super) fn loop_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 2)?;
    let mut effects = Vec::new();
    for &kid in kids {
        effects.extend(loop_effect_sites(il, interner, kid)?);
    }
    Some(effects)
}

pub(super) fn mixed_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 2)?;
    if !exactly_one_kid(il, kids, |k| k == NodeKind::Loop) {
        return None;
    }
    if !exactly_one_kid(il, kids, |k| {
        matches!(k, NodeKind::ExprStmt | NodeKind::Assign)
    }) {
        return None;
    }
    let mut effects = Vec::new();
    for &kid in kids {
        match il.kind(kid) {
            NodeKind::Loop => effects.extend(loop_effect_sites(il, interner, kid)?),
            NodeKind::ExprStmt | NodeKind::Assign => {
                effects.push(direct_effect_site(il, interner, kid)?)
            }
            _ => return None,
        }
    }
    Some(effects)
}

pub(super) fn conditional_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 2)?;
    if !kids.iter().all(|&kid| il.kind(kid) == NodeKind::If) {
        return None;
    }
    let mut effects = Vec::new();
    for &kid in kids {
        effects.extend(conditional_direct_effect_sites(il, interner, kid)?);
    }
    Some(effects)
}

pub(super) fn conditional_mixed_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 2)?;
    if !exactly_one_kid(il, kids, |k| k == NodeKind::If) {
        return None;
    }
    if !exactly_one_kid(il, kids, |k| {
        matches!(k, NodeKind::ExprStmt | NodeKind::Assign)
    }) {
        return None;
    }
    let mut effects = Vec::new();
    for &kid in kids {
        match il.kind(kid) {
            NodeKind::If => effects.extend(conditional_direct_effect_sites(il, interner, kid)?),
            NodeKind::ExprStmt | NodeKind::Assign => {
                effects.push(direct_effect_site(il, interner, kid)?)
            }
            _ => return None,
        }
    }
    Some(effects)
}

pub(super) fn loop_conditional_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 2)?;
    if !exactly_one_kid(il, kids, |k| k == NodeKind::Loop) {
        return None;
    }
    if !exactly_one_kid(il, kids, |k| k == NodeKind::If) {
        return None;
    }
    let mut effects = Vec::new();
    for &kid in kids {
        match il.kind(kid) {
            NodeKind::Loop => effects.extend(loop_effect_sites(il, interner, kid)?),
            NodeKind::If => effects.extend(conditional_direct_effect_sites(il, interner, kid)?),
            _ => return None,
        }
    }
    Some(effects)
}

pub(super) fn loop_conditional_mixed_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = block_children_exact_len(il, node, 3)?;
    if !exactly_one_kid(il, kids, |k| k == NodeKind::Loop) {
        return None;
    }
    if !exactly_one_kid(il, kids, |k| k == NodeKind::If) {
        return None;
    }
    if !exactly_one_kid(il, kids, |k| {
        matches!(k, NodeKind::ExprStmt | NodeKind::Assign)
    }) {
        return None;
    }
    let mut effects = Vec::new();
    for &kid in kids {
        match il.kind(kid) {
            NodeKind::Loop => effects.extend(loop_effect_sites(il, interner, kid)?),
            NodeKind::If => effects.extend(conditional_direct_effect_sites(il, interner, kid)?),
            NodeKind::ExprStmt | NodeKind::Assign => {
                effects.push(direct_effect_site(il, interner, kid)?)
            }
            _ => return None,
        }
    }
    Some(effects)
}

pub(super) fn append_effect_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = il.children(node);
    if il.kind(node) != NodeKind::Block || !(2..=5).contains(&kids.len()) {
        return None;
    }
    if !kids
        .iter()
        .all(|&kid| matches!(il.kind(kid), NodeKind::Assign | NodeKind::ExprStmt))
    {
        return None;
    }
    let expected_effects = match kids
        .iter()
        .filter(|&&kid| append_statement(il, interner, kid))
        .count()
    {
        2 if kids.len() <= 4 => 2,
        3 => 3,
        _ => return None,
    };
    let mut effects = Vec::new();
    let mut idx = 0;
    while idx < kids.len() {
        if idx + 2 < kids.len()
            && temp_chain_consumed_by_append(il, interner, kids[idx], kids[idx + 1], kids[idx + 2])
        {
            effects.push(EffectSite::observable(Effect::Append));
            idx += 3;
            continue;
        }
        if idx + 1 < kids.len()
            && temp_assignment_consumed_by_append(il, interner, kids[idx], kids[idx + 1])
        {
            effects.push(EffectSite::observable(Effect::Append));
            idx += 2;
            continue;
        }
        if append_statement(il, interner, kids[idx]) {
            effects.push(EffectSite::observable(Effect::Append));
            idx += 1;
            continue;
        }
        return None;
    }
    (effects.len() == expected_effects).then_some(effects)
}

pub(super) fn index_assignment_effect_sequence(il: &Il, node: NodeId) -> Option<Vec<EffectSite>> {
    let kids = il.children(node);
    if il.kind(node) != NodeKind::Block || !(2..=5).contains(&kids.len()) {
        return None;
    }
    if !kids.iter().all(|&kid| il.kind(kid) == NodeKind::Assign) {
        return None;
    }
    let expected_effects = match kids
        .iter()
        .filter(|&&kid| index_assignment(il, kid))
        .count()
    {
        2 if kids.len() <= 4 => 2,
        3 => 3,
        _ => return None,
    };
    let mut effects = Vec::new();
    let mut idx = 0;
    while idx < kids.len() {
        if idx + 2 < kids.len()
            && temp_chain_consumed_by_index_assignment(il, kids[idx], kids[idx + 1], kids[idx + 2])
        {
            effects.push(EffectSite::observable(Effect::IndexWrite));
            idx += 3;
            continue;
        }
        if idx + 1 < kids.len()
            && temp_assignment_consumed_by_index_assignment(il, kids[idx], kids[idx + 1])
        {
            effects.push(EffectSite::observable(Effect::IndexWrite));
            idx += 2;
            continue;
        }
        if index_assignment(il, kids[idx]) {
            effects.push(EffectSite::observable(Effect::IndexWrite));
            idx += 1;
            continue;
        }
        return None;
    }
    (effects.len() == expected_effects).then_some(effects)
}

pub(super) fn self_field_assignment_sequence(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<Vec<EffectSite>> {
    let kids = il.children(node);
    if il.kind(node) != NodeKind::Block || !(2..=3).contains(&kids.len()) {
        return None;
    }
    kids.iter()
        .map(|&kid| self_field_assignment_site(il, interner, kid))
        .collect()
}

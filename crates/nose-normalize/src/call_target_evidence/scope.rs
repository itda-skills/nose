use super::*;

pub(super) fn parent_map(il: &Il) -> Vec<Option<NodeId>> {
    let mut parents = vec![None; il.nodes.len()];
    for (idx, _) in il.nodes.iter().enumerate() {
        let parent = NodeId(idx as u32);
        for &child in il.children(parent) {
            if let Some(slot) = parents.get_mut(child.0 as usize) {
                *slot = Some(parent);
            }
        }
    }
    parents
}

pub(super) fn is_top_level_function_root(
    il: &Il,
    parents: &[Option<NodeId>],
    root: NodeId,
) -> bool {
    parents
        .get(root.0 as usize)
        .copied()
        .flatten()
        .is_some_and(|parent| il.kind(parent) == NodeKind::Module)
}

pub(super) fn function_unit_names(il: &Il) -> FxHashMap<u32, Symbol> {
    let mut names = FxHashMap::default();
    for unit in &il.units {
        if unit.kind == UnitKind::Function {
            if let Some(name) = unit.name {
                names.insert(unit.root.0, name);
            }
        }
    }
    names
}

pub(super) fn scope_bound_symbols(
    il: &Il,
    function_names: &FxHashMap<u32, Symbol>,
) -> FxHashMap<u32, FxHashSet<Symbol>> {
    let mut out = FxHashMap::default();
    collect_scope_bound_symbols(il, il.root, function_names, &mut out);
    out
}

fn collect_scope_bound_symbols(
    il: &Il,
    node: NodeId,
    function_names: &FxHashMap<u32, Symbol>,
    out: &mut FxHashMap<u32, FxHashSet<Symbol>>,
) {
    if is_scope(il.kind(node)) {
        let mut bound = FxHashSet::default();
        collect_bound_in_scope(il, node, true, il.kind(node), function_names, &mut bound);
        out.insert(node.0, bound);
    }
    for &child in il.children(node) {
        collect_scope_bound_symbols(il, child, function_names, out);
    }
}

fn collect_bound_in_scope(
    il: &Il,
    node: NodeId,
    is_root: bool,
    scope_kind: NodeKind,
    function_names: &FxHashMap<u32, Symbol>,
    out: &mut FxHashSet<Symbol>,
) {
    if !is_root && is_scope(il.kind(node)) {
        if scope_kind != NodeKind::Module {
            if let Some(&name) = function_names.get(&node.0) {
                out.insert(name);
            }
        }
        return;
    }
    match il.kind(node) {
        NodeKind::Param => {
            if let Payload::Name(name) = il.node(node).payload {
                out.insert(name);
            }
        }
        NodeKind::Assign => {
            if let Some(&lhs) = il.children(node).first() {
                collect_target_symbols(il, lhs, out);
            }
        }
        NodeKind::Loop if matches!(il.node(node).payload, Payload::Loop(LoopKind::ForEach)) => {
            if let Some(&pattern) = il.children(node).first() {
                collect_target_symbols(il, pattern, out);
            }
        }
        _ => {}
    }
    for &child in il.children(node) {
        collect_bound_in_scope(il, child, false, scope_kind, function_names, out);
    }
}

fn collect_target_symbols(il: &Il, node: NodeId, out: &mut FxHashSet<Symbol>) {
    match il.kind(node) {
        NodeKind::Var => {
            if let Payload::Name(name) = il.node(node).payload {
                out.insert(name);
            }
        }
        NodeKind::Seq => {
            for &child in il.children(node) {
                collect_target_symbols(il, child, out);
            }
        }
        _ => {}
    }
}

pub(super) fn is_scope(kind: NodeKind) -> bool {
    matches!(kind, NodeKind::Module | NodeKind::Func | NodeKind::Lambda)
}

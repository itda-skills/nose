use super::*;

pub(super) fn unique_direct_function_targets(
    il: &Il,
    interner: &Interner,
) -> FxHashMap<Symbol, DirectFunctionTarget> {
    let parents = parent_map(il);
    let mut targets = FxHashMap::default();
    let mut ambiguous = FxHashSet::default();
    // Names rebound at module scope from inside another function (`global name; name =
    // ...`): the runtime binding is no longer the `def` body, so the name is not a
    // DirectFunction target. Precise — a local `name = x` (no `global`) carries no fact
    // and stays a valid target (#302). Empty for non-Python and the common case.
    let rebound = nose_semantics::module_rebound_symbols(il, interner);
    for unit in &il.units {
        if unit.kind != UnitKind::Function || !is_top_level_function_root(il, &parents, unit.root) {
            continue;
        }
        let Some(name) = unit.name else { continue };
        if ambiguous.contains(&name) {
            continue;
        }
        // A decorated `def` binds `decorator(f)`, not the lowered body (coevo series 6,
        // S2-A); a `global`-reassigned name binds whatever was last assigned, not its
        // `def` body (#302). Both fail closed: no DirectFunction evidence, so the inline,
        // the content-keyed exact admission, and the behavioral oracle all stay opaque.
        if nose_semantics::decorated_definition_at_node(il, unit.root) || rebound.contains(&name) {
            continue;
        }
        let target = DirectFunctionTarget {
            root: unit.root,
            name_hash: interner.symbol_hash(name),
        };
        if targets.insert(name, target).is_some() {
            targets.remove(&name);
            ambiguous.insert(name);
        }
    }
    targets
}

pub(super) fn collect_call_targets(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    scope_stack: &mut Vec<NodeId>,
    targets: &FxHashMap<Symbol, DirectFunctionTarget>,
    scope_bound: &FxHashMap<u32, FxHashSet<Symbol>>,
    out: &mut Vec<(NodeId, DirectFunctionTarget)>,
) {
    let entered_scope = is_scope(il.kind(node));
    if entered_scope {
        scope_stack.push(node);
    }
    if let Some(target) = direct_call_target(il, interner, node, scope_stack, targets, scope_bound)
    {
        out.push((node, target));
    }
    for &child in il.children(node) {
        collect_call_targets(il, interner, child, scope_stack, targets, scope_bound, out);
    }
    if entered_scope {
        scope_stack.pop();
    }
}

fn direct_call_target(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    scope_stack: &[NodeId],
    targets: &FxHashMap<Symbol, DirectFunctionTarget>,
    scope_bound: &FxHashMap<u32, FxHashSet<Symbol>>,
) -> Option<DirectFunctionTarget> {
    if il.kind(node) != NodeKind::Call || !matches!(il.node(node).payload, Payload::None) {
        return None;
    }
    let callee = *il.children(node).first()?;
    if il.kind(callee) != NodeKind::Var {
        return None;
    }
    if var_has_symbol_identity_evidence(il, interner, callee) {
        return None;
    }
    let Payload::Name(name) = il.node(callee).payload else {
        return None;
    };
    if scope_stack.iter().any(|scope| {
        scope_bound
            .get(&scope.0)
            .is_some_and(|bound| bound.contains(&name))
    }) {
        return None;
    }
    targets.get(&name).copied()
}

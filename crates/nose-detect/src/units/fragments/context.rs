use super::super::tree::{collect_cids, parent_of};
use crate::il_utils::node_mentions_any_cid;
use nose_il::{Il, Interner, NodeId, NodeKind, Payload};
use nose_semantics::{
    admitted_builtin_semantics_at_call, opaque_argument_escape_args,
    receiver_mutation_call_receiver,
};
use rustc_hash::FxHashSet;

pub(crate) fn top_level_statement_fragment_context_safe(
    il: &Il,
    node: NodeId,
    parents: &[Option<NodeId>],
    interner: &Interner,
) -> bool {
    let Some(body) = parent_of(parents, node) else {
        return false;
    };
    if il.kind(body) != NodeKind::Block {
        return false;
    }
    let Some(func) = parent_of(parents, body) else {
        return false;
    };
    if il.kind(func) != NodeKind::Func {
        return false;
    }

    let mut used = FxHashSet::default();
    collect_cids(il, node, &mut used);
    if used.is_empty() {
        return true;
    }

    let mut blocked = used;
    for &stmt in il.children(body) {
        if stmt == node {
            return true;
        }
        if previous_statement_invalidates_fragment_inputs(il, interner, stmt, &mut blocked) {
            return false;
        }
    }
    false
}

fn previous_statement_invalidates_fragment_inputs(
    il: &Il,
    interner: &Interner,
    stmt: NodeId,
    blocked: &mut FxHashSet<u32>,
) -> bool {
    if assignment_aliases_or_mutates_blocked_cid(il, stmt, blocked) {
        return true;
    }
    if call_may_mutate_blocked_cid(il, interner, stmt, blocked) {
        return true;
    }
    for &child in il.children(stmt) {
        if previous_statement_invalidates_fragment_inputs(il, interner, child, blocked) {
            return true;
        }
    }
    false
}

fn assignment_aliases_or_mutates_blocked_cid(
    il: &Il,
    node: NodeId,
    blocked: &mut FxHashSet<u32>,
) -> bool {
    if il.kind(node) != NodeKind::Assign {
        return false;
    }
    let kids = il.children(node);
    if kids.len() != 2 {
        return false;
    }
    if node_mentions_any_cid(il, kids[0], blocked) {
        return true;
    }
    if !node_mentions_any_cid(il, kids[1], blocked) {
        return false;
    }
    if let (NodeKind::Var, Payload::Cid(cid)) = (il.kind(kids[0]), il.node(kids[0]).payload) {
        blocked.insert(cid);
    }
    false
}

pub(in crate::units) fn call_may_mutate_blocked_cid(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    blocked: &FxHashSet<u32>,
) -> bool {
    if il.kind(node) != NodeKind::Call {
        return false;
    }
    if let Some(receiver) = receiver_mutation_call_receiver(il, interner, node) {
        return node_mentions_any_cid(il, receiver, blocked);
    }
    if let Payload::Builtin(builtin) = il.node(node).payload {
        if admitted_builtin_semantics_at_call(il, node, builtin) {
            return false;
        }
        return il
            .children(node)
            .iter()
            .any(|&arg| node_mentions_any_cid(il, arg, blocked));
    }
    opaque_argument_escape_args(il, node).is_some_and(|args| {
        args.iter()
            .any(|&arg| node_mentions_any_cid(il, arg, blocked))
    })
}

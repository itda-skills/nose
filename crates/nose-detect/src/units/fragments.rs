mod assignment;
mod context;
mod loop_effect;
mod sequence;

use super::tree::subtree_spans_within;
use super::UnitRoot;
use crate::fragment::FragmentKind;
pub(in crate::units) use assignment::strict_exact_self_field_fragment_safe;
use assignment::{exact_assignment_fragment_kind, exact_function_body_self_field_fragment_root};
#[cfg(test)]
pub(in crate::units) use context::call_may_mutate_blocked_cid;
pub(crate) use context::top_level_statement_fragment_context_safe;
use loop_effect::exact_loop_effect_fragment_root;
use nose_il::{Il, Interner, NodeId, NodeKind, UnitKind, UnitOrigin};
use sequence::empty_or_single_direct_exact_statement_block;

/// Collect sub-function blocks and exact statement fragments in one DFS.
pub(super) fn collect_extra_unit_roots(
    il: &Il,
    root: NodeId,
    parents: &[Option<NodeId>],
    interner: &Interner,
    out: &mut Vec<UnitRoot>,
) {
    let mut block_roots = Vec::new();
    let mut exact_roots = Vec::new();
    collect_extra_unit_root_candidates(
        il,
        root,
        parents,
        interner,
        true,
        &mut block_roots,
        &mut exact_roots,
    );
    out.extend(block_roots);
    for (root, kind) in exact_roots {
        push_or_upgrade_exact_fragment_root(out, root, kind);
    }
}

fn collect_extra_unit_root_candidates(
    il: &Il,
    node: NodeId,
    parents: &[Option<NodeId>],
    interner: &Interner,
    exact_enabled: bool,
    block_roots: &mut Vec<UnitRoot>,
    exact_roots: &mut Vec<(NodeId, FragmentKind)>,
) {
    let is_statement_if = il.kind(node) == NodeKind::If
        && il
            .children(node)
            .iter()
            .skip(1)
            .any(|&child| il.kind(child) == NodeKind::Block);
    if matches!(il.kind(node), NodeKind::Loop | NodeKind::Try) || is_statement_if {
        block_roots.push(UnitRoot {
            root: node,
            kind: UnitKind::Block,
            name: None,
            origin: UnitOrigin::unknown(),
            fragment_kind: None,
        });
    }
    if exact_enabled && exact_fragment_candidate_node(il, node) {
        if let Some(contract) =
            crate::fragment::recognize::recognize_contract(il, node, parents, interner)
        {
            let kind = contract.kind;
            // The contract path is the production authority. Keep the old predicate
            // matrix as a debug-only differential guard while it remains in-tree.
            debug_assert!(
                exact_statement_fragment_root(il, node, parents, interner)
                    .is_some_and(|predicate_kind| predicate_kind == kind),
                "predicate path must agree with contract-first fragment production for {kind:?}"
            );
            exact_roots.push((node, kind));
        }
    }
    let child_exact_enabled = exact_enabled && il.kind(node) != NodeKind::Lambda;
    for &c in il.children(node) {
        collect_extra_unit_root_candidates(
            il,
            c,
            parents,
            interner,
            child_exact_enabled,
            block_roots,
            exact_roots,
        );
    }
}

fn exact_fragment_candidate_node(il: &Il, node: NodeId) -> bool {
    matches!(
        il.kind(node),
        NodeKind::Return
            | NodeKind::Throw
            | NodeKind::Assign
            | NodeKind::ExprStmt
            | NodeKind::If
            | NodeKind::Loop
            | NodeKind::Block
    )
}

fn push_or_upgrade_exact_fragment_root(out: &mut Vec<UnitRoot>, root: NodeId, kind: FragmentKind) {
    if let Some(existing) = out.iter_mut().find(|candidate| candidate.root == root) {
        existing.fragment_kind = Some(kind);
    } else {
        out.push(UnitRoot {
            root,
            kind: UnitKind::Block,
            name: None,
            origin: UnitOrigin::unknown(),
            fragment_kind: Some(kind),
        });
    }
}

/// Classify `node` as an exact sub-function fragment root, or `None` if it is not one.
///
/// `Some(kind)` is returned for exactly the nodes the previous boolean recognizer
/// accepted (`true`); the [`FragmentKind`] names which recognizer branch matched. This
/// is the single dispatch that lowers the standalone shape predicates into the fragment
/// substrate (issue #33, step 1).
pub(crate) fn exact_statement_fragment_root(
    il: &Il,
    node: NodeId,
    parents: &[Option<NodeId>],
    interner: &Interner,
) -> Option<FragmentKind> {
    if !subtree_spans_within(il, node, il.node(node).span) {
        return None;
    }
    if exact_function_body_self_field_fragment_root(il, interner, parents, node) {
        return Some(FragmentKind::SelfFieldBody);
    }
    if !top_level_statement_fragment_context_safe(il, node, parents, interner) {
        return None;
    }
    let kids = il.children(node);
    match il.kind(node) {
        NodeKind::Return => (kids.len() == 1
            && !matches!(il.kind(kids[0]), NodeKind::Var | NodeKind::Lit))
        .then_some(FragmentKind::DirectReturn),
        NodeKind::Throw => (kids.len() == 1
            && !matches!(il.kind(kids[0]), NodeKind::Var | NodeKind::Lit))
        .then_some(FragmentKind::DirectThrow),
        NodeKind::Assign => exact_assignment_fragment_kind(il, interner, node),
        NodeKind::ExprStmt => {
            exact_expr_statement_fragment_root(il, node).then_some(FragmentKind::ExprEffect)
        }
        NodeKind::If => exact_conditional_fragment_root(il, interner, node)
            .then_some(FragmentKind::ConditionalGuard),
        NodeKind::Loop => {
            exact_loop_effect_fragment_root(il, interner, node).then_some(FragmentKind::LoopEffect)
        }
        _ => None,
    }
}

pub(super) fn exact_expr_statement_fragment_root(il: &Il, node: NodeId) -> bool {
    let kids = il.children(node);
    kids.len() == 1
        && !matches!(
            il.kind(kids[0]),
            NodeKind::Return
                | NodeKind::Throw
                | NodeKind::Break
                | NodeKind::Continue
                | NodeKind::Var
                | NodeKind::Lit
        )
}

pub(super) fn exact_conditional_fragment_root(il: &Il, interner: &Interner, node: NodeId) -> bool {
    let kids = il.children(node);
    if !(kids.len() == 2 || kids.len() == 3) {
        return false;
    }
    let mut has_exact_statement = false;
    for &branch in kids.iter().skip(1) {
        let Some(branch_has_exact_statement) =
            empty_or_single_direct_exact_statement_block(il, interner, branch)
        else {
            return false;
        };
        has_exact_statement |= branch_has_exact_statement;
    }
    has_exact_statement
}

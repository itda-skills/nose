use nose_il::{Il, NodeId, NodeKind, Payload, Span};
use rustc_hash::FxHashSet;

pub(super) fn collect_cids(il: &Il, node: NodeId, out: &mut FxHashSet<u32>) {
    if let (NodeKind::Var, Payload::Cid(cid)) = (il.kind(node), il.node(node).payload) {
        out.insert(cid);
    }
    for &child in il.children(node) {
        collect_cids(il, child, out);
    }
}

pub(super) fn parent_of(parents: &[Option<NodeId>], node: NodeId) -> Option<NodeId> {
    parents.get(node.0 as usize).copied().flatten()
}

pub(crate) fn build_parent_index(il: &Il) -> Vec<Option<NodeId>> {
    let mut parents = vec![None; il.nodes.len()];
    for idx in 0..il.nodes.len() {
        let parent = NodeId(idx as u32);
        for &child in il.children(parent) {
            if let Some(slot) = parents.get_mut(child.0 as usize) {
                *slot = Some(parent);
            }
        }
    }
    parents
}

pub(crate) fn subtree_spans_within(il: &Il, node: NodeId, span: Span) -> bool {
    let node_span = il.node(node).span;
    if node_span.file != span.file
        || node_span.start_line < span.start_line
        || node_span.end_line > span.end_line
        || node_span.start_byte < span.start_byte
        || node_span.end_byte > span.end_byte
    {
        return false;
    }
    il.children(node)
        .iter()
        .all(|&child| subtree_spans_within(il, child, span))
}

/// Pre-order DFS collecting all descendant node ids of `root` (inclusive).
pub(super) fn collect_pre(il: &Il, root: NodeId, out: &mut Vec<NodeId>) {
    out.push(root);
    for &c in il.children(root) {
        collect_pre(il, c, out);
    }
}

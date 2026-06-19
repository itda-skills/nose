use crate::il::Il;
use crate::intern::Symbol;
use crate::node::{Node, NodeId, NodeKind, Payload};
use crate::span::{FileId, FileMeta, Span};
use crate::unit::Unit;

/// Incrementally builds an [`Il`] arena via post-order construction: lower the
/// children first, collect their ids, then [`add`](IlBuilder::add) the parent.
pub struct IlBuilder {
    nodes: Vec<Node>,
    edges: Vec<NodeId>,
    file: FileId,
}

impl IlBuilder {
    pub fn new(file: FileId) -> Self {
        IlBuilder {
            nodes: Vec::new(),
            edges: Vec::new(),
            file,
        }
    }

    pub fn with_capacity(file: FileId, nodes: usize, edges: usize) -> Self {
        IlBuilder {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
            file,
        }
    }

    pub fn file(&self) -> FileId {
        self.file
    }

    /// Allocate a node with the given children (copied into the edge arena).
    pub fn add(
        &mut self,
        kind: NodeKind,
        payload: Payload,
        span: Span,
        children: &[NodeId],
    ) -> NodeId {
        let child_start = self.edges.len() as u32;
        self.edges.extend_from_slice(children);
        let child_len = children.len() as u32;
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node {
            kind,
            payload,
            span,
            child_start,
            child_len,
        });
        id
    }

    /// Number of nodes allocated so far.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0 as usize]
    }

    #[inline]
    pub fn kind(&self, id: NodeId) -> NodeKind {
        self.node(id).kind
    }

    #[inline]
    pub fn payload(&self, id: NodeId) -> Payload {
        self.node(id).payload
    }

    #[inline]
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        let n = self.node(id);
        let s = n.child_start as usize;
        &self.edges[s..s + n.child_len as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn finish(
        self,
        root: NodeId,
        meta: FileMeta,
        units: Vec<Unit>,
        cid_names: Vec<Symbol>,
    ) -> Il {
        Il::from_parts(
            self.nodes, self.edges, root, self.file, meta, units, cid_names,
        )
    }
}

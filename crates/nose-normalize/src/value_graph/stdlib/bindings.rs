use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn proven_local_collection_binding_value(
        &mut self,
        expr: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        let value = self.proven_local_binding_initializer_value(expr, env, |domain| {
            domain.is_collection_or_set()
        })?;
        self.proven_collection_value(value)
    }
    pub(super) fn proven_local_map_binding_value(
        &mut self,
        expr: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        let value =
            self.proven_local_binding_initializer_value(expr, env, |domain| domain.is_map())?;
        self.proven_map_value(value)
    }
    fn proven_local_binding_initializer_value(
        &mut self,
        expr: NodeId,
        env: &FxHashMap<u32, ValueId>,
        accepts_domain: impl FnOnce(DomainEvidence) -> bool,
    ) -> Option<ValueId> {
        if self.il.kind(expr) != NodeKind::Var {
            return None;
        }
        let Payload::Cid(cid) = self.il.node(expr).payload else {
            return None;
        };
        let (lhs, rhs) = self.local_binding_initializer(cid, expr)?;
        if !nose_semantics::domain_evidence_for_binding_lhs(self.il, self.interner, lhs)
            .is_some_and(accepts_domain)
        {
            return None;
        }
        if self.node_contains_cid(rhs, cid) {
            return None;
        }
        Some(self.eval(rhs, env))
    }
    fn local_binding_initializer(&self, cid: u32, use_node: NodeId) -> Option<(NodeId, NodeId)> {
        let mut rhs = None;
        for (idx, node) in self.il.nodes.iter().enumerate() {
            if node.kind != NodeKind::Assign {
                continue;
            }
            let assign = NodeId(idx as u32);
            let kids = self.il.children(assign);
            if kids.len() != 2 {
                continue;
            }
            if self.node_refers_to_cid(kids[0], cid) {
                if node.span.end_byte > self.il.node(use_node).span.start_byte {
                    continue;
                }
                if rhs.is_some() {
                    return None;
                }
                rhs = Some((kids[0], kids[1]));
            } else if self.node_contains_cid(kids[0], cid) {
                return None;
            }
        }
        rhs
    }
}

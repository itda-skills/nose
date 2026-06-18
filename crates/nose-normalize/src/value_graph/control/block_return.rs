use super::super::*;

impl<'a> Builder<'a> {
    /// Walk a (possibly nested) block, applying assignments to `env`, and return the
    /// value of the first `return` expression reached.
    pub(in crate::value_graph) fn eval_block_return(
        &mut self,
        node: NodeId,
        env: &mut FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        match self.il.kind(node) {
            NodeKind::Block => {
                let kids = self.il.children(node).to_vec();
                let n = kids.len();
                for (i, &s) in kids.iter().enumerate() {
                    // An explicit `return` anywhere wins; a let-binding binds; the LAST
                    // statement is the *implicit* return value (Rust closures and Ruby
                    // blocks have no `return` — their trailing expression is the result).
                    if let Some(v) = self.eval_block_return(s, env) {
                        return Some(v);
                    }
                    if i + 1 == n {
                        if let NodeKind::ExprStmt = self.il.kind(s) {
                            return self.il.children(s).first().map(|&e| self.eval(e, env));
                        }
                    }
                }
                None
            }
            NodeKind::Return => self.il.children(node).first().map(|&e| self.eval(e, env)),
            NodeKind::Assign => {
                let kids = self.il.children(node).to_vec();
                if kids.len() == 2 && self.il.kind(kids[0]) == NodeKind::Var {
                    if let Payload::Cid(c) = self.il.node(kids[0]).payload {
                        let rhs = self.eval(kids[1], env);
                        env.insert(c, rhs);
                    }
                }
                None
            }
            // A bare-expression lambda body (`|a, v| a + v`) — its value is the result.
            NodeKind::ExprStmt => self.il.children(node).first().map(|&e| self.eval(e, env)),
            _ => Some(self.eval(node, env)),
        }
    }
}

use super::super::*;

impl<'a> Builder<'a> {
    pub(super) fn is_import_namespace_expr(
        &mut self,
        expr: NodeId,
        module: &str,
        env: &FxHashMap<u32, ValueId>,
    ) -> bool {
        let value = self.eval(expr, env);
        self.is_import_namespace_value(value, module)
    }
    pub(super) fn is_import_namespace_value(&self, value: ValueId, module: &str) -> bool {
        let node = &self.nodes[value as usize];
        matches!(
            node.op,
            ValOp::ImportNamespace { module_hash }
                if module_hash == stable_symbol_hash(module)
        )
    }
    #[cfg(test)]
    pub(in crate::value_graph) fn is_import_binding_value(
        &self,
        value: ValueId,
        module: &str,
        exported: &str,
    ) -> bool {
        let node = &self.nodes[value as usize];
        matches!(
            node.op,
            ValOp::ImportBinding {
                module_hash,
                exported_hash,
            } if module_hash == stable_symbol_hash(module)
                && exported_hash == stable_symbol_hash(exported)
        )
    }
    pub(in crate::value_graph) fn import_fact_value(&mut self, expr: NodeId) -> Option<ValueId> {
        let fact = import_fact_evidence_rhs(self.il, expr)?;
        match fact.kind {
            ImportFactKind::Namespace => Some(self.mk(
                ValOp::ImportNamespace {
                    module_hash: fact.module_hash,
                },
                vec![],
            )),
            ImportFactKind::Binding => Some(self.mk(
                ValOp::ImportBinding {
                    module_hash: fact.module_hash,
                    exported_hash: fact.exported_hash?,
                },
                vec![],
            )),
        }
    }
    pub(super) fn file_imports_namespace(&self, expr: NodeId, module: &str) -> bool {
        semantics(self.il.meta.lang)
            .modules()
            .go_import_namespace_facts()
            && imported_namespace_symbol(self.il, self.interner, expr, module)
    }
}

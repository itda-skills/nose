use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn eval_proven_collection_membership_call(
        &mut self,
        expr: NodeId,
        kids: &[NodeId],
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        let admitted = admitted_library_method_call_at_call(self.il, self.interner, expr)?;
        let result = admitted.contract.result;
        if result.semantic != MethodSemanticContract::Builtin(Builtin::Contains) {
            return None;
        }

        if result.args == MethodBuiltinArgs::FirstThenReceiver
            && matches!(
                result.receiver,
                MethodReceiverContract::ExactCollection
                    | MethodReceiverContract::ExactCollectionOrMap
                    | MethodReceiverContract::ExactCollectionOrJavaKeySet
                    | MethodReceiverContract::ExactSetOrMap
            )
            && kids.len() == 2
        {
            let receiver = admitted.receiver?;
            let element = self.eval(kids[1], env);
            if matches!(
                result.receiver,
                MethodReceiverContract::ExactCollectionOrMap
                    | MethodReceiverContract::ExactCollectionOrJavaKeySet
            ) {
                if let Some(map) = self.proven_map_key_view_expr(receiver, env) {
                    return Some(self.mk(ValOp::Bin(Op::In as u32), vec![element, map]));
                }
            }
            let receiver_value = self.eval(receiver, env);
            if let Some(collection) = self
                .proven_collection_value(receiver_value)
                .or_else(|| self.proven_local_collection_binding_value(receiver, env))
            {
                let collection = self.canonical_membership_collection_value(collection);
                return Some(self.mk(ValOp::Bin(Op::In as u32), vec![element, collection]));
            }
            let receiver_param_safe = match result.receiver {
                MethodReceiverContract::ExactSetOrMap => self.is_set_param_expr(receiver),
                _ => self.is_collection_param_expr(receiver),
            };
            if receiver_param_safe {
                let collection = self.eval_membership_collection(receiver, env);
                return Some(self.mk(ValOp::Bin(Op::In as u32), vec![element, collection]));
            }
            None
        } else if result.args == MethodBuiltinArgs::GoSliceContains && kids.len() == 3 {
            let receiver = admitted.receiver?;
            let MethodReceiverContract::ImportedNamespace(module) = result.receiver else {
                return None;
            };
            if !self.is_import_namespace_expr(receiver, module, env)
                && !self.file_imports_namespace(receiver, module)
            {
                return None;
            }
            let element = self.eval(kids[2], env);
            let collection = if self.is_collection_param_expr(kids[1]) {
                self.eval_membership_collection(kids[1], env)
            } else {
                self.proven_collection_expr(kids[1], env)?
            };
            let collection = self.canonical_membership_collection_value(collection);
            Some(self.mk(ValOp::Bin(Op::In as u32), vec![element, collection]))
        } else {
            None
        }
    }
}

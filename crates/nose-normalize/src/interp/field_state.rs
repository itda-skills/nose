use super::*;

impl<'a> Interp<'a> {
    pub(super) fn exact_field_place(&self, node: NodeId) -> Option<FieldPlace> {
        if self.il.kind(node) == NodeKind::Var && exact_java_this_var(self.il, self.interner, node)
        {
            Some(FieldPlace::SelfReceiver)
        } else {
            None
        }
    }

    pub(super) fn exact_field_write_key(&self, assign: NodeId, target: NodeId) -> Option<FieldKey> {
        if !exact_self_field_write_assignment(self.il, self.interner, assign) {
            return None;
        }
        self.exact_field_key(target)
    }

    pub(super) fn exact_field_key(&self, target: NodeId) -> Option<FieldKey> {
        if self.il.kind(target) != NodeKind::Field {
            return None;
        }
        if !exact_java_this_field(self.il, self.interner, target) {
            return None;
        }
        let Payload::Name(field) = self.il.node(target).payload else {
            return None;
        };
        let receiver = self.il.children(target).first().copied()?;
        let receiver = self.exact_field_place(receiver)?;
        Some(FieldKey {
            receiver,
            field: stable_symbol_hash(self.interner.resolve(field)),
        })
    }

    pub(super) fn field_receiver_errored(
        &mut self,
        receiver: NodeId,
        env: &mut FxHashMap<u32, Value>,
    ) -> R<bool> {
        if exact_java_this_var(self.il, self.interner, receiver) {
            return Ok(false);
        }
        Ok(matches!(self.eval(receiver, env)?, Value::Err))
    }
}

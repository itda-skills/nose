use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn library_api_span_call(
        &self,
        value: ValueId,
        callee: ValueId,
        receiver: Option<ValueId>,
        arg_count: usize,
    ) -> LibraryApiSpanCall {
        LibraryApiSpanCall {
            call_span: self.node_span[value as usize],
            callee_span: self.library_api_value_span(callee),
            receiver_span: self.library_api_receiver_query_span(value, callee, receiver),
            arg_count,
        }
    }
    fn library_api_value_span(&self, value: ValueId) -> Option<Span> {
        match self.nodes[value as usize].op {
            ValOp::ImportBinding { .. } | ValOp::ImportNamespace { .. } => None,
            _ => self.node_span[value as usize],
        }
    }
    fn library_api_receiver_query_span(
        &self,
        value: ValueId,
        callee: ValueId,
        receiver: Option<ValueId>,
    ) -> Option<Span> {
        let receiver_span = receiver.and_then(|receiver| self.library_api_value_span(receiver))?;
        let Some(call_span) = self.node_span[value as usize] else {
            return Some(receiver_span);
        };
        let Some(callee_span) = self.library_api_value_span(callee) else {
            return Some(receiver_span);
        };
        if self
            .source_call_receiver_span(call_span, callee_span)
            .is_some_and(|source_receiver_span| source_receiver_span != receiver_span)
        {
            None
        } else {
            Some(receiver_span)
        }
    }
    fn source_call_receiver_span(&self, call_span: Span, callee_span: Span) -> Option<Span> {
        self.il.nodes.iter().enumerate().find_map(|(idx, node)| {
            if node.kind != NodeKind::Call || node.span != call_span {
                return None;
            }
            let call = NodeId(idx as u32);
            let callee = self.il.children(call).first().copied()?;
            if self.il.node(callee).span != callee_span {
                return None;
            }
            self.il
                .children(callee)
                .first()
                .map(|&receiver| self.il.node(receiver).span)
        })
    }
}

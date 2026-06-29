use super::*;

impl<'a> Lowering<'a> {
    pub(super) fn static_global_method_api_contract(
        &self,
        callee: NodeId,
        arg_count: usize,
    ) -> Option<LibraryApiEvidencePlan> {
        let (receiver_node, receiver, method) = self.static_member_callee(callee)?;
        let contract = library_js_array_is_array_contract(self.lang, receiver, method, arg_count)
            .map(|contract| {
                (
                    contract.id,
                    contract.callee,
                    contract.result.qualified_path,
                    contract.result.requires_unshadowed_receiver,
                    contract.result.receiver,
                    contract.pack_id,
                    JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID,
                    None,
                )
            })
            .or_else(|| {
                library_map_key_view_wrapper_contract(self.lang, receiver, method, arg_count).map(
                    |contract| {
                        (
                            contract.id,
                            contract.callee,
                            contract.result.qualified_path,
                            true,
                            contract.result.receiver,
                            contract.pack_id,
                            JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID,
                            Some(library_map_key_view_wrapper_result_domain(contract)),
                        )
                    },
                )
            })
            .or_else(|| {
                library_promise_resolve_contract(self.lang, receiver, method, arg_count).map(
                    |contract| {
                        (
                            contract.id,
                            contract.callee,
                            contract.result.qualified_path,
                            true,
                            contract.result.receiver,
                            contract.pack_id,
                            JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
                            Some(contract.result.result_domain),
                        )
                    },
                )
            })
            .or_else(|| {
                library_promise_aggregate_contract(self.lang, receiver, method, arg_count).map(
                    |contract| {
                        (
                            contract.id,
                            contract.callee,
                            contract.result.qualified_path,
                            true,
                            contract.result.receiver,
                            contract.pack_id,
                            JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
                            Some(contract.result.result_domain),
                        )
                    },
                )
            })?;
        let qualified = self.qualified_global_evidence_id(callee, contract.2)?;
        let mut dependencies = vec![qualified];
        if contract.3 {
            dependencies.push(self.unshadowed_global_evidence_id(receiver_node, contract.4)?);
        }
        Some(LibraryApiEvidencePlan {
            id: contract.0,
            callee: contract.1,
            dependencies,
            pack_id: contract.5,
            rule: contract.6,
            result_domain: contract.7,
        })
    }
}

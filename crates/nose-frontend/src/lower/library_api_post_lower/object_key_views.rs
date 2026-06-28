use super::*;

pub(super) fn record_post_lower_object_key_view_library_api(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
) -> bool {
    let Some((callee, receiver_node, receiver, method, arg_count)) =
        post_lower_static_global_call_parts(il, interner, call)
    else {
        return false;
    };
    if receiver != "Object" || method != "keys" {
        return false;
    }
    let Some(contract) =
        library_object_key_view_contract(il.meta.lang, "Object", "keys", arg_count)
    else {
        return false;
    };
    let Some(qualified) = post_lower_qualified_global_symbol_evidence_id(il, callee, "Object.keys")
    else {
        return false;
    };
    let Some(root_dependency) =
        post_lower_unshadowed_symbol_evidence_id(il, receiver_node, "Object")
    else {
        return false;
    };
    let Some(mut dependencies) =
        js_object_key_view_argument_dependency_ids_for_call(il, interner, call)
    else {
        return false;
    };
    dependencies.insert(0, root_dependency);
    dependencies.insert(0, qualified);
    let api = post_lower_library_api_evidence_with_pack_id(
        il,
        call,
        contract.id,
        contract.callee,
        arg_count,
        contract.pack_id,
        MAP_KEY_VIEW_PROTOCOL_PRODUCER_ID,
        dependencies,
    );
    post_lower_record_library_api_result_domain(il, call, Some(DomainEvidence::Array), api);
    true
}

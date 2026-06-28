use super::*;

pub(super) fn record_post_lower_ruby_static_member_library_api(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
) -> bool {
    let kids = il.children(call);
    let Some((&callee, args)) = kids.split_first() else {
        return false;
    };
    let arg_count = args.len();
    if il.kind(callee) != NodeKind::Field {
        return false;
    }
    let Payload::Name(method) = il.node(callee).payload else {
        return false;
    };
    let method = interner.resolve(method);
    let Some(&receiver) = il.children(callee).first() else {
        return false;
    };
    let Some(receiver_name) = post_lower_var_name(il, interner, receiver) else {
        return false;
    };
    let Some(contract) =
        library_ruby_set_factory_contract(il.meta.lang, receiver_name, method, arg_count)
    else {
        return false;
    };
    let LibraryApiCalleeContract::RubyRequireStaticMember {
        receiver: expected_receiver,
        required_module,
        shadow_root,
        ..
    } = contract.callee
    else {
        return false;
    };
    if post_lower_file_defines_name_visible_at(il, interner, shadow_root, il.node(receiver).span) {
        return false;
    }
    let Some(receiver_dependency) =
        post_lower_unshadowed_symbol_evidence_id(il, receiver, expected_receiver)
    else {
        return false;
    };
    let Some(require_dependency) =
        post_lower_required_module_evidence_id(il, interner, required_module, il.node(call).span)
    else {
        return false;
    };
    let api = post_lower_library_api_evidence_with_pack_id(
        il,
        call,
        contract.id,
        contract.callee,
        arg_count,
        RUBY_STDLIB_SET_PACK_ID,
        RUBY_STDLIB_SET_PRODUCER_ID,
        vec![receiver_dependency, require_dependency],
    );
    post_lower_record_library_api_result_domain(
        il,
        call,
        library_collection_factory_result_domain_for_arity(contract, arg_count),
        api,
    );
    true
}

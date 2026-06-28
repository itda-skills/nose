use super::*;

pub(super) fn record_post_lower_property_library_api(
    il: &mut Il,
    interner: &Interner,
    field: NodeId,
    dependency_cache: &mut LibraryApiDependencyCache,
) -> bool {
    if il.kind(field) != NodeKind::Field {
        return false;
    }
    let Payload::Name(property) = il.node(field).payload else {
        return false;
    };
    let Some(contract) =
        library_property_builtin_contract(il.meta.lang, interner.resolve(property))
    else {
        return false;
    };
    let Some(dependencies) = library_api_property_dependencies_for_field_with_cache(
        il,
        interner,
        field,
        contract.callee,
        dependency_cache,
    ) else {
        return false;
    };
    post_lower_library_api_node_evidence_with_pack_id(
        il,
        field,
        contract.id,
        contract.callee,
        0,
        contract.pack_id,
        PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID,
        dependencies,
    );
    true
}

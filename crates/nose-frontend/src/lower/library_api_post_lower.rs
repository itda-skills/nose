use super::post_lower_evidence::*;
use super::*;

mod free_name_vars;
mod java_collection_constructors;
mod object_key_views;
mod properties;
mod python_iterator_builtins;
mod receiver_methods;
mod result_bindings;
mod ruby_static_members;
mod rust_option;
mod rust_result;
mod swift_factories;
use free_name_vars::record_post_lower_free_name_var_library_api;
use java_collection_constructors::record_post_lower_java_collection_constructor_library_api;
use object_key_views::record_post_lower_object_key_view_library_api;
use properties::record_post_lower_property_library_api;
use python_iterator_builtins::{
    post_lower_add_iterator_source_dependencies, post_lower_free_function_builtin_api_contract,
    post_lower_free_function_hof_api_contract,
};
use receiver_methods::record_post_lower_receiver_method_library_api;
use result_bindings::post_lower_record_assignment_binding_domain_from_call_result;
use ruby_static_members::record_post_lower_ruby_static_member_library_api;
use rust_option::{
    record_post_lower_rust_option_none_library_api,
    record_post_lower_rust_option_some_pattern_library_api,
};
use rust_result::{
    post_lower_rust_result_constructor_contract, record_post_lower_rust_result_pattern_library_api,
};
use swift_factories::post_lower_record_swift_map_factory_result_domain;

pub(super) struct PostLowerLibraryApiContract {
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    pack_id: &'static str,
    rule: &'static str,
    result_domain: Option<DomainEvidence>,
}

fn record_post_lower_library_api_contract(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
    arg_count: usize,
    contract: PostLowerLibraryApiContract,
    dependencies: Vec<EvidenceId>,
) -> EvidenceId {
    let api = post_lower_library_api_evidence_with_pack_id(
        il,
        call,
        contract.id,
        contract.callee,
        arg_count,
        contract.pack_id,
        contract.rule,
        dependencies,
    );
    if let Some(result_domain) =
        post_lower_record_library_api_result_domain(il, call, contract.result_domain, api)
    {
        post_lower_record_assignment_binding_domain_from_call_result(
            il,
            interner,
            call,
            contract.result_domain,
            result_domain,
        );
    }
    api
}

pub(super) fn record_post_lower_library_api_evidence(il: &mut Il, interner: &Interner) {
    let calls: Vec<NodeId> = il
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, node)| {
            (node.kind == NodeKind::Call && node.payload == Payload::None)
                .then_some(NodeId(idx as u32))
        })
        .collect();
    let fields: Vec<NodeId> = il
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, node)| (node.kind == NodeKind::Field).then_some(NodeId(idx as u32)))
        .collect();
    let vars: Vec<NodeId> = il
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, node)| (node.kind == NodeKind::Var).then_some(NodeId(idx as u32)))
        .collect();
    let mut dependency_cache = LibraryApiDependencyCache::default();
    for call in calls {
        if record_post_lower_free_name_library_api(il, interner, call) {
            continue;
        }
        if record_post_lower_ruby_static_member_library_api(il, interner, call) {
            continue;
        }
        if record_post_lower_java_collection_constructor_library_api(il, interner, call) {
            continue;
        }
        if record_post_lower_object_key_view_library_api(il, interner, call) {
            continue;
        }
        if record_post_lower_static_global_method_library_api(il, interner, call) {
            continue;
        }
        record_post_lower_receiver_method_library_api(il, interner, call, &mut dependency_cache);
    }
    for field in fields {
        record_post_lower_property_library_api(il, interner, field, &mut dependency_cache);
    }
    for var in vars {
        if !post_lower_rust_sum_type_selector_candidate(il, interner, var) {
            continue;
        }
        record_post_lower_rust_option_some_pattern_library_api(il, interner, var);
        record_post_lower_rust_option_none_library_api(il, interner, var);
        record_post_lower_rust_result_pattern_library_api(il, interner, var);
    }
}

fn record_post_lower_free_name_library_api(il: &mut Il, interner: &Interner, call: NodeId) -> bool {
    let kids = il.children(call).to_vec();
    let Some((&callee, args)) = kids.split_first() else {
        return false;
    };
    let Some(callee_name) = post_lower_var_name(il, interner, callee) else {
        return false;
    };
    let arg_count = args.len();
    let first_arg_label = args
        .first()
        .and_then(|&arg| post_lower_kwarg_name(il, interner, arg));
    let Some(contract) = post_lower_free_name_library_api_contract(
        il.meta.lang,
        callee_name,
        arg_count,
        first_arg_label,
    ) else {
        return false;
    };
    if il.meta.lang == Lang::Python && post_lower_has_python_wildcard_import_evidence(il) {
        return false;
    }
    let Some(dependencies) =
        post_lower_free_name_library_api_dependencies(il, interner, call, callee, contract.callee)
    else {
        return false;
    };
    let mut dependencies = dependencies;
    if !post_lower_add_iterator_source_dependencies(
        il,
        interner,
        args,
        contract.id,
        &mut dependencies,
    ) {
        return false;
    }
    let is_swift_map_factory = matches!(contract.id, LibraryApiContractId::SwiftMapFactory(_));
    let api = record_post_lower_library_api_contract(
        il,
        interner,
        call,
        arg_count,
        contract,
        dependencies,
    );
    if is_swift_map_factory {
        post_lower_record_swift_map_factory_result_domain(il, interner, call, api);
    }
    true
}

fn post_lower_free_name_library_api_contract(
    lang: Lang,
    callee_name: &str,
    arg_count: usize,
    first_arg_label: Option<&str>,
) -> Option<PostLowerLibraryApiContract> {
    post_lower_collection_factory_contract(lang, callee_name, arg_count, first_arg_label)
        .or_else(|| {
            post_lower_swift_map_factory_contract(lang, callee_name, arg_count, first_arg_label)
        })
        .or_else(|| post_lower_map_factory_contract(lang, callee_name, arg_count))
        .or_else(|| {
            library_rust_vec_macro_factory_contract(lang, callee_name).map(|contract| {
                PostLowerLibraryApiContract {
                    id: contract.id,
                    callee: contract.callee,
                    pack_id: contract.pack_id,
                    rule: RUST_STDLIB_VEC_PRODUCER_ID,
                    result_domain: library_collection_factory_result_domain_for_arity(
                        contract, arg_count,
                    ),
                }
            })
        })
        .or_else(|| {
            (arg_count == 0)
                .then(|| library_rust_vec_new_factory_contract(lang, callee_name))
                .flatten()
                .map(|contract| PostLowerLibraryApiContract {
                    id: contract.id,
                    callee: contract.callee,
                    pack_id: contract.pack_id,
                    rule: RUST_STDLIB_VEC_PRODUCER_ID,
                    result_domain: library_collection_factory_result_domain_for_arity(
                        contract, arg_count,
                    ),
                })
        })
        .or_else(|| {
            library_rust_option_some_constructor_contract(lang, callee_name, arg_count).map(
                |contract| PostLowerLibraryApiContract {
                    id: contract.id,
                    callee: contract.callee,
                    pack_id: contract.pack_id,
                    rule: RUST_STDLIB_OPTION_PRODUCER_ID,
                    result_domain: Some(contract.result_domain),
                },
            )
        })
        .or_else(|| post_lower_rust_result_constructor_contract(lang, callee_name, arg_count))
        .or_else(|| post_lower_free_function_hof_api_contract(lang, callee_name, arg_count))
        .or_else(|| post_lower_free_function_builtin_api_contract(lang, callee_name, arg_count))
}

fn post_lower_collection_factory_contract(
    lang: Lang,
    callee_name: &str,
    arg_count: usize,
    first_arg_label: Option<&str>,
) -> Option<PostLowerLibraryApiContract> {
    (arg_count == 1)
        .then(|| library_free_name_collection_factory_contract(lang, callee_name))
        .flatten()
        .filter(|contract| {
            !matches!(contract.id, LibraryApiContractId::SwiftCollectionFactory(_))
                || first_arg_label.is_none()
        })
        .map(|contract| PostLowerLibraryApiContract {
            id: contract.id,
            callee: contract.callee,
            pack_id: contract.pack_id,
            rule: post_lower_collection_factory_rule(contract.id),
            result_domain: library_collection_factory_result_domain_for_arity(contract, arg_count),
        })
}

fn post_lower_collection_factory_rule(id: LibraryApiContractId) -> &'static str {
    match id {
        LibraryApiContractId::PythonBuiltinCollectionFactory => {
            PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID
        }
        LibraryApiContractId::RustStdCollectionFactory => {
            RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID
        }
        LibraryApiContractId::SwiftCollectionFactory(_) => {
            SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID
        }
        _ => "library_api_free_name_collection_factory",
    }
}

fn post_lower_swift_map_factory_contract(
    lang: Lang,
    callee_name: &str,
    arg_count: usize,
    first_arg_label: Option<&str>,
) -> Option<PostLowerLibraryApiContract> {
    first_arg_label
        .filter(|_| arg_count == 1)
        .and_then(|label| library_swift_map_factory_contract(lang, callee_name, label))
        .map(|contract| PostLowerLibraryApiContract {
            id: contract.id,
            callee: contract.callee,
            pack_id: contract.pack_id,
            rule: SWIFT_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
            result_domain: None,
        })
}

fn post_lower_map_factory_contract(
    lang: Lang,
    callee_name: &str,
    arg_count: usize,
) -> Option<PostLowerLibraryApiContract> {
    (arg_count == 1)
        .then(|| library_free_name_map_factory_contract(lang, callee_name))
        .flatten()
        .map(|contract| PostLowerLibraryApiContract {
            id: contract.id,
            callee: contract.callee,
            pack_id: contract.pack_id,
            rule: match contract.id {
                LibraryApiContractId::RustStdMapFactory => RUST_STDLIB_MAP_FACTORY_PRODUCER_ID,
                _ => "library_api_free_name_map_factory",
            },
            result_domain: Some(library_map_factory_result_domain(contract)),
        })
}

fn record_post_lower_static_global_method_library_api(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
) -> bool {
    let Some((callee, receiver_node, receiver, method, arg_count)) =
        post_lower_static_global_call_parts(il, interner, call)
    else {
        return false;
    };
    let Some(contract) = post_lower_static_global_method_library_api_contract(
        il.meta.lang,
        receiver,
        method,
        arg_count,
    ) else {
        return false;
    };
    let LibraryApiCalleeContract::StaticGlobalMethod {
        receiver: expected_receiver,
        qualified_path,
        requires_unshadowed_receiver,
        ..
    } = contract.callee
    else {
        return false;
    };
    let Some(qualified) =
        post_lower_qualified_global_symbol_evidence_id(il, callee, qualified_path)
    else {
        return false;
    };
    let mut dependencies = vec![qualified];
    if requires_unshadowed_receiver {
        let Some(receiver_dependency) =
            post_lower_unshadowed_symbol_evidence_id(il, receiver_node, expected_receiver)
        else {
            return false;
        };
        dependencies.push(receiver_dependency);
    }
    record_post_lower_library_api_contract(il, interner, call, arg_count, contract, dependencies);
    true
}

fn post_lower_static_global_method_library_api_contract(
    lang: Lang,
    receiver: &str,
    method: &str,
    arg_count: usize,
) -> Option<PostLowerLibraryApiContract> {
    library_promise_resolve_contract(lang, receiver, method, arg_count)
        .map(|contract| PostLowerLibraryApiContract {
            id: contract.id,
            callee: contract.callee,
            pack_id: contract.pack_id,
            rule: JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
            result_domain: Some(contract.result.result_domain),
        })
        .or_else(|| {
            library_promise_aggregate_contract(lang, receiver, method, arg_count).map(|contract| {
                PostLowerLibraryApiContract {
                    id: contract.id,
                    callee: contract.callee,
                    pack_id: contract.pack_id,
                    rule: JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID,
                    result_domain: Some(contract.result.result_domain),
                }
            })
        })
}

fn post_lower_free_name_library_api_dependencies(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
    callee: NodeId,
    callee_contract: LibraryApiCalleeContract,
) -> Option<Vec<EvidenceId>> {
    let mut dependencies = Vec::new();
    match callee_contract {
        LibraryApiCalleeContract::FreeName { name, shadow } => {
            if !library_api_free_name_shadow_safe(il.meta.lang, name, shadow, |candidate| {
                post_lower_file_defines_name_visible_at(
                    il,
                    interner,
                    candidate,
                    il.node(callee).span,
                )
            }) {
                return None;
            }
            let dependency = post_lower_unshadowed_symbol_evidence_id(il, callee, name)?;
            dependencies.push(dependency);
        }
        LibraryApiCalleeContract::LabeledFreeName { name, shadow, .. } => {
            if !library_api_free_name_shadow_safe(il.meta.lang, name, shadow, |candidate| {
                post_lower_file_defines_name_visible_at(
                    il,
                    interner,
                    candidate,
                    il.node(callee).span,
                )
            }) {
                return None;
            }
            let dependency = post_lower_unshadowed_symbol_evidence_id(il, callee, name)?;
            dependencies.push(dependency);
        }
        LibraryApiCalleeContract::RustMacro { name, shadow } => {
            if !library_api_free_name_shadow_safe(il.meta.lang, name, shadow, |candidate| {
                post_lower_file_defines_name_visible_at(
                    il,
                    interner,
                    candidate,
                    il.node(callee).span,
                )
            }) {
                return None;
            }
            let source_dependency =
                post_lower_source_call_evidence_id(il, call, SourceCallKind::MacroInvocation)?;
            let symbol_dependency = post_lower_unshadowed_symbol_evidence_id(il, callee, name)?;
            dependencies.push(source_dependency);
            dependencies.push(symbol_dependency);
        }
        _ => return None,
    }
    Some(dependencies)
}

fn post_lower_kwarg_name<'a>(il: &Il, interner: &'a Interner, node: NodeId) -> Option<&'a str> {
    if il.kind(node) != NodeKind::KwArg {
        return None;
    }
    let Payload::Name(name) = il.node(node).payload else {
        return None;
    };
    Some(interner.resolve(name))
}

fn post_lower_rust_sum_type_selector_candidate(il: &Il, interner: &Interner, node: NodeId) -> bool {
    if il.meta.lang != Lang::Rust {
        return false;
    }
    let Some(name) = post_lower_var_name(il, interner, node) else {
        return false;
    };
    matches!(
        name,
        "Some"
            | "Option::Some"
            | "std::option::Option::Some"
            | "core::option::Option::Some"
            | "None"
            | "Option::None"
            | "std::option::Option::None"
            | "core::option::Option::None"
            | "Ok"
            | "Result::Ok"
            | "std::result::Result::Ok"
            | "core::result::Result::Ok"
            | "Err"
            | "Result::Err"
            | "std::result::Result::Err"
            | "core::result::Result::Err"
    )
}

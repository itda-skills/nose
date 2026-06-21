use super::*;

pub(crate) fn library_api_dependency_id_for_normalized_hof(
    il: &Il,
    receiver: NodeId,
) -> Option<EvidenceId> {
    let Payload::HoF(kind) = il.node(receiver).payload else {
        return None;
    };
    let expected_id = LibraryApiContractId::MethodCall(MethodSemanticContract::HoF(kind));
    let expected_contract_hash = library_api_contract_id_hash(expected_id);
    let anchor = EvidenceAnchor::node(il.node(receiver).span, NodeKind::Call);
    let mut found = None;
    for record in il.evidence_anchored_at(anchor.span()) {
        if record.anchor != anchor
            || record.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(record)
        {
            continue;
        }
        let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash,
            callee_hash,
            arity,
        }) = record.kind
        else {
            continue;
        };
        if contract_hash != expected_contract_hash {
            continue;
        }
        let Some(callee) =
            library_api_callee_contract_for_hash(il.meta.lang, expected_id, callee_hash)
        else {
            continue;
        };
        let Some(contract) =
            library_api_method_call_record_contract(il, expected_id, callee, arity)
        else {
            continue;
        };
        if !library_api_record_provenance_matches_contract(expected_id, callee, record)
            || !normalized_hof_method_call_dependencies_match(il, receiver, record, contract)
        {
            continue;
        }
        match found {
            None => found = Some(record.id),
            Some(existing) if existing == record.id => {}
            Some(_) => return None,
        }
    }
    found
}

pub(in crate::library_api) fn library_api_dependency_id_for_protocol_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<EvidenceId> {
    if let Some(id) = library_api_dependency_id_for_call(
        il,
        interner,
        call,
        LibraryApiContractId::IteratorIdentityAdapter,
    ) {
        return Some(id);
    }
    if let Some(id) = library_api_dependency_id_for_call(
        il,
        interner,
        call,
        LibraryApiContractId::StaticCollectionAdapter,
    ) {
        return Some(id);
    }
    library_api_dependency_id_for_call_predicate(il, interner, call, |id| {
        matches!(
            id,
            LibraryApiContractId::MethodCall(
                MethodSemanticContract::HoF(_) | MethodSemanticContract::Builtin(Builtin::Zip)
            )
        )
    })
}

pub(in crate::library_api) fn library_api_dependency_id_for_receiver_domain_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    contract: MethodReceiverContract,
) -> Option<EvidenceId> {
    let requirement = method_receiver_domain_requirement(contract)?;
    library_api_dependency_id_for_receiver_domain_requirement(il, interner, call, requirement)
}

pub(in crate::library_api) fn library_api_dependency_id_for_receiver_domain_requirement(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    requirement: DomainRequirement,
) -> Option<EvidenceId> {
    library_api_dependency_id_for_call_contract(il, interner, call, |id, callee, arity| {
        library_api_contract_result_domain_for_arity(id, callee, arity)
            .is_some_and(|domain| requirement.accepts(domain))
    })
}

pub(in crate::library_api) fn library_api_dependency_id_for_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    id: LibraryApiContractId,
) -> Option<EvidenceId> {
    library_api_dependency_id_for_call_predicate(il, interner, call, |actual| actual == id)
}

pub(crate) fn language_core_builtin_at_call(il: &Il, call: NodeId, builtin: Builtin) -> bool {
    let arity = il.children(call).len();
    match (il.meta.lang, builtin, arity) {
        (Lang::Go, Builtin::Contains, 2) => true,
        (Lang::Go, Builtin::Enumerate, 1) => true,
        (Lang::Python, Builtin::DictEntry, 2) => true,
        (
            Lang::JavaScript | Lang::TypeScript | Lang::Vue | Lang::Svelte | Lang::Html,
            Builtin::Keys,
            1,
        ) => true,
        (Lang::C, Builtin::UnsignedCast32, 1) => {
            source_cast_at_node(il, call) == Some(SourceCastKind::CUnsigned32)
        }
        (_, Builtin::Append, 2) => {
            asserted_effect_at_node(il, call, EffectEvidenceKind::BuilderAppendCall)
        }
        _ => false,
    }
}

/// The asserted same-span `LibraryApi` evidence record that licenses a canonical builtin call.
///
/// Normalization may rewrite a source/library call to `Payload::Builtin`, but the payload is only
/// an operation shape. Producers of downstream evidence can use this helper to preserve the
/// original source/API proof as a dependency instead of treating the canonical payload as proof.
pub fn library_api_dependency_id_for_canonical_builtin_call(
    il: &Il,
    call: NodeId,
    builtin: Builtin,
) -> Option<EvidenceId> {
    library_api_dependency_id_for_canonical_builtin_call_contract(il, call, |record, id, _, _| {
        library_api_record_models_canonical_builtin(il, call, record, id, builtin)
    })
}

pub fn library_api_dependency_id_for_canonical_builtin_method_call(
    il: &Il,
    call: NodeId,
    builtin: Builtin,
    expected_callee: LibraryApiCalleeContract,
    expected_arity: u16,
) -> Option<EvidenceId> {
    library_api_dependency_id_for_canonical_builtin_call_contract(
        il,
        call,
        |record, id, callee, arity| {
            library_api_record_models_canonical_builtin(il, call, record, id, builtin)
                && callee == Some(expected_callee)
                && arity == expected_arity
        },
    )
}

pub(in crate::library_api) fn library_api_dependency_id_for_canonical_builtin_call_contract(
    il: &Il,
    call: NodeId,
    accepts: impl Fn(
        &EvidenceRecord,
        LibraryApiContractId,
        Option<LibraryApiCalleeContract>,
        u16,
    ) -> bool,
) -> Option<EvidenceId> {
    if il.kind(call) != NodeKind::Call {
        return None;
    }
    let span = il.node(call).span;
    let mut found = None;
    for record in il.evidence_anchored_at(span) {
        if !matches!(
            record.anchor,
            EvidenceAnchor::Node {
                span: record_span,
                kind: NodeKind::Call | NodeKind::Field,
            } if record_span == span
        ) {
            continue;
        }
        let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash,
            callee_hash,
            arity,
        }) = record.kind
        else {
            continue;
        };
        let Some(id) = library_api_contract_id_from_hash(contract_hash) else {
            continue;
        };
        let callee = library_api_callee_contract_for_hash(il.meta.lang, id, callee_hash);
        if !canonical_record_provenance_and_dependencies_match(il, call, record, id, callee) {
            return None;
        }
        if !accepts(record, id, callee, arity) {
            return None;
        }
        if record.status != EvidenceStatus::Asserted || !il.evidence_dependencies_asserted(record) {
            return None;
        }
        match found {
            None => found = Some(record.id),
            Some(existing) if existing == record.id => {}
            Some(_) => return None,
        }
    }
    found
}

fn canonical_record_provenance_and_dependencies_match(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    callee: Option<LibraryApiCalleeContract>,
) -> bool {
    let Some(callee) = callee else {
        return !matches!(
            id,
            LibraryApiContractId::ScalarIntegerMethod(_) | LibraryApiContractId::MethodCall(_)
        );
    };
    if !library_api_record_provenance_matches_contract(id, callee, record) {
        return false;
    }
    match (id, callee) {
        (
            LibraryApiContractId::ScalarIntegerMethod(_),
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::ExactInteger,
                ..
            },
        ) => il
            .children(call)
            .first()
            .is_some_and(|&arg| canonical_integer_arg_dependency_present(il, record, arg)),
        (
            LibraryApiContractId::ScalarIntegerMethod(_),
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::UnshadowedGlobal("Math"),
                ..
            },
        ) => {
            canonical_record_has_unshadowed_math_dependency(il, call, record)
                && il
                    .children(call)
                    .iter()
                    .all(|&arg| canonical_integer_arg_dependency_present(il, record, arg))
        }
        (
            LibraryApiContractId::FreeFunctionBuiltin(_),
            LibraryApiCalleeContract::FreeName { name, .. },
        ) => canonical_record_has_unshadowed_symbol_dependency(il, call, record, name),
        (LibraryApiContractId::MethodCall(_), LibraryApiCalleeContract::Method { .. }) => {
            canonical_method_call_record_dependencies_match(il, call, record, id, callee)
        }
        _ => true,
    }
}

fn canonical_record_has_unshadowed_math_dependency(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
) -> bool {
    canonical_record_has_unshadowed_symbol_dependency(il, call, record, "Math")
}

fn canonical_record_has_unshadowed_symbol_dependency(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    name: &str,
) -> bool {
    let call_span = il.node(call).span;
    let expected = SymbolEvidenceKind::UnshadowedGlobal {
        name_hash: stable_symbol_hash(name),
    };
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        let EvidenceAnchor::Node {
            span,
            kind: NodeKind::Var,
        } = dependency.anchor
        else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && dependency.kind == EvidenceKind::Symbol(expected)
            && span.file == call_span.file
            && span.start_byte == call_span.start_byte
            && span.end_byte <= call_span.end_byte
    })
}

fn canonical_integer_arg_dependency_present(il: &Il, record: &EvidenceRecord, arg: NodeId) -> bool {
    if matches!(il.node(arg).payload, Payload::LitInt(_)) {
        return true;
    }
    let expected = EvidenceKind::Domain(DomainEvidence::Integer);
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        if dependency.status != EvidenceStatus::Asserted || dependency.kind != expected {
            return false;
        }
        match dependency.anchor {
            EvidenceAnchor::Node { span, kind } => {
                span == il.node(arg).span && kind == il.kind(arg)
            }
            EvidenceAnchor::Param { span } => {
                let Payload::Cid(cid) = il.node(arg).payload else {
                    return false;
                };
                il.nodes.iter().any(|node| {
                    node.kind == NodeKind::Param
                        && node.span == span
                        && matches!(node.payload, Payload::Cid(param_cid) if param_cid == cid)
                })
            }
            _ => false,
        }
    })
}

pub(in crate::library_api) fn library_api_record_models_canonical_builtin(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    builtin: Builtin,
) -> bool {
    if let LibraryApiContractId::FreeFunctionBuiltin(expected_builtin) = id {
        return expected_builtin == builtin
            && library_api_record_models_free_function_builtin(il, record, expected_builtin);
    }
    if library_api_record_models_rust_map_get_default(il, call, record, id, builtin) {
        return true;
    }
    if matches!(id, LibraryApiContractId::MethodCall(_)) {
        return library_api_record_models_method_call_builtin(il, record, id, builtin);
    }
    if library_api_contract_id_builtin_result(id) == Some(builtin) {
        return true;
    }
    false
}

fn library_api_record_models_free_function_builtin(
    il: &Il,
    record: &EvidenceRecord,
    builtin: Builtin,
) -> bool {
    let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
        callee_hash, arity, ..
    }) = record.kind
    else {
        return false;
    };
    let Some(LibraryApiCalleeContract::FreeName { name, .. }) =
        library_api_callee_contract_for_hash(
            il.meta.lang,
            LibraryApiContractId::FreeFunctionBuiltin(builtin),
            callee_hash,
        )
    else {
        return false;
    };
    library_free_function_builtin_contract(il.meta.lang, name, arity as usize)
        .is_some_and(|contract| contract.result.builtin == builtin)
}

fn library_api_record_models_method_call_builtin(
    il: &Il,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    builtin: Builtin,
) -> bool {
    let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
        callee_hash, arity, ..
    }) = record.kind
    else {
        return false;
    };
    let Some(callee) = library_api_callee_contract_for_hash(il.meta.lang, id, callee_hash) else {
        return false;
    };
    library_api_method_call_record_contract(il, id, callee, arity).is_some_and(|contract| {
        contract.result.semantic == MethodSemanticContract::Builtin(builtin)
    })
}

fn library_api_method_call_record_contract(
    il: &Il,
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
) -> Option<LibraryMethodCallContract> {
    let LibraryApiContractId::MethodCall(expected) = id else {
        return None;
    };
    let LibraryApiCalleeContract::Method { method, .. } = callee else {
        return None;
    };
    let contract = library_method_call_contract(il.meta.lang, method, arity as usize)?;
    (contract.id == id && contract.callee == callee && contract.result.semantic == expected)
        .then_some(contract)
}

fn canonical_method_call_record_dependencies_match(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
) -> bool {
    let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract { arity, .. }) = record.kind
    else {
        return false;
    };
    let Some(contract) = library_api_method_call_record_contract(il, id, callee, arity) else {
        return false;
    };
    method_call_receiver_dependencies_match(il, call, record, contract)
}

fn normalized_hof_method_call_dependencies_match(
    il: &Il,
    hof: NodeId,
    record: &EvidenceRecord,
    contract: LibraryMethodCallContract,
) -> bool {
    let Some(&receiver) = il.children(hof).first() else {
        return false;
    };
    receiver_contract_dependency_match(il, record, receiver, contract.result.receiver)
}

fn method_call_receiver_dependencies_match(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    contract: LibraryMethodCallContract,
) -> bool {
    match contract.result.receiver {
        MethodReceiverContract::UnshadowedGlobal(name) => {
            canonical_record_has_unshadowed_symbol_dependency(il, call, record, name)
        }
        MethodReceiverContract::ImportedNamespace(module) => {
            canonical_record_has_imported_namespace_dependency(il, record, module)
        }
        receiver => {
            let Some(receiver_node) =
                canonical_method_receiver_node(il, call, contract.result.args)
            else {
                return false;
            };
            if !receiver_contract_dependency_match(il, record, receiver_node, receiver) {
                return false;
            }
            if receiver == MethodReceiverContract::ExactProtocolPairArgument {
                let Some(&pair) = il.children(call).get(1) else {
                    return false;
                };
                return receiver_contract_dependency_match(
                    il,
                    record,
                    pair,
                    MethodReceiverContract::ExactProtocol,
                );
            }
            true
        }
    }
}

fn canonical_method_receiver_node(
    il: &Il,
    call: NodeId,
    args: MethodBuiltinArgs,
) -> Option<NodeId> {
    match args {
        MethodBuiltinArgs::All | MethodBuiltinArgs::First | MethodBuiltinArgs::GoSliceContains => {
            None
        }
        MethodBuiltinArgs::FirstThenReceiver => il.children(call).get(1).copied(),
        MethodBuiltinArgs::Fold => il.children(call).get(1).copied(),
        MethodBuiltinArgs::ReceiverOnly
        | MethodBuiltinArgs::ReceiverThenAll
        | MethodBuiltinArgs::ReceiverAndFirst
        | MethodBuiltinArgs::MapGetDefault
        | MethodBuiltinArgs::MapGetDefaultOrZeroArgLambda
        | MethodBuiltinArgs::RustMapGetOrOptionDefault
        | MethodBuiltinArgs::RustOptionDefaultLambda
        | MethodBuiltinArgs::RustOptionMapOrIdentity
        | MethodBuiltinArgs::RustZip
        | MethodBuiltinArgs::BoolReduction
        | MethodBuiltinArgs::Hof
        | MethodBuiltinArgs::CollectionReduction => il.children(call).first().copied(),
    }
}

fn receiver_contract_dependency_match(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    contract: MethodReceiverContract,
) -> bool {
    if contract == MethodReceiverContract::LiteralString
        && matches!(il.node(receiver).payload, Payload::LitStr(_))
    {
        return true;
    }
    if contract == MethodReceiverContract::RustMapGetOrExactOption
        && evidence_depends_on_library_api_contract(
            il,
            record,
            LibraryApiContractId::MapGet,
            Some(receiver),
        )
    {
        return true;
    }
    if matches!(il.node(receiver).payload, Payload::HoF(_))
        && library_api_record_depends_on_normalized_hof(il, record, receiver)
    {
        return true;
    }
    if let Some(requirement) = method_receiver_domain_requirement(contract) {
        return library_api_record_depends_on_receiver_requirement(
            il,
            record,
            receiver,
            requirement,
        ) || library_api_record_depends_on_receiver_sequence_surface(
            il, record, receiver, contract,
        ) || library_api_record_depends_on_receiver_result_domain(
            il,
            record,
            receiver,
            requirement,
        ) || library_api_record_depends_on_receiver_protocol_api_depth(
            il, record, receiver, contract, 4,
        );
    }
    contract == MethodReceiverContract::ExactMapLiteral
        && library_api_record_depends_on_receiver_sequence_surface(il, record, receiver, contract)
}

fn library_api_record_depends_on_normalized_hof(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
) -> bool {
    library_api_dependency_id_for_normalized_hof(il, receiver)
        .is_some_and(|id| record.dependencies.contains(&id))
}

fn canonical_record_has_imported_namespace_dependency(
    il: &Il,
    record: &EvidenceRecord,
    module: &str,
) -> bool {
    let expected = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
        module_hash: stable_symbol_hash(module),
    });
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(dependency)
            && dependency.kind == expected
            && matches!(
                dependency.anchor,
                EvidenceAnchor::Node {
                    kind: NodeKind::Var,
                    ..
                }
            )
    })
}

fn library_api_record_depends_on_receiver_result_domain(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    requirement: DomainRequirement,
) -> bool {
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        let Some((actual_id, callee, arity)) =
            asserted_library_api_dependency_contract(il, dependency)
        else {
            return false;
        };
        library_api_dependency_anchor_matches_receiver(il, dependency.anchor, receiver)
            && library_api_contract_result_domain_for_arity(actual_id, callee, arity)
                .is_some_and(|domain| requirement.accepts(domain))
    })
}

fn library_api_record_depends_on_receiver_protocol_api_depth(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    contract: MethodReceiverContract,
    depth: usize,
) -> bool {
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        let Some((actual_id, callee, arity)) =
            asserted_library_api_dependency_contract(il, dependency)
        else {
            return false;
        };
        (library_api_dependency_anchor_matches_receiver(il, dependency.anchor, receiver)
            || (depth > 0
                && library_api_dependency_record_has_receiver_proof_depth(
                    il,
                    dependency,
                    receiver,
                    contract,
                    depth - 1,
                )))
            && library_api_dependency_contract_satisfies_protocol_receiver(
                il, actual_id, callee, arity, contract,
            )
    })
}

fn asserted_library_api_dependency_contract(
    il: &Il,
    dependency: &EvidenceRecord,
) -> Option<(LibraryApiContractId, LibraryApiCalleeContract, u16)> {
    if dependency.status != EvidenceStatus::Asserted
        || !il.evidence_dependencies_asserted(dependency)
    {
        return None;
    }
    let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
        contract_hash,
        callee_hash,
        arity,
    }) = dependency.kind
    else {
        return None;
    };
    let actual_id = library_api_contract_id_from_hash(contract_hash)?;
    let callee = library_api_callee_contract_for_hash(il.meta.lang, actual_id, callee_hash)?;
    if !library_api_record_provenance_matches_contract(actual_id, callee, dependency) {
        return None;
    }
    if matches!(actual_id, LibraryApiContractId::MethodCall(_))
        && library_api_method_call_record_contract(il, actual_id, callee, arity).is_none()
    {
        return None;
    }
    Some((actual_id, callee, arity))
}

fn library_api_dependency_anchor_matches_receiver(
    il: &Il,
    anchor: EvidenceAnchor,
    receiver: NodeId,
) -> bool {
    let receiver_span = il.node(receiver).span;
    matches!(
        anchor,
        EvidenceAnchor::Node { span, .. }
            if span == receiver_span
                || (span.file == receiver_span.file
                    && span.start_byte <= receiver_span.start_byte
                    && span.end_byte >= receiver_span.end_byte)
    )
}

fn library_api_dependency_record_has_receiver_proof_depth(
    il: &Il,
    dependency: &EvidenceRecord,
    receiver: NodeId,
    contract: MethodReceiverContract,
    depth: usize,
) -> bool {
    let Some(requirement) = method_receiver_domain_requirement(contract) else {
        return false;
    };
    library_api_record_depends_on_receiver_requirement(il, dependency, receiver, requirement)
        || library_api_record_depends_on_receiver_sequence_surface(
            il, dependency, receiver, contract,
        )
        || library_api_record_depends_on_receiver_result_domain(
            il,
            dependency,
            receiver,
            requirement,
        )
        || (depth > 0
            && library_api_record_depends_on_receiver_protocol_api_depth(
                il,
                dependency,
                receiver,
                contract,
                depth - 1,
            ))
}

fn library_api_dependency_contract_satisfies_protocol_receiver(
    il: &Il,
    actual_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    receiver: MethodReceiverContract,
) -> bool {
    match actual_id {
        LibraryApiContractId::MapKeyView(kind) => map_key_view_satisfies_receiver(kind, receiver),
        LibraryApiContractId::IteratorIdentityAdapter
        | LibraryApiContractId::StaticCollectionAdapter => {
            protocol_api_satisfies_receiver(receiver)
        }
        LibraryApiContractId::MethodCall(
            MethodSemanticContract::HoF(_) | MethodSemanticContract::Builtin(Builtin::Zip),
        ) => {
            protocol_api_satisfies_receiver(receiver)
                && library_api_method_call_record_contract(il, actual_id, callee, arity).is_some()
        }
        _ => false,
    }
}

fn protocol_api_satisfies_receiver(receiver: MethodReceiverContract) -> bool {
    matches!(
        receiver,
        MethodReceiverContract::ExactProtocol | MethodReceiverContract::ExactProtocolPairArgument
    )
}

fn map_key_view_satisfies_receiver(kind: MapKeyViewKind, receiver: MethodReceiverContract) -> bool {
    match kind {
        MapKeyViewKind::Collection => matches!(
            receiver,
            MethodReceiverContract::ExactCollection
                | MethodReceiverContract::ExactCollectionOrMap
                | MethodReceiverContract::ExactCollectionOrJavaKeySet
                | MethodReceiverContract::ExactProtocol
                | MethodReceiverContract::ExactProtocolPairArgument
        ),
        MapKeyViewKind::Iterator => protocol_api_satisfies_receiver(receiver),
    }
}

pub(in crate::library_api) fn library_api_record_models_rust_map_get_default(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    builtin: Builtin,
) -> bool {
    if il.meta.lang != Lang::Rust || builtin != Builtin::GetOrDefault {
        return false;
    }
    let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
        callee_hash, arity, ..
    }) = record.kind
    else {
        return false;
    };
    if id
        != LibraryApiContractId::MethodCall(MethodSemanticContract::Builtin(
            Builtin::ValueOrDefault,
        ))
        || arity != 1
    {
        return false;
    }
    let Some(LibraryApiCalleeContract::Method {
        receiver: MethodReceiverContract::RustMapGetOrExactOption,
        ..
    }) = library_api_callee_contract_for_hash(il.meta.lang, id, callee_hash)
    else {
        return false;
    };
    let Some(&map_receiver) = il.children(call).first() else {
        return false;
    };
    evidence_depends_on_library_api_contract(
        il,
        record,
        LibraryApiContractId::MapGet,
        Some(map_receiver),
    )
}

pub(in crate::library_api) fn evidence_depends_on_library_api_contract(
    il: &Il,
    record: &EvidenceRecord,
    expected_id: LibraryApiContractId,
    required_receiver: Option<NodeId>,
) -> bool {
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        if dependency.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(dependency)
        {
            return false;
        }
        let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash,
            callee_hash,
            arity,
        }) = dependency.kind
        else {
            return false;
        };
        let Some(actual_id) = library_api_contract_id_from_hash(contract_hash) else {
            return false;
        };
        if actual_id != expected_id {
            return false;
        }
        let Some(callee) =
            library_api_callee_contract_for_hash(il.meta.lang, actual_id, callee_hash)
        else {
            return false;
        };
        library_api_record_provenance_matches_contract(actual_id, callee, dependency)
            && library_api_dependency_record_has_expected_arity(actual_id, arity)
            && library_api_dependency_record_has_required_dependencies(
                il,
                dependency,
                actual_id,
                required_receiver,
            )
    })
}

fn library_api_dependency_record_has_expected_arity(id: LibraryApiContractId, arity: u16) -> bool {
    match id {
        LibraryApiContractId::MapGet => arity == 1,
        _ => true,
    }
}

fn library_api_dependency_record_has_required_dependencies(
    il: &Il,
    record: &EvidenceRecord,
    id: LibraryApiContractId,
    required_receiver: Option<NodeId>,
) -> bool {
    match id {
        LibraryApiContractId::MapGet => required_receiver.is_some_and(|receiver| {
            library_api_record_depends_on_receiver_requirement(
                il,
                record,
                receiver,
                DomainRequirement::Map,
            )
        }),
        _ => true,
    }
}

fn library_api_record_depends_on_receiver_requirement(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    requirement: DomainRequirement,
) -> bool {
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        let EvidenceKind::Domain(domain) = dependency.kind else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(dependency)
            && requirement.accepts(domain)
            && domain_dependency_matches_canonical_receiver(il, dependency.anchor, receiver)
    })
}

fn library_api_record_depends_on_receiver_sequence_surface(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    contract: MethodReceiverContract,
) -> bool {
    if il.kind(receiver) != NodeKind::Seq {
        return false;
    }
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        let EvidenceKind::SequenceSurface(kind) = dependency.kind else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(dependency)
            && matches!(
                dependency.anchor,
                EvidenceAnchor::Sequence { span } if span == il.node(receiver).span
            )
            && sequence_surface_kind_satisfies_method_receiver(kind, contract)
    })
}

fn sequence_surface_kind_satisfies_method_receiver(
    kind: SequenceSurfaceKind,
    contract: MethodReceiverContract,
) -> bool {
    match contract {
        MethodReceiverContract::ExactCollection
        | MethodReceiverContract::ExactProtocol
        | MethodReceiverContract::ExactProtocolPairArgument
        | MethodReceiverContract::ExactCollectionOrJavaKeySet => {
            kind == SequenceSurfaceKind::Collection
        }
        MethodReceiverContract::ExactMap | MethodReceiverContract::ExactMapLiteral => {
            kind == SequenceSurfaceKind::Map
        }
        MethodReceiverContract::ExactCollectionOrMap
        | MethodReceiverContract::ExactCollectionOrMapLiteral => {
            matches!(
                kind,
                SequenceSurfaceKind::Collection | SequenceSurfaceKind::Map
            )
        }
        MethodReceiverContract::ExactSetOrMap => kind == SequenceSurfaceKind::Map,
        _ => false,
    }
}

fn domain_dependency_matches_canonical_receiver(
    il: &Il,
    anchor: EvidenceAnchor,
    receiver: NodeId,
) -> bool {
    match anchor {
        EvidenceAnchor::Node { span, kind } => {
            span == il.node(receiver).span && kind == il.kind(receiver)
        }
        EvidenceAnchor::Binding { span, .. } => {
            let Payload::Cid(cid) = il.node(receiver).payload else {
                return false;
            };
            il.nodes.iter().enumerate().any(|(idx, node)| {
                node.kind == NodeKind::Assign
                    && il
                        .children(NodeId(idx as u32))
                        .first()
                        .is_some_and(|&lhs| {
                            il.node(lhs).span == span
                                && matches!(il.node(lhs).payload, Payload::Cid(lhs_cid) if lhs_cid == cid)
                        })
            })
        }
        EvidenceAnchor::Param { span } => {
            let Payload::Cid(cid) = il.node(receiver).payload else {
                return false;
            };
            il.nodes.iter().any(|node| {
                node.kind == NodeKind::Param
                    && node.span == span
                    && matches!(node.payload, Payload::Cid(param_cid) if param_cid == cid)
            })
        }
        _ => false,
    }
}

pub(in crate::library_api) fn library_api_contract_id_builtin_result(
    id: LibraryApiContractId,
) -> Option<Builtin> {
    match id {
        LibraryApiContractId::PropertyBuiltin(builtin)
        | LibraryApiContractId::FreeFunctionBuiltin(builtin) => Some(builtin),
        LibraryApiContractId::MethodCall(MethodSemanticContract::Builtin(builtin)) => Some(builtin),
        LibraryApiContractId::ScalarIntegerMethod(ScalarIntegerMethod::Abs) => Some(Builtin::Abs),
        LibraryApiContractId::ScalarIntegerMethod(ScalarIntegerMethod::Min) => Some(Builtin::Min),
        LibraryApiContractId::ScalarIntegerMethod(ScalarIntegerMethod::Max) => Some(Builtin::Max),
        _ => None,
    }
}

pub(in crate::library_api) fn library_api_dependency_id_for_map_key_view_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    allowed: &[MapKeyViewKind],
) -> Option<EvidenceId> {
    library_api_dependency_id_for_call_predicate(
        il,
        interner,
        call,
        |id| matches!(id, LibraryApiContractId::MapKeyView(kind) if allowed.contains(&kind)),
    )
}

pub(in crate::library_api) fn library_api_dependency_id_for_call_predicate(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    accepts: impl Fn(LibraryApiContractId) -> bool,
) -> Option<EvidenceId> {
    library_api_dependency_id_for_call_contract(il, interner, call, |id, _, _| accepts(id))
}

pub(in crate::library_api) fn library_api_dependency_id_for_call_contract(
    il: &Il,
    interner: &Interner,
    call: NodeId,
    accepts: impl Fn(LibraryApiContractId, LibraryApiCalleeContract, u16) -> bool,
) -> Option<EvidenceId> {
    if il.kind(call) != NodeKind::Call {
        return None;
    }
    let anchor = EvidenceAnchor::node(il.node(call).span, NodeKind::Call);
    let mut found = None;
    for record in il.evidence_anchored_at(anchor.span()) {
        if record.anchor != anchor
            || record.status != EvidenceStatus::Asserted
            || !il.evidence_dependencies_asserted(record)
        {
            continue;
        }
        let EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash,
            callee_hash,
            arity,
        }) = record.kind
        else {
            continue;
        };
        let Some(id) = library_api_contract_id_from_hash(contract_hash) else {
            continue;
        };
        let Some(callee) = library_api_callee_contract_for_hash(il.meta.lang, id, callee_hash)
        else {
            continue;
        };
        if !accepts(id, callee, arity) {
            continue;
        }
        if !library_api_record_admitted_for_current_shape(il, interner, call, record) {
            continue;
        }
        match found {
            None => found = Some(record.id),
            Some(existing) if existing == record.id => {}
            Some(_) => return None,
        }
    }
    found
}

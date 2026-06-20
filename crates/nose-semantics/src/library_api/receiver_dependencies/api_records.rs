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
            ..
        }) = record.kind
        else {
            continue;
        };
        if contract_hash != expected_contract_hash {
            continue;
        }
        if library_api_callee_contract_for_hash(il.meta.lang, expected_id, callee_hash).is_none() {
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
        return !matches!(id, LibraryApiContractId::ScalarIntegerMethod(_));
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
        _ => true,
    }
}

fn canonical_record_has_unshadowed_math_dependency(
    il: &Il,
    call: NodeId,
    record: &EvidenceRecord,
) -> bool {
    let call_span = il.node(call).span;
    let expected = SymbolEvidenceKind::UnshadowedGlobal {
        name_hash: stable_symbol_hash("Math"),
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
            && span.start_byte >= call_span.start_byte
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
    if library_api_contract_id_builtin_result(id) == Some(builtin) {
        return true;
    }
    library_api_record_models_rust_map_get_default(il, call, record, id, builtin)
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
            library_api_record_depends_on_receiver_domain(il, record, receiver, DomainEvidence::Map)
        }),
        _ => true,
    }
}

fn library_api_record_depends_on_receiver_domain(
    il: &Il,
    record: &EvidenceRecord,
    receiver: NodeId,
    expected: DomainEvidence,
) -> bool {
    record.dependencies.iter().any(|&id| {
        let Some(dependency) = il.evidence_record_by_id(id) else {
            return false;
        };
        dependency.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(dependency)
            && dependency.kind == EvidenceKind::Domain(expected)
            && domain_dependency_matches_canonical_receiver(il, dependency.anchor, receiver)
    })
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

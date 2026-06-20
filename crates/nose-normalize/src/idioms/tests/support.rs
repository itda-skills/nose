pub(super) use super::super::*;
pub(super) use nose_il::stable_symbol_hash;
pub(super) use nose_il::{
    DomainEvidence, EvidenceAnchor, EvidenceEmitter, EvidenceId, EvidenceKind, EvidenceProvenance,
    EvidenceRecord, EvidenceStatus, FileId, FileMeta, IlBuilder, ImportEvidenceKind, Lang,
    LibraryApiEvidenceKind, ParamSemantic, SequenceSurfaceKind, Span, SymbolEvidenceKind, Unit,
    UnitKind,
};
pub(super) use nose_semantics::{
    library_free_function_builtin_contract, library_free_name_map_factory_contract,
    library_iterator_identity_adapter_contract, library_map_get_contract,
    library_map_key_view_contract, library_method_call_contract, LibraryApiContractId,
    ITERATOR_IDENTITY_ADAPTER_PACK_ID, ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID,
};

pub(super) fn sp() -> Span {
    Span::new(FileId(0), 1, 1, 1, 1)
}

pub(super) fn sp_at(start: u32, end: u32, line: u32) -> Span {
    Span::new(FileId(0), start, end, line, line)
}

pub(super) fn evidence(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    status: EvidenceStatus,
) -> EvidenceRecord {
    evidence_with_dependencies(id, anchor, kind, status, Vec::new())
}

pub(super) fn evidence_with_dependencies(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    status: EvidenceStatus,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor,
        kind,
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(nose_semantics::FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("test")),
        },
        dependencies,
        status,
    }
}

pub(super) fn next_evidence_id(il: &Il) -> u32 {
    il.evidence.len() as u32
}

pub(super) fn method_call_receiver(il: &Il, call: NodeId) -> Option<NodeId> {
    let callee = *il.children(call).first()?;
    (il.kind(callee) == NodeKind::Field)
        .then(|| il.children(callee).first().copied())
        .flatten()
}

pub(super) fn push_sequence_surface_evidence(
    il: &mut Il,
    node: NodeId,
    surface: SequenceSurfaceKind,
) -> EvidenceId {
    let id = next_evidence_id(il);
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::sequence(il.node(node).span),
        EvidenceKind::SequenceSurface(surface),
        EvidenceStatus::Asserted,
    ));
    EvidenceId(id)
}

pub(super) fn push_receiver_sequence_surface_evidence(
    il: &mut Il,
    call: NodeId,
    surface: SequenceSurfaceKind,
) -> EvidenceId {
    let receiver = method_call_receiver(il, call).expect("method receiver");
    push_sequence_surface_evidence(il, receiver, surface)
}

pub(super) fn push_receiver_method_library_api_evidence(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
) -> Option<EvidenceId> {
    let kids = il.children(call);
    let (&callee, args) = kids.split_first()?;
    if il.kind(callee) != NodeKind::Field {
        return None;
    }
    let Payload::Name(method) = il.node(callee).payload else {
        return None;
    };
    let method = interner.resolve(method);
    let arg_count = args.len();
    let contract = library_map_get_contract(il.meta.lang, method, arg_count)
        .map(|contract| (contract.id, contract.callee))
        .or_else(|| {
            library_map_key_view_contract(il.meta.lang, method, arg_count)
                .map(|contract| (contract.id, contract.callee))
        })
        .or_else(|| {
            library_iterator_identity_adapter_contract(il.meta.lang, method, arg_count)
                .map(|contract| (contract.id, contract.callee))
        })
        .or_else(|| {
            library_method_call_contract(il.meta.lang, method, arg_count)
                .map(|contract| (contract.id, contract.callee))
        })?;
    let dependencies =
        nose_semantics::library_api_receiver_dependencies_for_call(il, interner, call, contract.1)?;
    let id = next_evidence_id(il);
    let mut record = evidence_with_dependencies(
        id,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: nose_semantics::library_api_contract_id_hash(contract.0),
            callee_hash: nose_semantics::library_api_callee_contract_hash(contract.1),
            arity: arg_count as u16,
        }),
        EvidenceStatus::Asserted,
        dependencies,
    );
    if contract.0 == LibraryApiContractId::IteratorIdentityAdapter {
        record.provenance.pack_hash = Some(stable_symbol_hash(ITERATOR_IDENTITY_ADAPTER_PACK_ID));
        record.provenance.rule_hash =
            Some(stable_symbol_hash(ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID));
    }
    il.evidence.push(record);
    Some(EvidenceId(id))
}

pub(super) fn push_free_function_builtin_library_api_evidence(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
) -> Option<EvidenceId> {
    let kids = il.children(call);
    let (&callee, args) = kids.split_first()?;
    let arg_count = args.len();
    let callee_span = il.node(callee).span;
    let call_span = il.node(call).span;
    let Payload::Name(symbol) = il.node(callee).payload else {
        return None;
    };
    let name = interner.resolve(symbol);
    let contract = library_free_function_builtin_contract(il.meta.lang, name, arg_count)?;
    let symbol_id = next_evidence_id(il);
    il.evidence.push(evidence(
        symbol_id,
        EvidenceAnchor::node(callee_span, NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash(name),
        }),
        EvidenceStatus::Asserted,
    ));
    let id = next_evidence_id(il);
    il.evidence.push(evidence_with_dependencies(
        id,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: nose_semantics::library_api_contract_id_hash(contract.id),
            callee_hash: nose_semantics::library_api_callee_contract_hash(contract.callee),
            arity: arg_count as u16,
        }),
        EvidenceStatus::Asserted,
        vec![EvidenceId(symbol_id)],
    ));
    Some(EvidenceId(id))
}

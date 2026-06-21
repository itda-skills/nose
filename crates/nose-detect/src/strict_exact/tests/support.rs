use super::super::*;
pub(super) use nose_il::{
    stable_symbol_hash, CallTargetEvidenceKind, EvidenceAnchor, EvidenceEmitter, EvidenceId,
    EvidenceKind, EvidenceProvenance, EvidenceRecord, EvidenceStatus, FileId, FileMeta, IlBuilder,
    Lang, LibraryApiEvidenceKind, Span, Unit, UnitKind,
};
use nose_normalize::{normalize, NormalizeOptions};
use nose_semantics::{
    library_api_callee_contract_hash, library_api_contract_id_hash, library_map_get_contract,
    library_method_call_contract, LibraryApiCalleeContract, LibraryApiContractId,
    MethodReceiverContract, MethodSemanticContract, FIRST_PARTY_PACK_ID,
    MAP_GET_DEFAULT_PROTOCOL_PACK_ID, MAP_GET_DEFAULT_PROTOCOL_PRODUCER_ID,
    MAP_GET_PROTOCOL_PACK_ID, MAP_GET_PROTOCOL_PRODUCER_ID,
};

pub(super) fn sp(line: u32) -> Span {
    Span::new(FileId(0), line, line, line, line)
}

pub(super) fn normalized_python(src: &str, interner: &Interner) -> Il {
    let raw =
        nose_frontend::lower_source(FileId(0), "t.py", src.as_bytes(), Lang::Python, interner)
            .expect("lower python source");
    normalize(&raw, interner, &NormalizeOptions::default())
}

pub(super) fn first_call_with_target(
    il: &Il,
    interner: &Interner,
    target_matches: impl Fn(CallTargetEvidenceKind) -> bool,
) -> NodeId {
    il.nodes
        .iter()
        .enumerate()
        .find_map(|(idx, node)| {
            if node.kind != NodeKind::Call {
                return None;
            }
            let call = NodeId(idx as u32);
            matches!(
                call_target_evidence_status_at_call(il, interner, call),
                CallTargetEvidenceStatus::Admitted(target) if target_matches(target)
            )
            .then_some(call)
        })
        .expect("admitted call-target call")
}

pub(super) fn evidence(
    id: u32,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor,
        kind,
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("strict-exact-test")),
        },
        dependencies,
        status: EvidenceStatus::Asserted,
    }
}

pub(super) fn method_call_library_api_evidence(
    id: u32,
    lang: Lang,
    method: &str,
    call_span: Span,
    arity: usize,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let contract = library_method_call_contract(lang, method, arity).expect("method call contract");
    let mut record = evidence(
        id,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: arity as u16,
        }),
        dependencies,
    );
    if contract.id
        == LibraryApiContractId::MethodCall(MethodSemanticContract::Builtin(Builtin::GetOrDefault))
        && matches!(
            contract.callee,
            LibraryApiCalleeContract::Method {
                receiver: MethodReceiverContract::ExactMap,
                ..
            }
        )
    {
        record.provenance.pack_hash = Some(stable_symbol_hash(MAP_GET_DEFAULT_PROTOCOL_PACK_ID));
        record.provenance.rule_hash =
            Some(stable_symbol_hash(MAP_GET_DEFAULT_PROTOCOL_PRODUCER_ID));
    }
    record
}

pub(super) fn map_get_library_api_evidence(
    id: u32,
    lang: Lang,
    method: &str,
    call_span: Span,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    let contract = library_map_get_contract(lang, method, 1).expect("map get contract");
    let mut record = evidence(
        id,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 1,
        }),
        dependencies,
    );
    record.provenance.pack_hash = Some(stable_symbol_hash(MAP_GET_PROTOCOL_PACK_ID));
    record.provenance.rule_hash = Some(stable_symbol_hash(MAP_GET_PROTOCOL_PRODUCER_ID));
    record
}

pub(super) fn call_target_evidence(
    id: u32,
    call_span: Span,
    target: CallTargetEvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    evidence(
        id,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::CallTarget(target),
        dependencies,
    )
}

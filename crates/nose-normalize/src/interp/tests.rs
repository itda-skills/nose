use super::*;

mod basic;
mod calls_symbolic;
mod hof;
mod state;
mod try_effects;

use nose_il::{
    stable_symbol_hash, CallTargetEvidenceKind, EffectEvidenceKind, EvidenceAnchor,
    EvidenceEmitter, EvidenceId, EvidenceKind, EvidenceProvenance, EvidenceRecord, EvidenceStatus,
    FileId, FileMeta, HoFKind, IlBuilder, Interner, Lang, LibraryApiEvidenceKind, LitClass,
    PlaceEvidenceKind, SourceCastKind, SourceFactKind, Span, Unit, UnitKind,
};
use nose_semantics::{
    library_api_callee_contract_hash, library_api_contract_id_hash, LibraryApiCalleeContract,
    LibraryApiContractId, LibraryApiShadowPolicy, MethodSemanticContract, FIRST_PARTY_PACK_ID,
};

fn run_admitted_unit(mut il: Il, root: NodeId, args: &[Value]) -> Option<Behavior> {
    admit_test_builtin_calls(&mut il);
    let interner = Interner::new();
    run_unit(&il, &interner, root, args)
}

#[test]
fn behavior_equiv_treats_both_abort_as_equal() {
    let beh = |ret: Value, effects: Vec<Value>| Behavior {
        ret,
        effects,
        fields: vec![],
    };
    // Both abort (`Err`) with DIFFERENT pre-trap effects → equivalent: an erroring run
    // has no observable result, so reordering ops before a guaranteed trap preserves
    // behavior. This is the #-canon-preservation fix for impossible inputs.
    let abort_a = beh(Value::Err, vec![]);
    let abort_b = beh(Value::Err, vec![Value::Int(0), Value::Int(2)]);
    assert!(behavior_equiv(&abort_a, &abort_b));
    assert!(behavior_equiv(&abort_b, &abort_a));
    // Real behavior changes still compare unequal:
    // Ok→Err (the canon made it start/stop erroring)
    assert!(!behavior_equiv(
        &beh(Value::Int(1), vec![]),
        &beh(Value::Err, vec![])
    ));
    // two successful runs with different results
    assert!(!behavior_equiv(
        &beh(Value::Int(1), vec![]),
        &beh(Value::Int(2), vec![])
    ));
    // two successful runs with different effect traces
    assert!(!behavior_equiv(
        &beh(Value::Null, vec![Value::Int(1)]),
        &beh(Value::Null, vec![Value::Int(2)]),
    ));
    // identical successful runs are equal
    assert!(behavior_equiv(
        &beh(Value::Int(7), vec![Value::Int(1)]),
        &beh(Value::Int(7), vec![Value::Int(1)]),
    ));
}

fn test_span(offset: u32) -> Span {
    Span::new(FileId(0), offset, offset + 1, offset + 1, offset + 1)
}

fn admit_test_builtin_calls(il: &mut Il) {
    let mut seen_library_records = Vec::new();
    let mut next_id = 1000;
    for idx in 0..il.nodes.len() {
        let node = NodeId(idx as u32);
        let (NodeKind::Call, Payload::Builtin(builtin)) = (il.kind(node), il.node(node).payload)
        else {
            continue;
        };
        let span = il.node(node).span;
        if matches!(builtin, Builtin::Append) {
            il.evidence.push(test_effect_record(
                next_id,
                span,
                EffectEvidenceKind::BuilderAppendCall,
            ));
            next_id += 1;
        } else if let Some(contract_id) = test_library_contract_id_for_builtin(builtin) {
            if seen_library_records
                .iter()
                .any(|&(seen_span, seen_builtin)| seen_span == span && seen_builtin == builtin)
            {
                continue;
            }
            seen_library_records.push((span, builtin));
            il.evidence.push(test_library_api_record(
                next_id,
                span,
                contract_id,
                test_callee_contract(),
            ));
            next_id += 1;
        } else if matches!(builtin, Builtin::UnsignedCast32) {
            il.evidence.push(test_source_record(
                next_id,
                span,
                SourceFactKind::Cast(SourceCastKind::CUnsigned32),
            ));
            next_id += 1;
        }
    }
}

fn test_library_contract_id_for_builtin(builtin: Builtin) -> Option<LibraryApiContractId> {
    match builtin {
        Builtin::Len
        | Builtin::Print
        | Builtin::Range
        | Builtin::Sum
        | Builtin::Min
        | Builtin::Max
        | Builtin::Abs
        | Builtin::Zip
        | Builtin::Enumerate
        | Builtin::Any
        | Builtin::All => Some(LibraryApiContractId::FreeFunctionBuiltin(builtin)),
        Builtin::IsEmpty
        | Builtin::StartsWith
        | Builtin::EndsWith
        | Builtin::Contains
        | Builtin::GetOrDefault
        | Builtin::ValueOrDefault
        | Builtin::IsNull
        | Builtin::IsNotNull
        | Builtin::Join
        | Builtin::Reduce => Some(LibraryApiContractId::MethodCall(
            MethodSemanticContract::Builtin(builtin),
        )),
        Builtin::Append | Builtin::Keys | Builtin::DictEntry | Builtin::UnsignedCast32 => None,
    }
}

fn test_callee_contract() -> LibraryApiCalleeContract {
    LibraryApiCalleeContract::FreeName {
        name: "__test_builtin__",
        shadow: LibraryApiShadowPolicy::None,
    }
}

fn test_library_api_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(span, NodeKind::Call),
        kind: EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract_id),
            callee_hash: library_api_callee_contract_hash(callee),
            arity: 0,
        }),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    }
}

fn test_effect_record(id: u32, span: Span, effect: EffectEvidenceKind) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(span, NodeKind::Call),
        kind: EvidenceKind::Effect(effect),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    }
}

fn test_node_place_record(
    id: u32,
    il: &Il,
    node: NodeId,
    place: PlaceEvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(il.node(node).span, il.kind(node)),
        kind: EvidenceKind::Place(place),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies,
        status: EvidenceStatus::Asserted,
    }
}

fn test_node_effect_record(
    id: u32,
    il: &Il,
    node: NodeId,
    effect: EffectEvidenceKind,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(il.node(node).span, il.kind(node)),
        kind: EvidenceKind::Effect(effect),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies,
        status: EvidenceStatus::Asserted,
    }
}

fn admit_test_self_field(
    il: &mut Il,
    interner: &Interner,
    receiver: NodeId,
    field: NodeId,
    field_name: nose_il::Symbol,
    first_id: u32,
) -> EvidenceId {
    let receiver_id = EvidenceId(first_id);
    let field_id = EvidenceId(first_id + 1);
    let receiver_record = test_node_place_record(
        first_id,
        il,
        receiver,
        PlaceEvidenceKind::SelfReceiver,
        Vec::new(),
    );
    let field_record = test_node_place_record(
        first_id + 1,
        il,
        field,
        PlaceEvidenceKind::SelfField {
            field_hash: stable_symbol_hash(interner.resolve(field_name)),
        },
        vec![receiver_id],
    );
    il.evidence.push(receiver_record);
    il.evidence.push(field_record);
    field_id
}

fn admit_test_self_field_write(
    il: &mut Il,
    interner: &Interner,
    receiver: NodeId,
    field: NodeId,
    assign: NodeId,
    field_name: nose_il::Symbol,
    first_id: u32,
) {
    let field_id = admit_test_self_field(il, interner, receiver, field, field_name, first_id);
    let effect_record = test_node_effect_record(
        first_id + 2,
        il,
        assign,
        EffectEvidenceKind::SelfFieldWrite {
            field_hash: stable_symbol_hash(interner.resolve(field_name)),
        },
        vec![field_id],
    );
    il.evidence.push(effect_record);
}

fn test_source_record(id: u32, span: Span, fact: SourceFactKind) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::source_span(span),
        kind: EvidenceKind::Source(fact),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    }
}

fn test_call_target_record(
    id: u32,
    call_span: Span,
    target_span: Span,
    name_hash: u64,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::node(call_span, NodeKind::Call),
        kind: EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction {
            target_span,
            name_hash,
        }),
        provenance: EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("interp-test")),
        },
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    }
}

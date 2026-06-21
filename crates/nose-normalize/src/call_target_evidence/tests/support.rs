pub(super) use super::super::*;
pub(super) use nose_il::{
    stable_symbol_hash, EvidenceEmitter, EvidenceProvenance, EvidenceRecord, FileId, FileMeta,
    IlBuilder, Lang, Span, Unit,
};
pub(super) use nose_semantics::{
    call_target_evidence_at_call, direct_function_call_target_at_call,
    imported_function_call_target_at_call, imported_member_call_target_at_call,
    language_core_evidence_provenance,
};

pub(super) fn sp(n: u32) -> Span {
    Span::new(FileId(0), n, n + 1, n, n)
}

pub(super) fn wide_sp(start: u32, end: u32) -> Span {
    Span::new(FileId(0), start, end, start, end)
}

pub(super) fn language_core_provenance(lang: Lang) -> EvidenceProvenance {
    let (pack_id, producer_id) = language_core_evidence_provenance(lang);
    EvidenceProvenance {
        emitter: EvidenceEmitter::FirstParty,
        pack_hash: Some(stable_symbol_hash(pack_id)),
        rule_hash: Some(stable_symbol_hash(producer_id)),
    }
}

fn evidence_with_dependencies(
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
            pack_hash: Some(stable_symbol_hash(BUILTIN_COMPAT_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("test")),
        },
        dependencies,
        status,
    }
}

pub(super) fn binding_symbol(
    id: u32,
    span: Span,
    local: &str,
    symbol: SymbolEvidenceKind,
    status: EvidenceStatus,
) -> EvidenceRecord {
    evidence_with_dependencies(
        id,
        EvidenceAnchor::binding(span, stable_symbol_hash(local)),
        EvidenceKind::Symbol(symbol),
        status,
        Vec::new(),
    )
}

pub(super) fn function_with_call(
    interner: &Interner,
    func_name: &str,
    callee_name: &str,
    duplicate_unit: bool,
) -> (Il, NodeId, NodeId) {
    let mut b = IlBuilder::new(FileId(0));
    let func_sym = interner.intern(func_name);
    let callee_sym = interner.intern(callee_name);
    let callee = b.add(NodeKind::Var, Payload::Name(callee_sym), sp(10), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(11), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(12), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(13), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp(14), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(15), &[func]);
    let mut units = vec![Unit {
        root: func,
        kind: UnitKind::Function,
        name: Some(func_sym),
        origin: Default::default(),
    }];
    if duplicate_unit {
        units.push(Unit {
            root: func,
            kind: UnitKind::Function,
            name: Some(func_sym),
            origin: Default::default(),
        });
    }
    let il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        units,
        Vec::new(),
    );
    (il, func, call)
}

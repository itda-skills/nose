use super::support::*;

#[test]
fn emits_imported_function_call_target_from_binding_symbol() {
    let interner = Interner::new();
    let p = interner.intern("p");
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(NodeKind::Var, Payload::Name(p), sp(10), &[]);
    let arg = b.add(NodeKind::Lit, Payload::LitInt(3), sp(11), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(12), &[callee, arg]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(13), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(14), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, wide_sp(8, 20), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, wide_sp(0, 30), &[func]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(binding_symbol(
        0,
        sp(1),
        "p",
        SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
        },
        EvidenceStatus::Asserted,
    ));

    run(&mut il, &interner);

    let expected = CallTargetEvidenceKind::ImportedFunction {
        module_hash: stable_symbol_hash("math"),
        exported_hash: stable_symbol_hash("prod"),
        local_hash: interner.symbol_hash(p),
    };
    assert_eq!(
        call_target_evidence_at_call(&il, &interner, call),
        Some(expected)
    );
    assert!(imported_function_call_target_at_call(&il, &interner, call));
    let target_record = il
        .evidence
        .iter()
        .find(|record| record.kind == EvidenceKind::CallTarget(expected))
        .expect("imported function call-target evidence");
    let [occurrence_dependency] = target_record.dependencies.as_slice() else {
        panic!("call-target should depend on exactly one occurrence symbol");
    };
    let occurrence = il
        .evidence_record_by_id(*occurrence_dependency)
        .expect("occurrence dependency");
    assert_eq!(
        occurrence.kind,
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("math"),
            exported_hash: stable_symbol_hash("prod"),
        })
    );
    assert_eq!(occurrence.dependencies, vec![EvidenceId(0)]);
}

#[test]
fn emits_imported_member_call_target_from_namespace_symbol() {
    let interner = Interner::new();
    let m = interner.intern("m");
    let prod = interner.intern("prod");
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Name(m), sp(10), &[]);
    let callee = b.add(NodeKind::Field, Payload::Name(prod), sp(11), &[receiver]);
    let arg = b.add(NodeKind::Lit, Payload::LitInt(3), sp(12), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(13), &[callee, arg]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(14), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(15), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, wide_sp(8, 20), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, wide_sp(0, 30), &[func]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(binding_symbol(
        0,
        sp(1),
        "m",
        SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("math"),
        },
        EvidenceStatus::Asserted,
    ));

    run(&mut il, &interner);

    let member_hash = interner.symbol_hash(prod);
    assert_eq!(
        call_target_evidence_at_call(&il, &interner, call),
        Some(CallTargetEvidenceKind::ImportedMember {
            module_hash: stable_symbol_hash("math"),
            exported_hash: member_hash,
            member_hash,
        })
    );
    assert!(imported_member_call_target_at_call(&il, &interner, call));
}

#[test]
fn emits_imported_member_call_target_from_imported_binding_receiver() {
    let interner = Interner::new();
    let map = interner.intern("Map");
    let of = interner.intern("of");
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Name(map), sp(10), &[]);
    let callee = b.add(NodeKind::Field, Payload::Name(of), sp(11), &[receiver]);
    let call = b.add(NodeKind::Call, Payload::None, sp(12), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(13), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(14), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, wide_sp(8, 20), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, wide_sp(0, 30), &[func]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Java,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(binding_symbol(
        0,
        sp(1),
        "Map",
        SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("java.util"),
            exported_hash: stable_symbol_hash("Map"),
        },
        EvidenceStatus::Asserted,
    ));

    run(&mut il, &interner);

    assert_eq!(
        call_target_evidence_at_call(&il, &interner, call),
        Some(CallTargetEvidenceKind::ImportedMember {
            module_hash: stable_symbol_hash("java.util"),
            exported_hash: stable_symbol_hash("Map"),
            member_hash: interner.symbol_hash(of),
        })
    );
}

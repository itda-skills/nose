use super::*;

#[test]
fn library_api_evidence_resolution_accepts_import_backed_callees() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("Values");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(10), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(10), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(10), &[lhs, rhs]);
    let callee = b.add(NodeKind::Var, Payload::Name(local), sp(11), &[]);
    let arg = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(12),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(13), &[callee, arg]);
    let root = b.add(NodeKind::Module, Payload::None, sp(9), &[import, call]);
    let mut il = finish_il(b, root, Lang::Python);
    let contract =
        library_imported_collection_factory_contract(Lang::Python, "collections", "deque")
            .expect("deque contract");
    let binding_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("collections"),
        exported_hash: stable_symbol_hash("deque"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(10), stable_symbol_hash("Values")),
        binding_symbol,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(sp(11), NodeKind::Var),
        binding_symbol,
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));
    il.evidence.push(library_api_record(
        2,
        sp(13),
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[1],
    ));
    assert_eq!(
        contract_status_for_call(&il, &interner, call, contract.id, contract.callee),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn library_api_evidence_resolution_accepts_source_backed_callees() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let regex = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("/x/")),
        sp(20),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("test")),
        sp(21),
        &[regex],
    );
    let arg = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("s")),
        sp(22),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(23), &[callee, arg]);
    let root = b.add(NodeKind::Module, Payload::None, sp(19), &[call]);
    let mut il = finish_il(b, root, Lang::JavaScript);
    let contract =
        library_regex_test_contract(Lang::JavaScript, "test", 1).expect("regex contract");
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(sp(20)),
        EvidenceKind::Source(SourceFactKind::Literal(SourceLiteralKind::Regex)),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(library_api_record(
        1,
        sp(23),
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    ));
    assert_eq!(
        contract_status_for_call(&il, &interner, call, contract.id, contract.callee),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn library_api_evidence_resolution_accepts_free_name_backed_callees() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("list")),
        sp(40),
        &[],
    );
    let arg = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(41),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(42), &[callee, arg]);
    let root = b.add(NodeKind::Module, Payload::None, sp(39), &[call]);
    let mut il = finish_il(b, root, Lang::Python);
    let contract = library_free_name_collection_factory_contract(Lang::Python, "list")
        .expect("Python list contract");
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(40), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("list"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(library_api_record(
        1,
        sp(42),
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    ));
    assert_eq!(
        contract_status_for_call(&il, &interner, call, contract.id, contract.callee),
        LibraryApiEvidenceStatus::Admitted
    );
    assert_eq!(
        library_api_contract_evidence_at_call_span(
            &il,
            &interner,
            LibraryApiSpanEvidenceQuery {
                call_span: Some(sp(42)),
                callee_span: Some(sp(40)),
                receiver_span: None,
                id: contract.id,
                callee: contract.callee,
                arg_count: 1,
            },
        ),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn library_api_evidence_resolution_accepts_free_function_builtin_callees() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("len")),
        sp(45),
        &[],
    );
    let arg = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(46),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(47), &[callee, arg]);
    let root = b.add(NodeKind::Module, Payload::None, sp(44), &[call]);
    let mut il = finish_il(b, root, Lang::Python);
    let contract = library_free_function_builtin_contract(Lang::Python, "len", 1)
        .expect("Python len contract");
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(45), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("len"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(library_api_record(
        1,
        sp(47),
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0],
    ));
    assert_eq!(
        contract_status_for_call(&il, &interner, call, contract.id, contract.callee),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn library_api_evidence_resolution_accepts_require_backed_callees() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let require_callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("require")),
        sp(48),
        &[],
    );
    let require_arg = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("set")),
        sp(48),
        &[],
    );
    let require_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(48),
        &[require_callee, require_arg],
    );
    let set = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Set")),
        sp(50),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("new")),
        sp(51),
        &[set],
    );
    let arg = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(52),
        &[],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(53), &[callee, arg]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(49),
        &[require_call, call],
    );
    let mut il = finish_il(b, root, Lang::Ruby);
    let contract = library_ruby_set_factory_contract(Lang::Ruby, "Set", "new", 1).expect("Set.new");
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(50), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Set"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(48), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("require"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        2,
        EvidenceAnchor::source_span(sp(48)),
        EvidenceKind::Import(ImportEvidenceKind::Require {
            module_hash: stable_symbol_hash("set"),
        }),
        EvidenceStatus::Asserted,
        vec![EvidenceId(1)],
    ));
    il.evidence.push(library_api_record(
        3,
        sp(53),
        contract.id,
        contract.callee,
        EvidenceStatus::Asserted,
        &[0, 2],
    ));
    assert_eq!(
        contract_status_for_call(&il, &interner, call, contract.id, contract.callee),
        LibraryApiEvidenceStatus::Admitted
    );
}

#[test]
fn library_api_evidence_resolution_accepts_import_binding_outside_unit_root() {
    let interner = Interner::new();
    let (mut il, call, _root, _local, contract) = java_list_of_import_evidence_il(&interner, false);
    assert_eq!(
        library_api_contract_evidence_for_call(
            &il,
            &interner,
            call,
            contract.id,
            contract.callee,
            1,
        ),
        LibraryApiEvidenceStatus::Admitted
    );
    assert_eq!(
        library_api_contract_evidence_at_call_span(
            &il,
            &interner,
            LibraryApiSpanEvidenceQuery {
                call_span: Some(sp(34)),
                callee_span: Some(sp(32)),
                receiver_span: Some(sp(30)),
                id: contract.id,
                callee: contract.callee,
                arg_count: 1,
            },
        ),
        LibraryApiEvidenceStatus::Rejected
    );
    assert_eq!(
        library_api_contract_evidence_at_call_span(
            &il,
            &interner,
            LibraryApiSpanEvidenceQuery {
                call_span: Some(sp(34)),
                callee_span: Some(sp(32)),
                receiver_span: None,
                id: contract.id,
                callee: contract.callee,
                arg_count: 1,
            },
        ),
        LibraryApiEvidenceStatus::Admitted
    );

    il.evidence.push(evidence(
        3,
        EvidenceAnchor::binding(sp(36), stable_symbol_hash("List")),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("other.module"),
            exported_hash: stable_symbol_hash("List"),
        }),
        EvidenceStatus::Asserted,
    ));
    assert_eq!(
        library_api_contract_evidence_for_call(
            &il,
            &interner,
            call,
            contract.id,
            contract.callee,
            1,
        ),
        LibraryApiEvidenceStatus::Rejected
    );
}

#[test]
fn library_api_evidence_resolution_rejects_shadowed_java_static_members() {
    let interner = Interner::new();
    let (mut il, call, root, local, contract) = java_list_of_import_evidence_il(&interner, true);
    assert_eq!(
        library_api_contract_evidence_for_call(
            &il,
            &interner,
            call,
            contract.id,
            contract.callee,
            1,
        ),
        LibraryApiEvidenceStatus::Admitted
    );

    il.units.push(Unit {
        root,
        kind: UnitKind::Class,
        name: Some(local),
        origin: Default::default(),
    });
    assert_eq!(
        library_api_contract_evidence_for_call(
            &il,
            &interner,
            call,
            contract.id,
            contract.callee,
            1,
        ),
        LibraryApiEvidenceStatus::Rejected
    );
}

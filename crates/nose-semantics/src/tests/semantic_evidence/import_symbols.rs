use super::*;

fn import_fact_evidence(
    id: u32,
    lang: Lang,
    span: Span,
    kind: EvidenceKind,
    status: EvidenceStatus,
) -> EvidenceRecord {
    import_fact_evidence_with_provenance(
        id,
        span,
        kind,
        language_core_provenance(lang),
        status,
        Vec::new(),
    )
}

fn import_fact_evidence_with_provenance(
    id: u32,
    span: Span,
    kind: EvidenceKind,
    provenance: EvidenceProvenance,
    status: EvidenceStatus,
    dependencies: Vec<EvidenceId>,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId(id),
        anchor: EvidenceAnchor::sequence(span),
        kind,
        provenance,
        dependencies,
        status,
    }
}

fn language_core_provenance(lang: Lang) -> EvidenceProvenance {
    let (pack_id, producer_id) = language_core_evidence_provenance(lang);
    EvidenceProvenance {
        emitter: EvidenceEmitter::FirstParty,
        pack_hash: Some(stable_symbol_hash(pack_id)),
        rule_hash: Some(stable_symbol_hash(producer_id)),
    }
}

fn binding_import_fact(module: &str, exported: &str) -> EvidenceKind {
    EvidenceKind::Import(ImportEvidenceKind::Binding {
        module_hash: stable_symbol_hash(module),
        exported_hash: stable_symbol_hash(exported),
    })
}

fn namespace_import_fact(module: &str) -> EvidenceKind {
    EvidenceKind::Import(ImportEvidenceKind::Namespace {
        module_hash: stable_symbol_hash(module),
    })
}

#[test]
fn import_fact_contracts_resolve_evidence_only_binding_and_namespace_proofs() {
    let mut b = IlBuilder::new(FileId(0));
    let collections = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("collections")),
        sp(1),
        &[],
    );
    let deque = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("deque")),
        sp(1),
        &[],
    );
    let binding = b.add(NodeKind::Seq, Payload::None, sp(1), &[collections, deque]);
    let math = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("math")),
        sp(2),
        &[],
    );
    let namespace = b.add(NodeKind::Seq, Payload::None, sp(2), &[math]);
    let raw_coordinates = b.add(NodeKind::Seq, Payload::None, sp(3), &[math]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(1),
        &[binding, namespace, raw_coordinates],
    );
    let mut il = finish_il(b, root, Lang::Python);

    assert_eq!(
        import_fact_contract(ImportFactKind::Binding).channel,
        ChannelEligibility::ExactProven
    );
    assert_eq!(import_fact_evidence_rhs(&il, binding), None);
    assert_eq!(import_fact_evidence_rhs(&il, namespace), None);

    il.evidence.push(import_fact_evidence(
        0,
        Lang::Python,
        sp(1),
        binding_import_fact("collections", "deque"),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(import_fact_evidence(
        1,
        Lang::Python,
        sp(2),
        namespace_import_fact("math"),
        EvidenceStatus::Asserted,
    ));

    assert_eq!(
        import_fact_evidence_rhs(&il, binding),
        Some(ImportFact {
            kind: ImportFactKind::Binding,
            module_hash: stable_symbol_hash("collections"),
            exported_hash: Some(stable_symbol_hash("deque")),
        })
    );
    assert_eq!(
        import_fact_evidence_rhs(&il, namespace),
        Some(ImportFact {
            kind: ImportFactKind::Namespace,
            module_hash: stable_symbol_hash("math"),
            exported_hash: None,
        })
    );
    assert_eq!(import_fact_evidence_rhs(&il, raw_coordinates), None);
}

#[test]
fn import_fact_admission_requires_matching_language_core_provenance() {
    let mut b = IlBuilder::new(FileId(0));
    let module = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("collections")),
        sp(10),
        &[],
    );
    let exported = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("deque")),
        sp(10),
        &[],
    );
    let external = b.add(NodeKind::Seq, Payload::None, sp(10), &[module, exported]);
    let broad_builtin = b.add(NodeKind::Seq, Payload::None, sp(11), &[module, exported]);
    let wrong_language = b.add(NodeKind::Seq, Payload::None, sp(12), &[module, exported]);
    let no_pack = b.add(NodeKind::Seq, Payload::None, sp(13), &[module, exported]);
    let broken_dependency = b.add(NodeKind::Seq, Payload::None, sp(14), &[module, exported]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(10),
        &[
            external,
            broad_builtin,
            wrong_language,
            no_pack,
            broken_dependency,
        ],
    );
    let mut il = finish_il(b, root, Lang::Python);
    let kind = binding_import_fact("collections", "deque");

    il.evidence.push(import_fact_evidence_with_provenance(
        0,
        sp(10),
        kind,
        EvidenceProvenance {
            emitter: EvidenceEmitter::External,
            pack_hash: Some(stable_symbol_hash("com.example.pack")),
            rule_hash: Some(stable_symbol_hash("python.language.core")),
        },
        EvidenceStatus::Asserted,
        Vec::new(),
    ));
    il.evidence.push(import_fact_evidence_with_provenance(
        1,
        sp(11),
        kind,
        EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: Some(stable_symbol_hash(FIRST_PARTY_PACK_ID)),
            rule_hash: Some(stable_symbol_hash("import_fact")),
        },
        EvidenceStatus::Asserted,
        Vec::new(),
    ));
    il.evidence.push(import_fact_evidence(
        2,
        Lang::Java,
        sp(12),
        kind,
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(import_fact_evidence_with_provenance(
        3,
        sp(13),
        kind,
        EvidenceProvenance {
            emitter: EvidenceEmitter::FirstParty,
            pack_hash: None,
            rule_hash: Some(stable_symbol_hash("python.language.core")),
        },
        EvidenceStatus::Asserted,
        Vec::new(),
    ));
    il.evidence.push(import_fact_evidence_with_provenance(
        4,
        sp(14),
        kind,
        language_core_provenance(Lang::Python),
        EvidenceStatus::Asserted,
        vec![EvidenceId(99)],
    ));

    assert_eq!(import_fact_evidence_rhs(&il, external), None);
    assert_eq!(import_fact_evidence_rhs(&il, broad_builtin), None);
    assert_eq!(import_fact_evidence_rhs(&il, wrong_language), None);
    assert_eq!(import_fact_evidence_rhs(&il, no_pack), None);
    assert_eq!(import_fact_evidence_rhs(&il, broken_dependency), None);
}

#[test]
fn ambiguous_import_evidence_stays_closed_without_raw_seq_fallback() {
    let mut b = IlBuilder::new(FileId(0));
    let module = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("collections")),
        sp(10),
        &[],
    );
    let exported = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("deque")),
        sp(10),
        &[],
    );
    let binding = b.add(NodeKind::Seq, Payload::None, sp(10), &[module, exported]);
    let root = b.add(NodeKind::Module, Payload::None, sp(10), &[binding]);
    let mut il = finish_il(b, root, Lang::Python);
    il.evidence.push(import_fact_evidence(
        0,
        Lang::Python,
        sp(10),
        namespace_import_fact("math"),
        EvidenceStatus::Asserted,
    ));

    assert_eq!(
        import_fact_evidence_rhs(&il, binding),
        Some(ImportFact {
            kind: ImportFactKind::Namespace,
            module_hash: stable_symbol_hash("math"),
            exported_hash: None,
        })
    );

    il.evidence.push(import_fact_evidence(
        1,
        Lang::Python,
        sp(10),
        binding_import_fact("collections", "deque"),
        EvidenceStatus::Asserted,
    ));
    assert_eq!(import_fact_evidence_rhs(&il, binding), None);
}

#[test]
fn imported_symbol_identity_does_not_fall_back_to_raw_import_seq() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("deque");
    let module = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("collections")),
        sp(30),
        &[],
    );
    let exported = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("deque")),
        sp(30),
        &[],
    );
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(30), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(30), &[module, exported]);
    let assignment = b.add(NodeKind::Assign, Payload::None, sp(30), &[lhs, rhs]);
    let use_site = b.add(NodeKind::Var, Payload::Name(local), sp(31), &[]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(30),
        &[assignment, use_site],
    );
    let mut il = finish_il(b, root, Lang::Python);

    assert_eq!(import_fact_evidence_rhs(&il, rhs), None);
    assert!(!imported_binding_symbol(
        &il,
        &interner,
        use_site,
        "collections",
        "deque"
    ));

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(30), stable_symbol_hash("deque")),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
            module_hash: stable_symbol_hash("collections"),
            exported_hash: stable_symbol_hash("deque"),
        }),
        EvidenceStatus::Asserted,
    ));
    assert!(imported_binding_symbol(
        &il,
        &interner,
        use_site,
        "collections",
        "deque"
    ));
}

#[test]
fn imported_occurrence_symbol_evidence_requires_binding_dependency() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local_hash = stable_symbol_hash("m");
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("m")),
        sp(20),
        &[],
    );
    let root = b.add(NodeKind::Module, Payload::None, sp(20), &[receiver]);
    let mut il = finish_il(b, root, Lang::Python);
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(20), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("math"),
        }),
        EvidenceStatus::Asserted,
    ));

    assert!(!imported_namespace_symbol(&il, &interner, receiver, "math"));

    il.evidence.clear();
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(19), local_hash),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("math"),
        }),
        EvidenceStatus::Asserted,
    ));
    il.evidence.push(evidence_with_dependencies(
        1,
        EvidenceAnchor::node(sp(20), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("math"),
        }),
        EvidenceStatus::Asserted,
        vec![EvidenceId(0)],
    ));

    assert!(imported_namespace_symbol(&il, &interner, receiver, "math"));
    assert!(!imported_namespace_symbol(
        &il,
        &interner,
        receiver,
        "collections"
    ));
}

#[test]
fn symbol_evidence_blocks_import_assignment_fallback() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("math");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(21), &[]);
    let module = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("math")),
        sp(21),
        &[],
    );
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(21), &[module]);
    let assign = b.add(NodeKind::Assign, Payload::None, sp(21), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(22), &[]);
    let root = b.add(NodeKind::Module, Payload::None, sp(21), &[assign, receiver]);
    let mut il = finish_il(b, root, Lang::Python);
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(21), stable_symbol_hash("math")),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("other"),
        }),
        EvidenceStatus::Asserted,
    ));

    assert!(!imported_namespace_symbol(&il, &interner, receiver, "math"));
}

#[test]
fn binding_symbol_evidence_does_not_prove_rebound_alias_uses() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let local = interner.intern("math");
    let lhs = b.add(NodeKind::Var, Payload::Name(local), sp(24), &[]);
    let module = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("math")),
        sp(24),
        &[],
    );
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(24), &[module]);
    let import_assign = b.add(NodeKind::Assign, Payload::None, sp(24), &[lhs, rhs]);
    let rebound_lhs = b.add(NodeKind::Var, Payload::Name(local), sp(25), &[]);
    let rebound_rhs = b.add(NodeKind::Lit, Payload::LitInt(0), sp(25), &[]);
    let rebound = b.add(
        NodeKind::Assign,
        Payload::None,
        sp(25),
        &[rebound_lhs, rebound_rhs],
    );
    let receiver = b.add(NodeKind::Var, Payload::Name(local), sp(26), &[]);
    let root = b.add(
        NodeKind::Module,
        Payload::None,
        sp(24),
        &[import_assign, rebound, receiver],
    );
    let mut il = finish_il(b, root, Lang::Python);
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(24), stable_symbol_hash("math")),
        EvidenceKind::Symbol(SymbolEvidenceKind::ImportedNamespace {
            module_hash: stable_symbol_hash("math"),
        }),
        EvidenceStatus::Asserted,
    ));

    assert!(!imported_namespace_symbol(&il, &interner, receiver, "math"));
}

#[test]
fn global_symbol_requires_asserted_evidence() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let math = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Math")),
        sp(23),
        &[],
    );
    let root = b.add(NodeKind::Module, Payload::None, sp(23), &[math]);
    let mut il = finish_il(b, root, Lang::JavaScript);

    assert!(
        !asserted_unshadowed_global_symbol(&il, math, "Math"),
        "a bare spelling without Symbol evidence must not open the exact path"
    );

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(23), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Math"),
        }),
        EvidenceStatus::Ambiguous,
    ));
    assert!(
        !asserted_unshadowed_global_symbol(&il, math, "Math"),
        "ambiguous Symbol evidence keeps the exact path closed"
    );

    il.evidence.clear();
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(23), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Math"),
        }),
        EvidenceStatus::Asserted,
    ));
    assert!(
        asserted_unshadowed_global_symbol(&il, math, "Math"),
        "asserted Symbol evidence proves the unshadowed global"
    );
}

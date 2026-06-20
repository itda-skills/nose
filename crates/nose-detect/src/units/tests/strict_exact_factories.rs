use super::support::*;
use crate::strict_exact::{
    strict_exact_java_collection_factory_safe, strict_exact_java_map_factory_safe,
    strict_exact_python_collection_factory_safe, strict_exact_safe_tree,
    strict_exact_set_constructor_collection_safe, StrictFacts,
};
use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceId, EvidenceKind, FileId, FileMeta, IlBuilder,
    ImportEvidenceKind, Interner, Lang, NodeKind, Payload, SequenceSurfaceKind, SymbolEvidenceKind,
};
use nose_semantics::{
    library_free_name_collection_factory_contract, library_java_collection_factory_contract,
    library_js_like_map_constructor_contract, library_js_like_set_constructor_contract,
};

#[test]
fn strict_exact_js_constructor_requires_library_api_evidence() {
    let interner = Interner::new();
    let (mut il, call) = js_new_set_il(&interner);
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_set_constructor_collection_safe(
        &il, &interner, &facts, call
    ));

    let wrong = library_js_like_map_constructor_contract(Lang::JavaScript, "Map").unwrap();
    il.evidence.push(library_api_contract_evidence(
        3,
        sp(13),
        wrong.id,
        wrong.callee,
        1,
        vec![EvidenceId(0), EvidenceId(1)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_set_constructor_collection_safe(
        &il, &interner, &facts, call
    ));

    let (mut il, call) = js_new_set_il(&interner);
    let set = library_js_like_set_constructor_contract(Lang::JavaScript, "Set").unwrap();
    il.evidence.push(library_api_contract_evidence(
        3,
        sp(13),
        set.id,
        set.callee,
        1,
        vec![EvidenceId(0), EvidenceId(1)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_set_constructor_collection_safe(
        &il, &interner, &facts, call
    ));
}

#[test]
fn strict_exact_python_builtin_factory_requires_library_api_evidence() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("list")),
        sp(40),
        &[],
    );
    let item = b.add(NodeKind::Lit, Payload::LitInt(1), sp(41), &[]);
    let seq = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(42),
        &[item],
    );
    let call = b.add(NodeKind::Call, Payload::None, sp(43), &[callee, seq]);
    let root = b.add(NodeKind::Block, Payload::None, sp(39), &[call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.py".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::sequence(sp(42)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_python_collection_factory_safe(
        &il, &interner, &facts, call
    ));

    let contract = library_free_name_collection_factory_contract(Lang::Python, "list").unwrap();
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(40), NodeKind::Var),
        EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("list"),
        }),
        Vec::new(),
    ));
    il.evidence.push(python_builtin_collection_factory_evidence(
        2,
        sp(43),
        contract,
        1,
        vec![EvidenceId(1)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_python_collection_factory_safe(
        &il, &interner, &facts, call
    ));
}

#[test]
fn strict_exact_java_collection_factory_uses_library_api_evidence() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let list = interner.intern("List");
    let lhs = b.add(NodeKind::Var, Payload::Name(list), sp(20), &[]);
    let rhs = b.add(NodeKind::Seq, Payload::None, sp(20), &[]);
    let import = b.add(NodeKind::Assign, Payload::None, sp(20), &[lhs, rhs]);
    let receiver = b.add(NodeKind::Var, Payload::Name(list), sp(21), &[]);
    let factory_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("of")),
        sp(22),
        &[receiver],
    );
    let left = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("red")),
        sp(23),
        &[],
    );
    let right = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("blue")),
        sp(24),
        &[],
    );
    let factory = b.add(
        NodeKind::Call,
        Payload::None,
        sp(25),
        &[factory_callee, left, right],
    );
    let contains_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("contains")),
        sp(26),
        &[factory],
    );
    let value = b.add(NodeKind::Var, Payload::Cid(0), sp(27), &[]);
    let contains = b.add(
        NodeKind::Call,
        Payload::None,
        sp(28),
        &[contains_callee, value],
    );
    let root = b.add(NodeKind::Block, Payload::None, sp(20), &[import, contains]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.java".into(),
            lang: Lang::Java,
        },
        Vec::new(),
        Vec::new(),
    );
    let contract = library_java_collection_factory_contract(Lang::Java, "List", "of")
        .expect("List.of contract");
    let binding_symbol = EvidenceKind::Symbol(SymbolEvidenceKind::ImportedBinding {
        module_hash: stable_symbol_hash("java.util"),
        exported_hash: stable_symbol_hash("List"),
    });
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(20), stable_symbol_hash("List")),
        binding_symbol,
        Vec::new(),
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(21), NodeKind::Var),
        binding_symbol,
        vec![EvidenceId(0)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_java_collection_factory_safe(
        &il, &interner, &facts, factory
    ));
    assert!(!strict_exact_safe_tree(&il, &interner, &facts, contains));

    push_java_factory_contract_evidence(&mut il, contract.id, contract.callee);
    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_java_collection_factory_safe(
        &il, &interner, &facts, factory
    ));
    assert!(strict_exact_safe_tree(&il, &interner, &facts, contains));

    let wrong = library_js_like_set_constructor_contract(Lang::JavaScript, "Set").unwrap();
    il.evidence.pop();
    il.evidence.pop();
    push_java_factory_contract_evidence(&mut il, wrong.id, wrong.callee);
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_java_collection_factory_safe(
        &il, &interner, &facts, factory
    ));
    assert!(!strict_exact_safe_tree(&il, &interner, &facts, contains));
}

#[test]
fn strict_exact_java_map_provider_proof_does_not_replace_receiver_identity() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("FakeMap")),
        sp(30),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("of")),
        sp(31),
        &[receiver],
    );
    let key = b.add(
        NodeKind::Lit,
        Payload::LitStr(stable_symbol_hash("k")),
        sp(32),
        &[],
    );
    let value = b.add(NodeKind::Lit, Payload::LitInt(1), sp(33), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(34), &[callee, key, value]);
    let root = b.add(NodeKind::Block, Payload::None, sp(34), &[call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.java".into(),
            lang: Lang::Java,
        },
        Vec::new(),
        Vec::new(),
    );
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(sp(34), NodeKind::Call),
        EvidenceKind::Import(ImportEvidenceKind::ImmutableLiteralExport {
            module_hash: stable_symbol_hash("t"),
            exported_hash: stable_symbol_hash("VALUES"),
            root_kind: NodeKind::Call,
        }),
        Vec::new(),
    ));

    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_java_map_factory_safe(
        &il, &interner, &facts, call
    ));
}

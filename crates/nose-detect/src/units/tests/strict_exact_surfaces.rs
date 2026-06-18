use super::support::*;
use crate::strict_exact::{
    function_binding_safe, strict_exact_collection_contains_call_safe,
    strict_exact_membership_collection_safe, strict_exact_safe_tree, StrictFacts,
};
use crate::units::fragments::call_may_mutate_blocked_cid;
use nose_il::{
    stable_symbol_hash, Builtin, EvidenceAnchor, EvidenceId, EvidenceKind, EvidenceStatus, FileId,
    FileMeta, IlBuilder, Interner, Lang, NodeKind, Payload, SequenceSurfaceKind, SourceFactKind,
    SourceOperatorKind,
};
use nose_semantics::{
    library_free_function_builtin_contract, library_js_like_set_constructor_contract,
};
use rustc_hash::FxHashSet;

#[test]
fn strict_exact_sequence_surfaces_require_evidence() {
    let interner = Interner::new();
    let (mut il, seq) = raw_array_seq_il(&interner);
    let facts = StrictFacts::collect(&il, &interner);

    assert!(!strict_exact_safe_tree(&il, &interner, &facts, seq));
    assert!(!strict_exact_membership_collection_safe(
        &il, &interner, &facts, seq
    ));

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::sequence(sp(61)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);

    assert!(strict_exact_safe_tree(&il, &interner, &facts, seq));
    assert!(strict_exact_membership_collection_safe(
        &il, &interner, &facts, seq
    ));
}

#[test]
fn strict_exact_typeof_requires_source_operator_evidence() {
    let interner = Interner::new();
    let (mut il, call) = js_typeof_call_il(&interner);
    let facts = StrictFacts::collect(&il, &interner);

    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "Call(Var(\"typeof\"), arg) must not be exact-safe by spelling alone"
    );

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::source_span(sp(44)),
        EvidenceKind::Source(SourceFactKind::Operator(SourceOperatorKind::Typeof)),
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);

    assert!(strict_exact_safe_tree(&il, &interner, &facts, call));
}

#[test]
fn strict_exact_raw_builtin_payload_requires_admission() {
    let interner = Interner::new();
    let (mut il, call) = canonical_python_abs_il();
    let facts = StrictFacts::collect(&il, &interner);

    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "canonical Abs payload alone must not make a call strict-exact safe"
    );

    let contract = library_free_function_builtin_contract(Lang::Python, "abs", 1)
        .expect("Python abs contract");
    il.evidence.push(library_api_contract_evidence(
        0,
        sp(72),
        contract.id,
        contract.callee,
        1,
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_safe_tree(&il, &interner, &facts, call));
}

#[test]
fn function_binding_safe_raw_builtin_payload_requires_admission() {
    let interner = Interner::new();
    let (mut il, call) = canonical_python_abs_il();
    let facts = StrictFacts::collect(&il, &interner);

    assert!(
        !function_binding_safe(&il, &interner, &facts, call, call),
        "function binding safety must not trust a raw canonical Abs payload"
    );

    let contract = library_free_function_builtin_contract(Lang::Python, "abs", 1)
        .expect("Python abs contract");
    il.evidence.push(library_api_contract_evidence(
        0,
        sp(72),
        contract.id,
        contract.callee,
        1,
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(function_binding_safe(&il, &interner, &facts, call, call));
}

#[test]
fn raw_append_payload_without_effect_does_not_bypass_mutation_blocking() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(7), sp(80), &[]);
    let appended = b.add(NodeKind::Lit, Payload::LitInt(1), sp(81), &[]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp(82),
        &[receiver, appended],
    );
    let root = b.add(NodeKind::Block, Payload::None, sp(79), &[append]);
    let il = b.finish(
        root,
        FileMeta {
            path: "t.py".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    let mut blocked = FxHashSet::default();
    blocked.insert(7);

    assert!(
        call_may_mutate_blocked_cid(&il, &interner, append, &blocked),
        "raw Append payload must not be treated as a proven non-mutating builtin"
    );
}

#[test]
fn strict_exact_contains_consumes_receiver_domain_evidence() {
    let interner = Interner::new();
    let (mut il, call, receiver_span) = ts_contains_call_il(&interner);
    let facts = StrictFacts::collect(&il, &interner);
    assert!(!strict_exact_safe_tree(&il, &interner, &facts, call));

    il.evidence.push(evidence(
        0,
        EvidenceAnchor::node(receiver_span, NodeKind::Var),
        EvidenceKind::Domain(nose_semantics::DomainEvidence::Collection),
        Vec::new(),
    ));
    il.evidence.push(method_call_library_api_evidence(
        1,
        Lang::TypeScript,
        "includes",
        sp(53),
        1,
        vec![EvidenceId(0)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_safe_tree(&il, &interner, &facts, call));

    il.evidence.push(evidence(
        2,
        EvidenceAnchor::node(receiver_span, NodeKind::Var),
        EvidenceKind::Domain(nose_semantics::DomainEvidence::Map),
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "conflicting receiver-domain evidence must close strict exact fallback"
    );
}

#[test]
fn strict_exact_contains_consumes_binding_domain_evidence() {
    let interner = Interner::new();
    let xs = interner.intern("xs");
    let mut b = IlBuilder::new(FileId(0));
    let lhs = b.add(NodeKind::Var, Payload::Cid(0), sp(30), &[]);
    let seq = b.add(NodeKind::Seq, Payload::None, sp(31), &[]);
    let assign = b.add(NodeKind::Assign, Payload::None, sp(30), &[lhs, seq]);
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(32), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("includes")),
        sp(33),
        &[receiver],
    );
    let item = b.add(NodeKind::Lit, Payload::LitInt(7), sp(34), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(35), &[callee, item]);
    let root = b.add(NodeKind::Block, Payload::None, sp(29), &[assign, call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.ts".into(),
            lang: Lang::TypeScript,
        },
        Vec::new(),
        vec![xs],
    );
    il.evidence.push(evidence(
        0,
        EvidenceAnchor::binding(sp(30), stable_symbol_hash("xs")),
        EvidenceKind::Domain(nose_semantics::DomainEvidence::Collection),
        Vec::new(),
    ));
    il.evidence.push(method_call_library_api_evidence(
        1,
        Lang::TypeScript,
        "includes",
        sp(35),
        1,
        vec![EvidenceId(0)],
    ));

    let facts = StrictFacts::collect(&il, &interner);
    assert!(strict_exact_collection_contains_call_safe(
        &il, &interner, &facts, call, callee, "includes"
    ));

    il.evidence.push(evidence(
        2,
        EvidenceAnchor::binding(sp(30), stable_symbol_hash("xs")),
        EvidenceKind::Domain(nose_semantics::DomainEvidence::Map),
        Vec::new(),
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(
        !strict_exact_collection_contains_call_safe(
            &il, &interner, &facts, call, callee, "includes"
        ),
        "conflicting binding-domain evidence must close strict exact receiver proof"
    );
}

#[test]
fn strict_exact_contains_does_not_use_result_domain_as_exact_tree_proof() {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let factory_callee = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Set")),
        sp(40),
        &[],
    );
    let seed = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        sp(41),
        &[],
    );
    let receiver = b.add(
        NodeKind::Call,
        Payload::None,
        sp(42),
        &[factory_callee, seed],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("includes")),
        sp(43),
        &[receiver],
    );
    let item = b.add(NodeKind::Lit, Payload::LitInt(7), sp(44), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(45), &[callee, item]);
    let root = b.add(NodeKind::Block, Payload::None, sp(39), &[call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t.ts".into(),
            lang: Lang::TypeScript,
        },
        Vec::new(),
        Vec::new(),
    );
    let facts = StrictFacts::collect(&il, &interner);
    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "call-result receiver must not be collection-like without domain evidence"
    );

    let api = library_js_like_set_constructor_contract(Lang::TypeScript, "Set").unwrap();
    il.evidence.push(library_api_contract_evidence(
        0,
        sp(42),
        api.id,
        api.callee,
        1,
        Vec::new(),
    ));
    il.evidence.push(evidence(
        1,
        EvidenceAnchor::node(sp(42), NodeKind::Call),
        EvidenceKind::Domain(nose_semantics::DomainEvidence::Set),
        vec![EvidenceId(0)],
    ));
    let facts = StrictFacts::collect(&il, &interner);
    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "result-domain evidence proves the call result's receiver domain, not the exact-safety of the receiver expression"
    );

    il.evidence[0].status = EvidenceStatus::Ambiguous;
    let facts = StrictFacts::collect(&il, &interner);
    assert!(
        !strict_exact_safe_tree(&il, &interner, &facts, call),
        "ambiguous LibraryApi dependency must close strict exact receiver proof"
    );
}

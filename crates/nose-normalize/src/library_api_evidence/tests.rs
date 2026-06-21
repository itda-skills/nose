use super::*;
use nose_il::{FileId, FileMeta, IlBuilder, Lang, Span};
use nose_semantics::admitted_builder_append_method_call_args;

fn sp(byte: u32) -> Span {
    Span::new(FileId(0), byte, byte + 1, byte, byte + 1)
}

fn method_call_il(
    interner: &mut Interner,
    lang: Lang,
    method: &str,
    arg_count: usize,
) -> (Il, NodeId, NodeId, Option<NodeId>) {
    let mut builder = IlBuilder::new(FileId(0));
    let name = interner.intern("r");
    let seed_span = sp(1);
    let seed = builder.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("array")),
        seed_span,
        &[],
    );
    let target = builder.add(NodeKind::Var, Payload::Name(name), sp(2), &[]);
    let assign = builder.add(NodeKind::Assign, Payload::None, sp(2), &[target, seed]);
    let receiver = builder.add(NodeKind::Var, Payload::Name(name), sp(3), &[]);
    let field = builder.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(3),
        &[receiver],
    );
    let args: Vec<NodeId> = (0..arg_count)
        .map(|idx| builder.add(NodeKind::Var, Payload::Cid((idx + 1) as u32), sp(4), &[]))
        .collect();
    let first_arg = args.first().copied();
    let mut children = Vec::with_capacity(args.len() + 1);
    children.push(field);
    children.extend(args);
    let call = builder.add(NodeKind::Call, Payload::None, sp(5), &children);
    let root = builder.add(NodeKind::Func, Payload::None, sp(6), &[assign, call]);
    let mut il = builder.finish(
        root,
        FileMeta {
            path: "method".into(),
            lang,
        },
        Vec::new(),
        Vec::new(),
    );
    let (pack_id, producer_id) = language_core_evidence_provenance(lang);
    il.find_or_push_first_party_evidence(
        EvidenceAnchor::sequence(seed_span),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        pack_id,
        producer_id,
        Vec::new(),
    );
    (il, call, receiver, first_arg)
}

#[test]
fn builder_append_method_api_evidence_admits_first_party_rows() {
    for (lang, method) in [
        (Lang::Python, "append"),
        (Lang::JavaScript, "push"),
        (Lang::Java, "add"),
        (Lang::Rust, "push"),
    ] {
        let mut interner = Interner::new();
        let (mut il, call, receiver, item) = method_call_il(&mut interner, lang, method, 1);

        run(&mut il, &interner);

        let (admitted_receiver, admitted_item) =
            admitted_builder_append_method_call_args(&il, &interner, call)
                .expect("builder append method evidence");
        assert_eq!(admitted_receiver, receiver);
        assert_eq!(Some(admitted_item), item);
    }
}

#[test]
fn builder_append_method_api_evidence_is_language_and_arity_scoped() {
    for (lang, method, arg_count) in [
        (Lang::Ruby, "push", 1),
        (Lang::Python, "append", 2),
        (Lang::JavaScript, "push", 2),
    ] {
        let mut interner = Interner::new();
        let (mut il, call, _, _) = method_call_il(&mut interner, lang, method, arg_count);

        run(&mut il, &interner);

        assert!(admitted_builder_append_method_call_args(&il, &interner, call).is_none());
    }
}

#[test]
fn builder_append_method_api_evidence_closes_on_conflicting_sequence_surface_seed() {
    let mut interner = Interner::new();
    let (mut il, call, _, _) = method_call_il(&mut interner, Lang::JavaScript, "push", 1);
    let (pack_id, producer_id) = language_core_evidence_provenance(Lang::JavaScript);
    il.find_or_push_first_party_evidence(
        EvidenceAnchor::sequence(sp(1)),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Map),
        pack_id,
        producer_id,
        Vec::new(),
    );

    run(&mut il, &interner);

    assert!(
        admitted_builder_append_method_call_args(&il, &interner, call).is_none(),
        "conflicting sequence-surface proof must not seed builder append API evidence"
    );
}

#[test]
fn builder_append_method_api_evidence_rejects_untagged_sequence_surface_seed() {
    let interner = Interner::new();
    let mut builder = IlBuilder::new(FileId(0));
    let name = interner.intern("r");
    let seed_span = sp(1);
    let seed = builder.add(NodeKind::Seq, Payload::None, seed_span, &[]);
    let target = builder.add(NodeKind::Var, Payload::Name(name), sp(2), &[]);
    let assign = builder.add(NodeKind::Assign, Payload::None, sp(2), &[target, seed]);
    let receiver = builder.add(NodeKind::Var, Payload::Name(name), sp(3), &[]);
    let field = builder.add(
        NodeKind::Field,
        Payload::Name(interner.intern("push")),
        sp(3),
        &[receiver],
    );
    let item = builder.add(NodeKind::Var, Payload::Cid(1), sp(4), &[]);
    let call = builder.add(NodeKind::Call, Payload::None, sp(5), &[field, item]);
    let root = builder.add(NodeKind::Func, Payload::None, sp(6), &[assign, call]);
    let mut il = builder.finish(
        root,
        FileMeta {
            path: "method".into(),
            lang: Lang::JavaScript,
        },
        Vec::new(),
        Vec::new(),
    );
    let (pack_id, producer_id) = language_core_evidence_provenance(Lang::JavaScript);
    il.find_or_push_first_party_evidence(
        EvidenceAnchor::sequence(seed_span),
        EvidenceKind::SequenceSurface(SequenceSurfaceKind::Collection),
        pack_id,
        producer_id,
        Vec::new(),
    );

    run(&mut il, &interner);

    assert!(
        admitted_builder_append_method_call_args(&il, &interner, call).is_none(),
        "untagged sequences must not become collection seeds from evidence alone"
    );
}

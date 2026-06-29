use super::support::*;

#[test]
fn emits_direct_function_call_target_for_unique_unshadowed_function() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", false);
    run(&mut il, &interner);
    assert!(direct_function_call_target_at_call(
        &il, &interner, call, func
    ));
    let target = CallTargetEvidenceKind::DirectFunction {
        target_span: il.node(func).span,
        name_hash: stable_symbol_hash("f"),
    };
    let record = il
        .evidence
        .iter()
        .find(|record| record.kind == EvidenceKind::CallTarget(target))
        .expect("direct function call-target evidence");
    assert_eq!(record.provenance, language_core_provenance(Lang::Python));
}

#[test]
fn emits_promise_like_domain_for_direct_async_function_call_result() {
    let interner = Interner::new();
    let load = interner.intern("load");
    let use_value = interner.intern("useValue");
    let async_tag = interner.intern("async_function");
    let mut b = IlBuilder::new(FileId(0));

    let payload = b.add(NodeKind::Lit, Payload::LitInt(1), sp(1), &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(2), &[payload]);
    let body = b.add(NodeKind::Block, Payload::None, sp(3), &[ret]);
    let async_boundary = b.add(NodeKind::Raw, Payload::Name(async_tag), sp(4), &[body]);
    let async_func = b.add(NodeKind::Func, Payload::None, sp(5), &[async_boundary]);

    let callee = b.add(NodeKind::Var, Payload::Name(load), sp(10), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(11), &[callee]);
    let caller_ret = b.add(NodeKind::Return, Payload::None, sp(12), &[call]);
    let caller_body = b.add(NodeKind::Block, Payload::None, sp(13), &[caller_ret]);
    let caller = b.add(NodeKind::Func, Payload::None, sp(14), &[caller_body]);
    let module = b.add(
        NodeKind::Module,
        Payload::None,
        sp(15),
        &[async_func, caller],
    );
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::TypeScript,
        },
        vec![
            Unit {
                root: async_func,
                kind: UnitKind::Function,
                name: Some(load),
                origin: Default::default(),
            },
            Unit {
                root: caller,
                kind: UnitKind::Function,
                name: Some(use_value),
                origin: Default::default(),
            },
        ],
        Vec::new(),
    );
    let protocol_id = EvidenceId(500);
    il.evidence.push(EvidenceRecord {
        id: protocol_id,
        anchor: EvidenceAnchor::source_span(il.node(async_boundary).span),
        kind: EvidenceKind::Source(SourceFactKind::Protocol(SourceProtocolKind::AsyncFunction)),
        provenance: language_core_provenance(Lang::TypeScript),
        dependencies: Vec::new(),
        status: EvidenceStatus::Asserted,
    });

    run(&mut il, &interner);

    assert!(direct_function_call_target_at_call(
        &il, &interner, call, async_func,
    ));
    let call_target = il
        .evidence
        .iter()
        .find(|record| {
            matches!(
                record.kind,
                EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction { .. })
            )
        })
        .expect("direct function call-target evidence");
    let domain = il
        .evidence
        .iter()
        .find(|record| {
            record.anchor == EvidenceAnchor::node(il.node(call).span, NodeKind::Call)
                && record.kind == EvidenceKind::Domain(DomainEvidence::PromiseLike)
        })
        .expect("PromiseLike domain evidence for async function call result");
    assert_eq!(domain.dependencies, vec![call_target.id, protocol_id]);
}

#[test]
fn updates_legacy_first_party_direct_function_call_target() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", false);
    let target = EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction {
        target_span: il.node(func).span,
        name_hash: stable_symbol_hash("f"),
    });
    il.find_or_push_first_party_evidence(
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        target,
        BUILTIN_COMPAT_PACK_ID,
        "legacy_direct_function_call_target",
        Vec::new(),
    );

    run(&mut il, &interner);

    let records: Vec<_> = il
        .evidence
        .iter()
        .filter(|record| {
            record.anchor == EvidenceAnchor::node(il.node(call).span, NodeKind::Call)
                && record.kind == target
        })
        .collect();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].provenance,
        language_core_provenance(Lang::Python)
    );
}

#[test]
fn does_not_promote_current_and_legacy_direct_function_duplicates() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", false);
    let target = EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction {
        target_span: il.node(func).span,
        name_hash: stable_symbol_hash("f"),
    });
    il.find_or_push_first_party_evidence_with_provenance(
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        target,
        language_core_provenance(Lang::Python),
        Vec::new(),
    );
    il.find_or_push_first_party_evidence(
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        target,
        BUILTIN_COMPAT_PACK_ID,
        "legacy_direct_function_call_target",
        Vec::new(),
    );

    run(&mut il, &interner);

    let current_records = il
        .evidence
        .iter()
        .filter(|record| {
            record.kind == target && record.provenance == language_core_provenance(Lang::Python)
        })
        .count();
    let legacy_records = il
        .evidence
        .iter()
        .filter(|record| {
            record.kind == target
                && record.provenance.pack_hash == Some(stable_symbol_hash(BUILTIN_COMPAT_PACK_ID))
        })
        .count();
    assert_eq!(current_records, 1);
    assert_eq!(legacy_records, 1);
    assert!(direct_function_call_target_at_call(
        &il, &interner, call, func
    ));
}

#[test]
fn does_not_emit_when_local_binder_shadows_function_name() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let mut b = IlBuilder::new(FileId(0));
    let param = b.add(NodeKind::Param, Payload::Name(f), sp(1), &[]);
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(2), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(3), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(4), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(5), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp(6), &[param, body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(7), &[func]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![Unit {
            root: func,
            kind: UnitKind::Function,
            name: Some(f),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(
        &il, &interner, call, func
    ));
}

#[test]
fn does_not_emit_for_duplicate_function_names() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", true);
    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(
        &il, &interner, call, func
    ));
}

#[test]
fn does_not_emit_for_method_bare_call() {
    let interner = Interner::new();
    let method_sym = interner.intern("fac");
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(NodeKind::Var, Payload::Name(method_sym), sp(20), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(21), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(22), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(23), &[ret]);
    let method = b.add(NodeKind::Func, Payload::None, sp(24), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(25), &[method]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Java,
        },
        vec![Unit {
            root: method,
            kind: UnitKind::Method,
            name: Some(method_sym),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(
        &il, &interner, call, method
    ));
}

#[test]
fn does_not_emit_for_nested_function_not_visible_as_top_level() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let mut b = IlBuilder::new(FileId(0));
    let nested_body = b.add(NodeKind::Block, Payload::None, sp(1), &[]);
    let nested = b.add(NodeKind::Func, Payload::None, sp(2), &[nested_body]);
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(3), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(4), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(5), &[call]);
    let outer_body = b.add(NodeKind::Block, Payload::None, sp(6), &[nested, ret]);
    let outer = b.add(NodeKind::Func, Payload::None, sp(7), &[outer_body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(8), &[outer]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![Unit {
            root: nested,
            kind: UnitKind::Function,
            name: Some(f),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(
        &il, &interner, call, nested
    ));
}

#[test]
fn does_not_emit_when_enclosing_scope_binds_function_name() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let g = interner.intern("g");
    let mut b = IlBuilder::new(FileId(0));

    let target_body = b.add(NodeKind::Block, Payload::None, sp(1), &[]);
    let target = b.add(NodeKind::Func, Payload::None, sp(2), &[target_body]);

    let shadow_lhs = b.add(NodeKind::Var, Payload::Name(f), sp(3), &[]);
    let shadow_rhs = b.add(NodeKind::Lit, Payload::LitInt(1), sp(4), &[]);
    let shadow = b.add(
        NodeKind::Assign,
        Payload::None,
        sp(5),
        &[shadow_lhs, shadow_rhs],
    );
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(6), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(7), &[callee]);
    let inner_ret = b.add(NodeKind::Return, Payload::None, sp(8), &[call]);
    let inner_body = b.add(NodeKind::Block, Payload::None, sp(9), &[inner_ret]);
    let inner = b.add(NodeKind::Func, Payload::None, sp(10), &[inner_body]);
    let outer_body = b.add(NodeKind::Block, Payload::None, sp(11), &[shadow, inner]);
    let outer = b.add(NodeKind::Func, Payload::None, sp(12), &[outer_body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(13), &[target, outer]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![
            Unit {
                root: target,
                kind: UnitKind::Function,
                name: Some(f),
                origin: Default::default(),
            },
            Unit {
                root: outer,
                kind: UnitKind::Function,
                name: Some(g),
                origin: Default::default(),
            },
        ],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(
        &il, &interner, call, target
    ));
}

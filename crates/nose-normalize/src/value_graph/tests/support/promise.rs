use super::*;

pub(in crate::value_graph::tests) fn promise_resolve_then_call_il(
    literal_arg: bool,
) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let promise = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Promise")),
        sp(90),
        &[],
    );
    let resolve_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("resolve")),
        sp(91),
        &[promise],
    );
    let arg = if literal_arg {
        b.add(NodeKind::Lit, Payload::LitInt(1), sp(92), &[])
    } else {
        b.add(
            NodeKind::Var,
            Payload::Name(interner.intern("maybeThenable")),
            sp(92),
            &[],
        )
    };
    let resolve_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(93),
        &[resolve_callee, arg],
    );
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(94),
        &[resolve_call],
    );
    let param = b.add(NodeKind::Param, Payload::Cid(0), sp(95), &[]);
    let param_ref = b.add(NodeKind::Var, Payload::Cid(0), sp(96), &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(97), &[]);
    let callback_body = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(98),
        &[param_ref, one],
    );
    let callback = b.add(
        NodeKind::Lambda,
        Payload::None,
        sp(99),
        &[param, callback_body],
    );
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(100),
        &[then_callee, callback],
    );
    let sync_left = b.add(NodeKind::Lit, Payload::LitInt(1), sp(101), &[]);
    let sync_right = b.add(NodeKind::Lit, Payload::LitInt(1), sp(102), &[]);
    let sync_add = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(103),
        &[sync_left, sync_right],
    );
    let root = b.add(
        NodeKind::Block,
        Payload::None,
        sp(104),
        &[then_call, sync_add],
    );
    (
        finish_test_il(b, root, Lang::TypeScript),
        interner,
        then_call,
        sync_add,
    )
}

pub(in crate::value_graph::tests) fn promise_like_receiver_then_call_il() -> (Il, Interner, NodeId)
{
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(110), &[]);
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(111),
        &[receiver],
    );
    let param = b.add(NodeKind::Param, Payload::Cid(1), sp(112), &[]);
    let param_ref = b.add(NodeKind::Var, Payload::Cid(1), sp(113), &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(114), &[]);
    let callback_body = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(115),
        &[param_ref, one],
    );
    let callback = b.add(
        NodeKind::Lambda,
        Payload::None,
        sp(116),
        &[param, callback_body],
    );
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(117),
        &[then_callee, callback],
    );
    (
        finish_test_il(b, then_call, Lang::TypeScript),
        interner,
        then_call,
    )
}

pub(in crate::value_graph::tests) fn promise_reject_catch_call_il() -> (Il, Interner, NodeId, NodeId)
{
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let reject_call = promise_static_call(&mut b, &interner, "reject", 1, 120);
    let catch_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("catch")),
        sp(124),
        &[reject_call],
    );
    let callback = add_increment_lambda(&mut b, 125, 1);
    let catch_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(130),
        &[catch_callee, callback],
    );
    let sync_add = add_sync_add(&mut b, 131);
    let root = b.add(
        NodeKind::Block,
        Payload::None,
        sp(134),
        &[catch_call, sync_add],
    );
    (
        finish_test_il(b, root, Lang::TypeScript),
        interner,
        catch_call,
        sync_add,
    )
}

pub(in crate::value_graph::tests) fn promise_reject_then_rejection_call_il(
) -> (Il, Interner, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let reject_call = promise_static_call(&mut b, &interner, "reject", 1, 140);
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(144),
        &[reject_call],
    );
    let undefined = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("undefined")),
        sp(145),
        &[],
    );
    let callback = add_increment_lambda(&mut b, 146, 1);
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(151),
        &[then_callee, undefined, callback],
    );
    (
        finish_test_il(b, then_call, Lang::TypeScript),
        interner,
        then_call,
    )
}

pub(in crate::value_graph::tests) fn promise_then_returning_factory_il(
    method: &str,
) -> (Il, Interner, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let resolve_call = promise_static_call(&mut b, &interner, "resolve", 1, 160);
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(164),
        &[resolve_call],
    );
    let param = b.add(NodeKind::Param, Payload::Cid(0), sp(165), &[]);
    let param_ref = b.add(NodeKind::Var, Payload::Cid(0), sp(166), &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(167), &[]);
    let sum = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(168),
        &[param_ref, one],
    );
    let factory_callee = {
        let promise = b.add(
            NodeKind::Var,
            Payload::Name(interner.intern("Promise")),
            sp(169),
            &[],
        );
        b.add(
            NodeKind::Field,
            Payload::Name(interner.intern(method)),
            sp(170),
            &[promise],
        )
    };
    let factory_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(171),
        &[factory_callee, sum],
    );
    let callback = b.add(
        NodeKind::Lambda,
        Payload::None,
        sp(172),
        &[param, factory_call],
    );
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(173),
        &[then_callee, callback],
    );
    (
        finish_test_il(b, then_call, Lang::TypeScript),
        interner,
        then_call,
    )
}

pub(in crate::value_graph::tests) fn promise_then_returning_unknown_il() -> (Il, Interner, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let resolve_call = promise_static_call(&mut b, &interner, "resolve", 1, 180);
    let then_callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("then")),
        sp(184),
        &[resolve_call],
    );
    let param = b.add(NodeKind::Param, Payload::Cid(0), sp(185), &[]);
    let maybe_thenable = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("maybeThenable")),
        sp(186),
        &[],
    );
    let callback = b.add(
        NodeKind::Lambda,
        Payload::None,
        sp(187),
        &[param, maybe_thenable],
    );
    let then_call = b.add(
        NodeKind::Call,
        Payload::None,
        sp(188),
        &[then_callee, callback],
    );
    (
        finish_test_il(b, then_call, Lang::TypeScript),
        interner,
        then_call,
    )
}

fn promise_static_call(
    b: &mut IlBuilder,
    interner: &Interner,
    method: &str,
    value: i64,
    base_line: u32,
) -> NodeId {
    let promise = b.add(
        NodeKind::Var,
        Payload::Name(interner.intern("Promise")),
        sp(base_line),
        &[],
    );
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(base_line + 1),
        &[promise],
    );
    let arg = b.add(
        NodeKind::Lit,
        Payload::LitInt(value),
        sp(base_line + 2),
        &[],
    );
    b.add(
        NodeKind::Call,
        Payload::None,
        sp(base_line + 3),
        &[callee, arg],
    )
}

fn add_increment_lambda(b: &mut IlBuilder, base_line: u32, cid: u32) -> NodeId {
    let param = b.add(NodeKind::Param, Payload::Cid(cid), sp(base_line), &[]);
    let param_ref = b.add(NodeKind::Var, Payload::Cid(cid), sp(base_line + 1), &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp(base_line + 2), &[]);
    let body = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(base_line + 3),
        &[param_ref, one],
    );
    b.add(
        NodeKind::Lambda,
        Payload::None,
        sp(base_line + 4),
        &[param, body],
    )
}

fn add_sync_add(b: &mut IlBuilder, base_line: u32) -> NodeId {
    let left = b.add(NodeKind::Lit, Payload::LitInt(1), sp(base_line), &[]);
    let right = b.add(NodeKind::Lit, Payload::LitInt(1), sp(base_line + 1), &[]);
    b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp(base_line + 2),
        &[left, right],
    )
}

pub(in crate::value_graph::tests) fn push_domain_evidence(
    il: &mut Il,
    node: NodeId,
    id: u32,
    domain: DomainEvidence,
) {
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::node(il.node(node).span, il.kind(node)),
        EvidenceKind::Domain(domain),
    ));
}

pub(in crate::value_graph::tests) fn push_promise_resolve_evidence(
    il: &mut Il,
    call: NodeId,
    base_id: u32,
) {
    push_promise_factory_evidence(il, call, base_id, "resolve");
}

pub(in crate::value_graph::tests) fn push_promise_reject_evidence(
    il: &mut Il,
    call: NodeId,
    base_id: u32,
) {
    push_promise_factory_evidence(il, call, base_id, "reject");
}

fn push_promise_factory_evidence(il: &mut Il, call: NodeId, base_id: u32, method: &str) {
    let [callee, _arg] = il.children(call) else {
        panic!("Promise factory test call must have one argument");
    };
    let callee = *callee;
    let [promise] = il.children(callee) else {
        panic!("Promise factory test callee must have Promise receiver");
    };
    let promise = *promise;
    let callee_span = il.node(callee).span;
    let promise_span = il.node(promise).span;
    let call_span = il.node(call).span;
    let root_id = EvidenceId(base_id);
    let qualified_id = EvidenceId(base_id + 1);
    let receiver_id = EvidenceId(base_id + 2);
    let api_id = EvidenceId(base_id + 3);
    let qualified_path = match method {
        "resolve" => "Promise.resolve",
        "reject" => "Promise.reject",
        _ => panic!("unsupported Promise factory test method"),
    };
    il.evidence.push(language_core_symbol_evidence(
        root_id.0,
        Lang::JavaScript,
        EvidenceAnchor::source_span(callee_span),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Promise"),
        },
    ));
    il.evidence.push(evidence_with_dependencies(
        qualified_id.0,
        EvidenceAnchor::node(callee_span, NodeKind::Field),
        EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal {
            path_hash: stable_symbol_hash(qualified_path),
        }),
        vec![root_id],
    ));
    il.evidence.push(language_core_symbol_evidence(
        receiver_id.0,
        Lang::JavaScript,
        EvidenceAnchor::node(promise_span, NodeKind::Var),
        SymbolEvidenceKind::UnshadowedGlobal {
            name_hash: stable_symbol_hash("Promise"),
        },
    ));
    let contract = library_promise_resolve_contract(il.meta.lang, "Promise", method, 1).unwrap();
    il.evidence.push(js_like_promise_evidence_with_dependencies(
        api_id.0,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 1,
        }),
        vec![qualified_id, receiver_id],
    ));
    il.evidence.push(evidence_with_dependencies(
        base_id + 4,
        EvidenceAnchor::node(call_span, NodeKind::Call),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        vec![api_id],
    ));
}

pub(in crate::value_graph::tests) fn push_promise_then_evidence(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
    id: u32,
) {
    let arg_count = il.children(call).len().saturating_sub(1);
    let contract = library_promise_then_contract(il.meta.lang, "then", arg_count).unwrap();
    let dependencies = nose_semantics::library_api_receiver_dependencies_for_call(
        il,
        interner,
        call,
        contract.callee,
    )
    .expect("Promise.then receiver dependencies");
    il.evidence.push(js_like_promise_evidence_with_dependencies(
        id,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: arg_count as u16,
        }),
        dependencies,
    ));
}

pub(in crate::value_graph::tests) fn push_promise_catch_evidence(
    il: &mut Il,
    interner: &Interner,
    call: NodeId,
    id: u32,
) {
    let contract = library_promise_catch_contract(il.meta.lang, "catch", 1).unwrap();
    let dependencies = nose_semantics::library_api_receiver_dependencies_for_call(
        il,
        interner,
        call,
        contract.callee,
    )
    .expect("Promise.catch receiver dependencies");
    il.evidence.push(js_like_promise_evidence_with_dependencies(
        id,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::LibraryApi(LibraryApiEvidenceKind::Contract {
            contract_hash: library_api_contract_id_hash(contract.id),
            callee_hash: library_api_callee_contract_hash(contract.callee),
            arity: 1,
        }),
        dependencies,
    ));
}

use super::support::*;

fn promise_resolve_then_call_il(literal_arg: bool) -> (Il, Interner, NodeId, NodeId) {
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

fn promise_like_receiver_then_call_il() -> (Il, Interner, NodeId) {
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

fn push_domain_evidence(il: &mut Il, node: NodeId, id: u32, domain: DomainEvidence) {
    il.evidence.push(evidence(
        id,
        EvidenceAnchor::node(il.node(node).span, il.kind(node)),
        EvidenceKind::Domain(domain),
    ));
}

fn push_promise_resolve_evidence(il: &mut Il, call: NodeId, base_id: u32) {
    let [callee, _arg] = il.children(call) else {
        panic!("Promise.resolve test call must have one argument");
    };
    let callee = *callee;
    let [promise] = il.children(callee) else {
        panic!("Promise.resolve test callee must have Promise receiver");
    };
    let promise = *promise;
    let callee_span = il.node(callee).span;
    let promise_span = il.node(promise).span;
    let call_span = il.node(call).span;
    let root_id = EvidenceId(base_id);
    let qualified_id = EvidenceId(base_id + 1);
    let receiver_id = EvidenceId(base_id + 2);
    let api_id = EvidenceId(base_id + 3);
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
            path_hash: stable_symbol_hash("Promise.resolve"),
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
    let contract = library_promise_resolve_contract(il.meta.lang, "Promise", "resolve", 1).unwrap();
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

fn push_promise_then_evidence(il: &mut Il, interner: &Interner, call: NodeId, id: u32) {
    let contract = library_promise_then_contract(il.meta.lang, "then", 1).unwrap();
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
            arity: 1,
        }),
        dependencies,
    ));
}

#[test]
fn promise_then_over_resolve_reduces_behind_promise_boundary() {
    let (mut il, interner, then_call, sync_add) = promise_resolve_then_call_il(true);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    let mut builder = Builder::new(&il, &interner);
    let promise_value = builder.eval(then_call, &FxHashMap::default());
    let payload = {
        let node = &builder.nodes[promise_value as usize];
        assert!(matches!(
            node.op,
            ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
        ));
        *node
            .args
            .first()
            .expect("Promise boundary wraps one payload")
    };
    assert!(matches!(
        builder.nodes[payload as usize].op,
        ValOp::Bin(op) if op == Op::Add as u32
    ));

    let sync_value = builder.eval(sync_add, &FxHashMap::default());
    assert_eq!(payload, sync_value);
    assert_ne!(
        promise_value, sync_value,
        "Promise-returning continuation must not converge with a synchronous payload"
    );
}

#[test]
fn promise_then_over_possible_thenable_resolve_arg_stays_opaque() {
    let (mut il, interner, then_call, _sync_add) = promise_resolve_then_call_il(false);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn promise_then_over_explicit_thenable_resolve_arg_stays_opaque() {
    let (mut il, interner, then_call, _sync_add) = promise_resolve_then_call_il(false);
    let resolve_call = il.children(il.children(then_call)[0])[0];
    let resolve_arg = il.children(resolve_call)[1];
    push_domain_evidence(&mut il, resolve_arg, 20, DomainEvidence::PromiseLike);
    push_promise_resolve_evidence(&mut il, resolve_call, 0);
    push_promise_then_evidence(&mut il, &interner, then_call, 5);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

#[test]
fn promise_like_receiver_without_supported_settled_producer_stays_opaque() {
    let (mut il, interner, then_call) = promise_like_receiver_then_call_il();
    let then_callee = il.children(then_call)[0];
    let receiver = il.children(then_callee)[0];
    push_domain_evidence(&mut il, receiver, 0, DomainEvidence::PromiseLike);
    push_promise_then_evidence(&mut il, &interner, then_call, 1);

    assert!(!matches!(
        eval_op(&il, &interner, then_call),
        ValOp::Call(code) if code == PROMISE_RESOLVED_CODE
    ));
}

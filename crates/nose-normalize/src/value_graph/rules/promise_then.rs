//! Promise `.then` continuation canonicalization.
//!
//! The rule is exact only for admitted JS-like Promise occurrences. A `.then` selector alone is
//! never proof. The receiver must be admitted by `LibraryApi(PromiseThen)` plus PromiseLike receiver
//! evidence, and this value-graph rule must also be able to recover a settled value from a supported
//! Promise producer. The result stays behind a Promise boundary so Promise-returning code does not
//! converge with synchronous code that happens to compute the same payload.
//!
//! proof-obligation: normalize.value_graph.promise_then

use super::super::{Builder, ValOp, ValueId};
use nose_il::{DomainEvidence, NodeId, NodeKind, Payload};
use nose_semantics::{
    admitted_promise_resolve_at_call, admitted_promise_then_at_call,
    asserted_unshadowed_global_symbol, nullish_global_contract, PromiseFactoryKind,
};
use rustc_hash::FxHashMap;

use super::super::ops::PROMISE_RESOLVED_CODE;

pub(in super::super) fn apply(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    let kids = builder.il.children(expr).to_vec();
    if kids.len() != 2 {
        return None;
    }
    let admitted = admitted_promise_then_at_call(builder.il, builder.interner, expr)?;
    let contract = admitted.contract.result;
    if !contract.demand.is_async_boundary() {
        return None;
    }
    let cb = kids[1];
    if builder.il.kind(cb) != NodeKind::Lambda {
        return None;
    }
    let recv = admitted.receiver?;
    let settled = promise_receiver_settled_value(builder, recv, env)?;
    let body = builder.eval_lambda_body(cb, &[settled], env)?;
    Some(promise_boundary(builder, body))
}

pub(in super::super) fn promise_resolve_value(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    let settled = promise_resolve_settled_value(builder, expr, env)?;
    Some(promise_boundary(builder, settled))
}

fn promise_receiver_settled_value(
    builder: &mut Builder<'_>,
    recv: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    if let Some(settled) = promise_resolve_settled_value(builder, recv, env) {
        return Some(settled);
    }
    let chained = apply(builder, recv, env)?;
    promise_boundary_payload(builder, chained)
}

fn promise_resolve_settled_value(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    let admitted = admitted_promise_resolve_at_call(builder.il, builder.interner, expr)?;
    if admitted.contract.result.kind != PromiseFactoryKind::Resolve {
        return None;
    }
    let kids = builder.il.children(expr);
    let arg = *kids.get(1)?;
    if !promise_resolve_arg_is_non_thenable_safe(builder, arg) {
        return None;
    }
    Some(builder.eval(arg, env))
}

fn promise_resolve_arg_is_non_thenable_safe(builder: &Builder<'_>, arg: NodeId) -> bool {
    match builder.il.kind(arg) {
        NodeKind::Lit => true,
        NodeKind::Var if nullish_global_arg_is_safe(builder, arg) => true,
        _ => builder
            .domain_evidence_of_expr(arg)
            .is_some_and(non_thenable_scalar_domain),
    }
}

fn nullish_global_arg_is_safe(builder: &Builder<'_>, arg: NodeId) -> bool {
    let Payload::Name(name) = builder.il.node(arg).payload else {
        return false;
    };
    let name = builder.interner.resolve(name);
    let Some(contract) = nullish_global_contract(builder.il.meta.lang, name) else {
        return false;
    };
    !contract.requires_unshadowed
        || asserted_unshadowed_global_symbol(builder.il, arg, contract.name)
}

fn non_thenable_scalar_domain(domain: DomainEvidence) -> bool {
    domain.is_integer_or_number() || domain.is_string()
}

fn promise_boundary(builder: &mut Builder<'_>, payload: ValueId) -> ValueId {
    builder.mk(ValOp::Call(PROMISE_RESOLVED_CODE), vec![payload])
}

fn promise_boundary_payload(builder: &Builder<'_>, value: ValueId) -> Option<ValueId> {
    let node = builder.nodes.get(value as usize)?;
    match &node.op {
        ValOp::Call(code) if *code == PROMISE_RESOLVED_CODE => node.args.first().copied(),
        _ => None,
    }
}

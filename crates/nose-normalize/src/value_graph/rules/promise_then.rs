//! Promise `.then` continuation canonicalization.
//!
//! The rule is exact only for admitted JS-like Promise occurrences. A `.then` selector alone is
//! never proof. The receiver must be admitted by `LibraryApi(PromiseThen)` plus PromiseLike receiver
//! evidence, and this value-graph rule must also be able to recover a settled value from a supported
//! Promise producer. The result stays behind a Promise boundary so Promise-returning code does not
//! converge with synchronous code that happens to compute the same payload.
//!
//! proof-obligation: normalize.value_graph.promise_then

use super::super::{Builder, ConstKind, ValOp, ValueDomain, ValueId};
use nose_il::{DomainEvidence, NodeId, NodeKind, Payload};
use nose_semantics::{
    admitted_promise_catch_at_call, admitted_promise_resolve_at_call,
    admitted_promise_then_at_call, asserted_unshadowed_global_symbol, nullish_global_contract,
    PromiseFactoryKind,
};
use rustc_hash::FxHashMap;

use super::super::ops::{PROMISE_REJECTED_CODE, PROMISE_RESOLVED_CODE};

#[derive(Clone, Copy)]
enum PromiseState {
    Fulfilled(ValueId),
    Rejected(ValueId),
}

pub(in super::super) fn apply(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    let kids = builder.il.children(expr).to_vec();
    if let Some(admitted) = admitted_promise_then_at_call(builder.il, builder.interner, expr) {
        let contract = admitted.contract.result;
        if !contract.demand.is_async_boundary() {
            return None;
        }
        let recv = admitted.receiver?;
        let state = promise_receiver_state(builder, recv, env)?;
        return apply_then_continuation(builder, &kids, state, env)
            .map(|state| state.into_value(builder));
    }
    if let Some(admitted) = admitted_promise_catch_at_call(builder.il, builder.interner, expr) {
        let contract = admitted.contract.result;
        if !contract.demand.is_async_boundary() {
            return None;
        }
        let recv = admitted.receiver?;
        let state = promise_receiver_state(builder, recv, env)?;
        return apply_catch_continuation(builder, &kids, state, env)
            .map(|state| state.into_value(builder));
    }
    None
}

pub(in super::super) fn promise_resolve_value(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    promise_factory_state(builder, expr, env).map(|state| state.into_value(builder))
}

fn apply_then_continuation(
    builder: &mut Builder<'_>,
    kids: &[NodeId],
    state: PromiseState,
    env: &FxHashMap<u32, ValueId>,
) -> Option<PromiseState> {
    match state {
        PromiseState::Fulfilled(value) => {
            let on_fulfilled = *kids.get(1)?;
            if safe_absent_handler(builder, on_fulfilled) {
                return Some(PromiseState::Fulfilled(value));
            }
            apply_handler(builder, on_fulfilled, value, env)
        }
        PromiseState::Rejected(reason) => {
            let Some(&on_rejected) = kids.get(2) else {
                return Some(PromiseState::Rejected(reason));
            };
            apply_handler(builder, on_rejected, reason, env)
        }
    }
}

fn apply_catch_continuation(
    builder: &mut Builder<'_>,
    kids: &[NodeId],
    state: PromiseState,
    env: &FxHashMap<u32, ValueId>,
) -> Option<PromiseState> {
    match state {
        PromiseState::Fulfilled(value) => Some(PromiseState::Fulfilled(value)),
        PromiseState::Rejected(reason) => {
            let on_rejected = *kids.get(1)?;
            apply_handler(builder, on_rejected, reason, env)
        }
    }
}

fn apply_handler(
    builder: &mut Builder<'_>,
    handler: NodeId,
    value: ValueId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<PromiseState> {
    if builder.il.kind(handler) != NodeKind::Lambda {
        return None;
    }
    let body = builder.eval_lambda_body(handler, &[value], env)?;
    promise_state_from_handler_result(builder, body)
}

fn promise_receiver_state(
    builder: &mut Builder<'_>,
    recv: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<PromiseState> {
    if let Some(state) = promise_factory_state(builder, recv, env) {
        return Some(state);
    }
    if let Some(value) = builder.eval_direct_async_function_fulfillment_call(recv, env) {
        return promise_value_is_non_thenable_safe(builder, value)
            .then_some(PromiseState::Fulfilled(value));
    }
    if builder.domain_evidence_of_expr(recv) == Some(DomainEvidence::PromiseLike) {
        if let Some(value) = builder.eval_direct_function_return_call(recv, env) {
            if let Some(state) = promise_boundary_state(builder, value) {
                return Some(state);
            }
        }
        if let Some(value) =
            crate::value_graph::inline_direct_method::eval_direct_method_return_call(
                builder, recv, env,
            )
        {
            if let Some(state) = promise_boundary_state(builder, value) {
                return Some(state);
            }
        }
        let receiver_value = builder.eval(recv, env);
        if let Some(state) = promise_boundary_state(builder, receiver_value) {
            return Some(state);
        }
    }
    let chained = apply(builder, recv, env)?;
    promise_boundary_state(builder, chained)
}

fn promise_factory_state(
    builder: &mut Builder<'_>,
    expr: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<PromiseState> {
    let admitted = admitted_promise_resolve_at_call(builder.il, builder.interner, expr)?;
    let kids = builder.il.children(expr);
    let arg = *kids.get(1)?;
    match admitted.contract.result.kind {
        PromiseFactoryKind::Resolve => {
            let value = builder.eval(arg, env);
            if !promise_resolve_arg_is_non_thenable_safe(builder, arg)
                && !promise_value_is_non_thenable_safe(builder, value)
            {
                return None;
            }
            Some(PromiseState::Fulfilled(value))
        }
        PromiseFactoryKind::Reject => Some(PromiseState::Rejected(builder.eval(arg, env))),
    }
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

fn safe_absent_handler(builder: &Builder<'_>, handler: NodeId) -> bool {
    if builder.il.kind(handler) == NodeKind::Lit {
        return true;
    }
    matches!(builder.il.kind(handler), NodeKind::Var)
        && nullish_global_arg_is_safe(builder, handler)
}

fn promise_state_from_handler_result(
    builder: &mut Builder<'_>,
    value: ValueId,
) -> Option<PromiseState> {
    if let Some(state) = promise_boundary_state(builder, value) {
        return Some(state);
    }
    promise_value_is_non_thenable_safe(builder, value).then_some(PromiseState::Fulfilled(value))
}

fn promise_value_is_non_thenable_safe(builder: &Builder<'_>, value: ValueId) -> bool {
    match builder.nodes[value as usize].op {
        ValOp::Const { kind, .. } => !matches!(kind, ConstKind::Sentinel),
        ValOp::Input(cid) => builder
            .param_domain
            .get(&cid)
            .is_some_and(|domain| non_thenable_scalar_domain(*domain)),
        _ => matches!(
            builder.vty[value as usize],
            ValueDomain::Number | ValueDomain::String | ValueDomain::Boolean
        ),
    }
}

fn promise_boundary(builder: &mut Builder<'_>, payload: ValueId) -> ValueId {
    builder.mk(ValOp::Call(PROMISE_RESOLVED_CODE), vec![payload])
}

fn promise_rejection_boundary(builder: &mut Builder<'_>, payload: ValueId) -> ValueId {
    builder.mk(ValOp::Call(PROMISE_REJECTED_CODE), vec![payload])
}

fn promise_boundary_state(builder: &mut Builder<'_>, value: ValueId) -> Option<PromiseState> {
    let node = builder.nodes.get(value as usize)?;
    match &node.op {
        ValOp::Call(code) if *code == PROMISE_RESOLVED_CODE => {
            node.args.first().copied().map(PromiseState::Fulfilled)
        }
        ValOp::Call(code) if *code == PROMISE_REJECTED_CODE => {
            node.args.first().copied().map(PromiseState::Rejected)
        }
        ValOp::Phi => {
            let args = node.args.clone();
            let [cond, then_value, else_value] = args.as_slice() else {
                return None;
            };
            let then_state = promise_boundary_state(builder, *then_value)?;
            let else_state = promise_boundary_state(builder, *else_value)?;
            match (then_state, else_state) {
                (PromiseState::Fulfilled(then_payload), PromiseState::Fulfilled(else_payload)) => {
                    Some(PromiseState::Fulfilled(
                        builder.mk(ValOp::Phi, vec![*cond, then_payload, else_payload]),
                    ))
                }
                (PromiseState::Rejected(then_reason), PromiseState::Rejected(else_reason)) => {
                    Some(PromiseState::Rejected(
                        builder.mk(ValOp::Phi, vec![*cond, then_reason, else_reason]),
                    ))
                }
                (
                    PromiseState::Fulfilled(_) | PromiseState::Rejected(_),
                    PromiseState::Fulfilled(_) | PromiseState::Rejected(_),
                ) => None,
            }
        }
        _ => None,
    }
}

impl PromiseState {
    fn into_value(self, builder: &mut Builder<'_>) -> ValueId {
        match self {
            PromiseState::Fulfilled(value) => promise_boundary(builder, value),
            PromiseState::Rejected(reason) => promise_rejection_boundary(builder, reason),
        }
    }
}

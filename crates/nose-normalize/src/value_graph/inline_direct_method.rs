//! Direct-method Promise producer return recovery.
//!
//! Direct method target evidence proves a callable body, not a receiver value. This
//! module only evaluates a non-async single returned expression, and closes if that
//! expression refers to receiver context such as `this`, `super`, or `self`.

use super::inline::INLINE_MAX_DEPTH;
use super::*;
use nose_il::UnitKind;
use nose_semantics::direct_method_call_target_span_at_call;

pub(in crate::value_graph) fn eval_direct_method_return_call(
    builder: &mut Builder<'_>,
    call: NodeId,
    env: &FxHashMap<u32, ValueId>,
) -> Option<ValueId> {
    let kids = builder.il.children(call).to_vec();
    let (root, target) = direct_method_return_target_for_call(builder, call)?;
    if target.params.len() != kids.len().saturating_sub(1) {
        return None;
    }
    if builder.inline_stack.len() >= INLINE_MAX_DEPTH || builder.inline_stack.contains(&root) {
        return None;
    }
    let plan = crate::call_args::keyword_arg_binding_plan(builder.il, &target.params, &kids[1..])?;
    let mut fenv: FxHashMap<u32, ValueId> = FxHashMap::default();
    for (pc, arg) in plan {
        let v = builder.eval(arg, env);
        fenv.insert(pc, v);
    }
    builder.inline_stack.push(root);
    let value = builder.inline_eval_pure_body(target.body, &mut fenv);
    builder.inline_stack.pop();
    value
}

fn direct_method_return_target_for_call(
    builder: &Builder<'_>,
    call: NodeId,
) -> Option<(NodeId, InlineFunction)> {
    let proven_span = direct_method_call_target_span_at_call(builder.il, builder.interner, call)?;
    let mut found = None;
    for unit in &builder.il.units {
        if !matches!(unit.kind, UnitKind::Method | UnitKind::Function)
            || builder.il.kind(unit.root) != NodeKind::Func
            || builder.il.node(unit.root).span != proven_span
        {
            continue;
        }
        if builder.direct_function_has_async_protocol(unit.root) {
            continue;
        }
        let Some(function) = direct_method_return_target(builder, unit.root) else {
            continue;
        };
        if found.is_some() {
            return None;
        }
        found = Some((unit.root, function));
    }
    found
}

fn direct_method_return_target(builder: &Builder<'_>, root: NodeId) -> Option<InlineFunction> {
    let function = builder.direct_function_return_target(root)?;
    if return_expr_uses_receiver_context(builder, function.body) {
        return None;
    }
    Some(function)
}

fn return_expr_uses_receiver_context(builder: &Builder<'_>, node: NodeId) -> bool {
    if let Payload::Name(name) = builder.il.node(node).payload {
        let name = builder.interner.resolve(name);
        if matches!(name, "this" | "super" | "self") {
            return true;
        }
    }
    builder
        .il
        .children(node)
        .iter()
        .any(|&child| return_expr_uses_receiver_context(builder, child))
}

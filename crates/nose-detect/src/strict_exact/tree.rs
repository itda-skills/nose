use super::*;

pub(crate) fn function_binding_safe(
    il: &Il,
    interner: &Interner,
    facts: &StrictFacts,
    root: NodeId,
    node: NodeId,
) -> bool {
    if facts.decorated_definition_span(il.node(node).span) {
        return false;
    }
    match il.kind(node) {
        NodeKind::Raw
        | NodeKind::HoF
        | NodeKind::Lambda
        | NodeKind::Loop
        | NodeKind::Try
        | NodeKind::Throw => false,
        NodeKind::Func if node != root => false,
        NodeKind::Call => match il.node(node).payload {
            Payload::Builtin(builtin) => {
                admitted_builtin_semantics_at_call_with_interner(il, interner, node, builtin)
            }
            _ => false,
        },
        NodeKind::Seq => strict_exact_safe_seq(il, interner, node),
        NodeKind::Lit => exact_literal_safe(il, node),
        NodeKind::Var => {
            strict_exact_safe_var(il, facts, node)
                || strict_exact_nullish_global_safe(il, interner, node)
                || strict_exact_rust_option_none_safe(il, interner, node)
        }
        _ => il
            .children(node)
            .iter()
            .all(|&c| function_binding_safe(il, interner, facts, root, c)),
    }
}

pub(crate) fn strict_exact_safe_tree(
    il: &Il,
    interner: &Interner,
    facts: &StrictFacts,
    node: NodeId,
) -> bool {
    if facts.decorated_definition_span(il.node(node).span) {
        return false;
    }
    match il.kind(node) {
        NodeKind::Raw => false,
        // Ruby `raise`/`rescue` exact recovery needs exception propagation,
        // rescue matching, ensure ordering, and non-local-control semantics.
        // Keep those boundaries closed without disabling existing exact
        // fragment contracts for other languages' direct throw/exit shapes.
        NodeKind::Try | NodeKind::Throw if il.meta.lang == Lang::Ruby => false,
        // Declarative (CSS) units are constant, deterministic, effect-free data. A rule
        // is exact-safe iff it has no unparsed (`Raw`) construct, so recurse into the
        // rule but treat a declaration / selector (whose leaves are constant value
        // tokens, lowered as `Lit(Name=raw text)`) as safe outright.
        NodeKind::CssRule | NodeKind::HtmlElement | NodeKind::HtmlControl => il
            .children(node)
            .iter()
            .all(|&c| strict_exact_safe_tree(il, interner, facts, c)),
        NodeKind::CssDecl | NodeKind::CssSelector | NodeKind::HtmlAttr | NodeKind::HtmlText => true,
        NodeKind::Seq => {
            strict_exact_safe_seq(il, interner, node)
                && il
                    .children(node)
                    .iter()
                    .all(|&c| strict_exact_safe_tree(il, interner, facts, c))
        }
        NodeKind::Call => strict_exact_safe_call(il, interner, facts, node),
        NodeKind::HoF => strict_exact_safe_hof(il, interner, facts, node),
        NodeKind::Index
            if strict_exact_go_literal_zero_map_index_safe(il, interner, facts, node)
                || strict_exact_swift_default_subscript_index_safe(il, interner, facts, node) =>
        {
            true
        }
        NodeKind::BinOp if strict_exact_static_index_membership_safe(il, interner, facts, node) => {
            true
        }
        NodeKind::BinOp if matches!(il.node(node).payload, Payload::Op(Op::In)) => {
            strict_exact_in_membership_safe(il, interner, facts, node)
        }
        NodeKind::Lit => exact_literal_safe(il, node),
        NodeKind::Var => {
            strict_exact_safe_var(il, facts, node)
                || strict_exact_nullish_global_safe(il, interner, node)
                || strict_exact_rust_option_none_safe(il, interner, node)
        }
        _ => il
            .children(node)
            .iter()
            .all(|&c| strict_exact_safe_tree(il, interner, facts, c)),
    }
}

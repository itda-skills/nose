use super::*;

/// Lower a C# pattern into a boolean predicate over the (already lowered)
/// scrutinee — the Rust/Swift `match` discipline: patterns become comparison
/// conditions in an `if`/else chain. `None` means the pattern is irrefutable
/// (`_`, `var x`, `default`) so the caller treats that arm/section as the `else`.
/// Type tests (`string s`, `int`) erase to the scrutinee value, mirroring Java's
/// `instanceof` lowering. Unknown pattern forms keep a fail-closed `Raw` label so
/// two sections can never silently merge.
pub(super) fn pattern_condition(
    lo: &mut Lowering,
    scrutinee: NodeId,
    pattern: TsNode,
    span: Span,
) -> Option<NodeId> {
    match pattern.kind() {
        "discard" | "var_pattern" => None,
        "constant_pattern" => {
            let value = pattern.named_child(0).map(|c| lower_expr(lo, c))?;
            Some(lo.add(
                NodeKind::BinOp,
                Payload::Op(Op::Eq),
                span,
                &[scrutinee, value],
            ))
        }
        "relational_pattern" => {
            let op = relational_op(pattern)?;
            let value = pattern.named_child(0).map(|c| lower_expr(lo, c))?;
            Some(lo.add(NodeKind::BinOp, Payload::Op(op), span, &[scrutinee, value]))
        }
        "or_pattern" | "and_pattern" => {
            let l = pattern
                .child_by_field_name("left")
                .and_then(|p| pattern_condition(lo, scrutinee, p, span));
            let r = pattern
                .child_by_field_name("right")
                .and_then(|p| pattern_condition(lo, scrutinee, p, span));
            let op = if pattern.kind() == "or_pattern" {
                Op::Or
            } else {
                Op::And
            };
            match (l, r) {
                (Some(l), Some(r)) => Some(lo.add(NodeKind::BinOp, Payload::Op(op), span, &[l, r])),
                // An irrefutable side: `p or _` matches everything; `p and _` is `p`.
                (one, other) if op == Op::And => one.or(other),
                _ => None,
            }
        }
        "negated_pattern" => {
            let inner = pattern.named_child(0)?;
            let cond = pattern_condition(lo, scrutinee, inner, span)?;
            Some(lo.add(NodeKind::UnOp, Payload::Op(Op::Not), span, &[cond]))
        }
        "parenthesized_pattern" => pattern_condition(lo, scrutinee, pattern.named_child(0)?, span),
        // A type test: mirror Java `instanceof` — the runtime value being tested
        // (type erased); the declaration binding carries no discriminant.
        "declaration_pattern" | "type_pattern" => Some(scrutinee),
        "recursive_pattern" => recursive_pattern_condition(lo, scrutinee, pattern, span),
        k if k.ends_with("_pattern") => {
            Some(lo.raw(&format!("csharp_pattern {k}"), lo.span(pattern), &[]))
        }
        // Old-style `case <expr>:` label body — a constant comparison.
        _ => {
            let value = lower_expr(lo, pattern);
            Some(lo.add(
                NodeKind::BinOp,
                Payload::Op(Op::Eq),
                span,
                &[scrutinee, value],
            ))
        }
    }
}

/// `Type { Prop: p, … }` → And-fold of each subpattern's predicate applied to
/// `scrutinee.Prop`; a bare `Type { }` erases to the scrutinee (the `instanceof`
/// discipline). Positional clauses stay fail-closed `Raw`.
fn recursive_pattern_condition(
    lo: &mut Lowering,
    scrutinee: NodeId,
    pattern: TsNode,
    span: Span,
) -> Option<NodeId> {
    let mut conds = Vec::new();
    for child in Lowering::named_children(pattern) {
        match child.kind() {
            "property_pattern_clause" => {
                for sub in Lowering::named_children(child) {
                    if let Some(cond) = subpattern_condition(lo, scrutinee, sub, span) {
                        conds.push(cond);
                    }
                }
            }
            "positional_pattern_clause" => {
                conds.push(lo.raw("csharp_positional_pattern", lo.span(child), &[]));
            }
            _ => {} // the type name / designation — erased
        }
    }
    if conds.is_empty() {
        return Some(scrutinee);
    }
    let mut it = conds.into_iter();
    let mut acc = it.next()?;
    for cond in it {
        acc = lo.add(NodeKind::BinOp, Payload::Op(Op::And), span, &[acc, cond]);
    }
    Some(acc)
}

/// `Prop: p` inside a property pattern → `p`'s predicate over `scrutinee.Prop`.
fn subpattern_condition(
    lo: &mut Lowering,
    scrutinee: NodeId,
    sub: TsNode,
    span: Span,
) -> Option<NodeId> {
    let kids = Lowering::named_children(sub);
    let [name, inner] = kids[..] else {
        return Some(lo.raw("csharp_subpattern", lo.span(sub), &[]));
    };
    let field = lo.sym(lo.text(name));
    let projected = lo.add(NodeKind::Field, Payload::Name(field), span, &[scrutinee]);
    pattern_condition(lo, projected, inner, span)
}

/// `x is <pattern>` → the pattern's predicate; an irrefutable pattern keeps the
/// tested value (as Java lowers `instanceof` to the value under test).
pub(super) fn lower_is_pattern(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let value = node
        .child_by_field_name("expression")
        .map(|e| lower_expr(lo, e))
        .unwrap_or_else(|| lo.empty_block(span));
    node.child_by_field_name("pattern")
        .and_then(|p| pattern_condition(lo, value, p, span))
        .unwrap_or(value)
}

/// Combine a section/arm's pattern predicate with its `when` guard.
pub(super) fn combine_pattern_guard(
    lo: &mut Lowering,
    span: Span,
    pattern: Option<NodeId>,
    guard: Option<NodeId>,
) -> Option<NodeId> {
    match (pattern, guard) {
        (Some(p), Some(g)) => Some(lo.add(NodeKind::BinOp, Payload::Op(Op::And), span, &[p, g])),
        (Some(c), None) | (None, Some(c)) => Some(c),
        (None, None) => None,
    }
}

fn relational_op(node: TsNode) -> Option<Op> {
    if crate::lower::has_direct_token(node, ">=") {
        Some(Op::Ge)
    } else if crate::lower::has_direct_token(node, "<=") {
        Some(Op::Le)
    } else if crate::lower::has_direct_token(node, ">") {
        Some(Op::Gt)
    } else if crate::lower::has_direct_token(node, "<") {
        Some(Op::Lt)
    } else {
        None
    }
}

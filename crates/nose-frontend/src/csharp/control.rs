use super::*;

pub(super) fn lower_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Block, lower_stmt)
}

pub(super) fn lower_stmt(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    match node.kind() {
        "block" => Some(lower_block(lo, node)),
        "local_declaration_statement" => Lowering::named_children(node)
            .into_iter()
            .find(|c| c.kind() == "variable_declaration")
            .map(|d| lower_variable_declaration(lo, d)),
        "variable_declaration" => Some(lower_variable_declaration(lo, node)),
        "local_function_statement" => Some(lower_method(lo, node)),
        "expression_statement" => {
            let c = node.named_child(0)?;
            match c.kind() {
                "assignment_expression"
                | "postfix_unary_expression"
                | "prefix_unary_expression"
                | "invocation_expression" => Some(lower_expr(lo, c)),
                _ => {
                    let e = lower_expr(lo, c);
                    Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
                }
            }
        }
        "if_statement" => Some(lower_if(lo, node)),
        "for_statement" => Some(lower_for(lo, node)),
        "foreach_statement" => Some(lower_foreach(lo, node)),
        "while_statement" | "do_statement" => Some(lower_while(lo, node)),
        "switch_statement" => Some(lower_switch(lo, node)),
        "return_statement" => {
            let mut kids = Vec::new();
            if let Some(v) = node.named_child(0) {
                kids.push(lower_expr(lo, v));
            }
            Some(lo.add(NodeKind::Return, Payload::None, span, &kids))
        }
        "break_statement" => Some(lo.add(NodeKind::Break, Payload::None, span, &[])),
        "continue_statement" => Some(lo.add(NodeKind::Continue, Payload::None, span, &[])),
        // `label: stmt` (goto target) — lower the inner statement, drop the label
        // (the C frontend discipline).
        "labeled_statement" => Lowering::named_children(node)
            .into_iter()
            .next_back()
            .and_then(|s| lower_stmt(lo, s)),
        // `goto label` / `goto case v` / `goto default` — a jump; model as Break
        // (drop the target so the label doesn't leak).
        "goto_statement" => Some(lo.add(NodeKind::Break, Payload::None, span, &[])),
        "throw_statement" => {
            let mut kids = Vec::new();
            if let Some(v) = node.named_child(0) {
                kids.push(lower_expr(lo, v));
            }
            Some(lo.add(NodeKind::Throw, Payload::None, span, &kids))
        }
        // `yield return v;` → the generator protocol boundary (as Python/JS
        // `yield`); `yield break;` ends the iterator → `Return`.
        "yield_statement" => {
            if crate::lower::has_direct_token(node, "break") {
                Some(lo.add(NodeKind::Return, Payload::None, span, &[]))
            } else {
                let value = node.named_child(0).map(|c| lower_expr(lo, c));
                Some(lo.yield_boundary(span, value))
            }
        }
        "try_statement" => Some(lower_try(lo, node)),
        // `await using` acquires/disposes the resource asynchronously — the same
        // source-backed context boundary as Python's `async with`.
        "using_statement" => {
            let block = lower_guarded_block(lo, node);
            Some(if crate::lower::has_direct_token(node, "await") {
                lo.protocol_boundary(
                    span,
                    nose_il::SourceProtocolKind::AsyncContext,
                    "async_with",
                    &[block],
                )
            } else {
                block
            })
        }
        "lock_statement" | "fixed_statement" | "checked_statement" | "unsafe_statement" => {
            Some(lower_guarded_block(lo, node))
        }
        "preproc_if" | "preproc_elif" | "preproc_else" => Some(lower_preproc(lo, node, lower_stmt)),
        "empty_statement" | "line_comment" | "block_comment" | "comment" | ";" => None,
        k if is_preproc_directive(k) => None,
        k if is_type_decl(k) => lower_item(lo, node),
        _ => {
            let e = lower_expr(lo, node);
            Some(lo.add(NodeKind::ExprStmt, Payload::None, span, &[e]))
        }
    }
}

fn is_type_decl(k: &str) -> bool {
    matches!(
        k,
        "class_declaration"
            | "struct_declaration"
            | "record_declaration"
            | "interface_declaration"
            | "enum_declaration"
    )
}

/// A `variable_declaration` (`int total = 0, i = 1;`) → one `Assign` per
/// declarator; an uninitialized declarator binds to a `Null` literal (as in Java).
pub(super) fn lower_variable_declaration(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut assigns = Vec::new();
    for d in Lowering::named_children(node) {
        if d.kind() != "variable_declarator" {
            continue;
        }
        let dspan = lo.span(d);
        let kids = Lowering::named_children(d);
        // The target is the `name` field, or — for a deconstruction like
        // `var (a, b) = t` — the leading `tuple_pattern` (lowered as a `Seq` so
        // it converges with tuple-expression shapes).
        let lhs = match d.child_by_field_name("name") {
            Some(n) => lo.var(lo.text(n), dspan),
            None if kids.len() > 1 => lower_expr(lo, kids[0]),
            None => lo.empty_block(dspan),
        };
        // The name is the first named child; the initializer, if any, follows it.
        let rhs = kids
            .get(1)
            .map(|v| lower_expr(lo, *v))
            .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), dspan, &[]));
        assigns.push(lo.add(NodeKind::Assign, Payload::None, dspan, &[lhs, rhs]));
    }
    if assigns.len() == 1 {
        assigns.pop().unwrap()
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}

pub(super) fn stmt_as_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::stmt_as_block(lo, node, "block", lower_block, lower_stmt)
}

pub(super) fn lower_if(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::if_stmt(lo, node, lower_expr, stmt_as_block, stmt_as_block)
}

pub(super) fn lower_for(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::c_style_for(
        lo,
        node,
        "initializer",
        "update",
        lower_stmt,
        lower_expr,
        lower_expr,
        stmt_as_block,
    )
}

/// `foreach (var x in xs) …` → a canonical `ForEach` loop `[pattern, iterable,
/// body]`, converging with Java's enhanced-`for`. `await foreach` additionally
/// keeps the async-iteration protocol boundary (the Swift `for await` / Python
/// `async for` discipline), so it can never merge with a synchronous loop.
pub(super) fn lower_foreach(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let pat = node
        .child_by_field_name("left")
        .map(|n| {
            if n.kind() == "identifier" {
                lo.var(lo.text(n), lo.span(n))
            } else {
                lower_expr(lo, n)
            }
        })
        .unwrap_or_else(|| lo.empty_block(span));
    let iter = node
        .child_by_field_name("right")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    let body = node
        .child_by_field_name("body")
        .map(|b| stmt_as_block(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    let loop_node = lo.add(
        NodeKind::Loop,
        Payload::Loop(LoopKind::ForEach),
        span,
        &[pat, iter, body],
    );
    if crate::lower::has_direct_token(node, "await") {
        lo.protocol_boundary(
            span,
            nose_il::SourceProtocolKind::AsyncIteration,
            "async_for",
            &[loop_node],
        )
    } else {
        loop_node
    }
}

pub(super) fn lower_while(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::while_loop(lo, node, lower_expr, stmt_as_block)
}

/// C# `switch` statement → nested `if`/else chain, each section's patterns
/// lowered as predicates over the scrutinee (the Rust/Swift `match` discipline);
/// multiple labels Or-fold, a `when` guard Ands in, and a `default:` (or an
/// irrefutable pattern) becomes the final `else`.
pub(super) fn lower_switch(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let scrutinee = node
        .child_by_field_name("value")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    let sections: Vec<TsNode> = node
        .child_by_field_name("body")
        .map(|b| {
            Lowering::named_children(b)
                .into_iter()
                .filter(|c| c.kind() == "switch_section")
                .collect()
        })
        .unwrap_or_default();
    let mut branches = Vec::new();
    let mut default_block = None;
    for section in sections {
        let (cond, block) = lower_switch_section(lo, section, span, scrutinee);
        match cond {
            Some(cond) => branches.push((cond, block)),
            None => default_block = Some(block),
        }
    }
    let mut acc = default_block.unwrap_or_else(|| lo.empty_block(span));
    for (cond, block) in branches.into_iter().rev() {
        acc = lo.add(NodeKind::If, Payload::None, span, &[cond, block, acc]);
    }
    acc
}

fn lower_switch_section(
    lo: &mut Lowering,
    section: TsNode,
    span: Span,
    scrutinee: NodeId,
) -> (Option<NodeId>, NodeId) {
    let mut conds = Vec::new();
    let mut irrefutable = false;
    let mut guard = None;
    let mut stmts = Vec::new();
    for child in Lowering::named_children(section) {
        match child.kind() {
            "case_switch_label" => match child.named_child(0) {
                Some(inner) => match pattern_condition(lo, scrutinee, inner, span) {
                    Some(c) => conds.push(c),
                    None => irrefutable = true,
                },
                None => irrefutable = true,
            },
            "default_switch_label" => irrefutable = true,
            "when_clause" => guard = child.named_child(0).map(|g| lower_expr(lo, g)),
            k if k.ends_with("_pattern") || k == "discard" => {
                match pattern_condition(lo, scrutinee, child, span) {
                    Some(c) => conds.push(c),
                    None => irrefutable = true,
                }
            }
            _ => {
                if let Some(s) = lower_stmt(lo, child) {
                    stmts.push(s);
                }
            }
        }
    }
    let block = lo.add(NodeKind::Block, Payload::None, span, &stmts);
    // An irrefutable label (`default:`, `case var x:`) matches everything — the
    // pattern part vanishes and only a `when` guard (if any) still gates it.
    let pattern_cond = if irrefutable {
        None
    } else {
        crate::lower::fold_or(lo, span, conds)
    };
    (combine_pattern_guard(lo, span, pattern_cond, guard), block)
}

pub(super) fn lower_try(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(b) = try_body(node) {
        kids.push(lower_block(lo, b));
    }
    for c in Lowering::named_children(node) {
        if matches!(c.kind(), "catch_clause" | "finally_clause") {
            if let Some(b) = clause_block(c) {
                kids.push(lower_block(lo, b));
            }
        }
    }
    lo.add(NodeKind::Try, Payload::None, span, &kids)
}

fn try_body(node: TsNode) -> Option<TsNode> {
    node.child_by_field_name("body").or_else(|| {
        Lowering::named_children(node)
            .into_iter()
            .find(|c| c.kind() == "block")
    })
}

fn clause_block(node: TsNode) -> Option<TsNode> {
    node.child_by_field_name("body").or_else(|| {
        Lowering::named_children(node)
            .into_iter()
            .find(|c| c.kind() == "block")
    })
}

/// `using`/`lock`/`fixed`/`checked`/`unsafe` statements wrap a body; lower the
/// body (the resource/guard carries no clone-relevant value here yet).
fn lower_guarded_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    node.child_by_field_name("body")
        .map(|b| stmt_as_block(lo, b))
        .or_else(|| {
            Lowering::named_children(node)
                .into_iter()
                .rev()
                .find(|c| c.kind() == "block")
                .map(|b| lower_block(lo, b))
        })
        .unwrap_or_else(|| lo.empty_block(span))
}

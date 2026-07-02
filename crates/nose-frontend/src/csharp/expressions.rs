use super::*;

pub(super) fn lower_expr(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    match node.kind() {
        "identifier" => lo.var(lo.text(node), span),
        // `_ = expr;` — the discard binds like a variable (alpha-canonicalized).
        "discard" => lo.var("_", span),
        "this_expression" | "this" => lo.var("this", span),
        "base_expression" | "base" => lo.var("base", span),
        "integer_literal" => lo.int_lit(lo.text(node).trim_end_matches(['L', 'l', 'U', 'u']), span),
        "real_literal" => lo.float_lit(lo.text(node), span),
        "string_literal"
        | "verbatim_string_literal"
        | "raw_string_literal"
        | "character_literal" => lo.str_lit(lo.text(node), span),
        "boolean_literal" => {
            let v = lo.text(node).trim() == "true";
            lo.add(NodeKind::Lit, Payload::LitBool(v), span, &[])
        }
        "null_literal" => lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]),
        "binary_expression" => lower_binary(lo, node),
        "assignment_expression" => lower_assignment(lo, node),
        "prefix_unary_expression" | "postfix_unary_expression" => lower_unary(lo, node),
        "invocation_expression" => lower_call(lo, node),
        "member_access_expression" => lower_member_access(lo, node),
        "element_access_expression" => lower_element_access(lo, node),
        "conditional_access_expression" => lower_conditional_access(lo, node),
        "is_pattern_expression" => lower_is_pattern(lo, node),
        // `x is T` / `x as T` → the runtime value (type erased), mirroring Java's
        // `instanceof` lowering.
        "is_expression" | "as_expression" => node
            .child_by_field_name("left")
            .or_else(|| node.named_child(0))
            .map(|v| lower_expr(lo, v))
            .unwrap_or_else(|| lo.empty_block(span)),
        "await_expression" => {
            let inner = node
                .named_child(0)
                .map(|c| lower_expr(lo, c))
                .unwrap_or_else(|| lo.empty_block(span));
            lo.await_boundary(span, inner)
        }
        "object_creation_expression" | "implicit_object_creation_expression" => lower_new(lo, node),
        "cast_expression" => node
            .child_by_field_name("value")
            .map(|v| lower_expr(lo, v))
            .unwrap_or_else(|| lo.empty_block(span)),
        "parenthesized_expression" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        "conditional_expression" => {
            let kids: Vec<NodeId> = ["condition", "consequence", "alternative"]
                .iter()
                .filter_map(|f| node.child_by_field_name(f))
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.add(NodeKind::If, Payload::None, span, &kids)
        }
        "switch_expression" => lower_switch_expr(lo, node),
        "lambda_expression" | "anonymous_method_expression" => lower_lambda(lo, node),
        "throw_expression" => {
            let inner = node
                .named_child(0)
                .map(|c| lower_expr(lo, c))
                .unwrap_or_else(|| lo.empty_block(span));
            lo.add(NodeKind::Throw, Payload::None, span, &[inner])
        }
        _ => lower_expr_tail(lo, node, span),
    }
}

/// The wrapper/creation/type-level tail of [`lower_expr`] (split, as in the C
/// frontend, to keep each match within the complexity budget).
fn lower_expr_tail(lo: &mut Lowering, node: TsNode, span: Span) -> NodeId {
    match node.kind() {
        // `checked(e)`/`unchecked(e)` set overflow policy and `ref e` borrows a
        // location — both are wrappers whose value is the inner expression
        // (unwrap, as with a cast).
        "checked_expression" | "ref_expression" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        // `= <expr>` initializer wrapper and call `argument`: unwrap to the value.
        "equals_value_clause" | "argument" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        // C#12 collection-expression element wrappers: unwrap to the element value
        // (a spread's iterable stands for its elements, as in JS).
        "collection_element" | "expression_element" | "spread_element" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        // `out var x` / `out int x` → the declared binding as a variable.
        "declaration_expression" => node
            .child_by_field_name("name")
            .map(|n| lo.var(lo.text(n), span))
            .unwrap_or_else(|| lo.empty_block(span)),
        "anonymous_object_creation_expression" => lower_anonymous_object(lo, node),
        "tuple_expression"
        | "tuple_pattern"
        | "collection_expression"
        | "initializer_expression"
        | "argument_list" => {
            let kids: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.add(NodeKind::Seq, Payload::None, span, &kids)
        }
        // `new T[…] {…}` / `stackalloc T[…]` — the element type is erased; the
        // value is the initializer sequence (so `new int[] {1, 2}` converges
        // with the C#12 collection expression `[1, 2]`).
        "array_creation_expression"
        | "implicit_array_creation_expression"
        | "stackalloc_expression" => {
            let init = Lowering::named_children(node)
                .into_iter()
                .find(|c| c.kind() == "initializer_expression");
            match init {
                Some(init) => lower_expr(lo, init),
                None => lo.add(NodeKind::Seq, Payload::None, span, &[]),
            }
        }
        // `typeof(T)` → a field access named `class` over the (erased) type, the
        // shape Java gives `T.class`, so the two converge.
        "typeof_expression" => {
            let base = node
                .child_by_field_name("type")
                .map(|c| lower_expr(lo, c))
                .unwrap_or_else(|| lo.empty_block(span));
            let f = lo.sym("class");
            lo.add(NodeKind::Field, Payload::Name(f), span, &[base])
        }
        // `default` / `default(T)` — the zero/absent value of the type.
        "default_expression" => lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]),
        "range_expression" => lower_range(lo, node),
        "generic_name" => lo.var(ident_text(lo, node), span),
        "qualified_name" => lo.var(lo.text(node), span),
        // `global::X` resolves `X` from the root namespace — the alias is
        // resolution plumbing, not behavior; erase it so the read converges
        // with the unaliased `X`.
        "alias_qualified_name" => match node.child_by_field_name("name") {
            Some(n) => lo.var(ident_text(lo, n), span),
            None => lo.var(lo.text(node), span),
        },
        "with_expression" | "with_initializer" => lower_with(lo, node),
        "interpolated_string_expression" => lo.str_lit(lo.text(node), span),
        // LINQ query syntax desugars to the method-syntax chain, including the
        // transparent-identifier translation for `let`/`join`/a second `from`
        // and `into` continuations; shapes the translation cannot prove stay
        // fail-closed Raw.
        "query_expression" => match lower_query(lo, node) {
            Some(id) => id,
            None => {
                let kids: Vec<NodeId> = Lowering::named_children(node)
                    .into_iter()
                    .map(|c| lower_expr(lo, c))
                    .collect();
                lo.raw(node.kind(), span, &kids)
            }
        },
        // `#if` spanning an expression: lower the guarded children (statement
        // discipline falls back to expressions), skipping the condition.
        "preproc_if" | "preproc_elif" | "preproc_else" => lower_preproc(lo, node, lower_stmt),
        // Type-level nodes carry no behavior — erase rather than Raw.
        "predefined_type"
        | "nullable_type"
        | "array_type"
        | "pointer_type"
        | "type_argument_list"
        | "type_parameter_list"
        | "attribute_list"
        | "implicit_type"
        | "tuple_type"
        | "tuple_element"
        | "attribute_argument"
        | "global_attribute" => lo.empty_block(span),
        k if is_preproc_directive(k) => lo.empty_block(span),
        _ => {
            let kids: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.raw(node.kind(), span, &kids)
        }
    }
}

/// A record `with` expression copies the receiver, replacing the listed fields.
/// No other lowering emits this shape, so tag the `Seq` (the Rust
/// struct-expression discipline) to keep it from merging with tuples; each
/// `X = 1` initializer inside it is an assignment to the copy's field.
fn lower_with(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .map(|c| lower_expr(lo, c))
        .collect();
    if node.kind() == "with_initializer" {
        lo.add(NodeKind::Assign, Payload::None, span, &kids)
    } else {
        let tag = lo.sym("csharp_with_expression");
        lo.add(NodeKind::Seq, Payload::Name(tag), span, &kids)
    }
}

/// `a ?? b` → the `ValueOrDefault` builtin call, the shape Swift's `??` lowers
/// to, so nil/null-coalescing converges cross-language.
fn lower_binary(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    if node
        .child_by_field_name("operator")
        .is_some_and(|o| lo.text(o) == "??")
    {
        let kids: Vec<NodeId> = ["left", "right"]
            .iter()
            .filter_map(|f| node.child_by_field_name(f))
            .map(|c| lower_expr(lo, c))
            .collect();
        return lo.add(
            NodeKind::Call,
            Payload::Builtin(Builtin::ValueOrDefault),
            span,
            &kids,
        );
    }
    crate::lower::binary(lo, node, common_bin_op, lower_expr)
}

/// `a = b` is a plain assignment; `a op= b` desugars to `a = a op b` (both via the
/// shared helpers, which read the `operator` field C# exposes). `a ??= b`
/// desugars to `a = ValueOrDefault(a, b)`, matching the `??` lowering.
fn lower_assignment(lo: &mut Lowering, node: TsNode) -> NodeId {
    let op = node
        .child_by_field_name("operator")
        .map(|o| lo.text(o))
        .unwrap_or("=");
    if op == "=" {
        crate::lower::assignment(lo, node, lower_expr, lower_expr)
    } else if op == "??=" {
        let span = lo.span(node);
        let target = node
            .child_by_field_name("left")
            .map(|l| lower_expr(lo, l))
            .unwrap_or_else(|| lo.empty_block(span));
        let read = node
            .child_by_field_name("left")
            .map(|l| lower_expr(lo, l))
            .unwrap_or_else(|| lo.empty_block(span));
        let value = node
            .child_by_field_name("right")
            .map(|r| lower_expr(lo, r))
            .unwrap_or_else(|| lo.empty_block(span));
        let coalesce = lo.add(
            NodeKind::Call,
            Payload::Builtin(Builtin::ValueOrDefault),
            span,
            &[read, value],
        );
        lo.add(NodeKind::Assign, Payload::None, span, &[target, coalesce])
    } else {
        crate::lower::compound_assignment(lo, node, common_bin_op, lower_expr, lower_expr)
    }
}

/// `a?.B` / `a?.M()` / `a?[i]` — the null-conditional receiver is the `condition`
/// field; the trailing member/element binding applies to it. The null check is
/// type-erased (like `instanceof`): lower to the plain access so `a?.B`
/// converges with `a.B`.
fn lower_conditional_access(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let cond = node.child_by_field_name("condition");
    let base = cond
        .map(|c| lower_expr(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    let binding = Lowering::named_children(node)
        .into_iter()
        .find(|c| Some(c.id()) != cond.map(|n| n.id()));
    match binding {
        Some(b) if b.kind() == "member_binding_expression" => {
            let name = b
                .child_by_field_name("name")
                .map(|n| lo.sym(ident_text(lo, n)));
            lo.add(
                NodeKind::Field,
                name.map(Payload::Name).unwrap_or(Payload::None),
                span,
                &[base],
            )
        }
        Some(b) if b.kind() == "element_binding_expression" => {
            let mut kids = vec![base];
            for a in Lowering::named_children(b) {
                kids.push(lower_argument(lo, a));
            }
            lo.add(NodeKind::Index, Payload::None, span, &kids)
        }
        Some(b) => {
            let inner = lower_expr(lo, b);
            lo.add(NodeKind::Field, Payload::None, span, &[base, inner])
        }
        None => base,
    }
}

/// `a..b` → the `[start, end, inclusivity]` `Seq` shape Rust's range lowering
/// emits (C# ranges are end-exclusive; missing bounds become `Null` slots).
fn lower_range(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut start = None;
    let mut end = None;
    let mut seen_op = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == ".." {
            seen_op = true;
        } else if child.is_named() {
            let v = lower_expr(lo, child);
            if seen_op {
                end = Some(v);
            } else {
                start = Some(v);
            }
        }
    }
    let none = |lo: &mut Lowering| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]);
    let s = start.unwrap_or_else(|| none(lo));
    let e = end.unwrap_or_else(|| none(lo));
    let flag = lo.int_lit("0", span);
    lo.add(NodeKind::Seq, Payload::None, span, &[s, e, flag])
}

/// prefix/postfix unary. `++`/`--` desugar to `x = x ± 1` (matching Java's
/// `update_expression` shape); `-`/`+`/`!`/`~` become a `UnOp`. The operator token
/// is read as a direct child kind so a nested `--`/`++` in the operand can't fool it.
fn lower_unary(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let operand_node = node.named_child(0);
    if crate::lower::has_direct_token(node, "++") || crate::lower::has_direct_token(node, "--") {
        let op = if crate::lower::has_direct_token(node, "--") {
            Op::Sub
        } else {
            Op::Add
        };
        let target = operand_node
            .map(|o| lower_expr(lo, o))
            .unwrap_or_else(|| lo.empty_block(span));
        let read = operand_node
            .map(|o| lower_expr(lo, o))
            .unwrap_or_else(|| lo.empty_block(span));
        let one = lo.int_lit("1", span);
        let bin = lo.add(NodeKind::BinOp, Payload::Op(op), span, &[read, one]);
        return lo.add(NodeKind::Assign, Payload::None, span, &[target, bin]);
    }
    let operand = operand_node
        .map(|o| lower_expr(lo, o))
        .unwrap_or_else(|| lo.empty_block(span));
    let op = if crate::lower::has_direct_token(node, "!") {
        Op::Not
    } else if crate::lower::has_direct_token(node, "~") {
        Op::BitNot
    } else if crate::lower::has_direct_token(node, "+") {
        Op::Pos
    } else {
        Op::Neg
    };
    lo.add(NodeKind::UnOp, Payload::Op(op), span, &[operand])
}

/// `recv.Method(args)` → `Call(Field(Method, recv), args…)`; `Fn(args)` →
/// `Call(Var(Fn), args…)`, matching Java's call shape.
fn lower_call(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let callee = match node.child_by_field_name("function") {
        Some(f) if f.kind() == "member_access_expression" => {
            let recv = f
                .child_by_field_name("expression")
                .map(|e| lower_expr(lo, e))
                .unwrap_or_else(|| lo.empty_block(span));
            let name = f
                .child_by_field_name("name")
                .map(|n| lo.sym(ident_text(lo, n)));
            lo.add(
                NodeKind::Field,
                name.map(Payload::Name).unwrap_or(Payload::None),
                span,
                &[recv],
            )
        }
        Some(f) if matches!(f.kind(), "identifier" | "generic_name") => {
            let name = lo.sym(ident_text(lo, f));
            lo.add(NodeKind::Var, Payload::Name(name), span, &[])
        }
        Some(f) => lower_expr(lo, f),
        None => lo.empty_block(span),
    };
    let mut kids = vec![callee];
    if let Some(args) = node.child_by_field_name("arguments") {
        for a in Lowering::named_children(args) {
            kids.push(lower_argument(lo, a));
        }
    }
    lo.add(NodeKind::Call, Payload::None, span, &kids)
}

fn lower_argument(lo: &mut Lowering, arg: TsNode) -> NodeId {
    if arg.kind() == "argument" {
        arg.named_child(arg.named_child_count().saturating_sub(1))
            .map(|v| lower_expr(lo, v))
            .unwrap_or_else(|| lo.empty_block(lo.span(arg)))
    } else {
        lower_expr(lo, arg)
    }
}

fn lower_member_access(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let base = node
        .child_by_field_name("expression")
        .map(|e| lower_expr(lo, e))
        .unwrap_or_else(|| lo.empty_block(span));
    let name = node
        .child_by_field_name("name")
        .map(|n| lo.sym(ident_text(lo, n)));
    lo.add(
        NodeKind::Field,
        name.map(Payload::Name).unwrap_or(Payload::None),
        span,
        &[base],
    )
}

fn lower_element_access(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let base = node
        .child_by_field_name("expression")
        .map(|e| lower_expr(lo, e))
        .unwrap_or_else(|| lo.empty_block(span));
    let mut kids = vec![base];
    if let Some(sub) = node.child_by_field_name("subscript") {
        for a in Lowering::named_children(sub) {
            kids.push(lower_argument(lo, a));
        }
    }
    lo.add(NodeKind::Index, Payload::None, span, &kids)
}

fn lower_new(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(args) = node.child_by_field_name("arguments") {
        for a in Lowering::named_children(args) {
            kids.push(lower_argument(lo, a));
        }
    }
    lo.record_source_fact(span, SourceFactKind::Call(SourceCallKind::Construct));
    lo.add(NodeKind::Call, Payload::None, span, &kids)
}

/// `x switch { <pat> => <expr>, … }` → nested `if`/else, each arm's pattern
/// lowered as a predicate over the scrutinee (shared with `switch` statements
/// and `is`); an irrefutable arm (`_`, `var v`) is the `else`.
fn lower_switch_expr(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let scrutinee = node
        .named_child(0)
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    let arms: Vec<TsNode> = Lowering::named_children(node)
        .into_iter()
        .filter(|c| c.kind() == "switch_expression_arm")
        .collect();
    let mut branches = Vec::new();
    let mut default_body = None;
    for arm in arms {
        let children = Lowering::named_children(arm);
        let [pattern, .., result] = children[..] else {
            continue;
        };
        let pattern_cond = pattern_condition(lo, scrutinee, pattern, span);
        let guard = children
            .iter()
            .find(|c| c.kind() == "when_clause")
            .and_then(|w| w.named_child(0))
            .map(|g| lower_expr(lo, g));
        let body = lower_expr(lo, result);
        match combine_pattern_guard(lo, span, pattern_cond, guard) {
            Some(cond) => branches.push((cond, body)),
            None => default_body = Some(body),
        }
    }
    let mut acc = default_body.unwrap_or_else(|| lo.empty_block(span));
    for (cond, body) in branches.into_iter().rev() {
        acc = lo.add(NodeKind::If, Payload::None, span, &[cond, body, acc]);
    }
    acc
}

fn lower_lambda(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        // A bare `x => …` parameter is a single `implicit_parameter` node (no
        // children), so it must count as one Param — keeping `x => x` and
        // `(int x) => x` on one shape.
        if params.kind() == "implicit_parameter" {
            let sym = lo.sym(lo.text(params));
            kids.push(lo.add(NodeKind::Param, Payload::Name(sym), lo.span(params), &[]));
        }
        for p in Lowering::named_children(params) {
            let sym = if p.kind() == "identifier" {
                Some(lo.sym(lo.text(p)))
            } else {
                p.child_by_field_name("name").map(|n| lo.sym(lo.text(n)))
            };
            kids.push(lo.add(
                NodeKind::Param,
                sym.map(Payload::Name).unwrap_or(Payload::None),
                lo.span(p),
                &[],
            ));
        }
    }
    let body = node
        .child_by_field_name("body")
        .map(|b| {
            if b.kind() == "block" {
                lower_block(lo, b)
            } else {
                lower_expr(lo, b)
            }
        })
        .unwrap_or_else(|| lo.empty_block(span));
    kids.push(body);
    lo.add(NodeKind::Lambda, Payload::None, span, &kids)
}

/// Text of an `identifier` or the head identifier of a `generic_name` (`List<int>`
/// → `List`).
fn ident_text<'a>(lo: &Lowering<'a>, node: TsNode<'a>) -> &'a str {
    if node.kind() == "generic_name" {
        node.named_child(0)
            .map(|n| lo.text(n))
            .unwrap_or_else(|| lo.text(node))
    } else {
        lo.text(node)
    }
}

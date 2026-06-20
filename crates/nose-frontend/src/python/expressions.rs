use super::*;

/// Lower a string. A plain string is a value-retaining `LitStr`; an f-string
/// (one with `{expr}` interpolations) lowers to a string-concat chain — a base
/// `Str` literal then `Add` of each interpolated expression — so it converges with
/// a JS template literal and a `"…" + x` concatenation.
pub(super) fn lower_string(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let interps: Vec<TsNode> = semantic_named_children(node)
        .into_iter()
        .filter(|c| c.kind() == "interpolation")
        .collect();
    if interps.is_empty() {
        return lo.str_lit(lo.text(node), span);
    }
    let mut acc = lo.add(NodeKind::Lit, Payload::Lit(LitClass::Str), span, &[]);
    for interp in interps {
        // `interpolation` wraps the expression as its first named child.
        if let Some(e) = interp
            .child_by_field_name("expression")
            .or_else(|| interp.named_child(0))
        {
            let sub = lower_expr(lo, e);
            acc = lo.add(NodeKind::BinOp, Payload::Op(Op::Add), span, &[acc, sub]);
        }
    }
    acc
}
pub(super) fn lower_expr(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    match node.kind() {
        "case_pattern" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        "comment" | "line_continuation" => lo.empty_block(span),
        "identifier" => lo.var(lo.text(node), span),
        "dotted_name" => lower_dotted_name(lo, node),
        "integer" => {
            let t = lo.text(node);
            lo.int_lit(t, span)
        }
        "float" => lo.float_lit(lo.text(node), span),
        "string" | "concatenated_string" | "string_content" => lower_string(lo, node),
        "true" => lo.add(NodeKind::Lit, Payload::LitBool(true), span, &[]),
        "false" => lo.add(NodeKind::Lit, Payload::LitBool(false), span, &[]),
        "none" => lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]),
        "ellipsis" => lo.add(NodeKind::Lit, Payload::Lit(LitClass::Other), span, &[]),
        "call" => lower_call(lo, node),
        "binary_operator" => lower_binary(lo, node),
        "boolean_operator" => lower_boolop(lo, node),
        "comparison_operator" => lower_comparison(lo, node),
        "unary_operator" => lower_unary(lo, node),
        "not_operator" => lower_not(lo, node),
        "attribute" => lower_attribute(lo, node),
        "subscript" => lower_subscript(lo, node),
        "lambda" => lower_lambda(lo, node),
        "slice" => lower_slice(lo, node),
        "list" | "tuple" | "set" => {
            let kids: Vec<NodeId> = semantic_named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            let tag = lo.sym(node.kind());
            lo.add(NodeKind::Seq, Payload::Name(tag), span, &kids)
        }
        "pattern_list" | "expression_list" | "list_pattern" | "tuple_pattern" => {
            let kids: Vec<NodeId> = semantic_named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.add(NodeKind::Seq, Payload::None, span, &kids)
        }
        "dictionary" => lower_dictionary(lo, node),
        // `*expr` / `**expr` spread: keep a `Splat` marker so a spread argument is
        // distinct from a plain one (`f(*args)` ≠ `f(args)`); coevo series 7, S1.
        "list_splat" | "dictionary_splat" => lower_splat(lo, node),
        // Splat PATTERNS (assignment targets like `a, *rest = xs`) strip to the inner
        // expression — they are bind sites, not call arguments.
        "list_splat_pattern" | "dictionary_splat_pattern" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        // A standalone dict-comprehension `pair` (`{k: v for ...}`) is the loop
        // contribution `DictEntry(k, v)`. Plain dict literals use
        // `lower_dictionary_pair` instead so cross-language map literals retain a
        // language-neutral `pair` sequence tag.
        "pair" => lower_comprehension_pair(lo, node),
        "list_comprehension"
        | "set_comprehension"
        | "generator_expression"
        | "dictionary_comprehension" => lower_comprehension(lo, node),
        "conditional_expression" => lower_ternary(lo, node),
        "parenthesized_expression" => node
            .named_child(0)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        "await" => {
            let value = node
                .named_child(0)
                .map(|c| lower_expr(lo, c))
                .unwrap_or_else(|| lo.empty_block(span));
            lo.await_boundary(span, value)
        }
        "named_expression" => lower_named_expr(lo, node),
        "keyword_argument" => lower_keyword_argument(lo, node),
        "assignment" => lower_assignment(lo, node),
        "augmented_assignment" => lower_aug_assignment(lo, node),
        // comprehension clauses, if ever reached directly: lower the meaningful part
        "for_in_clause" => node
            .child_by_field_name("right")
            .or_else(|| node.named_child(1))
            .map(|r| lower_expr(lo, r))
            .unwrap_or_else(|| lo.empty_block(span)),
        "if_clause" => first_semantic_named_child(node)
            .map(|c| lower_expr(lo, c))
            .unwrap_or_else(|| lo.empty_block(span)),
        "yield" => {
            let value = node.named_child(0).map(|c| lower_expr(lo, c));
            lo.yield_boundary(span, value)
        }
        _ => {
            let kids: Vec<NodeId> = semantic_named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.raw(node.kind(), span, &kids)
        }
    }
}
pub(super) fn lower_unary(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let op = node
        .child_by_field_name("operator")
        .map(|o| lo.text(o))
        .unwrap_or("-");
    let il_op = match op {
        "-" => Op::Neg,
        "+" => Op::Pos,
        "~" => Op::BitNot,
        _ => Op::Neg,
    };
    let arg = node
        .child_by_field_name("argument")
        .map(|a| lower_expr(lo, a))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::UnOp, Payload::Op(il_op), span, &[arg])
}
pub(super) fn lower_not(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let arg = node
        .child_by_field_name("argument")
        .map(|a| lower_expr(lo, a))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::UnOp, Payload::Op(Op::Not), span, &[arg])
}
pub(super) fn lower_attribute(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let obj = node
        .child_by_field_name("object")
        .map(|o| lower_expr(lo, o))
        .unwrap_or_else(|| lo.empty_block(span));
    let attr = node
        .child_by_field_name("attribute")
        .map(|a| lo.text(a))
        .unwrap_or("");
    let sym = lo.sym(attr);
    lo.add(NodeKind::Field, Payload::Name(sym), span, &[obj])
}
pub(super) fn lower_subscript(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let base = node
        .child_by_field_name("value")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    let idx = node
        .child_by_field_name("subscript")
        .map(|s| lower_expr(lo, s))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::Index, Payload::None, span, &[base, idx])
}
pub(super) fn lower_lambda(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        lower_params(lo, params, &mut kids);
    }
    // Wrap the single-expression body in `Block(Return(expr))` so a
    // `lambda x: e` converges with a JS arrow `x => e` (and `x => { return e }`)
    // and a one-line function — all single-expression callables share a shape.
    let body = match node.child_by_field_name("body") {
        Some(b) => {
            let bspan = lo.span(b);
            let e = lower_expr(lo, b);
            let ret = lo.add(NodeKind::Return, Payload::None, bspan, &[e]);
            lo.add(NodeKind::Block, Payload::None, bspan, &[ret])
        }
        None => lo.empty_block(span),
    };
    kids.push(body);
    lo.add(NodeKind::Lambda, Payload::None, span, &kids)
}
pub(super) fn lower_slice(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    // Preserve start/stop/step POSITIONS: `a[1:]` (start=1) and `a[:1]` (stop=1)
    // are different slices and must not collapse. tree-sitter omits empty bounds
    // and the `:` separators are anonymous, so collecting only named children
    // loses which slot the bound occupies. Walk children in order, split on `:`,
    // and emit an explicit `None` placeholder for each empty slot so the `Seq` is
    // positional.
    let mut slots: Vec<NodeId> = Vec::new();
    let mut cur: Option<NodeId> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == ":" {
            slots.push(
                cur.take().unwrap_or_else(|| {
                    lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[])
                }),
            );
        } else if child.is_named() {
            cur = Some(lower_expr(lo, child));
        }
    }
    slots.push(
        cur.take()
            .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[])),
    );
    lo.add(NodeKind::Seq, Payload::None, span, &slots)
}
pub(super) fn lower_named_expr(lo: &mut Lowering, node: TsNode) -> NodeId {
    // walrus `name := value` → Assign in expression position
    let span = lo.span(node);
    let lhs = node
        .child_by_field_name("name")
        .map(|n| lower_expr(lo, n))
        .unwrap_or_else(|| lo.empty_block(span));
    let rhs = node
        .child_by_field_name("value")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs])
}
pub(super) fn lower_dotted_name(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut parts = semantic_named_children(node).into_iter();
    let Some(first) = parts.next() else {
        return lo.empty_block(span);
    };
    let mut acc = lo.var(lo.text(first), lo.span(first));
    for part in parts {
        let sym = lo.sym(lo.text(part));
        acc = lo.add(NodeKind::Field, Payload::Name(sym), lo.span(part), &[acc]);
    }
    acc
}
pub(super) fn lower_dictionary(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    for child in semantic_named_children(node) {
        match child.kind() {
            "pair" => kids.push(lower_dictionary_pair(lo, child)),
            // Dict unpacking has overwrite-order semantics that the strict value
            // graph does not prove yet. Preserve it for near mode, but make the
            // containing function ineligible for exact semantic reporting.
            "dictionary_splat" => {
                let inner: Vec<NodeId> = semantic_named_children(child)
                    .into_iter()
                    .map(|c| lower_expr(lo, c))
                    .collect();
                kids.push(lo.add(
                    NodeKind::Seq,
                    Payload::Name(lo.sym("python_dictionary_splat")),
                    lo.span(child),
                    &inner,
                ));
            }
            _ => kids.push(lower_expr(lo, child)),
        }
    }
    let tag = lo.sym("dictionary");
    lo.add(NodeKind::Seq, Payload::Name(tag), span, &kids)
}
pub(super) fn lower_dictionary_pair(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let kids: Vec<NodeId> = semantic_named_children(node)
        .into_iter()
        .map(|c| lower_expr(lo, c))
        .collect();
    let tag = lo.sym("pair");
    lo.add(NodeKind::Seq, Payload::Name(tag), span, &kids)
}
/// `f(name=value)` — keep the keyword NAME so the call's argument identity is by name,
/// not source position (`f(a=p,b=q)` must not equal `f(b=p,a=q)`). The value graph keys
/// a call's keyword args by name; an unhandled consumer treats `KwArg` opaquely (fail
/// closed). A keyword arg with no resolvable name falls back to the bare value.
/// `*expr` / `**expr` spread argument → a `Splat` marker over the inner expression so a
/// spread is fingerprint-distinct from a plain argument and the inline/oracle fail closed
/// on its dynamic arity (coevo series 7, S1).
pub(super) fn lower_splat(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let inner = node
        .named_child(0)
        .map(|c| lower_expr(lo, c))
        .unwrap_or_else(|| lo.empty_block(span));
    let mark = lo.sym(if node.kind() == "list_splat" {
        "*"
    } else {
        "**"
    });
    lo.add(NodeKind::Splat, Payload::Name(mark), span, &[inner])
}
pub(super) fn lower_keyword_argument(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let value = node
        .child_by_field_name("value")
        .map(|v| lower_expr(lo, v))
        .unwrap_or_else(|| lo.empty_block(span));
    match node.child_by_field_name("name") {
        Some(name) => {
            let sym = lo.sym(lo.text(name));
            lo.add(NodeKind::KwArg, Payload::Name(sym), span, &[value])
        }
        None => value,
    }
}
pub(super) fn lower_call(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(f) = node.child_by_field_name("function") {
        kids.push(lower_expr(lo, f));
    } else {
        let e = lo.empty_block(span);
        kids.push(e);
    }
    if let Some(args) = node.child_by_field_name("arguments") {
        // `f(x for x in xs)` — a bare generator argument: tree-sitter makes the
        // `generator_expression` the `arguments` node itself, so iterating its named
        // children would flatten the generator into separate args and drop the `for`
        // binding. Lower it as one comprehension argument (→ `HoF(Map)`).
        if args.kind() == "generator_expression" {
            kids.push(lower_comprehension(lo, args));
        } else {
            for a in semantic_named_children(args) {
                kids.push(lower_expr(lo, a));
            }
        }
    }
    lo.add(NodeKind::Call, Payload::None, span, &kids)
}
pub(super) fn lower_binary(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::binary(lo, node, py_bin_op, lower_expr)
}
pub(super) fn lower_boolop(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let op = match node.child_by_field_name("operator").map(|o| lo.text(o)) {
        Some("or") => Op::Or,
        _ => Op::And,
    };
    let l = node
        .child_by_field_name("left")
        .map(|n| lower_expr(lo, n))
        .unwrap_or_else(|| lo.empty_block(span));
    let r = node
        .child_by_field_name("right")
        .map(|n| lower_expr(lo, n))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::BinOp, Payload::Op(op), span, &[l, r])
}
/// Python comparison can chain (`a < b < c`). Two operands → one `BinOp`;
/// longer chains fold into `And` of pairwise comparisons.
pub(super) fn lower_comparison(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    // Walk children in order, separating operand expressions from operator tokens.
    // Operator keywords (`<`, `==`, `in`, `not`, `is`, …) may be anonymous or named,
    // combined (`not in`) or split (`not` + `in`); the operator between two operands is
    // the space-joined run of operator tokens seen between them. This keeps `not in` /
    // `is not` NEGATED — previously the negation was dropped (`x is not None` collapsed
    // with `x is None`) and `not in` mis-lowered to `==`.
    fn is_op_tok(t: &str) -> bool {
        matches!(
            t,
            "<" | "<="
                | ">"
                | ">="
                | "=="
                | "!="
                | "<>"
                | "in"
                | "not"
                | "is"
                | "not in"
                | "is not"
        )
    }
    let mut operand_nodes: Vec<TsNode> = Vec::new();
    let mut ops: Vec<(Op, bool, Option<SourceOperatorKind>)> = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        if c.kind() == "line_continuation" {
            continue;
        }
        let t = lo.text(c).trim();
        if is_op_tok(t) {
            pending.push(t.to_string());
        } else if c.is_named() {
            operand_nodes.push(c);
            if operand_nodes.len() >= 2 {
                let key = pending.join(" ");
                ops.push(py_cmp_op(&key).unwrap_or((Op::Eq, false, None)));
                pending.clear();
            }
        }
    }
    if operand_nodes.len() < 2 {
        return operand_nodes
            .first()
            .map(|n| lower_expr(lo, *n))
            .unwrap_or_else(|| lo.empty_block(span));
    }
    let mut acc: Option<NodeId> = None;
    for i in 0..operand_nodes.len() - 1 {
        // Lower each operand fresh per use so a chained `a<b<c` keeps `b` as two
        // independent subtrees (a tree, not a shared-child DAG).
        let l = lower_expr(lo, operand_nodes[i]);
        let r = lower_expr(lo, operand_nodes[i + 1]);
        let pair_span = lo
            .span(operand_nodes[i])
            .merge(lo.span(operand_nodes[i + 1]));
        let (op, neg, source_operator) = ops.get(i).copied().unwrap_or((Op::Eq, false, None));
        let cmp = lo.add(NodeKind::BinOp, Payload::Op(op), pair_span, &[l, r]);
        if let Some(source_operator) = source_operator {
            lo.record_source_fact(pair_span, SourceFactKind::Operator(source_operator));
        }
        let cmp = if neg {
            lo.add(NodeKind::UnOp, Payload::Op(Op::Not), pair_span, &[cmp])
        } else {
            cmp
        };
        acc = Some(match acc {
            None => cmp,
            Some(prev) => lo.add(NodeKind::BinOp, Payload::Op(Op::And), span, &[prev, cmp]),
        });
    }
    acc.unwrap_or_else(|| lo.empty_block(span))
}
pub(super) fn lower_ternary(lo: &mut Lowering, node: TsNode) -> NodeId {
    // Python: `then if cond else alt`. Named children order: [then, cond, alt].
    let span = lo.span(node);
    let kids = semantic_named_children(node);
    let then = kids
        .first()
        .map(|n| lower_expr(lo, *n))
        .unwrap_or_else(|| lo.empty_block(span));
    let cond = kids
        .get(1)
        .map(|n| lower_expr(lo, *n))
        .unwrap_or_else(|| lo.empty_block(span));
    let alt = kids
        .get(2)
        .map(|n| lower_expr(lo, *n))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.add(NodeKind::If, Payload::None, span, &[cond, then, alt])
}
pub(super) fn py_bin_op(text: &str) -> Option<Op> {
    Some(match text {
        "+" => Op::Add,
        "-" => Op::Sub,
        // `@` (matmul) is deliberately UNMAPPED: it is not elementwise `*`, so
        // mapping it to `Mul` merged `a @ b` with `a * b` — a false merge. The
        // raw fallback keys it by its own operator spelling instead.
        "*" => Op::Mul,
        // True division and floor division are distinct operations (`5 / 2 == 2.5`
        // vs `5 // 2 == 2`); each gets its own op so they never share a fingerprint.
        // Python `/` is TRUE (float) division — distinct from C-family truncated
        // `Op::Div` and Ruby/Python-`//` floored `Op::FloorDiv` (#283-D).
        "/" => Op::TrueDiv,
        "//" => Op::FloorDiv,
        "%" => Op::FloorMod,
        "**" => Op::Pow,
        "&" => Op::BitAnd,
        "|" => Op::BitOr,
        "^" => Op::BitXor,
        "<<" => Op::Shl,
        ">>" => Op::Shr,
        _ => return None,
    })
}
/// Map a comparison operator string to `(op, negated, source fact)`. `not in` / `is not`
/// carry the negation (the caller wraps the comparison in `Not`), while the source fact
/// preserves whether equality-shaped IL came from value equality or identity syntax.
pub(super) fn py_cmp_op(text: &str) -> Option<(Op, bool, Option<SourceOperatorKind>)> {
    Some(match text {
        "==" => (Op::Eq, false, Some(SourceOperatorKind::ValueEquality)),
        "!=" | "<>" => (Op::Ne, false, Some(SourceOperatorKind::ValueInequality)),
        "<" => (Op::Lt, false, None),
        "<=" => (Op::Le, false, None),
        ">" => (Op::Gt, false, None),
        ">=" => (Op::Ge, false, None),
        // Membership is directional and non-commutative — its own op, so `a in b` ≠
        // `b in a` ≠ `a == b`. Identity (`is`) stays equality-shaped (identity ≈ equality
        // in a value model). `not in` / `is not` negate.
        "in" => (Op::In, false, None),
        "not in" => (Op::In, true, None),
        "is" => (Op::Eq, false, Some(SourceOperatorKind::IdentityEquality)),
        "is not" => (Op::Eq, true, Some(SourceOperatorKind::IdentityInequality)),
        _ => return None,
    })
}

use super::*;

/// `a, b := f()` / `a = b` / `a += b`. Multiple targets become a Block of Assigns;
/// compound operators desugar to `a = a OP b`.
pub(super) fn lower_assign_like(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let left = node.child_by_field_name("left");
    let right = node.child_by_field_name("right");
    let op_text = node
        .child_by_field_name("operator")
        .map(|o| lo.text(o))
        .unwrap_or("=");
    let compound =
        op_text.len() > 1 && op_text.ends_with('=') && op_text != ":=" && op_text != "==";

    let lefts: Vec<TsNode> = left.map(expr_list_items).unwrap_or_default();
    let rights: Vec<TsNode> = right.map(expr_list_items).unwrap_or_default();

    if !compound && lefts.len() == 2 && rights.len() == 1 {
        if let Some(id) = lower_two_value_map_ok(lo, span, &lefts, &rights) {
            return id;
        }
        if let Some(id) = lower_two_value_channel_receive(lo, span, &lefts, &rights) {
            return id;
        }
    }

    let mut assigns = Vec::new();
    for (i, l) in lefts.iter().enumerate() {
        let lspan = lo.span(*l);
        let lhs = lower_store_target(lo, *l);
        let rhs = if compound {
            let lhs2 = lower_expr(lo, *l);
            let r = rights
                .get(i)
                .map(|n| lower_expr(lo, *n))
                .unwrap_or_else(|| lo.empty_block(lspan));
            // `a &^= b` → `a = a & ^b`, same bit-clear desugar as the binary form.
            if op_text.trim_end_matches('=') == "&^" {
                go_bitclear(lo, lspan, lhs2, r)
            } else if let Some(op) = js_like_compound_op(op_text) {
                lo.add(NodeKind::BinOp, Payload::Op(op), lspan, &[lhs2, r])
            } else {
                // Unmapped compound operator: keep its own raw shape rather than
                // defaulting to `Add` (which would merge it with `a += b`).
                lo.raw(&format!("compound_assignment {op_text}"), lspan, &[lhs2, r])
            }
        } else {
            rights
                .get(i)
                .map(|n| lower_expr(lo, *n))
                .unwrap_or_else(|| lo.empty_block(lspan))
        };
        assigns.push(lo.add(NodeKind::Assign, Payload::None, lspan, &[lhs, rhs]));
    }
    if assigns.len() == 1 {
        assigns.pop().unwrap()
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}
/// `v, ok := m[k]` — the comma-ok map lookup. The ok target becomes a
/// `Contains(key, map)` proof; a `_` ok target falls back to the generic path.
pub(super) fn lower_two_value_map_ok(
    lo: &mut Lowering,
    span: Span,
    lefts: &[TsNode],
    rights: &[TsNode],
) -> Option<NodeId> {
    let ok_rhs = lower_map_lookup_ok(lo, rights[0])?;
    if lo.text(lefts[1]) == "_" {
        return None;
    }
    let mut assigns = Vec::new();
    if lo.text(lefts[0]) != "_" {
        let lhs = lower_expr(lo, lefts[0]);
        let rhs = lower_expr(lo, rights[0]);
        assigns.push(lo.add(
            NodeKind::Assign,
            Payload::None,
            lo.span(lefts[0]),
            &[lhs, rhs],
        ));
    }
    let ok_lhs = lower_expr(lo, lefts[1]);
    assigns.push(lo.add(
        NodeKind::Assign,
        Payload::None,
        lo.span(lefts[1]),
        &[ok_lhs, ok_rhs],
    ));
    Some(if assigns.len() == 1 {
        assigns[0]
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    })
}
/// `v, ok := <-ch` — the comma-ok channel receive; both targets stay
/// source-backed protocol boundaries over the same channel operand.
pub(super) fn lower_two_value_channel_receive(
    lo: &mut Lowering,
    span: Span,
    lefts: &[TsNode],
    rights: &[TsNode],
) -> Option<NodeId> {
    if !is_channel_receive_expr(lo, rights[0]) {
        return None;
    }
    let mut assigns = Vec::new();
    let receive_span = lo.span(rights[0]);
    let channel = rights[0]
        .child_by_field_name("operand")
        .map(|operand| lower_expr(lo, operand))
        .unwrap_or_else(|| lo.empty_block(receive_span));
    if lo.text(lefts[0]) != "_" {
        let lhs = lower_expr(lo, lefts[0]);
        let value = lo.protocol_boundary(
            receive_span,
            SourceProtocolKind::ChannelReceive,
            "channel_receive",
            &[channel],
        );
        assigns.push(lo.add(
            NodeKind::Assign,
            Payload::None,
            lo.span(lefts[0]),
            &[lhs, value],
        ));
    }
    if lo.text(lefts[1]) != "_" {
        let ok_lhs = lower_expr(lo, lefts[1]);
        let ok = lo.protocol_boundary(
            receive_span,
            SourceProtocolKind::ChannelReceive,
            "channel_receive_status",
            &[channel],
        );
        assigns.push(lo.add(
            NodeKind::Assign,
            Payload::None,
            lo.span(lefts[1]),
            &[ok_lhs, ok],
        ));
    }
    Some(if assigns.len() == 1 {
        assigns[0]
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    })
}
pub(super) fn lower_map_lookup_ok(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    if node.kind() != "index_expression" {
        return None;
    }
    let map = node
        .child_by_field_name("operand")
        .map(|o| lower_expr(lo, o))?;
    let key = node
        .child_by_field_name("index")
        .map(|i| lower_expr(lo, i))?;
    Some(lo.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Contains),
        lo.span(node),
        &[key, map],
    ))
}
pub(super) fn is_channel_receive_expr(lo: &Lowering, node: TsNode) -> bool {
    node.kind() == "unary_expression"
        && node
            .child_by_field_name("operator")
            .is_some_and(|op| lo.text(op) == "<-")
}
pub(super) fn lower_channel_receive_expr(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let arg = node
        .child_by_field_name("operand")
        .map(|a| lower_expr(lo, a))
        .unwrap_or_else(|| lo.empty_block(span));
    lo.protocol_boundary(
        span,
        SourceProtocolKind::ChannelReceive,
        "channel_receive",
        &[arg],
    )
}
pub(super) fn expr_list_items(node: TsNode) -> Vec<TsNode> {
    if node.kind() == "expression_list" {
        Lowering::named_children(node)
    } else {
        vec![node]
    }
}
/// See [`crate::lower::deref_store_target`]: `*ls = …` must keep the store place.
pub(super) fn lower_store_target(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::deref_store_target(
        lo,
        node,
        |lo, n| {
            (n.kind() == "unary_expression"
                && n.child_by_field_name("operator")
                    .is_some_and(|o| lo.text(o) == "*"))
            .then(|| n.child_by_field_name("operand"))
            .flatten()
        },
        lower_expr,
    )
}
pub(super) fn lower_inc_dec(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let op = if node.kind() == "dec_statement" {
        Op::Sub
    } else {
        Op::Add
    };
    let target = node.named_child(0);
    let t1 = target
        .map(|t| lower_store_target(lo, t))
        .unwrap_or_else(|| lo.empty_block(span));
    let t2 = target
        .map(|t| lower_expr(lo, t))
        .unwrap_or_else(|| lo.empty_block(span));
    // `++`/`--` step by exactly 1 — emit a *concrete* `LitInt(1)` (like C does), not an
    // abstracted `Lit(Int)`, so `x++` converges with `x = x + 1` and the +1 step is
    // legible to induction-stride analysis in the value graph.
    let one = lo.int_lit("1", span);
    let binop = lo.add(NodeKind::BinOp, Payload::Op(op), span, &[t2, one]);
    lo.add(NodeKind::Assign, Payload::None, span, &[t1, binop])
}

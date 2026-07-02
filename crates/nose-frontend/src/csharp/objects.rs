use super::*;

/// The `new { a, b = e }` shape — a tagged `object` sequence of `pair`
/// sequences keyed by string literals, mirroring the JavaScript object
/// literal so the hand-written literal, the LINQ translation's synthesized
/// pairs, and JS records all converge on one shape. Member order is
/// preserved: reordering is observable (`ToString`, anonymous-type identity).
/// Every sequence needs its own span: the sequence-surface evidence that
/// admits `object`/`pair` to the exact channel is anchored per-span, and
/// mixed kinds on one span resolve as ambiguous (exact-closed).
pub(super) fn object_literal(
    lo: &mut Lowering,
    object_span: Span,
    members: &[(&str, NodeId, Span)],
) -> NodeId {
    let pair_tag = lo.sym("pair");
    let pairs: Vec<NodeId> = members
        .iter()
        .map(|(name, value, span)| {
            let key = lo.str_lit(name, *span);
            lo.add(
                NodeKind::Seq,
                Payload::Name(pair_tag),
                *span,
                &[key, *value],
            )
        })
        .collect();
    let tag = lo.sym("object");
    lo.add(NodeKind::Seq, Payload::Name(tag), object_span, &pairs)
}

/// `new { a, b = e }` — an anonymous object, lowered to the tagged
/// `object`/`pair` shape shared with JavaScript object literals (member order
/// preserved — reordering is observable through `ToString` and anonymous-type
/// identity), so the hand-written literal converges with the LINQ
/// transparent-identifier translation's synthesized pairs and with JS records.
/// A member whose name cannot be read off the declarator (only `name = value`
/// and identifier / member-access shorthands can) keeps the whole literal
/// fail-closed `Raw`.
pub(super) fn lower_anonymous_object(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    match anonymous_object_members(lo, node) {
        Some(members) => {
            // Each pair anchors on the member's name identifier: the
            // `object`/`pair` exact admission resolves sequence-surface
            // evidence per-span, and an identifier's span can never carry
            // another sequence's evidence (a value expression's span could).
            let lowered: Vec<(String, NodeId, Span)> = members
                .into_iter()
                .map(|(name, value, anchor)| {
                    let anchor_span = lo.span(anchor);
                    (name, lower_expr(lo, value), anchor_span)
                })
                .collect();
            let borrowed: Vec<(&str, NodeId, Span)> = lowered
                .iter()
                .map(|(n, v, s)| (n.as_str(), *v, *s))
                .collect();
            object_literal(lo, span, &borrowed)
        }
        None => {
            let kids: Vec<NodeId> = Lowering::named_children(node)
                .into_iter()
                .map(|c| lower_expr(lo, c))
                .collect();
            lo.raw(node.kind(), span, &kids)
        }
    }
}

/// The `(name, value, name-anchor)` members of an anonymous object. The
/// grammar flattens declarators (`r = q * 2` is a bare identifier followed by
/// the expression), so members are read positionally: an identifier directly
/// followed by an anonymous `=` token is a named member; otherwise the
/// declarator is a shorthand whose name is inferred from the identifier or
/// the member-access tail. Unrecognized declarators return `None`.
fn anonymous_object_members<'t>(
    lo: &Lowering,
    node: TsNode<'t>,
) -> Option<Vec<(String, TsNode<'t>, TsNode<'t>)>> {
    let mut cursor = node.walk();
    let children: Vec<TsNode> = node.children(&mut cursor).collect();
    let mut members = Vec::new();
    let mut i = 0;
    while i < children.len() {
        let c = children[i];
        if !c.is_named() || c.kind() == "comment" {
            i += 1;
            continue;
        }
        let named_member = children
            .get(i + 1)
            .is_some_and(|n| !n.is_named() && n.kind() == "=");
        if named_member {
            if c.kind() != "identifier" {
                return None;
            }
            let value = children.get(i + 2).copied()?;
            if !value.is_named() {
                return None;
            }
            members.push((lo.text(c).to_string(), value, c));
            i += 3;
        } else {
            let (name, anchor) = match c.kind() {
                "identifier" => (lo.text(c).to_string(), c),
                "member_access_expression" => {
                    let name_node = c.child_by_field_name("name")?;
                    (lo.text(name_node).to_string(), name_node)
                }
                _ => return None,
            };
            members.push((name, c, anchor));
            i += 1;
        }
    }
    Some(members)
}

use super::*;

/// `name!(args)` → `Call(name, args...)` (best-effort; macro tokens that don't
/// parse as expressions fall back to Raw children).
pub(super) fn lower_macro(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let macro_name = node.child_by_field_name("macro").map(|m| lo.text(m));
    let name = macro_name.map(|name| lo.sym(name));
    let mut args = Vec::new();
    for c in Lowering::named_children(node) {
        if c.kind() == "token_tree" {
            collect_macro_atoms(lo, c, &mut args);
        }
    }
    if macro_name.is_some_and(|name| name.trim_end_matches('!') == "panic") {
        return lo.add(NodeKind::Throw, Payload::None, span, &args);
    }
    lo.record_source_fact(span, SourceFactKind::Call(SourceCallKind::MacroInvocation));
    let callee = lo.add(
        NodeKind::Var,
        name.map(Payload::Name).unwrap_or(Payload::None),
        span,
        &[],
    );
    let mut kids = vec![callee];
    kids.extend(args);
    lo.add(NodeKind::Call, Payload::None, span, &kids)
}
pub(super) fn lower_macro_definition_shadow(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let macro_name = rust_macro_definition_name(lo, node).map(str::to_string);
    let name = macro_name.as_deref().map(|name| lo.sym(name));
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "macro_rule")
        .enumerate()
        .map(|(idx, rule)| {
            lower_macro_rule_body_unit(lo, rule, macro_name.as_deref().unwrap_or("macro"), idx)
        })
        .collect();
    lo.add(
        NodeKind::Block,
        name.map(Payload::Name).unwrap_or(Payload::None),
        span,
        &kids,
    )
}
pub(super) fn lower_macro_rule_body_unit(
    lo: &mut Lowering,
    rule: TsNode,
    macro_name: &str,
    arm_index: usize,
) -> NodeId {
    let right = rule.child_by_field_name("right").unwrap_or(rule);
    let span = lo.span(right);
    let mut atoms = Vec::new();
    collect_macro_atoms(lo, right, &mut atoms);
    // A macro_rules! RHS is still token-tree syntax, not normal Rust statements.
    // Keep a Raw boundary so strict semantic reporting cannot treat the atom
    // sequence as an exact runtime proof, while still giving syntax/near channels
    // a matchable unit instead of an invisible macro arm.
    let body = lo.raw("macro_rule_body", span, &atoms);
    let block = lo.add(NodeKind::Block, Payload::None, span, &[body]);
    let unit_name = lo.sym(&format!("{macro_name}:arm{arm_index}"));
    lo.push_unit_with_origin(
        block,
        UnitKind::Block,
        Some(unit_name),
        UnitOrigin::new(
            UnitDomains::of(UnitDomain::Preprocessor),
            UnitSubkind::Macro,
            UnitBodyKind::Preprocessor,
            SourceGranularity::Block,
            RegionKind::Preprocessor,
        )
        .with_evidence(UnitEvidenceFlag::MacroFunctionLike),
    );
    block
}
pub(super) fn rust_macro_definition_name<'a>(
    lo: &Lowering<'a>,
    node: TsNode<'a>,
) -> Option<&'a str> {
    if let Some(name) = node.child_by_field_name("name") {
        return Some(lo.text(name).trim());
    }
    let text = lo.text(node).trim_start();
    let rest = text.strip_prefix("macro_rules!")?.trim_start();
    let name = rest
        .split(|c: char| c == '{' || c == '(' || c == '[' || c.is_whitespace())
        .next()?
        .trim();
    (!name.is_empty()).then_some(name)
}
/// Macro arguments are an unparsed token stream: nested `()`/`[]`/`{}` are sub-
/// `token_tree`s, not expressions. Recurse through them collecting only real atoms
/// (names, literals) as call args — skipping delimiters/punctuation — so a macro
/// never leaves `Raw` token_tree nodes that would corrupt the value graph.
pub(super) fn collect_macro_atoms(lo: &mut Lowering, tt: TsNode, kids: &mut Vec<NodeId>) {
    for t in Lowering::named_children(tt) {
        if is_macro_token_container(t.kind()) {
            collect_macro_atoms(lo, t, kids);
        } else if is_macro_atom(t.kind()) {
            kids.push(lower_expr(lo, t));
        }
        // else: an unparsed token with no behavioral signal — drop it.
    }
}
pub(super) fn is_macro_token_container(k: &str) -> bool {
    matches!(
        k,
        "token_tree" | "token_repetition" | "token_tree_pattern" | "token_repetition_pattern"
    )
}
pub(super) fn is_macro_atom(k: &str) -> bool {
    const ATOMS: &[&str] = &[
        "identifier",
        "scoped_identifier",
        "field_identifier",
        "self",
        "crate",
        "super",
        "integer_literal",
        "float_literal",
        "string_literal",
        "raw_string_literal",
        "char_literal",
        "boolean_literal",
    ];
    ATOMS.contains(&k)
}

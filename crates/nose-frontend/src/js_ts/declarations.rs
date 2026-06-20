use super::expressions::lower_expr;
use super::syntax::static_string_key;
use super::{lower_block, lower_stmt};
use crate::lower::Lowering;
use nose_il::{
    LitClass, NodeId, NodeKind, Payload, RegionKind, SourceBindingKind, SourceFactKind,
    SourceGranularity, Span, UnitBodyKind, UnitDomain, UnitDomains, UnitEvidenceFlag, UnitKind,
    UnitOrigin, UnitSubkind,
};
use tree_sitter::Node as TsNode;

pub(super) fn lower_func(lo: &mut Lowering, node: TsNode, method: bool) -> NodeId {
    crate::lower::function_unit(lo, node, method, lower_params, lower_func_body)
}

/// A function body is normally a `statement_block`, but arrow functions may have
/// an expression body.
fn lower_func_body(lo: &mut Lowering, body: TsNode) -> NodeId {
    if body.kind() == "statement_block" {
        lower_block(lo, body)
    } else {
        let span = lo.span(body);
        let e = lower_expr(lo, body);
        let ret = lo.add(NodeKind::Return, Payload::None, span, &[e]);
        lo.add(NodeKind::Block, Payload::None, span, &[ret])
    }
}

pub(super) fn lower_class(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let body = node.child_by_field_name("body");
    let block = match body {
        Some(b) => super::lower_stmt_list(lo, b, NodeKind::Block, true),
        None => lo.empty_block(span),
    };
    let unit_root = lower_own_decorated_definition(lo, node, block);
    lo.push_unit_with_origin(
        unit_root,
        UnitKind::Class,
        name,
        UnitOrigin::new(
            UnitDomains::of(UnitDomain::ImplementationType),
            UnitSubkind::Class,
            UnitBodyKind::Implementation,
            SourceGranularity::WholeUnit,
            RegionKind::Code,
        )
        .with_evidence(UnitEvidenceFlag::HasReusableBody),
    );
    unit_root
}

pub(super) fn lower_decorator(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .map(|child| lower_expr(lo, child))
        .collect();
    lo.add(
        NodeKind::Seq,
        Payload::Name(lo.sym("js_decorator")),
        span,
        &kids,
    )
}

pub(super) fn lower_class_static_block(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .filter_map(|child| lower_stmt(lo, child, false))
        .collect();
    lo.add(
        NodeKind::Seq,
        Payload::Name(lo.sym("js_class_static_block")),
        span,
        &kids,
    )
}

pub(super) fn lower_decorated_definition(
    lo: &mut Lowering,
    decorators: Vec<NodeId>,
    definition_span: Span,
    decorated_span: Span,
    lowered: NodeId,
) -> NodeId {
    lo.record_source_fact(
        definition_span,
        SourceFactKind::Binding(SourceBindingKind::DecoratedDefinition),
    );
    if decorated_span != definition_span {
        lo.record_source_fact(
            decorated_span,
            SourceFactKind::Binding(SourceBindingKind::DecoratedDefinition),
        );
    }
    let mut kids = decorators;
    kids.push(lowered);
    let wrapper = lo.add(
        NodeKind::Seq,
        Payload::Name(lo.sym("js_decorated_definition")),
        decorated_span,
        &kids,
    );
    retarget_last_unit_root(lo, lowered, wrapper);
    wrapper
}

pub(super) fn lower_own_decorated_definition(
    lo: &mut Lowering,
    node: TsNode,
    lowered: NodeId,
) -> NodeId {
    let decorators: Vec<TsNode> = Lowering::named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "decorator")
        .collect();
    if decorators.is_empty() {
        return lowered;
    }
    let kids: Vec<NodeId> = decorators
        .into_iter()
        .map(|decorator| lower_decorator(lo, decorator))
        .collect();
    lower_decorated_definition(lo, kids, lo.span(node), lo.span(node), lowered)
}

fn retarget_last_unit_root(lo: &mut Lowering, from: NodeId, to: NodeId) {
    if let Some(unit) = lo.units.last_mut().filter(|unit| unit.root == from) {
        unit.root = to;
    }
}

pub(super) fn lower_field(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    // `name = value;` field initializer → Assign; bare declarations are dropped.
    let span = lo.span(node);
    let name = node.child_by_field_name("name")?;
    let value = node.child_by_field_name("value")?;
    let lhs = {
        let s = lo.span(name);
        lo.var(lo.text(name), s)
    };
    let rhs = lower_expr(lo, value);
    Some(lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs]))
}

fn lower_params(lo: &mut Lowering, params: TsNode, out: &mut Vec<NodeId>) {
    // A single-identifier arrow param arrives as the identifier itself.
    if matches!(params.kind(), "identifier" | "undefined") {
        let span = lo.span(params);
        let sym = lo.sym(lo.text(params));
        out.push(lo.add(NodeKind::Param, Payload::Name(sym), span, &[]));
        return;
    }
    for p in Lowering::named_children(params) {
        let span = lo.span(p);
        let name = param_name(lo, p);
        let payload = match name {
            Some(s) => Payload::Name(lo.sym(s)),
            None => Payload::None,
        };
        if let Some(domain) = lo.type_domain_from_text_with_dependencies(lo.text(p)) {
            lo.record_param_domain_resolution(span, domain);
        }
        out.push(lo.add(NodeKind::Param, payload, span, &[]));
    }
}

fn param_name<'a>(lo: &Lowering<'a>, p: TsNode<'a>) -> Option<&'a str> {
    match p.kind() {
        "identifier" | "shorthand_property_identifier_pattern" | "undefined" => Some(lo.text(p)),
        "required_parameter" | "optional_parameter" => p
            .child_by_field_name("pattern")
            .or_else(|| p.named_child(0))
            .map(|n| lo.text(n)),
        "rest_pattern" | "assignment_pattern" => p.named_child(0).map(|n| lo.text(n)),
        _ => p.named_child(0).map(|n| lo.text(n)),
    }
}

pub(super) fn lower_var_decl(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    // One or more `variable_declarator`s → a Block of Assigns (or a single Assign).
    let mut assigns = Vec::new();
    for d in Lowering::named_children(node) {
        if d.kind() != "variable_declarator" {
            continue;
        }
        let dspan = lo.span(d);
        let name_node = d.child_by_field_name("name");
        let rhs = match d.child_by_field_name("value") {
            // `const f = (…) => {…}` / `const f = function(){…}` is a *named function*,
            // not an inline lambda — lower it to a `Func` unit so it is extracted and
            // matched like any function. (Modern JS/TS defines most functions this way;
            // without this, arrow-const-heavy files yield zero detection units.)
            Some(v) if is_func_value(v.kind()) => {
                let nsym = name_node
                    .filter(|n| n.kind() == "identifier")
                    .map(|n| lo.sym(lo.text(n)));
                lower_func_value(lo, v, nsym)
            }
            Some(v) => lower_expr(lo, v),
            None => lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), dspan, &[]),
        };
        if let Some(name) = name_node {
            if let Some(mut projected) = lower_static_projection_pattern(lo, name, rhs, dspan) {
                assigns.append(&mut projected);
                continue;
            }
        }
        let lhs = match name_node {
            Some(n) => lower_expr(lo, n),
            None => lo.empty_block(dspan),
        };
        assigns.push(lo.add(NodeKind::Assign, Payload::None, dspan, &[lhs, rhs]));
    }
    if assigns.len() == 1 {
        assigns[0]
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}

fn lower_static_projection_pattern(
    lo: &mut Lowering,
    pattern: TsNode,
    base: NodeId,
    span: Span,
) -> Option<Vec<NodeId>> {
    if pattern.kind() != "object_pattern" {
        return None;
    }
    let mut assigns = Vec::new();
    for child in Lowering::named_children(pattern) {
        let (field, local) = object_pattern_projection(lo, child)?;
        let lhs = lo.var(&local, lo.span(child));
        let key = lo.sym(&field);
        let rhs = lo.add(NodeKind::Field, Payload::Name(key), lo.span(child), &[base]);
        assigns.push(lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs]));
    }
    (!assigns.is_empty()).then_some(assigns)
}

fn object_pattern_projection(lo: &Lowering, node: TsNode) -> Option<(String, String)> {
    match node.kind() {
        "shorthand_property_identifier_pattern" => {
            let name = lo.text(node).to_string();
            Some((name.clone(), name))
        }
        "pair_pattern" => {
            let kids = Lowering::named_children(node);
            let key = kids.first().and_then(|&k| static_property_key(lo, k))?;
            let local = kids
                .iter()
                .skip(1)
                .find_map(|&k| binding_pattern_name(lo, k))?;
            Some((key, local))
        }
        _ => None,
    }
}

fn binding_pattern_name(lo: &Lowering, node: TsNode) -> Option<String> {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" | "property_identifier" => {
            Some(lo.text(node).to_string())
        }
        _ => None,
    }
}

fn static_property_key(lo: &Lowering, node: TsNode) -> Option<String> {
    match node.kind() {
        "property_identifier" | "shorthand_property_identifier_pattern" | "identifier" => {
            Some(lo.text(node).to_string())
        }
        "string" => static_string_key(lo, node),
        _ => None,
    }
}

fn is_func_value(kind: &str) -> bool {
    matches!(
        kind,
        "arrow_function"
            | "function_expression"
            | "function"
            | "generator_function"
            | "generator_function_declaration"
    )
}

/// Lower a function-valued expression as a named `Func` unit (params + body),
/// registering it for detection. Mirrors `lower_func`/`lower_arrow` but takes the
/// binding name explicitly (arrow/function expressions have no own name).
pub(super) fn lower_func_value(
    lo: &mut Lowering,
    node: TsNode,
    name: Option<nose_il::Symbol>,
) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(params) = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"))
    {
        lower_params(lo, params, &mut kids);
    }
    let body = match node.child_by_field_name("body") {
        Some(b) => lower_func_body(lo, b),
        None => lo.empty_block(span),
    };
    kids.push(body);
    let func = lo.add(NodeKind::Func, Payload::None, span, &kids);
    lo.push_unit(func, UnitKind::Function, name);
    func
}

pub(super) fn lower_arrow(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(params) = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"))
    {
        lower_params(lo, params, &mut kids);
    }
    let body = match node.child_by_field_name("body") {
        Some(b) => lower_func_body(lo, b),
        None => lo.empty_block(span),
    };
    kids.push(body);
    lo.add(NodeKind::Lambda, Payload::None, span, &kids)
}

use super::*;

pub(super) fn lower_items(lo: &mut Lowering, node: TsNode) -> NodeId {
    crate::lower::collect_into(lo, node, NodeKind::Module, lower_item)
}

pub(super) fn lower_item(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    match node.kind() {
        "class_declaration"
        | "struct_declaration"
        | "record_declaration"
        | "record_struct_declaration"
        | "interface_declaration"
        | "enum_declaration" => Some(lower_type(lo, node)),
        "method_declaration"
        | "constructor_declaration"
        | "destructor_declaration"
        | "operator_declaration"
        | "conversion_operator_declaration"
        | "local_function_statement" => Some(lower_method(lo, node)),
        // An `event` with `add`/`remove` accessors shares the property CST shape
        // (`accessors` field of `accessor_declaration`s), so it lowers the same way.
        "property_declaration" | "indexer_declaration" | "event_declaration" => {
            Some(lower_property(lo, node))
        }
        "field_declaration" | "event_field_declaration" => Some(lower_field(lo, node)),
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            Some(lower_namespace(lo, node))
        }
        "using_directive" | "extern_alias_directive" => Some(crate::lower::import_tokens(lo, node)),
        "global_statement" => node.named_child(0).and_then(|c| lower_stmt(lo, c)),
        "preproc_if" | "preproc_elif" | "preproc_else" => Some(lower_preproc(lo, node, lower_item)),
        // A `delegate` declaration is a bodiless signature — nothing to detect.
        "delegate_declaration" => None,
        "global_attribute_list"
        | "attribute_list"
        | "line_comment"
        | "block_comment"
        | "comment" => None,
        k if is_preproc_directive(k) => None,
        _ => lower_stmt(lo, node),
    }
}

/// Non-conditional preprocessor directives (`#region`, `#pragma`, `#define`, …)
/// carry no runtime behavior — skip, never Raw.
pub(super) fn is_preproc_directive(kind: &str) -> bool {
    matches!(
        kind,
        "preproc_region"
            | "preproc_endregion"
            | "preproc_define"
            | "preproc_undef"
            | "preproc_nullable"
            | "preproc_pragma"
            | "preproc_error"
            | "preproc_warning"
            | "preproc_line"
            | "preproc_if_in_attribute_list"
    )
}

/// `#if COND … #elif … #else … #endif`: tree-sitter-c-sharp nests the guarded
/// members/statements *inside* the `preproc_*` node, so a flat walk would drop
/// them to Raw wholesale. Both branches are real code alternatives — lower every
/// named child except the compile-time condition into a `Block` (the `#elif`/
/// `#else` alternative is itself a child, recursed via `lower_child`).
pub(super) fn lower_preproc(
    lo: &mut Lowering,
    node: TsNode,
    lower_child: fn(&mut Lowering, TsNode) -> Option<NodeId>,
) -> NodeId {
    let span = lo.span(node);
    let cond = node.child_by_field_name("condition").map(|n| n.id());
    let kids: Vec<NodeId> = Lowering::named_children(node)
        .into_iter()
        .filter(|c| Some(c.id()) != cond)
        .filter_map(|c| lower_child(lo, c))
        .collect();
    lo.add(NodeKind::Block, Payload::None, span, &kids)
}

/// A `namespace` is module wiring: lower its members at top level (units register
/// themselves regardless of tree nesting), returning a `Block` of them.
pub(super) fn lower_namespace(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let members: Vec<TsNode> = match node.child_by_field_name("body") {
        Some(body) => Lowering::named_children(body),
        // file-scoped `namespace N;` — members are trailing siblings, not a body.
        None => Lowering::named_children(node)
            .into_iter()
            .filter(|c| !matches!(c.kind(), "identifier" | "qualified_name"))
            .collect(),
    };
    let kids: Vec<NodeId> = members
        .into_iter()
        .filter_map(|c| lower_item(lo, c))
        .collect();
    lo.add(NodeKind::Block, Payload::None, span, &kids)
}

/// `class`/`struct`/`record`/`interface`/`enum` → a `Class` unit; its members
/// become units too.
pub(super) fn lower_type(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name = node.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
    let mut kids = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        for c in Lowering::named_children(body) {
            if let Some(id) = lower_member(lo, c) {
                kids.push(id);
            }
        }
    }
    let block = lo.add(NodeKind::Block, Payload::None, span, &kids);
    lo.push_unit_with_origin(block, UnitKind::Class, name, csharp_type_origin(node));
    block
}

fn lower_member(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    match node.kind() {
        "enum_member_declaration" => node
            .child_by_field_name("name")
            .map(|n| lo.str_lit(lo.text(n), lo.span(n))),
        _ => lower_item(lo, node),
    }
}

fn csharp_type_origin(node: TsNode) -> UnitOrigin {
    let (domain, subkind) = match node.kind() {
        "interface_declaration" => (
            UnitDomain::TypeContract,
            UnitSubkind::InterfaceTraitProtocol,
        ),
        "struct_declaration" | "record_declaration" | "record_struct_declaration" => {
            (UnitDomain::Data, UnitSubkind::StructRecord)
        }
        "enum_declaration" => (UnitDomain::Data, UnitSubkind::Enum),
        _ => (UnitDomain::ImplementationType, UnitSubkind::Class),
    };
    let has_body = node.child_by_field_name("body").is_some_and(|b| {
        Lowering::named_children(b).iter().any(|c| {
            matches!(
                c.kind(),
                "method_declaration" | "constructor_declaration" | "property_declaration"
            )
        })
    });
    UnitOrigin::new(
        UnitDomains::of(domain),
        subkind,
        if has_body {
            UnitBodyKind::Implementation
        } else {
            UnitBodyKind::DeclarationOnly
        },
        SourceGranularity::WholeUnit,
        RegionKind::Code,
    )
    .with_evidence(if has_body {
        UnitEvidenceFlag::HasReusableBody
    } else {
        UnitEvidenceFlag::DeclarationOnly
    })
}

/// `method`/`constructor`/`local function` → a `Func` unit via the shared skeleton
/// (extract `name`/`parameters`/`body`, push the unit). An `async` body keeps the
/// async-function scheduling boundary (the JS/Python/Swift discipline), so an
/// async method can never merge with its synchronous twin.
pub(super) fn lower_method(lo: &mut Lowering, node: TsNode) -> NodeId {
    let is_local_fn = node.kind() == "local_function_statement";
    let is_async = has_async_modifier(lo, node);
    let span = lo.span(node);
    crate::lower::function_unit(lo, node, !is_local_fn, csharp_params, |lo, b| {
        let body = csharp_body(lo, b);
        if is_async {
            lo.protocol_boundary(
                span,
                nose_il::SourceProtocolKind::AsyncFunction,
                "async_function",
                &[body],
            )
        } else {
            body
        }
    })
}

/// Does this declaration carry the `async` modifier? (The C# grammar wraps each
/// keyword modifier in its own `modifier` node.)
pub(super) fn has_async_modifier(lo: &Lowering, node: TsNode) -> bool {
    let mut cur = node.walk();
    let found = node
        .children(&mut cur)
        .any(|c| c.kind() == "modifier" && lo.text(c) == "async");
    found
}

fn csharp_params(lo: &mut Lowering, params: TsNode, kids: &mut Vec<NodeId>) {
    for p in Lowering::named_children(params) {
        if p.kind() != "parameter" {
            continue;
        }
        let pspan = lo.span(p);
        let sym = p.child_by_field_name("name").map(|n| lo.sym(lo.text(n)));
        // Record the parameter's proven type domain (e.g. `int` → Integer) so the
        // numeric/collection canonicalizations converge with the other statically
        // typed frontends (Java/Go/C/Rust).
        if let Some(ty) = p.child_by_field_name("type") {
            if let Some(domain) = lo.type_domain_from_text_with_dependencies(lo.text(ty)) {
                lo.record_param_domain_resolution(pspan, domain);
            }
        }
        kids.push(lo.add(
            NodeKind::Param,
            sym.map(Payload::Name).unwrap_or(Payload::None),
            pspan,
            &[],
        ));
    }
}

/// A method/accessor body: a `block` lowers as-is; an expression-bodied member
/// (`=> expr`) lowers to `{ return expr; }` so it converges with the block form.
pub(super) fn csharp_body(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    match node.kind() {
        "block" => lower_block(lo, node),
        "arrow_expression_clause" => {
            let expr = node
                .named_child(0)
                .map(|e| lower_expr(lo, e))
                .unwrap_or_else(|| lo.empty_block(span));
            let ret = lo.add(NodeKind::Return, Payload::None, span, &[expr]);
            lo.add(NodeKind::Block, Payload::None, span, &[ret])
        }
        _ => lower_block(lo, node),
    }
}

/// A property/indexer/event lowers each accessor that has a body — or the
/// `=> expr` getter — into a method-like unit; auto-accessors (`{ get; set; }`)
/// carry no body and form no unit.
pub(super) fn lower_property(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let name_node = node.child_by_field_name("name");
    let name = name_node.map(|n| lo.sym(lo.text(n)));
    let value = node.child_by_field_name("value");
    // `int A => expr;` — an expression-bodied getter.
    if let Some(v) = value.filter(|v| v.kind() == "arrow_expression_clause") {
        let body = csharp_body(lo, v);
        return push_accessor_unit(lo, span, body, name);
    }
    let mut kids = Vec::new();
    if let Some(accessors) = node.child_by_field_name("accessors") {
        for acc in Lowering::named_children(accessors) {
            if acc.kind() != "accessor_declaration" {
                continue;
            }
            let Some(body_node) = acc.child_by_field_name("body") else {
                continue; // auto-accessor `get;` / `set;` — no body
            };
            let body = csharp_body(lo, body_node);
            kids.push(push_accessor_unit(lo, lo.span(acc), body, name));
        }
    }
    // `{ get; set; } = init;` — the same `value` field holds the auto-property
    // initializer, a field-like binding to the property name (the `lower_field`
    // discipline), not an accessor body.
    if let Some(v) = value {
        let vspan = lo.span(v);
        let lhs = match name_node {
            Some(n) => lo.var(lo.text(n), lo.span(n)),
            None => lo.empty_block(vspan),
        };
        let rhs = lower_expr(lo, v);
        kids.push(lo.add(NodeKind::Assign, Payload::None, vspan, &[lhs, rhs]));
    }
    if kids.len() == 1 {
        kids.pop().unwrap()
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &kids)
    }
}

fn push_accessor_unit(lo: &mut Lowering, span: Span, body: NodeId, name: Option<Symbol>) -> NodeId {
    let func = lo.add(NodeKind::Func, Payload::None, span, &[body]);
    lo.push_unit_with_origin(
        func,
        UnitKind::Method,
        name,
        crate::lower::imperative_callable_origin(UnitSubkind::Method, true),
    );
    func
}

/// A `field_declaration` / `event_field_declaration` → the `Assign`s of its
/// `variable_declaration`.
pub(super) fn lower_field(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    Lowering::named_children(node)
        .into_iter()
        .find(|c| c.kind() == "variable_declaration")
        .map(|d| lower_variable_declaration(lo, d))
        .unwrap_or_else(|| lo.empty_block(span))
}

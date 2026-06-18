use super::*;

pub(super) fn lower_items(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    collect_top_items(lo, node, &mut kids);
    lo.add(NodeKind::Module, Payload::None, span, &kids)
}
/// Collect top-level items, descending through preprocessor conditionals. A function
/// guarded by `#if PLATFORM … #endif` (ubiquitous in C: nginx/curl per-OS code) lives
/// *inside* a `preproc_if`/`preproc_ifdef` node, so a flat scan of the translation
/// unit would discard it entirely — the file would lower to an empty module and its
/// functions become invisible to detection. Recurse into the conditional's body
/// (skipping the condition/macro-name field, which is not an item).
pub(super) fn collect_top_items(lo: &mut Lowering, node: TsNode, out: &mut Vec<NodeId>) {
    let skip = node
        .child_by_field_name("condition")
        .or_else(|| node.child_by_field_name("name"))
        .map(|n| n.id());
    for c in Lowering::named_children(node) {
        if Some(c.id()) == skip {
            continue; // the `#if COND` / `#ifdef NAME` test, not an item
        }
        match c.kind() {
            "preproc_if"
            | "preproc_ifdef"
            | "preproc_else"
            | "preproc_elif"
            | "preproc_elifdef"
            | "linkage_specification" => collect_top_items(lo, c, out),
            _ => {
                if let Some(n) = lower_item(lo, c) {
                    out.push(n);
                }
            }
        }
    }
}
pub(super) fn lower_item(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    match node.kind() {
        "function_definition" => Some(lower_func(lo, node)),
        "declaration" => Some(lower_decl(lo, node)),
        "preproc_include" => Some(crate::lower::import_tokens(lo, node)),
        "type_definition" => {
            record_c_type_definition(lo, node);
            None
        }
        "preproc_def"
        | "preproc_function_def"
        | "preproc_ifdef"
        | "preproc_if"
        | "struct_specifier"
        | "union_specifier"
        | "enum_specifier"
        | "comment" => None,
        _ => lower_stmt(lo, node),
    }
}
/// Find the binding identifier inside a (possibly pointer/array) C declarator.
pub(super) fn declarator_name(lo: &Lowering, node: TsNode) -> Option<nose_il::Symbol> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" => Some(lo.sym(lo.text(node))),
        _ => node
            .child_by_field_name("declarator")
            .or_else(|| node.named_child(0))
            .and_then(|c| declarator_name(lo, c)),
    }
}
pub(super) fn lower_func(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let decl = node.child_by_field_name("declarator");
    let name = decl.and_then(|d| declarator_name(lo, d));
    let mut kids = Vec::new();
    // parameters live under the function_declarator's parameter_list
    if let Some(d) = decl {
        if let Some(params) = find_param_list(d) {
            for p in Lowering::named_children(params) {
                let pspan = lo.span(p);
                let sym = p
                    .child_by_field_name("declarator")
                    .and_then(|x| declarator_name(lo, x));
                let param_name = sym.map(|symbol| lo.interner.resolve(symbol));
                if let Some((domain, dependencies)) =
                    c_param_domain_from_text(lo, lo.text(p), param_name)
                {
                    lo.record_param_domain_with_dependencies(pspan, domain, dependencies);
                }
                kids.push(lo.add(
                    NodeKind::Param,
                    sym.map(Payload::Name).unwrap_or(Payload::None),
                    pspan,
                    &[],
                ));
            }
        }
    }
    let body = node
        .child_by_field_name("body")
        .map(|b| lower_block(lo, b))
        .unwrap_or_else(|| lo.empty_block(span));
    kids.push(body);
    let func = lo.add(NodeKind::Func, Payload::None, span, &kids);
    lo.push_unit_with_origin(
        func,
        UnitKind::Function,
        name,
        crate::lower::imperative_callable_origin(UnitSubkind::Function, true),
    );
    func
}
pub(super) fn record_c_type_definition(lo: &mut Lowering, node: TsNode) {
    let span = lo.span(node);
    if let Some(alias) = c_unsigned_char_typedef_alias(lo.text(node)) {
        let type_id = record_c_type_alias_evidence(
            lo,
            span,
            &alias,
            CTypeTarget::UnsignedInteger { bits: 8 },
            Vec::new(),
        );
        lo.record_type_domain_alias_exact_with_evidence(
            &alias,
            DomainEvidence::ByteArray,
            Some(type_id),
        );
    }
    if let Some(alias) = c_unsigned_32_typedef_alias(lo.text(node)) {
        let type_id = record_c_type_alias_evidence(
            lo,
            span,
            &alias,
            CTypeTarget::UnsignedInteger { bits: 32 },
            Vec::new(),
        );
        lo.record_unsigned_32_alias_with_evidence(&alias, Some(type_id));
    }
}
pub(super) fn c_unsigned_char_typedef_alias(text: &str) -> Option<String> {
    let compact = compact_c_type_text(text);
    let rest = compact.strip_prefix("typedefunsignedchar")?;
    let alias = rest.strip_suffix(';').unwrap_or(rest);
    if is_c_identifier(alias) {
        Some(alias.to_string())
    } else {
        None
    }
}
pub(super) fn c_unsigned_32_typedef_alias(text: &str) -> Option<String> {
    let tokens = c_identifier_tokens(text);
    let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let alias = match token_refs.as_slice() {
        ["typedef", "unsigned", "int", alias] => Some(alias),
        ["typedef", "unsigned", alias] => Some(alias),
        ["typedef", "uint32_t", alias] => Some(alias),
        _ => None,
    }?;
    is_c_identifier(alias).then(|| alias.to_string())
}
pub(super) fn c_param_domain_from_text(
    lo: &Lowering,
    text: &str,
    param_name: Option<&str>,
) -> Option<(DomainEvidence, Vec<EvidenceId>)> {
    if let Some(dependencies) = c_byte_buffer_param_dependencies(lo, text, param_name) {
        Some((DomainEvidence::ByteArray, dependencies))
    } else {
        nose_semantics::type_domain_from_source_text(Lang::C, text)
            .map(|domain| (domain, Vec::new()))
    }
}
pub(super) fn c_byte_buffer_param_dependencies(
    lo: &Lowering,
    text: &str,
    param_name: Option<&str>,
) -> Option<Vec<EvidenceId>> {
    let compact = compact_c_type_text(text);
    if !(compact.contains('*') || compact.contains('[')) {
        return None;
    }
    let tokens = c_parameter_type_tokens(text, param_name);
    if tokens.iter().any(|token| token == "uint8_t")
        || (tokens.iter().any(|token| token == "unsigned")
            && tokens.iter().any(|token| token == "char"))
    {
        return Some(Vec::new());
    }
    lo.type_domain_aliases.iter().find_map(|known| {
        (known.domain == DomainEvidence::ByteArray
            && c_type_tokens_contain_plain_alias(&tokens, &known.alias))
        .then(|| known.evidence.into_iter().collect())
    })
}
pub(super) fn compact_c_type_text(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}
pub(super) fn c_identifier_tokens(text: &str) -> Vec<String> {
    text.split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}
pub(super) fn c_parameter_type_tokens(text: &str, param_name: Option<&str>) -> Vec<String> {
    let mut tokens = c_identifier_tokens(text);
    if let Some(param_name) = param_name {
        if let Some(index) = tokens.iter().rposition(|token| token == param_name) {
            tokens.remove(index);
        }
    }
    tokens
}
pub(super) fn c_type_tokens_contain_plain_alias(tokens: &[String], alias: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        token == alias
            && !matches!(
                index
                    .checked_sub(1)
                    .and_then(|prev| tokens.get(prev))
                    .map(String::as_str),
                Some("struct" | "union" | "enum")
            )
    })
}
pub(super) fn is_c_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
pub(super) fn find_param_list(decl: TsNode) -> Option<TsNode> {
    if decl.kind() == "parameter_list" {
        return Some(decl);
    }
    Lowering::named_children(decl).into_iter().find_map(|c| {
        if c.kind() == "parameter_list" {
            Some(c)
        } else {
            find_param_list(c)
        }
    })
}
pub(super) fn lower_decl(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut assigns = Vec::new();
    for d in Lowering::named_children(node) {
        let (name, value) = match d.kind() {
            "init_declarator" => (
                d.child_by_field_name("declarator")
                    .and_then(|x| declarator_name(lo, x)),
                d.child_by_field_name("value"),
            ),
            "identifier" => (Some(lo.sym(lo.text(d))), None),
            _ => (declarator_name(lo, d), None),
        };
        if let Some(sym) = name {
            let lhs = lo.add(NodeKind::Var, Payload::Name(sym), span, &[]);
            let rhs = value
                .map(|v| lower_expr(lo, v))
                .unwrap_or_else(|| lo.add(NodeKind::Lit, Payload::Lit(LitClass::Null), span, &[]));
            assigns.push(lo.add(NodeKind::Assign, Payload::None, span, &[lhs, rhs]));
        }
    }
    if assigns.len() == 1 {
        assigns.pop().unwrap()
    } else {
        lo.add(NodeKind::Block, Payload::None, span, &assigns)
    }
}

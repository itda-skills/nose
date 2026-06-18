use super::*;

pub(super) fn lower_lambda(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let mut kids = Vec::new();
    if let Some(lambda_type) = node.child_by_field_name("type") {
        lower_lambda_type_params(lo, lambda_type, &mut kids);
    }
    for child in Lowering::named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "lambda_function_type")
    {
        lower_lambda_type_params(lo, child, &mut kids);
    }
    if kids.is_empty() {
        for name in lambda_parameter_names_from_text(lo.text(node)) {
            kids.push(lo.add(NodeKind::Param, Payload::Name(lo.sym(&name)), span, &[]));
        }
    }
    let body = first_statements_child(node)
        .map(|body| lower_function_body(lo, body))
        .unwrap_or_else(|| lo.empty_block(span));
    dedupe_lambda_params(lo, &mut kids);
    kids.push(body);
    lo.add(NodeKind::Lambda, Payload::None, span, &kids)
}
pub(super) fn dedupe_lambda_params(lo: &Lowering, kids: &mut Vec<NodeId>) {
    let mut seen = Vec::new();
    kids.retain(|&kid| {
        if lo.b.kind(kid) != NodeKind::Param {
            return true;
        }
        let Payload::Name(name) = lo.b.payload(kid) else {
            return true;
        };
        if seen.contains(&name) {
            false
        } else {
            seen.push(name);
            true
        }
    });
}
pub(super) fn lower_lambda_type_params(lo: &mut Lowering, node: TsNode, out: &mut Vec<NodeId>) {
    for child in Lowering::named_children(node) {
        if child.kind() == "lambda_parameter" {
            lower_param(lo, child, out);
        } else if matches!(
            child.kind(),
            "lambda_function_type" | "lambda_function_type_parameters"
        ) {
            lower_lambda_type_params(lo, child, out);
        }
    }
}
pub(super) fn lambda_parameter_names_from_text(text: &str) -> Vec<String> {
    let Some(inner) = text
        .trim()
        .strip_prefix('{')
        .and_then(|text| text.strip_suffix('}'))
    else {
        return Vec::new();
    };
    let inner = inner.trim();
    if let Some((header, _body)) = inner.split_once(" in ") {
        return header
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .filter_map(lambda_parameter_name_from_header_part)
            .collect();
    }
    if inner.contains("$0") {
        return vec!["$0".to_string()];
    }
    Vec::new()
}
pub(super) fn lambda_parameter_name_from_header_part(part: &str) -> Option<String> {
    let before_type = part.trim().split(':').next()?.trim();
    let name = before_type
        .split_whitespace()
        .last()
        .unwrap_or(before_type)
        .trim();
    if name.is_empty() || name == "_" {
        None
    } else {
        Some(name.to_string())
    }
}

use crate::lower::Lowering;
use nose_il::{NodeId, NodeKind, Payload};
use tree_sitter::Node as TsNode;

pub(super) fn lower_static_import(lo: &mut Lowering, node: TsNode) -> Option<NodeId> {
    let span = lo.span(node);
    let text = lo.text(node).trim().trim_end_matches(';').trim();
    if text.starts_with("import type ") {
        return None;
    }
    let module = quoted_after_from(text)?;
    let mut assigns = Vec::new();

    if let Some(ns) = text
        .strip_prefix("import * as ")
        .and_then(|rest| rest.split(" from ").next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        assigns.push(crate::lower::import_namespace(lo, span, ns, module));
    } else if let Some((start, end)) = brace_range(text) {
        let inner = &text[start + 1..end];
        for part in inner.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if part.starts_with("type ") {
                continue;
            }
            let (exported, local) = js_import_specifier(part)?;
            assigns.push(crate::lower::import_binding(
                lo, span, local, module, exported,
            ));
        }
    } else if let Some(default_part) = text
        .strip_prefix("import ")
        .and_then(|rest| rest.split(" from ").next())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with(['{', '*']))
    {
        let local = default_part.split(',').next()?.trim();
        assigns.push(crate::lower::import_binding(
            lo, span, local, module, "default",
        ));
    }

    match assigns.len() {
        0 => None,
        1 => assigns.pop(),
        _ => Some(lo.add(NodeKind::Block, Payload::None, span, &assigns)),
    }
}

fn quoted_after_from(text: &str) -> Option<&str> {
    let rest = text.split(" from ").nth(1)?.trim();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)? + 1;
    Some(&rest[1..end])
}

fn brace_range(text: &str) -> Option<(usize, usize)> {
    let start = text.find('{')?;
    let end = text[start + 1..].find('}')? + start + 1;
    Some((start, end))
}

fn js_import_specifier(part: &str) -> Option<(&str, &str)> {
    let part = part.strip_prefix("type ").unwrap_or(part).trim();
    if let Some((exported, local)) = part.split_once(" as ") {
        Some((exported.trim(), local.trim()))
    } else {
        Some((part, part))
    }
}

pub(super) fn is_exportable_decl(k: &str) -> bool {
    matches!(
        k,
        "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "class"
            | "lexical_declaration"
            | "variable_declaration"
            | "type_alias_declaration"
            | "interface_declaration"
            | "enum_declaration"
    )
}

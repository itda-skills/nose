use crate::lower::Lowering;
use nose_il::is_js_identifier_continue;
use tree_sitter::Node as TsNode;

pub(super) fn static_string_key(lo: &Lowering, node: TsNode) -> Option<String> {
    let text = lo.text(node);
    let bytes = text.as_bytes();
    let quote = *bytes.first()?;
    if bytes.len() < 2 || bytes.last().copied()? != quote || !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let inner = &text[1..text.len() - 1];
    if inner.contains('\\') || inner.contains('\n') || inner.contains('\r') {
        return None;
    }
    Some(inner.to_string())
}

pub(super) fn compact_js_expr(text: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            out.push(c);
        } else if c.is_whitespace() {
            let next = chars.clone().find(|next| !next.is_whitespace());
            if out
                .chars()
                .next_back()
                .is_some_and(is_js_identifier_continue)
                && next.is_some_and(is_js_identifier_continue)
            {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub(super) fn strip_outer_parens_owned(mut text: &str) -> String {
    loop {
        let Some(inner) = text.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
            return text.to_string();
        };
        if !balanced_parens(inner) {
            return text.to_string();
        }
        text = inner;
    }
}

fn balanced_parens(text: &str) -> bool {
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

pub(super) fn simple_js_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first == '$' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

use crate::lower::Lowering;
use nose_il::{
    Lang, NodeId, NodeKind, Payload, RegionKind, SourceGranularity, UnitBodyKind,
    UnitContainerKind, UnitDomain, UnitDomains, UnitEvidenceFlag, UnitKind, UnitOrigin,
    UnitSubkind,
};
use tree_sitter::Node as TsNode;

pub(super) fn lower_jsx(lo: &mut Lowering, node: TsNode) -> NodeId {
    let span = lo.span(node);
    let raw_tag = jsx_tag(node).map(|t| lo.text(t));
    let tag = raw_tag.map(canonical_jsx_tag).unwrap_or_default(); // fragment → ""
    let tag_sym = lo.sym(&tag);
    let mut children = Vec::new();
    let mut has_rendered_attrs = false;
    let mut has_bound_attrs = jsx_has_spread_attribute(node);
    for attr in jsx_attributes(node) {
        if let Some(a) = lower_jsx_attr(lo, attr) {
            has_rendered_attrs = true;
            has_bound_attrs |= a.bound;
            children.push(a.id);
        }
    }
    for c in Lowering::named_children(node) {
        match c.kind() {
            "jsx_element" | "jsx_self_closing_element" | "jsx_fragment" => {
                children.push(lower_jsx(lo, c));
            }
            "jsx_text" => {
                let t = jsx_ws(lo.text(c));
                if !t.is_empty() {
                    let s = lo.sym(&t);
                    children.push(lo.add(NodeKind::HtmlText, Payload::Name(s), lo.span(c), &[]));
                }
            }
            "jsx_expression" => {
                // `{title}` → a text node carrying the verbatim expression (exact keeps it
                // distinct; near abstracts it). `{items.map(x => <li/>)}` → a `repeat`
                // control wrapping the template; `{c && <a/>}` / `{a ? <X/> : <Y/>}` → an
                // `if` control. The control node keeps a loop distinct from one element.
                let mut jsxs = Vec::new();
                collect_jsx_descendants(c, &mut jsxs);
                if jsxs.is_empty() {
                    let txt = jsx_ws(lo.text(c));
                    if !txt.is_empty() {
                        let s = lo.sym(&txt);
                        children.push(lo.add(
                            NodeKind::HtmlText,
                            Payload::Name(s),
                            lo.span(c),
                            &[],
                        ));
                    }
                } else {
                    let cspan = lo.span(c);
                    let is_repeat = {
                        let t = lo.text(c);
                        t.contains(".map(") || t.contains(".flatMap(")
                    };
                    let mut tkids = Vec::new();
                    for j in jsxs {
                        tkids.push(lower_jsx(lo, j));
                    }
                    let ksym = lo.sym(if is_repeat { "repeat" } else { "if" });
                    children.push(lo.add(
                        NodeKind::HtmlControl,
                        Payload::Name(ksym),
                        cspan,
                        &tkids,
                    ));
                }
            }
            _ => {}
        }
    }
    let el = lo.add(
        NodeKind::HtmlElement,
        Payload::Name(tag_sym),
        span,
        &children,
    );
    lo.push_unit_with_origin(
        el,
        UnitKind::Block,
        Some(tag_sym),
        jsx_markup_origin(
            lo.lang,
            raw_tag.is_none(),
            raw_tag.is_some_and(is_jsx_component_tag),
            has_rendered_attrs,
            has_bound_attrs,
        ),
    );
    el
}

fn jsx_markup_origin(
    lang: Lang,
    fragment: bool,
    component_tag: bool,
    has_rendered_attrs: bool,
    has_bound_attrs: bool,
) -> UnitOrigin {
    let mut origin = UnitOrigin::new(
        UnitDomains::of(UnitDomain::Markup),
        if fragment {
            UnitSubkind::MarkupFragment
        } else {
            UnitSubkind::HtmlElement
        },
        UnitBodyKind::DeclarativeDenotation,
        if fragment {
            SourceGranularity::Fragment
        } else {
            SourceGranularity::Element
        },
        RegionKind::Markup,
    )
    .with_container(if lang == Lang::TypeScript {
        UnitContainerKind::Tsx
    } else {
        UnitContainerKind::Jsx
    });
    if component_tag {
        origin = origin.with_evidence(UnitEvidenceFlag::ComponentTag);
    }
    if has_bound_attrs {
        origin = origin.with_evidence(UnitEvidenceFlag::BoundAttributes);
    } else if has_rendered_attrs {
        origin = origin.with_evidence(UnitEvidenceFlag::StaticAttrsOnly);
    }
    origin
}

/// Collect the top-most JSX elements nested inside an expression (not recursing into the
/// JSX itself) — finds the template child of a `.map`, `&&`, or ternary.
fn collect_jsx_descendants<'a>(node: TsNode<'a>, out: &mut Vec<TsNode<'a>>) {
    for c in Lowering::named_children(node) {
        if matches!(
            c.kind(),
            "jsx_element" | "jsx_self_closing_element" | "jsx_fragment"
        ) {
            out.push(c);
        } else {
            collect_jsx_descendants(c, out);
        }
    }
}

/// Collapse JSX whitespace text (DOM-insignificant) and trim.
fn jsx_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// DOM tag for a JSX element name: lowercased (DOM tags already are; components become a
/// stable lowercase token), with React-Router link components mapped to `a`.
fn canonical_jsx_tag(name: &str) -> String {
    match name {
        "Link" | "NavLink" | "RouterLink" => "a".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn is_jsx_component_tag(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

/// Map a JSX attribute to its rendered DOM attribute, or `None` to drop it (events,
/// React bookkeeping). `className`→`class`, `htmlFor`→`for`, router `to`→`href`.
fn canonical_jsx_attr(name: &str) -> Option<String> {
    match name {
        "className" => Some("class".to_string()),
        "htmlFor" => Some("for".to_string()),
        "to" => Some("href".to_string()),
        "key" | "ref" => None,
        n if n.starts_with("on") && n.len() > 2 && n.as_bytes()[2].is_ascii_uppercase() => None,
        n => Some(n.to_ascii_lowercase()),
    }
}

struct LoweredJsxAttr {
    id: NodeId,
    bound: bool,
}

/// `name="str"` → `HtmlAttr(name)[Lit(value)]`; `name={expr}` → hole value; bare → boolean.
/// Returns `None` for dropped (event/bookkeeping) attributes.
fn lower_jsx_attr(lo: &mut Lowering, attr: TsNode) -> Option<LoweredJsxAttr> {
    let span = lo.span(attr);
    let kids = Lowering::named_children(attr);
    let raw_name = lo.text(*kids.first()?);
    let name = canonical_jsx_attr(raw_name)?;
    let bound = matches!(kids.get(1), Some(v) if v.kind() != "string");
    let children: Vec<NodeId> = match kids.get(1) {
        None => Vec::new(), // boolean attribute
        Some(v) if v.kind() == "string" => {
            let raw = lo.text(*v);
            let val = jsx_ws(raw.trim_matches('"').trim_matches('\''));
            let s = lo.sym(&val);
            vec![lo.add(NodeKind::Lit, Payload::Name(s), span, &[])]
        }
        Some(v) => {
            // `{expr}` value → keep the verbatim expression text (exact distinguishes
            // different bound expressions; near abstracts the value node).
            let val = jsx_ws(lo.text(*v));
            let s = lo.sym(&val);
            vec![lo.add(NodeKind::Lit, Payload::Name(s), span, &[])]
        }
    };
    let nsym = lo.sym(&name);
    Some(LoweredJsxAttr {
        id: lo.add(NodeKind::HtmlAttr, Payload::Name(nsym), span, &children),
        bound,
    })
}

fn jsx_tag(node: TsNode) -> Option<TsNode> {
    if let Some(n) = node.child_by_field_name("name") {
        return Some(n);
    }
    Lowering::named_children(node)
        .into_iter()
        .find(|c| c.kind() == "jsx_opening_element")
        .and_then(|o| o.child_by_field_name("name"))
}

fn jsx_attr_host(node: TsNode) -> Option<TsNode> {
    if node.kind() == "jsx_element" {
        Lowering::named_children(node)
            .into_iter()
            .find(|c| c.kind() == "jsx_opening_element")
    } else {
        Some(node)
    }
}

fn jsx_attributes(node: TsNode) -> Vec<TsNode> {
    match jsx_attr_host(node) {
        Some(h) => Lowering::named_children(h)
            .into_iter()
            .filter(|c| c.kind() == "jsx_attribute")
            .collect(),
        None => Vec::new(),
    }
}

fn jsx_has_spread_attribute(node: TsNode) -> bool {
    jsx_attr_host(node).is_some_and(|host| {
        Lowering::named_children(host).into_iter().any(|child| {
            child.kind() == "jsx_expression"
                && Lowering::named_children(child)
                    .into_iter()
                    .any(|inner| inner.kind() == "spread_element")
        })
    })
}

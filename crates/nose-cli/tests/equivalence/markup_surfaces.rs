use super::css_surfaces::css_fp;
use super::*;

// ----- HTML (declarative markup) exact-channel semantics -----
//
// Markup units are fingerprinted by their canonical rendered DOM (see
// `nose-normalize::html`): attribute order/boolean-form/whitespace/class-set are
// normalized, but tag/structure/text/value differences are kept distinct.

/// The declarative DOM fingerprint of the first top-level `HtmlElement` in `src`.
fn html_fp(interner: &Interner, src: &str) -> Vec<u64> {
    let ils = nose_frontend::lower_source_regions(
        FileId(0),
        "t.html",
        src.as_bytes(),
        Lang::Html,
        interner,
    );
    let markup = ils
        .iter()
        .find(|il| il.meta.lang == Lang::Html)
        .expect("a markup (html) region");
    let n = normalize(markup, interner, &NormalizeOptions::default());
    let root = n
        .children(n.root)
        .iter()
        .copied()
        .find(|&c| n.node(c).kind == nose_il::NodeKind::HtmlElement)
        .expect("a top-level HtmlElement unit");
    nose_normalize::value_fingerprint(&n, root, interner)
}

#[test]
fn html_same_dom_converges_under_attr_order_boolean_whitespace_class_set() {
    let i = Interner::new();
    let a = r#"<div class="card x"><img src="a.png" alt="p"><button type="button" disabled>Go</button></div>"#;
    // attrs reordered, boolean form `disabled=""`, class tokens reordered, extra whitespace
    let b = "<div class=\"x card\">\n  <img alt=\"p\"   src=\"a.png\">\n  <button disabled=\"\" type=\"button\">Go</button>\n</div>";
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, b),
        "same rendered DOM must converge"
    );
}

#[test]
fn html_text_and_value_differences_do_not_converge() {
    let i = Interner::new();
    let a = r#"<div class="card"><h3>Title</h3><a href="/a">Link</a></div>"#;
    let b = r#"<div class="card"><h3>Other</h3><a href="/a">Link</a></div>"#; // different text
    let c = r#"<div class="card"><h3>Title</h3><a href="/b">Link</a></div>"#; // different href
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "different text must not merge"
    );
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, c),
        "different attr value must not merge"
    );
}

#[test]
fn html_child_order_is_significant() {
    let i = Interner::new();
    let a = r#"<ul class="m"><li>one</li><li>two</li></ul>"#;
    let b = r#"<ul class="m"><li>two</li><li>one</li></ul>"#;
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "DOM child order must be significant"
    );
}

// ---- cross-dialect markup (HTML/Vue/Svelte/JSX/TSX) ---------------------------------
// The exact (semantic) fingerprint of a markup unit must stay FAITHFUL to the rendered
// DOM even as the five dialects' idioms (control flow, directives, holes) are normalized
// into the common IL — so the moat (equal fingerprint ⟹ equal rendered DOM) still holds.

/// Exact DOM fingerprint of the OUTERMOST markup element in `src`, lowered as `lang`
/// (works for `.vue`/`.svelte` markup AND JSX/TSX, whose markup lives in the JS region).
fn markup_fp(i: &Interner, path: &str, lang: Lang, src: &str) -> Vec<u64> {
    fn outer(n: &nose_il::Il, node: NodeId) -> Option<NodeId> {
        if n.node(node).kind == nose_il::NodeKind::HtmlElement {
            return Some(node);
        }
        n.children(node).iter().find_map(|&c| outer(n, c))
    }
    let ils = nose_frontend::lower_source_regions(FileId(0), path, src.as_bytes(), lang, i);
    for il in &ils {
        let n = normalize(il, i, &NormalizeOptions::default());
        if let Some(root) = outer(&n, n.root) {
            return nose_normalize::value_fingerprint(&n, root, i);
        }
    }
    panic!("no markup unit lowered from {path}");
}

#[test]
fn markup_loop_does_not_merge_with_single_element() {
    // SOUNDNESS: a `{#each}`/`v-for`/`.map` repeat renders 0..N elements — it must NEVER
    // share a fingerprint with a single static element (the HtmlControl wrapper keeps it
    // distinct), or nose would claim a loop and one element have equal rendered DOM.
    let i = Interner::new();
    let loop_ = r#"<ul class="m">{#each items as x}<li class="r">row</li>{/each}</ul>"#;
    let single = r#"<ul class="m"><li class="r">row</li></ul>"#;
    assert_ne!(
        html_fp(&i, loop_),
        html_fp(&i, single),
        "a repeat must not exact-merge with a single element"
    );
}

#[test]
fn markup_repeat_and_conditional_controls_are_distinct() {
    // A repeat (`{#each}`) and a conditional (`{#if}`) are different rendered behavior.
    let i = Interner::new();
    let each = r#"<ul class="m">{#each xs as x}<li>z</li>{/each}</ul>"#;
    let if_ = r#"<ul class="m">{#if cond}<li>z</li>{/if}</ul>"#;
    assert_ne!(
        html_fp(&i, each),
        html_fp(&i, if_),
        "repeat and conditional controls must not merge"
    );
}

#[test]
fn markup_control_converges_vue_directive_and_svelte_block() {
    // CROSS-DIALECT (sound positive): a Vue `v-for` element and the equivalent Svelte
    // `{#each}` block render the SAME DOM (a repeat of an identical template) → converge.
    let i = Interner::new();
    let vue = r#"<ul class="list"><li class="item" v-for="x in xs" :key="x.id">row text</li></ul>"#;
    let svelte =
        r#"<ul class="list">{#each xs as x (x.id)}<li class="item">row text</li>{/each}</ul>"#;
    assert_eq!(
        html_fp(&i, vue),
        html_fp(&i, svelte),
        "v-for and {{#each}} of the same template must converge"
    );
}

#[test]
fn markup_bound_attribute_keeps_its_expression_distinct() {
    // SOUNDNESS: a directive-bound attribute keeps its expression verbatim — two different
    // bound expressions must not be claimed equal (no hole-collapse on the exact channel).
    let i = Interner::new();
    let a = r#"<img class="a" :src="hero" :alt="name">"#;
    let b = r#"<img class="a" :src="thumb" :alt="name">"#;
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "different bound expressions must not merge on the exact channel"
    );
}

#[test]
fn markup_jsx_converges_with_html_on_identical_static_dom() {
    // CROSS-DIALECT (sound positive): a JSX element and a hand-written HTML element with
    // the SAME rendered DOM (className→class, static text/attrs identical) converge — and
    // JSX markup now lives in the common declarative IL, not an imperative Call-tree.
    let i = Interner::new();
    let jsx = r#"export function Nav(){return <li className="nav-item"><a className="nav-link" href="/login">Sign in</a></li>;}"#;
    let html = r#"<li class="nav-item"><a class="nav-link" href="/login">Sign in</a></li>"#;
    assert_eq!(
        markup_fp(&i, "n.jsx", Lang::JavaScript, jsx),
        markup_fp(&i, "n.html", Lang::Html, html),
        "JSX and HTML with identical rendered DOM must converge"
    );
}

#[test]
fn markup_jsx_map_does_not_merge_with_single_element() {
    // SOUNDNESS (JSX side): `{xs.map(x => <li/>)}` is a repeat — distinct from a single li.
    let i = Interner::new();
    let mapped = r#"export function L(){return <ul className="m">{xs.map(x => <li className="r">row</li>)}</ul>;}"#;
    let single =
        r#"export function L(){return <ul className="m"><li className="r">row</li></ul>;}"#;
    assert_ne!(
        markup_fp(&i, "a.jsx", Lang::JavaScript, mapped),
        markup_fp(&i, "b.jsx", Lang::JavaScript, single),
        "a JSX .map repeat must not merge with a single element"
    );
}

#[test]
fn html_inline_style_is_computed_canonicalized() {
    // Inline `style="…"` reuses the CSS computed-style canon: order-independent,
    // color/shorthand/unit normalized.
    let i = Interner::new();
    let a = r#"<div style="margin: 0 0 0 0; color: #ffffff; padding: 4px"><span>x</span></div>"#;
    let b = r#"<div style="color: white; padding: 4px; margin: 0"><span>x</span></div>"#;
    let c = r#"<div style="color: black; padding: 4px; margin: 0"><span>x</span></div>"#;
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, b),
        "computed-equal inline styles converge"
    );
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, c),
        "different inline style must not merge"
    );
}

#[test]
fn html_vue_directive_shorthand_canonicalizes() {
    // `:x` ≡ `v-bind:x` and `@x` ≡ `v-on:x` — same binding, different spelling.
    let i = Interner::new();
    let short = r#"<button :class="c" @click="f"><i class="ico"></i>Go</button>"#;
    let long = r#"<button v-bind:class="c" v-on:click="f"><i class="ico"></i>Go</button>"#;
    assert_eq!(
        html_fp(&i, short),
        html_fp(&i, long),
        "directive shorthand must canonicalize"
    );
}

#[test]
fn html_preformatted_whitespace_is_significant() {
    // Soundness: inside <pre>/<textarea> whitespace is preserved by the renderer, so
    // two blocks differing only in indentation render differently and must NOT merge.
    let i = Interner::new();
    let a = "<pre>fn f() {\n    return 1;\n    return 2;\n}</pre>";
    let b = "<pre>fn f() {\n  return 1;\n  return 2;\n}</pre>"; // 2-space indent
    assert_ne!(
        html_fp(&i, a),
        html_fp(&i, b),
        "<pre> whitespace must be significant"
    );
    let a2 = "<pre>fn f() {\n    return 1;\n    return 2;\n}</pre>";
    assert_eq!(
        html_fp(&i, a),
        html_fp(&i, a2),
        "identical <pre> still converges"
    );
    // Outside <pre>, flow whitespace stays INsignificant.
    let p1 = "<p>hello world   again here friend</p>";
    let p2 = "<p>hello world\n  again here friend</p>";
    assert_eq!(
        html_fp(&i, p1),
        html_fp(&i, p2),
        "flow whitespace stays insignificant"
    );
}

#[test]
fn html_fingerprint_is_domain_disjoint_from_css_and_imperative() {
    let i = Interner::new();
    let html = html_fp(
        &i,
        r#"<div class="card"><h3>Title</h3><p>body text</p></div>"#,
    );
    let css = css_fp(&i, ".card { display: flex; gap: 8px; padding: 12px; }");
    let py = value_fp(&i, "def f(x):\n    return x + 5\n", Lang::Python);
    assert_ne!(html, css, "HTML and CSS fingerprints must be disjoint");
    assert_ne!(
        html, py,
        "HTML and imperative fingerprints must be disjoint"
    );
}

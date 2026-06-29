use super::support::{evidence_by_id, imported_binding_symbol_records, lower_js};
use nose_il::{stable_symbol_hash, EvidenceAnchor, EvidenceKind, NodeKind, SymbolEvidenceKind};

#[test]
fn commonjs_node_timers_destructuring_emits_dependency_backed_imported_bindings() {
    let il = lower_js(
        r#"
const { setTimeout, setImmediate: immediate } = require("node:timers/promises");
function f() {
  return setTimeout(1).then(() => immediate());
}
"#,
    );

    let timeout = imported_binding_symbol_records(&il, "node:timers/promises", "setTimeout");
    let immediate = imported_binding_symbol_records(&il, "node:timers/promises", "setImmediate");
    assert_eq!(
        timeout.len(),
        1,
        "setTimeout should have one CJS import proof"
    );
    assert_eq!(
        immediate.len(),
        1,
        "setImmediate alias should have one CJS import proof"
    );

    for record in timeout.iter().chain(immediate.iter()) {
        assert_eq!(
            record.dependencies.len(),
            1,
            "CJS import proof must depend on unshadowed require"
        );
        let dependency =
            evidence_by_id(&il, record.dependencies[0]).expect("require dependency exists");
        assert!(
            matches!(
                dependency.anchor,
                EvidenceAnchor::Node {
                    kind: NodeKind::Var,
                    ..
                }
            ),
            "require dependency should be anchored to the require callee"
        );
        assert!(
            matches!(
                dependency.kind,
                EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal { name_hash })
                    if name_hash == stable_symbol_hash("require")
            ),
            "CJS import proof should depend on unshadowed require evidence"
        );
    }
}

#[test]
fn commonjs_node_timers_destructuring_stays_closed_for_unsafe_shapes() {
    let il = lower_js(
        r#"
let { setTimeout } = require("node:timers/promises");
const { [name]: setImmediate } = require("node:timers/promises");
function g(require) {
  const { setTimeout: delay } = require("timers/promises");
  return delay;
}
"#,
    );

    assert!(
        imported_binding_symbol_records(&il, "node:timers/promises", "setTimeout").is_empty(),
        "mutable CJS destructuring should remain closed"
    );
    assert!(
        imported_binding_symbol_records(&il, "node:timers/promises", "setImmediate").is_empty(),
        "computed CJS destructuring should remain closed"
    );
    assert!(
        imported_binding_symbol_records(&il, "timers/promises", "setTimeout").is_empty(),
        "shadowed require CJS destructuring should remain closed"
    );
}

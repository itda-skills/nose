use super::support::{
    js_own_property_guard_evidence, js_record_shape_guard_evidence, lower_js,
    qualified_global_evidence_count, record_has_qualified_global_dependency_with_root,
};
use nose_il::{
    stable_symbol_hash, EvidenceKind, GuardEvidenceKind, JsRecordGuardComparison,
    JsRecordGuardNullCheck, NodeKind,
};

#[test]
fn js_own_property_guards_emit_guard_evidence_with_api_dependencies() {
    let il = lower_js(
        "function hasOwn(value) { return Object.hasOwn(value, \"ready\"); }
         function hasOwnCall(value) { return Object.prototype.hasOwnProperty.call(value, \"ready\"); }",
    );

    let guards = js_own_property_guard_evidence(&il);
    assert_eq!(guards.len(), 2);
    assert!(guards.iter().any(|record| {
        matches!(
            record.kind,
            EvidenceKind::Guard(GuardEvidenceKind::JsOwnProperty { api_path_hash })
                if api_path_hash == stable_symbol_hash("Object.hasOwn")
        ) && record.dependencies.len() == 1
            && record_has_qualified_global_dependency_with_root(
                &il,
                record,
                "Object.hasOwn",
                "Object",
            )
    }));
    assert!(guards.iter().any(|record| {
        matches!(
            record.kind,
            EvidenceKind::Guard(GuardEvidenceKind::JsOwnProperty { api_path_hash })
                if api_path_hash == stable_symbol_hash("Object.prototype.hasOwnProperty.call")
        ) && record.dependencies.len() == 1
            && record_has_qualified_global_dependency_with_root(
                &il,
                record,
                "Object.prototype.hasOwnProperty.call",
                "Object",
            )
    }));
}

#[test]
fn js_record_shape_guards_emit_guard_evidence_with_api_dependencies() {
    let il = lower_js(
        "function direct(value) {
            return typeof value === \"object\" && value !== null && !Array.isArray(value);
         }
         function truthy(input) {
            return Boolean(input) && typeof input === \"object\" && !Array.isArray(input);
         }",
    );

    let guards = js_record_shape_guard_evidence(&il);
    assert_eq!(guards.len(), 2);
    assert!(guards.iter().any(|record| {
        matches!(
            record.kind,
            EvidenceKind::Guard(GuardEvidenceKind::JsRecordShape {
                subject_hash,
                null_check: JsRecordGuardNullCheck::StrictNonNull,
                comparison: JsRecordGuardComparison::StrictOnly,
            }) if subject_hash == stable_symbol_hash("value")
        ) && record.dependencies.len() == 1
            && record_has_qualified_global_dependency_with_root(
                &il,
                record,
                "Array.isArray",
                "Array",
            )
    }));
    assert!(guards.iter().any(|record| {
        matches!(
            record.kind,
            EvidenceKind::Guard(GuardEvidenceKind::JsRecordShape {
                subject_hash,
                null_check: JsRecordGuardNullCheck::BooleanGlobalTruthy,
                comparison: JsRecordGuardComparison::StrictOnly,
            }) if subject_hash == stable_symbol_hash("input")
        ) && record.dependencies.len() == 2
            && record_has_qualified_global_dependency_with_root(
                &il,
                record,
                "Array.isArray",
                "Array",
            )
    }));
}

#[test]
fn js_record_shape_guard_evidence_respects_array_shadowing() {
    let il = lower_js(
        "function f(Array, value) {
            return typeof value === \"object\" && value !== null && !Array.isArray(value);
         }
         function g(scope, value) {
            const { Array } = scope;
            return typeof value === \"object\" && value !== null && !Array.isArray(value);
         }
         function h(Boolean, value) {
            return Boolean(value) && typeof value === \"object\" && !Array.isArray(value);
         }
         function i(scope, value) {
            const { Boolean } = scope;
            return Boolean(value) && typeof value === \"object\" && !Array.isArray(value);
         }",
    );

    assert!(js_record_shape_guard_evidence(&il).is_empty());
    assert!(
        il.evidence
            .iter()
            .all(|record| !matches!(record.kind, EvidenceKind::LibraryApi(_))),
        "shadowed Array/Boolean roots must not emit API contract evidence"
    );
}

#[test]
fn js_record_shape_guard_evidence_requires_typeof_keyword_boundary() {
    let il = lower_js(
        "function f(value) {
            return typeofvalue === \"object\" && value !== null && !Array.isArray(value);
         }
         function g(value) {
            return \"object\" === typeofvalue && value !== null && !Array.isArray(value);
         }",
    );

    assert!(js_record_shape_guard_evidence(&il).is_empty());
}

#[test]
fn js_qualified_global_evidence_respects_shadowed_roots() {
    let il = lower_js(
        "function a(Object, value) { return Object.hasOwn(value, \"ready\"); }
         const Object = { prototype: { hasOwnProperty: { call() { return true; } } } };
         function b(value) { return Object.prototype.hasOwnProperty.call(value, \"ready\"); }
         function c(Array, lookup) { return Array.from(lookup.keys()).includes(\"ready\"); }
         function d(scope, lookup) { const { Array } = scope; return Array.from(lookup.keys()).includes(\"ready\"); }",
    );

    assert_eq!(
        qualified_global_evidence_count(&il, "Object.hasOwn", NodeKind::Seq),
        0
    );
    assert_eq!(
        qualified_global_evidence_count(&il, "Object.prototype.hasOwnProperty.call", NodeKind::Seq),
        0
    );
    assert_eq!(
        qualified_global_evidence_count(&il, "Array.from", NodeKind::Field),
        0
    );
    assert!(
        il.evidence
            .iter()
            .all(|record| !matches!(record.kind, EvidenceKind::LibraryApi(_))),
        "shadowed JS static globals must not emit API contract evidence"
    );
    assert!(js_own_property_guard_evidence(&il).is_empty());
}

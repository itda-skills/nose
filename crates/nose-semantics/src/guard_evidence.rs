//! Guard sequence evidence admission.

use super::*;

pub(super) fn guard_evidence_at_sequence_span(
    il: &Il,
    span: Span,
) -> EvidenceResolution<GuardEvidenceKind> {
    let mut found = None;
    for record in il.evidence_anchored_at(span) {
        if !matches!(record.anchor, EvidenceAnchor::Sequence { span: anchor_span } if anchor_span == span)
        {
            continue;
        }
        let EvidenceKind::Guard(kind) = record.kind else {
            continue;
        };
        if record.status != EvidenceStatus::Asserted
            || !guard_evidence_dependencies_valid(il, record, kind, span)
        {
            return EvidenceResolution::Ambiguous;
        }
        match found {
            None => found = Some(kind),
            Some(existing) if existing == kind => {}
            Some(_) => return EvidenceResolution::Ambiguous,
        }
    }
    found.map_or(EvidenceResolution::Missing, EvidenceResolution::Found)
}

pub(super) fn guard_evidence_dependencies_valid(
    il: &Il,
    record: &EvidenceRecord,
    kind: GuardEvidenceKind,
    span: Span,
) -> bool {
    match kind {
        GuardEvidenceKind::JsRecordShape { null_check, .. } => {
            js_record_shape_guard_dependencies_valid(il, record, null_check, span)
        }
        GuardEvidenceKind::JsOwnProperty { api_path_hash } => {
            js_own_property_guard_dependencies_valid(il, record, api_path_hash, span)
        }
    }
}

pub(super) fn js_record_shape_guard_dependencies_valid(
    il: &Il,
    record: &EvidenceRecord,
    null_check: nose_il::JsRecordGuardNullCheck,
    span: Span,
) -> bool {
    let mut has_array_is_array = false;
    let mut has_boolean = null_check != nose_il::JsRecordGuardNullCheck::BooleanGlobalTruthy;
    for id in &record.dependencies {
        let Some(dependency) = il.evidence_record_by_id(*id) else {
            return false;
        };
        if dependency.id != *id || !dependency.anchor.matches_span(span) {
            return false;
        }
        match dependency.kind {
            EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal { path_hash })
                if path_hash == stable_symbol_hash("Array.isArray")
                    && qualified_global_dependency_valid(il, dependency, span, "Array.isArray") =>
            {
                has_array_is_array = true;
            }
            EvidenceKind::Symbol(SymbolEvidenceKind::UnshadowedGlobal { name_hash })
                if null_check == nose_il::JsRecordGuardNullCheck::BooleanGlobalTruthy
                    && name_hash == stable_symbol_hash("Boolean")
                    && dependency.status == EvidenceStatus::Asserted
                    && il.evidence_dependencies_asserted(dependency) =>
            {
                has_boolean = true;
            }
            _ => return false,
        }
    }
    has_array_is_array && has_boolean
}

pub(super) fn js_own_property_guard_api_path(path_hash: u64) -> Option<&'static str> {
    if path_hash == stable_symbol_hash("Object.hasOwn") {
        Some("Object.hasOwn")
    } else if path_hash == stable_symbol_hash("Object.prototype.hasOwnProperty.call") {
        Some("Object.prototype.hasOwnProperty.call")
    } else {
        None
    }
}

pub(super) fn js_own_property_guard_dependencies_valid(
    il: &Il,
    record: &EvidenceRecord,
    api_path_hash: u64,
    span: Span,
) -> bool {
    let Some(api_path) = js_own_property_guard_api_path(api_path_hash) else {
        return false;
    };
    let mut has_api = false;
    for id in &record.dependencies {
        let Some(dependency) = il.evidence_record_by_id(*id) else {
            return false;
        };
        if dependency.id != *id || !dependency.anchor.matches_span(span) {
            return false;
        }
        match dependency.kind {
            EvidenceKind::Symbol(SymbolEvidenceKind::QualifiedGlobal { path_hash })
                if path_hash == api_path_hash
                    && qualified_global_dependency_valid(il, dependency, span, api_path) =>
            {
                has_api = true;
            }
            _ => return false,
        }
    }
    has_api
}

/// Prove that a lowered `Seq("record_guard")` denotes the first-party JS-like
/// record-shape guard contract. The surface tag is not enough: the sequence must
/// carry both matching sequence-surface evidence and a dedicated guard evidence
/// record whose dependencies are asserted.
pub fn record_shape_guard_for_node(il: &Il, interner: &Interner, node: NodeId) -> bool {
    record_shape_guard_evidence_for_node(il, interner, node).is_some()
}

pub fn record_shape_guard_evidence_for_node(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<GuardEvidenceKind> {
    if il.kind(node) != NodeKind::Seq || !js_like_lang(il.meta.lang) {
        return None;
    }
    let span = il.node(node).span;
    if !matches!(
        sequence_surface_evidence_at_sequence_span(il, span),
        EvidenceResolution::Found(SequenceSurfaceKind::RecordGuard)
    ) {
        return None;
    }
    match guard_evidence_at_sequence_span(il, span) {
        EvidenceResolution::Found(
            evidence @ GuardEvidenceKind::JsRecordShape { subject_hash, .. },
        ) if record_shape_guard_sequence_matches(il, interner, node, subject_hash) => {
            Some(evidence)
        }
        EvidenceResolution::Found(_)
        | EvidenceResolution::Ambiguous
        | EvidenceResolution::Missing => None,
    }
}

pub(super) fn record_shape_guard_sequence_matches(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    subject_hash: u64,
) -> bool {
    let Payload::Name(tag) = il.node(node).payload else {
        return false;
    };
    if sequence_surface_kind_for_tag(il.meta.lang, Some(interner.resolve(tag)))
        != Some(SequenceSurfaceKind::RecordGuard)
    {
        return false;
    }
    let [subject, object, non_null, not_array] = il.children(node) else {
        return false;
    };
    record_shape_guard_subject_matches(il, interner, *subject, subject_hash)
        && literal_string_hash(il, *object) == Some(stable_symbol_hash("object"))
        && literal_string_hash(il, *non_null) == Some(stable_symbol_hash("non_null"))
        && literal_string_hash(il, *not_array) == Some(stable_symbol_hash("not_array"))
}

pub(super) fn record_shape_guard_subject_matches(
    il: &Il,
    interner: &Interner,
    subject: NodeId,
    subject_hash: u64,
) -> bool {
    if il.kind(subject) != NodeKind::Var {
        return false;
    }
    match il.node(subject).payload {
        Payload::Name(_) => node_name_hash(il, interner, subject) == Some(subject_hash),
        Payload::Cid(_) => true,
        _ => false,
    }
}

/// Prove that a lowered `Seq("own_property_guard")` denotes a first-party
/// JS-like own-property test such as `Object.hasOwn(obj, key)`. The surface tag
/// is not enough: exact consumers require matching sequence evidence, dedicated
/// guard evidence, and a supported qualified-global API dependency.
pub fn own_property_guard_for_node(il: &Il, interner: &Interner, node: NodeId) -> bool {
    own_property_guard_evidence_for_node(il, interner, node).is_some()
}

pub fn own_property_guard_evidence_for_node(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> Option<GuardEvidenceKind> {
    if il.kind(node) != NodeKind::Seq || !js_like_lang(il.meta.lang) {
        return None;
    }
    let span = il.node(node).span;
    if !matches!(
        sequence_surface_evidence_at_sequence_span(il, span),
        EvidenceResolution::Found(SequenceSurfaceKind::OwnPropertyGuard)
    ) {
        return None;
    }
    match guard_evidence_at_sequence_span(il, span) {
        EvidenceResolution::Found(evidence @ GuardEvidenceKind::JsOwnProperty { .. })
            if own_property_guard_sequence_matches(il, interner, node) =>
        {
            Some(evidence)
        }
        EvidenceResolution::Found(_)
        | EvidenceResolution::Ambiguous
        | EvidenceResolution::Missing => None,
    }
}

pub fn own_property_guard_evidence_at_span(il: &Il, span: Span) -> bool {
    if !js_like_lang(il.meta.lang)
        || !matches!(
            sequence_surface_evidence_at_sequence_span(il, span),
            EvidenceResolution::Found(SequenceSurfaceKind::OwnPropertyGuard)
        )
    {
        return false;
    }
    matches!(
        guard_evidence_at_sequence_span(il, span),
        EvidenceResolution::Found(GuardEvidenceKind::JsOwnProperty { .. })
    )
}

pub(super) fn own_property_guard_sequence_matches(
    il: &Il,
    interner: &Interner,
    node: NodeId,
) -> bool {
    let Payload::Name(tag) = il.node(node).payload else {
        return false;
    };
    if sequence_surface_kind_for_tag(il.meta.lang, Some(interner.resolve(tag)))
        != Some(SequenceSurfaceKind::OwnPropertyGuard)
    {
        return false;
    }
    let [_, _, own, present] = il.children(node) else {
        return false;
    };
    literal_string_hash(il, *own) == Some(stable_symbol_hash("own"))
        && literal_string_hash(il, *present) == Some(stable_symbol_hash("present"))
}

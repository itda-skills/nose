//! First-party call-target evidence producer.
//!
//! Consumers must not resolve user calls from raw spelling. This pass is the
//! narrow boundary where the lowered file's binding shape is checked and a
//! direct in-file or imported callable target is materialized as evidence.

use nose_il::{
    stable_symbol_hash, CallTargetEvidenceKind, EvidenceAnchor, EvidenceEmitter, EvidenceId,
    EvidenceKind, EvidenceProvenance, EvidenceRecord, EvidenceStatus, Il, Interner, LoopKind,
    NodeId, NodeKind, Payload, Symbol, SymbolEvidenceKind, UnitKind,
};
use nose_semantics::{
    imported_occurrence_symbol_dependencies_valid_with_cache, language_core_evidence_provenance,
    ImportedOccurrenceValidationCache, BUILTIN_COMPAT_PACK_ID,
};
use rustc_hash::{FxHashMap, FxHashSet};

const DIRECT_FUNCTION_RULE: &str = "normalize.call_target.direct_function";
const IMPORTED_FUNCTION_RULE: &str = "normalize.call_target.imported_function";
const IMPORTED_MEMBER_RULE: &str = "normalize.call_target.imported_member";
const IMPORTED_BINDING_OCCURRENCE_RULE: &str =
    "normalize.symbol_imported_binding_occurrence_for_call_target";
const IMPORTED_NAMESPACE_OCCURRENCE_RULE: &str =
    "normalize.symbol_imported_namespace_occurrence_for_call_target";

#[derive(Clone, Copy)]
struct DirectFunctionTarget {
    root: NodeId,
    name_hash: u64,
}

#[derive(Clone, Copy)]
struct ImportedFunctionTarget {
    module_hash: u64,
    exported_hash: u64,
    local_hash: u64,
    dependency: EvidenceId,
}

#[derive(Clone, Copy)]
struct ImportedMemberTarget {
    module_hash: u64,
    exported_hash: u64,
    member_hash: u64,
    dependency: EvidenceId,
}

#[derive(Clone, Copy)]
enum ImportedBindingUse {
    FunctionCallee,
    MemberReceiver,
}

#[derive(Clone, Copy)]
struct CallTargetEvidenceProvenance {
    current: EvidenceProvenance,
    legacy_pack_hash: u64,
}

pub(crate) fn run(il: &mut Il, interner: &Interner) {
    let provenance = call_target_evidence_provenance(il);
    let targets = unique_direct_function_targets(il, interner);
    let mut calls = Vec::new();
    collect_call_nodes(il, il.root, &mut calls);

    let function_names = function_unit_names(il);
    let scope_bound = scope_bound_symbols(il, &function_names);
    let mut proven = Vec::new();
    let mut scope_stack = Vec::new();
    if !targets.is_empty() {
        collect_call_targets(
            il,
            interner,
            il.root,
            &mut scope_stack,
            &targets,
            &scope_bound,
            &mut proven,
        );
    }
    for (call, target) in proven {
        upsert(
            il,
            EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
            EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction {
                target_span: il.node(target.root).span,
                name_hash: target.name_hash,
            }),
            DIRECT_FUNCTION_RULE,
            provenance,
            Vec::new(),
        );
    }
    let mut imported_occurrence_cache = ImportedOccurrenceValidationCache::default();
    for call in calls {
        record_imported_call_target(
            il,
            interner,
            call,
            provenance,
            &mut imported_occurrence_cache,
        );
    }
}

fn upsert(
    il: &mut Il,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    _rule: &'static str,
    provenance: CallTargetEvidenceProvenance,
    dependencies: Vec<EvidenceId>,
) -> EvidenceId {
    let mut current = None;
    let mut legacy = None;
    // Index-backed: call-target and imported occurrence refreshes run per call,
    // so only scan records anchored at this exact node span. In-place migration
    // avoids broad `nose.first_party` duplicates that can bloat evidence output.
    for idx in il.evidence_indices_anchored_at(anchor.span()) {
        let record = &il.evidence[idx as usize];
        if record.anchor == anchor
            && record.kind == kind
            && record.status == EvidenceStatus::Asserted
            && record.provenance.emitter == EvidenceEmitter::Builtin
        {
            if record.provenance.pack_hash == provenance.current.pack_hash {
                current.get_or_insert(idx);
            } else if record.provenance.pack_hash == Some(provenance.legacy_pack_hash) {
                legacy.get_or_insert(idx);
            }
        }
    }
    if let Some(idx) = current.or(legacy) {
        let record = &mut il.evidence[idx as usize];
        record.provenance.pack_hash = provenance.current.pack_hash;
        record.provenance.rule_hash = provenance.current.rule_hash;
        record.dependencies = dependencies;
        return record.id;
    }
    il.find_or_push_builtin_evidence_with_provenance(anchor, kind, provenance.current, dependencies)
}

fn call_target_evidence_provenance(il: &Il) -> CallTargetEvidenceProvenance {
    let (pack_id, producer_id) = language_core_evidence_provenance(il.meta.lang);
    CallTargetEvidenceProvenance {
        current: EvidenceProvenance {
            emitter: EvidenceEmitter::Builtin,
            pack_hash: Some(stable_symbol_hash(pack_id)),
            rule_hash: Some(stable_symbol_hash(producer_id)),
        },
        legacy_pack_hash: stable_symbol_hash(BUILTIN_COMPAT_PACK_ID),
    }
}

mod direct;
mod imported;
mod scope;

use direct::*;
use imported::*;
use scope::*;

#[cfg(test)]
mod tests;

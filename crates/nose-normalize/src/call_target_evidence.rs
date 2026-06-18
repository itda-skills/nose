//! First-party call-target evidence producer.
//!
//! Consumers must not resolve user calls from raw spelling. This pass is the
//! narrow boundary where the lowered file's binding shape is checked and a
//! direct in-file or imported callable target is materialized as evidence.

use nose_il::{
    CallTargetEvidenceKind, EvidenceAnchor, EvidenceEmitter, EvidenceId, EvidenceKind,
    EvidenceProvenance, EvidenceRecord, EvidenceStatus, Il, Interner, LoopKind, NodeId, NodeKind,
    Payload, Symbol, SymbolEvidenceKind, UnitKind,
};
use nose_semantics::{
    imported_occurrence_symbol_dependencies_valid_with_cache, ImportedOccurrenceValidationCache,
    FIRST_PARTY_PACK_ID,
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

pub(crate) fn run(il: &mut Il, interner: &Interner) {
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
        il.find_or_push_first_party_evidence(
            EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
            EvidenceKind::CallTarget(CallTargetEvidenceKind::DirectFunction {
                target_span: il.node(target.root).span,
                name_hash: target.name_hash,
            }),
            FIRST_PARTY_PACK_ID,
            DIRECT_FUNCTION_RULE,
            Vec::new(),
        );
    }
    let mut imported_occurrence_cache = ImportedOccurrenceValidationCache::default();
    for call in calls {
        record_imported_call_target(il, interner, call, &mut imported_occurrence_cache);
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

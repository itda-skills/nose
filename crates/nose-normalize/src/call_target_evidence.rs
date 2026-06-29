//! First-party call-target evidence producer.
//!
//! Consumers must not resolve user calls from raw spelling. This pass is the
//! narrow boundary where the lowered file's binding shape is checked and a
//! direct in-file or imported callable target is materialized as evidence.

use nose_il::{
    stable_symbol_hash, CallTargetEvidenceKind, DomainEvidence, EvidenceAnchor, EvidenceEmitter,
    EvidenceId, EvidenceKind, EvidenceProvenance, EvidenceRecord, EvidenceStatus, Il, Interner,
    LoopKind, NodeId, NodeKind, Payload, SourceFactKind, SourceProtocolKind, Symbol,
    SymbolEvidenceKind, UnitKind,
};
use nose_semantics::{
    imported_occurrence_symbol_dependencies_valid_with_cache, language_core_evidence_provenance,
    ImportedOccurrenceValidationCache, BUILTIN_COMPAT_PACK_ID,
};
use rustc_hash::{FxHashMap, FxHashSet};

const DIRECT_FUNCTION_RULE: &str = "normalize.call_target.direct_function";
const DIRECT_ASYNC_FUNCTION_RETURN_DOMAIN_RULE: &str =
    "normalize.call_target.direct_async_function_return_domain";
const DIRECT_FUNCTION_RETURN_DOMAIN_RULE: &str =
    "normalize.call_target.direct_function_return_domain";
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
        let target_evidence = upsert(
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
        record_direct_async_function_result_domain(
            il,
            call,
            target.root,
            target_evidence,
            provenance,
        );
        record_direct_function_return_promise_like_domain(
            il,
            call,
            target.root,
            target_evidence,
            provenance,
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

fn record_direct_async_function_result_domain(
    il: &mut Il,
    call: NodeId,
    target_root: NodeId,
    target_evidence: EvidenceId,
    provenance: CallTargetEvidenceProvenance,
) -> Option<EvidenceId> {
    let boundary = direct_async_function_protocol_boundary(il, target_root)?;
    let protocol =
        asserted_source_protocol_evidence_id(il, boundary, SourceProtocolKind::AsyncFunction)?;
    Some(upsert(
        il,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        DIRECT_ASYNC_FUNCTION_RETURN_DOMAIN_RULE,
        provenance,
        vec![target_evidence, protocol],
    ))
}

fn direct_async_function_protocol_boundary(il: &Il, target_root: NodeId) -> Option<NodeId> {
    if il.kind(target_root) != NodeKind::Func {
        return None;
    }
    let &boundary = il.children(target_root).last()?;
    (nose_semantics::source_protocol_at_node(il, boundary)
        == Some(SourceProtocolKind::AsyncFunction))
    .then_some(boundary)
}

fn record_direct_function_return_promise_like_domain(
    il: &mut Il,
    call: NodeId,
    target_root: NodeId,
    target_evidence: EvidenceId,
    provenance: CallTargetEvidenceProvenance,
) -> Option<EvidenceId> {
    if direct_async_function_protocol_boundary(il, target_root).is_some() {
        return None;
    }
    let return_domain =
        direct_function_return_domain_evidence_id(il, target_root, DomainEvidence::PromiseLike)?;
    Some(upsert(
        il,
        EvidenceAnchor::node(il.node(call).span, NodeKind::Call),
        EvidenceKind::Domain(DomainEvidence::PromiseLike),
        DIRECT_FUNCTION_RETURN_DOMAIN_RULE,
        provenance,
        vec![target_evidence, return_domain],
    ))
}

fn direct_function_return_domain_evidence_id(
    il: &Il,
    target_root: NodeId,
    domain: DomainEvidence,
) -> Option<EvidenceId> {
    if il.kind(target_root) != NodeKind::Func {
        return None;
    }
    let &body = il.children(target_root).last()?;
    if il.kind(body) == NodeKind::Raw {
        return None;
    }
    let return_expr = single_statement_return_expr(il, body)?;
    asserted_domain_evidence_id_at_node(il, return_expr, domain)
}

fn single_statement_return_expr(il: &Il, body: NodeId) -> Option<NodeId> {
    let ret = if il.kind(body) == NodeKind::Block {
        let kids = il.children(body);
        (kids.len() == 1).then_some(kids[0])?
    } else {
        body
    };
    (il.kind(ret) == NodeKind::Return).then_some(())?;
    il.children(ret).first().copied()
}

fn asserted_domain_evidence_id_at_node(
    il: &Il,
    node: NodeId,
    domain: DomainEvidence,
) -> Option<EvidenceId> {
    let anchor = EvidenceAnchor::node(il.node(node).span, il.kind(node));
    il.evidence_anchored_at(anchor.span()).find_map(|record| {
        (record.anchor == anchor
            && record.kind == EvidenceKind::Domain(domain)
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record))
        .then_some(record.id)
    })
}

fn asserted_source_protocol_evidence_id(
    il: &Il,
    node: NodeId,
    protocol: SourceProtocolKind,
) -> Option<EvidenceId> {
    let span = il.node(node).span;
    il.evidence_anchored_at(span).find_map(|record| {
        (record.kind == EvidenceKind::Source(SourceFactKind::Protocol(protocol))
            && record.status == EvidenceStatus::Asserted
            && il.evidence_dependencies_asserted(record))
        .then_some(record.id)
    })
}

fn upsert(
    il: &mut Il,
    anchor: EvidenceAnchor,
    kind: EvidenceKind,
    _rule: &'static str,
    provenance: CallTargetEvidenceProvenance,
    dependencies: Vec<EvidenceId>,
) -> EvidenceId {
    let existing = asserted_builtin_record_index_with_pack_hash(
        il,
        anchor,
        &kind,
        provenance.current.pack_hash,
    )
    .or_else(|| {
        asserted_builtin_record_index_with_pack_hash(
            il,
            anchor,
            &kind,
            Some(provenance.legacy_pack_hash),
        )
    });
    if let Some(idx) = existing {
        let record = &mut il.evidence[idx as usize];
        record.provenance.pack_hash = provenance.current.pack_hash;
        record.provenance.rule_hash = provenance.current.rule_hash;
        record.dependencies = dependencies;
        return record.id;
    }
    il.find_or_push_builtin_evidence_with_provenance(anchor, kind, provenance.current, dependencies)
}

fn asserted_builtin_record_index_with_pack_hash(
    il: &Il,
    anchor: EvidenceAnchor,
    kind: &EvidenceKind,
    pack_hash: Option<u64>,
) -> Option<u32> {
    il.evidence_indices_anchored_at(anchor.span())
        .into_iter()
        .find(|&idx| {
            let record = &il.evidence[idx as usize];
            record.anchor == anchor
                && &record.kind == kind
                && record.status == EvidenceStatus::Asserted
                && record.provenance.emitter == EvidenceEmitter::Builtin
                && record.provenance.pack_hash == pack_hash
        })
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

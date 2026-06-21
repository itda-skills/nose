//! Corpus-level import proof facts that need more than one lowered file.
//!
//! Frontends lower a static import as an assignment whose RHS carries only module
//! coordinates; `EvidenceKind::Import` records prove those coordinates. Once the
//! whole corpus is available, a sibling module can prove that this binding names a
//! single immutable literal value. In that narrow case we replace the import fact
//! RHS with a cloned literal subtree, so the existing per-file value-graph
//! module-binding seed can reuse its mutation and canonicalization logic.

mod bindings;
mod exports;
mod modules;
mod snapshot;

use bindings::{
    assignment_name, collect_top_level_statements, import_binding_proof, BindingUseIndex,
};
use exports::collect_literal_exports;
use modules::file_module_hashes;
use nose_il::{EvidenceId, Il, Interner, NodeId};
use nose_semantics::semantics;
use snapshot::{
    append_snapshot, prepend_root_statement, record_immutable_literal_export_evidence,
    record_imported_literal_snapshot_evidence, replace_assignment_rhs, snapshot_subtree,
    SubtreeSnapshot,
};

#[derive(Clone)]
struct ExportedBinding {
    file_idx: usize,
    deps: Vec<SubtreeSnapshot>,
    rhs: NodeId,
}

#[derive(Clone)]
struct ImportReplacement {
    stmt: NodeId,
    import_evidence: EvidenceId,
    module_hash: u64,
    exported_hash: u64,
    deps: Vec<SubtreeSnapshot>,
    rhs_snapshot: SubtreeSnapshot,
}

pub(crate) fn resolve_imported_immutable_bindings(files: &mut [Il], interner: &Interner) {
    let contexts: Vec<FileImportContext> = files
        .iter()
        .map(|il| FileImportContext::new(il, interner))
        .collect();
    let exports = collect_literal_exports(files, interner, &contexts);
    if exports.is_empty() {
        return;
    }
    for (&(module_hash, exported_hash), export) in &exports {
        record_immutable_literal_export_evidence(
            &mut files[export.file_idx],
            export.rhs,
            module_hash,
            exported_hash,
            &export.deps,
        );
    }

    let replacements: Vec<Vec<ImportReplacement>> = files
        .iter()
        .enumerate()
        .map(|(file_idx, il)| {
            let context = &contexts[file_idx];
            let Some(top_level) = context.top_level.as_deref() else {
                return Vec::new();
            };
            let Some(binding_uses) = context.binding_uses.as_ref() else {
                return Vec::new();
            };
            top_level
                .iter()
                .copied()
                .filter_map(|stmt| {
                    let local = assignment_name(il, stmt)?;
                    let proof = import_binding_proof(il, stmt)?;
                    let key = (proof.module_hash, proof.exported_hash);
                    let export = exports.get(&key)?;
                    if export.file_idx == file_idx {
                        return None;
                    }
                    if files[export.file_idx].meta.lang != il.meta.lang {
                        return None;
                    }
                    if binding_uses.binding_mutated(il, local, stmt) {
                        return None;
                    }
                    Some(ImportReplacement {
                        stmt,
                        import_evidence: proof.evidence,
                        module_hash: proof.module_hash,
                        exported_hash: proof.exported_hash,
                        deps: export.deps.clone(),
                        rhs_snapshot: snapshot_subtree(&files[export.file_idx], export.rhs),
                    })
                })
                .collect()
        })
        .collect();

    for (file_idx, file_replacements) in replacements.into_iter().enumerate() {
        for replacement in file_replacements {
            let mut snapshot_evidence = Vec::new();
            for dep in replacement.deps {
                let dep = append_snapshot(&mut files[file_idx], &dep);
                snapshot_evidence.extend(dep.evidence);
                prepend_root_statement(&mut files[file_idx], dep.root);
            }
            let rhs = append_snapshot(&mut files[file_idx], &replacement.rhs_snapshot);
            snapshot_evidence.extend(rhs.evidence);
            replace_assignment_rhs(&mut files[file_idx], replacement.stmt, rhs.root);
            record_imported_literal_snapshot_evidence(
                &mut files[file_idx],
                rhs.root,
                replacement.module_hash,
                replacement.exported_hash,
                replacement.import_evidence,
                snapshot_evidence,
            );
        }
    }
}

struct FileImportContext {
    top_level: Option<Vec<NodeId>>,
    module_hashes: Vec<u64>,
    binding_uses: Option<BindingUseIndex>,
}

impl FileImportContext {
    fn new(il: &Il, interner: &Interner) -> Self {
        let module_semantics = semantics(il.meta.lang).modules();
        let participates = module_semantics.sibling_literal_exports()
            || module_semantics.java_class_literal_exports();
        Self {
            top_level: participates.then(|| collect_top_level_statements(il)),
            module_hashes: file_module_hashes(il),
            binding_uses: participates.then(|| BindingUseIndex::new(il, interner)),
        }
    }
}

#[cfg(test)]
mod tests;

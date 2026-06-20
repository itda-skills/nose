//! Python → raw IL lowering.
//!
//! Covers the constructs that matter for clone detection (functions, classes,
//! control flow, calls, operators, literals, comprehensions) and falls back to
//! `Raw` for the rest. A few convergence-friendly choices are made here because
//! they are language-specific: compound assignment is desugared (no core node
//! for it), and ternary lowers to an expression-position `If`. `await e` stays
//! as a source-backed async boundary until a protocol contract proves erasure.

use crate::lower::Lowering;
use nose_il::{
    stable_symbol_hash, Builtin, EvidenceAnchor, EvidenceKind, FileId, HoFKind, Il,
    ImportEvidenceKind, Interner, Lang, LitClass, LoopKind, NodeId, NodeKind, Op, Payload,
    SourceBindingKind, SourceComprehensionKind, SourceFactKind, SourceOperatorKind, Span, Symbol,
    UnitKind,
};
use tree_sitter::Node as TsNode;

mod comprehensions;
mod control;
mod expressions;
mod functions;
mod statements;

use self::{comprehensions::*, control::*, expressions::*, functions::*, statements::*};

pub(super) fn semantic_named_children(node: TsNode) -> Vec<TsNode> {
    Lowering::named_children(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), "comment" | "line_continuation"))
        .collect()
}

pub(super) fn first_semantic_named_child(node: TsNode) -> Option<TsNode> {
    semantic_named_children(node).into_iter().next()
}

pub(crate) fn lower(
    file: FileId,
    path: &str,
    src: &[u8],
    interner: &Interner,
) -> anyhow::Result<Il> {
    crate::lower::lower_file(
        file,
        path,
        src,
        interner,
        crate::lower::grammar::PYTHON,
        || tree_sitter_python::LANGUAGE.into(),
        Lang::Python,
        lower_module,
    )
}

#[cfg(test)]
mod tests;

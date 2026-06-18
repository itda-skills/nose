//! C → raw IL lowering.
//!
//! Convergence-friendly lowering: `x op= y` / `x++` desugar to assignments; `for`,
//! `while`, `do` map to the unified `Loop`; `switch` becomes an `if`/`else if`
//! chain; `function_definition` becomes a function unit. struct/union/enum are
//! data definitions (not unit-ified). `*p`, `&x`, casts peel to the operand.

use crate::lower::{common_bin_op, Lowering};
use nose_il::{
    contains_c_identifier, stable_symbol_hash, Builtin, CTypeTarget, DomainEvidence,
    EvidenceAnchor, EvidenceId, EvidenceKind, FileId, Il, ImportEvidenceKind, Interner, Lang,
    LitClass, NodeId, NodeKind, Op, Payload, SourceCastKind, SourceFactKind, Span,
    TypeEvidenceKind, UnitKind, UnitSubkind,
};
use std::{fs, path::Path};
use tree_sitter::Node as TsNode;

const C_INCLUDE_ALIAS_READ_LIMIT: u64 = 256 * 1024;

mod expressions;
mod includes;
mod items;
mod statements;

use self::{expressions::*, includes::*, items::*, statements::*};

pub(crate) fn lower(
    file: FileId,
    path: &str,
    src: &[u8],
    interner: &Interner,
) -> anyhow::Result<Il> {
    crate::lower::lower_file_with_setup(
        file,
        path,
        src,
        interner,
        crate::lower::grammar::C,
        || tree_sitter_c::LANGUAGE.into(),
        Lang::C,
        |lo| record_c_direct_include_type_aliases(path, src, lo),
        lower_items,
    )
}

#[cfg(test)]
mod tests;

//! Go → raw IL lowering.
//!
//! Convergence-friendly lowering: `x++`/`x op= y` desugar to assignments; the
//! several `for` forms map to the unified `Loop`; Go concurrency/channel
//! constructs stay as source-backed protocol boundaries; `switch` becomes an
//! `if`/`else if` chain; `true`/`false`/`nil` identifiers become literals.

use crate::lower::Lowering;
use nose_il::{
    Builtin, FileId, Il, Interner, Lang, LitClass, LoopKind, NodeId, NodeKind, Op, Payload,
    SourceProtocolKind, Span,
};
use tree_sitter::Node as TsNode;

mod assignments;
mod control;
mod expressions;
mod functions;
mod statements;

use self::{assignments::*, control::*, expressions::*, functions::*, statements::*};

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
        crate::lower::grammar::GO,
        || tree_sitter_go::LANGUAGE.into(),
        Lang::Go,
        lower_source,
    )
}

#[cfg(test)]
mod tests;

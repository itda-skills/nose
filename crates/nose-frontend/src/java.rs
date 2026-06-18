//! Java → raw IL lowering.
//!
//! Convergence-friendly lowering: `x op= y` / `x++` desugar to assignments; the
//! `for`, enhanced-for, `while`, and `do` forms map to the unified `Loop`;
//! `switch` becomes an `if`/`else if` chain; `class`/`interface`/`enum` become
//! class-like units and `method`/`constructor` become function units. Type
//! annotations and generics are not modeled (Java is statically typed).

use crate::lower::{common_bin_op, Lowering};
use nose_il::{
    stable_symbol_hash, EvidenceAnchor, EvidenceKind, FileId, Il, ImportEvidenceKind, Interner,
    Lang, LitClass, LoopKind, NodeId, NodeKind, Op, Payload, RegionKind, SourceCallKind,
    SourceFactKind, SourceGranularity, Span, UnitBodyKind, UnitDomain, UnitDomains,
    UnitEvidenceFlag, UnitKind, UnitOrigin, UnitSubkind,
};
use nose_semantics::{
    library_java_collection_constructor_contract, LibraryApiCalleeContract,
    LibraryCollectionFactoryResult,
};
use tree_sitter::Node as TsNode;

mod constructors;
mod control;
mod expressions;
mod items;

use self::{constructors::*, control::*, expressions::*, items::*};

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
        crate::lower::grammar::JAVA,
        || tree_sitter_java::LANGUAGE.into(),
        Lang::Java,
        lower_items,
    )
}

#[cfg(test)]
mod tests;

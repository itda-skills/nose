//! C# (.NET) → raw IL lowering.
//!
//! C# is lowered as a statically-typed C-family frontend, closely mirroring Java:
//! `class`/`struct`/`record`/`interface`/`enum` become class-like units and
//! `method`/`constructor`/`local function`/property accessors become function
//! units; `if`/`for`/`foreach`/`while`/`do` map to the shared canonical control
//! shapes; `switch` (statement and expression) and `is` lower their patterns as
//! predicates in an `if`/else-if chain (the Rust/Swift `match` discipline, with
//! type tests erased like Java's `instanceof`); `x op= y` / `x++` desugar to
//! assignments; `a ?? b` / `a ??= b` lower to the `ValueOrDefault` builtin (as
//! Swift's `??`); expression-bodied members (`=> expr`) lower to `{ return expr;
//! }` so they converge with block bodies; `#if`-guarded members/statements lower
//! in place (both branches are real code). `await` and `yield` stay source-backed
//! protocol boundaries. Type annotations, generics, and attributes carry no
//! behavior and are erased. LINQ query syntax desugars to the spec's
//! method-syntax chain (`from`/`where`/`orderby`/`select`/`group by`), including
//! the transparent-identifier translation for `let`/`join`/a second `from` and
//! `into` continuations; query shapes the translation cannot prove stay honest
//! `Raw` gaps.

use crate::lower::{common_bin_op, Lowering};
use nose_il::{
    Builtin, FileId, Il, Interner, Lang, LitClass, LoopKind, NodeId, NodeKind, Op, Payload,
    RegionKind, SourceCallKind, SourceFactKind, SourceGranularity, Span, Symbol, UnitBodyKind,
    UnitDomain, UnitDomains, UnitEvidenceFlag, UnitKind, UnitOrigin, UnitSubkind,
};
use tree_sitter::Node as TsNode;

mod control;
mod expressions;
mod items;
mod linq;
mod objects;
mod patterns;

use self::{control::*, expressions::*, items::*, linq::*, objects::*, patterns::*};

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
        crate::lower::grammar::CSHARP,
        || tree_sitter_c_sharp::LANGUAGE.into(),
        Lang::CSharp,
        lower_items,
    )
}

//! `nose review` — flag clone families edited inconsistently in a change set.
//!
//! Given a git ref (`--base`), `review` detects clone families **at that base** (where every
//! copy still matches), finds which lines the diff changed, and flags every family where
//! *some* copies were edited but *siblings were not* — a likely un-propagated edit ("you
//! changed X; its clone Y was not updated"). This is the divergent-edit (Kim *Inconsistent
//! Change*) predicate applied to one diff.
//!
//! Detection runs at the base, not the working tree, on purpose: an edit can push a copy out
//! of its clone family (a fix changes its shape), so it would be invisible in the current
//! tree. At the base the family is still intact, and the diff tells us which member moved.
//!
//! The structural signal is a candidate surfacer, not a proof: review the flagged siblings
//! yourself.

mod detect;
mod git;
mod output;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{ReportFormat, ScanMode};
use nose_detect::{EnclosingUnit, FragmentKind, Loc, RefactorFamily};

pub(crate) use detect::{cmd_review, detect_divergences, divergences_fire};
pub(crate) use output::divergence_items_json;

pub(crate) fn review_sarif(
    flagged: &[Divergence],
    top: Option<usize>,
    top_zero_spelling: &str,
) -> Result<String> {
    output::review_sarif(flagged, top, top_zero_spelling)
}

pub(crate) struct ReviewArgs {
    pub paths: Vec<PathBuf>,
    pub base: String,
    pub mode: Vec<ScanMode>,
    pub min_size: Option<usize>,
    pub min_lines: Option<u32>,
    pub exclude: Vec<String>,
    pub config: Option<PathBuf>,
    pub ignore_file: Option<PathBuf>,
    pub format: ReportFormat,
    pub top: Option<usize>,
    pub fail: bool,
    pub fail_on: ReviewFailOn,
}

/// What `--fail` fires on. The default is the #245 conservative tier: only
/// findings where the diff PROVABLY touches lines a changed member shares with
/// its un-updated sibling (§BR measured span-overlap firing at 33% of merged
/// PRs with ~4% top-1 precision — a gate that cries wolf gets disabled).
#[derive(Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub(crate) enum ReviewFailOn {
    /// Fire only on shared-logic-touching findings (the conservative gate).
    #[default]
    SharedLogic,
    /// Fire on any flagged finding (the pre-#245 span-overlap behavior).
    Any,
}

/// A flagged family: a clone whose copies were edited apart in this change set. Locations
/// are repo-relative (the report navigates the real working tree). `pub(crate)` so the
/// `nose query <paths> base=<ref>` view renders the same findings (the review/query
/// unification): query reuses this exact detection, preserving §BV fire precision.
pub(crate) struct Divergence {
    pub(crate) family_id: String,
    pub(crate) similarity: f64,
    pub(crate) hazard: f64,
    pub(crate) review_priority: u8,
    pub(crate) complexity: usize,
    /// Family scope: `prod` / `test` / `mixed` (test scaffolding fires differently).
    pub(crate) scope: &'static str,
    /// The family's equivalence-witness kind (`exact-value-graph`,
    /// `copy-paste-run`, `shared-sub-dag`, `structural-similarity`).
    pub(crate) witness_kind: Option<&'static str>,
    /// The #245 conservative gate verdict: some changed member PROVABLY touches
    /// lines it shares with an un-updated sibling. `--fail` fires only on these;
    /// `--fail-on any` restores span-overlap firing.
    pub(crate) fire_eligible: bool,
    /// The near family's graded equivalence witness (#315), when present — evidence
    /// for the consumer to judge a fire: a clean `equal_modulo_holes` family is a
    /// strong missed-propagation candidate, while `referent_mismatches` /
    /// `decorator-differs` mark a family whose copies are not really the same logic
    /// (a likely false fire). It does NOT gate `fire_eligible` (that would risk
    /// dropping a genuine shared-body propagation).
    pub(crate) graded: Option<nose_detect::GradedWitness>,
    /// Members whose base span was changed by the diff (the edit landed here).
    pub(crate) changed: Vec<Site>,
    /// Sibling members the change did *not* touch (where it may be missing).
    pub(crate) not_updated: Vec<Site>,
}

#[derive(Clone)]
pub(crate) struct Site {
    pub(crate) file: String,
    pub(crate) name: Option<String>,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) lang: String,
    pub(crate) kind: nose_il::UnitKind,
    pub(crate) span_lines: u32,
    pub(crate) span_tokens: usize,
    pub(crate) is_fragment: bool,
    pub(crate) fragment_kind: Option<FragmentKind>,
    pub(crate) reason_code: Option<&'static str>,
    pub(crate) enclosing_unit: Option<EnclosingUnit>,
    /// For CHANGED sites: does the diff touch lines this member shares with an
    /// un-updated sibling? `Some(false)` = the edit stayed inside this member's
    /// varying spots; `None` = unprovable (unreadable source / capped diff) or a
    /// not-updated site.
    pub(crate) touches_shared: Option<bool>,
}

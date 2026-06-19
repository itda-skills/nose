use crate::legacy_prelude::*;

/// The default-surface families to render, in display order: overlapping slices folded
/// out, then production-scope findings ahead of test-scope, then truncated to `--top`.
///
/// §2c default-surface honesty: test duplication is a real smell (never dropped, still
/// ranked, in `--format json`, one `--scope test` away), but production leads the
/// bare-default screen so it is not buried — test scope was measured at 60–76% of the
/// default head ([default-surface-noise-audit](../../../docs/default-surface-noise-audit-2026-06-14.md)).
/// The reorder is stable (extractability rank preserved within each scope) and only runs
/// when no `--scope` filter has already narrowed the set to one scope.
pub(crate) fn select_shown_reportable<'a>(
    reportable: &[&'a nose_detect::RefactorFamily],
    opportunities: &OpportunityGroups,
    scope: ScopeFilter,
    limit: usize,
) -> Vec<&'a nose_detect::RefactorFamily> {
    let mut shown: Vec<_> = reportable
        .iter()
        .filter(|f| !opportunities.is_slice(f))
        .copied()
        .collect();
    if matches!(scope, ScopeFilter::All) {
        shown.sort_by_key(|f| f.scope == "test");
    }
    shown.truncate(limit);
    shown
}

/// Render one ranked family entry of the human report: headline, hint, folded
/// opportunity slices, abstraction witness, the (capped) site list, and optional
/// diff/proposal views.
fn print_family_entry(
    i: usize,
    f: &nose_detect::RefactorFamily,
    opportunities: &OpportunityGroups,
    diff: bool,
    proposal: bool,
) {
    // Every site is listed (you can't act on a clone you can't see); only pathological
    // fanout is capped, with a pointer to the full machine-readable list.
    const SITE_CAP: usize = 30;
    println!(
        "\n#{}  id {} · {}",
        i + 1,
        baseline::family_id(f),
        family_summary(f)
    );
    println!("    → {}", family_hint(f));
    if let Some(slices) = opportunities.slices(f) {
        let listed = slices
            .iter()
            .take(4)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let more = if slices.len() > 4 { ", …" } else { "" };
        let (n, noun, verb) = match slices.len() {
            1 => (1, "family", "folds"),
            n => (n, "families", "fold"),
        };
        println!(
            "    ↳ {n} overlapping slice {noun} {verb} into this entry (id{} {listed}{more})",
            if n == 1 { ":" } else { "s:" }
        );
    }
    if let Some(witness) = &f.abstraction_witness {
        println!("    witness {}", abstraction_witness_summary(witness));
    }
    for l in f.locations.iter().take(SITE_CAP) {
        let name = l
            .name
            .as_deref()
            .map(|n| format!("  {n}"))
            .unwrap_or_default();
        // For a partial / sub-DAG clone, point at where the shared computation sits here.
        let shared = match l.shared_subdag {
            Some((s, e)) if (s, e) != (l.start_line, l.end_line) => {
                format!("  (shared computation: lines {s}-{e})")
            }
            _ => String::new(),
        };
        println!(
            "    {}:{}-{}{}{}",
            l.file, l.start_line, l.end_line, name, shared
        );
    }
    if f.locations.len() > SITE_CAP {
        println!(
            "    … and {} more sites (--format json lists every one)",
            f.locations.len() - SITE_CAP
        );
    }
    if diff && f.locations.len() >= 2 {
        print_member_diff(&f.locations[0], &f.locations[1]);
    }
    if proposal && f.locations.len() >= 2 {
        print_member_proposal(&f.locations, proposal_action_label(f));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn print_refactor_human(
    all: &[&nose_detect::RefactorFamily],
    shown: &[&nose_detect::RefactorFamily],
    sort: SortKey,
    mode: ScanChannels,
    diff: bool,
    proposal: bool,
    omitted_note: Option<&str>,
    opportunities: &OpportunityGroups,
) {
    if all.is_empty() {
        println!(
            "no {} found — nothing above the reporting thresholds",
            mode.report_label(0)
        );
        if let Some(note) = omitted_note {
            println!("{note}");
        }
        return;
    }
    println!(
        "{} {}, ranked by {}  ·  ~{} duplicated lines  (showing {})",
        all.len(),
        mode.report_label(all.len()),
        sort_name(sort),
        total_dup_lines_refs(all),
        shown.len()
    );
    if let Some(note) = omitted_note {
        println!("{note}");
    }
    // Production findings lead; a single separator marks where test-scope duplication
    // (already sorted beneath, never dropped) begins. Skipped when the list is all one
    // scope (e.g. under `--scope test`/`prod`).
    let any_nontest = shown.iter().any(|f| f.scope != "test");
    let mut test_header_shown = false;
    for (i, f) in shown.iter().enumerate() {
        if any_nontest && !test_header_shown && f.scope == "test" {
            let n = shown.iter().filter(|g| g.scope == "test").count();
            println!(
                "\n── {n} test-scope {} ranked beneath production · --scope test to focus, --scope prod to hide ──",
                if n == 1 { "family" } else { "families" }
            );
            test_header_shown = true;
        }
        print_family_entry(i, f, opportunities, diff, proposal);
    }
    // Test-scope duplication is a real smell (never dropped), but production leads the
    // default screen — so when `--top` cut some test families, say so rather than let
    // them vanish silently. Skipped under `--scope test`/`prod` (no production above).
    let test_total = all
        .iter()
        .filter(|f| f.scope == "test" && !opportunities.is_slice(f))
        .count();
    let test_shown = shown.iter().filter(|f| f.scope == "test").count();
    if any_nontest && test_total > test_shown {
        let more = test_total - test_shown;
        println!(
            "\n+{more} more test-scope {} ranked beneath production (--scope test to focus, --top 0 for all)",
            if more == 1 { "family" } else { "families" }
        );
    }
    // Discoverability: the report's natural next steps, shown once a real report
    // exists and only when no extra view was already requested.
    if !shown.is_empty() && !diff && !proposal {
        println!(
            "\nhint: `--show diff` shows what differs inside each family · `--show proposal` \
             drafts the extraction · `--top 0` lists every family"
        );
    }
}

/// Print a unified diff between two family members' source — the few lines that
/// differ are what a reviewer needs to judge how cleanly the copies can be merged.
pub(crate) fn print_member_diff(a: &nose_detect::Loc, b: &nose_detect::Loc) {
    let (Some(la), Some(lb)) = (
        read_lines(&a.file, a.start_line, a.end_line),
        read_lines(&b.file, b.start_line, b.end_line),
    ) else {
        return;
    };
    println!(
        "     diff  {}:{}-{}  vs  {}:{}-{}",
        a.file, a.start_line, a.end_line, b.file, b.start_line, b.end_line
    );
    let ar: Vec<&str> = la.iter().map(String::as_str).collect();
    let br: Vec<&str> = lb.iter().map(String::as_str).collect();
    for (tag, line) in line_diff(&ar, &br) {
        println!("       {tag} {line}");
    }
}

/// Synthesize an *extraction proposal* aligned across **all** the family's copies (#360):
/// the lines invariant across *every* copy become the body of the shared helper, and each
/// maximal run that varies in *any* copy collapses to a `⟨param N⟩` placeholder — line-
/// granularity anti-unification, N-way. Turns "these are similar" into "extract this,
/// parameterize these N spots", and — unlike a pairwise skeleton — the result is safe to
/// apply to *every* member, not just the two largest, so it never claims a shared line a
/// third copy actually diverges on. Bounded to one family, paid only on `--show proposal`.
pub(crate) fn print_member_proposal(locations: &[nose_detect::Loc], action: &str) {
    // Read every copy's source; align across all of them. A copy whose source can't be
    // read is dropped, and the count reflects the copies actually aligned.
    let members: Vec<Vec<String>> = locations
        .iter()
        .filter_map(|l| read_lines(&l.file, l.start_line, l.end_line))
        .collect();
    if members.len() < 2 {
        return;
    }
    let (skeleton, shared, params) = anti_unify_all(&members);
    let copies = members.len();
    println!("     proposal  {action} · {shared} shared lines · {params} parameter(s) vary (across all {copies} copies)");
    for line in skeleton.iter().take(40) {
        println!("       │ {line}");
    }
}

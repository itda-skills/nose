use super::*;

/// Everything the format arms need to render one scan's report.
pub(super) struct ScanReportView<'a> {
    pub(super) scope: &'a ScanScope,
    pub(super) settings: &'a ScanSettings,
    pub(super) reinvented: &'a [nose_detect::ReinventedHelper],
    pub(super) families: &'a [nose_detect::RefactorFamily],
    pub(super) shown: &'a [&'a nose_detect::RefactorFamily],
    pub(super) reportable: &'a [&'a nose_detect::RefactorFamily],
    pub(super) shown_reportable: &'a [&'a nose_detect::RefactorFamily],
    pub(super) baseline: Option<&'a BaselineComparison>,
    pub(super) ignored_families: &'a [IgnoredFamily],
    pub(super) omitted_note: Option<&'a str>,
    pub(super) overrides: &'a SurfaceOverrides,
    pub(super) opportunities: &'a OpportunityGroups,
}

pub(super) fn render_scan_report(args: &ScanArgs, view: &ScanReportView) -> Result<()> {
    let settings = view.settings;
    match args.format {
        ReportFormat::Json => {
            let json = ScanJsonReport::new(ScanJsonInput {
                scope: view.scope,
                reinvented: view.reinvented,
                sort: settings.sort,
                top: settings.top,
                families: view.families,
                shown: view.shown,
                baseline: view.baseline,
                ignore_set: settings.ignore_set.as_ref(),
                ignored_families: view.ignored_families,
                semantic_packs: &settings.semantic_packs,
                overrides: view.overrides,
                opportunities: view.opportunities,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ReportFormat::Markdown => {
            // Scope line first — tells the reader what was actually scanned (so a small
            // count from `.gitignore`/`--exclude` pruning is visible, not a silent gap).
            println!("{}\n", view.scope.summary());
            if let Some(line) = semantic_pack_summary_line(&settings.semantic_packs) {
                println!("{line}\n");
            }
            print_refactor_markdown(
                view.reportable,
                view.shown_reportable,
                settings.channels,
                view.baseline,
                settings.ignore_set.as_ref(),
                view.ignored_families.len(),
                view.omitted_note,
            );
        }
        ReportFormat::Human => {
            println!("{}", view.scope.summary());
            if let Some(line) = semantic_pack_summary_line(&settings.semantic_packs) {
                println!("{line}");
            }
            if let Some(comparison) = view.baseline {
                println!("{}", comparison.summary.line());
            }
            if let Some(ignore_set) = &settings.ignore_set {
                println!("{}", ignore_set.summary(view.ignored_families.len()).line());
            }
            print_refactor_human(
                view.reportable,
                view.shown_reportable,
                settings.sort,
                settings.channels,
                args.show.contains(&ShowView::Diff),
                args.show.contains(&ShowView::Proposal),
                view.omitted_note,
                view.opportunities,
            );
            print_reinvented_helpers(view.reinvented, args.show.contains(&ShowView::Reinvented));
        }
        ReportFormat::Sarif => println!(
            "{}",
            refactor_sarif(view.shown_reportable, view.reportable.len())?
        ),
    }
    Ok(())
}

/// How many reinvented-helper findings the bare default surface lists before collapsing
/// the rest into a "+N more" line — kept short so the section stays a focused aid.
const REINVENTED_DEFAULT_LIMIT: usize = 5;

/// The reinvented-helper section of the human report. Promoted to the bare-default
/// surface after a field audit (docs/reinvented-helper-audit-2026-06-13.md): of 17 corpus
/// findings, ~13/14 non-test ones were genuine value-duplications and ~10 directly
/// actionable, while the non-actionable noise was dominated by TEST-container findings
/// (a test asserts the helper's value as a literal — calling it would be circular). So
/// the default lists the non-test findings (top by weight) and excludes test-container
/// ones (§2b decidable classification); `--show reinvented` lists EVERY finding.
fn print_reinvented_helpers(reinvented: &[nose_detect::ReinventedHelper], show: bool) {
    if reinvented.is_empty() {
        return;
    }
    let print_one = |r: &nose_detect::ReinventedHelper| {
        let helper_name = r.helper_name.as_deref().unwrap_or("-");
        let container_name = r.container_name.as_deref().unwrap_or("-");
        let approx = if r.site_approximate { " ~approx" } else { "" };
        println!(
            "  {}:{}-{}  {}  reimplements  {}:{}-{}  {}  (lines {}-{}{}, ~{} value nodes)",
            r.container_file,
            r.container_start_line,
            r.container_end_line,
            container_name,
            r.helper_file,
            r.helper_start_line,
            r.helper_end_line,
            helper_name,
            r.site_start_line,
            r.site_end_line,
            approx,
            r.weight,
        );
    };
    if show {
        println!(
            "\nreinvented helpers — call the existing helper instead (exact matches, experimental):"
        );
        for r in reinvented {
            print_one(r);
        }
        return;
    }
    // Default surface: non-test findings only, top by weight (already sorted).
    let shown: Vec<&nose_detect::ReinventedHelper> =
        reinvented.iter().filter(|r| !r.container_in_test).collect();
    let test_count = reinvented.len() - shown.len();
    if shown.is_empty() {
        println!(
            "\n{test_count} reinvented-helper finding{} in test code · `--show reinvented` lists them",
            if test_count == 1 { "" } else { "s" },
        );
        return;
    }
    println!("\nreinvented helpers — code that reimplements an existing helper; call it instead:");
    for r in shown.iter().take(REINVENTED_DEFAULT_LIMIT) {
        print_one(r);
    }
    let hidden = shown.len().saturating_sub(REINVENTED_DEFAULT_LIMIT);
    if hidden > 0 || test_count > 0 {
        let mut parts = Vec::new();
        if hidden > 0 {
            parts.push(format!("{hidden} more"));
        }
        if test_count > 0 {
            parts.push(format!("{test_count} in test code"));
        }
        println!("  … {} · `--show reinvented` lists all", parts.join(", "));
    }
}

pub(super) fn enforce_scan_fail_on(
    args: &ScanArgs,
    channels: ScanChannels,
    reportable: &[&nose_detect::RefactorFamily],
    baseline_comparison: Option<&BaselineComparison>,
) {
    if let (true, Some(comparison)) = (
        matches!(args.fail_on, Some(FailOn::New)) && !reportable.is_empty(),
        baseline_comparison,
    ) {
        let mut new_families = 0usize;
        let mut changed_families = 0usize;
        for family in reportable {
            match comparison.statuses.get(&baseline::family_key(family)) {
                Some(BaselineStatus::Changed) => changed_families += 1,
                Some(BaselineStatus::New) => new_families += 1,
                None => {}
            }
        }
        let reportable_families = new_families + changed_families;
        eprintln!(
            "\nnose: {} new and {} changed {} found (--fail-on new)",
            new_families,
            changed_families,
            channels.report_label(reportable_families)
        );
        std::process::exit(1);
    }
    if matches!(args.fail_on, Some(FailOn::Any)) && !reportable.is_empty() {
        eprintln!(
            "\nnose: {} {} found (--fail-on any)",
            reportable.len(),
            channels.report_label(reportable.len())
        );
        std::process::exit(1);
    }
}

pub(super) fn print_hotspots_refs(families: &[&nose_detect::RefactorFamily]) {
    use std::collections::{HashMap, HashSet};
    // directory -> (lines residing here that are in a family, distinct families touching it)
    let mut lines: HashMap<&str, u32> = HashMap::new();
    let mut fams: HashMap<&str, HashSet<usize>> = HashMap::new();
    for (fi, f) in families.iter().enumerate() {
        for l in &f.locations {
            let m = std::path::Path::new(&l.file)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            *lines.entry(m).or_insert(0) += l.end_line.saturating_sub(l.start_line) + 1;
            fams.entry(m).or_default().insert(fi);
        }
    }
    if lines.is_empty() {
        return;
    }
    let mut ranked: Vec<(&str, u32, usize)> = lines
        .iter()
        .map(|(m, d)| (*m, *d, fams.get(m).map_or(0, |s| s.len())))
        .collect();
    // Most duplicated lines first; ties by family count, then path for determinism.
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(b.0)));
    println!("\nduplication hotspots (directories by lines that sit in a clone family):");
    for (m, dup, n) in ranked.iter().take(10) {
        let dir = if m.is_empty() { "." } else { m };
        println!("  ~{dup:>5} dup lines · {n:>3} families  {dir}");
    }
}

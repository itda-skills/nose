use super::query_dashboard::render_query_dashboard;
use super::query_model::*;
use super::query_open::render_query_family;
use super::query_views::*;
use super::*;

/// The flat family set a report format (`--format markdown`/`sarif`) emits for a query: the
/// single addressed family for `at=`/`id=`, otherwise the same default-surface (or `all`/
/// `surface=`-widened, slice-folded, filtered) selection the list view shows. Report formats
/// are non-interactive, so they collapse the dashboard/group views to this set.
fn query_selection<'a>(
    families: &'a [nose_detect::RefactorFamily],
    ov: &SurfaceOverrides,
    opp: &OpportunityGroups,
    q: &Query,
    path_arg: &str,
    since: Option<&BaselineComparison>,
) -> Result<Vec<&'a nose_detect::RefactorFamily>> {
    if let Some(at) = &q.at {
        let idv = baseline::family_id(family_at(families, at, path_arg)?);
        return Ok(families
            .iter()
            .filter(|f| baseline::family_id(f) == idv)
            .collect());
    }
    if let Some(idv) = &q.id {
        return Ok(families
            .iter()
            .filter(|f| baseline::family_id(f).starts_with(idv.as_str()))
            .collect());
    }
    let widen = q.all || q.filters.iter().any(|flt| flt.field == "surface");
    Ok(families
        .iter()
        .filter(|f| {
            (widen || is_default_surface(f, ov))
                && !opp.is_slice(f)
                && q.filters
                    .iter()
                    .all(|flt| family_matches(f, ov, flt, since))
        })
        .collect())
}

/// The `base=<git-ref>` view: divergent edits (a clone changed in one copy but not its
/// siblings) detected at the ref by the `nose review` pipeline, surfaced under query. Reuses
/// review's detection verbatim — so the §BV fire precision is preserved by construction — and
/// gates with the same conservative shared-logic policy.
fn run_query_base(args: &ScanArgs, base_ref: &str, q: &Query, path_arg: &str) -> Result<()> {
    validate_base_query(q, args)?;
    // `base=` gates on a diff against a ref, not a saved baseline — `--fail-on new` (which
    // needs `--baseline`) is meaningless here.
    if matches!(args.fail_on, Some(FailOn::New)) {
        anyhow::bail!(
            "`base=` gates on a diff, not a baseline — use `--fail-on any` (fires on a proven divergence)"
        );
    }
    let review_args = review::ReviewArgs {
        paths: args.paths.clone(),
        base: base_ref.to_string(),
        mode: args.mode.clone(),
        min_size: args.min_size,
        min_lines: args.min_lines,
        exclude: args.exclude.clone(),
        config: args.config.clone(),
        ignore_file: args.ignore_file.clone(),
        format: args.format,
        top: q.top,
        fail: false,
        fail_on: review::ReviewFailOn::default(),
    };
    let (flagged, changed_files) = review::detect_divergences(&review_args)?.unwrap_or_default();
    match args.format {
        ReportFormat::Json => {
            render_query_base(&flagged, changed_files, base_ref, path_arg, q.top, true)
        }
        ReportFormat::Sarif => println!("{}", review::review_sarif(&flagged, q.top, "top=0")?),
        _ => render_query_base(&flagged, changed_files, base_ref, path_arg, q.top, false),
    }
    // The gate fires on the §BV conservative policy (a proven shared-logic divergence) — the
    // only sane CI default — reused verbatim from review so the fire decision is identical.
    if matches!(args.fail_on, Some(FailOn::Any))
        && review::divergences_fire(&flagged, review::ReviewFailOn::SharedLogic)
    {
        std::process::exit(1);
    }
    Ok(())
}

fn validate_base_query(q: &Query, args: &ScanArgs) -> Result<()> {
    let unsupported_terms = q.reinvented
        || q.all
        || q.id_full
        || q.group.is_some()
        || q.id.is_some()
        || q.at.is_some()
        || q.since.is_some()
        || q.sort.is_some()
        || !q.filters.is_empty();
    if unsupported_terms {
        anyhow::bail!(
            "`base=` is its own divergent-edit view; combine it only with `top=N`, detection flags, `--format`, or `--fail-on any`"
        );
    }
    let mut unsupported_flags = Vec::new();
    if args.min_members.is_some() {
        unsupported_flags.push("--min-members");
    }
    if args.min_value.is_some() {
        unsupported_flags.push("--min-value");
    }
    if args.cache_dir.is_some() {
        unsupported_flags.push("--cache-dir");
    }
    if !args.semantic_pack.is_empty() {
        unsupported_flags.push("--semantic-pack");
    }
    if args.baseline.is_some() {
        unsupported_flags.push("--baseline");
    }
    if args.write_baseline {
        unsupported_flags.push("--write-baseline");
    }
    if !unsupported_flags.is_empty() {
        anyhow::bail!(
            "`base=` does not support {}; combine it only with `top=N`, detection flags, `--format`, or `--fail-on any`",
            unsupported_flags.join(", ")
        );
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // a flat command dispatcher: dashboard / at= / id= / group / list
pub(super) fn run_query_cmd(cmd: Cmd) -> Result<()> {
    let Cmd::Query {
        path,
        terms,
        format,
        mode,
        min_size,
        min_lines,
        min_value,
        min_members,
        exclude,
        cache_dir,
        ignore_file,
        semantic_pack,
        config,
        fail_on,
        baseline,
        write_baseline,
    } = cmd
    else {
        unreachable!("run_query_cmd requires Cmd::Query")
    };
    require_paths_exist(std::slice::from_ref(&path))?;
    let q = parse_query(&terms)?;
    // The path as the user typed it — every suggested next-command echoes it so the links
    // are runnable verbatim (the surface takes the path positionally).
    let path_arg = path.to_string_lossy().into_owned();
    // Build the full dataset (all families); query filters/views work in-memory over it.
    let args = ScanArgs {
        paths: vec![path],
        top: Some(0),
        min_members,
        min_value,
        sort: None,
        config,
        mode,
        show: vec![],
        cache_dir,
        fail_on,
        baseline,
        ignore_file,
        semantic_pack,
        write_baseline,
        format,
        exclude,
        min_size,
        min_lines,
        scope: ScopeFilter::All,
    };
    if matches!(args.fail_on, Some(FailOn::New)) && args.baseline.is_none() {
        anyhow::bail!(
            "--fail-on new requires --baseline (it gates on families new vs the baseline)"
        );
    }
    // `base=<git-ref>` is the divergent-edit view (the `nose review` pipeline): it detects at
    // the ref, not the working tree, so it short-circuits the working-tree dataset entirely.
    if let Some(base_ref) = &q.base {
        return run_query_base(&args, base_ref, &q, &path_arg);
    }
    let refs = paths_as_refs(&args.paths);
    let mut dataset = build_scan_dataset(&args, &refs)?;
    // query is a full surface (#375): prepare the family set exactly as scan does before
    // rendering — write/apply a baseline, drop ignored families — then run the same CI gate.
    if args.write_baseline {
        return write_scan_baseline(&args, &dataset.families);
    }
    let baseline_comparison = apply_scan_baseline(&args, &mut dataset.families)?;
    let ignore_set = dataset.settings.ignore_set.take();
    let (active, _ignored) =
        partition_ignored(std::mem::take(&mut dataset.families), ignore_set.as_ref());
    dataset.families = active;
    let overrides =
        classify_surface_overrides(&mut dataset.families, &refs, &dataset.settings.exclude);
    // `spotclass` reads the #315 graded witness, which `build_scan_dataset` does not compute
    // (re-deriving it is the dominant scan cost — netty: ~2.8s of a ~4.6s near scan). Run that
    // enrichment only when the query actually filters or groups by `spotclass`, and before the
    // filter so the class exists to match on; the common query path stays free of it.
    let needs_spotclass = q.group.as_deref() == Some("spotclass")
        || q.filters.iter().any(|flt| flt.field == "spotclass");
    if needs_spotclass {
        enrich_graded_witnesses(&mut dataset.families, &dataset.opts);
    }
    // `since=<snapshot>` annotates each family with a temporal `status` (new/changed/unchanged)
    // — an exploration lens over the baseline machinery, not a gate (it hides nothing). The
    // `status` field is unresolvable without it, so reject the combination loudly.
    let uses_status =
        q.group.as_deref() == Some("status") || q.filters.iter().any(|flt| flt.field == "status");
    if uses_status && q.since.is_none() {
        anyhow::bail!("`status` needs a snapshot — add `since=<baseline-file>` (write one with `--write-baseline`)");
    }
    let since_cmp = match &q.since {
        Some(p) => Some(compare_since(p, &dataset.families)?),
        None => None,
    };
    let since = since_cmp.as_ref();
    if let Some(sk) = q.sort {
        dataset.families.sort_by(|a, b| {
            sk.score(b)
                .total_cmp(&sk.score(a))
                .then(b.value.total_cmp(&a.value))
                .then_with(|| family_anchor(a).cmp(&family_anchor(b)))
        });
    }
    // Fold overlapping slice families under their best-ranked primary (scan's #263/#264
    // grouping) so query doesn't double-count or under-report the same region vs scan.
    let default_fams: Vec<&nose_detect::RefactorFamily> = dataset
        .families
        .iter()
        .filter(|f| is_default_surface(f, &overrides))
        .collect();
    let opp = OpportunityGroups::from_ranked(&default_fams);
    // Markdown is a query domain: a docs-only tree (no code, but real `.md` near-dups) is not
    // "nothing found". The dashboard sets this so the empty-corpus warning below stays accurate.
    let mut markdown_found = false;
    match format {
        // Report formats (for PRs / code-scanning, like scan's): non-interactive, so they
        // ignore dashboard/group navigation and just render the family set the query selects,
        // reusing scan's own formatters (#374 + scan parity).
        ReportFormat::Markdown | ReportFormat::Sarif => {
            let selected =
                query_selection(&dataset.families, &overrides, &opp, &q, &path_arg, since)?;
            let top = query_row_limit(q.top);
            let shown: Vec<&nose_detect::RefactorFamily> =
                selected.iter().take(top).copied().collect();
            if matches!(format, ReportFormat::Sarif) {
                println!("{}", refactor_sarif(&shown, selected.len())?);
            } else {
                print_refactor_markdown(
                    &selected,
                    &shown,
                    dataset.settings.channels,
                    baseline_comparison.as_ref(),
                    None,
                    0,
                    None,
                );
                // `id=<fam>` is a single-family drilldown: render the extraction skeleton
                // (and, on `full`, the representative diff) so markdown composes with
                // `id=`/`full` the way the human/JSON views do (#422). Bulk reports stay a
                // compact location list — the skeleton is paid only on drilldown.
                if q.id.is_some() {
                    for f in &shown {
                        if f.locations.len() >= 2 {
                            markdown_member_proposal(&f.locations);
                            if q.id_full {
                                markdown_member_diff(&f.locations[0], &f.locations[1]);
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let json = matches!(format, ReportFormat::Json);
            if q.reinvented {
                // The reinvented-helper channel as a query view — code that reimplements an
                // existing helper (the action is "call it"). Complements (does not duplicate)
                // `shape=call-existing-helper`: those are the cases the family clusterer caught,
                // these are the ones it did not.
                render_query_reinvented(&dataset.reinvented, &path_arg, q.top, json);
            } else if terms.is_empty() {
                let reinvented_prod = dataset
                    .reinvented
                    .iter()
                    .filter(|r| !r.container_in_test)
                    .count();
                // Markdown near-duplicate prose is a query domain (converged from the former
                // `nose markdown` command): the dashboard reports `.md` near-dup families
                // alongside code clones, via the separate `nose-markdown` engine.
                let md = markdown::detect_under(&args.paths[0], &dataset.settings.exclude);
                markdown_found = !md.is_empty();
                render_query_dashboard(
                    &dataset.families,
                    &overrides,
                    &opp,
                    &dataset.scope,
                    &path_arg,
                    reinvented_prod,
                    json,
                    since,
                    &md,
                );
            } else if let Some(at) = &q.at {
                // `at=file:line` → the family whose member span covers that location (stable
                // across edits, unlike the span-derived id). Resolve to an `id=` open.
                let idv = baseline::family_id(family_at(&dataset.families, at, &path_arg)?);
                render_query_family(
                    &dataset.families,
                    &overrides,
                    &opp,
                    &idv,
                    q.id_full,
                    &path_arg,
                    json,
                    since,
                );
            } else if let Some(idv) = &q.id {
                render_query_family(
                    &dataset.families,
                    &overrides,
                    &opp,
                    idv,
                    q.id_full,
                    &path_arg,
                    json,
                    since,
                );
            } else {
                // Default to the curated default surface (the same families the dashboard
                // counts and `scan` trusts); `all` or an explicit `surface=` filter widens to
                // the raw universe.
                let widen = q.all || q.filters.iter().any(|flt| flt.field == "surface");
                let sel =
                    query_selection(&dataset.families, &overrides, &opp, &q, &path_arg, since)?;
                match &q.group {
                    Some(field) => render_query_group(&sel, field, &terms, &path_arg, json, since),
                    None => {
                        render_query_list(
                            &sel, &overrides, &opp, &q, &terms, &path_arg, widen, json, since,
                        );
                    }
                }
            }
        }
    }
    // Warn when discovery found nothing to report, so a mistyped path or unsupported tree
    // doesn't masquerade as "no duplication found" (and a CI `--fail-on` gate doesn't silently
    // pass on a bad path). A docs-only tree with markdown near-dups is a real result, so the
    // dashboard's `markdown_found` suppresses it there.
    if dataset.scope.files == 0 && !markdown_found {
        warn_no_files(&args.paths);
    }
    // CI gate (same as scan), scoped to the same untruncated family selection the query terms
    // address. `top=N` stays presentation-only; filters/id/at/status narrow the gate. The
    // `reinvented` view is a separate helper channel, not a RefactorFamily report.
    let reportable = if q.reinvented {
        Vec::new()
    } else {
        query_selection(&dataset.families, &overrides, &opp, &q, &path_arg, since)?
            .into_iter()
            .filter(|f| is_default_report_family(f, &overrides))
            .collect()
    };
    enforce_scan_fail_on(
        &args,
        dataset.settings.channels,
        &reportable,
        baseline_comparison.as_ref(),
    );
    Ok(())
}

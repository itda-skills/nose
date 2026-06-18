//! `nose` — multi-language code clone detector CLI.

mod baseline;
mod baseline_view;
mod cache;
mod capabilities;
mod cli_args;
mod config;
mod detect_command;
mod diagnostic_commands;
mod falsify;
mod family_display;
mod fnv;
mod ignores;
mod il_command;
mod markdown;
mod oracle_gate;
mod query_commands;
mod query_dashboard;
mod query_model;
mod query_open;
mod query_terms;
mod query_views;
mod review;
mod scan_baseline_gate;
mod scan_commands;
mod scan_human;
mod scan_json;
mod scan_markdown;
mod scan_opportunities;
mod scan_options;
mod scan_report;
mod scan_sarif;
mod scan_source_lines;
mod scan_witness;
mod schema_versions;
mod semantic_pack;
mod surfaces;
mod timing;
mod verify_census;
mod verify_collect;
mod verify_report;

use anyhow::{Context, Result};
use baseline_view::*;
use clap::Parser;
pub(crate) use cli_args::*;
pub(crate) use detect_command::*;
use diagnostic_commands::*;
use family_display::*;
pub(crate) use il_command::*;
use nose_il::{Corpus, FileId, Interner, Lang};
use oracle_gate::*;
use query_commands::*;
use query_terms::{family_at, parse_query, QFilter, QOp, Query};
use rayon::prelude::*;
pub(crate) use scan_baseline_gate::*;
use scan_commands::*;
pub(crate) use scan_human::*;
use scan_json::{ScanJsonInput, ScanJsonReport};
pub(crate) use scan_markdown::*;
pub(crate) use scan_opportunities::*;
pub(crate) use scan_options::*;
use scan_report::*;
pub(crate) use scan_sarif::*;
pub(crate) use scan_source_lines::*;
pub(crate) use scan_witness::*;
use std::path::PathBuf;
use surfaces::{
    classify_surface_overrides, effective_surface, family_actionability_reason,
    is_default_report_family, surface_omission_note, SurfaceOverrides,
};
pub(crate) use timing::*;
use verify_collect::*;
use verify_report::*;

/// Terminal styling for the human report. Colour is emitted only when stdout is a real
/// terminal and `NO_COLOR` is unset (so piped/redirected output — JSON/markdown/SARIF, and
/// the test harness — stays plain ASCII). Each helper returns its input unchanged when colour
/// is off, so callers wrap freely without branching; widths are always measured on the plain
/// text, never the wrapped string.
mod style {
    use std::io::IsTerminal;
    use std::sync::OnceLock;

    fn enabled() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| {
            std::env::var_os("NO_COLOR").is_none()
                && std::env::var("TERM").map_or(true, |t| t != "dumb")
                && std::io::stdout().is_terminal()
        })
    }

    fn paint(code: &str, s: &str) -> String {
        if s.is_empty() || !enabled() {
            s.to_string()
        } else {
            format!("\x1b[{code}m{s}\x1b[0m")
        }
    }

    pub(crate) fn bold(s: &str) -> String {
        paint("1", s)
    }
    pub(crate) fn dim(s: &str) -> String {
        paint("2", s)
    }
    pub(crate) fn green(s: &str) -> String {
        paint("32", s)
    }
    pub(crate) fn yellow(s: &str) -> String {
        paint("33", s)
    }
    pub(crate) fn blue(s: &str) -> String {
        paint("34", s)
    }
    pub(crate) fn bold_green(s: &str) -> String {
        paint("1;32", s)
    }
}

/// Borrow a slice of owned `PathBuf`s as `&Path` references — the form the detection entry
/// points take. Used by every scan/refactor subcommand that holds its input paths as a
/// `Vec<PathBuf>`.
fn paths_as_refs(paths: &[PathBuf]) -> Vec<&std::path::Path> {
    paths.iter().map(|p| p.as_path()).collect()
}

/// Stack size for the worker pool and the main worker thread. Lowering/normalization
/// walk the syntax tree recursively, so a pathologically deep file (minified bundle,
/// generated code) can need a deep stack — far more than the default ~2 MB (rayon
/// worker) or ~8 MB (main). Sized generously so nose never crashes on real repos.
/// Virtual only; pages commit lazily. See `deeply_nested_file_does_not_overflow`.
const STACK_SIZE: usize = 1024 * 1024 * 1024;

fn main() -> Result<()> {
    install_broken_pipe_guard();
    // rayon executes tasks both on its pool workers AND inline on the calling thread,
    // so enlarge the workers' stacks here and run the command body on a big-stack
    // thread below — otherwise a deep file lowered inline on a normal-stack thread
    // still overflows.
    let _ = rayon::ThreadPoolBuilder::new()
        .stack_size(STACK_SIZE)
        .build_global();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("spawn worker thread")
        .join()
        .expect("worker thread panicked")
}

/// When a reader closes the pipe early — `nose scan … | head`, quitting a pager —
/// the next write to stdout fails with `BrokenPipe`, and `println!` turns that into
/// a panic (the ugly `failed printing to stdout` message). The Unix convention for a
/// filter is to stop quietly instead. The textbook fix is to reset the `SIGPIPE`
/// disposition to `SIG_DFL`, but that needs `unsafe` and this crate is `unsafe`-free
/// (`unsafe_code = "forbid"`), so we install a panic hook that recognizes the
/// broken-pipe panic and exits 0 without a backtrace, while leaving every other panic
/// to the normal hook (and the big-stack join handling above).
fn install_broken_pipe_guard() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if is_broken_pipe_panic(info) {
            std::process::exit(0);
        }
        default_hook(info);
    }));
}

/// True for the panic `println!`/`writeln!` raise when stdout (or stderr) is a broken
/// pipe. The payload is a `String` like `failed printing to stdout: Broken pipe
/// (os error 32)`; we match both the textual kind and the numeric `EPIPE` (32 on
/// Linux and macOS) so a localized `strerror` message is still caught.
fn is_broken_pipe_panic(info: &std::panic::PanicHookInfo<'_>) -> bool {
    let payload = info.payload();
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied());
    matches!(msg, Some(m) if m.contains("Broken pipe") || m.contains("os error 32"))
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Il {
            path,
            format,
            normalized,
            no_cfg_norm,
        } => cmd_il(path, format, normalized, no_cfg_norm),
        Cmd::Capabilities => capabilities::print(),
        Cmd::SemanticPack { cmd } => match cmd {
            SemanticPackCmd::Check { paths, format } => semantic_pack::cmd_check(paths, format),
        },
        cmd @ Cmd::Detect { .. } => run_detect_cmd(cmd),
        Cmd::Eval {
            gold,
            predictions,
            hard_negatives,
            corpus,
        } => cmd_eval(gold, predictions, hard_negatives, corpus),
        cmd @ Cmd::Scan { .. } => run_scan_cmd(cmd),
        cmd @ Cmd::Query { .. } => run_query_cmd(cmd),
        cmd @ Cmd::Review { .. } => run_review_cmd(cmd),
        Cmd::Ceiling {
            gold,
            units,
            candidates,
        } => cmd_ceiling(gold, units, candidates),
        Cmd::Stats { paths, top, format } => cmd_stats(paths, top, format == StatsFormat::Json),
        Cmd::Features {
            paths,
            min_lines,
            min_tokens,
            no_cfg_norm,
            no_blocks,
        } => cmd_features(paths, min_lines, min_tokens, no_cfg_norm, no_blocks),
        Cmd::ValueCensus { paths, no_cfg_norm } => cmd_value_census(paths, no_cfg_norm),
        Cmd::Verify {
            paths,
            no_cfg_norm,
            json,
            max_violations,
            leads,
            exclusion_census,
            falsify,
        } => cmd_verify(
            paths,
            no_cfg_norm,
            json,
            max_violations,
            leads,
            exclusion_census,
            falsify,
        ),
        Cmd::BehavioralGate {
            paths,
            manifest,
            battery,
        } => cmd_behavioral_gate(paths, manifest, battery),
    }
}

fn run_detect_cmd(cmd: Cmd) -> Result<()> {
    let Cmd::Detect {
        paths,
        min_lines,
        min_tokens,
        threshold,
        candidates,
        minhash_k,
        bands,
        no_cfg_norm,
        dce,
        no_blocks,
        out,
        summary,
        bench_schema,
        repos_root,
        dump,
    } = cmd
    else {
        unreachable!("run_detect_cmd requires Cmd::Detect")
    };
    cmd_detect(DetectArgs {
        paths,
        min_lines,
        min_tokens,
        threshold,
        candidates,
        minhash_k,
        bands,
        no_cfg_norm,
        dce,
        no_blocks,
        out,
        summary,
        bench_schema,
        repos_root,
        dump,
    })
}

fn run_scan_cmd(cmd: Cmd) -> Result<()> {
    let Cmd::Scan {
        paths,
        top,
        min_members,
        min_value,
        sort,
        config,
        mode,
        show,
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
        scope,
    } = cmd
    else {
        unreachable!("run_scan_cmd requires Cmd::Scan")
    };
    require_paths_exist(&paths)?;
    cmd_scan(ScanArgs {
        paths,
        top,
        min_members,
        min_value,
        sort,
        config,
        mode,
        show,
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
        scope,
    })
}

fn run_review_cmd(cmd: Cmd) -> Result<()> {
    let Cmd::Review {
        paths,
        base,
        mode,
        min_size,
        min_lines,
        exclude,
        config,
        ignore_file,
        format,
        top,
        fail,
        fail_on,
    } = cmd
    else {
        unreachable!("run_review_cmd requires Cmd::Review")
    };
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    };
    require_paths_exist(&paths)?;
    // `nose review` is deprecated in favour of `nose query <paths> base=<ref>` (#375), which
    // runs this exact detection under the unified query surface (same findings, same gate).
    // The nudge is interactive-only — gated on a TTY stderr — so machine/CI/test runs (piped
    // stderr) are never spammed; `capabilities.commands.deprecated` is the machine signal.
    if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        eprintln!(
            "note: `nose review` is deprecated — use `nose query {} base={}` (same divergent-edit \
             detection; add --fail-on any for the gate). See `nose query --help`.",
            paths
                .first()
                .map_or(".".into(), |p| p.display().to_string()),
            base,
        );
    }
    review::cmd_review(review::ReviewArgs {
        paths,
        base,
        mode,
        min_size,
        min_lines,
        exclude,
        config,
        ignore_file,
        format,
        top,
        fail,
        fail_on,
    })
}

fn cmd_verify(
    paths: Vec<PathBuf>,
    no_cfg_norm: bool,
    json: bool,
    max_violations: Option<usize>,
    leads: Option<PathBuf>,
    exclusion_census: Option<PathBuf>,
    falsify: bool,
) -> Result<()> {
    let refs = paths_as_refs(&paths);
    let corpus = nose_frontend::lower_corpus_many(&refs);
    warn_if_empty(&corpus, &paths);
    let opts = nose_normalize::NormalizeOptions {
        cfg_norm: !no_cfg_norm,
        ..Default::default()
    };
    // Mine the literals the corpus branches on, to probe value-keyed branches. The
    // battery is identical for every unit (a function uses only its first `arity`
    // inputs), so behavior vectors are always length-comparable.
    let battery = verify_battery(&verify_probes(&corpus));
    let oracle = collect_verify_recs(&corpus, &opts, &battery, exclusion_census.is_some());
    if let Some(path) = &exclusion_census {
        verify_census::write_report(path, &oracle.census)?;
        println!("exclusion census written to {}", path.display());
    }

    if json {
        return print_verify_json(&oracle);
    }

    println!("=== value-graph oracle (soundness + completeness) ===");
    println!(
        "units: {} total, {} interpretable ({} excluded)",
        oracle.total,
        oracle.recs.len(),
        oracle.total - oracle.recs.len()
    );
    print_verify_exclusions(&oracle.exclusions);

    // --- Canon preservation: full-normalize behavior must equal pre-canon core behavior. ---
    println!("\nCANON PRESERVATION — normalization preserves behavior:");
    println!(
        "  units checked (interpretable both ways): {}",
        oracle.canon_checked
    );
    if oracle.canon_violations.is_empty() {
        println!("  PRESERVED: every canon-changed unit computes the same thing ✓");
    } else {
        println!(
            "  [!] {} unit(s) whose behavior CHANGED under canonicalization:",
            oracle.canon_violations.len()
        );
        for loc in &oracle.canon_violations {
            println!("    {loc}");
        }
    }

    let mut n_violations = report_verify_soundness(&oracle.recs);
    report_verify_completeness(&oracle.recs, leads.as_deref())?;
    report_verify_calibration(&oracle.recs);
    if falsify {
        // Falsification search (#317): augment the fixed battery with a per-group
        // distinguishing-input search. Any hit is a false merge the battery's input
        // starvation missed; count it toward the gate so `--falsify --max-violations 0` is
        // the stronger engine the issue calls for.
        n_violations += report_falsify(&corpus, &opts, &oracle.recs, &verify_probes(&corpus));
    }

    // CI soundness gate: fail if false merges exceed the budget, or if any normalization
    // pass changes a unit's behavior vs the pre-canon core IL. The independent oracle thus
    // becomes a permanent regression gate on the detection campaign: a new canon that
    // introduces a real false merge, or changes behavior before it even collides with a twin,
    // trips this gate.
    if let Some(budget) = max_violations {
        if !oracle.canon_violations.is_empty() {
            anyhow::bail!(
                "verify gate: {} canon-preservation violations",
                oracle.canon_violations.len()
            );
        }
        if n_violations > budget {
            anyhow::bail!("verify gate: {n_violations} false merges exceed the budget of {budget}");
        }
        println!("\nGATE: {n_violations} ≤ {budget} false merges — OK ✓");
    }
    Ok(())
}

struct ChannelDetector {
    name: &'static str,
    detectors: Vec<Box<dyn nose_detect::Detector>>,
}

impl nose_detect::Detector for ChannelDetector {
    fn name(&self) -> &str {
        self.name
    }

    fn score(&self, a: &nose_detect::UnitFeat, b: &nose_detect::UnitFeat) -> f64 {
        self.detectors
            .iter()
            .map(|d| d.score(a, b))
            .fold(0.0, f64::max)
    }
}

/// Lower + detect + rank clone families for a set of paths — the shared core of `scan`
/// and `review` (no cache, baseline, or presentation post-processing).
pub(crate) fn detect_families(
    paths: &[PathBuf],
    exclude: &[String],
    mode: Vec<ScanMode>,
    cfg_mode: Vec<ScanMode>,
    min_tokens: usize,
    min_lines: u32,
) -> Result<Vec<nose_detect::RefactorFamily>> {
    validate_exclude_globs(exclude)?;
    let refs = paths_as_refs(paths);
    let channels = ScanChannels::resolve(mode, cfg_mode, REVIEW_DEFAULT_MODES)?;
    let opts = scan_detect_options(channels, min_tokens, min_lines);
    let detector = scan_detector(channels, &opts);
    let corpus = nose_frontend::lower_corpus_filtered(&refs, exclude);
    let report = nose_detect::detect(&corpus, &opts, detector.as_ref());
    let mut families = nose_detect::rank_families(&report);
    if channels.abstraction_only() {
        families.retain(|f| f.abstraction_witness.is_some());
    }
    // The graded witness is NOT attached here: `review` enriches only the *flagged*
    // families (a small subset of a diff) in `flag_divergences`, not every near family
    // in the repo — enriching all of them on every gate run would be wasted work.
    Ok(families)
}

/// Detection options for the resolved scan channels — shared by `scan` and `review`.
fn scan_detect_options(
    channels: ScanChannels,
    min_tokens: usize,
    min_lines: u32,
) -> nose_detect::DetectOptions {
    nose_detect::DetectOptions {
        threshold: channels.threshold(),
        min_lines,
        min_tokens,
        contiguous_min_tokens: min_tokens,
        contiguous_min_lines: min_lines,
        structural: channels.structural(),
        contiguous: channels.syntax,
        // Near also generates VALUE candidates so behaviorally-convergent but shape-divergent
        // pairs (async `.then` ≡ await, impure loop ≡ comprehension) reach the candidate scorer —
        // they share no shape band, so shape-LSH alone would never propose them.
        value_candidates: channels.semantic || channels.near || channels.abstraction,
        shape_candidates: channels.near || channels.abstraction,
        shape_features: channels.near || channels.abstraction,
        abstraction_witnesses: channels.abstraction,
        ..Default::default()
    }
}

fn validate_exclude_globs(exclude: &[String]) -> Result<()> {
    if exclude.is_empty() {
        return Ok(());
    }
    let mut builder = ignore::overrides::OverrideBuilder::new(".");
    for glob in exclude {
        builder
            .add(&format!("!{glob}"))
            .with_context(|| format!("invalid exclude glob {glob:?}"))?;
    }
    builder.build().context("building exclude glob matcher")?;
    Ok(())
}

fn scan_detector(
    channels: ScanChannels,
    opts: &nose_detect::DetectOptions,
) -> Box<dyn nose_detect::Detector> {
    let mut detectors: Vec<Box<dyn nose_detect::Detector>> = Vec::new();
    if channels.semantic {
        detectors.push(Box::new(nose_detect::ExactBehaviorDetector));
    }
    if channels.near || channels.abstraction {
        detectors.push(Box::new(
            nose_detect::StructuralDetector::candidates(opts.jaccard_weight)
                .without_exact_behavior()
                .with_threshold(opts.threshold),
        ));
    }

    match detectors.len() {
        0 => Box::new(nose_detect::CopyPasteDetector),
        1 => detectors.pop().expect("one detector"),
        _ => Box::new(ChannelDetector {
            name: if channels.abstraction && !channels.near {
                "semantic+abstraction"
            } else if channels.abstraction {
                "semantic+near+abstraction"
            } else {
                "semantic+near"
            },
            detectors,
        }),
    }
}

/// Warn (to stderr) when discovery turned up nothing, so a mistyped path or an
/// unsupported tree doesn't masquerade as "no duplication found". Returns true if
/// the corpus is empty (caller may choose to stop early).
fn warn_if_empty(corpus: &Corpus, paths: &[PathBuf]) -> bool {
    if corpus.files.is_empty() {
        warn_no_files(paths);
        return true;
    }
    false
}

/// Render `file` relative to `cwd` when it sits underneath it; otherwise leave it
/// as-is (an absolute path outside cwd is more useful whole than mangled).
fn relativize(file: &str, cwd: &std::path::Path) -> String {
    std::path::Path::new(file)
        .strip_prefix(cwd)
        .ok()
        .and_then(|p| p.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| file.to_string())
}

fn relativize_loc(loc: &mut nose_detect::Loc, cwd: &std::path::Path) {
    loc.file = relativize(&loc.file, cwd);
    if let Some(parent) = &mut loc.enclosing_unit {
        parent.file = relativize(&parent.file, cwd);
        parent.refresh_unit_key();
    }
}

/// Stderr notice that discovery found nothing — so a mistyped path or unsupported
/// tree doesn't masquerade as "no duplication found".
/// A named path that doesn't exist is a usage error, not an empty scan: a typo'd
/// path in a CI gate must fail loudly instead of passing on a 0-file report.
/// "Exists but contains no supported files" stays a warning (`warn_no_files`).
fn require_paths_exist(paths: &[PathBuf]) -> Result<()> {
    let missing: Vec<String> = paths
        .iter()
        .filter(|p| !p.exists())
        .map(|p| p.display().to_string())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    anyhow::bail!("path does not exist: {}", missing.join(", "))
}

fn warn_no_files(paths: &[PathBuf]) {
    let joined = paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!(
        "warning: no supported source files found under: {joined}\n  \
         (supported extensions: py/pyi, js/jsx/mjs/cjs, ts/tsx/mts/cts, go, rs, java, c/h, rb, swift, css, vue/svelte/html/htm, md/markdown)"
    );
}

fn semantic_pack_summary_line(packs: &nose_semantics::SemanticPackSet) -> Option<String> {
    let first_party_count = packs
        .packs()
        .iter()
        .filter(|pack| pack.source == nose_semantics::SemanticPackSource::CompiledFirstParty)
        .count();
    let local = packs
        .packs()
        .iter()
        .filter(|pack| pack.source == nose_semantics::SemanticPackSource::LocalManifest)
        .map(|pack| format!("{}@{} ({})", pack.id, pack.version, pack.influence.as_str()))
        .collect::<Vec<_>>();
    (!local.is_empty()).then(|| {
        format!(
            "semantic packs: {first_party_count} first-party default · {} local opt-in: {}",
            local.len(),
            local.join(", ")
        )
    })
}

// ===================== shared report text helpers =====================

/// The right noun form for a count: singular when `n == 1`, plural otherwise (so `0`
/// reads "0 families"). Returns just the noun — the caller prints the number.
fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 {
        one
    } else {
        many
    }
}

// Query planning, rendering, and command orchestration live in query_* modules.

// Scan dataset construction and command orchestration live in scan_commands.rs.

/// Attach the #315 graded equivalence witness to each near (structural-similarity)
/// same-language family: re-lower its two representative copies, anti-unify their value
/// DAGs, and record "equal except these k holes" — each hole's value class, the
/// referent check, and source text. Best-effort enrichment, exactly like
/// `weight_shared_lines`: this layer has the source access the detector lacks, and
/// families it cannot witness (cross-language, fragments, pathological files) simply
/// keep their ungraded witness. The representative pair is the family's two largest
/// copies (`locations[0]` and `locations[1]`).
/// Attach graded witnesses only when they will actually be emitted. The witness is
/// serialized solely by the JSON report (the human and SARIF surfaces never render it),
/// and re-deriving it for every near family is the dominant scan cost on large repos
/// (netty: ~2.8s of a ~4.6s near scan) — so the common interactive `nose scan` (human)
/// pays nothing for evidence it does not show.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_model::{
        family_existing_helper, family_spotclass, query_family_json, short_id,
    };
    use crate::surfaces::{
        family_is_compiled_css_pipeline, has_version_tag, looks_compiled_css, span_is_declarations,
    };
    use nose_detect::{LineSpan, Loc, LocInit, RefactorFamily};

    #[test]
    fn compiled_css_is_detected_but_hand_written_is_not() {
        // Distributed / compiled stylesheets carry build markers → treated as generated.
        assert!(looks_compiled_css(
            "dist/app.css",
            "/*! App v1.2.3 | MIT */\n.a{x:1}"
        ));
        // minified bundle: banner collapsed behind a leading @charset on one line
        assert!(looks_compiled_css(
            "css/pico.amber.css",
            "@charset \"UTF-8\";/*! Pico CSS v2.1.1 */\n.x{y:1}"
        ));
        assert!(looks_compiled_css(
            "css/sakura.css",
            "/* Sakura.css v1.5.1 */\nhtml{x:1}"
        ));
        assert!(looks_compiled_css("a/b.min.css", ".x{y:1}"));
        assert!(looks_compiled_css(
            "css/bulma.css",
            "@charset \"UTF-8\";\n.x{y:1}\n/*# sourceMappingURL=bulma.css.map */"
        ));
        // Hand-written application/source CSS has none of these markers → NOT generated.
        assert!(!looks_compiled_css(
            "src/styles/app.css",
            "/* app styles */\n.card { padding: 1rem; }\n.btn { color: red; }"
        ));
        assert!(!looks_compiled_css(
            "src/parts/_range.css",
            "input[type=range]{ width:100% }"
        ));
        // A preprocessor SOURCE file is the input, not compiled output.
        assert!(!looks_compiled_css(
            "scss/_buttons.css",
            "/*! v1.0 */\n.b{x:1}"
        ));
        // Non-CSS is never matched here.
        assert!(!looks_compiled_css("app.js", "/*! lib v1.2.3 */"));
        assert!(has_version_tag("Sakura.css v1.5.1"));
        assert!(has_version_tag("Pico v2.1"));
        assert!(!has_version_tag("version two point oh"));
        assert!(!has_version_tag("v2 final")); // no dotted minor → not a release tag
    }

    #[test]
    fn decorator_prefix_is_language_aware() {
        // `@` is a decorator in these languages...
        assert_eq!(decorator_prefix("python"), Some("@"));
        assert_eq!(decorator_prefix("typescript"), Some("@"));
        assert_eq!(decorator_prefix("java"), Some("@"));
        assert_eq!(decorator_prefix("rust"), Some("#["));
        // ...but in Ruby a leading `@` is an INSTANCE VARIABLE, not a decorator, and
        // Go/C have no such syntax — these must report none, or `@token = …` would be
        // misread as a decorator and falsely split equal families.
        assert_eq!(decorator_prefix("ruby"), None);
        assert_eq!(decorator_prefix("go"), None);
        assert_eq!(decorator_prefix("c"), None);
    }

    #[test]
    fn decorator_difference_detects_arg_changes() {
        let a = vec![r#"@click.argument("arg")"#.to_string()];
        let b = vec![r#"@click.argument("arg", metavar="m")"#.to_string()];
        let (a_only, b_only) = decorator_difference(&a, &b).expect("differs");
        assert_eq!(a_only, vec![r#"@click.argument("arg")"#.to_string()]);
        assert_eq!(
            b_only,
            vec![r#"@click.argument("arg", metavar="m")"#.to_string()]
        );
        // Identical decorator sets do not differ (the legit equal-modulo-holes case).
        assert!(decorator_difference(&a, &a).is_none());
        // Extra decorator on one side only.
        let c = vec!["@a".to_string(), "@b".to_string()];
        let d = vec!["@a".to_string()];
        let (c_only, d_only) = decorator_difference(&c, &d).expect("differs");
        assert_eq!(c_only, vec!["@b".to_string()]);
        assert!(d_only.is_empty());
    }

    fn fam(langs: usize, modules: usize, names: &[Option<&str>]) -> RefactorFamily {
        fam_kind(langs, modules, names, nose_il::UnitKind::Function)
    }

    fn fam_kind(
        langs: usize,
        modules: usize,
        names: &[Option<&str>],
        kind: nose_il::UnitKind,
    ) -> RefactorFamily {
        let locations = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                Loc::new(LocInit {
                    file: format!("m{i}/f.rs"),
                    source_span: LineSpan::new(1, 10),
                    lang: "rust".into(),
                    kind,
                    origin: Default::default(),
                    name: n.map(|s| s.to_string()),
                    sem: 50,
                    span_tokens: 50,
                })
            })
            .collect();
        RefactorFamily {
            value: 1.0,
            members: names.len(),
            files: names.len(),
            modules,
            languages: langs,
            mean_score: 0.9,
            mean_lines: 10,
            dup_lines: 10,
            shared_lines: 0,
            params: 0,
            shared_weight: 0.0,
            locations,
            mean_sem: 50.0,
            scope: "prod",
            discount: 1.0,
            abstraction_witness: None,
            witness: None,
            varying_spots: Vec::new(),
            semantic_laws: Vec::new(),
        }
    }

    #[test]
    fn verify_battery_budget_is_node_row_bounded() {
        assert!(
            !verify_battery_over_budget(2_000, 192),
            "the documented 2k x 192-row boundary stays inside the verify budget"
        );
        assert!(
            verify_battery_over_budget(2_001, 192),
            "one node beyond the boundary fails closed as battery-bail"
        );
        assert!(
            !verify_battery_over_budget(6_000, 1),
            "large units are allowed when the battery is tiny"
        );
    }

    #[test]
    fn shared_lines_params_come_from_first_successful_pair() {
        use std::io::Write;
        // The representative pair can be unreadable while a *later* pair reads fine
        // (e.g. a deleted/edited file among the family members). The parameter count
        // must then come from the first pair that actually reads — not be dropped
        // just because the readable pair wasn't iteration 0.
        let dir = std::env::temp_dir().join(format!("nose_slo_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, body: &str| {
            let p = dir.join(name);
            std::fs::File::create(&p)
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
            p.to_string_lossy().to_string()
        };
        let f0 = write("a.rs", "AAA\nshared1\nshared2\n");
        let f2 = write("c.rs", "BBB\nshared1\nshared2\n");
        let missing = dir.join("missing.rs").to_string_lossy().to_string();

        let mk = |file: String| {
            Loc::new(LocInit {
                file,
                source_span: LineSpan::new(1, 3),
                lang: "rust".into(),
                kind: nose_il::UnitKind::Function,
                origin: Default::default(),
                name: None,
                sem: 50,
                span_tokens: 50,
            })
        };
        // locs[1] (the first compared pair) is unreadable; locs[2] reads and differs
        // from the representative by one parameter line.
        let locs = vec![mk(f0), mk(missing), mk(f2)];
        let mut cache = FileLineCache(std::collections::HashMap::new());
        let s = shared_lines_of(&locs, &mut cache).expect("a later pair reads");

        assert!(
            s.rank_lines.contains(&"shared1".to_string()),
            "shared lines extracted: {:?}",
            s.rank_lines
        );
        assert_eq!(
            s.params, 1,
            "params must come from the first successful pair, not iteration 0"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hint_shared_name_consolidates() {
        let f = fam(1, 3, &[Some("series"), Some("series"), Some("series")]);
        assert_eq!(family_hint(&f), "consolidate `series` — 3 copies");
    }

    #[test]
    fn hint_cross_language_is_flagged() {
        let f = fam(2, 2, &[Some("parse"), Some("parse")]);
        assert!(family_hint(&f).ends_with("(cross-language)"));
    }

    #[test]
    fn hint_mixed_names_falls_back_to_spread() {
        let f = fam(1, 3, &[Some("replace"), Some("replaceOrAppend"), None]);
        assert_eq!(
            family_hint(&f),
            "repeated across 3 directories — extract a shared abstraction"
        );
    }

    #[test]
    fn hint_test_scope_flags_scaffolding_caveat() {
        let mut f = fam(1, 2, &[None, None]);
        f.scope = "test";
        let h = family_hint(&f);
        assert!(h.contains("extract a helper"), "{h}");
        assert!(h.ends_with("not per-scenario setup"), "{h}");
    }

    #[test]
    fn hint_prod_scope_has_no_test_caveat() {
        let f = fam(1, 2, &[None, None]); // scope defaults to prod
        assert!(!family_hint(&f).contains("test scaffolding"));
    }

    #[test]
    fn hint_high_param_caution_wins_over_test_caveat() {
        let mut f = fam(1, 2, &[None, None]);
        f.scope = "test";
        f.params = 8; // >= HIGH_PARAM_SPOTS
        let h = family_hint(&f);
        assert!(h.contains("high-parameter"), "{h}");
        assert!(
            !h.contains("test scaffolding"),
            "high-param branch wins: {h}"
        );
    }

    #[test]
    fn hint_local_duplication() {
        let f = fam(1, 1, &[None, None]);
        assert_eq!(family_hint(&f), "local duplication — extract a helper");
    }

    #[test]
    fn hint_class_family_suggests_base_class() {
        let f = fam_kind(1, 3, &[None, None, None], nose_il::UnitKind::Class);
        assert_eq!(
            family_hint(&f),
            "repeated across 3 directories — extract a shared base class / mixin"
        );
    }

    #[test]
    fn hint_origin_protocol_contract_avoids_base_class() {
        use nose_il::{
            RegionKind, SourceGranularity, UnitBodyKind, UnitDomain, UnitDomains, UnitSubkind,
        };
        let mut f = fam_kind(
            1,
            2,
            &[Some("TraceReadable"), Some("TraceWritable")],
            nose_il::UnitKind::Class,
        );
        for loc in &mut f.locations {
            loc.lang = "swift".into();
            loc.origin = nose_il::UnitOrigin::new(
                UnitDomains::of(UnitDomain::TypeContract),
                UnitSubkind::InterfaceTraitProtocol,
                UnitBodyKind::DeclarationOnly,
                SourceGranularity::WholeUnit,
                RegionKind::Code,
            );
        }
        assert_eq!(
            family_hint(&f),
            "duplicated across 2 directories — consolidate one shared interface/protocol contract"
        );
        assert!(hint_reasons(&f)
            .iter()
            .any(|reason| reason == "no implementation body was found"));
    }

    #[test]
    fn hint_origin_behavior_class_keeps_base_class() {
        use nose_il::{
            RegionKind, SourceGranularity, UnitBodyKind, UnitDomain, UnitDomains, UnitSubkind,
        };
        let mut f = fam_kind(1, 2, &[None, None], nose_il::UnitKind::Class);
        for loc in &mut f.locations {
            loc.origin = nose_il::UnitOrigin::new(
                UnitDomains::of(UnitDomain::ImplementationType),
                UnitSubkind::Class,
                UnitBodyKind::Implementation,
                SourceGranularity::WholeUnit,
                RegionKind::Code,
            );
        }
        assert_eq!(
            family_hint(&f),
            "duplicated across 2 directories — extract a shared base class / mixin"
        );
    }

    #[test]
    fn hint_origin_data_record_is_type_contract_not_base_class() {
        // A data record (TypeContract + Data, no implementation facet) must render as a
        // type/API contract, never "extract a shared base class / mixin" (#453).
        use nose_il::{
            RegionKind, SourceGranularity, UnitBodyKind, UnitDomain, UnitDomains, UnitSubkind,
        };
        let mut f = fam_kind(
            1,
            2,
            &[Some("Point"), Some("Coord")],
            nose_il::UnitKind::Class,
        );
        for loc in &mut f.locations {
            loc.lang = "java".into();
            loc.origin = nose_il::UnitOrigin::new(
                UnitDomains::of(UnitDomain::TypeContract).with(UnitDomain::Data),
                UnitSubkind::StructRecord,
                UnitBodyKind::DeclarativeDenotation,
                SourceGranularity::WholeUnit,
                RegionKind::Code,
            );
        }
        assert_eq!(
            family_hint(&f),
            "duplicated across 2 directories — consolidate one shared type/API contract"
        );
    }

    #[test]
    fn hint_origin_style_is_declarative() {
        use nose_il::{
            RegionKind, SourceGranularity, UnitBodyKind, UnitDomain, UnitDomains, UnitSubkind,
        };
        let mut f = fam_kind(1, 1, &[None, None], nose_il::UnitKind::Block);
        for loc in &mut f.locations {
            loc.lang = "css".into();
            loc.origin = nose_il::UnitOrigin::new(
                UnitDomains::of(UnitDomain::Style),
                UnitSubkind::CssRule,
                UnitBodyKind::DeclarativeDenotation,
                SourceGranularity::Rule,
                RegionKind::Style,
            );
        }
        assert_eq!(
            family_hint(&f),
            "local duplication — merge selectors or move the declarations to a shared class/token if these elements should be coupled"
        );
    }

    #[test]
    fn hint_block_family_suggests_method() {
        let f = fam_kind(1, 1, &[None, None], nose_il::UnitKind::Block);
        assert_eq!(
            family_hint(&f),
            "local duplication — extract a method from the repeated block"
        );
    }

    fn loc_at(file: &str, start: u32, end: u32, kind: nose_il::UnitKind) -> Loc {
        Loc::new(LocInit {
            file: file.to_string(),
            source_span: LineSpan::new(start, end),
            lang: "go".into(),
            kind,
            origin: Default::default(),
            name: None,
            sem: 50,
            span_tokens: 50,
        })
    }

    fn fam_at(spans: &[(&str, u32, u32)]) -> RefactorFamily {
        let mut f = fam_kind(1, 1, &vec![None; spans.len()], nose_il::UnitKind::Block);
        f.locations = spans
            .iter()
            .map(|(file, s, e)| loc_at(file, *s, *e, nose_il::UnitKind::Block))
            .collect();
        f
    }

    #[test]
    fn compiled_css_pipeline_demotes_source_plus_outputs_but_not_cross_source() {
        let gen: std::collections::HashSet<String> = [
            "css/bundle.css".to_string(),
            "css/bundle.min.css".to_string(),
        ]
        .into_iter()
        .collect();
        // 1 source partial + its compiled + minified outputs → build pipeline (demote).
        let pipe = fam_at(&[
            ("src/_a.css", 1, 9),
            ("css/bundle.css", 100, 108),
            ("css/bundle.min.css", 1, 1),
        ]);
        assert!(family_is_compiled_css_pipeline(&pipe, &gen));
        let ov = SurfaceOverrides {
            generated_sources: gen.clone(),
            declaration_run_ids: std::collections::HashSet::new(),
        };
        assert_eq!(effective_surface(&pipe, &ov), "generated");
        assert!(
            !is_default_report_family(&pipe, &ov),
            "CSS build-pipeline families stay off scan's default surface"
        );
        assert_eq!(
            family_actionability_reason(&pipe, &ov),
            Some("generated-source")
        );
        assert_eq!(
            surface_omission_note(std::slice::from_ref(&pipe), &ov).as_deref(),
            Some("omitted 1 family from default output (1 generated-code)")
        );
        // 2 distinct hand-written sources sharing a block (+ a compiled copy) → keep.
        let dedup = fam_at(&[
            ("src/_a.css", 1, 9),
            ("src/_b.css", 1, 9),
            ("css/bundle.css", 100, 108),
        ]);
        assert!(!family_is_compiled_css_pipeline(&dedup, &gen));
        // all-compiled also matches (subsumes the all-generated case for CSS).
        let allc = fam_at(&[("css/bundle.css", 1, 9), ("css/bundle.min.css", 1, 1)]);
        assert!(family_is_compiled_css_pipeline(&allc, &gen));
        // a non-CSS member disqualifies — this rule is CSS-only.
        let mixed = fam_at(&[("src/_a.css", 1, 9), ("app.js", 1, 9)]);
        assert!(!family_is_compiled_css_pipeline(&mixed, &gen));
    }

    #[test]
    fn overlapping_slices_fold_under_their_primary() {
        // B's members are both shifted slices of A's regions → one opportunity.
        // C shares only ONE region with A (its other member lives elsewhere) —
        // a single shared region can be coincidence, so C stays its own entry.
        let a = fam_at(&[("t/a.go", 100, 130), ("t/b.go", 50, 70)]);
        let b = fam_at(&[("t/a.go", 105, 128), ("t/b.go", 52, 66)]);
        let c = fam_at(&[("t/a.go", 100, 130), ("t/z.go", 5, 25)]);
        let ranked = [&a, &b, &c];
        let groups = OpportunityGroups::from_ranked(&ranked);
        assert!(groups.is_slice(&b), "b is a slice of a");
        assert!(
            !groups.is_slice(&a),
            "the best-ranked family is the primary"
        );
        assert!(!groups.is_slice(&c), "one shared region must not group");
        assert_eq!(
            groups.slices(&a),
            Some(&[baseline::family_id(&b)][..]),
            "a lists exactly b as its folded slice"
        );
    }

    #[test]
    fn query_family_json_carries_fold_navigation() {
        // a subsumes b (b's two members are shifted slices of a's regions).
        let a = fam_at(&[("t/a.go", 100, 130), ("t/b.go", 50, 70)]);
        let b = fam_at(&[("t/a.go", 105, 128), ("t/b.go", 52, 66)]);
        let ranked = [&a, &b];
        let opp = OpportunityGroups::from_ranked(&ranked);
        let ov = SurfaceOverrides {
            generated_sources: std::collections::HashSet::new(),
            declaration_run_ids: std::collections::HashSet::new(),
        };
        // The primary lists the slice ids it subsumes (navigable id= handles).
        let ja = query_family_json(&a, &ov, &opp, false, None);
        assert_eq!(
            ja["subsumes"],
            serde_json::json!([short_id(&baseline::family_id(&b))]),
            "primary names the slices it subsumes: {ja}"
        );
        assert!(ja.get("subsumed_by").is_none(), "a primary is not subsumed");
        // The slice points back at its primary.
        let jb = query_family_json(&b, &ov, &opp, false, None);
        assert_eq!(
            jb["subsumed_by"],
            serde_json::Value::from(short_id(&baseline::family_id(&a))),
            "slice points at its primary: {jb}"
        );
    }

    #[test]
    fn classify_param_hints_value_class() {
        assert_eq!(classify_param(&["  42"]), "literal");
        assert_eq!(classify_param(&["\"hello\""]), "literal");
        assert_eq!(classify_param(&["foo.bar"]), "name");
        assert_eq!(classify_param(&["compute(x, y)"]), "call");
        assert_eq!(classify_param(&["a + b * c"]), "expr");
        assert_eq!(classify_param(&["line one", "line two"]), "block");
        assert_eq!(classify_param(&[]), "expr");
    }

    #[test]
    fn query_family_json_carries_proof_depth() {
        let ov = SurfaceOverrides {
            generated_sources: std::collections::HashSet::new(),
            declaration_run_ids: std::collections::HashSet::new(),
        };
        let empty = OpportunityGroups::default();
        // Exact channel: how much is proven identical (the shared value-multiset size).
        let mut exact = fam(1, 2, &[Some("a"), Some("b")]);
        exact.witness = Some(nose_detect::EquivalenceWitness {
            kind: "exact-value-graph",
            value_nodes: Some(12),
            mean_value_jaccard: None,
            mean_shape_jaccard: None,
            graded: None,
        });
        let je = query_family_json(&exact, &ov, &empty, false, None);
        assert_eq!(
            je["value_nodes"], 12,
            "exact family carries value_nodes: {je}"
        );
        // Sub-dag channel: the proven shared-computation span per location.
        let mut sub = fam(1, 2, &[Some("c"), Some("d")]);
        sub.locations[0].shared_subdag = Some((10, 14));
        let js = query_family_json(&sub, &ov, &empty, false, None);
        assert_eq!(
            js["locations"][0]["shared_subdag"],
            serde_json::json!([10, 14]),
            "location carries the proven shared-subdag span: {js}"
        );
    }

    #[test]
    fn hint_prefers_calling_the_existing_helper() {
        let mut f = fam(1, 2, &[None, None, None]);
        f.locations = vec![
            {
                let mut l = loc_at("core/math.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("clamp".to_string());
                l
            },
            loc_at("ui/model.ts", 80, 84, nose_il::UnitKind::Block),
            loc_at("worker/job.ts", 33, 37, nose_il::UnitKind::Block),
        ];
        assert_eq!(
            family_hint(&f),
            "2 sites reimplement `clamp` — call the existing helper (core/math.ts)"
        );
    }

    #[test]
    fn existing_helper_names_the_call_target_member() {
        // A call-existing-helper family: one named function + inline copies that recompute it.
        let mut f = fam(1, 2, &[None, None, None]);
        f.locations = vec![
            {
                let mut l = loc_at("core/math.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("clamp".to_string());
                l
            },
            loc_at("ui/model.ts", 80, 84, nose_il::UnitKind::Block),
            loc_at("worker/job.ts", 33, 37, nose_il::UnitKind::Block),
        ];
        let helper = family_existing_helper(&f).expect("call-existing-helper has a helper member");
        assert_eq!(helper.name.as_deref(), Some("clamp"));
        assert_eq!(helper.file, "core/math.ts");
        // A plain multi-function family is a fresh extraction — there is no member to call.
        assert!(family_existing_helper(&fam(1, 2, &[Some("a"), Some("b")])).is_none());
    }

    #[test]
    fn spotclass_grades_near_family_holes() {
        use nose_detect::{EquivalenceWitness, GradedWitness, WitnessHole};
        let hole = |class: &'static str| WitnessHole {
            class,
            a_lines: None,
            b_lines: None,
            effect: false,
            a_text: String::new(),
            b_text: String::new(),
        };
        let graded = |spots: Vec<WitnessHole>, referent: Vec<String>| {
            let mut f = fam(1, 2, &[Some("x"), Some("y")]);
            f.witness = Some(EquivalenceWitness {
                kind: "structural-similarity",
                value_nodes: None,
                mean_value_jaccard: None,
                mean_shape_jaccard: None,
                graded: Some(GradedWitness {
                    holes: spots.len(),
                    spots,
                    patterns: Vec::new(),
                    referent_mismatches: referent,
                    caveat_names: Vec::new(),
                    equal_modulo_holes: true,
                    modeled_caveat: false,
                }),
            });
            f
        };
        // Only value-leaf holes → a clean parameterize/extract candidate.
        assert_eq!(
            family_spotclass(&graded(vec![hole("literal"), hole("call")], vec![])),
            Some("leaf-only")
        );
        // A shape/arity hole → genuine logic divergence, not just a parameter.
        assert_eq!(
            family_spotclass(&graded(vec![hole("literal"), hole("shape")], vec![])),
            Some("structural")
        );
        // A referent mismatch (same name, behaviorally distinct) → structural even with leaf holes.
        assert_eq!(
            family_spotclass(&graded(vec![hole("literal")], vec!["equals".into()])),
            Some("structural")
        );
        // No graded witness (not enriched / not a near family) → no class.
        assert!(family_spotclass(&fam(1, 1, &[Some("a"), Some("b")])).is_none());
    }

    #[test]
    fn helper_hint_never_points_prod_at_a_test_helper() {
        // Coevo C2: the named function lives in test code while the inline
        // copies are production — "call the existing helper" would be wrong-
        // direction advice, so the hint falls back to plain extraction.
        let mut f = fam(1, 2, &[None, None, None]);
        f.scope = "mixed";
        f.locations = vec![
            {
                let mut l = loc_at("tests/helpers.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("clamp".to_string());
                l
            },
            loc_at("ui/model.ts", 80, 84, nose_il::UnitKind::Block),
            loc_at("worker/job.ts", 33, 37, nose_il::UnitKind::Block),
        ];
        let hint = family_hint(&f);
        assert!(
            !hint.contains("call the existing helper"),
            "a test-code helper must not be recommended to prod copies: {hint}"
        );
        // All-test families may keep the recommendation: tests calling a test
        // helper is exactly the refactor.
        f.scope = "test";
        assert!(
            family_hint(&f).contains("call the existing helper"),
            "an all-test family may still consolidate on its test helper"
        );
    }

    #[test]
    fn helper_hint_allows_test_copies_to_call_a_prod_helper() {
        // C5 boundary: the inverse direction is fine — tests calling a
        // production helper is exactly the refactor.
        let mut f = fam(1, 2, &[None, None]);
        f.scope = "mixed";
        f.locations = vec![
            {
                let mut l = loc_at("core/math.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("clamp".to_string());
                l
            },
            loc_at("tests/model.spec.ts", 80, 84, nose_il::UnitKind::Block),
        ];
        assert!(
            family_hint(&f).contains("call the existing helper"),
            "prod helper recommended to test copies is the right direction"
        );
    }

    #[test]
    fn high_parameter_caution_boundary_is_six() {
        // S3-C5 gap: the >= boundary itself was untested.
        let mut f = fam(1, 1, &[None, None]);
        f.shared_lines = 30;
        f.params = 5;
        assert!(
            !family_hint(&f).contains("high-parameter"),
            "five spots is below the caution boundary"
        );
        f.params = 6;
        assert!(
            family_hint(&f).contains("high-parameter (6 varying spots)"),
            "six spots is the boundary and must carry the caution"
        );
    }

    #[test]
    fn helper_hint_carries_the_high_parameter_caution() {
        // S2-C2: the early return must not bypass the params caution — six
        // varying spots mean the inline copies diverge from the helper.
        let mut f = fam(1, 2, &[None, None, None]);
        f.params = 8;
        f.shared_lines = 12;
        f.locations = vec![
            {
                let mut l = loc_at("core/math.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("clamp".to_string());
                l
            },
            loc_at("ui/model.ts", 80, 84, nose_il::UnitKind::Block),
            loc_at("worker/job.ts", 33, 37, nose_il::UnitKind::Block),
        ];
        let hint = family_hint(&f);
        assert!(
            hint.contains("call the existing helper") && hint.contains("high-parameter (8"),
            "helper advice at 8 varying spots must carry the caution: {hint}"
        );
    }

    #[test]
    fn helper_hint_never_points_at_generated_code() {
        let mut f = fam(1, 2, &[None, None]);
        f.locations = vec![
            {
                let mut l = loc_at("gen/api.ts", 10, 14, nose_il::UnitKind::Function);
                l.name = Some("encode".to_string());
                l.looks_generated = true;
                l
            },
            loc_at("ui/model.ts", 80, 84, nose_il::UnitKind::Block),
        ];
        let hint = family_hint(&f);
        assert!(
            !hint.contains("call the existing helper"),
            "a generated-file helper is not the maintainer's API: {hint}"
        );
    }

    #[test]
    fn hint_flags_high_parameter_extractions() {
        let mut f = fam(1, 1, &[None, None]);
        f.params = 8;
        f.shared_lines = 12;
        let hint = family_hint(&f);
        assert!(
            hint.contains("high-parameter (8 varying spots)"),
            "an 8-spot extraction must carry the readability caution: {hint}"
        );
    }

    #[test]
    fn summary_names_the_equivalence_evidence() {
        let mut f = fam(1, 1, &[None, None]);
        f.witness = Some(nose_detect::EquivalenceWitness {
            kind: "exact-value-graph",
            value_nodes: Some(12),
            mean_value_jaccard: None,
            mean_shape_jaccard: None,
            graded: None,
        });
        assert!(
            family_summary(&f).contains("· exact behavior match"),
            "the human line names WHY the members merged: {}",
            family_summary(&f)
        );
    }

    /// Run the whole-source span through the AST-facts classifier — the same
    /// path `declaration_run_span` takes, minus the file I/O.
    fn ast_classifies(ext: &str, src: &str) -> bool {
        let Some(facts) = nose_frontend::declaration_facts(ext, src) else {
            return false;
        };
        let all: Vec<String> = src.lines().map(str::to_string).collect();
        let end = all.len().max(1) as u32;
        span_is_declarations(&facts, &all, 1, end)
    }

    #[test]
    fn declaration_spans_classify_per_language() {
        let yes: &[(&str, &str)] = &[
            ("ts", "import { a } from './a';\nimport { b } from './b';"),
            ("ts", "import {\n  a,\n  b,\n} from './ab';"),
            ("ts", "export { a } from './a';\nexport * from './b';"),
            ("ts", "const fs = require('fs');"),
            ("py", "import os\nfrom typing import (\n    Any,\n)"),
            (
                "go",
                "package main\n\nimport (\n\t\"fmt\"\n\talias \"net/http\"\n)",
            ),
            (
                "rs",
                "use std::fmt;\npub use crate::x::{\n    A,\n};\nmod wiring;",
            ),
            (
                "java",
                "package com.x;\nimport java.util.List;\nimport static java.util.Map.entry;",
            ),
            ("c", "#include <stdio.h>\n#include \"x.h\"\n#pragma once"),
            ("rb", "require 'json'\nrequire_relative 'x'"),
            // S2-C3 coverage rows: shapes the code supports but no test locked.
            ("rs", "pub(crate) use crate::x::Y;"),
            ("go", "import http \"net/http\""),
            ("py", "from os import path"),
            ("rb", "require('json')"),
            ("c", "#include<stdio.h>"),
            ("ts", "import{a} from './a';"),
            // ASI: a multi-line import may close without a semicolon.
            ("ts", "import {\n  a,\n} from './ab'"),
            // The closer may carry the final import names (corpus re-price
            // regression in series 2: bare-`)` leaked real Python imports).
            ("py", "from typing import (\n    Any,\n    Mapping)"),
            // S4-C5 coverage adoptions (supported kinds with no locked row).
            ("rs", "extern crate serde;\nextern crate serde_json;"),
            ("go", "package main"),
            ("rb", "require_relative './helpers'"),
            // S3-C5 coverage adoptions.
            ("go", "import (\n\t. \"fmt\"\n\t_ \"encoding/json\"\n)"),
            ("rs", "use std::{\n    io::{self, Read},\n};"),
            ("ts", "import {\n  $ref,\n} from './x';"),
            ("ts", "export {\n  a,\n  b,\n} from './lib';"),
            ("ts", "const $lib = require('lib');"),
            ("py", "from typing import (\n    Dict as D,\n)"),
            ("py", "from x import *"),
            // Corpus re-price regressions (series 3): inert trailing comments
            // and single-line parenthesized name lists are real wiring.
            ("py", "import os  # noqa"),
            ("py", "from os import path  # comment"),
            ("py", "from x import (a, b)"),
        ];
        for (ext, src) in yes {
            assert!(
                ast_classifies(ext, src),
                "should classify as declarations: {src}"
            );
        }
    }

    #[test]
    fn declaration_spans_fail_open_per_language() {
        // Fail-open: anything not provably a declaration keeps the family on
        // its ranked surface — misclassifying a real finding is the error
        // class this filter must never make.
        let no: &[(&str, &str)] = &[
            ("ts", "import { a } from './a';\nexport const x = a;"),
            ("ts", "import {\n  a,"),
            ("py", "import os\nx = os.environ"),
            ("go", "import (\n\t\"fmt\""),
            ("rs", "use std::fmt;\nfn main() {}"),
            ("java", "import java.util.List;\nclass X {}"),
            ("c", "#include <stdio.h>\n#define MAX 4"),
            ("rb", "require 'json'\nputs 'hi'"),
            ("py", ""),
            // C1 claim-violation packets: a single LINE mixing a declaration
            // with executable code must never classify (the "provably no
            // extraction exists" claim breaks if real code rides along).
            ("ts", "import { a } from './a'; doEvil();"),
            ("ts", "var a = require('a'), b = compute();"),
            ("go", "import \"fmt\"; func main() { hack() }"),
            ("rb", "require 'json'; system('x')"),
            ("py", "from x import y; z = 1"),
            ("java", "import java.util.List; int x = 1;"),
            ("rs", "use std::fmt; let x = 1;"),
            ("c", "#includeevil <x.h>"),
            // C5 boundary re-attack on the C1 defense itself.
            ("ts", "import { a } from './a';;"),
            ("rb", "require 'x' if expensive_check()"),
            // S2-C1 blind-attacker packets: open-block interiors and closers
            // were unvalidated (tree-sitter error tolerance voids any "the
            // file parsed, so interiors are specifiers" assumption).
            ("rb", "require 'fs' + 1"),
            ("c", "#include <stdio.h> int x = 1;"),
            ("ts", "import {\n  a,\n} || x();"),
            ("go", "import (\n\t\"fmt\"\n\tos.Exit(1))"),
            ("rs", "use std::{\ninvalid;\n};"),
            // S3-C1 blind-attacker packets: from-clause sources, Python name
            // lists, and Java paths smuggled expressions through shape checks.
            ("ts", "import { x } from Math.max(\"a\", \"b\");"),
            ("ts", "export { x } from path.join(\"c\", \"d\");"),
            ("py", "from x import max(\"a\", \"b\")"),
            ("java", "import java.util.x + y;"),
            ("java", "package com.example.x + y;"),
            // S3-C5 boundary re-attacks on the strict closers.
            ("rs", "use std::{\n  A,\n}x;"),
            ("go", "import (\n\tfunc() \"x\"\n)"),
            // S4-C1: call-shaped declaration entries whose binding/block
            // smuggles execution past the node-kind whitelist.
            (
                "js",
                "const { boom = stealCreditCards() } = require('lit');",
            ),
            ("js", "const { [exfiltrate()]: grabbed } = require('lit');"),
            (
                "ts",
                "const { boom = stealCreditCards() } = require('lit');",
            ),
            ("rb", "require('socket') { launch_missiles }"),
        ];
        for (ext, src) in no {
            assert!(!ast_classifies(ext, src), "must fail open on: {src:?}");
        }
    }

    #[test]
    fn declaration_spans_inert_destructure_still_classifies() {
        // The S4-C1 fix must not over-reject: a plain destructuring require
        // executes nothing and stays wiring.
        assert!(ast_classifies(
            "js",
            "const { boom, fizz } = require('lit');"
        ));
        assert!(ast_classifies("py", "\u{feff}import os")); // BOM-tolerant
    }
}

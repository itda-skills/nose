use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::cli_args::{Cli, Cmd, ScanArgs, SemanticPackCmd, StatsFormat};
use crate::detect_command::{cmd_detect, DetectArgs};
use crate::diagnostic_commands::{
    cmd_ceiling, cmd_eval, cmd_features, cmd_stats, cmd_value_census,
};
use crate::il_command::cmd_il;
use crate::oracle_gate::cmd_behavioral_gate;
use crate::oracle_gate::{verify_battery, verify_probes};
use crate::path_utils::{paths_as_refs, require_paths_exist, warn_if_empty};
use crate::query_commands::run_query_cmd;
use crate::scan_commands::cmd_scan;
use crate::verify_collect::collect_verify_recs;
use crate::verify_report::{
    print_verify_exclusions, print_verify_json, report_falsify, report_verify_calibration,
    report_verify_completeness, report_verify_soundness,
};
use crate::{capabilities, review, semantic_pack, verify_census};

pub(crate) fn run() -> Result<()> {
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

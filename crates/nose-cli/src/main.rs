//! `nose` — multi-language code clone detector CLI.

mod baseline;
mod baseline_view;
mod cache;
mod capabilities;
mod config;
mod diagnostic_commands;
mod falsify;
mod family_display;
mod fnv;
mod ignores;
mod markdown;
mod oracle_gate;
mod query_commands;
mod query_dashboard;
mod query_model;
mod query_open;
mod query_terms;
mod query_views;
mod review;
mod scan_commands;
mod scan_json;
mod schema_versions;
mod semantic_pack;
mod surfaces;
mod verify_census;
mod verify_collect;
mod verify_report;

use anyhow::{Context, Result};
use baseline_view::*;
use clap::{Parser, Subcommand};
use diagnostic_commands::*;
use family_display::*;
use nose_il::{Corpus, FileId, Interner, Lang};
use oracle_gate::*;
use query_commands::*;
use query_terms::{family_at, parse_query, QFilter, QOp, Query};
use rayon::prelude::*;
use scan_commands::*;
use scan_json::{ScanJsonInput, ScanJsonReport};
use std::path::PathBuf;
use surfaces::{
    classify_surface_overrides, effective_surface, family_actionability_reason,
    is_default_report_family, surface_omission_note, SurfaceOverrides,
};
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

#[derive(Parser)]
#[command(
    name = "nose",
    version,
    about = "Find duplicated code worth refactoring — exact, semantic (Type-4), and near-duplicate clone families",
    long_about = "nose scans source files, groups duplicated code into clone families,\n\
                  and ranks the results by how useful they are to inspect or refactor.\n\
                  • `nose query <paths>`                  — scan and show a summary with next commands\n\
                  • `nose query <paths> id=<fam> full`    — open one family: every copy + its extraction skeleton\n\
                  • `nose query <paths> base=origin/main` — flag a change applied to one clone copy but not its siblings\n\
                  • `nose query <paths> --fail-on any`    — gate CI (exit non-zero on duplication); add `--format json` for the contract\n\
                  • `nose stats <paths>`                  — language coverage and unsupported syntax\n\
                  • `nose il <file>`                      — inspect why two snippets do or do not match\n\
                  • `nose capabilities`                   — machine-readable integration contract\n\
                  `nose scan` and `nose review` still work but are deprecated in favour of `nose query`."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Research interface for raw unit clone pairs/groups.
    /// Hidden: `scan` is the user-facing command; `detect` is the strict/research
    /// and benchmark interface (`--bench-schema`, `--dump`, …).
    #[command(hide = true)]
    Detect {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Minimum unit line count.
        #[arg(long, default_value_t = 5)]
        min_lines: u32,
        /// Minimum unit token (IL node) count.
        #[arg(long, default_value_t = 24)]
        min_tokens: usize,
        /// Acceptance threshold in `[0,1]`. Defaults: 0.86 on the strict research
        /// path, 0.70 with --candidates. Lower for recall, raise for precision.
        #[arg(long, value_parser = parse_threshold)]
        threshold: Option<f64>,
        /// Candidate mode: disable the behavioral-precision gates and default the
        /// threshold to 0.70. Surfaces near-duplicate FAMILIES (locale classes,
        /// comparison operators, sync/async wrappers) for human review. Use the
        /// default strict path for behavioral-clone research runs.
        #[arg(long)]
        candidates: bool,
        /// MinHash signature length (LSH).
        #[arg(long, default_value_t = 128, value_parser = parse_minhash_k)]
        minhash_k: usize,
        /// LSH band count (more bands → catches lower-similarity candidates).
        #[arg(long, default_value_t = 32, value_parser = parse_bands)]
        bands: usize,
        /// Disable control-flow normalization (ablation).
        #[arg(long)]
        no_cfg_norm: bool,
        /// Enable dead-code / dead-assignment elimination (experimental).
        #[arg(long)]
        dce: bool,
        /// Disable sub-function block units (loops/ifs/try plus exact statement
        /// fragments). Blocks are ON by default — they lift recall and pool-precision
        /// by catching fragment-level clones; `--no-blocks` reverts to
        /// function/method/class units only.
        #[arg(long)]
        no_blocks: bool,
        /// Write the report JSON here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Print a human-readable summary instead of JSON.
        #[arg(long)]
        summary: bool,
        /// Emit predictions in the benchmark schema (needs --repos-root).
        #[arg(long)]
        bench_schema: bool,
        /// Root whose immediate subdirectories are repo ids (for path→repo
        /// mapping when emitting benchmark-schema predictions).
        #[arg(long)]
        repos_root: Option<PathBuf>,
        /// Write diagnostic dump (units.json, candidates.json, predictions.json)
        /// to this directory. Requires --repos-root.
        #[arg(long)]
        dump: Option<PathBuf>,
    },
    /// Rank refactoring candidates as a one-shot report. DEPRECATED — use `nose query`.
    ///
    /// Scans files/directories (respecting .gitignore), groups duplicated code into
    /// clone families, and ranks them by extractability — how cleanly each family
    /// folds into one shared helper. Default channels: `syntax,semantic,near`
    /// (copy-paste runs + exact semantic Type-4 + fuzzy near-duplicates). Passing
    /// --mode replaces that default with exactly the channels listed.
    /// `nose query` reads the same dataset and carries the gate, baselines, and a
    /// structured `--format json` contract; `scan` still works but will be removed later.
    #[command(hide = true)]
    Scan {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// How many top families to show (`0` = all). [default: 30]
        #[arg(long)]
        top: Option<usize>,
        /// Only families with at least this many duplicated sites. [default: 2]
        #[arg(long)]
        min_members: Option<usize>,
        /// Hide families whose refactoring value is below this (noise floor on
        /// large repos). 0 shows every family. [default: 0]
        #[arg(long, value_parser = parse_min_value)]
        min_value: Option<f64>,
        /// Rank families by: `extractability` (how cleanly it folds into one helper —
        /// the default), `value` (raw duplicated volume), `sites` (most copies), or
        /// `hazard` (experimental divergent-edit propensity).
        #[arg(long)]
        sort: Option<SortKey>,
        /// Read defaults from this config file (else `nose.toml`/`.nose.toml`).
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Detection channels to run. Omit for `syntax,semantic,near`. If present,
        /// this replaces the default; pass a comma-list or repeat it, e.g.
        /// `--mode syntax,near` or `--mode syntax --mode semantic`. Fuzzy channels
        /// take an optional acceptance threshold inline: `--mode near:0.8`.
        #[arg(
            long,
            value_delimiter = ',',
            num_args = 1,
            action = clap::ArgAction::Append,
            value_parser = parse_scan_mode,
            value_name = "MODE"
        )]
        mode: Vec<ScanMode>,
        /// Extra views (repeatable / comma-list): `diff` (each family as a unified
        /// diff of its two copies), `proposal` (an extraction skeleton over all copies),
        /// `hotspots` (directories ranked by duplicated lines), `reinvented` (helpers
        /// reimplemented inline instead of called). e.g. `--show diff,hotspots`.
        #[arg(long, value_delimiter = ',', value_name = "VIEW")]
        show: Vec<ShowView>,
        /// Cache per-file analysis under this directory. Re-runs reuse the cache for
        /// unchanged files (keyed by content hash), skipping parse/normalize/extract
        /// — much faster on repeated invocations (CI, pre-commit, iterating).
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,
        /// CI gate — exit non-zero when families are reported: `any` (any reported
        /// family fails) or `new` (only families new/changed vs `--baseline` fail;
        /// requires `--baseline`). e.g. `nose scan src --mode syntax --fail-on any`.
        #[arg(long, value_name = "WHAT")]
        fail_on: Option<FailOn>,
        /// Baseline file of already-accepted families. Families recorded here are
        /// hidden from the report and don't trip `--fail-on`, so a run flags only
        /// *new* duplication — the way to adopt on a codebase that already has clones.
        #[arg(long, value_name = "FILE")]
        baseline: Option<PathBuf>,
        /// Structured ignore file for intentionally suppressed families. Defaults
        /// to `nose.ignore.json` when that file exists.
        #[arg(long, value_name = "FILE")]
        ignore_file: Option<PathBuf>,
        /// Local semantic-pack v0 manifest file or directory to load. Repeatable;
        /// each path is an explicit opt-in and currently contributes provenance metadata only.
        #[arg(long = "semantic-pack", value_name = "FILE_OR_DIR")]
        semantic_pack: Vec<PathBuf>,
        /// Write the current families to the `--baseline` file (accept today's state)
        /// and exit, instead of reporting.
        #[arg(long, requires = "baseline")]
        write_baseline: bool,
        /// Output format.
        #[arg(long, default_value = "human")]
        format: ReportFormat,
        /// Skip files matching a gitignore-style glob (repeatable), e.g.
        /// `--exclude tests --exclude 'vendor/**' --exclude '**/*.generated.ts'`.
        /// (.gitignore is already respected automatically.)
        #[arg(long)]
        exclude: Vec<String>,
        /// Ignore units or syntax copy-paste runs smaller than this size, measured in
        /// IL tokens (the unit's node count). [default: 24]
        #[arg(long)]
        min_size: Option<usize>,
        /// Advanced: also require this many source lines (most users only need
        /// --min-size). [default: 5]
        #[arg(long, hide = true)]
        min_lines: Option<u32>,
        /// Keep only one side of the test boundary: `prod` (drop all-test
        /// families; test↔prod leaks stay), `test` (only all-test families),
        /// or `all` (default). Applies to every output format and `--fail-on`.
        #[arg(long, value_enum, default_value_t = ScopeFilter::All)]
        scope: ScopeFilter,
    },
    /// Scan a path, list duplicated-code families, and drill into the results.
    ///
    /// With no terms, `nose query <path>` prints a summary and runnable next commands.
    /// Add terms to filter (`witness=exact`, `path~api`), group (`group=dir`), sort
    /// (`sort=value`), or open one family (`id=<fam> full`). Carries the analysis flags,
    /// the `--fail-on` CI gate, and a versioned `--format json` contract.
    Query {
        /// Path to a file or directory (recursively scanned).
        #[arg(required = true)]
        path: PathBuf,
        /// Query terms (none → summary): `field=value` `field>N` `field<N`
        /// `path~substr` filter (AND-ed; negate with `field!=value` / `path!~substr`);
        /// `group=FIELD` facet; `id=FAM` or `at=FILE:LINE` open one family (add `full` to
        /// align all copies); `sort=KEY`; `top=N`.
        terms: Vec<String>,
        /// Output format (`human` or `json`).
        #[arg(long, default_value = "human")]
        format: ReportFormat,
        /// Detection channels to run; omit for `syntax,semantic,near`. Pass a comma-list
        /// or repeat the flag; fuzzy channels take an inline threshold (`near:0.8`).
        #[arg(long, value_delimiter = ',')]
        mode: Vec<ScanMode>,
        /// Ignore units smaller than this size, in IL tokens (the unit's node count). [default: 24]
        #[arg(long)]
        min_size: Option<usize>,
        /// Advanced: also require this many source lines (most uses only need --min-size). [default: 5]
        #[arg(long, hide = true)]
        min_lines: Option<u32>,
        /// Hide families whose refactoring value is below this (noise floor on large repos).
        #[arg(long, value_parser = parse_min_value)]
        min_value: Option<f64>,
        /// Keep only families with at least this many duplicated copies. [default: 2]
        #[arg(long)]
        min_members: Option<usize>,
        /// Skip paths matching a gitignore-style glob (repeatable). (.gitignore is already respected.)
        #[arg(long)]
        exclude: Vec<String>,
        /// Cache per-file analysis under this directory; re-runs reuse it for unchanged files.
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,
        /// Structured-ignore file for suppressed families; auto-read `nose.ignore.json` when present.
        #[arg(long, value_name = "FILE")]
        ignore_file: Option<PathBuf>,
        /// Local semantic-pack v0 manifest file or directory to load (repeatable; explicit opt-in).
        #[arg(long = "semantic-pack", value_name = "FILE_OR_DIR")]
        semantic_pack: Vec<PathBuf>,
        /// Read defaults from this config file (else `nose.toml`/`.nose.toml`).
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// CI gate — exit non-zero when default-surface families are reported: `any`, or
        /// `new` (only new/changed vs `--baseline`).
        #[arg(long, value_name = "WHAT")]
        fail_on: Option<FailOn>,
        /// Accepted-baseline file: hide already-recorded families so only new/changed
        /// duplication is shown and gated.
        #[arg(long, value_name = "FILE")]
        baseline: Option<PathBuf>,
        /// Write the current families to `--baseline` (accept today's state) and exit.
        #[arg(long, requires = "baseline")]
        write_baseline: bool,
    },
    /// Flag a change applied to one clone copy but not its siblings (PR/CI check).
    ///
    /// Compares the working tree to a git ref and reports clone families changed
    /// inconsistently in that diff: a copy was edited but its sibling clones were
    /// not — a likely un-propagated change. Needs a git repository. e.g.
    /// `nose review --base origin/main` in CI, or `nose review` for local changes.
    #[command(hide = true)]
    Review {
        /// Paths to scan (recursively). Defaults to the current directory.
        paths: Vec<PathBuf>,
        /// Compare the working tree against this git ref (`origin/main` for a PR branch;
        /// the default `HEAD` reviews uncommitted local changes).
        #[arg(long, default_value = "HEAD")]
        base: String,
        /// Detection channels, like `scan`: `syntax`, `semantic`, `near[:T]` (comma-list
        /// or repeatable). Omit for `syntax,semantic` (review keeps the conservative
        /// mix; `scan`'s default also includes `near`).
        #[arg(
            long,
            value_delimiter = ',',
            num_args = 1,
            action = clap::ArgAction::Append,
            value_parser = parse_scan_mode,
            value_name = "MODE"
        )]
        mode: Vec<ScanMode>,
        /// Ignore units smaller than this size, in IL tokens. [default: 24]
        #[arg(long)]
        min_size: Option<usize>,
        /// Advanced: also require this many source lines. [default: 5]
        #[arg(long, hide = true)]
        min_lines: Option<u32>,
        /// Skip paths matching a gitignore-style glob (repeatable).
        #[arg(long)]
        exclude: Vec<String>,
        /// Read defaults from this config file (else `nose.toml`/`.nose.toml`).
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Structured ignore file for accepted divergences (same format as `scan`).
        /// Defaults to `nose.ignore.json` when it exists.
        #[arg(long, value_name = "FILE")]
        ignore_file: Option<PathBuf>,
        /// Output format.
        #[arg(long, default_value = "human")]
        format: ReportFormat,
        /// Show at most N findings (0 = all). [default: 30]
        #[arg(long)]
        top: Option<usize>,
        /// Exit non-zero when the gate fires (CI gate). What fires is governed
        /// by --fail-on (default: only findings whose change provably touches
        /// lines shared with the un-updated sibling).
        #[arg(long)]
        fail: bool,
        /// Gate tier for --fail: `shared-logic` (default — fire only when the
        /// diff provably touches lines a changed copy shares with its
        /// un-updated sibling) or `any` (fire on every flagged finding).
        #[arg(long, value_enum, default_value_t = review::ReviewFailOn::SharedLogic)]
        fail_on: review::ReviewFailOn,
    },
    /// Recall-ceiling diagnostic: split gold recall across unit-extraction /
    /// candidate-generation stages. (Hidden — benchmark/research tooling.)
    #[command(hide = true)]
    Ceiling {
        #[arg(long)]
        gold: PathBuf,
        #[arg(long)]
        units: PathBuf,
        #[arg(long)]
        candidates: PathBuf,
    },
    /// Score predictions against a gold set (precision/recall/F1, macro, HN-FP).
    /// (Hidden — benchmark/research tooling.)
    #[command(hide = true)]
    Eval {
        /// Gold set JSON.
        #[arg(long)]
        gold: PathBuf,
        /// Predictions JSON (benchmark schema).
        #[arg(long)]
        predictions: PathBuf,
        /// Hard-negatives JSON (precision guard); optional.
        #[arg(long)]
        hard_negatives: Option<PathBuf>,
        /// Corpus JSON with dev/heldout split (for macro F1); optional.
        #[arg(long)]
        corpus: Option<PathBuf>,
    },
    /// Report IL lowering coverage (Raw ratio + top unhandled constructs).
    Stats {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// How many top unhandled surface kinds to list.
        #[arg(long, default_value_t = 30)]
        top: usize,
        /// Output format (`human` or `json`) — the same `--format` contract as `query` and `il`.
        #[arg(long, value_enum, default_value_t = StatsFormat::Human)]
        format: StatsFormat,
    },
    /// Dump the IL for a source file — debug why two snippets do or don't converge.
    Il {
        /// Path to a source file.
        path: PathBuf,
        /// Output format.
        #[arg(long, default_value = "sexpr")]
        format: Format,
        /// Show normalized (canonical) IL instead of raw.
        #[arg(long)]
        normalized: bool,
        /// Disable control-flow normalization (ablation).
        #[arg(long)]
        no_cfg_norm: bool,
    },
    /// Emit the machine-readable capability contract for integrations.
    Capabilities,
    /// Validate semantic-pack v0 manifests and declared conformance fixtures.
    #[command(name = "semantic-pack")]
    SemanticPack {
        #[command(subcommand)]
        cmd: SemanticPackCmd,
    },
    /// Dump per-unit detection features (value-graph / shape / return fingerprints)
    /// as JSON — the raw signal, before candidate generation or thresholding. Lets a
    /// convergence evaluator measure representation-convergence and behavioral-separation
    /// directly on the fingerprints, free of gate/threshold/cluster confounds.
    /// (Hidden — research.)
    #[command(hide = true)]
    Features {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Minimum unit line count.
        #[arg(long, default_value_t = 3)]
        min_lines: u32,
        /// Minimum unit token (IL node) count.
        #[arg(long, default_value_t = 8)]
        min_tokens: usize,
        /// Disable control-flow normalization (ablation).
        #[arg(long)]
        no_cfg_norm: bool,
        /// Disable sub-function block units (loops/ifs/try plus exact statement fragments).
        #[arg(long)]
        no_blocks: bool,
    },
    /// Value-graph coverage census (#391 prevalence probe): per IL construct, how many
    /// `Opaque` fallbacks the value graph mints — the value-graph analog of `stats`'s lowering
    /// `Raw` ratio. Ranks which constructs the fingerprint cannot model (map reads, dynamic
    /// dispatch, …) by unproven mass. JSON only. (Hidden — research.)
    #[command(hide = true)]
    ValueCensus {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Disable control-flow normalization (ablation).
        #[arg(long)]
        no_cfg_norm: bool,
    },
    /// Soundness oracle: verify that value-fingerprint-equal units actually compute
    /// the same thing. Interprets each function on a battery of inputs and reports any
    /// fingerprint-equal pair whose behavior differs (a false merge — the cardinal sin
    /// of a clone detector). Also reports completeness. (Hidden — research.)
    #[command(hide = true)]
    Verify {
        /// Paths to source files or directories (recursively scanned).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Disable control-flow normalization (ablation).
        #[arg(long)]
        no_cfg_norm: bool,
        /// Emit interpretable units as JSON `{units:[{file,start_line,end_line,
        /// behavior,trivial}], exclusions:{...}}` (the oracle's behavioral ground truth
        /// plus fail-closed exclusion counts) instead of the soundness/completeness report.
        /// Used by the value-add evaluator.
        #[arg(long)]
        json: bool,
        /// CI soundness gate: exit non-zero if the false-merge count EXCEEDS this budget.
        /// Use `--max-violations 0` on real code (the SOUND invariant); on the synthetic
        /// Type-4 corpus use the characterized baseline (its residual is oracle-fidelity
        /// artifacts — see experiments §A2), so a new real false merge from a future canon
        /// pushes the count over budget and fails the gate.
        #[arg(long)]
        max_violations: Option<usize>,
        /// Write the oracle's UNDER-MERGED groups (behavior-equal on the battery but
        /// fingerprint-split — candidate MISSED clones) to a JSON file, sorted by structural
        /// nearness. Feeds the detection campaign with oracle-discovered convergence leads
        /// (vj ≥ 0.7 are the strongest: structurally near AND behavior-equal).
        #[arg(long)]
        leads: Option<PathBuf>,
        /// Write a JSON census of the units the oracle could NOT interpret — exclusion
        /// reasons, the construct tags they carry, and how much fingerprint-merge mass
        /// is unverified per construct. The instrument for ranking oracle-coverage work
        /// by unverified-merge mass instead of by guess.
        #[arg(long)]
        exclusion_census: Option<PathBuf>,
        /// Run the falsification-driven distinguishing-input SEARCH (#317) IN ADDITION to the
        /// fixed battery: for each fingerprint-equal group the battery found equal, search a
        /// value-kind-rich input domain (two distinct strings/lists, int32-wrapping ints, float
        /// magnitudes, mined constants) for a row that distinguishes two members — a false merge
        /// the fixed battery's input STARVATION missed. Offline/opt-in; reports the search delta.
        #[arg(long)]
        falsify: bool,
    },
    /// EXPERIMENTAL Type-4 benchmark harness (research tool, not a stable interface).
    ///
    /// Measures a behavioral-equivalence ACCEPTANCE gate: groups interpretable units
    /// by their behavior on an input battery (two units are "accepted" iff identical
    /// on every input) and, against a Type-4 manifest, reports the recall it recovers
    /// BEYOND exact-fingerprint matching and the hard-negative false merges it would
    /// introduce. `--battery wide` is the larger bounded input domain.
    #[command(hide = true)]
    BehavioralGate {
        /// Path to the generated Type-4 corpus `sources/` directory.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// The Type-4 manifest.json with labeled positive/negative pairs.
        #[arg(long)]
        manifest: PathBuf,
        /// Input battery: `standard` (leap 2) or `wide` (leap 3, larger domain).
        #[arg(long, default_value = "standard")]
        battery: BatteryKind,
    },
}

#[derive(Subcommand)]
enum SemanticPackCmd {
    /// Check local semantic-pack v0 manifests for structural conformance.
    Check {
        /// Semantic-pack manifest file or directory of direct `*.json` manifests.
        #[arg(required = true, value_name = "FILE_OR_DIR")]
        paths: Vec<PathBuf>,
        /// Output format.
        #[arg(long, default_value = "human")]
        format: semantic_pack::CheckFormat,
    },
}

#[derive(Clone, Copy, PartialEq, clap::ValueEnum)]
enum BatteryKind {
    Standard,
    Wide,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format {
    Sexpr,
    Json,
}

#[derive(Clone, Copy, PartialEq, Default, clap::ValueEnum)]
enum StatsFormat {
    /// Human-readable coverage table.
    #[default]
    Human,
    /// Machine-readable JSON.
    Json,
}

#[derive(Clone, Copy, PartialEq, clap::ValueEnum)]
pub(crate) enum ReportFormat {
    /// Ranked, human-readable terminal report.
    Human,
    /// Machine-readable JSON report with a versioned top-level schema.
    Json,
    /// Markdown report (for PRs / issues / docs).
    Markdown,
    /// SARIF 2.1.0 (GitHub code-scanning / PR annotations).
    Sarif,
}

/// One `--mode` channel. Fuzzy modes carry their acceptance threshold inline
/// (`near:0.8` / `abstraction:0.5`), so there is no separate `--threshold` flag
/// to mis-combine.
#[derive(Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(try_from = "String")]
enum ScanMode {
    /// CPD-style syntax copy-paste runs (the Type-1/2 floor).
    Syntax,
    /// Exact value-fingerprint Type-4 semantic clones.
    Semantic,
    /// Fuzzy Type-3 near-duplicate candidates, with an optional acceptance threshold.
    Near(Option<f64>),
    /// Experimental weak refactoring-template witnesses over near candidates.
    Abstraction(Option<f64>),
}

impl std::str::FromStr for ScanMode {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        let s = s.trim();
        match s {
            "syntax" => Ok(ScanMode::Syntax),
            "semantic" => Ok(ScanMode::Semantic),
            "near" => Ok(ScanMode::Near(None)),
            "abstraction" => Ok(ScanMode::Abstraction(None)),
            _ => {
                if let Some(t) = s.strip_prefix("near:") {
                    let v: f64 = t
                        .parse()
                        .map_err(|_| format!("invalid near threshold in `{s}`"))?;
                    if !(0.0..=1.0).contains(&v) {
                        return Err(format!("near threshold must be in [0,1], got {v}"));
                    }
                    Ok(ScanMode::Near(Some(v)))
                } else if let Some(t) = s.strip_prefix("abstraction:") {
                    let v: f64 = t
                        .parse()
                        .map_err(|_| format!("invalid abstraction threshold in `{s}`"))?;
                    if !(0.0..=1.0).contains(&v) {
                        return Err(format!("abstraction threshold must be in [0,1], got {v}"));
                    }
                    Ok(ScanMode::Abstraction(Some(v)))
                } else {
                    Err(format!(
                        "unknown mode `{s}` (expected syntax, semantic, near, near:T, abstraction, or abstraction:T)"
                    ))
                }
            }
        }
    }
}

impl TryFrom<String> for ScanMode {
    type Error = String;
    fn try_from(s: String) -> std::result::Result<Self, String> {
        s.parse()
    }
}

/// Borrow a slice of owned `PathBuf`s as `&Path` references — the form the detection entry
/// points take. Used by every scan/refactor subcommand that holds its input paths as a
/// `Vec<PathBuf>`.
fn paths_as_refs(paths: &[PathBuf]) -> Vec<&std::path::Path> {
    paths.iter().map(|p| p.as_path()).collect()
}

fn parse_scan_mode(s: &str) -> std::result::Result<ScanMode, String> {
    s.parse()
}

fn parse_positive_usize(s: &str, label: &str) -> std::result::Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("{label} must be positive"))?;
    if value == 0 {
        Err(format!("{label} must be positive"))
    } else {
        Ok(value)
    }
}

fn parse_minhash_k(s: &str) -> std::result::Result<usize, String> {
    parse_positive_usize(s, "minhash-k")
}

fn parse_bands(s: &str) -> std::result::Result<usize, String> {
    parse_positive_usize(s, "bands")
}

const THRESHOLD_ERROR: &str = "threshold must be a finite number in [0,1]";

fn valid_threshold(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn parse_threshold(s: &str) -> std::result::Result<f64, String> {
    let value = s.parse::<f64>().map_err(|_| THRESHOLD_ERROR.to_string())?;
    if valid_threshold(value) {
        Ok(value)
    } else {
        Err(THRESHOLD_ERROR.to_string())
    }
}

const MIN_VALUE_ERROR: &str = "min-value must be a finite non-negative number";

fn valid_min_value(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn parse_min_value(s: &str) -> std::result::Result<f64, String> {
    let value = s.parse::<f64>().map_err(|_| MIN_VALUE_ERROR.to_string())?;
    if valid_min_value(value) {
        Ok(value)
    } else {
        Err(MIN_VALUE_ERROR.to_string())
    }
}

fn validate_min_value(value: f64) -> Result<f64> {
    if valid_min_value(value) {
        Ok(value)
    } else {
        anyhow::bail!("{MIN_VALUE_ERROR}")
    }
}

/// The `scan` default surface: include unthresholded `near` (experiments §BM:
/// +8.2pp held-out worthy-recall at no held-out P@10 price — consumer 1 filters).
const SCAN_DEFAULT_MODES: &[ScanMode] =
    &[ScanMode::Syntax, ScanMode::Semantic, ScanMode::Near(None)];

/// The `review` default stays the conservative mix: review feeds a gate
/// (consumer 2 — false fires are the failure mode), and §BM priced the scan
/// surface only. Revisit with the fire-precision benchmark (#243/#245).
const REVIEW_DEFAULT_MODES: &[ScanMode] = &[ScanMode::Syntax, ScanMode::Semantic];

#[derive(Clone, Copy)]
struct ScanChannels {
    syntax: bool,
    semantic: bool,
    near: bool,
    abstraction: bool,
    /// The shared fuzzy acceptance threshold, if one was given in the mode spec.
    threshold: Option<f64>,
}

impl ScanChannels {
    fn resolve(cli: Vec<ScanMode>, cfg: Vec<ScanMode>, default: &[ScanMode]) -> Result<Self> {
        let selected = if !cli.is_empty() {
            cli
        } else if !cfg.is_empty() {
            cfg
        } else {
            default.to_vec()
        };
        let mut channels = ScanChannels {
            syntax: false,
            semantic: false,
            near: false,
            abstraction: false,
            threshold: None,
        };
        for mode in selected {
            match mode {
                ScanMode::Syntax => channels.syntax = true,
                ScanMode::Semantic => channels.semantic = true,
                ScanMode::Near(t) => {
                    channels.near = true;
                    channels.set_threshold("near", t)?;
                }
                ScanMode::Abstraction(t) => {
                    channels.abstraction = true;
                    channels.set_threshold("abstraction", t)?;
                }
            }
        }
        if !channels.syntax && !channels.semantic && !channels.near && !channels.abstraction {
            anyhow::bail!(
                "--mode must include at least one of syntax, semantic, near, or abstraction"
            );
        }
        Ok(channels)
    }

    fn set_threshold(&mut self, mode: &'static str, threshold: Option<f64>) -> Result<()> {
        let Some(next) = threshold else {
            return Ok(());
        };
        if let Some(prev) = self.threshold {
            if (prev - next).abs() > f64::EPSILON {
                anyhow::bail!(
                    "conflicting --mode thresholds: near and abstraction share one acceptance threshold; got {prev} and {mode}:{next}"
                );
            }
        }
        self.threshold = Some(next);
        Ok(())
    }

    fn structural(self) -> bool {
        self.semantic || self.near || self.abstraction
    }

    fn report_label(self, count: usize) -> &'static str {
        let singular = count == 1;
        match (
            self.syntax,
            self.semantic,
            self.near,
            self.abstraction,
            singular,
        ) {
            (true, false, false, false, true) => "syntax clone family",
            (true, false, false, false, false) => "syntax clone families",
            (false, true, false, false, true) => "semantic clone family",
            (false, true, false, false, false) => "semantic clone families",
            (false, false, true, false, true) => "near-duplicate family",
            (false, false, true, false, false) => "near-duplicate families",
            (false, false, false, true, true) => "abstraction candidate family",
            (false, false, false, true, false) => "abstraction candidate families",
            (_, _, _, _, true) => "clone family",
            (_, _, _, _, false) => "clone families",
        }
    }

    fn markdown_title(self) -> &'static str {
        match (self.syntax, self.semantic, self.near, self.abstraction) {
            (true, false, false, false) => "Syntax Clone Families",
            (false, true, false, false) => "Semantic Clone Families",
            (false, false, true, false) => "Near-Duplicate Families",
            (false, false, false, true) => "Abstraction Candidate Families",
            _ => "Clone Families",
        }
    }

    fn threshold(self) -> f64 {
        if self.near || self.abstraction {
            self.threshold.unwrap_or(if self.abstraction && !self.near {
                0.50
            } else {
                0.70
            })
        } else {
            1.0
        }
    }

    fn abstraction_only(self) -> bool {
        self.abstraction && !self.syntax && !self.semantic && !self.near
    }
}

/// How to rank families — what "most worth your attention first" means.
#[derive(Clone, Copy, PartialEq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SortKey {
    /// How cleanly it extracts: invariant (shared) lines × copies × spread, penalized
    /// by the number of parameters the helper would need. Surfaces the duplication you
    /// can actually fold into one helper, not the biggest block that merely *looks*
    /// similar (a *fixability* axis). The default.
    Extractability,
    /// Raw duplicated volume: duplicated lines (mean span × copies) × similarity ×
    /// spread. Ranks by how much *code* repeats, NOT by the `removable` field — a
    /// structural (Type-4) family can have high volume yet `removable=0` when no
    /// literal lines survive across all copies (nothing cleanly extractable).
    Value,
    /// Most copies first — the most-repeated patterns.
    Sites,
    /// Divergent-edit *hazard*: how likely a family is to be edited inconsistently
    /// (one copy fixed, the siblings missed) and cause a bug. A severity axis, not a
    /// fixability one — surfaces copies that share little text yet are behaviorally the
    /// same (the invisible siblings a developer won't update). Calibrated against mined
    /// history as a *divergence-propensity* signal — it is NOT yet a validated *harm*
    /// ranker (an LLM-gold audit found ~chance harm discrimination); see
    /// `docs/hazard-ranking.md`. Opt-in via `--sort hazard`.
    Hazard,
}

impl SortKey {
    fn json_name(self) -> &'static str {
        match self {
            SortKey::Extractability => "extractability",
            SortKey::Value => "value",
            SortKey::Sites => "sites",
            SortKey::Hazard => "hazard",
        }
    }

    /// The ranking score for `f` under this key (higher = ranked first).
    fn score(self, f: &nose_detect::RefactorFamily) -> f64 {
        match self {
            SortKey::Extractability => f.extractability(),
            SortKey::Value => f.value,
            SortKey::Sites => f.members as f64,
            SortKey::Hazard => f.hazard(),
        }
    }
}

/// Plain-language name of the active ranking, shown once in the header (the per-family
/// lines don't repeat a cryptic score — the order already conveys it).
fn sort_name(s: SortKey) -> &'static str {
    match s {
        SortKey::Extractability => "extractability (cleanest to fold into one helper)",
        SortKey::Value => "raw duplicated volume",
        SortKey::Sites => "number of copies",
        SortKey::Hazard => "divergent-edit hazard (most likely to be edited inconsistently)",
    }
}

/// CI fail-gate policy, selected with `--fail-on`.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FailOn {
    /// Any reported family (after filters) fails the run.
    Any,
    /// Only families new or changed vs `--baseline` fail. Requires `--baseline`.
    New,
}

/// Extra per-report views (human/markdown), selected with `--show`.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ShowView {
    /// Each family inline as a unified diff between its two representative copies.
    Diff,
    /// An extraction skeleton per family: the shared structure with varying spots as ⟨param N⟩.
    Proposal,
    /// After the report, directories ranked by total duplicated lines.
    Hotspots,
    /// Reinvented-helper containment findings: code that reimplements an existing pure
    /// helper inline instead of calling it (exact-grade; experimental surface).
    Reinvented,
}

/// What `scan` actually looked at: the file count and per-language breakdown, shown as
/// a one-line header. A repo where `.gitignore`/`--exclude` pruned vendored deps scans
/// far fewer files than sit on disk; surfacing the count (and which languages) makes
/// that scope visible instead of a silent gap the reader has to guess at. (The *ignored*
/// count is deliberately not shown — computing it means descending into the very trees
/// `.gitignore` exists to skip, slow on exactly the big repos where it matters.)
struct ScanScope {
    files: usize,
    /// `(language name, file count)`, largest first.
    langs: Vec<(&'static str, usize)>,
}

impl ScanScope {
    fn from_corpus(corpus: &Corpus) -> Self {
        let mut counts: std::collections::HashMap<&'static str, usize> =
            std::collections::HashMap::new();
        for f in &corpus.files {
            *counts.entry(f.meta.lang.name()).or_insert(0) += 1;
        }
        let mut langs: Vec<(&'static str, usize)> = counts.into_iter().collect();
        // Largest language first; name as a stable tie-break for deterministic output.
        langs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        ScanScope {
            files: corpus.files.len(),
            langs,
        }
    }

    /// `scanned 1113 files · typescript 900 · tsx 213` (languages omitted when unknown).
    fn summary(&self) -> String {
        let unit = if self.files == 1 { "file" } else { "files" };
        if self.langs.is_empty() {
            return format!("scanned {} {unit}", self.files);
        }
        let langs = self
            .langs
            .iter()
            .map(|(l, n)| format!("{l} {n}"))
            .collect::<Vec<_>>()
            .join(" · ");
        format!("scanned {} {unit} · {langs}", self.files)
    }
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

struct ScanArgs {
    paths: Vec<PathBuf>,
    top: Option<usize>,
    min_members: Option<usize>,
    min_value: Option<f64>,
    sort: Option<SortKey>,
    config: Option<PathBuf>,
    mode: Vec<ScanMode>,
    show: Vec<ShowView>,
    cache_dir: Option<PathBuf>,
    fail_on: Option<FailOn>,
    baseline: Option<PathBuf>,
    ignore_file: Option<PathBuf>,
    semantic_pack: Vec<PathBuf>,
    write_baseline: bool,
    format: ReportFormat,
    exclude: Vec<String>,
    min_size: Option<usize>,
    min_lines: Option<u32>,
    scope: ScopeFilter,
}

/// `--scope`: which test-boundary side of the report to keep. An explicit
/// consumer choice (issue #264 asked to read production findings first), not a
/// worthiness call — the rubric's "location never excuses duplication" governs
/// labels, while this governs what one invocation displays and gates on.
#[derive(Clone, Copy, PartialEq, Default, clap::ValueEnum)]
enum ScopeFilter {
    /// Everything (the default).
    #[default]
    All,
    /// Drop all-test families; keep prod and mixed (a test↔prod leak is prod's
    /// problem).
    Prod,
    /// Only all-test families (e.g. hunting scaffolding to consolidate).
    Test,
}

impl ScopeFilter {
    fn keeps(self, family: &nose_detect::RefactorFamily) -> bool {
        match self {
            ScopeFilter::All => true,
            ScopeFilter::Prod => family.scope != "test",
            ScopeFilter::Test => family.scope == "test",
        }
    }
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
fn enrich_witnesses_for_format(
    families: &mut [nose_detect::RefactorFamily],
    opts: &nose_detect::DetectOptions,
    format: ReportFormat,
) {
    if matches!(format, ReportFormat::Json) {
        time_stage("enrich", || enrich_graded_witnesses(families, opts));
    }
}

/// Enrich the graded witness on exactly the families scan JSON serializes — the top
/// `limit` shown families plus every ignored family — and nothing else. The witness is
/// JSON-only and computed per family, so the thousands of lower-ranked near families JSON
/// never prints can skip it (netty `--top 50`: ~2.3s → a fraction). `--top 0` sets
/// `limit == families.len()`, so it still enriches everything — the full-audit contract.
/// Human/SARIF skip entirely (they don't render the witness).
fn enrich_serialized_witnesses(
    families: &mut [nose_detect::RefactorFamily],
    ignored: &mut [IgnoredFamily],
    limit: usize,
    opts: &nose_detect::DetectOptions,
    format: ReportFormat,
) {
    enrich_witnesses_for_format(&mut families[..limit], opts, format);
    enrich_ignored_witnesses_for_format(ignored, opts, format);
}

/// Enrich the graded witness on ignored families, which scan JSON serializes under
/// `ignored_families` (so an audit of the ignore set sees the same evidence as an active
/// family). They are wrapped in [`IgnoredFamily`], so enrich each in place; the set is the
/// user's ignore list — typically small, often empty — so the lost batching is immaterial.
fn enrich_ignored_witnesses_for_format(
    ignored: &mut [IgnoredFamily],
    opts: &nose_detect::DetectOptions,
    format: ReportFormat,
) {
    if !matches!(format, ReportFormat::Json) {
        return;
    }
    time_stage("enrich_ignored", || {
        for fam in ignored {
            enrich_graded_witnesses(std::slice::from_mut(&mut fam.family), opts);
        }
    });
}

pub(crate) fn enrich_graded_witnesses(
    families: &mut [nose_detect::RefactorFamily],
    opts: &nose_detect::DetectOptions,
) {
    use std::collections::HashMap;
    let is_near = |f: &nose_detect::RefactorFamily| {
        f.languages == 1
            && f.locations.len() >= 2
            && f.witness.as_ref().map(|w| w.kind) == Some("structural-similarity")
    };
    if !families.iter().any(is_near) {
        return;
    }
    // The representative spans needed, grouped by source file.
    let mut wanted: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
    for f in families.iter().filter(|f| is_near(f)) {
        for loc in [&f.locations[0], &f.locations[1]] {
            wanted
                .entry(loc.file.clone())
                .or_default()
                .push((loc.start_line, loc.end_line));
        }
    }
    // Lower each needed file once; export the value DAGs of its requested unit spans.
    let mut dags: HashMap<(String, (u32, u32)), (nose_normalize::ValueDag, bool)> = HashMap::new();
    for (file, spans) in &wanted {
        let Some(lang) = Lang::from_path(file) else {
            continue;
        };
        let Ok(src) = std::fs::read(file) else {
            continue;
        };
        let interner = Interner::new();
        let Ok(il) = nose_frontend::lower_source(FileId(0), file, &src, lang, &interner) else {
            continue;
        };
        let mut uniq = spans.clone();
        uniq.sort_unstable();
        uniq.dedup();
        let exported = nose_detect::unit_dags_at(&il, &interner, opts, &uniq);
        for (span, dag) in uniq.into_iter().zip(exported) {
            if let Some(dag) = dag {
                dags.insert((file.clone(), span), dag);
            }
        }
    }
    // Compute and attach each family's witness, filling hole source text.
    let mut lines = FileLineCache::default();
    for f in families.iter_mut().filter(|f| is_near(f)) {
        let a_file = f.locations[0].file.clone();
        let a_lines = (f.locations[0].start_line, f.locations[0].end_line);
        let b_file = f.locations[1].file.clone();
        let b_lines = (f.locations[1].start_line, f.locations[1].end_line);
        let (Some((da, a_exact)), Some((db, b_exact))) = (
            dags.get(&(a_file.clone(), a_lines)),
            dags.get(&(b_file.clone(), b_lines)),
        ) else {
            continue;
        };
        let Some(mut witness) = nose_detect::graded_witness(da, db, !a_exact, !b_exact) else {
            continue;
        };
        for hole in &mut witness.spots {
            if let Some((s, e)) = hole.a_lines {
                hole.a_text = witness_spot_text(&mut lines, &a_file, s, e);
            }
            if let Some((s, e)) = hole.b_lines {
                hole.b_text = witness_spot_text(&mut lines, &b_file, s, e);
            }
        }
        // Definition-site modifiers (decorators/attributes) are erased at lowering, so
        // the value graph cannot see a `@deco(x)` vs `@deco(y)` difference. Compare them
        // from source here: if the two copies' decorator/attribute lines differ, the
        // bodies being equal-modulo-holes is NOT the whole story — record the difference
        // as a hole and demote the claim (fail-closed). Identical decorators leave the
        // witness untouched.
        let lang = f.locations[0].lang.as_str();
        let a_decos = decorator_lines(&mut lines, lang, &a_file, a_lines.0, a_lines.1);
        let b_decos = decorator_lines(&mut lines, lang, &b_file, b_lines.0, b_lines.1);
        if let Some((a_only, b_only)) = decorator_difference(&a_decos, &b_decos) {
            witness.spots.push(nose_detect::WitnessHole {
                class: "decorator",
                a_lines: None,
                b_lines: None,
                effect: false,
                a_text: cap_join(&a_only),
                b_text: cap_join(&b_only),
            });
            witness.holes += 1;
            witness.equal_modulo_holes = false;
            if !witness.patterns.contains(&"decorator-differs") {
                witness.patterns.push("decorator-differs");
            }
        }
        if let Some(w) = f.witness.as_mut() {
            w.graded = Some(witness);
        }
    }
}

/// The line prefix that marks a definition-site modifier in `lang`: `@` for the
/// decorator/annotation languages, `#[` for Rust attributes. `None` for languages with
/// no such syntax — crucially Ruby, where a leading `@` is an *instance variable*
/// (`@token = …`), not a decorator, so it must NOT be treated as one.
fn decorator_prefix(lang: &str) -> Option<&'static str> {
    match lang {
        "python" | "java" | "javascript" | "typescript" => Some("@"),
        "rust" => Some("#["),
        _ => None,
    }
}

/// The sorted decorator/attribute lines inside a unit's source span. These modify
/// behavior at the definition site but their arguments are dropped at lowering, so the
/// value graph is blind to them (a nested `@click.argument("x")` vs
/// `@click.argument("x", metavar="m")` produces the same IL). Comparing the source text
/// is the only place the difference is visible.
fn decorator_lines(
    lines: &mut FileLineCache,
    lang: &str,
    file: &str,
    start: u32,
    end: u32,
) -> Vec<String> {
    let Some(prefix) = decorator_prefix(lang) else {
        return Vec::new();
    };
    let Some(slice) = lines.slice(file, start, end) else {
        return Vec::new();
    };
    let mut out: Vec<String> = slice
        .iter()
        .map(|l| l.trim())
        .filter(|l| l.starts_with(prefix))
        .map(str::to_string)
        .collect();
    out.sort();
    out
}

/// Multiset difference of two decorator-line lists: `Some((a_only, b_only))` when they
/// differ, `None` when identical.
fn decorator_difference(a: &[String], b: &[String]) -> Option<(Vec<String>, Vec<String>)> {
    let mut b_remaining: Vec<&String> = b.iter().collect();
    let mut a_only = Vec::new();
    for d in a {
        if let Some(pos) = b_remaining.iter().position(|x| *x == d) {
            b_remaining.remove(pos);
        } else {
            a_only.push(d.clone());
        }
    }
    let b_only: Vec<String> = b_remaining.into_iter().cloned().collect();
    (!a_only.is_empty() || !b_only.is_empty()).then_some((a_only, b_only))
}

/// Join lines with a visible separator, capped on a char boundary (witness hole text).
fn cap_join(lines: &[String]) -> String {
    const CAP: usize = 160;
    let joined = lines.join(" ⏎ ");
    if joined.len() > CAP {
        let mut end = CAP;
        while !joined.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &joined[..end])
    } else {
        joined
    }
}

/// Trimmed, length-capped source text of lines `start..=end` of `file`, for a witness
/// hole. Multi-line spots are joined with a visible separator; the result is capped on
/// a char boundary so the JSON stays compact.
fn witness_spot_text(lines: &mut FileLineCache, file: &str, start: u32, end: u32) -> String {
    const TEXT_CAP: usize = 160;
    let Some(slice) = lines.slice(file, start, end) else {
        return String::new();
    };
    let joined = slice
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ⏎ ");
    let joined = joined.trim();
    if joined.len() > TEXT_CAP {
        let mut end = TEXT_CAP;
        while !joined.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &joined[..end])
    } else {
        joined.to_string()
    }
}

fn write_scan_baseline(args: &ScanArgs, families: &[nose_detect::RefactorFamily]) -> Result<()> {
    let path = args
        .baseline
        .as_ref()
        .expect("--write-baseline requires --baseline");
    baseline::write(path, families, family_hint)
        .with_context(|| format!("writing baseline {}", path.display()))?;
    eprintln!(
        "nose: wrote baseline of {} families to {}",
        families.len(),
        path.display()
    );
    Ok(())
}

/// Compare against an accepted baseline: build the comparison, then drop already-accepted
/// families in place so only new/changed duplication is reported and gated. `None` when no
/// `--baseline` is set. (`--write-baseline` is handled earlier, before this runs.)
fn apply_scan_baseline(
    args: &ScanArgs,
    families: &mut Vec<nose_detect::RefactorFamily>,
) -> Result<Option<BaselineComparison>> {
    let Some(path) = args.baseline.as_ref() else {
        return Ok(None);
    };
    let accepted = baseline::load(path)?;
    let comparison = BaselineComparison::new(path, &accepted, families);
    families.retain(|f| !accepted.keys.contains(&baseline::family_key(f)));
    Ok(Some(comparison))
}

/// `since=<baseline>`: compare to a saved snapshot WITHOUT hiding anything — the temporal
/// exploration lens. Unlike `--baseline` (which drops accepted families for the gate), this
/// keeps every family and lets the caller slice by the `status` field. `nose query <path>
/// since=B status=new --fail-on any` is the composable equivalent of `--baseline B --fail-on
/// new`; the two baseline paths converge as the gate folds into query (the review-unification).
fn compare_since(
    path: &str,
    families: &[nose_detect::RefactorFamily],
) -> Result<BaselineComparison> {
    let snapshot = baseline::load(std::path::Path::new(path))?;
    Ok(BaselineComparison::new(
        std::path::Path::new(path),
        &snapshot,
        families,
    ))
}

/// A family's status against a `since=` snapshot: `new`/`changed` (in the comparison) or
/// `unchanged` (present in the snapshot, so absent from the changed/new map).
fn family_status(f: &nose_detect::RefactorFamily, cmp: &BaselineComparison) -> &'static str {
    cmp.statuses
        .get(&baseline::family_key(f))
        .map_or("unchanged", BaselineStatus::as_str)
}

fn partition_ignored(
    families: Vec<nose_detect::RefactorFamily>,
    ignore_set: Option<&ignores::IgnoreSet>,
) -> (Vec<nose_detect::RefactorFamily>, Vec<IgnoredFamily>) {
    let Some(ignore_set) = ignore_set else {
        return (families, Vec::new());
    };
    let mut active = Vec::with_capacity(families.len());
    let mut ignored_families = Vec::new();
    for family in families {
        if let Some(ignore) = ignore_set.match_family(&family) {
            ignored_families.push(IgnoredFamily { family, ignore });
        } else {
            active.push(family);
        }
    }
    (active, ignored_families)
}

/// Everything the format arms need to render one scan's report.
struct ScanReportView<'a> {
    scope: &'a ScanScope,
    settings: &'a ScanSettings,
    reinvented: &'a [nose_detect::ReinventedHelper],
    families: &'a [nose_detect::RefactorFamily],
    shown: &'a [&'a nose_detect::RefactorFamily],
    reportable: &'a [&'a nose_detect::RefactorFamily],
    shown_reportable: &'a [&'a nose_detect::RefactorFamily],
    baseline: Option<&'a BaselineComparison>,
    ignored_families: &'a [IgnoredFamily],
    omitted_note: Option<&'a str>,
    overrides: &'a SurfaceOverrides,
    opportunities: &'a OpportunityGroups,
}

fn render_scan_report(args: &ScanArgs, view: &ScanReportView) -> Result<()> {
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

fn enforce_scan_fail_on(
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

fn print_hotspots_refs(families: &[&nose_detect::RefactorFamily]) {
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

/// Time a named CLI-side stage under `NOSE_TIME` (the in-pipeline detector stages
/// report themselves; this covers post-detection CLI work — lowering, the graded-witness
/// enrichment — that is otherwise invisible).
fn time_stage<T>(label: &str, f: impl FnOnce() -> T) -> T {
    if std::env::var_os("NOSE_TIME").is_none() {
        return f();
    }
    let t0 = std::time::Instant::now();
    let out = f();
    eprintln!(
        "  [time] {:<12} {:>7.1}ms",
        label,
        t0.elapsed().as_secs_f64() * 1e3
    );
    out
}

/// Run the corpus discover+parse+lower step, printing its wall time under
/// `NOSE_TIME` (the in-pipeline stages report themselves; this covers the
/// frontend, which usually dominates and is otherwise invisible).
fn time_lower<T>(f: impl FnOnce() -> T) -> T {
    time_stage("lower", f)
}

fn total_dup_lines_refs(fs: &[&nose_detect::RefactorFamily]) -> u32 {
    fs.iter().map(|f| f.dup_lines).sum()
}

/// Overlap grouping (issues #263/#264): families whose members are
/// overlapping slices of the same source regions are one refactoring
/// *opportunity*, not several. The primary (best-ranked) family keeps its
/// numbered entry; its slices fold into a one-line note under it and carry
/// `overlap_primary_id` in JSON. Grouping is presentation policy: every
/// family stays in JSON, baselines, ignores, and `--fail-on` exactly as
/// before.
#[derive(Default)]
struct OpportunityGroups {
    /// Slice family id → its primary's family id.
    primary_of: std::collections::HashMap<String, String>,
    /// Primary family id → slice family ids, in rank order.
    slices_of: std::collections::HashMap<String, Vec<String>>,
}

impl OpportunityGroups {
    /// Group `families` (already in rank order — the first family of a group
    /// is its primary). Two families join when at least two distinct member
    /// pairs overlap on the same file by ≥ half of the shorter span: one
    /// shared region can be coincidence, two parallel shared regions are the
    /// same opportunity sliced (the craken-cli shape — six families, two
    /// insights). Conservative by construction: 2-member families must
    /// overlap on *both* members.
    fn from_ranked(families: &[&nose_detect::RefactorFamily]) -> Self {
        // A file listing implausibly many families would make candidate
        // generation quadratic; skip it rather than risk the scan's speed.
        const PER_FILE_CAP: usize = 200;
        let mut by_file: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, f) in families.iter().enumerate() {
            let mut files: Vec<&str> = f.locations.iter().map(|l| l.file.as_str()).collect();
            files.sort_unstable();
            files.dedup();
            for file in files {
                by_file.entry(file).or_default().push(i);
            }
        }
        let mut candidates = std::collections::BTreeSet::new();
        for idxs in by_file.values().filter(|v| v.len() <= PER_FILE_CAP) {
            for (p, &i) in idxs.iter().enumerate() {
                for &j in &idxs[p + 1..] {
                    candidates.insert((i.min(j), i.max(j)));
                }
            }
        }
        // Union-find keyed so each set's root is its smallest (best-ranked)
        // index — that root is the opportunity's primary.
        let mut parent: Vec<usize> = (0..families.len()).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        for (i, j) in candidates {
            if overlapping_member_pairs(families[i], families[j]) >= 2 {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                let (lo, hi) = (ri.min(rj), ri.max(rj));
                parent[hi] = lo;
            }
        }
        let mut groups = Self::default();
        for i in 0..families.len() {
            let root = find(&mut parent, i);
            if root != i {
                let primary = baseline::family_id(families[root]);
                let slice = baseline::family_id(families[i]);
                groups.primary_of.insert(slice.clone(), primary.clone());
                groups.slices_of.entry(primary).or_default().push(slice);
            }
        }
        groups
    }

    fn is_slice(&self, family: &nose_detect::RefactorFamily) -> bool {
        self.primary_of.contains_key(&baseline::family_id(family))
    }

    fn slices(&self, family: &nose_detect::RefactorFamily) -> Option<&[String]> {
        self.slices_of
            .get(&baseline::family_id(family))
            .map(Vec::as_slice)
    }
}

/// Greedy one-to-one count of member pairs that overlap on the same file by
/// at least half of the shorter span.
fn overlapping_member_pairs(
    a: &nose_detect::RefactorFamily,
    b: &nose_detect::RefactorFamily,
) -> usize {
    let mut used = vec![false; b.locations.len()];
    let mut pairs = 0;
    for la in &a.locations {
        for (j, lb) in b.locations.iter().enumerate() {
            if used[j] || la.file != lb.file {
                continue;
            }
            let lo = la.start_line.max(lb.start_line);
            let hi = la.end_line.min(lb.end_line);
            if lo > hi {
                continue;
            }
            let overlap = hi - lo + 1;
            let len_a = la.end_line - la.start_line + 1;
            let len_b = lb.end_line - lb.start_line + 1;
            if overlap * 2 >= len_a.min(len_b) {
                used[j] = true;
                pairs += 1;
                break;
            }
        }
    }
    pairs
}

/// Build a SARIF 2.1.0 document — one result per family, every member site a
/// location so GitHub code-scanning annotates each. The first location is primary;
/// the rest are `relatedLocations`.
/// `shown` is the (possibly `--top`-truncated) slice that gets emitted; `total` is the
/// full active-family count before truncation. A SARIF consumer (GitHub code scanning)
/// otherwise can't tell a truncated upload from a complete one, so the run carries both
/// counts in `properties` and — when families were hidden — a `note` notification telling
/// the reader to pass `--top 0` for the full set.
fn refactor_sarif(shown: &[&nose_detect::RefactorFamily], total: usize) -> Result<String> {
    use serde_json::json;
    let phys = |l: &nose_detect::Loc| {
        json!({
            "physicalLocation": {
                "artifactLocation": { "uri": l.file },
                "region": { "startLine": l.start_line, "endLine": l.end_line }
            }
        })
    };
    let results: Vec<_> = shown
        .iter()
        .map(|f| {
            let msg = format!(
                "{} — {} sites, {} {}, ~{} duplicated lines (sim {:.2})",
                family_hint(f),
                f.members,
                f.files,
                plural(f.files, "file", "files"),
                f.dup_lines,
                f.mean_score
            );
            json!({
                "ruleId": "duplicate-family",
                "level": "warning",
                "message": { "text": msg },
                "locations": f.locations.first().map(phys).into_iter().collect::<Vec<_>>(),
                "relatedLocations": f.locations.iter().skip(1).map(phys).collect::<Vec<_>>(),
                "properties": { "family_id": baseline::family_id(f) },
            })
        })
        .collect();
    let mut run = json!({
        "tool": { "driver": {
            "name": "nose",
            "informationUri": "https://github.com/",
            "version": env!("CARGO_PKG_VERSION"),
            "rules": [{
                "id": "duplicate-family",
                "name": "DuplicateFamily",
                "shortDescription": { "text": "Duplicated code worth refactoring" }
            }]
        }},
        "results": results,
        "properties": { "total_families": total, "shown_families": shown.len() },
    });
    if shown.len() < total {
        run["invocations"] = json!([{
            "executionSuccessful": true,
            "toolExecutionNotifications": [{
                "level": "note",
                "message": { "text": format!(
                    "Showing {} of {total} clone families (the --top limit). \
                     Pass --top 0 to emit every family.",
                    shown.len()
                ) }
            }]
        }]);
    }
    let doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [run],
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Distinct languages in a family, sorted — e.g. `"python, typescript"`. Empty
/// when the family is single-language (caller decides whether to show anything).
fn family_langs(f: &nose_detect::RefactorFamily) -> String {
    if f.languages <= 1 {
        return String::new();
    }
    let mut langs: Vec<&str> = f.locations.iter().map(|l| l.lang.as_str()).collect();
    langs.sort_unstable();
    langs.dedup();
    langs.join(", ")
}

/// A short, fact-grounded refactoring hint for a family — only from signals the
/// report already establishes (a shared symbol name, cross-language spread, the
/// number of directories), never a guess about semantics.
fn family_hint(f: &nose_detect::RefactorFamily) -> String {
    use nose_il::UnitKind;
    // Exactly one member is a whole named function/method while every other
    // member is an inline block or fragment: the family itself proves the
    // inline copies compute what the existing helper computes (issue #263's
    // local-`clamp` case) — the action is "call it", not "extract a second
    // one". Stronger and safer than a fresh extraction, so it wins the hint.
    let named_units: Vec<&nose_detect::Loc> = f
        .locations
        .iter()
        .filter(|l| {
            matches!(l.kind, UnitKind::Function | UnitKind::Method)
                && l.name.is_some()
                && !l.is_fragment
        })
        .collect();
    let inline_copies = f
        .locations
        .iter()
        .filter(|l| l.kind == UnitKind::Block || l.is_fragment)
        .count();
    // Coevo C2 guards: never point production copies at a helper that lives
    // in test code (tests may call prod, not the reverse), and never
    // recommend calling into a generated file (not the maintainer's API).
    if let [helper] = named_units[..] {
        let helper_callable =
            !helper.looks_generated && (f.scope == "test" || !nose_detect::is_test_loc(helper));
        if helper_callable && inline_copies >= 1 && inline_copies == f.locations.len() - 1 {
            let name = helper.name.as_deref().unwrap_or("the helper");
            let sites = if inline_copies == 1 {
                "1 site reimplements".to_string()
            } else {
                format!("{inline_copies} sites reimplement")
            };
            let base = format!(
                "{sites} `{name}` — call the existing helper ({})",
                helper.file
            );
            // Series 2: many varying spots mean the copies diverge from the
            // helper — the early return must not bypass the caution.
            return if f.params >= HIGH_PARAM_SPOTS && f.languages == 1 {
                format!(
                    "{base} — high-parameter ({} varying spots): verify the \
                     copies really match the helper before swapping in calls",
                    f.params
                )
            } else {
                base
            };
        }
    }

    // If every named site shares one identifier, it's the same thing copied.
    let mut names = f.locations.iter().filter_map(|l| l.name.as_deref());
    let shared_name = names.next().filter(|first| {
        f.locations.iter().filter(|l| l.name.is_some()).count() == f.members
            && names.all(|n| n == *first)
    });

    let cross = if f.languages > 1 {
        " (cross-language)"
    } else {
        ""
    };
    // The unit that all/most sites are: classes → a base class/mixin; blocks → a
    // method extracted from the repeated region; functions/methods → a helper.
    let all_classes = f.locations.iter().all(|l| l.kind == UnitKind::Class);
    let all_blocks = f.locations.iter().all(|l| l.kind == UnitKind::Block);
    // A computation-poor "class" unit is really a type/interface/enum/schema
    // declaration (lowered to a `Class` skeleton); its refactor is "move to one shared
    // type", not "extract a function with parameters".
    let type_decl = all_classes && f.mean_sem < 12.0;
    let extract = if let Some(origin_hint) = origin_extract_hint(f) {
        origin_hint
    } else if type_decl {
        "consolidate into one shared type"
    } else if all_classes {
        "extract a shared base class / mixin"
    } else if all_blocks {
        "extract a method from the repeated block"
    } else {
        "extract a helper"
    };

    let base = match (shared_name, f.modules) {
        (Some(name), _) => format!("consolidate `{name}` — {} copies{cross}", f.members),
        (None, m) if m >= 3 && all_classes => {
            format!("repeated across {m} directories — {extract}{cross}")
        }
        (None, m) if m >= 3 => {
            format!("repeated across {m} directories — extract a shared abstraction{cross}")
        }
        (None, m) if m >= 2 => format!("duplicated across {m} directories — {extract}{cross}"),
        (None, _) => format!("local duplication — {extract}{cross}"),
    };
    // "Extract a method" overclaims when the helper would take many parameters
    // (issue #264 hit 6–16 varying spots): keep the fact-grounded action but
    // flag the readability price instead of asserting a clean extraction.
    if f.params >= HIGH_PARAM_SPOTS && f.languages == 1 {
        return format!(
            "{base} — high-parameter ({} varying spots): review readability; \
             a smaller helper for the invariant core may fit better",
            f.params
        );
    }
    // Test-scope duplication is a real smell, but Arrange/Act/Assert setup is often
    // duplicated on purpose — extracting it can hide each scenario's intent (issue
    // #264). Flag that triage caveat without asserting a verdict; the worthy
    // fixture-vs-scaffold call is the reader's (and is not feature-decidable — see the
    // default-surface-noise-audit). `mixed` (a test↔prod leak) is a real extract, no caveat.
    if f.scope == "test" {
        return format!(
            "{base} — test scaffolding: consolidate only a genuinely shared fixture/helper, \
             not per-scenario setup"
        );
    }
    base
}

fn origin_extract_hint(f: &nose_detect::RefactorFamily) -> Option<&'static str> {
    use nose_il::{UnitBodyKind, UnitDomain, UnitSubkind};

    if f.locations.iter().all(|loc| loc.origin.is_unknown()) {
        return None;
    }
    let all_have_domain = |domain| f.locations.iter().all(|loc| loc.origin.has_domain(domain));
    let all_subkind = |subkind| f.locations.iter().all(|loc| loc.origin.subkind == subkind);
    let any_body = |body_kind| {
        f.locations
            .iter()
            .any(|loc| loc.origin.body_kind == body_kind)
    };

    if all_have_domain(UnitDomain::Style) {
        return Some(
            "merge selectors or move the declarations to a shared class/token if these elements should be coupled",
        );
    }
    if all_have_domain(UnitDomain::Markup) {
        return Some("share a component/template only if the data shape matches");
    }
    if all_have_domain(UnitDomain::Preprocessor) {
        return Some("review macro expansion and conditional context before sharing");
    }
    if all_have_domain(UnitDomain::TypeContract)
        && !f
            .locations
            .iter()
            .any(|loc| loc.origin.has_domain(UnitDomain::ImplementationType))
    {
        if all_subkind(UnitSubkind::InterfaceTraitProtocol) {
            return Some("consolidate one shared interface/protocol contract");
        }
        return Some("consolidate one shared type/API contract");
    }
    if all_have_domain(UnitDomain::TypeContract)
        && f.locations
            .iter()
            .any(|loc| loc.origin.has_domain(UnitDomain::ImplementationType))
    {
        return Some(
            "consolidate the type contract; review whether shared behavior should move too",
        );
    }
    if all_have_domain(UnitDomain::ImplementationType) {
        if all_subkind(UnitSubkind::Class)
            && (any_body(UnitBodyKind::Implementation) || any_body(UnitBodyKind::Mixed))
        {
            return Some("extract a shared base class / mixin");
        }
        return Some("consolidate shared type implementation");
    }
    if all_have_domain(UnitDomain::Imperative) {
        return Some("extract a helper");
    }
    None
}

fn proposal_action_label(f: &nose_detect::RefactorFamily) -> &'static str {
    use nose_il::UnitKind;

    if let Some(origin_hint) = origin_extract_hint(f) {
        return match origin_hint {
            "extract a helper" => "extract a shared helper",
            other => other,
        };
    }
    let all_classes = f.locations.iter().all(|loc| loc.kind == UnitKind::Class);
    let all_blocks = f.locations.iter().all(|loc| loc.kind == UnitKind::Block);
    let type_decl = all_classes && f.mean_sem < 12.0;
    if type_decl {
        "consolidate into one shared type"
    } else if all_classes {
        "extract a shared base class / mixin"
    } else if all_blocks {
        "extract a method from the repeated block"
    } else {
        "extract a shared helper"
    }
}

fn hint_reasons(f: &nose_detect::RefactorFamily) -> Vec<String> {
    use nose_il::{UnitBodyKind, UnitDomain, UnitSubkind};

    if f.locations.iter().all(|loc| loc.origin.is_unknown()) {
        return Vec::new();
    }
    let all_have_domain = |domain| f.locations.iter().all(|loc| loc.origin.has_domain(domain));
    let all_subkind = |subkind| f.locations.iter().all(|loc| loc.origin.subkind == subkind);
    let all_body = |body_kind| {
        f.locations
            .iter()
            .all(|loc| loc.origin.body_kind == body_kind)
    };
    let any_body = |body_kind| {
        f.locations
            .iter()
            .any(|loc| loc.origin.body_kind == body_kind)
    };

    let mut reasons = Vec::new();
    if all_have_domain(UnitDomain::TypeContract) {
        if all_subkind(UnitSubkind::InterfaceTraitProtocol) {
            reasons.push(format!(
                "all copies are {} interface/protocol contracts",
                family_language_label(f)
            ));
        } else {
            reasons.push("all copies are type/API contract regions".to_string());
        }
    } else if all_have_domain(UnitDomain::ImplementationType) {
        reasons.push("all copies are behavior-bearing type implementation regions".to_string());
    } else if all_have_domain(UnitDomain::Style) {
        reasons.push("all copies are declarative style rules".to_string());
    } else if all_have_domain(UnitDomain::Markup) {
        reasons.push("all copies are rendered markup/template regions".to_string());
    } else if all_have_domain(UnitDomain::Preprocessor) {
        reasons.push("all copies are macro/preprocessor regions".to_string());
    } else if all_have_domain(UnitDomain::Imperative) {
        reasons.push("all copies are imperative callable regions".to_string());
    }

    if all_body(UnitBodyKind::DeclarationOnly) {
        reasons.push("no implementation body was found".to_string());
    } else if all_body(UnitBodyKind::DeclarativeDenotation) {
        reasons
            .push("the duplicate is a declaration/denotation, not an imperative body".to_string());
    } else if any_body(UnitBodyKind::Mixed) {
        reasons.push("some copied regions mix declarations with reusable behavior".to_string());
    } else if any_body(UnitBodyKind::Implementation) {
        reasons.push("an implementation body was found".to_string());
    }

    let mut names = f.locations.iter().filter_map(|loc| loc.name.as_deref());
    if let Some(first) = names.next() {
        if f.locations.iter().filter(|loc| loc.name.is_some()).count() == f.members
            && names.all(|name| name == first)
        {
            reasons.push("every copy has the same symbol name".to_string());
        }
    }
    reasons
}

fn family_language_label(f: &nose_detect::RefactorFamily) -> String {
    let mut langs = f
        .locations
        .iter()
        .map(|loc| loc.lang.as_str())
        .collect::<Vec<_>>();
    langs.sort_unstable();
    langs.dedup();
    if langs.len() == 1 {
        language_label(langs[0]).to_string()
    } else {
        "cross-language".to_string()
    }
}

fn language_label(lang: &str) -> &'static str {
    match lang {
        "css" => "CSS",
        "go" => "Go",
        "html" => "HTML",
        "javascript" => "JavaScript",
        "typescript" => "TypeScript",
        "rust" => "Rust",
        "swift" => "Swift",
        "java" => "Java",
        "python" => "Python",
        "ruby" => "Ruby",
        "c" => "C",
        "vue" => "Vue",
        "svelte" => "Svelte",
        _ => "same-language",
    }
}

/// At this many varying spots an extraction stops being clean (issue #264's
/// triage experience: 6+ spots read as scenario-shaped, not helper-shaped).
const HIGH_PARAM_SPOTS: u32 = 6;

/// The default-surface families to render, in display order: overlapping slices folded
/// out, then production-scope findings ahead of test-scope, then truncated to `--top`.
///
/// §2c default-surface honesty: test duplication is a real smell (never dropped, still
/// ranked, in `--format json`, one `--scope test` away), but production leads the
/// bare-default screen so it is not buried — test scope was measured at 60–76% of the
/// default head ([default-surface-noise-audit](../../../docs/default-surface-noise-audit-2026-06-14.md)).
/// The reorder is stable (extractability rank preserved within each scope) and only runs
/// when no `--scope` filter has already narrowed the set to one scope.
fn select_shown_reportable<'a>(
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
fn print_refactor_human(
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
fn print_member_diff(a: &nose_detect::Loc, b: &nose_detect::Loc) {
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
fn print_member_proposal(locations: &[nose_detect::Loc], action: &str) {
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

/// Anti-unify N line-blocks at line granularity. Anchored on the first (largest) copy,
/// a line *survives* into the shared body only if it is matched in *every* other copy
/// (each copy votes via a pairwise `line_diff` against the anchor); any maximal run of
/// non-surviving anchor lines collapses to one `⟨param N⟩` placeholder. Returns the
/// skeleton, the count of lines shared across all copies, and the parameter count. With
/// two copies this is exactly the old pairwise anti-unification; with more, it is the
/// honest intersection — what the `--show proposal` view renders.
fn anti_unify_all(members: &[Vec<String>]) -> (Vec<String>, u32, u32) {
    let anchor: Vec<&str> = members[0].iter().map(String::as_str).collect();
    let n = anchor.len();
    // survive[i]: anchor line i is matched in every other copy.
    let mut survive = vec![true; n];
    for other in &members[1..] {
        let b: Vec<&str> = other.iter().map(String::as_str).collect();
        let mut matched = vec![false; n];
        let mut ai = 0usize;
        for (tag, _line) in line_diff(&anchor, &b) {
            match tag {
                // matched line — advances the anchor cursor and votes the line in.
                ' ' => {
                    if ai < n {
                        matched[ai] = true;
                    }
                    ai += 1;
                }
                // anchor-only line — advances the cursor, not voted in.
                '-' => ai += 1,
                // other-only line ('+') — does not advance the anchor cursor.
                _ => {}
            }
        }
        for (s, m) in survive.iter_mut().zip(matched) {
            *s &= m;
        }
    }
    let mut skeleton: Vec<String> = Vec::new();
    let mut shared = 0u32;
    let mut params = 0u32;
    // The open hole, if any: (the skeleton slot to fill once it closes, the placeholder
    // indent, and the anchor lines that vary across it — kept so the placeholder can carry a
    // value-class hint for the helper signature, #374 item 6).
    let mut hole: Option<(usize, &str, Vec<&str>)> = None;
    for (line, &kept) in anchor.iter().zip(&survive) {
        if kept {
            if let Some((slot, indent, lines)) = hole.take() {
                skeleton[slot] = format!("{indent}⟨param {params}: {}⟩", classify_param(&lines));
            }
            shared += 1;
            skeleton.push((*line).to_string());
        } else {
            match &mut hole {
                Some((_, _, lines)) => lines.push(line),
                None => {
                    params += 1;
                    let indent = &line[..line.len() - line.trim_start().len()];
                    let slot = skeleton.len();
                    skeleton.push(String::new());
                    hole = Some((slot, indent, vec![line]));
                }
            }
        }
    }
    if let Some((slot, indent, lines)) = hole.take() {
        skeleton[slot] = format!("{indent}⟨param {params}: {}⟩", classify_param(&lines));
    }
    (skeleton, shared, params)
}

/// A coarse value-class for one skeleton hole, from its (line-granularity) varying text — a
/// signature hint for the extracted helper, not a proof: `literal` (a constant → a value
/// parameter), `name` (a bare identifier), `call` (a call expression → maybe a closure/fn
/// parameter), `block` (a multi-line region → a large or divergent parameter), or `expr`
/// (anything else single-line).
fn classify_param(lines: &[&str]) -> &'static str {
    if lines.len() > 1 {
        return "block";
    }
    let t = lines.first().map_or("", |s| s.trim());
    let Some(first) = t.chars().next() else {
        return "expr";
    };
    if first.is_ascii_digit() || matches!(first, '"' | '\'' | '`') {
        "literal"
    } else if t.ends_with(')') && t.contains('(') {
        "call"
    } else if t
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
    {
        "name"
    } else {
        "expr"
    }
}

/// The invariant (shared) source lines across a family, plus the parameter count — the
/// honest counterpart to structural similarity. Returns *all* shared lines, including
/// boilerplate (`if err != nil {`, `}`): when a family genuinely shares a block, that
/// boilerplate is part of the helper you'd extract. The caller separates signal from
/// noise by *gating* on the substantive (non-trivial, rare) shared lines — a family
/// that shares only boilerplate scores ~0, while one with real shared content is
/// credited for its whole block (this is what stops idioms from ranking yet still
/// credits a `resolve*()` trio that shares a 13-line skeleton around a few varying args).
///
/// The shared set is intersected over a *majority* of members (up to `MEMBER_CAP`), not
/// just the closest pair — so a diverging copy shrinks the count honestly rather than
/// the flattering pair count overstating `N of M shared`. Parameters come from the first
/// pair that reads (a lower bound on the varying spots). `None` if no pair reads.
/// What the difference analysis yields for a family: the lines that drive the
/// *ranking* weight, the *displayed* invariant-line count, and the parameter
/// count — kept as three values because the display count and the ranking set
/// answer different questions (coevo S4-C2).
struct SharedLines {
    /// Majority-voted invariant lines (deduped, sorted) — the robust signal the
    /// ranking weights by IDF. Robustness is the point: a 6-copy family isn't
    /// tanked because its 6th copy diverges.
    rank_lines: Vec<String>,
    /// Lines invariant across **all** copies (#366) — the all-copies anti-unification
    /// count, the same number `nose query` shows, so scan and query report one
    /// shared/removable headline per family. Bounded by the representative pair's
    /// invariant count (`display ≤ rep-pair-invariant`), so with `params` (rep-pair
    /// holes ≤ `M − rep-pair-invariant`) it still holds `display + params ≤ M`: the
    /// `N of M shared, K spots differ` summary can never read `5 of 6 + 2 spots`
    /// (the §S4-C2 self-contradiction).
    display: u32,
    /// The representative pair's hole count — `K` in `N of M shared, K spots differ`,
    /// kept tied to `varying_spots` and the `param_penalty`/`shallow-extraction`
    /// ranking. Deliberately representative-pair, not all-copies: the all-copies
    /// hole count was gold-set-measured into the shallow ratio and regressed held-out
    /// (experiments §CL), so only `display` moved to the all-copies basis.
    params: u32,
}

fn shared_lines_of(locs: &[nose_detect::Loc], cache: &mut FileLineCache) -> Option<SharedLines> {
    const MEMBER_CAP: usize = 8;
    // Read the anchor (largest copy) and up to MEMBER_CAP-1 others once.
    let anchor = cache.slice(&locs[0].file, locs[0].start_line, locs[0].end_line)?;
    let mut members: Vec<Vec<String>> = vec![anchor];
    // The pairwise pass against the anchor feeds the majority-vote `rank_lines`
    // (→ `shared_weight`) and `params` (the representative-pair hole count, which stays
    // tied to `varying_spots` and drives `param_penalty`/`shallow-extraction`). These are
    // the ranking inputs and are computed exactly as before, so the family order is
    // unchanged. Only `display` becomes the all-copies count, below (#366).
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut n_others = 0usize;
    let mut params = 0u32;
    for b in locs.iter().skip(1).take(MEMBER_CAP - 1) {
        let Some(lb) = cache.slice(&b.file, b.start_line, b.end_line) else {
            continue;
        };
        let ar: Vec<&str> = members[0].iter().map(String::as_str).collect();
        let br: Vec<&str> = lb.iter().map(String::as_str).collect();
        let mut shared = Vec::new();
        let mut p = 0u32;
        let mut in_hole = false;
        for (tag, line) in &line_diff(&ar, &br) {
            if *tag == ' ' {
                in_hole = false;
                let t = line.trim();
                if !t.is_empty() {
                    shared.push(t.to_string());
                }
            } else if !in_hole {
                in_hole = true;
                p += 1;
            }
        }
        // Params come from the first pair that actually reads (the rep pair).
        if n_others == 0 {
            params = p;
        }
        n_others += 1;
        let uniq: std::collections::HashSet<String> = shared.into_iter().collect();
        for l in uniq {
            *counts.entry(l).or_insert(0) += 1;
        }
        members.push(lb);
    }
    if n_others == 0 {
        return None;
    }
    // Display: lines invariant across **all** copies (#366) — the same all-copies
    // anti-unification `nose query` renders, so scan and query report one shared/removable
    // headline per family (the old pairwise count over-stated families whose 3rd+ copies
    // diverge). Display-only and gold-set-measured ranking-neutral: the order reads
    // `shared_weight`/`params`, never this. (All-copies *params* was measured too and
    // regressed held-out — experiments §CL — so `params` stays representative-pair.)
    let (_skeleton, display, _params) = anti_unify_all(&members);
    let need = ((n_others as f64) * 0.6).ceil().max(1.0) as usize;
    let mut rank_lines: Vec<String> = counts
        .into_iter()
        .filter(|(_, c)| *c >= need)
        .map(|(l, _)| l)
        .collect();
    // Sort to a deterministic order: the caller sums `idf.weight()` over these lines,
    // and float addition isn't associative, so a `HashMap`-iteration order would make
    // `shared_weight` (and, via sort ties, the family order) vary run-to-run and across
    // thread counts — violating the byte-identical-output guarantee.
    rank_lines.sort_unstable();
    Some(SharedLines {
        rank_lines,
        display,
        params,
    })
}

/// The varying spots between two location line-blocks (#223): each maximal
/// differing run in the line diff becomes one spot carrying both sides' ABSOLUTE
/// source-line ranges and trimmed, length-capped text — so an agent can see WHAT
/// an extracted helper would parameterize (e.g. "every spot is a data literal")
/// without opening files. Same diff the `params` count walks.
pub(crate) fn varying_spots_of(
    a: &nose_detect::Loc,
    b: &nose_detect::Loc,
    cache: &mut FileLineCache,
) -> Option<Vec<nose_detect::VaryingSpot>> {
    const SPOT_CAP: usize = 16;
    const TEXT_CAP: usize = 160;
    let la = cache.slice(&a.file, a.start_line, a.end_line)?;
    let lb = cache.slice(&b.file, b.start_line, b.end_line)?;
    let ar: Vec<&str> = la.iter().map(String::as_str).collect();
    let br: Vec<&str> = lb.iter().map(String::as_str).collect();
    let cap_text = |t: &str| {
        let t = t.trim();
        if t.len() > TEXT_CAP {
            let mut end = TEXT_CAP;
            while !t.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}…", &t[..end])
        } else {
            t.to_string()
        }
    };
    let mut spots: Vec<nose_detect::VaryingSpot> = Vec::new();
    let (mut ai, mut bi) = (0u32, 0u32);
    let mut open = false;
    for (tag, line) in line_diff(&ar, &br) {
        match tag {
            ' ' => {
                open = false;
                ai += 1;
                bi += 1;
            }
            _ => {
                if !open {
                    open = true;
                    if spots.len() >= SPOT_CAP {
                        return Some(spots);
                    }
                    spots.push(nose_detect::VaryingSpot {
                        param: spots.len() as u32 + 1,
                        a_lines: None,
                        b_lines: None,
                        a_text: String::new(),
                        b_text: String::new(),
                    });
                }
                let spot = spots.last_mut().expect("opened above");
                if tag == '-' {
                    let abs = a.start_line + ai;
                    spot.a_lines = Some(match spot.a_lines {
                        None => (abs, abs),
                        Some((s, _)) => (s, abs),
                    });
                    if !spot.a_text.is_empty() {
                        spot.a_text.push(' ');
                    }
                    if spot.a_text.len() <= TEXT_CAP {
                        spot.a_text.push_str(&cap_text(&line));
                    }
                    ai += 1;
                } else {
                    let abs = b.start_line + bi;
                    spot.b_lines = Some(match spot.b_lines {
                        None => (abs, abs),
                        Some((s, _)) => (s, abs),
                    });
                    if !spot.b_text.is_empty() {
                        spot.b_text.push(' ');
                    }
                    if spot.b_text.len() <= TEXT_CAP {
                        spot.b_text.push_str(&cap_text(&line));
                    }
                    bi += 1;
                }
            }
        }
    }
    for s in &mut spots {
        s.a_text = cap_text(&s.a_text);
        s.b_text = cap_text(&s.b_text);
    }
    Some(spots)
}

/// A line with no extractable content on its own: blank, pure delimiters (`}`, `});`,
/// `)`), or a bare control keyword. Sharing one of these between two blocks says
/// nothing about whether they're the same code.
fn is_trivial_line(t: &str) -> bool {
    t.is_empty()
        || t.chars().all(|c| {
            matches!(
                c,
                '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | ' ' | '\t'
            )
        })
        || matches!(
            t,
            "return" | "break" | "continue" | "else" | "else {" | "};" | "})" | "});"
        )
}

/// How *idiomatic* (pervasive) each source line is across the scanned corpus, by the
/// fraction of files it appears in. A line in a large fraction of files is a language
/// idiom (`if err != nil {`, a ubiquitous logging call) and earns ~0 weight; a line in
/// few files is specific and earns full weight — so a language idiom, however often it's
/// literally duplicated, can't rank as an extractable refactor, with no hardcoded
/// idiom list. The floor is generous (`LO`): ordinary cross-file duplication — the very
/// thing we want to surface — keeps full weight; only genuinely pervasive lines are
/// docked. This matters on small repos, where naive IDF would penalize everything.
struct LineIdf {
    df: std::collections::HashMap<String, u32>,
    n_files: f64,
}

impl LineIdf {
    fn weight(&self, line: &str) -> f64 {
        if self.n_files <= 1.0 {
            return 1.0; // single-file corpus: no frequency signal
        }
        let frac = self.df.get(line).copied().unwrap_or(1) as f64 / self.n_files;
        const LO: f64 = 0.25; // ≤25% of files: specific → full weight
        const HI: f64 = 0.60; // ≥60% of files: pervasive idiom → no weight
        ((HI - frac) / (HI - LO)).clamp(0.0, 1.0)
    }
}

/// Build the [`LineIdf`] by reading every scanned file once (through `cache`, which the
/// per-family diffs then reuse) and counting, per trimmed non-trivial line, how many
/// distinct files contain it.
fn corpus_line_idf(
    refs: &[&std::path::Path],
    exclude: &[String],
    cache: &mut FileLineCache,
) -> LineIdf {
    let mut df: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut n_files = 0u32;
    for root in refs {
        for (path, _lang) in nose_frontend::discover_paths(root, exclude) {
            let Some(all) = cache.whole(&path) else {
                continue;
            };
            n_files += 1;
            let mut seen = std::collections::HashSet::new();
            for l in all {
                let t = l.trim();
                if !is_trivial_line(t) && seen.insert(t.to_string()) {
                    *df.entry(t.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
    LineIdf {
        df,
        n_files: n_files.max(1) as f64,
    }
}

/// Deterministic ranking tie-break: a family's first site `(file, start line)`.
fn family_anchor(f: &nose_detect::RefactorFamily) -> (String, u32) {
    f.locations
        .first()
        .map(|l| (l.file.clone(), l.start_line))
        .unwrap_or_default()
}

/// Memoizes file contents (split into lines) so ranking many families that touch the
/// same files reads each file at most once. `None` for files that fail to read.
#[derive(Default)]
pub(crate) struct FileLineCache(std::collections::HashMap<String, Option<Vec<String>>>);

impl FileLineCache {
    /// All lines of `file`, reading and caching on first touch. `None` if unreadable.
    pub(crate) fn whole(&mut self, file: &str) -> Option<&[String]> {
        self.0
            .entry(file.to_string())
            .or_insert_with(|| {
                std::fs::read_to_string(file)
                    .ok()
                    .map(|t| t.lines().map(str::to_string).collect())
            })
            .as_deref()
    }

    /// Lines `start..=end` (1-based) of `file`.
    fn slice(&mut self, file: &str, start: u32, end: u32) -> Option<Vec<String>> {
        let all = self.whole(file)?;
        let (s, e) = (
            start.saturating_sub(1) as usize,
            (end as usize).min(all.len()),
        );
        (s < e).then(|| all[s..e].to_vec())
    }
}

/// Read lines `start..=end` (1-based) of `file` as raw strings.
fn read_lines(file: &str, start: u32, end: u32) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let (s, e) = (
        start.saturating_sub(1) as usize,
        (end as usize).min(lines.len()),
    );
    (s < e).then(|| lines[s..e].iter().map(|l| l.to_string()).collect())
}

/// Minimal LCS line diff → `(' '|'-'|'+', line)`. Caps each side so the O(n·m)
/// table stays small on large members (the differing lines are what matter).
fn line_diff(a: &[&str], b: &[&str]) -> Vec<(char, String)> {
    const CAP: usize = 120;
    let a = &a[..a.len().min(CAP)];
    let b = &b[..b.len().min(CAP)];
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((' ', a[i].to_string()));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(('-', a[i].to_string()));
            i += 1;
        } else {
            out.push(('+', b[j].to_string()));
            j += 1;
        }
    }
    out.extend(a[i..].iter().map(|l| ('-', l.to_string())));
    out.extend(b[j..].iter().map(|l| ('+', l.to_string())));
    out
}

/// Markdown form of the all-copies extraction skeleton (#360), rendered on an `id=<fam>`
/// drilldown so `--format markdown` honors the help's "every copy + extraction skeleton"
/// promise the same way the human/JSON views do (#422). The bulk report stays a compact
/// location list; the skeleton is paid only when the consumer drills into one family.
fn markdown_member_proposal(locations: &[nose_detect::Loc]) {
    let members: Vec<Vec<String>> = locations
        .iter()
        .filter_map(|l| read_lines(&l.file, l.start_line, l.end_line))
        .collect();
    if members.len() < 2 {
        return;
    }
    let (skeleton, shared, params) = anti_unify_all(&members);
    let copies = members.len();
    println!(
        "**proposal** — extract a shared helper · {shared} shared lines · {params} parameter(s) vary (across all {copies} copies)\n"
    );
    println!("```text");
    for line in skeleton.iter().take(40) {
        println!("{line}");
    }
    println!("```\n");
}

/// Markdown form of the representative two-copy diff, added on `id=<fam> full` (the `full`
/// view, mirroring the human renderer's extra diff line).
fn markdown_member_diff(a: &nose_detect::Loc, b: &nose_detect::Loc) {
    let (Some(la), Some(lb)) = (
        read_lines(&a.file, a.start_line, a.end_line),
        read_lines(&b.file, b.start_line, b.end_line),
    ) else {
        return;
    };
    println!(
        "**diff** — `{}:{}-{}` vs `{}:{}-{}`\n",
        a.file, a.start_line, a.end_line, b.file, b.start_line, b.end_line
    );
    let ar: Vec<&str> = la.iter().map(String::as_str).collect();
    let br: Vec<&str> = lb.iter().map(String::as_str).collect();
    println!("```diff");
    for (tag, line) in line_diff(&ar, &br) {
        println!("{tag} {line}");
    }
    println!("```\n");
}

fn print_refactor_markdown(
    all: &[&nose_detect::RefactorFamily],
    shown: &[&nose_detect::RefactorFamily],
    mode: ScanChannels,
    baseline: Option<&BaselineComparison>,
    ignore_set: Option<&ignores::IgnoreSet>,
    ignored_families: usize,
    omitted_note: Option<&str>,
) {
    println!("# {}\n", mode.markdown_title());
    println!(
        "{} {} · ~{} duplicated lines · showing top {}\n",
        all.len(),
        plural(all.len(), "family", "families"),
        total_dup_lines_refs(all),
        shown.len()
    );
    if let Some(note) = omitted_note {
        println!("{note}\n");
    }
    if let Some(comparison) = baseline {
        println!("{}\n", comparison.summary.line());
    }
    if let Some(ignore_set) = ignore_set {
        println!("{}\n", ignore_set.summary(ignored_families).line());
    }
    for (i, f) in shown.iter().enumerate() {
        let xlang = match family_langs(f) {
            s if s.is_empty() => String::new(),
            s => format!(" · cross-language: {s}"),
        };
        println!(
            "## {}. `{}` — {} sites, {} {}, {} {} — ~{} dup lines ({}){}",
            i + 1,
            baseline::family_id(f),
            f.members,
            f.files,
            plural(f.files, "file", "files"),
            f.modules,
            plural(f.modules, "directory", "directories"),
            f.dup_lines,
            similarity_cell(f),
            xlang
        );
        println!("\n*{}*\n", family_hint(f));
        if let Some(witness) = &f.abstraction_witness {
            println!("_witness: {}_\n", abstraction_witness_summary(witness));
        }
        for l in &f.locations {
            let name = l
                .name
                .as_deref()
                .map(|n| format!(" `{n}`"))
                .unwrap_or_default();
            println!("- `{}:{}-{}`{}", l.file, l.start_line, l.end_line, name);
        }
        println!();
    }
}

struct DetectArgs {
    paths: Vec<PathBuf>,
    min_lines: u32,
    min_tokens: usize,
    threshold: Option<f64>,
    candidates: bool,
    minhash_k: usize,
    bands: usize,
    no_cfg_norm: bool,
    dce: bool,
    no_blocks: bool,
    out: Option<PathBuf>,
    summary: bool,
    bench_schema: bool,
    repos_root: Option<PathBuf>,
    dump: Option<PathBuf>,
}

fn cmd_detect(args: DetectArgs) -> Result<()> {
    let refs = paths_as_refs(&args.paths);
    let corpus = time_lower(|| nose_frontend::lower_corpus_many(&refs));
    warn_if_empty(&corpus, &args.paths);

    let opts = nose_detect::DetectOptions {
        min_lines: args.min_lines,
        min_tokens: args.min_tokens,
        threshold: args
            .threshold
            .unwrap_or(if args.candidates { 0.70 } else { 0.86 }),
        minhash_k: args.minhash_k,
        bands: args.bands,
        cfg_norm: !args.no_cfg_norm,
        dce: args.dce,
        block_units: !args.no_blocks,
        ..Default::default()
    };
    let detector = if args.candidates {
        nose_detect::StructuralDetector::candidates(opts.jaccard_weight)
            .with_threshold(opts.threshold)
    } else {
        nose_detect::StructuralDetector::strict(opts.jaccard_weight).with_threshold(opts.threshold)
    };

    // Diagnostic dump: units + candidates + predictions to a directory.
    if let Some(dir) = &args.dump {
        let root = args
            .repos_root
            .as_ref()
            .context("--dump requires --repos-root")?;
        let (report, dump) = nose_detect::detect_with_dump(&corpus, &opts, &detector);
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

        let units: Vec<nose_eval::UnitRegion> = dump
            .units
            .iter()
            .map(|u| match map_to_repo(root, &u.path) {
                // keep index alignment for unmappable units with a sentinel repo
                Some((repo, file)) => nose_eval::UnitRegion {
                    repo,
                    file,
                    start_line: u.start_line,
                    end_line: u.end_line,
                },
                None => nose_eval::UnitRegion {
                    repo: String::new(),
                    file: u.path.clone(),
                    start_line: u.start_line,
                    end_line: u.end_line,
                },
            })
            .collect();
        std::fs::write(
            dir.join("units.json"),
            serde_json::to_string(&nose_eval::UnitsDump { units })?,
        )?;
        std::fs::write(
            dir.join("candidates.json"),
            serde_json::to_string(&nose_eval::CandidatesDump {
                candidates: dump.candidates,
            })?,
        )?;
        let preds = to_benchmark_predictions(&report, root);
        std::fs::write(dir.join("predictions.json"), serde_json::to_string(&preds)?)?;
        eprintln!(
            "dump → {}: {} units, {} candidate pairs, {} predictions",
            dir.display(),
            report.metrics.units,
            report.metrics.candidate_pairs,
            preds.duplicates.len()
        );
        return Ok(());
    }

    let report = nose_detect::detect(&corpus, &opts, &detector);

    if args.bench_schema {
        let root = args
            .repos_root
            .as_ref()
            .context("--bench-schema requires --repos-root")?;
        let preds = to_benchmark_predictions(&report, root);
        let json = serde_json::to_string_pretty(&preds)?;
        match &args.out {
            Some(path) => {
                std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?
            }
            None => println!("{json}"),
        }
        eprintln!(
            "emitted {} benchmark-schema predictions",
            preds.duplicates.len()
        );
        return Ok(());
    }

    if args.summary {
        print_summary(&report);
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&report)?;
    match args.out {
        Some(path) => {
            std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?
        }
        None => println!("{json}"),
    }
    Ok(())
}

/// Map an absolute/scanned path to `(repo_id, repo_relative_path)` given the root
/// whose immediate children are repo ids.
fn map_to_repo(root: &std::path::Path, path: &str) -> Option<(String, String)> {
    let rel = std::path::Path::new(path).strip_prefix(root).ok()?;
    let mut comps = rel.components();
    let repo = comps.next()?.as_os_str().to_str()?.to_string();
    let relpath = comps.as_path().to_str()?.to_string();
    if relpath.is_empty() {
        return None;
    }
    Some((repo, relpath))
}

/// Convert nose's clone pairs into benchmark predictions (repo id +
/// repo-relative paths, 1-based lines, `nose_semantic` channel).
fn to_benchmark_predictions(
    report: &nose_detect::Report,
    root: &std::path::Path,
) -> nose_eval::Predictions {
    let mut duplicates = Vec::new();
    for d in &report.duplicates {
        let (lrepo, lfile) = match map_to_repo(root, &d.left.file) {
            Some(x) => x,
            None => continue,
        };
        let (rrepo, rfile) = match map_to_repo(root, &d.right.file) {
            Some(x) => x,
            None => continue,
        };
        duplicates.push(nose_eval::PredPair {
            repo: Some(lrepo.clone()),
            channel: Some("nose_semantic".to_string()),
            score: Some(d.score),
            left: nose_eval::PredRegion {
                repo: Some(lrepo),
                file: lfile,
                start_line: d.left.start_line,
                end_line: d.left.end_line,
                symbol: d.left.name.clone(),
            },
            right: nose_eval::PredRegion {
                repo: Some(rrepo),
                file: rfile,
                start_line: d.right.start_line,
                end_line: d.right.end_line,
                symbol: d.right.name.clone(),
            },
        });
    }
    nose_eval::Predictions {
        schema_version: "0.1.0".to_string(),
        tool: "nose".to_string(),
        duplicates,
    }
}

fn print_summary(report: &nose_detect::Report) {
    let m = &report.metrics;
    eprintln!(
        "nose [{}]: {} files, {} units, {} candidate pairs, {} accepted, {} clone groups",
        report.detector, m.files, m.units, m.candidate_pairs, m.accepted_pairs, m.groups
    );
    for (n, g) in report.groups.iter().enumerate() {
        println!(
            "\nGroup {} (score {:.3}, {} members):",
            n + 1,
            g.score,
            g.members.len()
        );
        for mem in &g.members {
            let name = mem.name.as_deref().unwrap_or("");
            println!(
                "  {}:{}-{}  [{}] {}",
                mem.file, mem.start_line, mem.end_line, mem.lang, name
            );
        }
    }
}

fn cmd_il(path: PathBuf, format: Format, normalized: bool, no_cfg_norm: bool) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let lang = Lang::from_path(&path_str)
        .with_context(|| format!("unsupported file extension: {path_str}"))?;
    let src = std::fs::read(&path).with_context(|| format!("reading {path_str}"))?;
    let interner = Interner::new();
    // Use the region-aware entry so `<script>`/`<style>`/markup of a Vue/Svelte/HTML
    // container are each shown (single-region languages still yield exactly one Il).
    let regions = nose_frontend::lower_source_regions(FileId(0), &path_str, &src, lang, &interner);
    if regions.is_empty() {
        anyhow::bail!("no analyzable region lowered from {path_str}");
    }
    let multi = regions.len() > 1;
    for raw in regions {
        let region_lang = raw.meta.lang;
        let il = if normalized {
            let opts = nose_normalize::NormalizeOptions {
                cfg_norm: !no_cfg_norm,
                ..Default::default()
            };
            nose_normalize::normalize(&raw, &interner, &opts)
        } else {
            raw
        };
        match format {
            Format::Sexpr => {
                if multi {
                    println!("; region: {}", region_lang.name());
                }
                println!("{}", il.to_sexpr(il.root, &interner));
            }
            Format::Json => {
                println!("{}", serde_json::to_string_pretty(&il)?);
            }
        }
    }
    Ok(())
}

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

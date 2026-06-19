//! `nose` — multi-language code clone detector CLI.

mod baseline;
mod baseline_view;
mod cache;
mod capabilities;
mod cli_args;
mod command_dispatch;
mod config;
mod detect_command;
mod detect_pipeline;
mod diagnostic_commands;
mod falsify;
mod family_display;
mod fnv;
mod ignores;
mod il_command;
#[cfg(test)]
mod main_tests;
mod markdown;
mod oracle_gate;
mod path_utils;
mod query_commands;
mod query_dashboard;
mod query_model;
mod query_open;
mod query_terms;
mod query_views;
mod report_text;
mod review;
mod runtime;
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
mod style;
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
pub(crate) use detect_pipeline::*;
use diagnostic_commands::*;
use family_display::*;
pub(crate) use il_command::*;
use nose_il::{Corpus, FileId, Interner, Lang};
use oracle_gate::*;
pub(crate) use path_utils::*;
use query_commands::*;
use query_terms::{family_at, parse_query, QFilter, QOp, Query};
use rayon::prelude::*;
pub(crate) use report_text::*;
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

fn main() -> Result<()> {
    runtime::install_broken_pipe_guard();
    // rayon executes tasks both on its pool workers AND inline on the calling thread,
    // so enlarge the workers' stacks here and run the command body on a big-stack
    // thread below — otherwise a deep file lowered inline on a normal-stack thread
    // still overflows.
    let _ = rayon::ThreadPoolBuilder::new()
        .stack_size(runtime::STACK_SIZE)
        .build_global();
    std::thread::Builder::new()
        .stack_size(runtime::STACK_SIZE)
        .spawn(command_dispatch::run)
        .expect("spawn worker thread")
        .join()
        .expect("worker thread panicked")
}

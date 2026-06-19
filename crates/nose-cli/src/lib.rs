mod baseline;
mod baseline_comparison;
mod cache;
mod capabilities;
mod cli_args;
mod command_dispatch;
mod config;
mod detect_command;
mod detect_pipeline;
mod diagnostic_commands;
mod divergence;
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
mod query_baseline_gate;
mod query_commands;
mod query_dashboard;
mod query_dataset;
mod query_family_text;
mod query_markdown;
mod query_model;
mod query_open;
mod query_opportunities;
mod query_options;
mod query_sarif;
mod query_terms;
mod query_views;
mod query_witness;
mod report_text;
mod runtime;
mod schema_versions;
mod semantic_pack;
mod source_lines;
mod style;
mod surfaces;
mod timing;
mod verify_census;
mod verify_collect;
mod verify_report;

use anyhow::{Context, Result};
use baseline_comparison::*;
pub(crate) use cli_args::*;
pub(crate) use detect_command::*;
pub(crate) use detect_pipeline::*;
use diagnostic_commands::*;
use family_display::*;
pub(crate) use il_command::*;
use nose_il::{Corpus, FileId, Interner, Lang};
use oracle_gate::*;
pub(crate) use path_utils::*;
pub(crate) use query_baseline_gate::*;
use query_commands::*;
use query_dataset::*;
pub(crate) use query_family_text::*;
pub(crate) use query_markdown::*;
pub(crate) use query_opportunities::*;
pub(crate) use query_options::*;
pub(crate) use query_sarif::*;
use query_terms::{family_at, parse_query, QFilter, QOp, Query};
pub(crate) use query_witness::*;
use rayon::prelude::*;
pub(crate) use report_text::*;
pub(crate) use source_lines::*;
use std::path::PathBuf;
#[cfg(test)]
use surfaces::family_actionability_reason;
use surfaces::{
    classify_surface_overrides, effective_surface, is_default_report_family, surface_omission_note,
    SurfaceOverrides,
};
pub(crate) use timing::*;
use verify_collect::*;
use verify_report::*;

pub fn install_broken_pipe_guard() {
    runtime::install_broken_pipe_guard();
}

pub const STACK_SIZE: usize = runtime::STACK_SIZE;

pub fn run_command() -> Result<()> {
    command_dispatch::run()
}

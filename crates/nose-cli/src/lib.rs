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
mod legacy_prelude;
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

use anyhow::Result;

pub fn install_broken_pipe_guard() {
    runtime::install_broken_pipe_guard();
}

pub const STACK_SIZE: usize = runtime::STACK_SIZE;

pub fn run_command() -> Result<()> {
    command_dispatch::run()
}

//! `alp-cli` — headless tools for Audio Labs Pro.
//!
//! The CLI is the Phase 1 acceptance harness: it generates fixtures, decodes
//! files, builds and reopens peak caches, and renders projects without a UI.

pub mod cli;
pub mod commands;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::Cli;

/// Parse arguments from the environment and run.
pub fn run_from_env() -> Result<()> {
    let cli = Cli::parse();
    alp_core::init_logging(cli.log.as_deref()).context("initialise logging")?;
    run(cli)
}

/// Run an already parsed command.
pub fn run(cli: Cli) -> Result<()> {
    commands::dispatch(cli.command)
}

//! cli.rs — clap argument definitions for the `datagov` command tree.
//!
//! 1. Defines `Cli`, the root parser: global `--output`, `--quiet`,
//!    `--verbose` flags. `--quiet` and `--verbose` are mutually
//!    exclusive; clap exits 2 (its own default usage-error code) if both
//!    are given.
//! 2. Defines `Command`, the Bolt 1 subcommand set (`version`,
//!    `capabilities`).
//! 3. Defines `OutputFormat`, the clap-facing rendering choice
//!    (`json` | `table`, default `table`).

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "datagov", version, about = "DataGovOps CLI")]
pub struct Cli {
    /// Output rendering: human table or the canonical JSON envelope.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Table)]
    pub output: OutputFormat,

    /// Suppress all diagnostics except errors.
    #[arg(long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Emit DEBUG-level JSON-lines diagnostics on stderr.
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print the `datagov` version.
    Version,
    /// Report compiled commands, supported formats, and enabled features.
    Capabilities,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
}

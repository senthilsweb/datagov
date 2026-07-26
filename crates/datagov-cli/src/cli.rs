//! cli.rs — clap argument definitions for the `datagov` command tree.
//!
//! 1. Defines `Cli`, the root parser: global `--output`, `--quiet`,
//!    `--verbose` flags. `--quiet` and `--verbose` are mutually
//!    exclusive; clap exits 2 (its own default usage-error code) if both
//!    are given.
//! 2. Defines `Command`, the subcommand set: `version`, `capabilities`
//!    (Bolt 1), and `inspect` (Bolt 2) — `datagov inspect <path|-> [--type
//!    <csv|tsv|json|jsonl|parquet>]`.
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
    /// Inspect a dataset: format, size, row/column counts, schema,
    /// nullability, Parquet row groups/compression, masked sample rows.
    Inspect {
        /// Path to the dataset, or `-` to read from stdin.
        path: String,
        /// Explicit format when it cannot be inferred from the path
        /// (required when reading from stdin).
        #[arg(long = "type", value_name = "FORMAT")]
        r#type: Option<String>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
}

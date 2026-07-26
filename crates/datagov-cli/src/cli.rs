//! cli.rs — clap argument definitions for the `datagov` command tree.
//!
//! 1. Defines `Cli`, the root parser: global `--output`, `--quiet`,
//!    `--verbose` flags. `--quiet` and `--verbose` are mutually
//!    exclusive; clap exits 2 (its own default usage-error code) if both
//!    are given.
//! 2. Defines `Command`, the subcommand set: `version`, `capabilities`
//!    (Bolt 1), `inspect` (Bolt 2) — `datagov inspect <path|-> [--type
//!    <csv|tsv|json|jsonl|parquet>]` — and (Bolt 3) `profile` (`datagov
//!    profile <path> [--type ..] [--columns a,b] [--sample n]`) and
//!    `query` (`datagov query "<sql>" [--limit n]`).
//! 3. Defines `OutputFormat`, the clap-facing rendering choice (`json` |
//!    `table` | `csv`, default `table`). `csv` is Bolt 3, accepted only
//!    by `query` — every other command rejects it explicitly rather than
//!    silently falling back to table rendering.

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
    /// Compute per-column statistics (nulls, distinct, min/max/mean/
    /// median/stddev, quantiles, string lengths, top values, semantic
    /// type, possible identifiers) via the embedded DataFusion engine.
    /// CSV and Parquet only.
    Profile {
        /// Path to the dataset (a real file — DataFusion table
        /// registration needs random file access, so `-` is not
        /// supported).
        path: String,
        /// Explicit format when it cannot be inferred from the path.
        #[arg(long = "type", value_name = "FORMAT")]
        r#type: Option<String>,
        /// Profile only these column names (comma-separated).
        #[arg(long, value_delimiter = ',', value_name = "COLUMNS")]
        columns: Option<Vec<String>>,
        /// Profile only the first N rows in source order (deterministic
        /// — not a random sample).
        #[arg(long, value_name = "N")]
        sample: Option<u64>,
    },
    /// Run SQL over local CSV/Parquet files via the embedded DataFusion
    /// engine. Quoted file paths in `FROM`/`JOIN` position (e.g.
    /// `'customers.parquet'`) are resolved automatically.
    Query {
        /// The SQL statement to execute.
        sql: String,
        /// Override the default row bound (see
        /// `datagov_data::query::DEFAULT_QUERY_LIMIT`).
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    /// Bolt 3: raw CSV to stdout, no envelope. Only `query` accepts this.
    Csv,
}

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
//!    `query` (`datagov query "<sql>" [--limit n]`). Bolt 4 adds `sql`,
//!    a nested subcommand (`SqlAction`): `parse <path|-> [--dialect
//!    <name>]`, `format <path|-> [--dialect <name>] [--write]`,
//!    `transpile <path|-> --from <dialect> --to <dialect>`. All three
//!    default `--dialect` to `datagov_sql::DEFAULT_DIALECT` ("ansi")
//!    when omitted. Bolt 5 adds `pii`, a nested subcommand
//!    (`PiiAction`): `scan <path> [--type ..] [--sample n] [--field
//!    a,b] [--recognizers path] [--fail-on 0.x]` and `recognizers`
//!    (itself nested, `RecognizersAction`): `list` and
//!    `validate <path>`.
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
    /// Parse, format, or transpile SQL across the 11 priority dialects
    /// (ANSI, PostgreSQL, DuckDB, Spark, Databricks, Snowflake,
    /// BigQuery, Trino, MySQL, SQLite, T-SQL), via `datagov-sql`
    /// (`sqlglot-rust`).
    Sql {
        #[command(subcommand)]
        action: SqlAction,
    },
    /// Deterministic PII detection (PRD §10.8/§10.9): `scan` a dataset
    /// for entity-typed, confidence-scored findings (masked evidence
    /// only — no raw value ever appears in the output), or manage
    /// recognizers.
    Pii {
        #[command(subcommand)]
        action: PiiAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum PiiAction {
    /// Scan a dataset (CSV, TSV, JSONL, or Parquet) for PII. Every
    /// finding carries entity, column, confidence, recognizer, masked
    /// evidence, match count/percentage, and reason — never a raw value.
    Scan {
        /// Path to the dataset (a real file — DataFusion table
        /// registration needs random file access, so `-` is not
        /// supported).
        path: String,
        /// Explicit format when it cannot be inferred from the path.
        #[arg(long = "type", value_name = "FORMAT")]
        r#type: Option<String>,
        /// Scan only the first N rows in source order (deterministic —
        /// not a random sample).
        #[arg(long, value_name = "N")]
        sample: Option<u64>,
        /// Scan only these column names (comma-separated); default is
        /// all columns.
        #[arg(long, value_delimiter = ',', value_name = "FIELDS")]
        field: Option<Vec<String>>,
        /// Load additional recognizers from a PRD §10.9 YAML file. A
        /// custom recognizer whose `id` matches a built-in overrides it;
        /// otherwise it is added.
        #[arg(long, value_name = "PATH")]
        recognizers: Option<String>,
        /// Exit with code 12 if any finding's confidence is at or above
        /// this threshold — checked after the full report is emitted,
        /// never instead of it.
        #[arg(long, value_name = "CONFIDENCE")]
        fail_on: Option<f64>,
    },
    /// Enumerate or validate PII recognizers.
    Recognizers {
        #[command(subcommand)]
        action: RecognizersAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum RecognizersAction {
    /// List the 10 built-in recognizers (id, entity, confidence, pattern
    /// count).
    List,
    /// Validate a PRD §10.9 recognizers YAML file: regex compiles,
    /// confidence is in range, and every validator name is known.
    Validate {
        /// Path to the recognizers YAML file.
        path: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SqlAction {
    /// Parse a SQL statement: statement type, tables, columns, joins,
    /// filters, grouping, ordering, CTEs, and the full AST.
    Parse {
        /// Path to a `.sql` file, or `-` to read from stdin.
        path: String,
        /// The source dialect (default: `ansi`).
        #[arg(long, value_name = "DIALECT")]
        dialect: Option<String>,
    },
    /// Pretty-print a SQL statement. Writes to stdout by default; the
    /// source file is modified only when `--write` is given.
    Format {
        /// Path to a `.sql` file, or `-` to read from stdin (`--write`
        /// is not compatible with stdin — there is no source file to
        /// modify).
        path: String,
        /// The source dialect (default: `ansi`).
        #[arg(long, value_name = "DIALECT")]
        dialect: Option<String>,
        /// Modify `path` in place with the formatted SQL instead of
        /// printing to stdout.
        #[arg(long)]
        write: bool,
    },
    /// Transpile a SQL statement from one dialect to another.
    Transpile {
        /// Path to a `.sql` file, or `-` to read from stdin.
        path: String,
        /// The source dialect.
        #[arg(long)]
        from: String,
        /// The target dialect.
        #[arg(long)]
        to: String,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    /// Bolt 3: raw CSV to stdout, no envelope. Only `query` accepts this.
    Csv,
}

//! mod.rs — Command implementations for the `datagov` CLI.
//!
//! 1. Declares the per-subcommand modules. Each exposes a `run` function
//!    that is the only thing `main.rs` calls for its subcommand: parse
//!    (already done by clap) -> call a `datagov-core`/`datagov-data`
//!    function -> render. No business logic lives in these files beyond
//!    that adaptation.
//! 2. Declares `reject_csv`, the shared guard every command except
//!    `query` calls first: `--output csv` is `DatagovError::InvalidArgs`
//!    (exit 2), naming which command was called and that only `query`
//!    supports CSV — never a silent fallback to table rendering.

pub mod capabilities;
pub mod inspect;
pub mod profile;
pub mod query;
pub mod sql;
pub mod version;

use crate::cli::OutputFormat;
use datagov_core::DatagovError;

/// Reject `--output csv` for every command except `query`.
pub fn reject_csv(output: OutputFormat, command_name: &str) -> Result<(), DatagovError> {
    if output == OutputFormat::Csv {
        return Err(DatagovError::invalid_args(
            format!("'{command_name}' does not support --output csv"),
            "use --output json or --output table; only 'query' supports csv".to_string(),
        ));
    }
    Ok(())
}

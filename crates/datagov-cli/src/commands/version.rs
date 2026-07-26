//! version.rs — `datagov version`: thin adapter, no business logic.
//!
//! 1. Human rendering: `datagov <version>`.
//! 2. `--output json`: the full report envelope (no `input`, no
//!    populated sections, summary defaults to success).
//! 3. `--output csv` is rejected (exit 2) via `commands::reject_csv` —
//!    only `query` supports CSV.

use crate::cli::OutputFormat;
use crate::commands::reject_csv;
use datagov_core::DatagovError;
use datagov_core::report::ReportBuilder;

pub fn run(output: OutputFormat) -> Result<(), DatagovError> {
    reject_csv(output, "version")?;

    match output {
        OutputFormat::Table => {
            println!("datagov {}", env!("CARGO_PKG_VERSION"));
        }
        OutputFormat::Json => {
            let report = ReportBuilder::new().build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }
    Ok(())
}

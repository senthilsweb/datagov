//! capabilities.rs — `datagov capabilities`: thin adapter, no business
//! logic.
//!
//! 1. Reports the compiled command set (`version`, `capabilities`,
//!    `inspect`, `profile`, `query`), supported input formats (the five
//!    `datagov-data` readers landed in Bolt 2 — `profile`/`query` are
//!    CSV/Parquet-only, see the Bolt 3 report), and enabled cargo
//!    features (none defined yet).
//! 2. Human rendering: a small table. `--output json`: the envelope with
//!    the payload under `extensions.capabilities`.
//! 3. `--output csv` is rejected (exit 2) via `commands::reject_csv` —
//!    only `query` supports CSV.

use crate::cli::OutputFormat;
use crate::commands::reject_csv;
use datagov_core::DatagovError;
use datagov_core::report::ReportBuilder;
use serde_json::json;

const COMMANDS: &[&str] = &["version", "capabilities", "inspect", "profile", "query"];
const FORMATS: &[&str] = &["csv", "tsv", "json", "jsonl", "parquet"];
const FEATURES: &[&str] = &[];

pub fn run(output: OutputFormat) -> Result<(), DatagovError> {
    reject_csv(output, "capabilities")?;

    match output {
        OutputFormat::Table => {
            let formats_display = if FORMATS.is_empty() {
                "(none yet)".to_string()
            } else {
                FORMATS.join(", ")
            };
            let features_display = if FEATURES.is_empty() {
                "(none)".to_string()
            } else {
                FEATURES.join(", ")
            };
            println!("commands:  {}", COMMANDS.join(", "));
            println!("formats:   {formats_display}");
            println!("features:  {features_display}");
        }
        OutputFormat::Json => {
            let payload = json!({
                "commands": COMMANDS,
                "formats": FORMATS,
                "features": FEATURES,
            });
            let report = ReportBuilder::new()
                .extension("capabilities", payload)
                .build();
            let json_str = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json_str}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }
    Ok(())
}

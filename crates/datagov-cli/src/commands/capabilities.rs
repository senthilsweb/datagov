//! capabilities.rs — `datagov capabilities`: thin adapter, no business
//! logic.
//!
//! 1. Reports the compiled command set (`version`, `capabilities`),
//!    supported input formats (empty in Bolt 1 — honest, no format
//!    readers exist yet), and enabled cargo features (none defined in
//!    Bolt 1).
//! 2. Human rendering: a small table. `--output json`: the envelope with
//!    the payload under `extensions.capabilities`.

use crate::cli::OutputFormat;
use datagov_core::DatagovError;
use datagov_core::report::ReportBuilder;
use serde_json::json;

const COMMANDS: &[&str] = &["version", "capabilities"];
const FORMATS: &[&str] = &[];
const FEATURES: &[&str] = &[];

pub fn run(output: OutputFormat) -> Result<(), DatagovError> {
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
    }
    Ok(())
}

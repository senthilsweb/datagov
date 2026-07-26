//! pii.rs — `datagov pii scan | recognizers list | recognizers validate`:
//! thin adapters, no business logic.
//!
//! 1. **`scan`**: resolves the input `Format` (same detection as
//!    `inspect`/`profile`), builds the merged recognizer set (10
//!    built-ins, plus any `--recognizers` YAML overrides/additions via
//!    `datagov_pii::registry::merge`), and delegates the actual scan to
//!    `datagov_pii::scanner::scan` on a dedicated, current-thread Tokio
//!    runtime (same rationale as `profile`/`query`: deterministic
//!    execution over a single DataFusion target partition). Wraps the
//!    resulting `PiiSection` into the standard envelope
//!    (`input.uri`/`input.format`/`input.content_hash`). Human rendering
//!    is a table of findings (column/entity/confidence/match%/masked
//!    evidence) — **never** a raw value, under any output mode or error
//!    path. `--output csv` is rejected via `commands::reject_csv`.
//! 2. **`--fail-on <confidence>`**: after the full report has been
//!    printed (JSON or table — never suppressed), if any finding's
//!    `confidence >= threshold`, the process exits 12
//!    (`ExitCode::PiiFailed`) via an explicit `std::io::stdout().flush()`
//!    followed by `std::process::exit` — the one place in this command
//!    that bypasses the `Result`-based exit-code funnel in `main.rs`,
//!    because "succeed at printing, then fail the process" has no other
//!    representation in that funnel (every other exit path is "did not
//!    produce a report at all").
//! 3. **`recognizers list`**: the 10 built-ins' `{id, entity, confidence,
//!    pattern_count}` — table, or JSON under
//!    `extensions.pii_recognizers` (metadata about the tool, not a scan
//!    result, so it lives under `extensions` like `query`/`sql
//!    parse`'s results — not a `Sections` field).
//! 4. **`recognizers validate <path>`**: delegates to
//!    `datagov_pii::yaml::validate_recognizers_file`; exit 0 with a
//!    confirmation naming the recognizer count on success, exit 2 (via
//!    the loader's own `DatagovError::InvalidArgs`) naming the offending
//!    recognizer id and field on failure.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use comfy_table::Table;
use datagov_core::DatagovError;
use datagov_core::report::{Input, PiiSection, ReportBuilder, Sections};
use datagov_pii::RecognizerSummary;

use crate::cli::OutputFormat;
use crate::commands::reject_csv;

fn build_runtime() -> Result<tokio::runtime::Runtime, DatagovError> {
    // Single-threaded (current-thread) runtime — same rationale as
    // `commands::profile`/`commands::query`: a single DataFusion target
    // partition already makes execution deterministic, and pinning the
    // whole async call to one OS thread keeps that guarantee end to end.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DatagovError::internal(format!("failed to start async runtime: {e}")))
}

fn load_recognizers(
    recognizers_path: Option<&str>,
) -> Result<Vec<datagov_pii::Recognizer>, DatagovError> {
    let customs = match recognizers_path {
        Some(path) => datagov_pii::yaml::load_recognizers_file(Path::new(path))?,
        None => Vec::new(),
    };
    Ok(datagov_pii::registry::merge(customs))
}

#[allow(clippy::too_many_arguments)]
pub fn run_scan(
    path: &str,
    type_flag: Option<&str>,
    sample: Option<u64>,
    field: Option<&[String]>,
    recognizers_path: Option<&str>,
    fail_on: Option<f64>,
    output: OutputFormat,
) -> Result<(), DatagovError> {
    reject_csv(output, "pii scan")?;

    if path == "-" {
        return Err(DatagovError::invalid_args(
            "'pii scan' requires a real file path",
            "DataFusion table registration needs random file access; stdin is not supported for pii scan",
        ));
    }

    let format = datagov_data::format::detect(path, type_flag)?;
    let path_buf = PathBuf::from(path);
    let content_hash = datagov_core::report::content_hash_sha256(&path_buf).map_err(|source| {
        DatagovError::input_not_found(
            format!("input not found: {path} ({source})"),
            format!("check that {path} exists and is readable"),
        )
    })?;

    let recognizers = load_recognizers(recognizers_path)?;

    let runtime = build_runtime()?;
    let section = runtime.block_on(datagov_pii::scan(datagov_pii::ScanRequest {
        path: &path_buf,
        format,
        fields: field,
        sample,
        recognizers: &recognizers,
    }))?;

    let input = Input {
        uri: path.to_string(),
        format: format.as_str().to_string(),
        content_hash: Some(content_hash),
    };

    // Decided before the section is moved into the report below — the
    // report must be emitted in full regardless of this outcome, and
    // never in place of it.
    let threshold_breached = fail_on.is_some_and(|threshold| {
        section
            .findings
            .iter()
            .any(|finding| finding.confidence >= threshold)
    });

    match output {
        OutputFormat::Table => render_scan_table(&section),
        OutputFormat::Json => {
            let report = ReportBuilder::new()
                .input(input)
                .sections(Sections {
                    pii: Some(section),
                    ..Default::default()
                })
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    if threshold_breached {
        let _ = std::io::stdout().flush();
        std::process::exit(datagov_core::ExitCode::PiiFailed.code());
    }

    Ok(())
}

fn render_scan_table(section: &PiiSection) {
    println!(
        "scanned {} column(s), {} row(s){}",
        section.scanned_columns,
        section.scanned_rows,
        section
            .sample_size
            .map(|n| format!(" (--sample {n})"))
            .unwrap_or_default()
    );

    if section.findings.is_empty() {
        println!("no PII findings.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec![
        "column",
        "entity",
        "recognizer",
        "confidence",
        "match %",
        "evidence",
    ]);
    for finding in &section.findings {
        table.add_row(vec![
            finding.column.clone(),
            finding.entity.clone(),
            finding.recognizer.clone(),
            format!("{:.2}", finding.confidence),
            format!("{:.1}", finding.match_percentage),
            finding.sample_evidence.join(", "),
        ]);
    }
    println!("{table}");
}

pub fn run_recognizers_list(output: OutputFormat) -> Result<(), DatagovError> {
    reject_csv(output, "pii recognizers list")?;

    let summaries: Vec<RecognizerSummary> = datagov_pii::builtins::builtin_recognizers()
        .iter()
        .map(|r| r.summary())
        .collect();

    match output {
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_header(vec!["id", "entity", "confidence", "patterns"]);
            for summary in &summaries {
                table.add_row(vec![
                    summary.id.clone(),
                    summary.entity.clone(),
                    format!("{:.2}", summary.confidence),
                    summary.pattern_count.to_string(),
                ]);
            }
            println!("{table}");
        }
        OutputFormat::Json => {
            let payload = serde_json::to_value(&summaries).map_err(|e| {
                DatagovError::internal(format!("failed to render recognizer list: {e}"))
            })?;
            let report = ReportBuilder::new()
                .extension("pii_recognizers", payload)
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

pub fn run_recognizers_validate(path: &str, output: OutputFormat) -> Result<(), DatagovError> {
    reject_csv(output, "pii recognizers validate")?;

    let count = datagov_pii::yaml::validate_recognizers_file(Path::new(path))?;

    match output {
        OutputFormat::Table => {
            println!("'{path}' is valid: {count} recognizer(s) loaded.");
        }
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "path": path,
                "valid": true,
                "recognizer_count": count,
            });
            let report = ReportBuilder::new()
                .extension("pii_recognizers_validate", payload)
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

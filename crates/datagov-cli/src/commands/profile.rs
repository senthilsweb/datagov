//! profile.rs — `datagov profile`: thin adapter, no business logic.
//!
//! 1. Resolves the input `Format` (same detection as `inspect`: `--type`
//!    flag, then extension, then content sniffing) and rejects `-`
//!    (stdin) — DataFusion table registration needs a real, seekable
//!    file.
//! 2. Delegates the actual column-statistics computation to
//!    `datagov_data::profile::compute_profile` (run to completion on a
//!    dedicated, single-threaded Tokio runtime — `datagov`'s command
//!    dispatch is otherwise synchronous; DataFusion is async-only. The
//!    runtime is deliberately current-thread, not the multi-thread
//!    default: see the comment at its construction below for why.
//! 3. Wraps the resulting `ProfileSection` into the `ReportBuilder`,
//!    same envelope shape as `inspect`: `input.uri`/`input.format`/
//!    `input.content_hash` set from the file, `sections.profile`
//!    populated.
//! 4. Human rendering (`comfy-table`): one row per profiled column
//!    summarizing type, nulls, distinct/uniqueness, and (numeric/
//!    string-only) mean/stddev or string-length stats.
//! 5. `--output csv` is rejected (exit 2) via `commands::reject_csv` —
//!    `profile` only supports `json`/`table` per PRD §10.2.

use std::path::PathBuf;

use comfy_table::Table;
use datagov_core::DatagovError;
use datagov_core::report::{Input, ProfileSection, ReportBuilder, Sections};

use crate::cli::OutputFormat;
use crate::commands::reject_csv;

pub fn run(
    path: &str,
    type_flag: Option<&str>,
    columns: Option<&[String]>,
    sample: Option<u64>,
    output: OutputFormat,
) -> Result<(), DatagovError> {
    reject_csv(output, "profile")?;

    if path == "-" {
        return Err(DatagovError::invalid_args(
            "'profile' requires a real file path",
            "DataFusion table registration needs random file access; stdin is not supported for profile",
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

    // A single-threaded (current-thread) runtime, not the multi-thread
    // default: DataFusion's `APPROX_PERCENTILE_CONT` t-digest merge is
    // not perfectly bit-for-bit associative, so running it across a
    // multi-thread Tokio pool was observed to occasionally produce a
    // last-ULP-different result between otherwise identical runs —
    // incompatible with the HARD determinism requirement. Combined with
    // `crate::engine::new_session_context`'s single DataFusion target
    // partition, this pins execution to one OS thread end to end.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DatagovError::internal(format!("failed to start async runtime: {e}")))?;
    let profile_section = runtime.block_on(datagov_data::profile::compute_profile(
        &path_buf, format, columns, sample,
    ))?;

    let input = Input {
        uri: path.to_string(),
        format: format.as_str().to_string(),
        content_hash: Some(content_hash),
    };

    match output {
        OutputFormat::Table => render_table(&profile_section),
        OutputFormat::Json => {
            let report = ReportBuilder::new()
                .input(input)
                .sections(Sections {
                    profile: Some(profile_section),
                    ..Default::default()
                })
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

fn render_table(section: &ProfileSection) {
    if let Some(n) = section.sample_size {
        println!("sample size: {n} rows");
    }

    let mut table = Table::new();
    table.set_header(vec![
        "column",
        "type",
        "nulls %",
        "distinct",
        "unique %",
        "mean",
        "stddev",
        "semantic type",
        "identifier",
    ]);
    for column in &section.columns {
        table.add_row(vec![
            column.name.clone(),
            data_type_label(column.data_type).to_string(),
            format!("{:.1}", column.null_percentage),
            column.distinct_count.to_string(),
            format!("{:.1}", column.uniqueness_percentage),
            column
                .mean
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "-".to_string()),
            column
                .stddev
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "-".to_string()),
            column.semantic_type.clone(),
            column.possible_identifier.to_string(),
        ]);
    }
    println!("{table}");
}

fn data_type_label(data_type: datagov_core::report::DataType) -> &'static str {
    use datagov_core::report::DataType;
    match data_type {
        DataType::Boolean => "boolean",
        DataType::Integer => "integer",
        DataType::Float => "float",
        DataType::String => "string",
        DataType::Object => "object",
        DataType::Array => "array",
        DataType::Null => "null",
        DataType::Mixed => "mixed",
    }
}

#[cfg(test)]
mod tests {
    use super::data_type_label;
    use datagov_core::report::DataType;

    #[test]
    fn data_type_label_covers_every_variant() {
        assert_eq!(data_type_label(DataType::Boolean), "boolean");
        assert_eq!(data_type_label(DataType::Integer), "integer");
        assert_eq!(data_type_label(DataType::Float), "float");
        assert_eq!(data_type_label(DataType::String), "string");
        assert_eq!(data_type_label(DataType::Object), "object");
        assert_eq!(data_type_label(DataType::Array), "array");
        assert_eq!(data_type_label(DataType::Null), "null");
        assert_eq!(data_type_label(DataType::Mixed), "mixed");
    }
}

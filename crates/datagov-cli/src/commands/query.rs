//! query.rs — `datagov query`: thin adapter, no business logic.
//!
//! 1. Delegates SQL execution (file-path resolution, DataFusion
//!    execution, bounding) to `datagov_data::query::run_query`, run to
//!    completion on a dedicated Tokio runtime.
//! 2. **JSON** output: the full envelope, deliberately with no `input`
//!    section — a query can reference zero, one, or many files, so
//!    there's no single dataset to describe there. The `QueryResult` is
//!    serialized into `extensions.query`.
//! 3. **table** output: `comfy-table` rendering of `columns` + the
//!    bounded `rows`.
//! 4. **csv** output: raw CSV to stdout — header + bounded rows, nothing
//!    else (no envelope; pipeable). This is the one command that accepts
//!    `--output csv`.

use comfy_table::Table;
use datagov_core::DatagovError;
use datagov_core::report::ReportBuilder;
use datagov_data::query::{DEFAULT_QUERY_LIMIT, QueryResult};

use crate::cli::OutputFormat;

pub fn run(sql: &str, limit: Option<usize>, output: OutputFormat) -> Result<(), DatagovError> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT);

    // Single-threaded runtime — see `commands::profile::run` for why
    // (DataFusion's approximate-aggregate merges are not perfectly
    // bit-for-bit associative across a multi-thread Tokio pool).
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DatagovError::internal(format!("failed to start async runtime: {e}")))?;
    let result = runtime.block_on(datagov_data::query::run_query(sql, limit))?;

    match output {
        OutputFormat::Csv => print_csv(&result)?,
        OutputFormat::Table => render_table(&result),
        OutputFormat::Json => {
            let payload = serde_json::to_value(&result).map_err(|e| {
                DatagovError::internal(format!("failed to render query result: {e}"))
            })?;
            let report = ReportBuilder::new().extension("query", payload).build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
    }

    Ok(())
}

fn render_table(result: &QueryResult) {
    let mut table = Table::new();
    table.set_header(result.columns.clone());
    for row in &result.rows {
        table.add_row(row.iter().map(render_cell).collect::<Vec<_>>());
    }
    println!("{table}");
    if result.truncated {
        println!(
            "(showing {} of {} rows)",
            result.row_count_returned, result.total_row_count
        );
    }
}

fn print_csv(result: &QueryResult) -> Result<(), DatagovError> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer
        .write_record(&result.columns)
        .map_err(|e| DatagovError::internal(format!("failed to write CSV header: {e}")))?;
    for row in &result.rows {
        let record: Vec<String> = row.iter().map(render_cell).collect();
        writer
            .write_record(&record)
            .map_err(|e| DatagovError::internal(format!("failed to write CSV row: {e}")))?;
    }
    writer
        .flush()
        .map_err(|e| DatagovError::internal(format!("failed to flush CSV output: {e}")))?;
    Ok(())
}

fn render_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_cell;

    #[test]
    fn null_renders_as_empty_string() {
        assert_eq!(render_cell(&serde_json::Value::Null), "");
    }

    #[test]
    fn string_renders_verbatim() {
        assert_eq!(
            render_cell(&serde_json::Value::String("CA".to_string())),
            "CA"
        );
    }

    #[test]
    fn number_renders_via_display() {
        assert_eq!(render_cell(&serde_json::json!(42)), "42");
    }
}

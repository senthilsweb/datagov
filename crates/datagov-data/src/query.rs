//! query.rs — `datagov query` execution: resolves quoted file-path
//! references in the SQL text into registered DataFusion tables, runs
//! the (rewritten) query, and returns a bounded `QueryResult`.
//!
//! 1. `DEFAULT_QUERY_LIMIT`, the row bound applied when `--limit` is
//!    absent.
//! 2. `QueryResult`/`QueryExecutionStats`, the query-specific payload
//!    the CLI serializes into `extensions.query` — not a `Sections`
//!    field, since a query can reference zero, one, or many files, so
//!    there is no single dataset for an `input` block to describe (see
//!    the Bolt 3 brief).
//! 3. `resolve_file_references` is the file-path-resolution mechanism:
//!    it regex-scans the SQL text for single-quoted string literals
//!    whose content has a recognized dataset-file extension
//!    (`datagov_data::format::Format`). CSV/Parquet matches are
//!    validated to exist, registered via
//!    `crate::engine::register_dataset_table` (de-duplicated by path,
//!    tracked in `sources` in first-seen order), and the literal is
//!    rewritten in the query text to the registered (double-quoted)
//!    table identifier; the rewritten text is what actually executes.
//!    Any other recognized-but-unsupported extension (TSV/JSON/JSONL) is
//!    `DatagovError::UnsupportedInput` (exit 4) — PRD §10.3 scopes
//!    `query` to CSV/Parquet only. This is "approach 2" from the Bolt 3
//!    brief; see the Bolt 3 report for why it was chosen over
//!    DataFusion's native `SessionContext::enable_url_table` (verified
//!    working, but unable to enforce our exit-code contract for
//!    not-found/unsupported files without parsing DataFusion's own error
//!    text).
//! 4. `run_query` builds the session, resolves/rewrites references,
//!    executes via `ctx.sql(...)` (a parse/plan failure here is
//!    `InvalidArgs`, exit 2 — invalid SQL), materializes the full result
//!    (documented simplification: no streaming/pushdown optimization in
//!    this bolt), then applies the row bound, reporting
//!    `truncated`/`total_row_count`.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use datafusion::prelude::SessionContext;
use datagov_core::DatagovError;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::engine::{new_session_context, register_dataset_table};
use crate::format::Format;
use crate::rows::{batches_to_row_maps, row_map_to_ordered_values};

/// The row bound applied to `query` output when `--limit` is not given.
pub const DEFAULT_QUERY_LIMIT: usize = 1000;

/// The `query`-specific payload serialized into `extensions.query`.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    /// Resolved file paths, in first-seen order.
    pub sources: Vec<String>,
    pub columns: Vec<String>,
    /// Bounded to at most the applied limit.
    pub rows: Vec<Vec<Value>>,
    pub row_count_returned: u64,
    /// The full result size, before truncation.
    pub total_row_count: u64,
    pub truncated: bool,
    pub execution: QueryExecutionStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryExecutionStats {
    pub duration_ms: u64,
}

fn quoted_literal_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"'([^']*)'").expect("static regex is valid"))
}

/// A quoted literal's content parses as a recognized dataset-file
/// extension (any of `datagov_data::format::Format`'s five), or it
/// isn't a file reference at all.
fn extension_format(literal: &str) -> Option<Format> {
    let ext = Path::new(literal).extension()?.to_str()?;
    Format::from_flag(ext).ok()
}

/// Scan `sql` for single-quoted file-path literals, register each unique
/// CSV/Parquet reference as a DataFusion table, and return the rewritten
/// SQL text plus the resolved source paths in first-seen order.
async fn resolve_file_references(
    ctx: &SessionContext,
    sql: &str,
) -> Result<(String, Vec<String>), DatagovError> {
    let mut rewritten = String::with_capacity(sql.len());
    let mut last_end = 0;
    let mut table_names: HashMap<String, String> = HashMap::new();
    let mut used_names: HashSet<String> = HashSet::new();
    let mut sources: Vec<String> = Vec::new();

    for cap in quoted_literal_regex().captures_iter(sql) {
        let whole = cap.get(0).expect("capture group 0 always matches");
        let literal = &cap[1];

        let Some(format) = extension_format(literal) else {
            continue;
        };

        let table_name = match format {
            Format::Csv | Format::Parquet => {
                if let Some(existing) = table_names.get(literal) {
                    existing.clone()
                } else {
                    let path = Path::new(literal);
                    if !path.exists() {
                        return Err(DatagovError::input_not_found(
                            format!("query references a file that does not exist: {literal}"),
                            format!("check that {literal} exists and is readable"),
                        ));
                    }
                    let name = register_dataset_table(ctx, path, format, &mut used_names).await?;
                    table_names.insert(literal.to_string(), name.clone());
                    sources.push(literal.to_string());
                    name
                }
            }
            other => {
                return Err(DatagovError::unsupported_input(
                    format!(
                        "query only supports CSV and Parquet files; '{literal}' is {}",
                        other.as_str()
                    ),
                    "reference a .csv or .parquet file".to_string(),
                ));
            }
        };

        rewritten.push_str(&sql[last_end..whole.start()]);
        rewritten.push('"');
        rewritten.push_str(&table_name);
        rewritten.push('"');
        last_end = whole.end();
    }
    rewritten.push_str(&sql[last_end..]);

    Ok((rewritten, sources))
}

/// Execute `sql`, bounding the returned rows at `limit`. See the module
/// docs for the file-reference resolution mechanism.
pub async fn run_query(sql: &str, limit: usize) -> Result<QueryResult, DatagovError> {
    let started = Instant::now();
    let ctx = new_session_context();

    let (rewritten_sql, sources) = resolve_file_references(&ctx, sql).await?;

    let df = ctx.sql(&rewritten_sql).await.map_err(|source| {
        DatagovError::invalid_args(
            format!("invalid SQL: {source}"),
            "check the query syntax".to_string(),
        )
    })?;

    let columns: Vec<String> = df
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    let batches = df
        .collect()
        .await
        .map_err(|source| DatagovError::internal(format!("failed to execute query: {source}")))?;

    let row_maps = batches_to_row_maps(&batches)?;
    let total_row_count = row_maps.len() as u64;
    let truncated = row_maps.len() > limit;
    let rows: Vec<Vec<Value>> = row_maps
        .iter()
        .take(limit)
        .map(|row| row_map_to_ordered_values(row, &columns))
        .collect();
    let row_count_returned = rows.len() as u64;

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(QueryResult {
        sources,
        columns,
        rows,
        row_count_returned,
        total_row_count,
        truncated,
        execution: QueryExecutionStats { duration_ms },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn examples_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    #[tokio::test]
    async fn aggregates_a_parquet_fixture_with_a_quoted_path() {
        let path = examples_dir().join("customers.parquet");
        let sql = format!(
            "SELECT state, COUNT(*) AS total FROM '{}' GROUP BY state ORDER BY state LIMIT 3",
            path.display()
        );
        let result = run_query(&sql, DEFAULT_QUERY_LIMIT).await.unwrap();
        assert_eq!(result.sources, vec![path.display().to_string()]);
        assert_eq!(result.columns, vec!["state", "total"]);
        assert!(!result.rows.is_empty());
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn missing_file_is_input_not_found() {
        let err = run_query(
            "SELECT * FROM 'does-not-exist.parquet'",
            DEFAULT_QUERY_LIMIT,
        )
        .await
        .unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InputNotFound);
    }

    #[tokio::test]
    async fn unsupported_extension_is_unsupported_input() {
        let path = examples_dir().join("customers.json");
        let sql = format!("SELECT * FROM '{}'", path.display());
        let err = run_query(&sql, DEFAULT_QUERY_LIMIT).await.unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }

    #[tokio::test]
    async fn invalid_sql_is_invalid_args() {
        let err = run_query("SELECT FROM WHERE", DEFAULT_QUERY_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }

    #[tokio::test]
    async fn bounded_output_reports_truncation() {
        let path = examples_dir().join("customers.parquet");
        let sql = format!("SELECT * FROM '{}'", path.display());
        let result = run_query(&sql, 5).await.unwrap();
        assert_eq!(result.row_count_returned, 5);
        assert!(result.truncated);
        assert!(result.total_row_count > 5);
    }
}

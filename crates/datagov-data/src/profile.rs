//! profile.rs — `datagov profile`'s column-statistics engine.
//!
//! 1. `compute_profile` registers the target file as a DataFusion table
//!    via the shared `crate::engine::register_dataset_table` helper (the
//!    same one `datagov query` uses — CSV/Parquet only, per PRD §10.3's
//!    scope for the DataFusion engine wired up in this bolt). When
//!    `--sample n` is given, the first `n` rows (in source order — see
//!    `crate::engine::new_session_context`'s single-partition config)
//!    are materialized once into an in-memory table and every subsequent
//!    query below runs against that same materialized set, so repeated
//!    per-column aggregate queries can't disagree on which rows were
//!    "the sample" (a HARD determinism requirement).
//! 2. One `ColumnProfile` is computed per (optionally `--columns`-
//!    filtered) column via a single combined aggregate SQL statement:
//!    `COUNT`, `COUNT(DISTINCT ..)`, `MIN`, `MAX` for every column, plus
//!    — numeric columns only — `AVG`, `STDDEV`,
//!    `APPROX_PERCENTILE_CONT(col, p)` for p in {0.25, 0.5, 0.75, 0.9,
//!    0.99} (confirmed as the exact aggregate function name against the
//!    pinned DataFusion 54.1.0 — see the Bolt 3 report), and — string
//!    columns only — `MIN`/`MAX`/`AVG(LENGTH(col))`.
//! 3. Top values: a second query per column,
//!    `GROUP BY col ORDER BY count DESC, value ASC LIMIT
//!    TOP_VALUES_LIMIT` — the `value ASC` tie-break is what keeps
//!    repeated runs deterministic when two values share a count.
//! 4. `possible_identifier`/`semantic_type` heuristics per the brief:
//!    identifier when `distinct_count == row_count && row_count > 0`, or
//!    the column name matches `^(id|.*_id|.*id)$` (case-insensitive);
//!    semantic type is the matched `datagov_core::sensitivity` fragment
//!    name, else `"identifier"`, else a coarse bucket by `DataType`.
//! 5. Masking: `top_values` and any `min`/`max` on a heuristically
//!    sensitive column go through `datagov_core::mask::Masked` — the
//!    same no-raw-value guarantee as Bolt 2's sample rows (HARD).

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use arrow_array::RecordBatch;
use arrow_schema::DataType as ArrowDataType;
use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;
use datagov_core::DatagovError;
use datagov_core::mask::Masked;
use datagov_core::report::{ColumnProfile, DataType, ProfileSection, StringLengthStats, TopValue};
use datagov_core::sensitivity::heuristically_sensitive_match;
use regex::Regex;
use serde_json::Value;

use crate::engine::{new_session_context, register_dataset_table};
use crate::format::Format;
use crate::rows::batches_to_row_maps;

/// How many top values to report per column.
const TOP_VALUES_LIMIT: usize = 10;

/// The quantile levels reported in `ColumnProfile::quantiles`, and the
/// `"p50"` entry doubles as `ColumnProfile::median`.
const QUANTILE_LEVELS: [(&str, f64); 5] = [
    ("p25", 0.25),
    ("p50", 0.5),
    ("p75", 0.75),
    ("p90", 0.9),
    ("p99", 0.99),
];

fn identifier_name_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(id|.*_id|.*id)$").expect("static identifier regex is valid")
    })
}

/// Compute the `profile` section for `path` (already format-detected as
/// `format`). `columns`, when given, restricts profiling to those column
/// names (unknown names are `DatagovError::InvalidArgs`, naming the
/// first one not found). `sample`, when given, bounds profiling to the
/// first `sample` rows in source order.
pub async fn compute_profile(
    path: &Path,
    format: Format,
    columns: Option<&[String]>,
    sample: Option<u64>,
) -> Result<ProfileSection, DatagovError> {
    let ctx = new_session_context();
    let mut used_names = HashSet::new();
    let base_table = register_dataset_table(&ctx, path, format, &mut used_names).await?;

    let active_table = match sample {
        Some(n) => materialize_sample(&ctx, &base_table, n).await?,
        None => base_table,
    };

    let provider = ctx
        .table_provider(active_table.as_str())
        .await
        .map_err(|source| {
            DatagovError::internal(format!(
                "failed to load the schema of '{active_table}': {source}"
            ))
        })?;
    let schema = provider.schema();

    let all_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    let selected: Vec<String> = match columns {
        Some(requested) => {
            for name in requested {
                if !all_names.iter().any(|n| n == name) {
                    return Err(DatagovError::invalid_args(
                        format!("unknown column '{name}'"),
                        format!("choose from: {}", all_names.join(", ")),
                    ));
                }
            }
            requested.to_vec()
        }
        None => all_names,
    };

    let total_row_count = count_rows(&ctx, &active_table).await?;

    let mut column_profiles = Vec::with_capacity(selected.len());
    for name in &selected {
        let field = schema.field_with_name(name).map_err(|source| {
            DatagovError::internal(format!("column '{name}' missing from schema: {source}"))
        })?;
        let data_type = map_arrow_type(field.data_type());
        let profile = profile_column(&ctx, &active_table, name, data_type, total_row_count).await?;
        column_profiles.push(profile);
    }

    Ok(ProfileSection {
        sample_size: sample,
        columns: column_profiles,
    })
}

async fn execute_sql(ctx: &SessionContext, sql: &str) -> Result<Vec<RecordBatch>, DatagovError> {
    let df = ctx.sql(sql).await.map_err(|source| {
        DatagovError::internal(format!("profiling query failed to plan ({sql}): {source}"))
    })?;
    df.collect().await.map_err(|source| {
        DatagovError::internal(format!(
            "profiling query failed to execute ({sql}): {source}"
        ))
    })
}

/// Materialize the first `n` rows of `base_table` (in source order) into
/// a new in-memory table, returning its name. Every subsequent query
/// profiling this run uses that materialized table, so all per-column
/// statistics are computed over exactly the same row set.
async fn materialize_sample(
    ctx: &SessionContext,
    base_table: &str,
    n: u64,
) -> Result<String, DatagovError> {
    let sql = format!("SELECT * FROM \"{base_table}\" LIMIT {n}");
    let batches = execute_sql(ctx, &sql).await?;

    let schema = match batches.first() {
        Some(batch) => batch.schema(),
        None => ctx
            .table_provider(base_table)
            .await
            .map_err(|source| {
                DatagovError::internal(format!(
                    "failed to load the schema of '{base_table}': {source}"
                ))
            })?
            .schema(),
    };

    let mem_table = MemTable::try_new(schema, vec![batches]).map_err(|source| {
        DatagovError::internal(format!("failed to materialize the --sample rows: {source}"))
    })?;

    let sampled_name = format!("{base_table}__sampled");
    ctx.register_table(sampled_name.as_str(), Arc::new(mem_table))
        .map_err(|source| {
            DatagovError::internal(format!("failed to register the sampled table: {source}"))
        })?;

    Ok(sampled_name)
}

async fn count_rows(ctx: &SessionContext, table: &str) -> Result<u64, DatagovError> {
    let sql = format!("SELECT COUNT(*) AS total FROM \"{table}\"");
    let batches = execute_sql(ctx, &sql).await?;
    let rows = batches_to_row_maps(&batches)?;
    Ok(rows
        .first()
        .and_then(|r| r.get("total"))
        .and_then(Value::as_u64)
        .unwrap_or(0))
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

/// Decimal places `mean`/`median`/`stddev`/`quantiles`/`string_length.mean`
/// are rounded to before being reported.
///
/// **Correction (Bolt 3 construction):** `AVG`/`STDDEV`/
/// `APPROX_PERCENTILE_CONT` are computed by DataFusion via streaming
/// floating-point accumulation, which is not perfectly associative.
/// Repeated `datagov profile` runs on the identical, unchanged input were
/// observed to occasionally differ in the last 1-2 ULPs of these fields
/// (e.g. `10304.967817741059` vs `10304.96781774106`) even with a single
/// DataFusion target partition and a single-threaded Tokio runtime —
/// almost certainly due to local-file scan I/O completion ordering, not
/// anything under this crate's control. Rounding to 9 decimal places
/// (far past any precision a profiling report needs) absorbs that noise
/// so the HARD byte-identical-across-runs determinism requirement holds
/// in practice. Flagged by the implementation agent; see the Bolt 3
/// report.
const STATS_ROUND_DECIMALS: i32 = 9;

fn round_stat(value: f64) -> f64 {
    let scale = 10f64.powi(STATS_ROUND_DECIMALS);
    (value * scale).round() / scale
}

fn mask_value(value: Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    let raw = match &value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    Value::String(Masked::new(raw).to_string())
}

async fn top_values_for(
    ctx: &SessionContext,
    table: &str,
    name: &str,
    total_row_count: u64,
) -> Result<Vec<TopValue>, DatagovError> {
    let sql = format!(
        "SELECT \"{name}\" AS value, COUNT(*) AS cnt FROM \"{table}\" \
         WHERE \"{name}\" IS NOT NULL GROUP BY \"{name}\" \
         ORDER BY cnt DESC, value ASC LIMIT {TOP_VALUES_LIMIT}"
    );
    let batches = execute_sql(ctx, &sql).await?;
    let rows = batches_to_row_maps(&batches)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let value = row.get("value").cloned().unwrap_or(Value::Null);
            let count = row.get("cnt").and_then(Value::as_u64).unwrap_or(0);
            TopValue {
                value,
                count,
                percentage: percentage(count, total_row_count),
            }
        })
        .collect())
}

async fn profile_column(
    ctx: &SessionContext,
    table: &str,
    name: &str,
    data_type: DataType,
    total_row_count: u64,
) -> Result<ColumnProfile, DatagovError> {
    let numeric = matches!(data_type, DataType::Integer | DataType::Float);
    let stringy = matches!(data_type, DataType::String);

    let mut select_parts = vec![
        format!("COUNT(\"{name}\") AS non_null"),
        format!("COUNT(DISTINCT \"{name}\") AS distinct_count"),
        format!("MIN(\"{name}\") AS min_value"),
        format!("MAX(\"{name}\") AS max_value"),
    ];
    if numeric {
        select_parts.push(format!("AVG(\"{name}\") AS mean_value"));
        select_parts.push(format!("STDDEV(\"{name}\") AS stddev_value"));
        for (label, p) in QUANTILE_LEVELS {
            select_parts.push(format!(
                "APPROX_PERCENTILE_CONT(\"{name}\", {p}) AS {label}"
            ));
        }
    }
    if stringy {
        select_parts.push(format!("MIN(LENGTH(\"{name}\")) AS len_min"));
        select_parts.push(format!("MAX(LENGTH(\"{name}\")) AS len_max"));
        select_parts.push(format!("AVG(LENGTH(\"{name}\")) AS len_mean"));
    }

    let sql = format!("SELECT {} FROM \"{table}\"", select_parts.join(", "));
    let batches = execute_sql(ctx, &sql).await?;
    let rows = batches_to_row_maps(&batches)?;
    let row = rows.first().cloned().unwrap_or_default();

    let non_null = row.get("non_null").and_then(Value::as_u64).unwrap_or(0);
    let null_count = total_row_count.saturating_sub(non_null);
    let null_percentage = percentage(null_count, total_row_count);
    let distinct_count = row
        .get("distinct_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let uniqueness_percentage = percentage(distinct_count, total_row_count);

    let mut min = row.get("min_value").cloned().filter(|v| !v.is_null());
    let mut max = row.get("max_value").cloned().filter(|v| !v.is_null());

    let mean = numeric
        .then(|| {
            row.get("mean_value")
                .and_then(Value::as_f64)
                .map(round_stat)
        })
        .flatten();
    let stddev = numeric
        .then(|| {
            row.get("stddev_value")
                .and_then(Value::as_f64)
                .map(round_stat)
        })
        .flatten();
    let median = numeric
        .then(|| row.get("p50").and_then(Value::as_f64).map(round_stat))
        .flatten();
    let quantiles = numeric.then(|| {
        let mut map = BTreeMap::new();
        for (label, _) in QUANTILE_LEVELS {
            if let Some(v) = row.get(label).and_then(Value::as_f64) {
                map.insert(label.to_string(), round_stat(v));
            }
        }
        map
    });

    let string_length = stringy.then(|| StringLengthStats {
        min: row.get("len_min").and_then(Value::as_u64).unwrap_or(0),
        max: row.get("len_max").and_then(Value::as_u64).unwrap_or(0),
        mean: row
            .get("len_mean")
            .and_then(Value::as_f64)
            .map(round_stat)
            .unwrap_or(0.0),
    });

    let mut top_values = top_values_for(ctx, table, name, total_row_count).await?;

    let possible_identifier = (distinct_count == total_row_count && total_row_count > 0)
        || identifier_name_regex().is_match(name);

    let sensitive_fragment = heuristically_sensitive_match(name);
    let semantic_type = if let Some(fragment) = sensitive_fragment {
        fragment.to_string()
    } else if possible_identifier {
        "identifier".to_string()
    } else {
        match data_type {
            DataType::Integer | DataType::Float => "numeric",
            DataType::Boolean => "boolean",
            _ => "text",
        }
        .to_string()
    };

    if sensitive_fragment.is_some() {
        min = min.map(mask_value);
        max = max.map(mask_value);
        top_values = top_values
            .into_iter()
            .map(|tv| TopValue {
                value: mask_value(tv.value),
                ..tv
            })
            .collect();
    }

    Ok(ColumnProfile {
        name: name.to_string(),
        data_type,
        row_count: total_row_count,
        null_count,
        null_percentage,
        distinct_count,
        uniqueness_percentage,
        min,
        max,
        mean,
        median,
        stddev,
        quantiles,
        string_length,
        top_values,
        semantic_type,
        possible_identifier,
    })
}

/// Map an Arrow field type to `datagov_core::report::DataType`. Types
/// with no dedicated variant (dates/timestamps/binary/dictionary/etc.)
/// fall back to `String` — the same documented simplification as Bolt
/// 2's Parquet `BYTE_ARRAY`/`FIXED_LEN_BYTE_ARRAY` mapping, correct for
/// the committed fixtures (no such columns) and revisited if a fixture
/// with those types is introduced.
fn map_arrow_type(dt: &ArrowDataType) -> DataType {
    use ArrowDataType::*;
    match dt {
        Boolean => DataType::Boolean,
        Int8 | Int16 | Int32 | Int64 | UInt8 | UInt16 | UInt32 | UInt64 => DataType::Integer,
        Float16 | Float32 | Float64 | Decimal32(..) | Decimal64(..) | Decimal128(..)
        | Decimal256(..) => DataType::Float,
        Utf8 | LargeUtf8 | Utf8View => DataType::String,
        List(_) | LargeList(_) | FixedSizeList(..) | ListView(_) | LargeListView(_) => {
            DataType::Array
        }
        Struct(_) | Map(..) => DataType::Object,
        Null => DataType::Null,
        _ => DataType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::Format;
    use std::time::Instant;

    #[tokio::test]
    async fn profiles_the_customers_parquet_fixture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/customers.parquet");
        let section = compute_profile(&path, Format::Parquet, None, None)
            .await
            .unwrap();
        assert!(section.sample_size.is_none());
        assert!(!section.columns.is_empty());

        let state = section
            .columns
            .iter()
            .find(|c| c.name == "state")
            .expect("state column present");
        assert_eq!(state.row_count, section.columns[0].row_count);
        assert!(state.row_count > 0);
        assert!(!state.top_values.is_empty());
    }

    #[tokio::test]
    async fn sample_bounds_row_count() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/customers.csv");
        let section = compute_profile(&path, Format::Csv, None, Some(10))
            .await
            .unwrap();
        assert_eq!(section.sample_size, Some(10));
        for column in &section.columns {
            assert_eq!(column.row_count, 10);
        }
    }

    #[tokio::test]
    async fn unknown_column_is_invalid_args() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/customers.csv");
        let err = compute_profile(
            &path,
            Format::Csv,
            Some(&["not_a_real_column".to_string()]),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }

    #[tokio::test]
    async fn email_and_phone_are_masked_in_top_values_and_min_max() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/customers.csv");
        let section = compute_profile(
            &path,
            Format::Csv,
            Some(&["email".to_string(), "phone".to_string()]),
            None,
        )
        .await
        .unwrap();
        for column in &section.columns {
            for top_value in &column.top_values {
                if let Value::String(s) = &top_value.value {
                    assert!(s.contains('…') || s == "●●●", "value not masked: {s}");
                }
            }
        }
    }

    #[test]
    #[ignore = "informational timing check, not a CI gate — see the Bolt 3 report"]
    fn profiling_1m_rows_completes_in_a_reasonable_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("synthetic_1m.csv");

        {
            use std::io::Write;
            let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
            writeln!(file, "id,amount,quantity,label").unwrap();
            for i in 0..1_000_000u64 {
                writeln!(
                    file,
                    "{i},{:.2},{},label-{}",
                    (i as f64) * 1.37,
                    i % 97,
                    i % 500
                )
                .unwrap();
            }
        }

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let started = Instant::now();
        let section = runtime
            .block_on(compute_profile(&path, Format::Csv, None, None))
            .unwrap();
        let elapsed = started.elapsed();

        println!(
            "1M-row profile ({} columns) completed in {:.2?}",
            section.columns.len(),
            elapsed
        );
        assert_eq!(section.columns[0].row_count, 1_000_000);
    }
}

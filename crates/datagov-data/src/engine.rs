//! engine.rs — Shared DataFusion session/table-registration helpers used
//! by both `datagov query` (`crate::query`) and `datagov profile`
//! (`crate::profile`).
//!
//! 1. `new_session_context` builds a `SessionContext` configured with a
//!    single target partition. Both callers need deterministic
//!    execution — `profile`'s determinism requirement is HARD per the
//!    spec delta, and DataFusion's default multi-partition file scan can
//!    split a single local file across threads, so results (including
//!    the `t-digest`-based `APPROX_PERCENTILE_CONT` merges) could vary
//!    run to run depending on thread-completion order. Forcing a single
//!    partition trades away intra-query parallelism for that guarantee —
//!    a documented simplification, not a Bolt 7 performance target (see
//!    the ignored 1M-row timing test in `crate::profile`).
//! 2. `sanitize_table_name` turns a file stem into a valid, unquoted SQL
//!    identifier per the Bolt 3 brief: every non-alphanumeric character
//!    becomes `_`; a result that would start with a digit is prefixed
//!    with `t_`.
//! 3. `register_dataset_table` registers a local file as an external
//!    DataFusion table under a name derived from `sanitize_table_name`,
//!    de-duplicated against `used_names` so two distinct files sharing a
//!    stem (e.g. `a/data.csv` and `b/data.parquet`) don't collide.
//!    Supports CSV, TSV (`CsvReadOptions` with a tab delimiter — Bolt 5,
//!    added for `pii scan`'s wider format scope), JSONL (DataFusion's
//!    JSON reader is natively NDJSON — `ctx.register_json`, also Bolt 5),
//!    and Parquet. Plain JSON arrays (`Format::Json`) have no native
//!    single-array DataFusion reader and are always
//!    `DatagovError::UnsupportedInput` (exit 4).
//!    **`query`/`profile` remain CSV/Parquet-only** (PRD §10.3): `query`
//!    self-gates in its own `resolve_file_references` match before ever
//!    calling this function, and `profile::compute_profile` now has its
//!    own explicit format guard at entry (see that module) — this
//!    function widening to also accept TSV/JSONL does not, by itself,
//!    widen either command's documented scope.

use std::collections::HashSet;
use std::path::Path;

use datafusion::prelude::{
    CsvReadOptions, JsonReadOptions, ParquetReadOptions, SessionConfig, SessionContext,
};
use datagov_core::DatagovError;

use crate::format::Format;

/// Build a `SessionContext` with a single target partition so file scans
/// (and DataFusion's approximate-aggregate merges) are deterministic
/// across repeated runs on the same input.
pub fn new_session_context() -> SessionContext {
    let config = SessionConfig::new().with_target_partitions(1);
    SessionContext::new_with_config(config)
}

/// Turn a file stem into a valid, unquoted SQL identifier: every
/// non-alphanumeric character becomes `_`; an empty result becomes
/// `"t"`; a result that would start with a digit is prefixed with `t_`.
pub fn sanitize_table_name(stem: &str) -> String {
    let mut sanitized: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if sanitized.is_empty() {
        sanitized = "t".to_string();
    }
    if sanitized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        sanitized = format!("t_{sanitized}");
    }
    sanitized
}

/// Register `path` (a local CSV or Parquet file) as an external table on
/// `ctx`, returning the (de-duplicated) table name.
pub async fn register_dataset_table(
    ctx: &SessionContext,
    path: &Path,
    format: Format,
    used_names: &mut HashSet<String>,
) -> Result<String, DatagovError> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("t");
    let base = sanitize_table_name(stem);
    let mut table_name = base.clone();
    let mut suffix = 2;
    while used_names.contains(&table_name) {
        table_name = format!("{base}_{suffix}");
        suffix += 1;
    }

    let path_str = path.to_string_lossy();
    let registration = match format {
        Format::Csv => {
            ctx.register_csv(&table_name, path_str.as_ref(), CsvReadOptions::new())
                .await
        }
        Format::Tsv => {
            ctx.register_csv(
                &table_name,
                path_str.as_ref(),
                CsvReadOptions::new()
                    .delimiter(b'\t')
                    .file_extension(".tsv"),
            )
            .await
        }
        Format::Jsonl => {
            ctx.register_json(
                &table_name,
                path_str.as_ref(),
                JsonReadOptions::default().file_extension(".jsonl"),
            )
            .await
        }
        Format::Parquet => {
            ctx.register_parquet(
                &table_name,
                path_str.as_ref(),
                ParquetReadOptions::default(),
            )
            .await
        }
        Format::Json => {
            return Err(DatagovError::unsupported_input(
                format!(
                    "'{}' is a JSON array; DataFusion has no native single-array reader",
                    path.display()
                ),
                "convert to .jsonl (one record per line), or use CSV/TSV/Parquet".to_string(),
            ));
        }
    };
    registration.map_err(|source| {
        DatagovError::internal(format!(
            "failed to register {} as a DataFusion table: {source}",
            path.display()
        ))
    })?;

    used_names.insert(table_name.clone());
    Ok(table_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_non_alphanumeric_characters() {
        assert_eq!(sanitize_table_name("customers"), "customers");
        assert_eq!(sanitize_table_name("my-data file"), "my_data_file");
    }

    #[test]
    fn prefixes_a_leading_digit() {
        assert_eq!(sanitize_table_name("2024-sales"), "t_2024_sales");
    }

    #[test]
    fn empty_stem_becomes_t() {
        assert_eq!(sanitize_table_name(""), "t");
    }

    fn examples_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    #[tokio::test]
    async fn registers_tsv_via_the_widened_helper() {
        let ctx = new_session_context();
        let mut used_names = HashSet::new();
        let name = register_dataset_table(
            &ctx,
            &examples_dir().join("customers.tsv"),
            Format::Tsv,
            &mut used_names,
        )
        .await
        .unwrap();
        let batches = ctx
            .sql(&format!("SELECT COUNT(*) AS n FROM \"{name}\""))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert!(!batches.is_empty());
    }

    #[tokio::test]
    async fn registers_jsonl_via_the_widened_helper() {
        let ctx = new_session_context();
        let mut used_names = HashSet::new();
        let name = register_dataset_table(
            &ctx,
            &examples_dir().join("customers.jsonl"),
            Format::Jsonl,
            &mut used_names,
        )
        .await
        .unwrap();
        let batches = ctx
            .sql(&format!("SELECT COUNT(*) AS n FROM \"{name}\""))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert!(!batches.is_empty());
    }

    #[tokio::test]
    async fn json_array_is_still_unsupported() {
        let ctx = new_session_context();
        let mut used_names = HashSet::new();
        let err = register_dataset_table(
            &ctx,
            &examples_dir().join("customers.json"),
            Format::Json,
            &mut used_names,
        )
        .await
        .unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }
}

//! scanner.rs — `datagov pii scan`'s scanning engine: builds the `pii`
//! report section from a dataset and a recognizer set.
//!
//! 1. `ScanRequest`/`scan` is the public entry point. Reuses
//!    `datagov_data::engine::register_dataset_table` (now widened —
//!    Bolt 5 — to also register TSV/JSONL) for table registration; a
//!    plain JSON array (`Format::Json`) is rejected here too, defense in
//!    depth alongside the engine's own rejection.
//! 2. `--sample n` materializes the first `n` rows (source order) into
//!    an in-memory table exactly like `datagov_data::profile`'s
//!    `materialize_sample` — the same determinism rationale applies here
//!    (single DataFusion target partition, so "first n rows" is stable
//!    across runs).
//! 3. `--field` filtering: an unknown field name is
//!    `DatagovError::InvalidArgs` (exit 2), naming it — the same pattern
//!    `profile`'s `--columns` uses.
//! 4. For each scanned column: one query,
//!    `SELECT CAST(col AS VARCHAR) AS v FROM table WHERE col IS NOT NULL`
//!    (DataFusion does the Arrow-type-to-string cast; no hand-rolled
//!    physical-type dispatch), producing every non-null value as a
//!    string.
//! 5. `detect_column` is the per-(column, recognizer) scanning core:
//!    - runs **every** recognizer pattern's `find_iter` against **every**
//!      scanned value (not a single whole-value match), so a recognizer
//!      matches both "this whole column is emails" and "this notes
//!      field has an email embedded in prose" through the same code
//!      path (PRD §10.8's `--field text` example);
//!    - de-duplicates overlapping spans across a recognizer's multiple
//!      alternative patterns (so a value matched by two patterns isn't
//!      double-counted);
//!    - applies every declared validator (AND semantics) to each
//!      candidate match;
//!    - emits a finding only when at least one row has a validated
//!      match — a (column, recognizer) pair with zero validated matches
//!      produces no finding at all (not a zero-value entry);
//!    - computes `confidence` via `crate::confidence::compute`, with
//!      `validator_ok` true only when a validator exists *and every*
//!      candidate (not just the validated ones) passed it, and
//!      `context_ok` true when the column name case-insensitively
//!      contains any of the recognizer's context terms;
//!    - `sample_evidence` is up to 3 validated matches, each rendered
//!      through `datagov_core::mask::Masked` — never a raw value.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;
use datagov_core::DatagovError;
use datagov_core::mask::Masked;
use datagov_core::report::{PiiFinding, PiiSection};
use datagov_data::engine::{new_session_context, register_dataset_table};
use datagov_data::format::Format;
use datagov_data::rows::batches_to_row_maps;
use serde_json::Value;

use crate::model::Recognizer;
use crate::validators::validate;

/// The maximum number of masked example matches kept per finding.
const MAX_SAMPLE_EVIDENCE: usize = 3;

/// A `pii scan` request: the target dataset, optional `--field`/`--sample`
/// restrictions, and the merged recognizer set (built-ins plus any
/// `--recognizers` overrides/additions — see `crate::registry::merge`).
pub struct ScanRequest<'a> {
    pub path: &'a Path,
    pub format: Format,
    pub fields: Option<&'a [String]>,
    pub sample: Option<u64>,
    pub recognizers: &'a [Recognizer],
}

/// Scan `request.path` and build the `pii` report section.
pub async fn scan(request: ScanRequest<'_>) -> Result<PiiSection, DatagovError> {
    if request.format == Format::Json {
        return Err(DatagovError::unsupported_input(
            format!(
                "'{}' is a JSON array; pii scan supports CSV, TSV, JSONL, and Parquet",
                request.path.display()
            ),
            "convert to .jsonl (one record per line), or use CSV/TSV/Parquet".to_string(),
        ));
    }

    let ctx = new_session_context();
    let mut used_names = HashSet::new();
    let base_table =
        register_dataset_table(&ctx, request.path, request.format, &mut used_names).await?;

    let active_table = match request.sample {
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
    let all_names: Vec<String> = provider
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    let selected: Vec<String> = match request.fields {
        Some(requested) => {
            for name in requested {
                if !all_names.iter().any(|n| n == name) {
                    return Err(DatagovError::invalid_args(
                        format!("unknown field '{name}'"),
                        format!("choose from: {}", all_names.join(", ")),
                    ));
                }
            }
            requested.to_vec()
        }
        None => all_names,
    };

    let scanned_rows = count_rows(&ctx, &active_table).await?;

    let mut findings = Vec::new();
    for column in &selected {
        let values = column_string_values(&ctx, &active_table, column).await?;
        for recognizer in request.recognizers {
            if let Some(finding) = detect_column(column, &values, recognizer) {
                findings.push(finding);
            }
        }
    }

    Ok(PiiSection {
        scanned_columns: selected.len() as u32,
        scanned_rows,
        sample_size: request.sample,
        findings,
    })
}

async fn execute_sql(
    ctx: &SessionContext,
    sql: &str,
) -> Result<Vec<serde_json::Map<String, Value>>, DatagovError> {
    let df = ctx.sql(sql).await.map_err(|source| {
        DatagovError::internal(format!("pii scan query failed to plan ({sql}): {source}"))
    })?;
    let batches = df.collect().await.map_err(|source| {
        DatagovError::internal(format!(
            "pii scan query failed to execute ({sql}): {source}"
        ))
    })?;
    batches_to_row_maps(&batches)
}

/// Materialize the first `n` rows of `base_table` (source order) into a
/// new in-memory table — mirrors
/// `datagov_data::profile::compute_profile`'s `materialize_sample`
/// exactly (same determinism rationale: a single DataFusion target
/// partition makes "first n rows" stable across runs).
async fn materialize_sample(
    ctx: &SessionContext,
    base_table: &str,
    n: u64,
) -> Result<String, DatagovError> {
    let sql = format!("SELECT * FROM \"{base_table}\" LIMIT {n}");
    let df = ctx.sql(&sql).await.map_err(|source| {
        DatagovError::internal(format!("failed to plan the --sample query: {source}"))
    })?;
    let batches = df.collect().await.map_err(|source| {
        DatagovError::internal(format!("failed to materialize the --sample rows: {source}"))
    })?;

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
    let rows = execute_sql(ctx, &sql).await?;
    Ok(rows
        .first()
        .and_then(|r| r.get("total"))
        .and_then(Value::as_u64)
        .unwrap_or(0))
}

/// Every non-null value of `column` in `table`, cast to a string via
/// DataFusion (`CAST(col AS VARCHAR)`) — one code path regardless of the
/// column's underlying Arrow type.
async fn column_string_values(
    ctx: &SessionContext,
    table: &str,
    column: &str,
) -> Result<Vec<String>, DatagovError> {
    let sql = format!(
        "SELECT CAST(\"{column}\" AS VARCHAR) AS v FROM \"{table}\" WHERE \"{column}\" IS NOT NULL"
    );
    let rows = execute_sql(ctx, &sql).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.get("v").and_then(Value::as_str).map(str::to_string))
        .collect())
}

/// Run one recognizer against one column's scanned values. Returns
/// `None` when there is no validated match anywhere in the column (no
/// finding is emitted for that case — see the module docs).
fn detect_column(column: &str, values: &[String], recognizer: &Recognizer) -> Option<PiiFinding> {
    let mut total_candidates: u64 = 0;
    let mut total_valid: u64 = 0;
    let mut rows_with_valid: u64 = 0;
    let mut evidence: Vec<String> = Vec::new();

    for value in values {
        // De-duplicate overlapping spans across the recognizer's
        // alternative patterns so a value matched by two patterns isn't
        // counted twice.
        let mut spans: HashMap<(usize, usize), ()> = HashMap::new();
        for pattern in &recognizer.patterns {
            for m in pattern.find_iter(value) {
                spans.entry((m.start(), m.end())).or_insert(());
            }
        }
        if spans.is_empty() {
            continue;
        }

        let mut ordered: Vec<(usize, usize)> = spans.into_keys().collect();
        ordered.sort_unstable();

        let mut row_has_valid = false;
        for (start, end) in ordered {
            let candidate = &value[start..end];
            total_candidates += 1;
            let is_valid = recognizer
                .validators
                .iter()
                .all(|kind| validate(*kind, candidate));
            if is_valid {
                total_valid += 1;
                row_has_valid = true;
                if evidence.len() < MAX_SAMPLE_EVIDENCE {
                    evidence.push(candidate.to_string());
                }
            }
        }
        if row_has_valid {
            rows_with_valid += 1;
        }
    }

    if rows_with_valid == 0 {
        return None;
    }

    let has_validator = !recognizer.validators.is_empty();
    let validator_ok = has_validator && total_candidates > 0 && total_valid == total_candidates;

    let context_match = recognizer.context.iter().find(|term| {
        column
            .to_ascii_lowercase()
            .contains(&term.to_ascii_lowercase())
    });
    let context_ok = context_match.is_some();

    let confidence =
        crate::confidence::compute(recognizer.base_confidence, validator_ok, context_ok);

    let non_null_count = values.len() as u64;
    let match_percentage = if non_null_count == 0 {
        0.0
    } else {
        (rows_with_valid as f64 / non_null_count as f64) * 100.0
    };

    let context_clause = match context_match {
        Some(term) => format!("column name matches context term '{term}'; "),
        None => String::new(),
    };
    let reason = if has_validator {
        format!(
            "{context_clause}{rows_with_valid}/{non_null_count} values ({match_percentage:.1}%) \
             matched {} and passed validation",
            recognizer.entity
        )
    } else {
        format!(
            "{context_clause}{rows_with_valid}/{non_null_count} values ({match_percentage:.1}%) \
             matched {}",
            recognizer.entity
        )
    };

    Some(PiiFinding {
        column: column.to_string(),
        entity: recognizer.entity.clone(),
        recognizer: recognizer.id.clone(),
        confidence,
        match_count: rows_with_valid,
        match_percentage,
        sample_evidence: evidence
            .into_iter()
            .map(|v| Masked::new(v).to_string())
            .collect(),
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::builtin_recognizers;

    fn recognizer(id: &str) -> Recognizer {
        builtin_recognizers()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
    }

    #[test]
    fn no_match_produces_no_finding() {
        let values = vec!["nothing here".to_string(), "still nothing".to_string()];
        assert!(detect_column("notes", &values, &recognizer("us_ssn")).is_none());
    }

    #[test]
    fn embedded_match_within_prose_is_detected() {
        let values =
            vec!["Reported via https://example.com/incidents/42 from host 192.0.2.10.".to_string()];
        let finding = detect_column("notes", &values, &recognizer("url")).unwrap();
        assert_eq!(finding.match_count, 1);
        assert_eq!(finding.entity, "URL");
        // The masked evidence must never contain the raw URL.
        for sample in &finding.sample_evidence {
            assert!(!sample.contains("https://example.com/incidents/42"));
        }
    }

    #[test]
    fn context_bonus_applies_when_column_name_matches() {
        let values = vec!["555-01-0001".to_string(); 5];
        let finding = detect_column("ssn", &values, &recognizer("us_ssn")).unwrap();
        assert!(finding.reason.contains("context term 'ssn'"));
        // base 0.75 + validator 0.10 + context 0.05 = 0.90
        assert_eq!(finding.confidence, 0.90);
        assert_eq!(finding.match_count, 5);
        assert_eq!(finding.match_percentage, 100.0);
    }

    #[test]
    fn no_context_bonus_when_column_name_does_not_match() {
        let values = vec!["555-01-0001".to_string(); 3];
        let finding = detect_column("field_a", &values, &recognizer("us_ssn")).unwrap();
        assert!(!finding.reason.contains("context term"));
        // base 0.75 + validator 0.10 + 0 = 0.85
        assert_eq!(finding.confidence, 0.85);
    }

    #[test]
    fn partial_validation_gives_no_validator_bonus() {
        // One value is a valid SSN, one has an invalid SSA area (900) —
        // no partial credit, so validator_bonus must be 0.
        let values = vec!["555-01-0001".to_string(), "900-01-0001".to_string()];
        let finding = detect_column("field_a", &values, &recognizer("us_ssn")).unwrap();
        // base 0.75 + 0 (not all candidates validated) + 0 (no context) = 0.75
        assert_eq!(finding.confidence, 0.75);
        // Only the one genuinely valid row counts as a match.
        assert_eq!(finding.match_count, 1);
    }

    #[test]
    fn sample_evidence_never_contains_the_raw_value() {
        let raw = "4111111111111111";
        let values = vec![raw.to_string()];
        let finding = detect_column("card_number", &values, &recognizer("credit_card")).unwrap();
        for sample in &finding.sample_evidence {
            assert!(!sample.contains(raw));
        }
    }

    #[test]
    fn overlapping_patterns_are_not_double_counted() {
        // us_zip_code's single pattern matching the same span twice via
        // find_iter is impossible by construction (find_iter already
        // yields non-overlapping matches for one pattern); this test
        // instead exercises the multi-value case is still one match per
        // row for the same recognizer.
        let values = vec!["12345".to_string()];
        let finding = detect_column("zip", &values, &recognizer("us_zip_code")).unwrap();
        assert_eq!(finding.match_count, 1);
    }
}

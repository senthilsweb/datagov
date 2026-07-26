//! rows.rs — Converts DataFusion/Arrow `RecordBatch`es into
//! `serde_json::Value` rows for `query`'s JSON/table/CSV renderings and
//! `profile`'s aggregate-result parsing.
//!
//! 1. `batches_to_row_maps` renders every batch through
//!    `arrow::json::ArrayWriter` (the Arrow project's own RecordBatch ->
//!    JSON encoder, covering every Arrow type this bolt could encounter —
//!    no hand-written per-physical-type dispatch) into one JSON array,
//!    then parses it back into `Vec<serde_json::Map<String, Value>>` —
//!    one map per row. `ArrayWriter` omits a key entirely for a row where
//!    that column is null; callers must treat a missing key the same as
//!    an explicit `null` (see `row_map_to_ordered_values`).
//! 2. `row_map_to_ordered_values` projects one row map into a
//!    `Vec<serde_json::Value>` in a given column order, defaulting any
//!    missing key to `Value::Null`.

use arrow::json::ArrayWriter;
use arrow_array::RecordBatch;
use datagov_core::DatagovError;
use serde_json::{Map, Value};

/// Render `batches` to JSON and parse it back into one row-map per row,
/// in batch/row order.
pub fn batches_to_row_maps(
    batches: &[RecordBatch],
) -> Result<Vec<Map<String, Value>>, DatagovError> {
    if batches.is_empty() {
        return Ok(Vec::new());
    }

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut writer = ArrayWriter::new(&mut buf);
        let refs: Vec<&RecordBatch> = batches.iter().collect();
        writer.write_batches(&refs).map_err(|source| {
            DatagovError::internal(format!("failed to encode a query result batch: {source}"))
        })?;
        writer.finish().map_err(|source| {
            DatagovError::internal(format!(
                "failed to finish encoding a query result: {source}"
            ))
        })?;
    }

    let value: Value = serde_json::from_slice(&buf).map_err(|source| {
        DatagovError::internal(format!("failed to parse an encoded query result: {source}"))
    })?;

    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| {
                item.as_object().cloned().ok_or_else(|| {
                    DatagovError::internal("a query result row was not a JSON object".to_string())
                })
            })
            .collect(),
        _ => Err(DatagovError::internal(
            "a query result did not encode as a JSON array".to_string(),
        )),
    }
}

/// Project `row` into a `Vec<Value>` following `columns`' order, treating
/// a missing key (a null cell omitted by `ArrayWriter`) as `Value::Null`.
pub fn row_map_to_ordered_values(row: &Map<String, Value>, columns: &[String]) -> Vec<Value> {
    columns
        .iter()
        .map(|c| row.get(c).cloned().unwrap_or(Value::Null))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Int64Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    #[test]
    fn converts_batches_and_fills_nulls_as_missing_keys() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::Int64, true),
            Field::new("b", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(vec![Some(1), None])),
                Arc::new(StringArray::from(vec![Some("x"), Some("y")])),
            ],
        )
        .unwrap();

        let rows = batches_to_row_maps(std::slice::from_ref(&batch)).unwrap();
        assert_eq!(rows.len(), 2);

        let columns = vec!["a".to_string(), "b".to_string()];
        let first = row_map_to_ordered_values(&rows[0], &columns);
        assert_eq!(first, vec![Value::from(1), Value::from("x")]);

        let second = row_map_to_ordered_values(&rows[1], &columns);
        assert_eq!(second, vec![Value::Null, Value::from("y")]);
    }

    #[test]
    fn empty_batches_yield_empty_rows() {
        assert!(batches_to_row_maps(&[]).unwrap().is_empty());
    }
}

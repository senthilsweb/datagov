//! json.rs — The JSON (single top-level array) and JSONL (one object per
//! line) `DatasetReader`.
//!
//! 1. Parses the whole input into `serde_json::Value`s. JSONL: one
//!    record per non-empty line. JSON: the top-level value must be an
//!    array (each element a record) or, for the ambiguous
//!    single-non-empty-line content-sniffed case, a bare object treated
//!    as a one-record dataset.
//! 2. Column set: the union of top-level object keys across all
//!    records, in first-seen order.
//! 3. `data_type` per column: the JSON "family" (bool / number / string /
//!    array / object) of its non-null occurrences, narrowed the same way
//!    CSV narrows numbers (all-whole-number occurrences → `Integer`, any
//!    fractional occurrence → `Float`); a field with no non-null
//!    occurrences at all is `Null`; occurrences spanning more than one
//!    family is `Mixed`. `nullable` is true if any record omits the
//!    field or has it explicitly `null`.
//! 4. `approx_memory_bytes`: the same per-value formula as CSV (String:
//!    byte length + 24; Integer/Float: 8; Boolean: 1; null: 1), applied
//!    to the parsed values. Object/Array values count as a flat 24 bytes
//!    — not recursively sized — to keep this cheap; a field entirely
//!    absent from a record (as opposed to explicitly `null`) contributes
//!    nothing, since there is no value at all to size.
//! 5. Sample rows: the first `DEFAULT_SAMPLE_ROWS` records in source
//!    order, masked via `super::common::mask_sample_rows`.

use datagov_core::DatagovError;
use datagov_core::report::{ColumnSchema, DataType, DatasetSection};
use serde_json::{Map, Value};
use std::io::Read;

use crate::reader::{DEFAULT_SAMPLE_ROWS, DatasetReader};
use crate::source::Source;

use super::common::mask_sample_rows;

pub struct JsonReader {
    pub jsonl: bool,
}

/// The JSON "family" a value belongs to, coarser than `DataType`
/// (Integer and Float are both `Number` here) so numeric columns don't
/// spuriously become `Mixed` just for having both whole and fractional
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Boolean,
    Number,
    String,
    Array,
    Object,
}

fn family_of(value: &Value) -> Option<Family> {
    match value {
        Value::Null => None,
        Value::Bool(_) => Some(Family::Boolean),
        Value::Number(_) => Some(Family::Number),
        Value::String(_) => Some(Family::String),
        Value::Array(_) => Some(Family::Array),
        Value::Object(_) => Some(Family::Object),
    }
}

impl DatasetReader for JsonReader {
    fn inspect(&self, source: &Source) -> Result<DatasetSection, DatagovError> {
        let file_size_bytes = source.file_size_bytes()?;
        let mut read = source.open_read()?;
        let mut content = String::new();
        read.read_to_string(&mut content)
            .map_err(|source| DatagovError::internal(format!("failed to read input: {source}")))?;

        let records = if self.jsonl {
            parse_jsonl(&content)?
        } else {
            parse_json(&content)?
        };

        let row_count = records.len() as u64;

        let mut column_order: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for record in &records {
            for key in record.keys() {
                if seen.insert(key.clone()) {
                    column_order.push(key.clone());
                }
            }
        }
        let column_count = column_order.len() as u32;

        let mut schema = Vec::with_capacity(column_order.len());
        let mut approx_memory_bytes: u64 = 0;

        for column in &column_order {
            let mut nullable = false;
            let mut families: Vec<Family> = Vec::new();
            let mut any_fractional_number = false;

            for record in &records {
                match record.get(column) {
                    None => nullable = true,
                    Some(Value::Null) => {
                        nullable = true;
                        approx_memory_bytes += 1;
                    }
                    Some(other) => {
                        if let Some(family) = family_of(other) {
                            if !families.contains(&family) {
                                families.push(family);
                            }
                            if family == Family::Number
                                && let Value::Number(n) = other
                                && !(n.is_i64() || n.is_u64())
                            {
                                any_fractional_number = true;
                            }
                        }
                        approx_memory_bytes += value_memory_bytes(other);
                    }
                }
            }

            let data_type = match families.len() {
                0 => DataType::Null,
                1 => match families[0] {
                    Family::Boolean => DataType::Boolean,
                    Family::Number => {
                        if any_fractional_number {
                            DataType::Float
                        } else {
                            DataType::Integer
                        }
                    }
                    Family::String => DataType::String,
                    Family::Array => DataType::Array,
                    Family::Object => DataType::Object,
                },
                _ => DataType::Mixed,
            };

            schema.push(ColumnSchema {
                name: column.clone(),
                data_type,
                nullable,
            });
        }

        let sample_rows: Vec<Map<String, Value>> =
            records.into_iter().take(DEFAULT_SAMPLE_ROWS).collect();
        let sample_rows = mask_sample_rows(sample_rows);

        Ok(DatasetSection {
            format: if self.jsonl { "jsonl" } else { "json" }.to_string(),
            file_size_bytes,
            row_count,
            column_count,
            schema,
            approx_memory_bytes,
            parquet: None,
            sample_rows,
        })
    }
}

fn value_memory_bytes(value: &Value) -> u64 {
    match value {
        Value::Null => 1,
        Value::Bool(_) => 1,
        Value::Number(_) => 8,
        Value::String(s) => s.len() as u64 + 24,
        Value::Array(_) | Value::Object(_) => 24,
    }
}

fn parse_jsonl(content: &str) -> Result<Vec<Map<String, Value>>, DatagovError> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let value: Value = serde_json::from_str(line).map_err(|source| {
                DatagovError::internal(format!("malformed JSONL line: {source}"))
            })?;
            value.as_object().cloned().ok_or_else(|| {
                DatagovError::internal("a JSONL line is not a JSON object".to_string())
            })
        })
        .collect()
}

fn parse_json(content: &str) -> Result<Vec<Map<String, Value>>, DatagovError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|source| DatagovError::internal(format!("malformed JSON: {source}")))?;
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| {
                item.as_object().cloned().ok_or_else(|| {
                    DatagovError::internal("a JSON array element is not an object".to_string())
                })
            })
            .collect(),
        // The ambiguous content-sniffed case: a bare single object,
        // treated as a one-record dataset.
        Value::Object(obj) => Ok(vec![obj]),
        _ => Err(DatagovError::unsupported_input(
            "top-level JSON value is neither an array nor an object",
            "provide a JSON array of objects, or a single JSON object",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::Format;
    use crate::reader::reader_for;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn mixed_type_field_is_detected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, r#"[{"a":1},{"a":"two"}]"#).unwrap();

        let source = Source::from_path(path, Format::Json);
        let section = reader_for(Format::Json).inspect(&source).unwrap();

        assert_eq!(section.schema[0].data_type, DataType::Mixed);
    }

    #[test]
    fn numeric_field_with_whole_and_fractional_values_narrows_to_float() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, r#"[{"a":1},{"a":1.5}]"#).unwrap();

        let source = Source::from_path(path, Format::Json);
        let section = reader_for(Format::Json).inspect(&source).unwrap();

        assert_eq!(section.schema[0].data_type, DataType::Float);
    }

    #[test]
    fn missing_and_null_fields_are_nullable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.jsonl");
        fs::write(&path, "{\"a\":1}\n{\"a\":null}\n{}\n").unwrap();

        let source = Source::from_path(path, Format::Jsonl);
        let section = reader_for(Format::Jsonl).inspect(&source).unwrap();

        assert!(section.schema[0].nullable);
        assert_eq!(section.schema[0].data_type, DataType::Integer);
    }

    #[test]
    fn all_null_field_has_null_data_type() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.jsonl");
        fs::write(&path, "{\"a\":null}\n{\"a\":null}\n").unwrap();

        let source = Source::from_path(path, Format::Jsonl);
        let section = reader_for(Format::Jsonl).inspect(&source).unwrap();

        assert_eq!(section.schema[0].data_type, DataType::Null);
    }
}

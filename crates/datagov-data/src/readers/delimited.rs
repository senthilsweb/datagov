//! delimited.rs — The CSV/TSV `DatasetReader` (one implementation, one
//! `csv::ReaderBuilder` delimiter byte, shared between both formats).
//!
//! 1. Full scan: exact row count, per-column type narrowing, and
//!    `approx_memory_bytes` all come from the same single pass over
//!    every record — these are small fixtures, so a full scan is fine
//!    (the PRD's "without scanning all rows" language is specific to
//!    Parquet).
//! 2. Type narrowing per column: try Boolean → Integer (i64) → Float
//!    (f64) → String, in that order, using the narrowest type that fits
//!    every non-null value; a column with no non-null values at all is
//!    `DataType::Null`. Empty string counts as a null occurrence;
//!    `nullable` is true if any row's value for that column was empty.
//! 3. `approx_memory_bytes`: sum over every non-null value of (UTF-8
//!    byte length + 24 for String columns; 8 for Integer/Float; 1 for
//!    Boolean; 1 for any null) — measured from the same scan.
//! 4. Sample rows: the first `DEFAULT_SAMPLE_ROWS` records in source
//!    order, rendered with each column's inferred JSON type, then masked
//!    via `super::common::mask_sample_rows`.

use datagov_core::DatagovError;
use datagov_core::report::{ColumnSchema, DataType, DatasetSection};
use serde_json::{Map, Value};

use crate::reader::{DEFAULT_SAMPLE_ROWS, DatasetReader};
use crate::source::Source;

use super::common::mask_sample_rows;

/// Reads CSV (`delimiter = b','`) or TSV (`delimiter = b'\t'`) — the same
/// `csv` crate reader, just a different delimiter byte.
pub struct DelimitedReader {
    pub delimiter: u8,
}

impl DatasetReader for DelimitedReader {
    fn inspect(&self, source: &Source) -> Result<DatasetSection, DatagovError> {
        let file_size_bytes = source.file_size_bytes()?;
        let read = source.open_read()?;

        let mut csv_reader = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(true)
            .flexible(true)
            .from_reader(read);

        let headers: Vec<String> = csv_reader
            .headers()
            .map_err(|source| {
                DatagovError::internal(format!("failed to read the header row: {source}"))
            })?
            .iter()
            .map(str::to_string)
            .collect();

        let mut columns: Vec<Vec<Option<String>>> = vec![Vec::new(); headers.len()];
        let mut sample_raw: Vec<Vec<Option<String>>> = Vec::new();
        let mut row_count: u64 = 0;

        for result in csv_reader.records() {
            let record = result.map_err(|source| {
                DatagovError::internal(format!("failed to read a record: {source}"))
            })?;

            let row_values: Vec<Option<String>> = (0..headers.len())
                .map(|i| {
                    let raw = record.get(i).unwrap_or("");
                    if raw.is_empty() {
                        None
                    } else {
                        Some(raw.to_string())
                    }
                })
                .collect();

            if row_count < DEFAULT_SAMPLE_ROWS as u64 {
                sample_raw.push(row_values.clone());
            }
            for (column, value) in columns.iter_mut().zip(row_values) {
                column.push(value);
            }
            row_count += 1;
        }

        let mut schema = Vec::with_capacity(headers.len());
        let mut approx_memory_bytes: u64 = 0;
        for (name, values) in headers.iter().zip(columns.iter()) {
            let nullable = values.iter().any(Option::is_none);
            let data_type = narrow_type(values);
            approx_memory_bytes += column_memory_bytes(values, data_type);
            schema.push(ColumnSchema {
                name: name.clone(),
                data_type,
                nullable,
            });
        }

        let sample_rows: Vec<Map<String, Value>> = sample_raw
            .into_iter()
            .map(|row| {
                let mut map = Map::new();
                for (i, value) in row.into_iter().enumerate() {
                    map.insert(
                        headers[i].clone(),
                        render_value(value.as_deref(), schema[i].data_type),
                    );
                }
                map
            })
            .collect();
        let sample_rows = mask_sample_rows(sample_rows);

        let format = if self.delimiter == b'\t' {
            "tsv"
        } else {
            "csv"
        };

        Ok(DatasetSection {
            format: format.to_string(),
            file_size_bytes,
            row_count,
            column_count: headers.len() as u32,
            schema,
            approx_memory_bytes,
            parquet: None,
            sample_rows,
        })
    }
}

/// Narrow a column's non-null string values to the narrowest type that
/// fits every one of them: Boolean → Integer → Float → String. A column
/// with no non-null values is `DataType::Null`.
fn narrow_type(values: &[Option<String>]) -> DataType {
    let mut any_value = false;
    let mut is_bool = true;
    let mut is_int = true;
    let mut is_float = true;

    for value in values.iter().filter_map(|v| v.as_deref()) {
        any_value = true;
        is_bool &= value.parse::<bool>().is_ok();
        is_int &= value.parse::<i64>().is_ok();
        is_float &= value.parse::<f64>().is_ok();
    }

    if !any_value {
        DataType::Null
    } else if is_bool {
        DataType::Boolean
    } else if is_int {
        DataType::Integer
    } else if is_float {
        DataType::Float
    } else {
        DataType::String
    }
}

fn column_memory_bytes(values: &[Option<String>], data_type: DataType) -> u64 {
    values
        .iter()
        .map(|value| match value {
            None => 1,
            Some(s) => match data_type {
                DataType::Integer | DataType::Float => 8,
                DataType::Boolean => 1,
                DataType::String
                | DataType::Null
                | DataType::Object
                | DataType::Array
                | DataType::Mixed => s.len() as u64 + 24,
            },
        })
        .sum()
}

/// Render one cell as the JSON value its column's inferred type implies.
fn render_value(value: Option<&str>, data_type: DataType) -> Value {
    let Some(raw) = value else {
        return Value::Null;
    };
    match data_type {
        DataType::Boolean => raw
            .parse::<bool>()
            .map(Value::Bool)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        DataType::Integer => raw
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        DataType::Float => raw
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(raw.to_string())),
        DataType::String
        | DataType::Null
        | DataType::Object
        | DataType::Array
        | DataType::Mixed => Value::String(raw.to_string()),
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
    fn boolean_column_narrows_to_boolean() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.csv");
        fs::write(&path, "flag\ntrue\nfalse\ntrue\n").unwrap();

        let source = Source::from_path(path, Format::Csv);
        let section = reader_for(Format::Csv).inspect(&source).unwrap();

        assert_eq!(section.schema[0].data_type, DataType::Boolean);
        assert!(!section.schema[0].nullable);
        assert_eq!(section.row_count, 3);
    }

    #[test]
    fn mixed_text_and_numbers_narrows_to_string() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.csv");
        fs::write(&path, "value\n123\nabc\n456\n").unwrap();

        let source = Source::from_path(path, Format::Csv);
        let section = reader_for(Format::Csv).inspect(&source).unwrap();

        assert_eq!(section.schema[0].data_type, DataType::String);
    }

    #[test]
    fn empty_values_count_as_null_and_mark_nullable() {
        // A blank *field* (not a blank line, which the `csv` crate skips
        // entirely) in a multi-column row.
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.csv");
        fs::write(&path, "value,other\n1,x\n,y\n3,z\n").unwrap();

        let source = Source::from_path(path, Format::Csv);
        let section = reader_for(Format::Csv).inspect(&source).unwrap();

        assert!(section.schema[0].nullable);
        assert_eq!(section.schema[0].data_type, DataType::Integer);
        assert_eq!(section.row_count, 3);
    }

    #[test]
    fn tsv_delimiter_is_used() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.tsv");
        fs::write(&path, "a\tb\n1\t2\n").unwrap();

        let source = Source::from_path(path, Format::Tsv);
        let section = reader_for(Format::Tsv).inspect(&source).unwrap();

        assert_eq!(section.format, "tsv");
        assert_eq!(section.column_count, 2);
    }
}

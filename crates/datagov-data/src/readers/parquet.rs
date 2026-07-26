//! parquet.rs — The Parquet `DatasetReader`, using the `parquet` crate's
//! own `file::reader` and `record` APIs directly — no `arrow` (see the
//! Bolt 2 brief's scope note: DataFusion/Arrow arrive in Bolt 3 for
//! profiling/query, not here).
//!
//! 1. Row count, schema (names/types/nullability), and
//!    `approx_memory_bytes` all come from file metadata — no full row
//!    scan, per the PRD's Parquet-specific "without scanning all rows"
//!    requirement. `approx_memory_bytes` sums the uncompressed byte size
//!    (`RowGroupMetaData::total_byte_size`) across all row groups.
//! 2. `ParquetInfo.row_groups` and `.compression_codecs` are read from
//!    the same metadata; codec names come from
//!    `ColumnChunkMetaData::compression()`'s `Display` rendering (e.g.
//!    `"SNAPPY"`), deduplicated and sorted.
//! 3. Only the bounded sample rows require actually reading data, via
//!    `FileReader::get_row_iter`, capped at `DEFAULT_SAMPLE_ROWS`; each
//!    `Row` is converted with `Row::to_json_value` (the `parquet` crate's
//!    own JSON rendering, gated behind its `json` feature — not
//!    `serde_json` glue we wrote ourselves) and then masked.
//! 4. Column types: `BOOLEAN` → Boolean; `INT32`/`INT64`/`INT96` →
//!    Integer; `FLOAT`/`DOUBLE` → Float; `BYTE_ARRAY` /
//!    `FIXED_LEN_BYTE_ARRAY` → String (this bolt's `DataType` has no
//!    dedicated binary/decimal/date variant, so both physical byte-array
//!    kinds map to String — a documented simplification covering the
//!    fixture's actual schema, not a Correction). Nullability comes from
//!    the column's `Repetition` (`OPTIONAL`/`REPEATED` → nullable,
//!    `REQUIRED` → not).

use datagov_core::DatagovError;
use datagov_core::report::{ColumnSchema, DataType, DatasetSection, ParquetInfo, RowGroupInfo};
use parquet::basic::{Repetition, Type as PhysicalType};
use parquet::file::reader::{FileReader, SerializedFileReader};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::reader::{DEFAULT_SAMPLE_ROWS, DatasetReader};
use crate::source::Source;

use super::common::mask_sample_rows;

pub struct ParquetReader;

impl DatasetReader for ParquetReader {
    fn inspect(&self, source: &Source) -> Result<DatasetSection, DatagovError> {
        let path = source.path().ok_or_else(|| {
            DatagovError::invalid_args(
                "parquet input must be a real file path",
                "pass a file path instead of '-'; Parquet requires random access and cannot be read from stdin",
            )
        })?;

        let file_size_bytes = source.file_size_bytes()?;

        let file = std::fs::File::open(path).map_err(|source| {
            DatagovError::input_not_found(
                format!("input not found: {} ({source})", path.display()),
                format!("check that {} exists and is readable", path.display()),
            )
        })?;

        let file_reader = SerializedFileReader::new(file).map_err(|source| {
            DatagovError::internal(format!(
                "failed to read Parquet metadata for {}: {source}",
                path.display()
            ))
        })?;

        let metadata = file_reader.metadata();
        let file_metadata = metadata.file_metadata();
        let row_count = file_metadata.num_rows().max(0) as u64;

        let schema_descr = file_metadata.schema_descr();
        let column_count = schema_descr.num_columns() as u32;
        let mut schema = Vec::with_capacity(schema_descr.num_columns());
        for i in 0..schema_descr.num_columns() {
            let column = schema_descr.column(i);
            let repetition = column.self_type().get_basic_info().repetition();
            schema.push(ColumnSchema {
                name: column.name().to_string(),
                data_type: map_physical_type(column.physical_type()),
                nullable: repetition != Repetition::REQUIRED,
            });
        }

        let mut row_groups = Vec::with_capacity(metadata.row_groups().len());
        let mut approx_memory_bytes: u64 = 0;
        let mut codecs: BTreeSet<String> = BTreeSet::new();
        for (index, row_group) in metadata.row_groups().iter().enumerate() {
            approx_memory_bytes += row_group.total_byte_size().max(0) as u64;
            row_groups.push(RowGroupInfo {
                index,
                row_count: row_group.num_rows(),
                compressed_size_bytes: row_group.compressed_size(),
                uncompressed_size_bytes: row_group.total_byte_size(),
            });
            for column in row_group.columns() {
                codecs.insert(column.compression().to_string());
            }
        }

        let mut sample_rows: Vec<Map<String, Value>> = Vec::with_capacity(DEFAULT_SAMPLE_ROWS);
        let mut row_iter = file_reader.get_row_iter(None).map_err(|source| {
            DatagovError::internal(format!(
                "failed to open a Parquet row iterator for {}: {source}",
                path.display()
            ))
        })?;
        for _ in 0..DEFAULT_SAMPLE_ROWS {
            match row_iter.next() {
                Some(Ok(row)) => {
                    if let Value::Object(map) = row.to_json_value() {
                        sample_rows.push(map);
                    }
                }
                Some(Err(source)) => {
                    return Err(DatagovError::internal(format!(
                        "failed to read a Parquet row for {}: {source}",
                        path.display()
                    )));
                }
                None => break,
            }
        }
        let sample_rows = mask_sample_rows(sample_rows);

        Ok(DatasetSection {
            format: "parquet".to_string(),
            file_size_bytes,
            row_count,
            column_count,
            schema,
            approx_memory_bytes,
            parquet: Some(ParquetInfo {
                row_groups,
                compression_codecs: codecs.into_iter().collect(),
            }),
            sample_rows,
        })
    }
}

fn map_physical_type(physical: PhysicalType) -> DataType {
    match physical {
        PhysicalType::BOOLEAN => DataType::Boolean,
        PhysicalType::INT32 | PhysicalType::INT64 | PhysicalType::INT96 => DataType::Integer,
        PhysicalType::FLOAT | PhysicalType::DOUBLE => DataType::Float,
        PhysicalType::BYTE_ARRAY | PhysicalType::FIXED_LEN_BYTE_ARRAY => DataType::String,
    }
}

//! reader.rs — The shared `DatasetReader` trait and its per-format
//! dispatcher.
//!
//! 1. Defines `DatasetReader`, the one trait every format reader
//!    implements, so `inspect` (and later `profile`) share it rather
//!    than branching on format themselves.
//! 2. Defines `DEFAULT_SAMPLE_ROWS`, the bounded sample-row count shared
//!    by every reader (no `--sample` flag exists until `profile` in
//!    Bolt 3).
//! 3. Provides `reader_for`, which picks the concrete reader for a
//!    resolved `Format`.

use datagov_core::DatagovError;
use datagov_core::report::DatasetSection;

use crate::format::Format;
use crate::readers::{delimited::DelimitedReader, json::JsonReader, parquet::ParquetReader};
use crate::source::Source;

/// The number of sample rows every reader takes from the front of a
/// dataset, in source order. No `--sample` flag exists until `profile`
/// (Bolt 3).
pub const DEFAULT_SAMPLE_ROWS: usize = 5;

/// Shared by every per-format reader so `inspect` (and later `profile`)
/// can call through one interface regardless of input format.
pub trait DatasetReader {
    fn inspect(&self, source: &Source) -> Result<DatasetSection, DatagovError>;
}

/// Construct the `DatasetReader` for a resolved `Format`.
pub fn reader_for(format: Format) -> Box<dyn DatasetReader> {
    match format {
        Format::Csv => Box::new(DelimitedReader { delimiter: b',' }),
        Format::Tsv => Box::new(DelimitedReader { delimiter: b'\t' }),
        Format::Json => Box::new(JsonReader { jsonl: false }),
        Format::Jsonl => Box::new(JsonReader { jsonl: true }),
        Format::Parquet => Box::new(ParquetReader),
    }
}

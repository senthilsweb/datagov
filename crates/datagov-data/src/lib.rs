//! lib.rs — Crate root for `datagov-data`: format detection, the shared
//! `DatasetReader` trait, and one reader per Milestone-0.1 dataset
//! format.
//!
//! 1. Declares the public modules: `format` (`Format` + detection),
//!    `source` (`Source`, the input a reader consumes), `reader` (the
//!    `DatasetReader` trait, `DEFAULT_SAMPLE_ROWS`, and the `reader_for`
//!    dispatcher), and `readers` (the per-format implementations).
//! 2. Re-exports the most commonly used items at the crate root for
//!    ergonomic `datagov_data::{Format, Source, ...}`-style access from
//!    `datagov-cli`.
//! 3. Scope note (Bolt 2): CSV/TSV use the `csv` crate; JSON/JSONL use
//!    `serde_json`; Parquet uses the `parquet` crate's own
//!    `file::reader`/`record` APIs directly — no `arrow`/DataFusion in
//!    this crate until Bolt 3 (profiling/query), per the Bolt 2 brief's
//!    scope decision.

pub mod format;
pub mod reader;
pub mod readers;
pub mod source;

pub use format::Format;
pub use reader::{DEFAULT_SAMPLE_ROWS, DatasetReader, reader_for};
pub use source::Source;

//! lib.rs — Crate root for `datagov-data`: format detection, the shared
//! `DatasetReader` trait, one reader per Milestone-0.1 dataset format,
//! and (Bolt 3) the DataFusion-backed `profile`/`query` engines.
//!
//! 1. Declares the public modules: `format` (`Format` + detection),
//!    `source` (`Source`, the input a reader consumes), `reader` (the
//!    `DatasetReader` trait, `DEFAULT_SAMPLE_ROWS`, and the `reader_for`
//!    dispatcher), and `readers` (the per-format implementations) — all
//!    Bolt 2. Bolt 3 adds `engine` (shared DataFusion session/table-
//!    registration helpers), `rows` (`RecordBatch` -> JSON row
//!    conversion), `profile` (`datagov profile`'s column-statistics
//!    engine), and `query` (`datagov query`'s SQL execution engine).
//! 2. Re-exports the most commonly used items at the crate root for
//!    ergonomic `datagov_data::{Format, Source, ...}`-style access from
//!    `datagov-cli`.
//! 3. Scope note (Bolt 2, superseded by Bolt 3 for profiling/query):
//!    CSV/TSV use the `csv` crate; JSON/JSONL use `serde_json`; Parquet
//!    uses the `parquet` crate's own `file::reader`/`record` APIs
//!    directly for `inspect` — no `arrow`/DataFusion in that path. Bolt
//!    3's `engine`/`profile`/`query` modules bring in `datafusion` and
//!    `arrow` directly (see the Bolt 3 report for the resulting
//!    coexistence of two `parquet` crate versions in the build graph).

pub mod engine;
pub mod format;
pub mod profile;
pub mod query;
pub mod reader;
pub mod readers;
pub mod rows;
pub mod source;

pub use format::Format;
pub use reader::{DEFAULT_SAMPLE_ROWS, DatasetReader, reader_for};
pub use source::Source;

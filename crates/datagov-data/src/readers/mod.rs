//! mod.rs — Per-format `DatasetReader` implementations.
//!
//! 1. Declares one submodule per format-reader implementation
//!    (`delimited` for CSV/TSV, `json` for JSON/JSONL, `parquet`), plus
//!    `common`, the shared sample-row masking helper every reader calls
//!    before returning its `DatasetSection`.

pub mod common;
pub mod delimited;
pub mod json;
pub mod parquet;

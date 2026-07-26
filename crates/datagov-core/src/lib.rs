//! lib.rs — Crate root for `datagov-core`, the shared engine used by every
//! `datagov` command and domain crate.
//!
//! 1. Declares the public modules: `report` (the canonical envelope, PRD
//!    §23), `exit` (the exit-code contract, PRD §24), `error` (the typed
//!    error enum and its mapping to exit codes), `logging` (the shared
//!    `tracing` subscriber setup), `config` (the configuration resolution
//!    chain, PRD §28), and `mask` (the single masking implementation used
//!    for any sensitive value that reaches an output surface).
//! 2. Re-exports the most commonly used types at the crate root for
//!    ergonomic `datagov_core::Report`-style access from callers.

pub mod config;
pub mod error;
pub mod exit;
pub mod logging;
pub mod mask;
pub mod report;

pub use error::DatagovError;
pub use exit::ExitCode;
pub use report::{Report, ReportBuilder};

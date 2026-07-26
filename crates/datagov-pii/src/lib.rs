//! lib.rs — Crate root for `datagov-pii`: the deterministic PII
//! recognizer engine behind `datagov pii scan` and
//! `datagov pii recognizers list|validate` (PRD §10.8/§10.9, Bolt 5).
//!
//! 1. Declares the public modules: `validators` (the 7 deterministic
//!    validators — Luhn, `std::net` IP parsing, the `url` crate, SSA
//!    range rejection, UUID nibble checks, ZIP deny-listing), `model`
//!    (the `Recognizer`/`RecognizerSummary` shapes), `builtins` (the 10
//!    locked-at-inception-gate built-in recognizers), `yaml` (the PRD
//!    §10.9 custom recognizer loader), `registry` (merging custom
//!    recognizers into the built-in table, override-by-id), `confidence`
//!    (the documented `base + validator_bonus + context_bonus` formula),
//!    and `scanner` (the actual dataset-scanning engine that produces
//!    `datagov_core::report::PiiSection`).
//! 2. Re-exports the most commonly used items at the crate root for
//!    ergonomic `datagov_pii::{scan, ScanRequest, ...}`-style access from
//!    `datagov-cli`.

pub mod builtins;
pub mod confidence;
pub mod model;
pub mod registry;
pub mod scanner;
pub mod validators;
pub mod yaml;

pub use model::{Recognizer, RecognizerSummary};
pub use scanner::{ScanRequest, scan};
pub use validators::ValidatorKind;

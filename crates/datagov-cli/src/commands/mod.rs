//! mod.rs — Command implementations for the `datagov` CLI.
//!
//! 1. Declares the per-subcommand modules. Each exposes a `run` function
//!    that is the only thing `main.rs` calls for its subcommand: parse
//!    (already done by clap) -> call a `datagov-core` function -> render.
//!    No business logic lives in these files beyond that adaptation.

pub mod capabilities;
pub mod inspect;
pub mod version;

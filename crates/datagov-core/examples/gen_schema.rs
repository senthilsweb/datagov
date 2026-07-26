//! gen_schema.rs — Developer utility: prints the current `Report` JSON
//! Schema to stdout.
//!
//! 1. Calls `datagov_core::report::generate_schema_document()` and prints
//!    it verbatim so it can be redirected to
//!    `docs/schema/report-v1.json` when the schema legitimately changes.

fn main() {
    print!("{}", datagov_core::report::generate_schema_document());
}

//! common.rs — Sample-row masking shared by every per-format reader.
//!
//! 1. Defines `mask_sample_rows`, applied by every reader after building
//!    its (unbounded-in-count-but-bounded-to-`DEFAULT_SAMPLE_ROWS`)
//!    sample rows: any column whose name matches
//!    `datagov_core::sensitivity::is_heuristically_sensitive` has its
//!    value replaced with the shared `Masked` rendering (as a JSON
//!    string), for every row, before the rows ever reach the report
//!    envelope.

use datagov_core::mask::Masked;
use datagov_core::sensitivity::is_heuristically_sensitive;
use serde_json::{Map, Value};

/// Mask every sensitive-named column's value across all sample rows.
/// `null` values are left as `null` — there is no raw value to mask.
pub fn mask_sample_rows(rows: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    rows.into_iter()
        .map(|mut row| {
            for (key, value) in row.iter_mut() {
                if is_heuristically_sensitive(key) && !value.is_null() {
                    let raw = raw_string_of(value);
                    *value = Value::String(Masked::new(raw).to_string());
                }
            }
            row
        })
        .collect()
}

/// Render a JSON value as the "raw" string that gets masked. Strings are
/// used verbatim; any other non-null JSON value falls back to its
/// compact JSON rendering (sensitive columns are expected to be strings
/// in practice, but this stays total instead of panicking on a
/// surprising shape).
fn raw_string_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

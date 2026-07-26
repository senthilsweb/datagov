//! confidence.rs — The Bolt 5 confidence model, documented precisely so
//! it is deterministic and testable (see `crate::scanner` for how the
//! two bonus flags are computed and `docs/prd.md` §10.8/§10.9 for the
//! surrounding requirements).
//!
//! ```text
//! confidence = clamp(base_confidence + validator_bonus + context_bonus, 0.0, 1.0)
//! ```
//!
//! - `base_confidence`: the recognizer's declared confidence (a
//!   built-in's own value from `crate::builtins`, or a custom
//!   recognizer's YAML `confidence` field).
//! - `validator_bonus` = `+0.10` if the recognizer has at least one
//!   validator **and every** matched candidate substring found in the
//!   column passes **all** of them; `0.0` otherwise — no partial credit,
//!   and no bonus at all when the recognizer has no validator.
//! - `context_bonus` = `+0.05` if the column name case-insensitively
//!   contains any of the recognizer's context terms; `0.0` otherwise.

/// The `validator_bonus` awarded when a recognizer has a validator and
/// every candidate match in the column passed it.
pub const VALIDATOR_BONUS: f64 = 0.10;

/// The `context_bonus` awarded when the column name matches a context
/// term.
pub const CONTEXT_BONUS: f64 = 0.05;

/// Decimal places the final confidence is rounded to. `base_confidence`
/// plus one or two `+0.05`/`+0.10` bonuses can land on a binary `f64`
/// that isn't exactly representable (e.g. `0.80 + 0.05` prints as
/// `0.8500000000000001`, not `0.85`) — rounding to 6 decimal places (far
/// finer than this model's own bonus granularity of `0.05`) absorbs that
/// noise so the documented arithmetic is exactly what golden tests can
/// pin, the same rationale as `datagov_data::profile`'s
/// `STATS_ROUND_DECIMALS` correction in Bolt 3.
const CONFIDENCE_ROUND_DECIMALS: i32 = 6;

/// Compute the final confidence per the documented formula. `validator_ok`
/// and `context_ok` are the two boolean bonus conditions, already decided
/// by the caller (`crate::scanner`); this function only does the
/// arithmetic and the final clamp.
pub fn compute(base_confidence: f64, validator_ok: bool, context_ok: bool) -> f64 {
    let mut confidence = base_confidence;
    if validator_ok {
        confidence += VALIDATOR_BONUS;
    }
    if context_ok {
        confidence += CONTEXT_BONUS;
    }
    let scale = 10f64.powi(CONFIDENCE_ROUND_DECIMALS);
    let rounded = (confidence * scale).round() / scale;
    rounded.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_only() {
        assert_eq!(compute(0.75, false, false), 0.75);
    }

    #[test]
    fn base_plus_validator_bonus() {
        assert_eq!(compute(0.75, true, false), 0.85);
    }

    #[test]
    fn base_plus_context_bonus() {
        assert_eq!(compute(0.75, false, true), 0.80);
    }

    #[test]
    fn base_plus_both_bonuses() {
        // The golden arithmetic pinned in the Bolt 5 report: US_SSN's
        // base 0.75, plus both bonuses, is 0.90.
        assert_eq!(compute(0.75, true, true), 0.90);
    }

    #[test]
    fn clamps_at_one() {
        assert_eq!(compute(0.95, true, true), 1.0);
    }

    #[test]
    fn clamps_at_zero_for_a_negative_base() {
        assert_eq!(compute(-0.5, false, false), 0.0);
    }
}

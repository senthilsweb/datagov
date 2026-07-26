//! mask.rs — The single masking implementation for sensitive values.
//!
//! 1. Defines `Masked`, a newtype wrapping a sensitive `String` whose
//!    `Display` and `Serialize` impls emit only the masked form — no
//!    method on the type ever returns the inner value.
//! 2. Masking rule: strings of length 8 or more show up to 2 leading
//!    characters, an ellipsis, and up to 2 trailing characters; shorter
//!    strings are fully masked as `●●●`.
//! 3. **Bolt 5 note:** `pii scan`'s `PiiFinding::sample_evidence` reuses
//!    this exact generic type unchanged — the brief was explicit that
//!    there is to be no second masking implementation, so no per-entity
//!    mask styles were added here. This module stays deliberately
//!    generic.

use serde::Serialize;

/// A sensitive string that can only ever be observed in masked form.
///
/// There is no unmasked accessor — not on this type, not anywhere in
/// `datagov-core`. Masking is the only path by which a sampled value
/// reaches any output surface (stdout, stderr, logs, reports).
#[derive(Clone)]
pub struct Masked(String);

impl Masked {
    /// Wrap a sensitive value. The original is retained only internally;
    /// every observable rendering goes through the masking rule.
    pub fn new(value: impl Into<String>) -> Self {
        Masked(value.into())
    }

    fn masked(&self) -> String {
        let chars: Vec<char> = self.0.chars().collect();
        let len = chars.len();
        if len < 8 {
            "●●●".to_string()
        } else {
            let lead: String = chars.iter().take(2).collect();
            let trail: String = chars.iter().skip(len - 2).collect();
            format!("{lead}…{trail}")
        }
    }
}

impl std::fmt::Display for Masked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.masked())
    }
}

impl std::fmt::Debug for Masked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Masked").field(&self.masked()).finish()
    }
}

impl Serialize for Masked {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.masked())
    }
}

#[cfg(test)]
mod tests {
    use super::Masked;

    #[test]
    fn long_value_never_appears_in_full_in_display_or_serialize() {
        let sample = "4111111122223333"; // sampled full value, never to leak
        let masked = Masked::new(sample);

        let displayed = format!("{masked}");
        assert!(!displayed.contains(sample));
        assert_eq!(displayed, "41…33");

        let serialized = serde_json::to_string(&masked).unwrap();
        assert!(!serialized.contains(sample));
        assert_eq!(serialized, "\"41…33\"");
    }

    #[test]
    fn short_value_is_fully_masked() {
        let sample = "12345"; // len < 8
        let masked = Masked::new(sample);
        assert_eq!(format!("{masked}"), "●●●");
        assert_eq!(serde_json::to_string(&masked).unwrap(), "\"●●●\"");
    }

    #[test]
    fn boundary_length_eight_uses_lead_ellipsis_trail() {
        let sample = "12345678"; // len == 8
        let masked = Masked::new(sample);
        assert_eq!(format!("{masked}"), "12…78");
    }

    #[test]
    fn debug_rendering_also_never_leaks_the_value() {
        let sample = "sensitive-value-1234";
        let masked = Masked::new(sample);
        let debugged = format!("{masked:?}");
        assert!(!debugged.contains(sample));
    }
}

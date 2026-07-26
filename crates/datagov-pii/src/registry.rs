//! registry.rs — Merges built-in recognizers with custom ones loaded
//! from a `--recognizers` YAML file.
//!
//! `merge` starts from `crate::builtins::builtin_recognizers()` (fixed
//! order) and, for each custom recognizer in file order: replaces the
//! built-in with the same `id` in place (preserving the built-in's
//! position, so `pii recognizers list`-style ordering stays stable), or
//! appends it if no built-in shares that `id` — per the brief: "Custom
//! recognizers with an id matching a built-in override it; otherwise
//! they're added."

use crate::builtins::builtin_recognizers;
use crate::model::Recognizer;

/// Merge `customs` into the built-in table, overriding by `id`.
pub fn merge(customs: Vec<Recognizer>) -> Vec<Recognizer> {
    let mut merged = builtin_recognizers();

    for custom in customs {
        if let Some(slot) = merged.iter_mut().find(|r| r.id == custom.id) {
            *slot = custom;
        } else {
            merged.push(custom);
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::ValidatorKind;
    use regex::Regex;

    fn custom(id: &str) -> Recognizer {
        Recognizer {
            id: id.to_string(),
            entity: "CUSTOM".to_string(),
            base_confidence: 0.99,
            patterns: vec![Regex::new("x").unwrap()],
            context: vec![],
            validators: vec![],
        }
    }

    #[test]
    fn custom_recognizer_overrides_a_built_in_by_id() {
        let merged = merge(vec![custom("us_ssn")]);
        assert_eq!(merged.len(), 10, "override must not add a new entry");
        let us_ssn = merged.iter().find(|r| r.id == "us_ssn").unwrap();
        assert_eq!(us_ssn.base_confidence, 0.99);
        assert_eq!(us_ssn.entity, "CUSTOM");
    }

    #[test]
    fn custom_recognizer_with_a_new_id_is_appended() {
        let merged = merge(vec![custom("my_custom_entity")]);
        assert_eq!(merged.len(), 11);
        assert!(merged.iter().any(|r| r.id == "my_custom_entity"));
    }

    #[test]
    fn no_customs_returns_exactly_the_built_ins() {
        let merged = merge(vec![]);
        assert_eq!(merged.len(), builtin_recognizers().len());
    }

    #[test]
    fn validators_kind_still_usable_after_merge() {
        let mut with_validator = custom("with_validator");
        with_validator.validators = vec![ValidatorKind::Luhn];
        let merged = merge(vec![with_validator]);
        let found = merged.iter().find(|r| r.id == "with_validator").unwrap();
        assert_eq!(found.validators, vec![ValidatorKind::Luhn]);
    }
}

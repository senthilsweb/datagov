//! model.rs — The in-memory `Recognizer` shape shared by built-ins
//! (`crate::builtins`) and custom recognizers loaded from YAML
//! (`crate::yaml`), plus `RecognizerSummary` for `pii recognizers list`.
//!
//! 1. `Recognizer`: `id`, `entity`, `base_confidence`, compiled
//!    `patterns` (one or more alternative regexes — a match against
//!    *any* pattern counts), `context` terms (case-insensitive substring
//!    match against the column name), and `validators` (zero or more;
//!    when more than one is declared, a candidate match must pass *all*
//!    of them — an AND, generalizing the brief's singular "the
//!    recognizer has a validator" language to the YAML schema's plural
//!    `validators:` list).
//! 2. `RecognizerSummary`: the flat `{id, entity, confidence,
//!    pattern_count}` shape `pii recognizers list` renders — deliberately
//!    not the full `Recognizer` (which holds compiled, non-serializable
//!    `Regex` values).

use regex::Regex;
use serde::Serialize;

use crate::validators::ValidatorKind;

/// A single PII recognizer: a built-in (`crate::builtins`) or one loaded
/// from a `--recognizers` YAML file (`crate::yaml`).
#[derive(Debug, Clone)]
pub struct Recognizer {
    pub id: String,
    pub entity: String,
    pub base_confidence: f64,
    pub patterns: Vec<Regex>,
    pub context: Vec<String>,
    pub validators: Vec<ValidatorKind>,
}

impl Recognizer {
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn summary(&self) -> RecognizerSummary {
        RecognizerSummary {
            id: self.id.clone(),
            entity: self.entity.clone(),
            confidence: self.base_confidence,
            pattern_count: self.pattern_count(),
        }
    }
}

/// The `pii recognizers list` row shape (id, entity, declared base
/// confidence, pattern count) — table or JSON rendering.
#[derive(Debug, Clone, Serialize)]
pub struct RecognizerSummary {
    pub id: String,
    pub entity: String,
    pub confidence: f64,
    pub pattern_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reports_pattern_count() {
        let recognizer = Recognizer {
            id: "test".to_string(),
            entity: "TEST".to_string(),
            base_confidence: 0.5,
            patterns: vec![Regex::new("a").unwrap(), Regex::new("b").unwrap()],
            context: vec![],
            validators: vec![],
        };
        let summary = recognizer.summary();
        assert_eq!(summary.pattern_count, 2);
        assert_eq!(summary.confidence, 0.5);
    }
}

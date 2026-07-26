//! yaml.rs — Loads custom recognizers from the PRD §10.9 YAML schema
//! (`--recognizers <path>`, and `datagov pii recognizers validate`).
//!
//! 1. `RawRecognizersFile`/`RawRecognizer` mirror the exact PRD §10.9
//!    shape: `recognizers: [{id, entity, confidence, patterns, context,
//!    validators}]` — `context` and `validators` default to empty when
//!    omitted.
//! 2. `load_recognizers_file` reads and parses the YAML, then validates
//!    each entry before building a `Recognizer`. Each `patterns` entry
//!    must compile as a regex (a bad one is a load-time error naming the
//!    recognizer `id`, the `patterns` field, and the invalid pattern
//!    text); `confidence` must be in `[0.0, 1.0]` (naming `id` +
//!    `confidence`); each `validators` entry must be a known validator
//!    name, see `crate::validators::ValidatorKind` (naming `id` +
//!    `validators` and the unrecognized name). A malformed YAML document
//!    (fails to parse at all) is a load-time error naming the file path
//!    — there is no recognizer `id` to name in that case since the
//!    structure itself didn't parse. All of the above map to
//!    `DatagovError::InvalidArgs` (exit 2), per the `pii-detection` spec
//!    delta's "Invalid recognizer file" scenario.
//! 3. `validate_recognizers_file` is the same load path used by
//!    `datagov pii recognizers validate <path>` — success just means
//!    "it loaded", returning the count of recognizers found.

use std::path::Path;

use datagov_core::DatagovError;
use regex::Regex;
use serde::Deserialize;

use crate::model::Recognizer;
use crate::validators::ValidatorKind;

#[derive(Debug, Deserialize)]
struct RawRecognizersFile {
    recognizers: Vec<RawRecognizer>,
}

#[derive(Debug, Deserialize)]
struct RawRecognizer {
    id: String,
    entity: String,
    confidence: f64,
    patterns: Vec<String>,
    #[serde(default)]
    context: Vec<String>,
    #[serde(default)]
    validators: Vec<String>,
}

fn invalid(message: impl Into<String>) -> DatagovError {
    DatagovError::invalid_args(
        message,
        "fix the recognizer file and try again — see PRD §10.9 for the expected schema",
    )
}

/// Load and validate a `--recognizers` YAML file, returning the parsed
/// `Recognizer`s in file order. Every structural/semantic problem is
/// reported per the module docs above.
pub fn load_recognizers_file(path: &Path) -> Result<Vec<Recognizer>, DatagovError> {
    let text = std::fs::read_to_string(path).map_err(|source| {
        DatagovError::input_not_found(
            format!("cannot read recognizers file {}: {source}", path.display()),
            format!("check that {} exists and is readable", path.display()),
        )
    })?;
    parse_recognizers_text(&text, &path.display().to_string())
}

/// Load a `--recognizers` file purely to validate it (`pii recognizers
/// validate <path>`), returning the count of recognizers on success.
pub fn validate_recognizers_file(path: &Path) -> Result<usize, DatagovError> {
    load_recognizers_file(path).map(|recognizers| recognizers.len())
}

fn parse_recognizers_text(text: &str, source_name: &str) -> Result<Vec<Recognizer>, DatagovError> {
    let raw: RawRecognizersFile = serde_yaml::from_str(text).map_err(|source| {
        invalid(format!(
            "{source_name} is not a valid recognizers file: {source}"
        ))
    })?;

    let mut recognizers = Vec::with_capacity(raw.recognizers.len());
    for entry in raw.recognizers {
        recognizers.push(build_recognizer(entry)?);
    }
    Ok(recognizers)
}

fn build_recognizer(raw: RawRecognizer) -> Result<Recognizer, DatagovError> {
    let RawRecognizer {
        id,
        entity,
        confidence,
        patterns,
        context,
        validators,
    } = raw;

    if !(0.0..=1.0).contains(&confidence) {
        return Err(invalid(format!(
            "recognizer '{id}': field 'confidence' must be within [0.0, 1.0], got {confidence}"
        )));
    }

    if patterns.is_empty() {
        return Err(invalid(format!(
            "recognizer '{id}': field 'patterns' must have at least one entry"
        )));
    }

    let mut compiled_patterns = Vec::with_capacity(patterns.len());
    for pattern in &patterns {
        let compiled = Regex::new(pattern).map_err(|source| {
            invalid(format!(
                "recognizer '{id}': field 'patterns' has an invalid regex '{pattern}': {source}"
            ))
        })?;
        compiled_patterns.push(compiled);
    }

    let mut resolved_validators = Vec::with_capacity(validators.len());
    for name in &validators {
        let kind = ValidatorKind::from_name(name).ok_or_else(|| {
            invalid(format!(
                "recognizer '{id}': field 'validators' has an unknown validator '{name}'"
            ))
        })?;
        resolved_validators.push(kind);
    }

    Ok(Recognizer {
        id,
        entity,
        base_confidence: confidence,
        patterns: compiled_patterns,
        context,
        validators: resolved_validators,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_prd_10_9_example() {
        let text = r#"
recognizers:
  - id: us_ssn
    entity: US_SSN
    confidence: 0.85
    patterns:
      - '\b\d{3}-\d{2}-\d{4}\b'
    context:
      - ssn
      - social_security
      - taxpayer_id
    validators:
      - us_ssn
"#;
        let recognizers = parse_recognizers_text(text, "test.yaml").unwrap();
        assert_eq!(recognizers.len(), 1);
        assert_eq!(recognizers[0].id, "us_ssn");
        assert_eq!(recognizers[0].base_confidence, 0.85);
        assert_eq!(recognizers[0].validators, vec![ValidatorKind::UsSsn]);
    }

    #[test]
    fn context_and_validators_default_to_empty() {
        let text = r#"
recognizers:
  - id: custom_id
    entity: CUSTOM
    confidence: 0.5
    patterns:
      - 'abc'
"#;
        let recognizers = parse_recognizers_text(text, "test.yaml").unwrap();
        assert!(recognizers[0].context.is_empty());
        assert!(recognizers[0].validators.is_empty());
    }

    #[test]
    fn malformed_regex_names_the_id_and_field() {
        let text = r#"
recognizers:
  - id: broken
    entity: CUSTOM
    confidence: 0.5
    patterns:
      - '(unclosed'
"#;
        let err = parse_recognizers_text(text, "test.yaml").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
        assert!(err.message().contains("broken"));
        assert!(err.message().contains("patterns"));
    }

    #[test]
    fn out_of_range_confidence_names_the_id_and_field() {
        let text = r#"
recognizers:
  - id: too_high
    entity: CUSTOM
    confidence: 1.5
    patterns:
      - 'abc'
"#;
        let err = parse_recognizers_text(text, "test.yaml").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
        assert!(err.message().contains("too_high"));
        assert!(err.message().contains("confidence"));
    }

    #[test]
    fn unknown_validator_names_the_id_and_field() {
        let text = r#"
recognizers:
  - id: bad_validator
    entity: CUSTOM
    confidence: 0.5
    patterns:
      - 'abc'
    validators:
      - not_a_real_validator
"#;
        let err = parse_recognizers_text(text, "test.yaml").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
        assert!(err.message().contains("bad_validator"));
        assert!(err.message().contains("validators"));
        assert!(err.message().contains("not_a_real_validator"));
    }

    #[test]
    fn totally_malformed_yaml_is_invalid_args() {
        let err = parse_recognizers_text("not: [valid, yaml", "test.yaml").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }
}

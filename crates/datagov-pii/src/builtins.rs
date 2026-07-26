//! builtins.rs — The 10 built-in recognizers locked at the inception
//! gate (proposal.md Q3: email, phone, IPv4, IPv6, URL, US SSN, credit
//! card, UUID, MAC, US ZIP).
//!
//! **Decision (Bolt 5 construction):** the brief's per-entity table
//! (pattern approach + validator) does not specify numeric
//! `base_confidence` values — those are a genuine per-recognizer design
//! choice (PRD §10.9's own custom-recognizer example uses `0.85` for
//! `us_ssn` as its own YAML value, not a mandate for built-ins). The
//! values below are chosen to reflect each recognizer's inherent
//! precision *before* the validator/context bonuses: recognizers with a
//! validator that meaningfully rules out false positives (SSN's SSA
//! range check, credit card's Luhn checksum, MAC's fully-specific
//! format) start higher; recognizers with only a generic, easily
//! coincidental shape (a 5-digit ZIP, a bare UUID pattern) start lower.
//! Documented here as the single source of truth; the confidence-model
//! golden tests in `crates/datagov-cli/tests/pii.rs` pin the resulting
//! arithmetic against these exact numbers.
//!
//! | id | entity | base_confidence | validator |
//! |---|---|---|---|
//! | `email_address` | `EMAIL_ADDRESS` | 0.75 | none |
//! | `phone_number` | `PHONE_NUMBER` | 0.60 | none |
//! | `ip_address_v4` | `IP_ADDRESS_V4` | 0.55 | `ipv4` |
//! | `ip_address_v6` | `IP_ADDRESS_V6` | 0.55 | `ipv6` |
//! | `url` | `URL` | 0.65 | `url` |
//! | `us_ssn` | `US_SSN` | 0.75 | `us_ssn` |
//! | `credit_card` | `CREDIT_CARD` | 0.70 | `luhn` |
//! | `uuid` | `UUID` | 0.55 | `uuid` |
//! | `mac_address` | `MAC_ADDRESS` | 0.80 | none |
//! | `us_zip_code` | `US_ZIP_CODE` | 0.50 | `us_zip` |
//!
//! Pattern notes (see the brief for the full rationale):
//! - `PHONE_NUMBER`: North American Numbering Plan formats only — the
//!   fixture's `(664) 602-4412` style, plus `NXX-NXX-XXXX` and
//!   `NXX.NXX.XXXX`. International/E.164 numbers are explicitly out of
//!   scope.
//! - `IP_ADDRESS_V6`: the regex is a deliberately broad hex-and-colon
//!   prefilter (`[0-9A-Fa-f:]{2,45}`) — per the brief, hand-rolling a
//!   precise IPv6 regex is not the goal; `std::net::Ipv6Addr`'s own
//!   parser is the real gate, and it correctly rejects hex-only runs
//!   (no colon), MAC-shaped 6-group hex, and anything else that isn't a
//!   genuine address.
//! - `CREDIT_CARD`: matches a digit run of 13-19 digits allowing spaces
//!   or dashes as separators; the `luhn` validator strips separators and
//!   re-checks the resulting length before the checksum.
//! - `UUID`: canonical 8-4-4-4-12 hex regex; the `uuid` validator checks
//!   the version/variant nibbles (see `crate::validators`).

use regex::Regex;

use crate::model::Recognizer;
use crate::validators::ValidatorKind;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("built-in recognizer patterns are compile-time constants")
}

/// Build the fixed table of 10 built-in recognizers, in the stable order
/// documented above (used both by `pii recognizers list` and as the
/// starting point custom `--recognizers` entries are merged into).
pub fn builtin_recognizers() -> Vec<Recognizer> {
    vec![
        Recognizer {
            id: "email_address".to_string(),
            entity: "EMAIL_ADDRESS".to_string(),
            base_confidence: 0.75,
            patterns: vec![re(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b")],
            context: vec!["email".to_string(), "e-mail".to_string()],
            validators: vec![],
        },
        Recognizer {
            id: "phone_number".to_string(),
            entity: "PHONE_NUMBER".to_string(),
            base_confidence: 0.60,
            patterns: vec![re(
                r"\(\d{3}\)\s?\d{3}-\d{4}|\b\d{3}-\d{3}-\d{4}\b|\b\d{3}\.\d{3}\.\d{4}\b",
            )],
            context: vec![
                "phone".to_string(),
                "phone_number".to_string(),
                "mobile".to_string(),
                "cell".to_string(),
                "telephone".to_string(),
            ],
            validators: vec![],
        },
        Recognizer {
            id: "ip_address_v4".to_string(),
            entity: "IP_ADDRESS_V4".to_string(),
            base_confidence: 0.55,
            patterns: vec![re(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")],
            context: vec![
                "ipv4".to_string(),
                "ip_address".to_string(),
                "ip address".to_string(),
                "ipaddr".to_string(),
            ],
            validators: vec![ValidatorKind::Ipv4],
        },
        Recognizer {
            id: "ip_address_v6".to_string(),
            entity: "IP_ADDRESS_V6".to_string(),
            base_confidence: 0.55,
            patterns: vec![re(r"[0-9A-Fa-f:]{2,45}")],
            context: vec![
                "ipv6".to_string(),
                "ip_address".to_string(),
                "ip address".to_string(),
                "ipaddr".to_string(),
            ],
            validators: vec![ValidatorKind::Ipv6],
        },
        Recognizer {
            id: "url".to_string(),
            entity: "URL".to_string(),
            base_confidence: 0.65,
            patterns: vec![re(r#"(?i)\bhttps?://[^\s"'<>]+"#)],
            context: vec![
                "url".to_string(),
                "website".to_string(),
                "link".to_string(),
                "uri".to_string(),
            ],
            validators: vec![ValidatorKind::Url],
        },
        Recognizer {
            id: "us_ssn".to_string(),
            entity: "US_SSN".to_string(),
            base_confidence: 0.75,
            patterns: vec![re(r"\b\d{3}-\d{2}-\d{4}\b")],
            context: vec![
                "ssn".to_string(),
                "social_security".to_string(),
                "taxpayer_id".to_string(),
            ],
            validators: vec![ValidatorKind::UsSsn],
        },
        Recognizer {
            id: "credit_card".to_string(),
            entity: "CREDIT_CARD".to_string(),
            base_confidence: 0.70,
            patterns: vec![re(r"\b\d[\d -]{11,21}\d\b")],
            context: vec![
                "credit_card".to_string(),
                "card_number".to_string(),
                "creditcard".to_string(),
                "cc_number".to_string(),
            ],
            validators: vec![ValidatorKind::Luhn],
        },
        Recognizer {
            id: "uuid".to_string(),
            entity: "UUID".to_string(),
            base_confidence: 0.55,
            patterns: vec![re(
                r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
            )],
            context: vec!["uuid".to_string(), "guid".to_string()],
            validators: vec![ValidatorKind::Uuid],
        },
        Recognizer {
            id: "mac_address".to_string(),
            entity: "MAC_ADDRESS".to_string(),
            base_confidence: 0.80,
            patterns: vec![re(r"(?i)\b(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}\b")],
            context: vec![
                "mac_address".to_string(),
                "macaddr".to_string(),
                "hwaddr".to_string(),
            ],
            validators: vec![],
        },
        Recognizer {
            id: "us_zip_code".to_string(),
            entity: "US_ZIP_CODE".to_string(),
            base_confidence: 0.50,
            patterns: vec![re(r"\b\d{5}(?:-\d{4})?\b")],
            context: vec![
                "zip".to_string(),
                "zip_code".to_string(),
                "zipcode".to_string(),
                "postal_code".to_string(),
                "postal".to_string(),
            ],
            validators: vec![ValidatorKind::UsZip],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_built_ins_with_unique_ids() {
        let recognizers = builtin_recognizers();
        assert_eq!(recognizers.len(), 10);
        let mut ids: Vec<&str> = recognizers.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn every_pattern_compiles_and_confidence_is_in_range() {
        for recognizer in builtin_recognizers() {
            assert!(!recognizer.patterns.is_empty());
            assert!(recognizer.base_confidence >= 0.0 && recognizer.base_confidence <= 1.0);
        }
    }
}

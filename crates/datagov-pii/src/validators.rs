//! validators.rs — Deterministic validators for the 10 built-in PII
//! entities (PRD §10.8), plus dispatch for custom recognizers' YAML
//! `validators:` names (PRD §10.9).
//!
//! 1. Defines `ValidatorKind`, one variant per known validator name
//!    (`luhn`, `ipv4`, `ipv6`, `url`, `us_ssn`, `uuid`, `us_zip`) —
//!    `ValidatorKind::from_name`/`name` round-trip the YAML string form,
//!    used both by the built-in table (`crate::builtins`) and the custom
//!    recognizer loader (`crate::yaml`), which rejects any other name as
//!    a load-time error naming the recognizer id and field.
//! 2. `validate` dispatches a matched substring to the right check:
//!    - `Luhn`: strips spaces/dashes, requires 13-19 resulting digits,
//!      then the standard Luhn checksum (credit cards).
//!    - `Ipv4`/`Ipv6`: **the parse *is* the validator** — no hand-written
//!      octet-range regex, per the brief. `std::net::Ipv4Addr`/
//!      `Ipv6Addr`'s `FromStr` do the real work.
//!    - `Url`: parses with the `url` crate and requires a non-empty
//!      scheme and a host (rejects `mailto:`-style URLs with no host,
//!      and anything that fails to parse at all).
//!    - `UsSsn`: rejects known-invalid SSA area numbers (`000`, `666`,
//!      `900`-`999`) — the brief's explicit, minimal check; group/serial
//!      validity is out of scope.
//!    - `Uuid`: strips dashes to the 32-hex-character form and checks
//!      the version nibble (hex position 13, 1-based, i.e. index 12) is
//!      `1`-`5` and the variant nibble (hex position 17, i.e. index 16)
//!      is `8`/`9`/`a`/`b` — the brief's exact positions, confirmed
//!      against the canonical 8-4-4-4-12 grouping (group1+group2 = 12
//!      hex chars, so the version nibble is the first char of group3;
//!      +group3 = 16, so the variant nibble is the first char of
//!      group4).
//!    - `UsZip`: rejects a small, documented deny-list of obviously
//!      invalid placeholder ZIPs (`00000`, `99999`) on the 5-digit
//!      prefix — kept deliberately light per the brief.

use std::net::{Ipv4Addr, Ipv6Addr};

/// One of the validators a recognizer (built-in or custom) can declare.
/// The `name`/`from_name` pair is the YAML `validators:` vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorKind {
    Luhn,
    Ipv4,
    Ipv6,
    Url,
    UsSsn,
    Uuid,
    UsZip,
}

impl ValidatorKind {
    pub fn name(self) -> &'static str {
        match self {
            ValidatorKind::Luhn => "luhn",
            ValidatorKind::Ipv4 => "ipv4",
            ValidatorKind::Ipv6 => "ipv6",
            ValidatorKind::Url => "url",
            ValidatorKind::UsSsn => "us_ssn",
            ValidatorKind::Uuid => "uuid",
            ValidatorKind::UsZip => "us_zip",
        }
    }

    /// Parse a YAML `validators:` entry name (case-sensitive, matching
    /// PRD §10.9's own example verbatim, e.g. `us_ssn`). `None` for any
    /// unrecognized name — the caller (`crate::yaml`) turns that into a
    /// load-time `InvalidArgs` error naming the recognizer id and field.
    pub fn from_name(name: &str) -> Option<ValidatorKind> {
        match name {
            "luhn" => Some(ValidatorKind::Luhn),
            "ipv4" => Some(ValidatorKind::Ipv4),
            "ipv6" => Some(ValidatorKind::Ipv6),
            "url" => Some(ValidatorKind::Url),
            "us_ssn" => Some(ValidatorKind::UsSsn),
            "uuid" => Some(ValidatorKind::Uuid),
            "us_zip" => Some(ValidatorKind::UsZip),
            _ => None,
        }
    }
}

/// Apply `kind` to a single matched substring `candidate`. `true` means
/// the candidate is a genuine instance of the entity, not just a regex
/// prefilter hit.
pub fn validate(kind: ValidatorKind, candidate: &str) -> bool {
    match kind {
        ValidatorKind::Luhn => luhn_valid(candidate),
        ValidatorKind::Ipv4 => candidate.parse::<Ipv4Addr>().is_ok(),
        ValidatorKind::Ipv6 => candidate.parse::<Ipv6Addr>().is_ok(),
        ValidatorKind::Url => url_valid(candidate),
        ValidatorKind::UsSsn => us_ssn_valid(candidate),
        ValidatorKind::Uuid => uuid_valid(candidate),
        ValidatorKind::UsZip => us_zip_valid(candidate),
    }
}

fn luhn_valid(raw: &str) -> bool {
    if raw
        .chars()
        .any(|c| !(c.is_ascii_digit() || c == ' ' || c == '-'))
    {
        return false;
    }
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let len = digits.len();
    if !(13..=19).contains(&len) {
        return false;
    }

    let mut sum = 0u32;
    let mut double = false;
    for c in digits.chars().rev() {
        let mut d = c.to_digit(10).expect("filtered to ascii digits above");
        if double {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        double = !double;
    }
    sum.is_multiple_of(10)
}

const INVALID_SSN_AREAS: [u32; 2] = [0, 666];

fn us_ssn_valid(raw: &str) -> bool {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 9 {
        return false;
    }
    let Ok(area) = digits[0..3].parse::<u32>() else {
        return false;
    };
    !INVALID_SSN_AREAS.contains(&area) && area < 900
}

fn uuid_valid(raw: &str) -> bool {
    let hex: String = raw.chars().filter(|&c| c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let chars: Vec<char> = hex.chars().collect();
    let version = chars[12];
    let variant = chars[16].to_ascii_lowercase();
    matches!(version, '1'..='5') && matches!(variant, '8' | '9' | 'a' | 'b')
}

fn url_valid(raw: &str) -> bool {
    match url::Url::parse(raw) {
        Ok(parsed) => !parsed.scheme().is_empty() && parsed.host().is_some(),
        Err(_) => false,
    }
}

const INVALID_ZIPS: [&str; 2] = ["00000", "99999"];

fn us_zip_valid(raw: &str) -> bool {
    let prefix = &raw[..raw.len().min(5)];
    !INVALID_ZIPS.contains(&prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_accepts_known_test_card_numbers() {
        assert!(validate(ValidatorKind::Luhn, "4111111111111111"));
        assert!(validate(ValidatorKind::Luhn, "5555555555554444"));
        assert!(validate(ValidatorKind::Luhn, "378282246310005"));
        assert!(validate(ValidatorKind::Luhn, "6011111111111117"));
    }

    #[test]
    fn luhn_accepts_separators_and_rejects_bad_checksum() {
        assert!(validate(ValidatorKind::Luhn, "4111 1111 1111 1111"));
        assert!(validate(ValidatorKind::Luhn, "4111-1111-1111-1111"));
        assert!(!validate(ValidatorKind::Luhn, "4111111111111112"));
    }

    #[test]
    fn luhn_rejects_out_of_range_lengths() {
        assert!(!validate(ValidatorKind::Luhn, "411111111111")); // 12 digits
        assert!(!validate(ValidatorKind::Luhn, "41111111111111111111")); // 20 digits
    }

    #[test]
    fn ipv4_parse_is_the_validator() {
        assert!(validate(ValidatorKind::Ipv4, "192.0.2.1"));
        assert!(!validate(ValidatorKind::Ipv4, "999.0.2.1"));
        assert!(!validate(ValidatorKind::Ipv4, "192.0.2"));
    }

    #[test]
    fn ipv6_parse_is_the_validator() {
        assert!(validate(ValidatorKind::Ipv6, "2001:db8::1"));
        assert!(!validate(ValidatorKind::Ipv6, "not-an-address"));
        assert!(!validate(ValidatorKind::Ipv6, "01:23:45:67:89:ab")); // MAC-shaped, not IPv6
    }

    #[test]
    fn url_requires_scheme_and_host() {
        assert!(validate(ValidatorKind::Url, "https://example.com/docs"));
        assert!(!validate(ValidatorKind::Url, "mailto:someone@example.com"));
        assert!(!validate(ValidatorKind::Url, "not a url"));
    }

    #[test]
    fn us_ssn_rejects_invalid_area_numbers() {
        assert!(validate(ValidatorKind::UsSsn, "555-01-0001"));
        assert!(!validate(ValidatorKind::UsSsn, "000-12-3456"));
        assert!(!validate(ValidatorKind::UsSsn, "666-12-3456"));
        assert!(!validate(ValidatorKind::UsSsn, "900-12-3456"));
        assert!(!validate(ValidatorKind::UsSsn, "999-12-3456"));
    }

    #[test]
    fn uuid_checks_version_and_variant_nibbles() {
        // Version 4, variant b — a well-known public example UUID.
        assert!(validate(
            ValidatorKind::Uuid,
            "3fa85f64-5717-4562-b3fc-2c963f66afa6"
        ));
        // Version 1, variant 8 — RFC 4122's own namespace UUID.
        assert!(validate(
            ValidatorKind::Uuid,
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        ));
        // Version nibble 0 is invalid (not 1-5).
        assert!(!validate(
            ValidatorKind::Uuid,
            "3fa85f64-5717-0562-b3fc-2c963f66afa6"
        ));
        // Variant nibble 'c' is invalid (not 8/9/a/b).
        assert!(!validate(
            ValidatorKind::Uuid,
            "3fa85f64-5717-4562-cbfc-2c963f66afa6"
        ));
    }

    #[test]
    fn us_zip_rejects_denylisted_placeholders() {
        assert!(validate(ValidatorKind::UsZip, "12345"));
        assert!(validate(ValidatorKind::UsZip, "12345-6789"));
        assert!(!validate(ValidatorKind::UsZip, "00000"));
        assert!(!validate(ValidatorKind::UsZip, "99999"));
    }

    #[test]
    fn validator_kind_name_round_trips() {
        for kind in [
            ValidatorKind::Luhn,
            ValidatorKind::Ipv4,
            ValidatorKind::Ipv6,
            ValidatorKind::Url,
            ValidatorKind::UsSsn,
            ValidatorKind::Uuid,
            ValidatorKind::UsZip,
        ] {
            assert_eq!(ValidatorKind::from_name(kind.name()), Some(kind));
        }
        assert_eq!(ValidatorKind::from_name("not_a_validator"), None);
    }
}

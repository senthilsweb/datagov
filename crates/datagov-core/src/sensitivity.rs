//! sensitivity.rs — The Bolt 2 sensitive-column-name heuristic.
//!
//! 1. Defines `is_heuristically_sensitive`, a case-insensitive substring
//!    match against a small const list of column-name fragments that
//!    typically identify direct identifiers.
//! 2. This is a conservative, always-on heuristic that predates the full
//!    PII recognizer engine arriving in Bolt 5 (PRD §10.8/§10.9). It
//!    exists so `inspect` never echoes obvious identifier columns in
//!    sample rows before that subsystem exists. Bolt 5's recognizer
//!    engine supersedes and extends this heuristic — column-name
//!    matching stays in place alongside value-based detection, it is not
//!    silently replaced.

/// Column-name fragments that, if present (case-insensitively, as a
/// substring), mark a column as sensitive under this heuristic.
const SENSITIVE_NAME_FRAGMENTS: &[&str] = &[
    "email",
    "phone",
    "ssn",
    "social_security",
    "credit_card",
    "card_number",
    "password",
    "secret",
];

/// Conservative, case-insensitive substring check on a column name.
///
/// Returns `true` if `column_name` contains any of the known sensitive
/// fragments (`email`, `phone`, `ssn`, `social_security`, `credit_card`,
/// `card_number`, `password`, `secret`) regardless of case. This is
/// deliberately blunt — a name-only heuristic, not a value-based
/// detector — so it is cheap enough to run on every sampled row before
/// the full recognizer engine (Bolt 5) exists.
pub fn is_heuristically_sensitive(column_name: &str) -> bool {
    let lower = column_name.to_ascii_lowercase();
    SENSITIVE_NAME_FRAGMENTS
        .iter()
        .any(|fragment| lower.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::is_heuristically_sensitive;

    #[test]
    fn matches_known_fragments_case_insensitively() {
        assert!(is_heuristically_sensitive("email"));
        assert!(is_heuristically_sensitive("Email_Address"));
        assert!(is_heuristically_sensitive("PHONE_NUMBER"));
        assert!(is_heuristically_sensitive("ssn"));
        assert!(is_heuristically_sensitive("Social_Security_Number"));
        assert!(is_heuristically_sensitive("credit_card_last4"));
        assert!(is_heuristically_sensitive("CARD_NUMBER"));
        assert!(is_heuristically_sensitive("user_password"));
        assert!(is_heuristically_sensitive("client_secret"));
    }

    #[test]
    fn does_not_match_unrelated_names() {
        assert!(!is_heuristically_sensitive("username"));
        assert!(!is_heuristically_sensitive("city"));
        assert!(!is_heuristically_sensitive("state"));
        assert!(!is_heuristically_sensitive("userid"));
    }
}

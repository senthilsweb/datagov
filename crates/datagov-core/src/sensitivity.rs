//! sensitivity.rs — The Bolt 2 sensitive-column-name heuristic.
//!
//! 1. Defines `heuristically_sensitive_match`, a case-insensitive
//!    substring match against a small const list of column-name
//!    fragments that typically identify direct identifiers, returning
//!    the specific fragment matched (used from Bolt 3 onward as the
//!    `semantic_type` label for a sensitive column).
//! 2. Defines `is_heuristically_sensitive` in terms of it, for callers
//!    that only need the yes/no check (Bolt 2's `inspect` sample-row
//!    masking).
//! 3. This is a conservative, always-on heuristic that predates the full
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

/// Conservative, case-insensitive substring check on a column name,
/// returning the specific fragment that matched (e.g. `"email"`,
/// `"phone"`), or `None` if `column_name` matches no known sensitive
/// fragment. This is deliberately blunt — a name-only heuristic, not a
/// value-based detector — so it is cheap enough to run on every sampled
/// row before the full recognizer engine (Bolt 5) exists.
pub fn heuristically_sensitive_match(column_name: &str) -> Option<&'static str> {
    let lower = column_name.to_ascii_lowercase();
    SENSITIVE_NAME_FRAGMENTS
        .iter()
        .find(|fragment| lower.contains(*fragment))
        .copied()
}

/// `true` if `column_name` matches any known sensitive fragment
/// (`email`, `phone`, `ssn`, `social_security`, `credit_card`,
/// `card_number`, `password`, `secret`) regardless of case. See
/// [`heuristically_sensitive_match`] for the fragment-naming variant.
pub fn is_heuristically_sensitive(column_name: &str) -> bool {
    heuristically_sensitive_match(column_name).is_some()
}

#[cfg(test)]
mod tests {
    use super::{heuristically_sensitive_match, is_heuristically_sensitive};

    #[test]
    fn match_names_the_matched_fragment() {
        assert_eq!(heuristically_sensitive_match("email"), Some("email"));
        assert_eq!(heuristically_sensitive_match("Phone_Number"), Some("phone"));
        assert_eq!(heuristically_sensitive_match("username"), None);
    }

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

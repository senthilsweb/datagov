//! exit.rs — The exit-code contract (PRD §24).
//!
//! 1. Defines `ExitCode`, a fixed enum with explicit discriminants that
//!    mirror the PRD §24 table verbatim; the table is a contract, so the
//!    discriminants must never be renumbered.
//! 2. Provides `ExitCode::code()` to convert into the `i32` value that
//!    `std::process::exit` expects.
//! 3. Unit tests pin every discriminant so an accidental reordering fails
//!    the test suite instead of silently shipping a different exit code.

/// The documented `datagov` exit-code contract (PRD §24).
///
/// New failure modes must map to an existing code or extend this table
/// through a spec change — never an ad-hoc `std::process::exit` value.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitCode {
    /// Success; no threshold violations.
    Success = 0,
    /// Execution or internal error.
    Internal = 1,
    /// Invalid arguments or configuration.
    InvalidArgs = 2,
    /// Input not found or unreadable.
    InputNotFound = 3,
    /// Unsupported input or dialect.
    UnsupportedInput = 4,
    /// Quality threshold failed.
    QualityFailed = 10,
    /// Policy threshold failed.
    PolicyFailed = 11,
    /// PII threshold failed.
    PiiFailed = 12,
    /// Schema validation failed.
    SchemaFailed = 13,
    /// Lineage extraction incomplete beyond configured threshold.
    LineageIncomplete = 14,
    /// Optional backend unavailable.
    BackendUnavailable = 20,
    /// Authentication or authorization failure.
    AuthFailed = 21,
    /// Plugin or model verification failed.
    VerificationFailed = 22,
}

impl ExitCode {
    /// The `i32` process exit code.
    pub fn code(self) -> i32 {
        self as i32
    }
}

impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::ExitCode;

    /// The PRD §24 table is a contract: every discriminant is pinned here
    /// so a reorder or renumber breaks the build instead of shipping
    /// silently.
    #[test]
    fn discriminants_match_prd_24() {
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::Internal.code(), 1);
        assert_eq!(ExitCode::InvalidArgs.code(), 2);
        assert_eq!(ExitCode::InputNotFound.code(), 3);
        assert_eq!(ExitCode::UnsupportedInput.code(), 4);
        assert_eq!(ExitCode::QualityFailed.code(), 10);
        assert_eq!(ExitCode::PolicyFailed.code(), 11);
        assert_eq!(ExitCode::PiiFailed.code(), 12);
        assert_eq!(ExitCode::SchemaFailed.code(), 13);
        assert_eq!(ExitCode::LineageIncomplete.code(), 14);
        assert_eq!(ExitCode::BackendUnavailable.code(), 20);
        assert_eq!(ExitCode::AuthFailed.code(), 21);
        assert_eq!(ExitCode::VerificationFailed.code(), 22);
    }
}

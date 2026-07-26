//! error.rs — The typed error enum shared by every `datagov` command.
//!
//! 1. Defines `DatagovError`, a `thiserror` enum whose variants each carry
//!    remediation text a user can act on.
//! 2. Maps every variant to its `ExitCode` per PRD §24 via
//!    `DatagovError::exit_code`.
//! 3. Renders errors to stderr: a human one-liner with remediation by
//!    default, or a single JSON object
//!    (`{"code", "exit_code", "message", "remediation"}`) under
//!    `--output json`.
//! 4. Unit tests pin the complete variant → exit-code mapping for Bolt 1
//!    (`Internal` = 1, `InvalidArgs` = 2, `InputNotFound` = 3,
//!    `UnsupportedInput` = 4). Later bolts add variants only as their
//!    domains land, never renumbering existing ones.

use crate::exit::ExitCode;
use thiserror::Error;

/// The error type every `datagov` command funnels failures through on the
/// way to a process exit code.
#[derive(Debug, Error)]
pub enum DatagovError {
    /// An unexpected internal failure (bug, I/O surprise, panic bridge).
    #[error("internal error: {message}")]
    Internal {
        message: String,
        remediation: String,
    },

    /// The user supplied invalid arguments or configuration.
    #[error("invalid arguments: {message}")]
    InvalidArgs {
        message: String,
        remediation: String,
    },

    /// The requested input path does not exist or cannot be read.
    #[error("input not found: {message}")]
    InputNotFound {
        message: String,
        remediation: String,
    },

    /// The input format or dialect is not supported.
    #[error("unsupported input: {message}")]
    UnsupportedInput {
        message: String,
        remediation: String,
    },
}

impl DatagovError {
    /// Construct an `Internal` error with default remediation guidance.
    pub fn internal(message: impl Into<String>) -> Self {
        DatagovError::Internal {
            message: message.into(),
            remediation: "This is likely a bug; please file an issue with the command you ran."
                .to_string(),
        }
    }

    /// Construct an `InvalidArgs` error.
    pub fn invalid_args(message: impl Into<String>, remediation: impl Into<String>) -> Self {
        DatagovError::InvalidArgs {
            message: message.into(),
            remediation: remediation.into(),
        }
    }

    /// Construct an `InputNotFound` error.
    pub fn input_not_found(message: impl Into<String>, remediation: impl Into<String>) -> Self {
        DatagovError::InputNotFound {
            message: message.into(),
            remediation: remediation.into(),
        }
    }

    /// Construct an `UnsupportedInput` error.
    pub fn unsupported_input(message: impl Into<String>, remediation: impl Into<String>) -> Self {
        DatagovError::UnsupportedInput {
            message: message.into(),
            remediation: remediation.into(),
        }
    }

    /// The stable machine-readable code (distinct from the process exit
    /// code) surfaced in the JSON error rendering.
    pub fn code(&self) -> &'static str {
        match self {
            DatagovError::Internal { .. } => "internal_error",
            DatagovError::InvalidArgs { .. } => "invalid_args",
            DatagovError::InputNotFound { .. } => "input_not_found",
            DatagovError::UnsupportedInput { .. } => "unsupported_input",
        }
    }

    /// The PRD §24 exit code this error maps to.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            DatagovError::Internal { .. } => ExitCode::Internal,
            DatagovError::InvalidArgs { .. } => ExitCode::InvalidArgs,
            DatagovError::InputNotFound { .. } => ExitCode::InputNotFound,
            DatagovError::UnsupportedInput { .. } => ExitCode::UnsupportedInput,
        }
    }

    /// The human-readable message (no remediation).
    pub fn message(&self) -> &str {
        match self {
            DatagovError::Internal { message, .. }
            | DatagovError::InvalidArgs { message, .. }
            | DatagovError::InputNotFound { message, .. }
            | DatagovError::UnsupportedInput { message, .. } => message,
        }
    }

    /// The remediation guidance for this error.
    pub fn remediation(&self) -> &str {
        match self {
            DatagovError::Internal { remediation, .. }
            | DatagovError::InvalidArgs { remediation, .. }
            | DatagovError::InputNotFound { remediation, .. }
            | DatagovError::UnsupportedInput { remediation, .. } => remediation,
        }
    }

    /// The single JSON object rendering used under `--output json`:
    /// `{"code", "exit_code", "message", "remediation"}`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.code(),
            "exit_code": self.exit_code().code(),
            "message": self.message(),
            "remediation": self.remediation(),
        })
    }

    /// Render this error to stderr: a single JSON object when
    /// `json_output` is set, otherwise a human one-liner with
    /// remediation. Diagnostics elsewhere in the CLI go through
    /// `tracing`; this is the terminal failure rendering, always on
    /// stderr, never stdout.
    pub fn print(&self, json_output: bool) {
        if json_output {
            eprintln!("{}", self.to_json());
        } else {
            eprintln!(
                "error: {}\nremediation: {}",
                self.message(),
                self.remediation()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_to_exit_code_mapping() {
        assert_eq!(
            DatagovError::internal("boom").exit_code(),
            ExitCode::Internal
        );
        assert_eq!(
            DatagovError::invalid_args("bad flag", "fix it").exit_code(),
            ExitCode::InvalidArgs
        );
        assert_eq!(
            DatagovError::input_not_found("missing.csv", "check the path").exit_code(),
            ExitCode::InputNotFound
        );
        assert_eq!(
            DatagovError::unsupported_input("unknown format", "use a supported format").exit_code(),
            ExitCode::UnsupportedInput
        );
    }

    #[test]
    fn json_rendering_has_all_four_fields() {
        let err = DatagovError::invalid_args("bad flag", "fix it");
        let json = err.to_json();
        assert_eq!(json["code"], "invalid_args");
        assert_eq!(json["exit_code"], 2);
        assert_eq!(json["message"], "bad flag");
        assert_eq!(json["remediation"], "fix it");
    }
}

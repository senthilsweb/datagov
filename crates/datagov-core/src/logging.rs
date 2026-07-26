//! logging.rs — Shared `tracing` subscriber setup for every `datagov`
//! command.
//!
//! 1. Provides `init(quiet, verbose)`, which installs a global
//!    `tracing-subscriber` writing exclusively to stderr — stdout is
//!    reserved for command output.
//! 2. Maps CLI flags to verbosity: `--verbose` → `DEBUG` level, JSON
//!    lines; default → `WARN` level, compact human text; `--quiet` →
//!    `ERROR` only.
//! 3. Honours the `DATAGOV_LOG` environment variable as a filter
//!    override that takes precedence over the `quiet`/`verbose` flags.
//! 4. Guards against double initialization with a `std::sync::Once`, so a
//!    second call is a silent no-op instead of a panic.
//!
//! Environment variables read: `DATAGOV_LOG` (tracing `EnvFilter`
//! directive string, e.g. `datagov=debug`).

use std::sync::Once;
use tracing_subscriber::EnvFilter;

static INIT: Once = Once::new();

/// Install the global `tracing` subscriber, writing to stderr only.
///
/// `verbose` takes DEBUG level with JSON-lines formatting; otherwise the
/// default is WARN with compact human formatting, or ERROR only when
/// `quiet` is set. `DATAGOV_LOG` overrides the computed level with an
/// explicit `EnvFilter` directive string when present. Calling this more
/// than once is a no-op — only the first call takes effect.
pub fn init(quiet: bool, verbose: bool) {
    INIT.call_once(|| {
        let filter = std::env::var("DATAGOV_LOG")
            .ok()
            .and_then(|directive| EnvFilter::try_new(directive).ok())
            .unwrap_or_else(|| {
                let level = if verbose {
                    "debug"
                } else if quiet {
                    "error"
                } else {
                    "warn"
                };
                EnvFilter::new(level)
            });

        if verbose {
            let _ = tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(filter)
                .json()
                .try_init();
        } else {
            let _ = tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(filter)
                .compact()
                .try_init();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::init;

    #[test]
    fn double_init_is_a_no_op() {
        // The first call installs the subscriber; the second must not
        // panic even though a global subscriber is already set.
        init(false, false);
        init(true, true);
    }
}

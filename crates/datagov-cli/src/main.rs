//! main.rs — `datagov` CLI entry point: clap parsing, subcommand dispatch,
//! and the exit-code funnel.
//!
//! 1. Parses global flags (`--output`, `--quiet`, `--verbose`) and
//!    subcommands (`version`, `capabilities`, `inspect`, `profile`,
//!    `query`) via clap derive (`cli::Cli`).
//! 2. Initializes the shared `tracing` subscriber via
//!    `datagov_core::logging::init` before dispatching any command.
//! 3. Dispatches to one thin command function per subcommand
//!    (`commands::*::run`) — parse -> call a `datagov-core` function ->
//!    render. No business logic lives in this file beyond wiring.
//! 4. Funnels every error through `DatagovError` -> `ExitCode`, printing
//!    it to stderr (human one-liner, or a single JSON object under
//!    `--output json`) and exiting with the mapped process exit code.
//!    Clap's own usage errors (unknown flag, unknown subcommand,
//!    conflicting flags) exit 2 — clap 4's default, pinned by an
//!    integration test rather than reimplemented here.
//!
//! Environment variables read: `DATAGOV_LOG` (see
//! `datagov_core::logging::init` for the filter format).

mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Command, OutputFormat};

fn main() {
    let cli = Cli::parse();
    datagov_core::logging::init(cli.quiet, cli.verbose);

    let json_output = matches!(cli.output, OutputFormat::Json);

    let result = match cli.command {
        Command::Version => commands::version::run(cli.output),
        Command::Capabilities => commands::capabilities::run(cli.output),
        Command::Inspect { path, r#type } => {
            commands::inspect::run(&path, r#type.as_deref(), cli.output)
        }
        Command::Profile {
            path,
            r#type,
            columns,
            sample,
        } => commands::profile::run(
            &path,
            r#type.as_deref(),
            columns.as_deref(),
            sample,
            cli.output,
        ),
        Command::Query { sql, limit } => commands::query::run(&sql, limit, cli.output),
    };

    if let Err(err) = result {
        err.print(json_output);
        std::process::exit(err.exit_code().code());
    }
}

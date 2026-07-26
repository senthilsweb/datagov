//! sql.rs — `datagov sql parse | format | transpile`: thin adapters, no
//! business logic.
//!
//! 1. `read_sql_input` is the shared stdin/file reader (`-` reads all of
//!    stdin, same convention as `inspect`); a missing file is
//!    `DatagovError::InputNotFound` (exit 3).
//! 2. All three subcommands populate the envelope's `input` block (`uri`,
//!    `format: "sql"`, `content_hash` — omitted for stdin) — unlike
//!    `query` (Bolt 3), which deliberately has no `input` section
//!    because a query can span zero/one/many files. `parse`/`format`/
//!    `transpile` each operate on exactly one SQL source, so the
//!    single-file `input` block used by `inspect`/`profile` is the
//!    consistent choice here.
//! 3. **`parse`**: delegates to `datagov_sql::parse`; JSON output puts
//!    the full `SqlParseResult` (including the raw AST) under
//!    `extensions.sql_parse`; table output is a compact human summary
//!    (dialect, statement type, tables, columns, joins, filters,
//!    grouping, ordering, CTEs) — the full AST is JSON-only (it isn't
//!    meant to be read as a table).
//! 4. **`format`**: delegates to `datagov_sql::format`. The **source
//!    file's `content_hash` is computed from the file before any
//!    write** (so JSON output always describes the input that was
//!    formatted, never the just-rewritten output). `--write` on stdin
//!    (`-`) is `InvalidArgs` (exit 2) — there is no source file to
//!    modify. Without `--write`, the file on disk is never touched
//!    (HARD: byte-identical + mtime unchanged); with `--write`, the
//!    formatted SQL replaces the file's contents. The formatted SQL is
//!    also always printed per `--output` (table: raw text to stdout;
//!    json: the envelope, `extensions.sql_format`), independent of
//!    `--write`.
//! 5. **`transpile`**: delegates to `datagov_sql::transpile`. Table
//!    output prints the transpiled SQL to stdout (kept pipeable — no
//!    extra text mixed in) and prints each warning to **stderr** (one
//!    line per `TranspileWarning`, `warning: [<construct>] <message>`)
//!    — the diagnostics channel per `AGENTS.md` #5, not swallowed
//!    either way. JSON output carries the same warnings in
//!    `extensions.sql_transpile.warnings`, always present (even when
//!    empty) — never omitted.
//! 6. `--output csv` is rejected (exit 2) via `commands::reject_csv` for
//!    all three — only `query` supports CSV.

use std::io::Read as _;
use std::path::Path;

use comfy_table::Table;
use datagov_core::DatagovError;
use datagov_core::report::{Input, ReportBuilder};
use datagov_sql::SqlParseResult;

use crate::cli::OutputFormat;
use crate::commands::reject_csv;

fn read_sql_input(path: &str) -> Result<String, DatagovError> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| DatagovError::internal(format!("failed to read stdin: {e}")))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(|e| {
            DatagovError::input_not_found(
                format!("input not found: {path} ({e})"),
                format!("check that {path} exists and is readable"),
            )
        })
    }
}

fn content_hash_for(path: &str) -> Option<String> {
    if path == "-" {
        None
    } else {
        datagov_core::report::content_hash_sha256(Path::new(path)).ok()
    }
}

fn input_for(path: &str) -> Input {
    Input {
        uri: path.to_string(),
        format: "sql".to_string(),
        content_hash: content_hash_for(path),
    }
}

pub fn run_parse(
    path: &str,
    dialect: Option<&str>,
    output: OutputFormat,
) -> Result<(), DatagovError> {
    reject_csv(output, "sql parse")?;

    let dialect_name = dialect.unwrap_or(datagov_sql::DEFAULT_DIALECT);
    let sql = read_sql_input(path)?;
    let input = input_for(path);

    let result = datagov_sql::parse(&sql, dialect_name)?;

    match output {
        OutputFormat::Table => render_parse_table(&result),
        OutputFormat::Json => {
            let payload = serde_json::to_value(&result).map_err(|e| {
                DatagovError::internal(format!("failed to render sql parse result: {e}"))
            })?;
            let report = ReportBuilder::new()
                .input(input)
                .extension("sql_parse", payload)
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

pub fn run_format(
    path: &str,
    dialect: Option<&str>,
    write: bool,
    output: OutputFormat,
) -> Result<(), DatagovError> {
    reject_csv(output, "sql format")?;

    if write && path == "-" {
        return Err(DatagovError::invalid_args(
            "'sql format --write' requires a real file path",
            "stdin ('-') has no source file to modify; omit --write or pass a file path",
        ));
    }

    let dialect_name = dialect.unwrap_or(datagov_sql::DEFAULT_DIALECT);
    let sql = read_sql_input(path)?;
    // Computed before any write, so it always describes the SQL that was
    // formatted, never the just-rewritten output.
    let input = input_for(path);

    let formatted = datagov_sql::format(&sql, dialect_name)?;

    if write {
        std::fs::write(path, &formatted).map_err(|e| {
            DatagovError::internal(format!("failed to write formatted SQL to {path}: {e}"))
        })?;
    }

    match output {
        OutputFormat::Table => println!("{formatted}"),
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "dialect": dialect_name,
                "output_sql": formatted,
                "written": write,
            });
            let report = ReportBuilder::new()
                .input(input)
                .extension("sql_format", payload)
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

pub fn run_transpile(
    path: &str,
    from: &str,
    to: &str,
    output: OutputFormat,
) -> Result<(), DatagovError> {
    reject_csv(output, "sql transpile")?;

    let sql = read_sql_input(path)?;
    let input = input_for(path);

    let result = datagov_sql::transpile(&sql, from, to)?;

    match output {
        OutputFormat::Table => {
            println!("{}", result.output_sql);
            for warning in &result.warnings {
                eprintln!("warning: [{}] {}", warning.construct, warning.message);
            }
        }
        OutputFormat::Json => {
            let payload = serde_json::to_value(&result).map_err(|e| {
                DatagovError::internal(format!("failed to render sql transpile result: {e}"))
            })?;
            let report = ReportBuilder::new()
                .input(input)
                .extension("sql_transpile", payload)
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Csv => unreachable!("reject_csv already returned an error"),
    }

    Ok(())
}

fn render_parse_table(result: &SqlParseResult) {
    println!("dialect:        {}", result.dialect);
    println!("statement_type: {}", result.statement_type);

    if !result.tables.is_empty() {
        let mut table = Table::new();
        table.set_header(vec!["table", "alias"]);
        for t in &result.tables {
            table.add_row(vec![t.name.clone(), t.alias.clone().unwrap_or_default()]);
        }
        println!("{table}");
    }

    if !result.columns.is_empty() {
        println!("columns:  {}", result.columns.join(", "));
    }

    if !result.joins.is_empty() {
        let mut table = Table::new();
        table.set_header(vec!["kind", "table", "on"]);
        for j in &result.joins {
            table.add_row(vec![
                j.kind.clone(),
                j.table.clone(),
                j.on.clone().unwrap_or_default(),
            ]);
        }
        println!("{table}");
    }

    if !result.filters.is_empty() {
        println!("filters:  {}", result.filters.join(" AND "));
    }
    if !result.group_by.is_empty() {
        println!("group by: {}", result.group_by.join(", "));
    }
    if !result.order_by.is_empty() {
        println!("order by: {}", result.order_by.join(", "));
    }
    if !result.ctes.is_empty() {
        println!("ctes:     {}", result.ctes.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_for_stdin_is_none() {
        assert!(content_hash_for("-").is_none());
    }
}

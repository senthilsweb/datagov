//! lib.rs — Crate root for `datagov-sql`: `parse` / `format` / `transpile`
//! over the 11 priority dialects (PRD §10.4–§10.6), backed by
//! `sqlglot-rust` (locked at the inception gate; verified viable in the
//! Bolt 4 Phase 1 spike — see `datagov-sql`'s module docs and the Bolt 4
//! report for what was checked).
//!
//! 1. Declares `dialect` (priority-dialect name resolution), `result`
//!    (`SqlParseResult` + its nested `TableRef`/`JoinRef` and the
//!    AST-projection logic), and `warnings` (`TranspileWarning` +
//!    lossy-construct detection) — re-exports the public types at the
//!    crate root.
//! 2. `parse(sql, dialect) -> Result<SqlParseResult, DatagovError>`:
//!    resolves the dialect name, parses, and projects the result via
//!    `result::build`.
//! 3. `format(sql, dialect) -> Result<String, DatagovError>`: resolves,
//!    parses, and renders pretty-printed SQL via
//!    `sqlglot_rust::generate_pretty`.
//! 4. `transpile(sql, from, to) -> Result<SqlTranspileResult, DatagovError>`:
//!    resolves both dialects, parses once under `from` purely to run
//!    `warnings::detect` against the pre-transform source AST (see that
//!    module's header for why this crate needs its own lossy-construct
//!    detection layer), then renders the output via
//!    `sqlglot_rust::transpile` itself — **not** a bare `generate` call
//!    on the already-parsed AST. See the code comment on `transpile`
//!    for why: the crate's own `transpile` runs an extra
//!    `dialects::transform` pass (e.g. `TOP n` <-> `LIMIT n` <->
//!    `FETCH FIRST n ROWS ONLY`) that plain `generate` skips — a real
//!    bug caught by this bolt's own stdin integration test, fixed
//!    before landing (see the Bolt 4 report).
//! 5. Error categorization (kept consistent with Bolt 3's `query`):
//!    an unrecognized dialect name is `DatagovError::UnsupportedInput`
//!    (exit 4); malformed/unparseable SQL is `DatagovError::InvalidArgs`
//!    (exit 2).

mod dialect;
mod result;
mod warnings;

pub use dialect::DEFAULT_DIALECT;
pub use result::{JoinRef, SqlParseResult, TableRef};
pub use warnings::TranspileWarning;

use datagov_core::DatagovError;
use serde::Serialize;

/// The `datagov sql transpile` result payload, serialized into
/// `extensions.sql_transpile` by the CLI.
#[derive(Debug, Clone, Serialize)]
pub struct SqlTranspileResult {
    pub from_dialect: String,
    pub to_dialect: String,
    pub output_sql: String,
    /// Lossy/unsupported constructs identified by `warnings::detect`.
    /// Always present (possibly empty) — never omitted, since the whole
    /// point is that these are never silent.
    pub warnings: Vec<TranspileWarning>,
}

fn parse_or_invalid_args(
    sql: &str,
    dialect: sqlglot_rust::Dialect,
) -> Result<sqlglot_rust::Statement, DatagovError> {
    sqlglot_rust::parse(sql, dialect).map_err(|source| {
        DatagovError::invalid_args(
            format!("invalid SQL: {source}"),
            "check the query syntax".to_string(),
        )
    })
}

/// Parse `sql` under `dialect_name` and project the result into a
/// `SqlParseResult`. `dialect_name` is resolved case-insensitively
/// against the 11 priority dialects (`crate::dialect::resolve`); an
/// unrecognized name is `UnsupportedInput` (exit 4). Malformed SQL is
/// `InvalidArgs` (exit 2).
pub fn parse(sql: &str, dialect_name: &str) -> Result<SqlParseResult, DatagovError> {
    let (label, dialect) = dialect::resolve(dialect_name)?;
    let stmt = parse_or_invalid_args(sql, dialect)?;
    result::build(&stmt, label)
}

/// Parse `sql` under `dialect_name` and render pretty-printed SQL back
/// in the same dialect. Same error categorization as `parse`.
pub fn format(sql: &str, dialect_name: &str) -> Result<String, DatagovError> {
    let (_, dialect) = dialect::resolve(dialect_name)?;
    let stmt = parse_or_invalid_args(sql, dialect)?;
    Ok(sqlglot_rust::generate_pretty(&stmt, dialect))
}

/// Transpile `sql` from `from_name` to `to_name`. Both dialect names are
/// resolved against the 11 priority dialects; either being unrecognized
/// is `UnsupportedInput` (exit 4). Malformed source SQL, or SQL the
/// crate's own transpile pipeline rejects for the target dialect (e.g.
/// an `ARRAY` literal transpiled into T-SQL — see the code comment
/// below), is `InvalidArgs` (exit 2). `warnings` is never omitted, even
/// when empty — see `crate::warnings` for what is (and is not yet)
/// detected.
pub fn transpile(
    sql: &str,
    from_name: &str,
    to_name: &str,
) -> Result<SqlTranspileResult, DatagovError> {
    let (from_label, from_dialect) = dialect::resolve(from_name)?;
    let (to_label, to_dialect) = dialect::resolve(to_name)?;

    // Parsed once, purely to run `warnings::detect` against the
    // pre-transform source AST (as the source author actually wrote
    // it).
    let stmt = parse_or_invalid_args(sql, from_dialect)?;
    let detected = warnings::detect(&stmt, to_dialect);

    // **Deliberately not `generate(&stmt, to_dialect)`.** Verified in
    // this bolt's own stdin integration test (see the Bolt 4 report):
    // `sqlglot_rust::transpile` runs `parse -> dialects::transform(from,
    // to) -> a private unsupported-construct validation -> generate`,
    // and `dialects::transform` is what actually rewrites `TOP n` <->
    // `LIMIT n` <-> `FETCH FIRST n ROWS ONLY`, `ILIKE` -> `LIKE(LOWER
    // (...))` for non-ILIKE dialects, etc. Calling bare `generate` on
    // the untransformed AST — this crate's first implementation —
    // skipped that rewrite entirely (T-SQL `TOP 10` transpiled to
    // Postgres came out as literal `TOP 10`, invalid Postgres syntax).
    // Re-parsing here (rather than reusing `stmt`) is a small, accepted
    // cost for calling the crate's real, tested entry point instead of
    // re-deriving its internal pipeline by hand.
    let output_sql = sqlglot_rust::transpile(sql, from_dialect, to_dialect).map_err(|source| {
        DatagovError::invalid_args(
            format!("failed to transpile SQL: {source}"),
            "check the query syntax, or that the source SQL is representable in the target dialect"
                .to_string(),
        )
    })?;

    Ok(SqlTranspileResult {
        from_dialect: from_label.to_string(),
        to_dialect: to_label.to_string(),
        output_sql,
        warnings: detected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reports_dialect_statement_type_and_tables() {
        let result = parse("SELECT a, b FROM t WHERE a > 1", "ansi").unwrap();
        assert_eq!(result.dialect, "ansi");
        assert_eq!(result.statement_type, "select");
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name, "t");
        assert_eq!(result.columns, vec!["a", "b"]);
        assert_eq!(result.filters, vec!["a > 1"]);
    }

    #[test]
    fn parse_unrecognized_dialect_is_unsupported_input() {
        let err = parse("SELECT 1", "not_a_dialect").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }

    #[test]
    fn parse_malformed_sql_is_invalid_args() {
        let err = parse("SELECT FROM WHERE", "ansi").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }

    #[test]
    fn format_pretty_prints_without_mutating_semantics() {
        let out = format("SELECT a,b FROM t WHERE a>1", "ansi").unwrap();
        assert!(out.contains("SELECT"));
        assert!(out.contains('\n'), "pretty output should be multi-line");
    }

    #[test]
    fn transpile_spark_to_duckdb_round_trips_a_join_group_by() {
        let sql = "SELECT t.state, COUNT(*) AS total FROM customers t INNER JOIN orders o ON t.id = o.customer_id GROUP BY t.state ORDER BY t.state";
        let result = transpile(sql, "spark", "duckdb").unwrap();
        assert_eq!(result.from_dialect, "spark");
        assert_eq!(result.to_dialect, "duckdb");
        assert!(result.output_sql.contains("GROUP BY"));
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn transpile_lossy_qualify_into_ansi_produces_a_warning() {
        let sql = "SELECT id, ROW_NUMBER() OVER (PARTITION BY state ORDER BY id) AS rn FROM customers QUALIFY rn = 1";
        let result = transpile(sql, "snowflake", "ansi").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].construct, "QUALIFY");
    }

    #[test]
    fn transpile_applies_the_crates_dialect_transform_not_just_generate() {
        // Regression test for a real bug found while building this
        // bolt: an earlier implementation called bare `generate(&stmt,
        // to_dialect)` on the untransformed AST, which does not rewrite
        // dialect-specific row-limit syntax — T-SQL `TOP 10` transpiled
        // to Postgres came out as literal (invalid) `TOP 10` instead of
        // `LIMIT 10`. `sqlglot_rust::transpile` runs an additional
        // `dialects::transform` pass that `generate` alone does not.
        let result = transpile("SELECT TOP 10 a FROM t", "tsql", "postgres").unwrap();
        assert_eq!(result.output_sql, "SELECT a FROM t LIMIT 10");
    }

    #[test]
    fn transpile_unrecognized_dialect_is_unsupported_input() {
        let err = transpile("SELECT 1", "foo", "bar").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }

    #[test]
    fn transpile_malformed_sql_is_invalid_args() {
        let err = transpile("SELECT FROM WHERE", "ansi", "postgres").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }
}

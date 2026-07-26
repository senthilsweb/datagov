//! warnings.rs — Lossy/unsupported-construct detection for `datagov sql
//! transpile`.
//!
//! **Why this module exists (Phase 1 finding):** `sqlglot_rust`'s
//! `transpile`/`generate` are pure syntax transformers — they carry a
//! dialect-specific *clause* (e.g. Snowflake `QUALIFY`, T-SQL/Snowflake/
//! BigQuery `PIVOT`) through to any target dialect's output unchanged,
//! without checking whether that target dialect's real engine actually
//! supports the clause. Verified empirically during the Bolt 4 spike:
//! `transpile("... QUALIFY rn = 1", Dialect::Snowflake, Dialect::Ansi)`
//! returns `Ok` with `QUALIFY` still in the ANSI output — syntax ANSI
//! engines reject. The crate's only built-in warnings API
//! (`dialects::time::format_time_with_warnings`) is scoped to date/time
//! format-string conversion, not general transpilation. So this module
//! is `datagov-sql`'s own explicitly-documented capability matrix,
//! covering the constructs this crate's conformance corpus actually
//! exercises — **not** a claim of exhaustive dialect-compatibility
//! knowledge. Extending it (new constructs, more precise per-dialect
//! support) is expected as the corpus grows.
//!
//! 1. `detect` inspects the *parsed source* statement (before
//!    generation) for constructs known to be dialect-specific, and
//!    flags each one whose home construct isn't supported by `to`.
//! 2. Two constructs are covered for this bolt: `QUALIFY` (real support:
//!    Snowflake, BigQuery, DuckDB, Databricks) and `PIVOT`/`UNPIVOT`
//!    (real support: T-SQL, Snowflake, BigQuery, Spark, Databricks,
//!    DuckDB). Both are checked only at the top level of a `SELECT`
//!    statement (not recursively inside subqueries/CTEs) — sufficient
//!    for this bolt's corpus, and documented as a scope boundary rather
//!    than silently narrowed.
//! 3. `TOP n` / `LIMIT` / `FETCH FIRST` are deliberately **not**
//!    flagged: `sqlglot_rust::generate` genuinely rewrites between them
//!    (verified: T-SQL `TOP 10` -> ANSI `LIMIT 10`), so carrying that
//!    construct across dialects is not lossy.

use sqlglot_rust::Dialect;
use sqlglot_rust::Statement;

/// One explicit, never-silent warning about a construct that may not
/// carry its full semantics into the target dialect.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranspileWarning {
    pub construct: String,
    pub message: String,
}

fn qualify_supported(dialect: Dialect) -> bool {
    matches!(
        dialect,
        Dialect::Snowflake | Dialect::BigQuery | Dialect::DuckDb | Dialect::Databricks
    )
}

fn pivot_supported(dialect: Dialect) -> bool {
    matches!(
        dialect,
        Dialect::Tsql
            | Dialect::Snowflake
            | Dialect::BigQuery
            | Dialect::Spark
            | Dialect::Databricks
            | Dialect::DuckDb
    )
}

/// Inspect `stmt` (as parsed in the source dialect) for constructs this
/// module knows are dialect-specific, and return one warning per
/// construct not supported by `to`. Never panics; returns an empty
/// `Vec` for anything outside a top-level `SELECT` or with none of the
/// known constructs present.
pub fn detect(stmt: &Statement, to: Dialect) -> Vec<TranspileWarning> {
    let mut warnings = Vec::new();

    let Statement::Select(select) = stmt else {
        return warnings;
    };

    if select.qualify.is_some() && !qualify_supported(to) {
        warnings.push(TranspileWarning {
            construct: "QUALIFY".to_string(),
            message: format!(
                "the QUALIFY clause was carried into the output unchanged, but {to} does not \
                 natively support QUALIFY — the generated SQL may not execute as-is on {to}"
            ),
        });
    }

    let from_source = select.from.as_ref().map(|f| &f.source);
    if has_pivot_or_unpivot(from_source) && !pivot_supported(to) {
        warnings.push(TranspileWarning {
            construct: "PIVOT/UNPIVOT".to_string(),
            message: format!(
                "a PIVOT/UNPIVOT clause was carried into the output unchanged, but {to} does not \
                 natively support PIVOT/UNPIVOT — the generated SQL may not execute as-is on {to}"
            ),
        });
    }

    warnings
}

fn has_pivot_or_unpivot(source: Option<&sqlglot_rust::ast::TableSource>) -> bool {
    use sqlglot_rust::ast::TableSource;
    match source {
        Some(TableSource::Pivot { .. }) | Some(TableSource::Unpivot { .. }) => true,
        Some(TableSource::Lateral { source }) => has_pivot_or_unpivot(Some(source.as_ref())),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlglot_rust::parse;

    #[test]
    fn qualify_into_unsupporting_dialect_warns() {
        let stmt = parse(
            "SELECT a, ROW_NUMBER() OVER (PARTITION BY a ORDER BY b) AS rn FROM t QUALIFY rn = 1",
            Dialect::Snowflake,
        )
        .unwrap();
        let warnings = detect(&stmt, Dialect::Ansi);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].construct, "QUALIFY");
    }

    #[test]
    fn qualify_into_supporting_dialect_is_silent() {
        let stmt = parse(
            "SELECT a, ROW_NUMBER() OVER (PARTITION BY a ORDER BY b) AS rn FROM t QUALIFY rn = 1",
            Dialect::Snowflake,
        )
        .unwrap();
        let warnings = detect(&stmt, Dialect::BigQuery);
        assert!(warnings.is_empty());
    }

    #[test]
    fn pivot_into_unsupporting_dialect_warns() {
        let stmt = parse(
            "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2')) AS pvt",
            Dialect::Tsql,
        )
        .unwrap();
        let warnings = detect(&stmt, Dialect::Postgres);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].construct, "PIVOT/UNPIVOT");
    }

    #[test]
    fn pivot_into_supporting_dialect_is_silent() {
        let stmt = parse(
            "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2')) AS pvt",
            Dialect::Tsql,
        )
        .unwrap();
        let warnings = detect(&stmt, Dialect::DuckDb);
        assert!(warnings.is_empty());
    }

    #[test]
    fn plain_select_has_no_warnings() {
        let stmt = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let warnings = detect(&stmt, Dialect::Tsql);
        assert!(warnings.is_empty());
    }

    #[test]
    fn top_n_is_not_flagged_as_lossy() {
        // TOP -> LIMIT is a genuine rewrite, not a lossy pass-through
        // (verified in the Phase 1 spike) — no construct in this
        // statement should ever warn.
        let stmt = parse("SELECT TOP 10 a FROM t", Dialect::Tsql).unwrap();
        let warnings = detect(&stmt, Dialect::Ansi);
        assert!(warnings.is_empty());
    }
}

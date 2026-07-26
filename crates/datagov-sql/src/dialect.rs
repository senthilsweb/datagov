//! dialect.rs — Priority-dialect name resolution for `datagov-sql`.
//!
//! 1. `PRIORITY_DIALECTS`, the 11 dialect names PRD §10.6 / the Bolt 4
//!    brief scope this crate to (ANSI, PostgreSQL, DuckDB, Spark,
//!    Databricks, Snowflake, BigQuery, Trino, MySQL, SQLite, T-SQL), each
//!    paired with its canonical lowercase label (used verbatim in
//!    `SqlParseResult::dialect` / `SqlTranspileResult::{from_dialect,
//!    to_dialect}`) and its `sqlglot_rust::Dialect` value.
//! 2. `resolve` maps a user-supplied dialect name (case-insensitive, a
//!    handful of common aliases: `postgresql`->postgres, `mssql`/
//!    `sqlserver`->tsql) to a `(canonical_label, Dialect)` pair.
//!    **Deliberately does not delegate to `sqlglot_rust::Dialect::
//!    from_str`**: that crate recognizes 30 dialects (19 more than our
//!    11-dialect priority list — Athena, ClickHouse, Hive, Oracle,
//!    Presto, Redshift, StarRocks, and 12 "Community" tier dialects), and
//!    the brief scopes `datagov sql` to exactly the priority 11. A name
//!    outside that list — whether or not the underlying crate would
//!    itself accept it — is `DatagovError::UnsupportedInput` (exit 4),
//!    per the brief ("An unrecognized dialect name (not one of the 11
//!    priority dialects, or a name the underlying crate itself
//!    rejects)").
//! 3. No `--dialect` given anywhere in the CLI defaults to `"ansi"`,
//!    resolved through the same table.

use datagov_core::DatagovError;
use sqlglot_rust::Dialect;

/// One entry per priority dialect: the canonical label surfaced in
/// envelopes, the accepted case-insensitive input aliases, and the
/// underlying crate's `Dialect` value.
struct DialectEntry {
    canonical: &'static str,
    aliases: &'static [&'static str],
    dialect: Dialect,
}

const PRIORITY_DIALECTS: &[DialectEntry] = &[
    DialectEntry {
        canonical: "ansi",
        aliases: &["ansi"],
        dialect: Dialect::Ansi,
    },
    DialectEntry {
        canonical: "postgres",
        aliases: &["postgres", "postgresql"],
        dialect: Dialect::Postgres,
    },
    DialectEntry {
        canonical: "duckdb",
        aliases: &["duckdb"],
        dialect: Dialect::DuckDb,
    },
    DialectEntry {
        canonical: "spark",
        aliases: &["spark"],
        dialect: Dialect::Spark,
    },
    DialectEntry {
        canonical: "databricks",
        aliases: &["databricks"],
        dialect: Dialect::Databricks,
    },
    DialectEntry {
        canonical: "snowflake",
        aliases: &["snowflake"],
        dialect: Dialect::Snowflake,
    },
    DialectEntry {
        canonical: "bigquery",
        aliases: &["bigquery"],
        dialect: Dialect::BigQuery,
    },
    DialectEntry {
        canonical: "trino",
        aliases: &["trino"],
        dialect: Dialect::Trino,
    },
    DialectEntry {
        canonical: "mysql",
        aliases: &["mysql"],
        dialect: Dialect::Mysql,
    },
    DialectEntry {
        canonical: "sqlite",
        aliases: &["sqlite"],
        dialect: Dialect::Sqlite,
    },
    DialectEntry {
        canonical: "tsql",
        aliases: &["tsql", "mssql", "sqlserver"],
        dialect: Dialect::Tsql,
    },
];

/// The default dialect name when `--dialect` is not given.
pub const DEFAULT_DIALECT: &str = "ansi";

/// Resolve a user-supplied dialect name to its canonical label and
/// `sqlglot_rust::Dialect`. Case-insensitive; a name outside the 11
/// priority dialects (or their aliases) is `UnsupportedInput` (exit 4).
pub fn resolve(name: &str) -> Result<(&'static str, Dialect), DatagovError> {
    let normalized = name.trim().to_ascii_lowercase();
    PRIORITY_DIALECTS
        .iter()
        .find(|entry| entry.aliases.contains(&normalized.as_str()))
        .map(|entry| (entry.canonical, entry.dialect))
        .ok_or_else(|| {
            let supported: Vec<&str> = PRIORITY_DIALECTS.iter().map(|e| e.canonical).collect();
            DatagovError::unsupported_input(
                format!("unsupported SQL dialect: '{name}'"),
                format!(
                    "use one of the supported dialects: {}",
                    supported.join(", ")
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_every_priority_dialect_by_canonical_name() {
        for entry in PRIORITY_DIALECTS {
            let (label, _) = resolve(entry.canonical).unwrap();
            assert_eq!(label, entry.canonical);
        }
    }

    #[test]
    fn resolves_case_insensitively() {
        let (label, _) = resolve("PostgreSQL").unwrap();
        assert_eq!(label, "postgres");
    }

    #[test]
    fn resolves_known_aliases() {
        assert_eq!(resolve("postgresql").unwrap().0, "postgres");
        assert_eq!(resolve("mssql").unwrap().0, "tsql");
        assert_eq!(resolve("sqlserver").unwrap().0, "tsql");
    }

    #[test]
    fn unrecognized_name_is_unsupported_input() {
        let err = resolve("not_a_real_dialect").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }

    #[test]
    fn dialect_the_underlying_crate_knows_but_is_outside_the_priority_list_is_rejected() {
        // `oracle` is a real sqlglot-rust `Dialect` variant (the crate
        // supports 30 dialects total) but not one of our 11 priority
        // dialects, so it must still be rejected.
        let err = resolve("oracle").unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }
}

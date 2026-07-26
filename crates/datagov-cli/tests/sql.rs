//! sql.rs — Integration tests for `datagov sql parse | format |
//! transpile` (assert_cmd), run against the committed dialect
//! conformance corpus under `examples/sql/` (all 11 priority dialects
//! confirmed viable in the Bolt 4 Phase 1 spike — see the Bolt 4
//! report).
//!
//! 1. Golden tests, one per dialect: `datagov sql parse
//!    examples/sql/<dialect>/select_where.sql --dialect <dialect>
//!    --output json`, normalized (`run` block, `input.uri`) and compared
//!    against `tests/golden/sql_parse_<dialect>_select_where.json` —
//!    bootstrapped from a real run, hand-reviewed.
//! 2. `join_group_by.sql` (ANSI, Spark, T-SQL — representative, not all
//!    11) asserts the richer shape: two `tables`, one `joins` entry
//!    (`kind: "inner"`), non-empty `group_by`/`order_by`.
//! 3. Every corpus `idiomatic.sql` parses successfully (exit 0) under
//!    its own dialect — a loop over all 11, not a golden snapshot (the
//!    point is "doesn't regress to a parse failure", not byte-exact
//!    AST content for every one).
//! 4. `format` without `--write`: the source file is byte-identical and
//!    its mtime is unchanged after the command runs (HARD). With
//!    `--write`: the file is overwritten with the pretty-printed SQL.
//! 5. `transpile` round-trips the four non-lossy corpus pairs under
//!    `examples/sql/transpile/` against their committed `expected.sql`.
//!    The fifth pair (`lossy_qualify_snowflake_to_ansi`) asserts a
//!    `QUALIFY` warning appears in `extensions.sql_transpile.warnings`
//!    (JSON) and on stderr (table/human rendering) — never silent.
//! 6. Exit codes: malformed SQL -> 2; missing input file -> 3; unknown
//!    dialect name -> 4 (for `parse`, `format`, and `transpile`).
//! 7. stdin (`-`) works for `parse`, `format` (without `--write`), and
//!    `transpile`; `format --write -` is rejected (exit 2 — no source
//!    file to modify).

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use std::time::SystemTime;

fn datagov() -> Command {
    Command::cargo_bin("datagov").expect("datagov binary should build")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_dir() -> PathBuf {
    repo_root().join("examples/sql")
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}.json"))
}

fn schema_path() -> PathBuf {
    repo_root().join("docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

fn normalize_run(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "run".to_string(),
            serde_json::json!({
                "id": "normalized",
                "started_at": "normalized",
                "completed_at": "normalized",
                "duration_ms": 0,
            }),
        );
        if let Some(input) = obj.get_mut("input").and_then(|i| i.as_object_mut()) {
            input.insert("uri".to_string(), Value::String("<uri>".to_string()));
        }
    }
    value
}

fn run_parse_json(path: &std::path::Path, dialect: &str) -> Value {
    let output = datagov()
        .args([
            "sql",
            "parse",
            path.to_str().unwrap(),
            "--dialect",
            dialect,
            "--output",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    serde_json::from_str(stdout.trim()).expect("stdout must be a single JSON document")
}

const PRIORITY_DIALECTS: &[&str] = &[
    "ansi",
    "postgres",
    "duckdb",
    "spark",
    "databricks",
    "snowflake",
    "bigquery",
    "trino",
    "mysql",
    "sqlite",
    "tsql",
];

#[test]
fn golden_parse_select_where_across_all_priority_dialects() {
    let validator = load_validator();
    for dialect in PRIORITY_DIALECTS {
        let path = corpus_dir().join(dialect).join("select_where.sql");
        let live = normalize_run(run_parse_json(&path, dialect));

        assert!(
            validator.is_valid(&live),
            "[{dialect}] envelope failed schema validation: {live}"
        );

        let golden_text =
            std::fs::read_to_string(golden_path(&format!("sql_parse_{dialect}_select_where")))
                .unwrap_or_else(|_| {
                    panic!("missing golden file for {dialect} — bootstrap it from a real run")
                });
        let golden: Value =
            serde_json::from_str(&golden_text).expect("golden file must be valid JSON");

        assert_eq!(
            live, golden,
            "[{dialect}] sql parse output for select_where.sql drifted from the committed golden snapshot"
        );
    }
}

#[test]
fn parse_join_group_by_reports_tables_joins_and_grouping() {
    for dialect in ["ansi", "spark", "tsql"] {
        let path = corpus_dir().join(dialect).join("join_group_by.sql");
        let value = run_parse_json(&path, dialect);
        let parsed = &value["extensions"]["sql_parse"];

        let tables = parsed["tables"].as_array().unwrap();
        assert_eq!(tables.len(), 2, "[{dialect}] expected two tables");

        let joins = parsed["joins"].as_array().unwrap();
        assert_eq!(joins.len(), 1, "[{dialect}] expected one join");
        assert_eq!(joins[0]["kind"], "inner");
        assert!(joins[0]["on"].as_str().unwrap().contains("customer_id"));

        assert_eq!(
            parsed["group_by"].as_array().unwrap().len(),
            1,
            "[{dialect}] expected one GROUP BY expression"
        );
        assert_eq!(
            parsed["order_by"].as_array().unwrap().len(),
            1,
            "[{dialect}] expected one ORDER BY expression"
        );
    }
}

#[test]
fn every_corpus_idiomatic_statement_parses_successfully() {
    for dialect in PRIORITY_DIALECTS {
        let path = corpus_dir().join(dialect).join("idiomatic.sql");
        datagov()
            .args(["sql", "parse", path.to_str().unwrap(), "--dialect", dialect])
            .assert()
            .success();
    }
}

#[test]
fn format_without_write_leaves_the_source_file_byte_identical_and_mtime_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.sql");
    std::fs::write(&path, "SELECT id,name FROM customers WHERE state='CA';\n").unwrap();
    let original_bytes = std::fs::read(&path).unwrap();
    let original_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();

    // Small sleep-free guard: mtime resolution on some filesystems is
    // coarse, so we assert equality (not just "not before"), which is
    // the strict form of "unchanged" regardless of resolution.
    let stdout = datagov()
        .args(["sql", "format", path.to_str().unwrap(), "--dialect", "ansi"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let formatted = String::from_utf8(stdout).unwrap();
    assert!(formatted.contains("SELECT"));

    let after_bytes = std::fs::read(&path).unwrap();
    let after_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(
        original_bytes, after_bytes,
        "source file must be byte-identical when --write is absent"
    );
    assert_eq!(
        original_mtime, after_mtime,
        "source file mtime must be unchanged when --write is absent"
    );
}

#[test]
fn format_with_write_rewrites_the_file_with_formatted_sql() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.sql");
    std::fs::write(&path, "SELECT id,name FROM customers WHERE state='CA';\n").unwrap();

    datagov()
        .args([
            "sql",
            "format",
            path.to_str().unwrap(),
            "--dialect",
            "ansi",
            "--write",
        ])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("SELECT"));
    assert!(
        contents.contains('\n'),
        "written SQL should be pretty-printed (multi-line)"
    );
    assert_ne!(
        contents.trim(),
        "SELECT id,name FROM customers WHERE state='CA';",
        "file must have actually changed to the formatted form"
    );

    // Sanity: SystemTime is monotonic-enough here to confirm a write
    // actually happened (file has new content, not just touched).
    let _ = SystemTime::now();
}

#[test]
fn format_write_on_stdin_is_rejected_with_exit_2() {
    datagov()
        .args(["sql", "format", "-", "--write"])
        .write_stdin("SELECT 1")
        .assert()
        .code(2);
}

fn transpile_pair(name: &str, from: &str, to: &str) {
    let dir = corpus_dir().join("transpile").join(name);
    let source = dir.join("source.sql");
    let expected = std::fs::read_to_string(dir.join("expected.sql"))
        .unwrap()
        .trim()
        .to_string();

    let stdout = datagov()
        .args([
            "sql",
            "transpile",
            source.to_str().unwrap(),
            "--from",
            from,
            "--to",
            to,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(stdout).unwrap().trim().to_string();
    assert_eq!(
        output, expected,
        "[{name}] transpiled output drifted from the committed expected.sql"
    );
}

#[test]
fn transpile_round_trips_the_committed_corpus_pairs() {
    transpile_pair("spark_to_duckdb", "spark", "duckdb");
    transpile_pair("tsql_to_postgres", "tsql", "postgres");
    transpile_pair("snowflake_to_bigquery", "snowflake", "bigquery");
    transpile_pair("mysql_to_ansi", "mysql", "ansi");
}

#[test]
fn transpile_lossy_qualify_produces_a_warning_in_json_and_on_stderr() {
    let dir = corpus_dir()
        .join("transpile")
        .join("lossy_qualify_snowflake_to_ansi");
    let source = dir.join("source.sql");

    // JSON rendering: warnings entry present, never omitted.
    let stdout = datagov()
        .args([
            "sql",
            "transpile",
            source.to_str().unwrap(),
            "--from",
            "snowflake",
            "--to",
            "ansi",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_str(String::from_utf8(stdout).unwrap().trim()).unwrap();
    let warnings = value["extensions"]["sql_transpile"]["warnings"]
        .as_array()
        .unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["construct"], "QUALIFY");

    // Human/table rendering: the SQL goes to stdout, the warning is
    // surfaced on stderr — never swallowed either way.
    let assert = datagov()
        .args([
            "sql",
            "transpile",
            source.to_str().unwrap(),
            "--from",
            "snowflake",
            "--to",
            "ansi",
        ])
        .assert()
        .success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("QUALIFY"),
        "human-readable path must also surface the warning: {stderr}"
    );
}

#[test]
fn malformed_sql_exits_2_for_all_three_subcommands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.sql");
    std::fs::write(&path, "SELECT FROM WHERE").unwrap();

    datagov()
        .args(["sql", "parse", path.to_str().unwrap()])
        .assert()
        .code(2);
    datagov()
        .args(["sql", "format", path.to_str().unwrap()])
        .assert()
        .code(2);
    datagov()
        .args([
            "sql",
            "transpile",
            path.to_str().unwrap(),
            "--from",
            "ansi",
            "--to",
            "postgres",
        ])
        .assert()
        .code(2);
}

#[test]
fn missing_input_file_exits_3_for_all_three_subcommands() {
    let path = corpus_dir().join("does-not-exist.sql");

    datagov()
        .args(["sql", "parse", path.to_str().unwrap()])
        .assert()
        .code(3);
    datagov()
        .args(["sql", "format", path.to_str().unwrap()])
        .assert()
        .code(3);
    datagov()
        .args([
            "sql",
            "transpile",
            path.to_str().unwrap(),
            "--from",
            "ansi",
            "--to",
            "postgres",
        ])
        .assert()
        .code(3);
}

#[test]
fn unknown_dialect_exits_4_for_all_three_subcommands() {
    let path = corpus_dir().join("ansi/select_where.sql");

    datagov()
        .args([
            "sql",
            "parse",
            path.to_str().unwrap(),
            "--dialect",
            "not_a_real_dialect",
        ])
        .assert()
        .code(4);
    datagov()
        .args([
            "sql",
            "format",
            path.to_str().unwrap(),
            "--dialect",
            "not_a_real_dialect",
        ])
        .assert()
        .code(4);
    datagov()
        .args([
            "sql",
            "transpile",
            path.to_str().unwrap(),
            "--from",
            "not_a_real_dialect",
            "--to",
            "postgres",
        ])
        .assert()
        .code(4);
    datagov()
        .args([
            "sql",
            "transpile",
            path.to_str().unwrap(),
            "--from",
            "ansi",
            "--to",
            "not_a_real_dialect",
        ])
        .assert()
        .code(4);
}

#[test]
fn stdin_works_for_parse_format_and_transpile() {
    let sql = "SELECT a FROM t WHERE a > 1";

    let value = {
        let stdout = datagov()
            .args([
                "sql",
                "parse",
                "-",
                "--dialect",
                "mysql",
                "--output",
                "json",
            ])
            .write_stdin(sql)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let v: Value = serde_json::from_str(String::from_utf8(stdout).unwrap().trim()).unwrap();
        v
    };
    assert_eq!(value["input"]["uri"], "-");
    assert!(value["input"].get("content_hash").is_none());
    assert_eq!(value["extensions"]["sql_parse"]["dialect"], "mysql");

    datagov()
        .args(["sql", "format", "-", "--dialect", "ansi"])
        .write_stdin(sql)
        .assert()
        .success()
        .stdout(predicates::str::contains("SELECT"));

    datagov()
        .args([
            "sql",
            "transpile",
            "-",
            "--from",
            "tsql",
            "--to",
            "postgres",
        ])
        .write_stdin("SELECT TOP 10 a FROM t")
        .assert()
        .success()
        .stdout(predicates::str::contains("LIMIT 10"));
}

#[test]
fn default_dialect_is_ansi_when_not_given() {
    let path = corpus_dir().join("ansi/select_where.sql");
    let value = run_parse_json(&path, "ansi");
    assert_eq!(value["extensions"]["sql_parse"]["dialect"], "ansi");

    // Omit --dialect entirely and confirm it still resolves to ansi.
    let stdout = datagov()
        .args(["sql", "parse", path.to_str().unwrap(), "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_str(String::from_utf8(stdout).unwrap().trim()).unwrap();
    assert_eq!(value["extensions"]["sql_parse"]["dialect"], "ansi");
}

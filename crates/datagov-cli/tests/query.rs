//! query.rs — Integration tests for `datagov query` (assert_cmd), run
//! against the real committed `examples/customers.*` fixtures.
//!
//! 1. `SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet'
//!    GROUP BY state --output json` — correct aggregation,
//!    `extensions.query.sources == ["examples/customers.parquet"]`,
//!    `execution.duration_ms` present. Run with `current_dir` set to the
//!    repo root so the quoted relative path in the SQL text resolves the
//!    same way a user invoking this from the repo root would see it —
//!    and so `sources` reports that exact relative string.
//! 2. `--output csv` and `--output table` on the same query: CSV is bare
//!    (no envelope) and parses as valid CSV with the expected header;
//!    table renders the same rows.
//! 3. Bounded output: a query returning more than `DEFAULT_QUERY_LIMIT`
//!    rows is truncated with `truncated: true` and the correct
//!    `total_row_count`; `--limit` overrides it.
//! 4. Exit codes: a missing file in `FROM` -> 3; an unsupported
//!    extension (`examples/customers.json`, a JSON array) -> 4; invalid
//!    SQL syntax -> 2.
//! 5. `datagov inspect --output csv` -> exit 2 (rejected, not silently
//!    downgraded to table) — the counterpart to `query`'s own CSV
//!    support.

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn datagov() -> Command {
    let mut cmd = Command::cargo_bin("datagov").expect("datagov binary should build");
    cmd.current_dir(repo_root());
    cmd
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

fn run_query_json(sql: &str, extra: &[&str]) -> Value {
    let output = datagov()
        .args(["query", sql, "--output", "json"])
        .args(extra)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    serde_json::from_str(stdout.trim()).expect("stdout must be a single JSON document")
}

#[test]
fn aggregates_a_parquet_fixture_and_reports_sources_and_duration() {
    let value = run_query_json(
        "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state",
        &[],
    );

    let validator = load_validator();
    assert!(
        validator.is_valid(&value),
        "envelope failed schema validation: {value}"
    );
    assert!(value.get("input").is_none(), "query has no input section");

    let query = &value["extensions"]["query"];
    assert_eq!(
        query["sources"],
        serde_json::json!(["examples/customers.parquet"])
    );
    assert_eq!(query["columns"], serde_json::json!(["state", "total"]));
    assert!(query["execution"]["duration_ms"].is_u64());
    assert!(!query["rows"].as_array().unwrap().is_empty());

    // Cross-check one known aggregate: customers.parquet's "state"
    // column includes "AB" with a documented count of 332
    // (examples/README.md).
    let ab_total = query["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row[0] == "AB")
        .map(|row| row[1].clone());
    assert_eq!(ab_total, Some(serde_json::json!(332)));
}

#[test]
fn csv_output_is_bare_and_parses_with_expected_header() {
    let output = datagov()
        .args([
            "query",
            "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state ORDER BY state LIMIT 3",
            "--output",
            "csv",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    // Bare CSV: no envelope, so it must not parse as JSON.
    assert!(serde_json::from_str::<Value>(stdout.trim()).is_err());

    let mut reader = csv::Reader::from_reader(stdout.as_bytes());
    let headers = reader.headers().unwrap().clone();
    assert_eq!(headers, csv::StringRecord::from(vec!["state", "total"]));
    let rows: Vec<csv::StringRecord> = reader.records().map(|r| r.unwrap()).collect();
    assert_eq!(rows.len(), 3);
}

#[test]
fn table_output_renders_the_same_query() {
    let output = datagov()
        .args([
            "query",
            "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state ORDER BY state LIMIT 3",
            "--output",
            "table",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("state"));
    assert!(stdout.contains("total"));
    assert!(stdout.contains("AB"));
}

#[test]
fn bounded_output_reports_truncation_and_limit_overrides_it() {
    let default_bound = run_query_json("SELECT * FROM 'examples/customers.parquet'", &[]);
    let query = &default_bound["extensions"]["query"];
    assert_eq!(query["truncated"], true);
    assert_eq!(query["row_count_returned"], 1000);
    assert_eq!(query["total_row_count"], 8682);

    let overridden = run_query_json(
        "SELECT * FROM 'examples/customers.parquet'",
        &["--limit", "20"],
    );
    let query = &overridden["extensions"]["query"];
    assert_eq!(query["truncated"], true);
    assert_eq!(query["row_count_returned"], 20);
    assert_eq!(query["total_row_count"], 8682);
}

#[test]
fn missing_file_in_from_exits_3() {
    datagov()
        .args(["query", "SELECT * FROM 'missing.parquet'"])
        .assert()
        .code(3)
        .stderr(predicates::str::contains("missing.parquet"));
}

#[test]
fn unsupported_extension_exits_4() {
    let path = examples_dir().join("customers.json");
    let sql = format!("SELECT * FROM '{}'", path.display());
    datagov().args(["query", &sql]).assert().code(4);
}

#[test]
fn invalid_sql_exits_2() {
    datagov()
        .args(["query", "SELECT FROM WHERE"])
        .assert()
        .code(2);
}

#[test]
fn inspect_output_csv_is_rejected_with_exit_2() {
    let path = examples_dir().join("customers.csv");
    datagov()
        .args(["inspect", path.to_str().unwrap(), "--output", "csv"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("csv"));
}

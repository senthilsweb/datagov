//! profile.rs — Integration tests for `datagov profile` (assert_cmd),
//! run against the real committed `examples/customers.*` fixtures (never
//! synthetic stand-ins) — mirrors the style of Bolt 2's `tests/inspect.rs`.
//!
//! 1. Golden tests, one per fixture format (`customers.csv`,
//!    `customers.parquet`): run `datagov profile examples/customers.<ext>
//!    --output json`, normalize the `run` block (id/timestamps/duration)
//!    and `input.uri`, and assert the result matches the committed
//!    snapshot under `tests/golden/profile_customers_<format>.json`.
//! 2. Determinism (HARD, per the spec delta): profiling the same fixture
//!    twice with the same flags produces byte-identical `profile`
//!    sections.
//! 3. Masking (HARD): grep the full JSON output for the known raw
//!    `email`/`phone` values of the fixture's first rows — computed
//!    independently from the raw CSV bytes, not via `datagov-data` — and
//!    assert none of them appear anywhere in the output.
//! 4. `--columns` restricts the output to the named columns; an unknown
//!    column name is `InvalidArgs` (exit 2), naming it.
//! 5. `--sample 10` sets `sample_size: 10` and produces stats consistent
//!    with a hand-computed expectation over the fixture's first 10 rows
//!    (`userid` is `1..=10` in `customers.csv`, so `mean == 5.5`).

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn datagov() -> Command {
    Command::cargo_bin("datagov").expect("datagov binary should build")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn golden_path(format: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/golden/profile_customers_{format}.json"))
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

/// Replace the volatile `run` block and `input.uri` the same way Bolt
/// 2's `tests/inspect.rs` does, so golden files compare equal across
/// machines/runs.
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

fn run_profile_json(args: &[&str]) -> Value {
    let output = datagov()
        .arg("profile")
        .args(args)
        .args(["--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    serde_json::from_str(stdout.trim()).expect("stdout must be a single JSON document")
}

/// Independently read the first `n` raw `email`/`phone` values from
/// `examples/customers.csv` — deliberately not going through
/// `datagov-data`, so the masking assertion has an oracle that doesn't
/// share code with the thing being tested.
fn raw_email_and_phone(n: usize) -> (Vec<String>, Vec<String>) {
    let path = examples_dir().join("customers.csv");
    let mut reader = csv::ReaderBuilder::new().from_path(&path).unwrap();
    let headers = reader.headers().unwrap().clone();
    let email_idx = headers.iter().position(|h| h == "email").unwrap();
    let phone_idx = headers.iter().position(|h| h == "phone").unwrap();

    let mut emails = Vec::new();
    let mut phones = Vec::new();
    for record in reader.records().take(n) {
        let record = record.unwrap();
        emails.push(record.get(email_idx).unwrap().to_string());
        phones.push(record.get(phone_idx).unwrap().to_string());
    }
    (emails, phones)
}

fn assert_golden(format: &str, ext: &str) {
    let path = examples_dir().join(format!("customers.{ext}"));
    let live = normalize_run(run_profile_json(&[path.to_str().unwrap()]));

    let golden_text = std::fs::read_to_string(golden_path(format)).unwrap_or_else(|_| {
        panic!("missing golden file for {format} — bootstrap it from a real, hand-reviewed run")
    });
    let golden: Value = serde_json::from_str(&golden_text).expect("golden file must be valid JSON");

    assert_eq!(
        live, golden,
        "profile output for customers.{ext} drifted from the committed golden snapshot"
    );

    let validator = load_validator();
    assert!(
        validator.is_valid(&live),
        "envelope failed schema validation: {live}"
    );
}

#[test]
fn golden_customers_csv() {
    assert_golden("csv", "csv");
}

#[test]
fn golden_customers_parquet() {
    assert_golden("parquet", "parquet");
}

#[test]
fn determinism_two_runs_produce_identical_profile_sections() {
    let path = examples_dir().join("customers.csv");
    let first = run_profile_json(&[path.to_str().unwrap()]);
    let second = run_profile_json(&[path.to_str().unwrap()]);
    assert_eq!(
        first["profile"], second["profile"],
        "profile sections must be byte-identical across repeated runs on an unchanged input"
    );
}

#[test]
fn email_and_phone_never_appear_raw_anywhere_in_output() {
    let path = examples_dir().join("customers.csv");
    let assert = datagov()
        .args(["profile", path.to_str().unwrap(), "--output", "json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let (emails, phones) = raw_email_and_phone(50);
    for raw in emails.iter().chain(phones.iter()) {
        assert!(
            !stdout.contains(raw.as_str()),
            "raw fixture value {raw:?} leaked into profile output"
        );
    }
}

#[test]
fn columns_flag_restricts_output() {
    let path = examples_dir().join("customers.csv");
    let value = run_profile_json(&[path.to_str().unwrap(), "--columns", "email,state"]);
    let columns = value["profile"]["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2);
    let names: Vec<&str> = columns
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["email", "state"]);
}

#[test]
fn unknown_column_exits_2() {
    let path = examples_dir().join("customers.csv");
    datagov()
        .args([
            "profile",
            path.to_str().unwrap(),
            "--columns",
            "not_a_real_column",
        ])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("not_a_real_column"));
}

#[test]
fn sample_10_matches_hand_computed_expectation() {
    let path = examples_dir().join("customers.csv");
    let value = run_profile_json(&[
        path.to_str().unwrap(),
        "--columns",
        "userid",
        "--sample",
        "10",
    ]);
    assert_eq!(value["profile"]["sample_size"], 10);

    let userid = &value["profile"]["columns"][0];
    assert_eq!(userid["row_count"], 10);
    // customers.csv's userid column is 1..=2000 in source order, so the
    // first 10 rows are exactly 1..=10.
    assert_eq!(userid["min"], 1);
    assert_eq!(userid["max"], 10);
    assert_eq!(userid["mean"], 5.5);
    assert_eq!(userid["distinct_count"], 10);
    assert_eq!(userid["uniqueness_percentage"], 100.0);
}

#[test]
fn output_csv_is_rejected_with_exit_2() {
    let path = examples_dir().join("customers.csv");
    datagov()
        .args(["profile", path.to_str().unwrap(), "--output", "csv"])
        .assert()
        .code(2);
}

#[test]
fn missing_file_exits_3() {
    let path = examples_dir().join("does-not-exist.csv");
    datagov()
        .args(["profile", path.to_str().unwrap()])
        .assert()
        .code(3);
}

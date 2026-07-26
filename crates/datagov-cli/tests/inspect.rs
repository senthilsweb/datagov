//! inspect.rs — Integration tests for `datagov inspect` (assert_cmd),
//! run against the real committed `examples/customers.*` fixtures (never
//! synthetic stand-ins).
//!
//! 1. Golden tests, one per fixture format: run `datagov inspect
//!    examples/customers.<ext> --output json`, normalize the `run` block
//!    (id/timestamps/duration), and assert the result matches the
//!    committed snapshot under `tests/golden/inspect_customers_<format>.json`
//!    — bootstrapped in this bolt from a real run, hand-reviewed against
//!    `examples/README.md`'s documented row counts.
//! 2. Per fixture: `row_count` matches `examples/README.md`; `schema`
//!    contains `email` and `phone`; every sample row's `email`/`phone` is
//!    masked — computed independently from the raw fixture bytes (not
//!    via `datagov-data`) and compared against `datagov_core::mask::Masked`,
//!    the single shared masking implementation.
//! 3. Exit codes: missing file → 3; unsupported format
//!    (`examples/README.md`, used as a non-dataset fixture) → 4; stdin
//!    with no `--type` → 2.
//! 4. Stdin: `cat customers.jsonl | datagov inspect - --type jsonl
//!    --output json` succeeds, `input.uri == "-"`, no `content_hash`.
//! 5. Parquet-specific: `compression_codecs == ["SNAPPY"]` and row-group
//!    row counts sum to the file's total row count.

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn datagov() -> Command {
    Command::cargo_bin("datagov").expect("datagov binary should build")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn golden_path(format: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/golden/inspect_customers_{format}.json"))
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

/// Replace the volatile `run` block (id/timestamps/duration) with a
/// fixed sentinel so two runs of the same command compare equal.
/// `input.uri` is also normalized, since the golden files are checked
/// out at different absolute paths on different machines/CI — the
/// committed golden JSON stores the sentinel `"<uri>"` rather than a
/// path that would only ever match on the machine it was bootstrapped
/// on.
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

fn run_inspect_json(path: &str) -> Value {
    let output = datagov()
        .args(["inspect", path, "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    serde_json::from_str(stdout.trim()).expect("stdout must be a single JSON document")
}

/// Independently read the first `n` raw values of `column` from the
/// fixture — deliberately not going through `datagov-data`, so the
/// masking assertion has an oracle that doesn't share code with the
/// thing being tested.
fn raw_first_n(format: &str, path: &Path, column: &str, n: usize) -> Vec<String> {
    match format {
        "csv" | "tsv" => {
            let delimiter = if format == "tsv" { b'\t' } else { b',' };
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(delimiter)
                .from_path(path)
                .unwrap();
            let headers = reader.headers().unwrap().clone();
            let idx = headers.iter().position(|h| h == column).unwrap();
            reader
                .records()
                .take(n)
                .map(|r| r.unwrap().get(idx).unwrap().to_string())
                .collect()
        }
        "json" => {
            let content = std::fs::read_to_string(path).unwrap();
            let value: Value = serde_json::from_str(&content).unwrap();
            value
                .as_array()
                .unwrap()
                .iter()
                .take(n)
                .map(|record| record[column].as_str().unwrap().to_string())
                .collect()
        }
        "jsonl" => {
            let content = std::fs::read_to_string(path).unwrap();
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(n)
                .map(|line| {
                    let value: Value = serde_json::from_str(line).unwrap();
                    value[column].as_str().unwrap().to_string()
                })
                .collect()
        }
        "parquet" => {
            use parquet::file::reader::{FileReader, SerializedFileReader};
            use parquet::record::RowAccessor;

            let file = std::fs::File::open(path).unwrap();
            let reader = SerializedFileReader::new(file).unwrap();
            let schema_descr = reader.metadata().file_metadata().schema_descr();
            let idx = (0..schema_descr.num_columns())
                .find(|&i| schema_descr.column(i).name() == column)
                .unwrap();
            let mut iter = reader.get_row_iter(None).unwrap();
            let mut values = Vec::new();
            for _ in 0..n {
                let row = iter.next().unwrap().unwrap();
                values.push(row.get_string(idx).unwrap().clone());
            }
            values
        }
        other => panic!("unexpected format: {other}"),
    }
}

fn assert_golden_and_masking(format: &str, ext: &str, expected_row_count: u64) {
    let path = examples_dir().join(format!("customers.{ext}"));
    let live = normalize_run(run_inspect_json(path.to_str().unwrap()));

    let golden_text = std::fs::read_to_string(golden_path(format)).unwrap_or_else(|_| {
        panic!("missing golden file for {format} — bootstrap it from a real, hand-reviewed run")
    });
    let golden: Value = serde_json::from_str(&golden_text).expect("golden file must be valid JSON");

    assert_eq!(
        live, golden,
        "inspect output for customers.{ext} drifted from the committed golden snapshot"
    );

    let validator = load_validator();
    assert!(
        validator.is_valid(&live),
        "envelope failed schema validation: {live}"
    );

    let dataset = &live["dataset"];
    assert_eq!(
        dataset["row_count"].as_u64().unwrap(),
        expected_row_count,
        "row_count for customers.{ext} must match examples/README.md"
    );

    let columns: Vec<&str> = dataset["schema"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(columns.contains(&"email"), "schema must contain 'email'");
    assert!(columns.contains(&"phone"), "schema must contain 'phone'");

    let sample_rows = dataset["sample_rows"].as_array().unwrap();
    let raw_emails = raw_first_n(format, &path, "email", sample_rows.len());
    let raw_phones = raw_first_n(format, &path, "phone", sample_rows.len());

    for (i, row) in sample_rows.iter().enumerate() {
        let masked_email = row["email"].as_str().unwrap();
        let masked_phone = row["phone"].as_str().unwrap();
        let expected_email = datagov_core::mask::Masked::new(raw_emails[i].clone()).to_string();
        let expected_phone = datagov_core::mask::Masked::new(raw_phones[i].clone()).to_string();

        assert_eq!(
            masked_email, expected_email,
            "email mismatch in row {i} of {ext}"
        );
        assert_eq!(
            masked_phone, expected_phone,
            "phone mismatch in row {i} of {ext}"
        );
        assert_ne!(
            masked_email, raw_emails[i],
            "email must never equal the raw value"
        );
        assert_ne!(
            masked_phone, raw_phones[i],
            "phone must never equal the raw value"
        );
    }
}

#[test]
fn golden_customers_csv() {
    assert_golden_and_masking("csv", "csv", 2000);
}

#[test]
fn golden_customers_tsv() {
    assert_golden_and_masking("tsv", "tsv", 200);
}

#[test]
fn golden_customers_json() {
    assert_golden_and_masking("json", "json", 50);
}

#[test]
fn golden_customers_jsonl() {
    assert_golden_and_masking("jsonl", "jsonl", 200);
}

#[test]
fn golden_customers_parquet() {
    assert_golden_and_masking("parquet", "parquet", 8682);
}

#[test]
fn parquet_reports_snappy_compression_and_consistent_row_groups() {
    let path = examples_dir().join("customers.parquet");
    let live = run_inspect_json(path.to_str().unwrap());
    let dataset = &live["dataset"];

    let parquet = &dataset["parquet"];
    assert_eq!(parquet["compression_codecs"], serde_json::json!(["SNAPPY"]));

    let row_groups = parquet["row_groups"].as_array().unwrap();
    assert!(!row_groups.is_empty());
    let summed: i64 = row_groups
        .iter()
        .map(|rg| rg["row_count"].as_i64().unwrap())
        .sum();
    assert_eq!(summed as u64, dataset["row_count"].as_u64().unwrap());
}

#[test]
fn missing_file_exits_3() {
    let path = examples_dir().join("does-not-exist.csv");
    datagov()
        .args(["inspect", path.to_str().unwrap()])
        .assert()
        .code(3);
}

#[test]
fn unsupported_format_exits_4() {
    // README.md is a real committed file, not a dataset — its content
    // sniffs to a tie (no commas or tabs on its first line) and is
    // reported as unsupported.
    let path = examples_dir().join("README.md");
    datagov()
        .args(["inspect", path.to_str().unwrap()])
        .assert()
        .code(4);
}

#[test]
fn stdin_without_type_exits_2() {
    // No `--type` given while reading from stdin is a hard error before
    // any bytes are read, so the piped content here is irrelevant.
    datagov()
        .args(["inspect", "-"])
        .write_stdin("irrelevant")
        .assert()
        .code(2);
}

#[test]
fn stdin_jsonl_succeeds_with_dash_uri_and_no_content_hash() {
    let content = std::fs::read(examples_dir().join("customers.jsonl")).unwrap();

    let assert = datagov()
        .args(["inspect", "-", "--type", "jsonl", "--output", "json"])
        .write_stdin(content)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();

    let validator = load_validator();
    assert!(
        validator.is_valid(&value),
        "envelope failed schema validation: {value}"
    );

    assert_eq!(value["input"]["uri"], "-");
    assert_eq!(value["input"]["format"], "jsonl");
    assert!(
        value["input"].get("content_hash").is_none(),
        "stdin input must not carry a content_hash"
    );
    assert_eq!(value["dataset"]["row_count"].as_u64().unwrap(), 200);
}

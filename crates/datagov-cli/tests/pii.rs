//! pii.rs — Integration tests for `datagov pii scan | recognizers
//! list | recognizers validate` (assert_cmd), run against the real
//! committed `examples/customers.*` fixtures (for `EMAIL_ADDRESS`/
//! `PHONE_NUMBER`) and the new `examples/pii-fixture.csv` (for the other
//! 8 inception-gate-confirmed entities).
//!
//! 1. **The masking eval (HARD, this bolt's single most important
//!    property)**: run `pii scan` against `examples/customers.parquet`
//!    and `examples/pii-fixture.csv` in both `--output json` and
//!    `--output table`, capture the complete stdout/stderr of every run,
//!    and grep all of it for every known raw fixture value — computed
//!    independently (via the `csv` crate directly, never through
//!    `datagov-data`/`datagov-pii`) from `examples/customers.csv` (first
//!    100 email/phone values — matching the bound Bolt 2/3's own masking
//!    evals already use) and from every data cell of
//!    `examples/pii-fixture.csv` (all 10 rows). Zero hits, no
//!    exceptions.
//! 2. Per-entity detection: each of the 10 built-in entities produces at
//!    least one finding on the appropriate fixture, with `confidence`,
//!    `match_count`, `match_percentage` present and sane.
//! 3. Confidence model: golden tests pinning the exact
//!    `base + validator_bonus + context_bonus` arithmetic for several
//!    findings (`email` on `customers.csv`, `ssn`/`credit_card_number`
//!    on `pii-fixture.csv`).
//! 4. `--sample N` restricts scanning; `--field` restricts columns;
//!    unknown field → exit 2.
//! 5. `--recognizers`: a custom recognizer overrides a built-in by id
//!    (`recognizers/example-custom.yaml`'s `us_ssn` entry); a malformed
//!    recognizer file (`recognizers/example-invalid.yaml`) → exit 2
//!    naming the bad id/field.
//! 6. `--fail-on`: a threshold below an actual finding's confidence →
//!    exit 12; a threshold above all findings → exit 0. The report is
//!    present (non-empty, schema-valid) in both cases.
//! 7. `pii recognizers list` (10 built-ins) and `validate` (valid +
//!    invalid file cases).
//! 8. Missing input file → exit 3; unsupported format (`.json`) → exit 4.

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn datagov() -> Command {
    Command::cargo_bin("datagov").expect("datagov binary should build")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn recognizers_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../recognizers")
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

fn run_scan_json(args: &[&str]) -> Value {
    let output = datagov()
        .arg("pii")
        .arg("scan")
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

fn findings(value: &Value) -> Vec<Value> {
    value["pii"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn find_entity<'a>(findings: &'a [Value], entity: &str) -> Option<&'a Value> {
    findings.iter().find(|f| f["entity"] == entity)
}

// ---------------------------------------------------------------------
// 1. The masking eval (HARD)
// ---------------------------------------------------------------------

/// Independently read the first `n` raw `email`/`phone` values from
/// `examples/customers.csv` — deliberately not going through
/// `datagov-data`/`datagov-pii`, so the masking assertion has an oracle
/// that doesn't share code with the thing being tested. Mirrors
/// `crates/datagov-cli/tests/profile.rs`'s `raw_email_and_phone`.
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

/// Every raw data-cell value in `examples/pii-fixture.csv` (all rows,
/// all columns except the non-PII `record_id`) — the independent oracle
/// for the fixture that covers the other 8 entities.
fn raw_pii_fixture_values() -> Vec<String> {
    let path = examples_dir().join("pii-fixture.csv");
    let mut reader = csv::ReaderBuilder::new().from_path(&path).unwrap();
    let headers = reader.headers().unwrap().clone();
    let record_id_idx = headers.iter().position(|h| h == "record_id");

    let mut values = Vec::new();
    for record in reader.records() {
        let record = record.unwrap();
        for (idx, field) in record.iter().enumerate() {
            if Some(idx) == record_id_idx {
                continue; // not PII, and too short/generic to grep safely
            }
            values.push(field.to_string());
        }
    }
    values
}

#[test]
fn masking_eval_no_raw_fixture_value_ever_appears_in_pii_scan_output() {
    let (emails, phones) = raw_email_and_phone(100);
    let fixture_values = raw_pii_fixture_values();

    let parquet_path = examples_dir().join("customers.parquet");
    let fixture_path = examples_dir().join("pii-fixture.csv");

    let mut full_surface = String::new();
    for path in [&parquet_path, &fixture_path] {
        for output_mode in ["json", "table"] {
            let assert = datagov()
                .args([
                    "pii",
                    "scan",
                    path.to_str().unwrap(),
                    "--output",
                    output_mode,
                ])
                .assert()
                .success();
            let out = assert.get_output();
            full_surface.push_str(&String::from_utf8_lossy(&out.stdout));
            full_surface.push('\n');
            full_surface.push_str(&String::from_utf8_lossy(&out.stderr));
            full_surface.push('\n');
        }
    }

    for raw in emails
        .iter()
        .chain(phones.iter())
        .chain(fixture_values.iter())
    {
        assert!(
            !full_surface.contains(raw.as_str()),
            "raw fixture value {raw:?} leaked into pii scan output"
        );
    }
}

#[test]
fn masking_eval_covers_fail_on_and_error_paths_too() {
    // The brief is explicit: the no-raw-value guarantee holds "with or
    // without a --fail-on threshold". Exercise the threshold-breach exit
    // path (still prints the full report first) and an error path
    // (unknown --field) against the fixture that actually contains raw
    // secrets.
    let fixture_values = raw_pii_fixture_values();
    let fixture_path = examples_dir().join("pii-fixture.csv");

    let breach = datagov()
        .args([
            "pii",
            "scan",
            fixture_path.to_str().unwrap(),
            "--fail-on",
            "0.5",
            "--output",
            "json",
        ])
        .assert()
        .code(12);
    let breach_out = breach.get_output();
    let mut surface = String::new();
    surface.push_str(&String::from_utf8_lossy(&breach_out.stdout));
    surface.push_str(&String::from_utf8_lossy(&breach_out.stderr));

    let error_path = datagov()
        .args([
            "pii",
            "scan",
            fixture_path.to_str().unwrap(),
            "--field",
            "not_a_real_field",
        ])
        .assert()
        .code(2);
    let error_out = error_path.get_output();
    surface.push_str(&String::from_utf8_lossy(&error_out.stdout));
    surface.push_str(&String::from_utf8_lossy(&error_out.stderr));

    for raw in &fixture_values {
        assert!(
            !surface.contains(raw.as_str()),
            "raw fixture value {raw:?} leaked into pii scan's --fail-on or error output"
        );
    }
}

// ---------------------------------------------------------------------
// 2. Per-entity detection
// ---------------------------------------------------------------------

fn assert_sane_finding(finding: &Value) {
    let confidence = finding["confidence"].as_f64().expect("confidence present");
    assert!(
        (0.0..=1.0).contains(&confidence),
        "confidence out of range: {confidence}"
    );
    let match_count = finding["match_count"]
        .as_u64()
        .expect("match_count present");
    assert!(match_count >= 1);
    let match_percentage = finding["match_percentage"]
        .as_f64()
        .expect("match_percentage present");
    assert!(
        (0.0..=100.0).contains(&match_percentage),
        "match_percentage out of range: {match_percentage}"
    );
    assert!(finding["reason"].as_str().is_some_and(|s| !s.is_empty()));
}

#[test]
fn email_address_detected_on_customers_csv() {
    let path = examples_dir().join("customers.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "email"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "EMAIL_ADDRESS").expect("EMAIL_ADDRESS finding");
    assert_sane_finding(f);
}

#[test]
fn phone_number_detected_on_customers_csv() {
    let path = examples_dir().join("customers.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "phone"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "PHONE_NUMBER").expect("PHONE_NUMBER finding");
    assert_sane_finding(f);
}

#[test]
fn all_eight_remaining_entities_detected_on_pii_fixture() {
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap()]);
    let found = findings(&value);

    for entity in [
        "IP_ADDRESS_V4",
        "IP_ADDRESS_V6",
        "URL",
        "US_SSN",
        "CREDIT_CARD",
        "UUID",
        "MAC_ADDRESS",
        "US_ZIP_CODE",
    ] {
        let f = find_entity(&found, entity)
            .unwrap_or_else(|| panic!("expected a {entity} finding on pii-fixture.csv"));
        assert_sane_finding(f);
    }

    // The validator loop above also indirectly confirms the schema
    // envelope is well-formed; explicitly validate it too.
    let validator = load_validator();
    assert!(
        validator.is_valid(&value),
        "envelope failed schema validation: {value}"
    );
}

#[test]
fn embedded_matches_are_found_in_the_notes_column() {
    // PRD §10.8's `--field text` example: find_iter must catch a match
    // embedded in prose, not just a whole-value match.
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "notes"]);
    let found = findings(&value);
    assert!(
        find_entity(&found, "URL").is_some() || find_entity(&found, "IP_ADDRESS_V4").is_some(),
        "expected an embedded URL or IPv4 match in the notes column, got {found:?}"
    );
}

// ---------------------------------------------------------------------
// 3. Confidence model golden tests
// ---------------------------------------------------------------------

#[test]
fn confidence_golden_email_on_customers_csv() {
    // email_address: base 0.75, no validator, column name "email"
    // matches its own context term -> +0.05 context bonus. 0.75+0.05=0.80.
    let path = examples_dir().join("customers.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "email"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "EMAIL_ADDRESS").unwrap();
    assert_eq!(f["confidence"], 0.80);
}

#[test]
fn confidence_golden_ssn_on_pii_fixture() {
    // us_ssn: base 0.75, validator (SSA area check) passes on every
    // candidate (area 555, never issued) -> +0.10, column name "ssn"
    // matches its own context term -> +0.05. 0.75+0.10+0.05=0.90.
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "ssn"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "US_SSN").unwrap();
    assert_eq!(f["confidence"], 0.90);
}

#[test]
fn confidence_golden_credit_card_on_pii_fixture() {
    // credit_card: base 0.70, Luhn validator passes on every known test
    // card number -> +0.10, column name "credit_card_number" matches
    // its own context term -> +0.05. 0.70+0.10+0.05=0.85.
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "credit_card_number"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "CREDIT_CARD").unwrap();
    assert_eq!(f["confidence"], 0.85);
}

#[test]
fn confidence_golden_mac_address_has_no_validator_bonus() {
    // mac_address: base 0.80, no validator declared -> no bonus even
    // though every value is a genuine MAC, column name "mac_address"
    // matches its own context term -> +0.05. 0.80+0.05=0.85.
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "mac_address"]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "MAC_ADDRESS").unwrap();
    assert_eq!(f["confidence"], 0.85);
}

// ---------------------------------------------------------------------
// 4. --sample / --field
// ---------------------------------------------------------------------

#[test]
fn sample_restricts_scanned_rows() {
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--sample", "3"]);
    assert_eq!(value["pii"]["scanned_rows"], 3);
    assert_eq!(value["pii"]["sample_size"], 3);
}

#[test]
fn field_restricts_scanned_columns() {
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--field", "ssn,zip_code"]);
    assert_eq!(value["pii"]["scanned_columns"], 2);
    let found = findings(&value);
    assert!(
        found
            .iter()
            .all(|f| f["column"] == "ssn" || f["column"] == "zip_code")
    );
}

#[test]
fn unknown_field_exits_2() {
    let path = examples_dir().join("pii-fixture.csv");
    datagov()
        .args([
            "pii",
            "scan",
            path.to_str().unwrap(),
            "--field",
            "not_a_real_field",
        ])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("not_a_real_field"));
}

// ---------------------------------------------------------------------
// 5. --recognizers
// ---------------------------------------------------------------------

#[test]
fn custom_recognizer_overrides_a_built_in_by_id() {
    let path = examples_dir().join("pii-fixture.csv");
    let recognizers_path = recognizers_dir().join("example-custom.yaml");
    let value = run_scan_json(&[
        path.to_str().unwrap(),
        "--field",
        "ssn",
        "--recognizers",
        recognizers_path.to_str().unwrap(),
    ]);
    let found_findings = findings(&value);
    let f = find_entity(&found_findings, "US_SSN").unwrap();
    // The override's confidence is 0.85 (vs the built-in's 0.75); with
    // both bonuses (+0.10 validator, +0.05 context) it clamps to 1.0.
    assert_eq!(f["confidence"], 1.0);
}

#[test]
fn malformed_recognizers_file_exits_2_naming_id_and_field() {
    let path = examples_dir().join("pii-fixture.csv");
    let recognizers_path = recognizers_dir().join("example-invalid.yaml");
    datagov()
        .args([
            "pii",
            "scan",
            path.to_str().unwrap(),
            "--recognizers",
            recognizers_path.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("broken_pattern"))
        .stderr(predicates::str::contains("patterns"));
}

// ---------------------------------------------------------------------
// 6. --fail-on
// ---------------------------------------------------------------------

#[test]
fn fail_on_below_a_finding_confidence_exits_12_with_report_present() {
    let path = examples_dir().join("pii-fixture.csv");
    let assert = datagov()
        .args([
            "pii",
            "scan",
            path.to_str().unwrap(),
            "--fail-on",
            "0.5",
            "--output",
            "json",
        ])
        .assert()
        .code(12);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(
        !findings(&value).is_empty(),
        "report must still be present on --fail-on breach"
    );
    let validator = load_validator();
    assert!(validator.is_valid(&value));
}

#[test]
fn fail_on_above_all_findings_exits_0_with_report_present() {
    let path = examples_dir().join("pii-fixture.csv");
    let value = run_scan_json(&[path.to_str().unwrap(), "--fail-on", "0.999"]);
    assert!(
        !findings(&value).is_empty(),
        "report must still be present when --fail-on doesn't trip"
    );
}

// ---------------------------------------------------------------------
// 7. pii recognizers list | validate
// ---------------------------------------------------------------------

#[test]
fn recognizers_list_enumerates_the_ten_built_ins() {
    let output = datagov()
        .args(["pii", "recognizers", "list", "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    let list = value["extensions"]["pii_recognizers"]
        .as_array()
        .expect("pii_recognizers extension present");
    assert_eq!(list.len(), 10);
    let ids: Vec<&str> = list.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"us_ssn"));
    assert!(ids.contains(&"credit_card"));
}

#[test]
fn recognizers_validate_succeeds_on_a_valid_file() {
    let path = recognizers_dir().join("example-custom.yaml");
    datagov()
        .args(["pii", "recognizers", "validate", path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn recognizers_validate_fails_on_an_invalid_file() {
    let path = recognizers_dir().join("example-invalid.yaml");
    datagov()
        .args(["pii", "recognizers", "validate", path.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("broken_pattern"));
}

// ---------------------------------------------------------------------
// 8. Exit codes: missing input / unsupported format
// ---------------------------------------------------------------------

#[test]
fn missing_input_file_exits_3() {
    let path = examples_dir().join("does-not-exist.csv");
    datagov()
        .args(["pii", "scan", path.to_str().unwrap()])
        .assert()
        .code(3);
}

#[test]
fn unsupported_json_format_exits_4() {
    let path = examples_dir().join("customers.json");
    datagov()
        .args(["pii", "scan", path.to_str().unwrap()])
        .assert()
        .code(4);
}

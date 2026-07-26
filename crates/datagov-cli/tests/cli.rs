//! cli.rs — Integration tests for the `datagov` binary (assert_cmd).
//!
//! 1. `datagov version` (human) exits 0 and prints the crate version.
//! 2. `datagov version --output json` exits 0, prints a JSON envelope
//!    that validates against the committed `docs/schema/report-v1.json`,
//!    with a UUID `run.id` and `schema_version == "1.0"`.
//! 3. Clap usage errors (`--bogus-flag`, an unknown subcommand) exit 2.
//! 4. `datagov capabilities --output json` exits 0 and its envelope's
//!    `extensions.capabilities.commands` lists both compiled commands.
//! 5. `--quiet` and `--verbose` together exit 2 (clap conflict).
//! 6. Under `--output json`, stdout is exactly one JSON document;
//!    diagnostics (if any) never land on stdout.

use assert_cmd::Command;
use std::path::PathBuf;

fn datagov() -> Command {
    Command::cargo_bin("datagov").expect("datagov binary should build")
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json")
}

fn load_validator() -> jsonschema::Validator {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile")
}

#[test]
fn version_human_output_contains_crate_version() {
    let assert = datagov().arg("version").assert();
    assert
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_json_output_is_a_valid_envelope() {
    let output = datagov()
        .args(["version", "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must parse as JSON");

    let validator = load_validator();
    assert!(
        validator.is_valid(&value),
        "envelope failed schema validation: {value}"
    );

    assert_eq!(value["schema_version"], "1.0");
    let run_id = value["run"]["id"]
        .as_str()
        .expect("run.id must be a string");
    uuid::Uuid::parse_str(run_id).expect("run.id must parse as a UUID");

    assert!(value.get("input").is_none(), "version has no input");
    assert!(value.get("dataset").is_none(), "version has no sections");
}

#[test]
fn bogus_flag_exits_2() {
    datagov().arg("--bogus-flag").assert().code(2);
}

#[test]
fn unknown_subcommand_exits_2() {
    datagov().arg("nonexistent-cmd").assert().code(2);
}

#[test]
fn capabilities_json_lists_compiled_commands() {
    let output = datagov()
        .args(["capabilities", "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();

    let validator = load_validator();
    assert!(
        validator.is_valid(&value),
        "envelope failed schema validation: {value}"
    );

    let commands = value["extensions"]["capabilities"]["commands"]
        .as_array()
        .expect("extensions.capabilities.commands must be an array");
    let commands: Vec<&str> = commands.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(commands.contains(&"version"));
    assert!(commands.contains(&"capabilities"));
}

#[test]
fn quiet_and_verbose_together_exit_2() {
    datagov()
        .args(["--quiet", "--verbose", "version"])
        .assert()
        .code(2);
}

#[test]
fn json_output_stdout_is_exactly_one_json_document() {
    let assert = datagov()
        .args(["version", "--verbose", "--output", "json"])
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Exactly one non-empty line, and that line is a complete JSON
    // document with nothing trailing it.
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "stdout must be exactly one JSON document: {stdout:?}"
    );
    serde_json::from_str::<serde_json::Value>(lines[0]).expect("the single line must be JSON");

    // Any diagnostics from --verbose land on stderr, never stdout.
    let stdout_bytes = &output.stdout;
    let text = String::from_utf8_lossy(stdout_bytes);
    assert!(
        !text.contains("DEBUG"),
        "diagnostics must not leak onto stdout"
    );
}

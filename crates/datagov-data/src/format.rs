//! format.rs — Format detection for `datagov` dataset inputs.
//!
//! 1. Defines `Format`, the five Milestone-0.1 dataset formats (CSV, TSV,
//!    JSON, JSONL, Parquet).
//! 2. Implements `detect`, the detection order from the Bolt 2 brief:
//!    `--type` flag first, then file extension (case-insensitive), then
//!    content sniffing for real files, else `DatagovError::UnsupportedInput`
//!    (exit 4). Reading from stdin (`-`) with no `--type` is a hard
//!    `DatagovError::InvalidArgs` (exit 2) — content sniffing is only
//!    reachable for real files.
//! 3. Content-sniffing rules: Parquet magic bytes (`PAR1`) at file start;
//!    else the first non-whitespace byte `[` → JSON (array); `{` → JSON
//!    when the sniffed prefix has exactly one non-empty line (a bare
//!    single object, treated as a one-record dataset), else JSONL when it
//!    has more than one; otherwise assume delimited text and compare
//!    comma vs. tab frequency on the first line — a tie (including
//!    zero/zero, e.g. prose text with neither) is "still undetermined"
//!    and reported as `UnsupportedInput`.

use datagov_core::DatagovError;
use std::io::Read;
use std::path::Path;

/// One of the five Milestone-0.1 dataset formats `datagov inspect`
/// supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Tsv,
    Json,
    Jsonl,
    Parquet,
}

impl Format {
    /// The canonical lowercase name used in `--type`, extensions, and the
    /// report envelope's `input.format` / `dataset.format` fields.
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Tsv => "tsv",
            Format::Json => "json",
            Format::Jsonl => "jsonl",
            Format::Parquet => "parquet",
        }
    }

    /// Parse a `--type` flag value (case-insensitive). Unrecognized names
    /// are an invalid argument (exit 2).
    pub fn from_flag(value: &str) -> Result<Format, DatagovError> {
        match value.to_ascii_lowercase().as_str() {
            "csv" => Ok(Format::Csv),
            "tsv" => Ok(Format::Tsv),
            "json" => Ok(Format::Json),
            "jsonl" | "ndjson" => Ok(Format::Jsonl),
            "parquet" => Ok(Format::Parquet),
            other => Err(DatagovError::invalid_args(
                format!("unrecognized --type '{other}'"),
                "use one of: csv, tsv, json, jsonl, parquet".to_string(),
            )),
        }
    }
}

/// Number of bytes read from the front of a real file for content
/// sniffing — cheap even on large inputs, since detection only needs to
/// see the very start of the file (magic bytes, first line).
const SNIFF_PREFIX_BYTES: usize = 64 * 1024;

/// Resolve the `Format` for `target` (a path, or `-` for stdin).
///
/// `type_flag`, when given, always wins. Otherwise the file extension is
/// used; failing that, real files are content-sniffed. Reading from
/// stdin with no `--type` is a hard error before any sniffing is
/// attempted.
pub fn detect(target: &str, type_flag: Option<&str>) -> Result<Format, DatagovError> {
    if let Some(flag) = type_flag {
        return Format::from_flag(flag);
    }

    if target == "-" {
        return Err(DatagovError::invalid_args(
            "--type is required when reading from stdin",
            "pass --type <csv|tsv|json|jsonl|parquet>",
        ));
    }

    if let Some(format) = format_from_extension(target) {
        return Ok(format);
    }

    format_from_content(Path::new(target))
}

fn format_from_extension(target: &str) -> Option<Format> {
    let ext = Path::new(target)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    match ext.as_str() {
        "csv" => Some(Format::Csv),
        "tsv" => Some(Format::Tsv),
        "json" => Some(Format::Json),
        "jsonl" | "ndjson" => Some(Format::Jsonl),
        "parquet" => Some(Format::Parquet),
        _ => None,
    }
}

fn format_from_content(path: &Path) -> Result<Format, DatagovError> {
    let mut file = std::fs::File::open(path).map_err(|source| {
        DatagovError::input_not_found(
            format!("cannot open {}: {source}", path.display()),
            format!("check that {} exists and is readable", path.display()),
        )
    })?;

    let mut buf = vec![0u8; SNIFF_PREFIX_BYTES];
    let read = file.read(&mut buf).map_err(|source| {
        DatagovError::internal(format!("failed to read {}: {source}", path.display()))
    })?;
    buf.truncate(read);

    format_from_bytes(&buf).ok_or_else(|| {
        DatagovError::unsupported_input(
            format!(
                "could not detect the format of {} from its extension or content",
                path.display()
            ),
            "pass --type <csv|tsv|json|jsonl|parquet> to disambiguate".to_string(),
        )
    })
}

/// Pure content-sniffing over a byte prefix (no I/O) — split out for unit
/// testing without touching the filesystem.
fn format_from_bytes(buf: &[u8]) -> Option<Format> {
    if buf.starts_with(b"PAR1") {
        return Some(Format::Parquet);
    }

    let first_non_ws = buf.iter().position(|b| !b.is_ascii_whitespace())?;

    match buf[first_non_ws] {
        b'[' => Some(Format::Json),
        b'{' => {
            let non_empty_lines = buf
                .split(|&b| b == b'\n')
                .filter(|line| !line.iter().all(|b| b.is_ascii_whitespace()))
                .count();
            if non_empty_lines > 1 {
                Some(Format::Jsonl)
            } else {
                Some(Format::Json)
            }
        }
        _ => {
            let rest = &buf[first_non_ws..];
            let first_line_end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
            let first_line = &rest[..first_line_end];
            let commas = first_line.iter().filter(|&&b| b == b',').count();
            let tabs = first_line.iter().filter(|&&b| b == b'\t').count();
            match commas.cmp(&tabs) {
                std::cmp::Ordering::Greater => Some(Format::Csv),
                std::cmp::Ordering::Less => Some(Format::Tsv),
                std::cmp::Ordering::Equal => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extension_detection_is_case_insensitive() {
        assert_eq!(detect("data.CSV", None).unwrap(), Format::Csv);
        assert_eq!(detect("data.Tsv", None).unwrap(), Format::Tsv);
        assert_eq!(detect("data.JSON", None).unwrap(), Format::Json);
        assert_eq!(detect("data.JSONL", None).unwrap(), Format::Jsonl);
        assert_eq!(detect("data.ndjson", None).unwrap(), Format::Jsonl);
        assert_eq!(detect("data.PARQUET", None).unwrap(), Format::Parquet);
    }

    #[test]
    fn type_flag_overrides_extension() {
        assert_eq!(detect("data.csv", Some("json")).unwrap(), Format::Json);
    }

    #[test]
    fn type_flag_rejects_unknown_names() {
        let err = detect("data.csv", Some("xml")).unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }

    #[test]
    fn stdin_without_type_flag_is_invalid_args() {
        let err = detect("-", None).unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InvalidArgs);
    }

    #[test]
    fn stdin_with_type_flag_succeeds() {
        assert_eq!(detect("-", Some("jsonl")).unwrap(), Format::Jsonl);
    }

    #[test]
    fn content_sniffing_ambiguous_extensionless_csv() {
        // No recognized extension; content sniffing must fall back to
        // comma-vs-tab frequency on the first line.
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, "a,b,c\n1,2,3\n").unwrap();
        assert_eq!(detect(path.to_str().unwrap(), None).unwrap(), Format::Csv);
    }

    #[test]
    fn content_sniffing_ambiguous_extensionless_tsv() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, "a\tb\tc\n1\t2\t3\n").unwrap();
        assert_eq!(detect(path.to_str().unwrap(), None).unwrap(), Format::Tsv);
    }

    #[test]
    fn content_sniffing_json_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, "[{\"a\":1}]").unwrap();
        assert_eq!(detect(path.to_str().unwrap(), None).unwrap(), Format::Json);
    }

    #[test]
    fn content_sniffing_jsonl_multiline_object() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, "{\"a\":1}\n{\"a\":2}\n").unwrap();
        assert_eq!(detect(path.to_str().unwrap(), None).unwrap(), Format::Jsonl);
    }

    #[test]
    fn content_sniffing_single_object_is_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, "{\"a\":1}").unwrap();
        assert_eq!(detect(path.to_str().unwrap(), None).unwrap(), Format::Json);
    }

    #[test]
    fn content_sniffing_parquet_magic_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.dat");
        fs::write(&path, b"PAR1garbage").unwrap();
        assert_eq!(
            detect(path.to_str().unwrap(), None).unwrap(),
            Format::Parquet
        );
    }

    #[test]
    fn content_sniffing_gives_up_on_a_tie_and_is_unsupported() {
        // Prose text: no commas, no tabs on the first line — a tie that
        // cannot be resolved into a delimited format.
        let dir = tempdir().unwrap();
        let path = dir.path().join("README.dat");
        fs::write(
            &path,
            "# Example fixtures\n\nAll datasets here are synthetic.\n",
        )
        .unwrap();
        let err = detect(path.to_str().unwrap(), None).unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::UnsupportedInput);
    }

    #[test]
    fn missing_file_with_unrecognized_extension_is_input_not_found() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.dat");
        let err = detect(path.to_str().unwrap(), None).unwrap_err();
        assert_eq!(err.exit_code(), datagov_core::ExitCode::InputNotFound);
    }
}

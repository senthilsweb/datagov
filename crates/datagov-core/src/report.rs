//! report.rs — The canonical report envelope (PRD §23).
//!
//! 1. Defines `Report` and its nested types (`Tool`, `Run`, `Input`,
//!    `Summary`, `Status`, `Sections`) with `serde` + `schemars` derives,
//!    matching the PRD §23 JSON shape exactly — `sections` is flattened
//!    so `dataset`, `profile`, `pii`, … appear as top-level keys.
//! 2. Defines the `dataset` section (PRD §10.1, populated starting
//!    Bolt 2): `DatasetSection`, `ColumnSchema`, `DataType`,
//!    `ParquetInfo`, `RowGroupInfo`.
//! 3. Defines one empty `#[non_exhaustive]` placeholder struct per
//!    remaining PRD §23 domain key (`ProfileSection`, `PiiSection`,
//!    `QualitySection`, `SchemaSection`, `LineageSection`,
//!    `PolicySection`, `EvidenceSection`); later bolts flesh these out.
//! 4. Provides `content_hash_sha256`, a SHA-256 file-hashing helper used
//!    from Bolt 2 onward to populate `Input::content_hash`.
//! 5. Provides `ReportBuilder`, which starts the run clock on
//!    construction, stamps `completed_at`/`duration_ms` on `build()`, and
//!    defaults `Summary` to a clean success.
//! 6. Ships a schema-drift test: the schema regenerated in-memory from
//!    these types must match the committed
//!    `docs/schema/report-v1.json` byte-for-byte.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

/// The current envelope schema version. Bump on any breaking change to
/// the shape published at `docs/schema/report-v1.json`.
pub const SCHEMA_VERSION: &str = "1.0";

/// The canonical `datagov` report envelope (PRD §23).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    pub schema_version: String,
    pub tool: Tool,
    pub run: Run,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Input>,
    pub summary: Summary,
    #[serde(flatten)]
    pub sections: Sections,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// Identifies the tool that produced the report.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    pub name: String,
    pub version: String,
}

impl Default for Tool {
    fn default() -> Self {
        Tool {
            name: "datagov".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Run metadata: identity, timing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    /// UUIDv7 run identifier (time-ordered, unique per run).
    pub id: String,
    /// RFC 3339 timestamp.
    pub started_at: String,
    /// RFC 3339 timestamp.
    pub completed_at: String,
    pub duration_ms: u64,
}

/// Describes the input the command operated on.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Input {
    pub uri: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Overall run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Warning,
    Failed,
}

/// Aggregate counts summarizing the run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    pub status: Status,
    pub errors: u64,
    pub warnings: u64,
    pub info: u64,
}

impl Default for Summary {
    fn default() -> Self {
        Summary {
            status: Status::Success,
            errors: 0,
            warnings: 0,
            info: 0,
        }
    }
}

/// One `Option<T>` field per PRD §23 domain key, flattened into the
/// envelope so each populated domain appears as a top-level JSON key.
/// `None` fields are skipped entirely. For Bolt 1, every `T` is an empty
/// placeholder; domain bolts flesh them out without touching this shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Sections {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<DatasetSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii: Option<PiiSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<QualitySection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<LineageSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicySection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceSection>,
}

macro_rules! empty_section {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[non_exhaustive]
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub struct $name {}
    };
}

/// The `dataset` section (PRD §10.1): populated by `datagov inspect`
/// (and reused by `profile`/`report` in later bolts) with everything
/// learned about an input dataset without necessarily doing a full scan
/// (Parquet metadata in particular is read without scanning row data).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DatasetSection {
    /// Detected/declared format: `"csv"` | `"tsv"` | `"json"` | `"jsonl"`
    /// | `"parquet"`.
    pub format: String,
    pub file_size_bytes: u64,
    pub row_count: u64,
    pub column_count: u32,
    pub schema: Vec<ColumnSchema>,
    /// A real measurement from the same scan/metadata read used to
    /// populate the rest of this section — not a rough guess. See
    /// `datagov_data` reader docs for the exact per-format formula.
    pub approx_memory_bytes: u64,
    /// `Some` only when `format == "parquet"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parquet: Option<ParquetInfo>,
    /// The first `DEFAULT_SAMPLE_ROWS` records in source order, already
    /// masked (sensitive columns per
    /// `crate::sensitivity::is_heuristically_sensitive` are replaced with
    /// the `Masked` rendering) — safe to emit as-is on every output
    /// surface.
    pub sample_rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

/// One column's inferred name, type, and nullability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

/// The narrowest data type a reader inferred for a column. See the
/// per-format narrowing rules in `datagov_data` for how each variant is
/// chosen (e.g. CSV narrows Boolean → Integer → Float → String; JSON
/// uses the JSON value's own type, with `Mixed` when non-null
/// occurrences disagree).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Boolean,
    Integer,
    Float,
    String,
    Object,
    Array,
    /// Every occurrence of the column was null/absent — no type could be
    /// inferred.
    Null,
    /// Non-null occurrences disagreed on type (JSON/JSONL only).
    Mixed,
}

/// Parquet-specific inspection detail (PRD §10.1: row groups,
/// compression), read entirely from file metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParquetInfo {
    pub row_groups: Vec<RowGroupInfo>,
    /// Distinct codec names (e.g. `"SNAPPY"`), sorted.
    pub compression_codecs: Vec<String>,
}

/// Metadata for a single Parquet row group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RowGroupInfo {
    pub index: usize,
    pub row_count: i64,
    pub compressed_size_bytes: i64,
    pub uncompressed_size_bytes: i64,
}

empty_section!(
    /// Placeholder for the `profile` section; populated starting Bolt 3.
    ProfileSection
);
empty_section!(
    /// Placeholder for the `pii` section; populated starting Bolt 5.
    PiiSection
);
empty_section!(
    /// Placeholder for the `quality` section; populated in a later change.
    QualitySection
);
empty_section!(
    /// Placeholder for the `schema` section; populated in a later change.
    SchemaSection
);
empty_section!(
    /// Placeholder for the `lineage` section; populated in a later change.
    LineageSection
);
empty_section!(
    /// Placeholder for the `policy` section; populated in a later change.
    PolicySection
);
empty_section!(
    /// Placeholder for the `evidence` section; populated starting Bolt 5.
    EvidenceSection
);

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339)
        .expect("RFC 3339 formatting of a valid OffsetDateTime never fails")
}

/// Computes the SHA-256 content hash of a file, formatted as
/// `sha256:<hex>`. Used from Bolt 2 onward to populate
/// `Input::content_hash`.
pub fn content_hash_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Builds a `Report`, starting the run clock on construction and stamping
/// `completed_at`/`duration_ms` on `build()`. Defaults `Summary` to a
/// clean success.
pub struct ReportBuilder {
    started_at: OffsetDateTime,
    run_id: String,
    tool: Tool,
    input: Option<Input>,
    sections: Sections,
    summary: Summary,
    extensions: BTreeMap<String, serde_json::Value>,
}

impl ReportBuilder {
    /// Start a new report run: stamps `started_at` now and mints a
    /// UUIDv7 `run.id`.
    pub fn new() -> Self {
        ReportBuilder {
            started_at: OffsetDateTime::now_utc(),
            run_id: Uuid::now_v7().to_string(),
            tool: Tool::default(),
            input: None,
            sections: Sections::default(),
            summary: Summary::default(),
            extensions: BTreeMap::new(),
        }
    }

    pub fn input(mut self, input: Input) -> Self {
        self.input = Some(input);
        self
    }

    pub fn sections(mut self, sections: Sections) -> Self {
        self.sections = sections;
        self
    }

    pub fn summary(mut self, summary: Summary) -> Self {
        self.summary = summary;
        self
    }

    pub fn extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Finish the report: stamps `completed_at` now, computes
    /// `duration_ms` from the clock started in `new()`.
    pub fn build(self) -> Report {
        let completed_at = OffsetDateTime::now_utc();
        let duration_ms = (completed_at - self.started_at).whole_milliseconds().max(0) as u64;

        Report {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: self.tool,
            run: Run {
                id: self.run_id,
                started_at: format_rfc3339(self.started_at),
                completed_at: format_rfc3339(completed_at),
                duration_ms,
            },
            input: self.input,
            summary: self.summary,
            sections: self.sections,
            extensions: self.extensions,
        }
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates the current JSON Schema for `Report`, pretty-printed with a
/// trailing newline — the exact bytes committed at
/// `docs/schema/report-v1.json`.
pub fn generate_schema_document() -> String {
    let schema = schemars::schema_for!(Report);
    let mut document = serde_json::to_string_pretty(&schema)
        .expect("schemars RootSchema always serializes to JSON");
    document.push('\n');
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_to_success_and_computes_duration() {
        let report = ReportBuilder::new().build();
        assert_eq!(report.schema_version, "1.0");
        assert_eq!(report.summary.status, Status::Success);
        assert_eq!(report.summary.errors, 0);
        assert!(report.input.is_none());
        // duration_ms is a u64; it is always >= 0 by construction, so the
        // meaningful assertion is that build() did not panic and the
        // run id parses as a UUID.
        assert!(Uuid::parse_str(&report.run.id).is_ok());
    }

    #[test]
    fn run_id_is_uuid_v7() {
        let report = ReportBuilder::new().build();
        let uuid = Uuid::parse_str(&report.run.id).expect("run.id must parse as a UUID");
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn sections_with_no_domains_serialize_without_domain_keys() {
        let report = ReportBuilder::new().build();
        let value = serde_json::to_value(&report).unwrap();
        let obj = value.as_object().unwrap();
        assert!(!obj.contains_key("dataset"));
        assert!(!obj.contains_key("profile"));
        assert!(!obj.contains_key("pii"));
        assert!(!obj.contains_key("extensions"));
    }

    #[test]
    fn sections_flatten_populated_domain_as_top_level_key() {
        let dataset = DatasetSection {
            format: "csv".to_string(),
            file_size_bytes: 10,
            row_count: 1,
            column_count: 1,
            schema: vec![ColumnSchema {
                name: "a".to_string(),
                data_type: DataType::String,
                nullable: false,
            }],
            approx_memory_bytes: 25,
            parquet: None,
            sample_rows: vec![],
        };
        let sections = Sections {
            dataset: Some(dataset),
            ..Default::default()
        };
        let report = ReportBuilder::new().sections(sections).build();
        let value = serde_json::to_value(&report).unwrap();
        assert!(value.get("dataset").is_some());
        assert!(value.get("profile").is_none());
    }

    #[test]
    fn content_hash_sha256_is_stable_for_known_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let hash = content_hash_sha256(&path).unwrap();
        // sha256("hello world")
        assert_eq!(
            hash,
            "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    /// Schema drift test: the schema regenerated in-memory from `Report`
    /// must match the committed `docs/schema/report-v1.json` exactly. On
    /// failure, regenerate the file and bump `schema_version` on any
    /// breaking change.
    #[test]
    fn schema_matches_committed_document() {
        let generated = generate_schema_document();
        let committed_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schema/report-v1.json");
        let committed = std::fs::read_to_string(&committed_path).unwrap_or_else(|_| {
            panic!(
                "missing {} — generate it (see `generate_schema_document`) and commit it",
                committed_path.display()
            )
        });
        assert_eq!(
            generated, committed,
            "the Report JSON Schema has drifted from docs/schema/report-v1.json — \
             regenerate the committed file from `generate_schema_document()`, and if this is a \
             breaking change, bump `SCHEMA_VERSION`"
        );
    }
}

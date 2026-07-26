# Bolt 2 implementation brief — format detection, readers, `datagov inspect`

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). Contract: implement exactly this; where reality forces a
> deviation, stop and report it as a proposed **Correction** — do not
> silently improvise. Read first: `AGENTS.md`, `openspec/project.md`,
> `openspec/changes/add-datagov-core-cli/{proposal,design,tasks}.md`,
> `openspec/changes/add-datagov-core-cli/specs/dataset-inspection/spec.md`,
> `openspec/changes/add-datagov-core-cli/specs/cli-interface/spec.md`,
> `docs/prd.md` §10.1, `examples/README.md`.
> This brief builds directly on Bolt 1 (`datagov-core`'s report
> envelope, exit codes, error type, mask module) — read that code
> before starting; do not redefine anything Bolt 1 already built.

## Ground rules (same as Bolt 1)

- Do NOT run any git command (no add/commit). Leave the tree dirty for
  architect review.
- Do NOT edit anything under `openspec/` or `docs/prd.md`.
- Every source file opens with the AGENTS.md file-header comment.
- Diagnostics only via `tracing` to stderr; stdout is command output.
- Done only when `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass — using the **real committed
  fixtures** in `examples/`, not synthetic in-memory stand-ins.

## Fixtures already exist — do not generate your own

`examples/customers.{csv,tsv,json,jsonl,parquet}` are already committed
(one logical TICKIT-shaped dataset across all five formats — see
`examples/README.md` for exact schema, row counts, and provenance).
`customers.parquet` is **Snappy-compressed** — make sure whatever
Parquet crate/feature set you choose can actually decode it; verify by
running your code against the real file, not a hand-built minimal
Parquet buffer. `examples/README.md` also exists (not a dataset) — use
it as the fixture for the "unsupported format" exit-4 test.

## Scope decision: no DataFusion in this bolt

`datagov-data`'s design note says "engine: DataFusion" — that refers to
**profiling and query (Bolt 3)**, where aggregation genuinely benefits
from a query engine. `inspect` is metadata + schema + a few sample
rows; pulling in DataFusion/Arrow here would be scope creep against
PRD §34.3 (binary-size risk). For this bolt, add directly to
`[workspace.dependencies]`:

- `csv` (CSV **and** TSV — same crate, switch `ReaderBuilder::delimiter`)
- `parquet` (Apache Parquet Rust — **not** the `arrow` crate; use its
  own `file::reader::{FileReader, SerializedFileReader}` and
  `record::{Row, RowAccessor}` APIs for both metadata and row iteration,
  enabling whatever compression-codec feature(s) are needed to read
  Snappy)
- `comfy-table` (human-readable rendering, per PRD §22's recommended
  stack)
- `serde_json` is already a workspace dependency (Bolt 1) — use it for
  JSON/JSONL parsing.

## `datagov-core` additions (in `report.rs`, replacing the Bolt 1
placeholder `DatasetSection`)

```rust
pub struct DatasetSection {
    pub format: String,           // "csv" | "tsv" | "json" | "jsonl" | "parquet"
    pub file_size_bytes: u64,
    pub row_count: u64,
    pub column_count: u32,
    pub schema: Vec<ColumnSchema>,
    pub approx_memory_bytes: u64, // see "Memory estimate" below
    pub parquet: Option<ParquetInfo>,   // Some only for format == "parquet"
    pub sample_rows: Vec<serde_json::Map<String, serde_json::Value>>, // already masked
}

pub struct ColumnSchema { pub name: String, pub data_type: DataType, pub nullable: bool }

pub enum DataType { Boolean, Integer, Float, String, Object, Array, Null, Mixed }

pub struct ParquetInfo {
    pub row_groups: Vec<RowGroupInfo>,
    pub compression_codecs: Vec<String>, // distinct codec names, sorted
}

pub struct RowGroupInfo {
    pub index: usize,
    pub row_count: i64,
    pub compressed_size_bytes: i64,
    pub uncompressed_size_bytes: i64,
}
```

All derive `Serialize`/`Deserialize`/`schemars::JsonSchema`. Regenerate
`docs/schema/report-v1.json` after adding these (the Bolt 1 drift test
will fail until you do — that's expected schema evolution, not a bug;
regenerate and commit the new schema file).

Add `datagov-core::sensitivity::is_heuristically_sensitive(column_name:
&str) -> bool`: case-insensitive substring match against a small const
list — `["email", "phone", "ssn", "social_security", "credit_card",
"card_number", "password", "secret"]`. Doc-comment it clearly as a
**conservative, always-on heuristic that predates the full PII
recognizer engine (arriving in Bolt 5)** — it exists so `inspect` never
echoes obvious identifier columns in samples before that subsystem
exists, and Bolt 5 supersedes/extends it, not replaces it silently.

## `datagov-data` crate

### Format detection

`enum Format { Csv, Tsv, Json, Jsonl, Parquet }`. Detection order:
1. `--type` flag if given (accepts the format names above).
2. Else file extension (`.csv`, `.tsv`, `.json`, `.jsonl`/`.ndjson`,
   `.parquet`) — case-insensitive.
3. Else content sniffing (only reachable for real files, since stdin
   with no `--type` is a hard error — see below): Parquet magic bytes
   `PAR1` at file start; else first non-whitespace byte `[` → JSON
   array, `{` → try JSON array-of-one-object vs JSONL (if there's more
   than one line, treat as JSONL); else assume delimited text, sniff
   comma vs tab frequency in the first line.
4. If still undetermined → `DatagovError::UnsupportedInput` (exit 4).

Reading from stdin (`-`) with no `--type` → `DatagovError::InvalidArgs`
(exit 2), message: "--type is required when reading from stdin".

### Reader trait

One trait, one implementation per format, so `inspect` (and later
`profile`) share it:

```rust
pub trait DatasetReader {
    fn inspect(&self, source: &Source) -> Result<DatasetSection, DatagovError>;
}
```

`Source` wraps either a `PathBuf` or a boxed `Read` (stdin), plus the
resolved `Format`. Each format's reader:

- **CSV/TSV**: full scan (these are small fixtures; exact row count is
  fine to get by scanning — PRD's "without scanning all rows" language
  is specific to Parquet). Column type inference per column: try
  Boolean → Integer (i64) → Float (f64) → String, in that order, using
  the **narrowest type that fits every non-null value**; any parse
  failure at a stricter level falls back to the next; mixed-that-still-
  fails-String is impossible (String always fits) — never panic on bad
  data. Empty string counts as a null occurrence; `nullable` is true if
  any row's value for that column was empty. `approx_memory_bytes`:
  sum over every non-null value of (UTF-8 byte length + 24 for String
  columns; 8 for Integer/Float; 1 for Boolean; 1 for any null) — this
  is a real measurement from the same scan, not a guess.
- **JSON** (single array) / **JSONL** (one object per line): parse into
  `serde_json::Value`; column set = union of top-level object keys
  across all records; a field's `data_type` is its JSON value type
  (`Null` doesn't count toward inferring the type — skip nulls when
  determining type, but they do count toward `nullable`); if non-null
  occurrences disagree on type, use `DataType::Mixed`. `nullable` is
  true if any record omits the field or has it explicitly `null`.
  `approx_memory_bytes`: same formula as CSV applied to the parsed
  values (Object/Array count as 24 bytes flat, not recursively sized —
  keep this cheap and document the simplification).
- **Parquet**: row count, schema (names/types/nullability), and
  `approx_memory_bytes` **all come from file metadata** — no full row
  scan (this is the PRD-mandated case). `approx_memory_bytes` = sum of
  uncompressed byte size across all row groups from metadata.
  `ParquetInfo.row_groups` and `.compression_codecs` come from the same
  metadata. Only the bounded sample rows require actually reading data
  (via the row iterator, capped at `DEFAULT_SAMPLE_ROWS`).

### Sample rows

`const DEFAULT_SAMPLE_ROWS: usize = 5;` (no `--sample` flag in this
bolt — that arrives with `profile` in Bolt 3). Take the first 5 rows in
source order. Before inserting a column's value into a sample row's
JSON map, check `datagov_core::sensitivity::is_heuristically_sensitive`
on that column's name; if true, replace the value with the `Masked`
rendering (as a JSON string) instead of the raw value — for **every**
row, not just some.

## `datagov-cli`: `datagov inspect`

`datagov inspect <path|-> [--type <csv|tsv|json|jsonl|parquet>]`. Thin
adapter: resolve `Format` → construct `Source` → call the matching
`datagov-data` reader → wrap the resulting `DatasetSection` into the
Bolt 1 `ReportBuilder` (set `input.uri`, `input.format`,
`input.content_hash` via the existing `content_hash_sha256` helper for
real files; omit `content_hash` for stdin) → render.

- `--output json`: the full envelope.
- Human (table, via `comfy-table`): format, file size (human-readable,
  e.g. "482.9 KiB"), row count, column count; a table of columns
  (name/type/nullable); for Parquet, a row-group/compression summary
  line; a small sample-rows table (already masked, so this is safe to
  print as-is).
- Missing file → `DatagovError::InputNotFound` (exit 3), naming the
  path.
- Unsupported/undetected format → `DatagovError::UnsupportedInput`
  (exit 4), naming what was attempted.

## Tests (pin before/alongside code)

Golden tests, one per fixture format (`crates/datagov-cli/tests/
inspect.rs`, using `assert_cmd`): run `datagov inspect
examples/customers.<ext> --output json`, parse the envelope, assert
against a committed golden JSON snapshot under
`crates/datagov-cli/tests/golden/inspect_customers_<format>.json` —
**you are bootstrapping these golden files in this bolt**; generate
them from a correct run, review the values by hand against
`examples/README.md`'s stated row counts before committing them, then
assert future runs match. Normalize the `run` block (id/timestamps/
duration) out of the comparison, matching Bolt 1's schema-drift test
style. Also assert, per fixture: `row_count` matches the row count
documented in `examples/README.md`; `schema` contains `email` and
`phone` as columns; every `sample_rows` entry's `email` and `phone`
values are masked (never equal to a raw value from the source file).

Additional integration tests:

- `datagov inspect examples/does-not-exist.csv` → exit 3.
- `datagov inspect examples/README.md` → exit 4 (unsupported format).
- `datagov inspect - ` (no `--type`, piping anything) → exit 2.
- `cat examples/customers.jsonl | datagov inspect - --type jsonl
  --output json` → exit 0, envelope validates, `input.uri` present
  (document what you set it to for stdin — e.g. `"-"` — and note it in
  your report), no `content_hash`.
- Parquet-specific: `customers.parquet`'s envelope has
  `parquet.compression_codecs == ["SNAPPY"]` and at least one row
  group whose `row_count` sums (across all row groups) to the file's
  total row count.

Unit tests in `datagov-data`: format detection (extension, `--type`
override, content sniffing for at least one ambiguous case, stdin
without `--type` → error), CSV type-inference narrowing (a column of
all `"true"/"false"` → Boolean, a column mixing text and numbers →
String), JSON `Mixed` type detection.

## Report back

What was built per crate; any deviations as **Proposed Corrections**
with reasoning (e.g., if the `parquet` crate's uncompressed-size API
differs from what's described above); the three gate commands' final
output; total test count (unit + integration) and status; and the
actual row/column counts you observed for each `customers.*` fixture
(so the architect can sanity-check them against `examples/README.md`
without re-running everything).

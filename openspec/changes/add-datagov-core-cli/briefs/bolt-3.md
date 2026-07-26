# Bolt 3 implementation brief — `datagov profile` + `datagov query` (DataFusion)

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). Contract: implement exactly this; where reality forces a
> deviation, stop and report it as a proposed **Correction** — do not
> silently improvise. Read first: `AGENTS.md`, `openspec/project.md`,
> `openspec/changes/add-datagov-core-cli/{proposal,design,tasks}.md`,
> `openspec/changes/add-datagov-core-cli/specs/{dataset-profiling,
> file-query,cli-interface}/spec.md`, `docs/prd.md` §10.2, §10.3.
> This builds on Bolts 1–2 — read the existing `datagov-core` (report
> envelope, exit codes, mask, sensitivity, config) and `datagov-data`
> (format detection, `DatasetReader`, readers) source before writing
> anything; extend it, don't redefine it.

## Ground rules (same as Bolts 1–2)

- Do NOT run any git command. Leave the tree dirty for review.
- Do NOT edit anything under `openspec/` or `docs/prd.md`.
- Every source file opens with the AGENTS.md file-header comment.
- Diagnostics only via `tracing` to stderr.
- Done only when `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass, using the real `examples/`
  fixtures.

## Scope: DataFusion arrives in this bolt

Add `datafusion` to `[workspace.dependencies]` (current stable). This
pulls in Arrow transitively — add `arrow`/`arrow-array`/`arrow-schema`
as direct deps too, since converting DataFusion's `RecordBatch` results
into our JSON output needs them directly. **Check whether DataFusion's
internal `parquet`/`arrow` version coexists cleanly with Bolt 2's
directly-pinned `parquet = "59"`** (`cargo tree -p datagov-data -i
parquet` after wiring things up) — two versions of `parquet` in the
build graph is acceptable (they don't share types across our code), but
report the actual versions and any compile-time/binary-size impact you
observe as part of your final report, not just a pass/fail.

## `query` is CSV/Parquet only (PRD §10.3 is explicit about this)

PRD §10.3 lists only CSV and Parquet support for `query` — do not
extend to TSV/JSON/JSONL in this bolt. A query referencing any other
extension is `UnsupportedInput` (exit 4).

### Resolving quoted file paths in the SQL text

`datagov query "SELECT state, COUNT(*) FROM 'customers.parquet' GROUP BY state"`
— the FROM/JOIN target is a quoted file path, not a registered table
name. You need a mechanism that makes this work. Two acceptable
approaches, in order of preference:

1. If the pinned `datafusion` version has native support for treating
   a quoted string literal in table-factor position as a file
   reference (verify this actually works with a real query against a
   real fixture before relying on it — don't assume from memory), use
   it directly.
2. Otherwise: scan the SQL text with a regex for single-quoted string
   literals ending in `.csv` or `.parquet` (case-insensitive). For each
   **unique** match, register it as an external table (`ctx
   .register_csv(name, path, CsvReadOptions::new())` /
   `ctx.register_parquet(name, path, ParquetReadOptions::default())`)
   under a name derived from the file stem (sanitized to a valid SQL
   identifier — e.g. replace non-alphanumeric characters with `_`,
   prefix with `t_` if the result would start with a digit). Rewrite
   the query text, replacing each matched quoted-path literal with its
   registered (double-quoted, to be safe with case/special chars) table
   identifier, then execute the rewritten text via `ctx.sql(...)`.

Whichever you use, **report which one** and why in your final report —
this is an architecturally significant choice, not a minor detail.
Track every file path resolved this way; they become
`extensions.query.sources` in the envelope (see below).

### `datagov query "<sql>"` command

- `--limit <n>`: overrides the default bound. `const
  DEFAULT_QUERY_LIMIT: usize = 1000;` when not given.
- `--output json|table|csv` (this command needs a third rendering the
  other commands don't — see "Extending `--output`" below).
- Execute via DataFusion, materialize the full result (this bolt does
  not need streaming/pushdown optimization — document that as a known
  simplification, not silently), then apply the bound.
- **JSON** output: the full envelope, **no `input` section** (query
  can reference zero, one, or multiple files — there's no single
  dataset to describe there). Instead:
  ```rust
  // lives under extensions.query, not a new Sections field
  pub struct QueryResult {
      pub sources: Vec<String>,        // resolved file paths, in first-seen order
      pub columns: Vec<String>,
      pub rows: Vec<Vec<serde_json::Value>>, // bounded
      pub row_count_returned: u64,
      pub total_row_count: u64,        // full result size, pre-truncation
      pub truncated: bool,
      pub execution: QueryExecutionStats,
  }
  pub struct QueryExecutionStats { pub duration_ms: u64 }
  ```
  Define these in `datagov-data` (they're query-specific, not a shared
  governance section type like `DatasetSection`); the CLI serializes
  the result into `extensions.insert("query", serde_json::to_value(...))`.
- **table** output: `comfy-table` rendering of `columns` + the bounded
  `rows`.
- **csv** output: raw CSV to stdout — header + bounded rows, nothing
  else (no envelope wrapper; this must be pipeable).
- Invalid SQL → `DatagovError::InvalidArgs` (exit 2). A referenced file
  that doesn't exist → `DatagovError::InputNotFound` (exit 3). A
  referenced file with an unsupported extension → `UnsupportedInput`
  (exit 4).

### Extending `--output`

Bolt 1's `OutputFormat` enum (in `datagov-core::config` or wherever it
lives) currently has `Json`/`Table`. Add a `Csv` variant. Every
existing command (`version`, `capabilities`, `inspect`) must reject
`--output csv` explicitly with `DatagovError::InvalidArgs` (exit 2,
message naming which commands support CSV) — don't let it silently
fall through to table rendering. Only `query` accepts `Csv`.
`profile` (below) stays `Json`/`Table` only, per PRD §10.2 (no CSV
listed there).

## `datagov profile <path> [--columns a,b] [--sample N]`

Single-file command, same envelope shape as `inspect` (Bolt 2): set
`input.uri`/`input.format`/`input.content_hash`. Register the file as
a DataFusion table (share the registration helper with `query` — put
it in a common `datagov-data` module both commands use) and compute
statistics with DataFusion aggregate SQL (`COUNT`, `COUNT(DISTINCT
col)`, `MIN`, `MAX`, `AVG`, `STDDEV`, `APPROX_PERCENTILE_CONT(col, p)`
for p ∈ {0.25, 0.5, 0.75, 0.9, 0.99} — verify the exact function name
in the pinned DataFusion version, it may be `approx_percentile_cont`
or similarly named; report if it differs from this).

```rust
// replaces the Bolt 1 empty ProfileSection placeholder
pub struct ProfileSection {
    pub sample_size: Option<u64>, // Some(n) when --sample n was used, else None
    pub columns: Vec<ColumnProfile>,
}
pub struct ColumnProfile {
    pub name: String,
    pub data_type: DataType,      // reuse Bolt 2's enum, don't redefine
    pub row_count: u64,
    pub null_count: u64,
    pub null_percentage: f64,
    pub distinct_count: u64,
    pub uniqueness_percentage: f64,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub mean: Option<f64>,        // numeric columns only
    pub median: Option<f64>,      // numeric columns only
    pub stddev: Option<f64>,      // numeric columns only
    pub quantiles: Option<std::collections::BTreeMap<String, f64>>, // "p25".."p99", numeric only
    pub string_length: Option<StringLengthStats>, // string columns only
    pub top_values: Vec<TopValue>,  // masked if the column is sensitive
    pub semantic_type: String,      // see below — heuristic, not ML
    pub possible_identifier: bool,
}
pub struct StringLengthStats { pub min: u64, pub max: u64, pub mean: f64 }
pub struct TopValue { pub value: serde_json::Value, pub count: u64, pub percentage: f64 }
```

- `--columns email,state`: profile only the named columns; an unknown
  column name → `InvalidArgs` (exit 2) naming it.
- `--sample N`: profile only the **first N rows in source order**
  (deterministic — not a random sample; document this explicitly, it's
  what keeps the determinism requirement below satisfiable). Set
  `sample_size = Some(N)`.
- **Determinism**: profiling the same input with the same flags twice
  must produce byte-identical `profile` sections (envelope `run` block
  excepted) — this is a HARD requirement per the spec delta, not
  optional. Top-N-values queries need a stable tie-break (e.g. `ORDER
  BY count DESC, value ASC`) or repeated runs could reorder equal-count
  ties non-deterministically.
- `possible_identifier`: true when `distinct_count == row_count &&
  row_count > 0`, OR the column name matches `^(id|.*_id|.*id)$`
  (case-insensitive).
- `semantic_type`: heuristic only (deterministic-first — no ML/LLM).
  Reuse `datagov_core::sensitivity::is_heuristically_sensitive` for
  the obvious cases (name it, e.g. `"email"`, `"phone"` — whatever the
  matched pattern was) with a fallback of `"identifier"` (when
  `possible_identifier`), else a coarse bucket by `data_type`
  (`"numeric"`, `"boolean"`, `"text"`).
- **Masking**: `top_values` (and `min`/`max` if they'd expose a raw
  value) for a sensitive column go through `datagov_core::mask::Masked`
  exactly like Bolt 2's sample rows — same heuristic, same no-raw-value
  guarantee. This is a HARD requirement per the spec delta.

## Lightweight 1M-row timing check (SOFT — informational, not a gate)

Acceptance criterion 8 (SOFT) wants profiling 1M rows in under 10s,
measured and published eventually (full `benchmarks/` + criterion
tooling is Bolt 7's job, not this one). For Bolt 3, add **one**
`#[ignore]`d test in `datagov-data` (so it doesn't run in normal CI):
generate a synthetic 1M-row CSV in a `tempdir` (simple synthetic data —
a few numeric columns, one string column, no need to reuse the
`customers` schema), profile it, print the elapsed wall-clock time.
Don't assert a hard bound in code — this is informational; report the
actual number you observe in your final report along with the release
binary's size (`ls -la target/release/datagov`) so the architect can
compare against the pre-DataFusion Bolt 2 binary size.

## Tests (pin before/alongside code)

`crates/datagov-cli/tests/profile.rs`:
- Golden JSON envelope tests on `examples/customers.csv` and
  `examples/customers.parquet` (`run` block normalized, same style as
  Bolt 2's `inspect` goldens).
- Determinism: run profile twice on the same fixture, assert identical
  `profile` sections.
- Masking: `email`/`phone` never appear raw in `top_values`, `min`, or
  `max` — grep the full JSON output for a known raw fixture value.
- `--columns` restricts the output; unknown column → exit 2.
- `--sample 10` produces `sample_size: 10` and stats consistent with
  only the first 10 rows (verify against a hand-computed expectation
  for at least one column).

`crates/datagov-cli/tests/query.rs`:
- `datagov query "SELECT state, COUNT(*) AS total FROM
  'examples/customers.parquet' GROUP BY state" --output json` —
  correct aggregation, `extensions.query.sources ==
  ["examples/customers.parquet"]`, `execution.duration_ms` present.
- `--output csv` and `--output table` on the same query — CSV output
  is bare (no envelope) and parses as valid CSV with the right header.
- Bounded output: a query returning more than `DEFAULT_QUERY_LIMIT`
  rows is truncated with `truncated: true` and the correct
  `total_row_count`; `--limit` overrides it.
- Referencing a missing file → exit 3. Referencing `examples/
  customers.json` (JSON array, unsupported for query) → exit 4.
  Invalid SQL syntax → exit 2.
- `--output csv` on `datagov inspect` → exit 2 (rejected, not silently
  downgraded to table).

## Report back

What was built per crate; the DataFusion file-path-resolution approach
chosen and why; any deviations as **Proposed Corrections**; the three
gate commands' final output; total test count and status; the observed
1M-row profiling time and release binary size (with a before/after
comparison against Bolt 2's binary if you still have it, or note if
you don't); and confirmation of the `parquet` crate version-coexistence
check.

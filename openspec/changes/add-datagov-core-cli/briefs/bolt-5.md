# Bolt 5 implementation brief — PII recognizers + `datagov pii scan`

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). Contract: implement exactly this; where reality forces a
> deviation, stop and report it as a proposed **Correction** — do not
> silently improvise. Read first: `AGENTS.md`, `openspec/project.md`,
> `openspec/changes/add-datagov-core-cli/{proposal,design,tasks}.md`,
> `openspec/changes/add-datagov-core-cli/specs/pii-detection/spec.md`,
> `docs/prd.md` §10.8, §10.9, §25 (security/privacy requirements).
> This builds on Bolts 1–4 — read the existing `datagov-core` (report
> envelope, exit codes, `mask::Masked`, `sensitivity` heuristic) and
> `datagov-data`'s `engine.rs` (the shared DataFusion registration
> helper `query`/`profile` already use) before writing anything.

## Ground rules (same as Bolts 1–4)

- Do NOT run any git command. Leave the tree dirty for review.
- Do NOT edit anything under `openspec/` or `docs/prd.md`.
- Every source file opens with the AGENTS.md file-header comment.
- Done only when `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass.

## This is the masking-discipline bolt — read this section twice

`datagov-core::sensitivity::is_heuristically_sensitive` (Bolt 2) was
explicitly documented as a stand-in for **this** bolt's real
recognizer engine — it stays as-is for `inspect`/`profile`'s own
independent masking (don't touch it, don't make it depend on this
bolt's code). `pii scan`'s findings are the real thing: entity-typed,
confidence-scored, evidence-masked. **No raw matched value may appear
anywhere in `pii scan`'s output** — JSON, human table, `reason`
strings, error messages — under any circumstance, with or without a
`--fail-on` threshold. This is the bolt's single most important
property; violating it anywhere is worse than any other defect here.

## Scope: format support and the shared DataFusion engine

Reuse `datagov-data::engine`'s table-registration helper — do not
build a second file-reading path. Extend it if needed to also
register JSONL (DataFusion's JSON reader is natively NDJSON, i.e.
exactly our `.jsonl` format — `ctx.register_json`) and TSV (CsvReadOptions
with a tab delimiter, already how Bolt 2's delimited reader
distinguishes CSV/TSV). **`pii scan` supports CSV, TSV, JSONL, and
Parquet** — not plain JSON arrays (`.json`) in this bolt, since
DataFusion has no native single-array reader and none of PRD §10.8's
example invocations use `.json`. An unsupported format (including
`.json`) → `DatagovError::UnsupportedInput` (exit 4). Get column
values as strings via a `CAST(col AS VARCHAR)` projection (reuse
DataFusion for the cast, don't hand-roll Arrow-type-to-string
conversion) — this is genuinely simpler than parallel format-specific
string extraction and keeps one code path.

## New crate content: `datagov-pii` (currently an empty stub)

### Confidence model (document this exactly — it must be
deterministic and testable)

```text
confidence = clamp(base_confidence + validator_bonus + context_bonus, 0.0, 1.0)
```
- `base_confidence`: the recognizer's declared confidence (built-ins
  below specify their own; a custom recognizer's YAML `confidence`
  field).
- `validator_bonus` = +0.10 if the recognizer has a validator and
  **every** matched value in the column passes it; 0 otherwise (no
  partial credit — either fully validated or not).
- `context_bonus` = +0.05 if the column name case-insensitively
  contains any of the recognizer's `context` terms; 0 otherwise.

### Built-in recognizers — the 10 entities locked at the inception gate

| Entity | Pattern approach | Validator |
|---|---|---|
| `EMAIL_ADDRESS` | pragmatic RFC-5322-lite regex (document it's not fully compliant — deterministic-first, good-enough-for-real-data is the goal) | none |
| `PHONE_NUMBER` | North American Numbering Plan formats only (matches `examples/customers.*`'s `(664) 602-4412` style) — document international/E.164 as explicitly out of scope | none |
| `IP_ADDRESS_V4` | regex prefilter, then **validate by parsing with `std::net::Ipv4Addr`** (the parse *is* the validator — don't hand-write octet-range regex) | `std::net::Ipv4Addr::parse`|
| `IP_ADDRESS_V6` | same approach with `std::net::Ipv6Addr` | `std::net::Ipv6Addr::parse` |
| `URL` | regex prefilter; validator parses with the `url` crate (add as a dependency) and confirms a scheme + host | `url` crate |
| `US_SSN` | `\b\d{3}-\d{2}-\d{4}\b` (PRD §10.9's own example pattern) | reject known-invalid SSA area numbers (000, 666, 900-999) |
| `CREDIT_CARD` | 13-19 digit sequences (allow spaces/dashes as separators, strip before checking) | **Luhn checksum** |
| `UUID` | canonical 8-4-4-4-12 hex regex | check the version nibble (position 13) is 1-5 and the variant nibble (position 17) is 8/9/a/b — reduces false positives on generic hex strings |
| `MAC_ADDRESS` | colon- or hyphen-separated hex pairs (6 groups) | none needed — the format itself is specific enough |
| `US_ZIP_CODE` | 5-digit or ZIP+4 (`\d{5}(-\d{4})?`) | reject `00000` and a couple of other obviously-invalid placeholders if you find real test data needs it; keep this light |

### Scanning semantics

For each column (or the `--field` subset — see below), for each
recognizer: cast the column to string, scan every non-null scanned
value with the recognizer's regex using **all non-overlapping matches
per value** (`find_iter`, not a single whole-value match) — this
naturally covers both "this whole column is emails" and "this notes
field has an email embedded in prose" with one code path, per PRD
§10.8's `--field text` example. Apply the validator (if any) to each
matched substring. A **finding** is only emitted for a
(column, recognizer) pair with **at least one validated match** — do
not emit zero-match findings for every column×recognizer combination,
that's noise, not signal.

```rust
// replaces the Bolt 1 empty PiiSection placeholder in datagov-core
pub struct PiiSection {
    pub scanned_columns: u32,
    pub scanned_rows: u64,
    pub sample_size: Option<u64>,  // Some(n) when --sample n was used
    pub findings: Vec<PiiFinding>,
}
pub struct PiiFinding {
    pub column: String,
    pub entity: String,           // "EMAIL_ADDRESS", "US_SSN", ...
    pub recognizer: String,       // recognizer id
    pub confidence: f64,
    pub match_count: u64,         // rows with >= 1 validated match
    pub match_percentage: f64,    // match_count / scanned_rows_for_this_column * 100
    pub sample_evidence: Vec<String>,  // masked, up to 3 examples
    pub reason: String,           // concrete, e.g. "column name matches
                                   // context term 'ssn'; 12/50 values
                                   // (24.0%) matched US_SSN and passed
                                   // validation"
}
```

`sample_evidence` entries go through `datagov_core::mask::Masked` —
same type Bolt 2 already uses, no second masking implementation.

### `--sample N`

Same convention as `profile` (Bolt 3): first N rows in source order,
applied via the DataFusion query (`LIMIT`), deterministic.

### `--field <name>[,<name>...]`

Restricts scanning to named columns (comma-separated, like `profile`'s
`--columns`); default is all columns. An unknown field name →
`DatagovError::InvalidArgs` (exit 2), naming it.

### `--recognizers <path>`

Loads additional recognizers from the PRD §10.9 YAML schema (exact
shape — `id`, `entity`, `confidence`, `patterns`, `context`,
`validators`). Custom recognizers with an `id` matching a built-in
override it; otherwise they're added. Malformed regex in a pattern, a
`confidence` outside `[0.0, 1.0]`, or an unknown validator name → a
clear load-time error (`InvalidArgs`, exit 2) naming the recognizer id
and the specific problem.

### `--fail-on <confidence>`

A float threshold. If **any** finding's `confidence >= threshold`,
the process exits with code 12 (`PiiFailed`) **after** emitting the
full report (the report is never suppressed by a threshold failure).
Without this flag, `pii scan` always exits 0 regardless of findings —
it's informational by default, matching how quality/policy thresholds
work elsewhere in the PRD.

## `datagov pii recognizers list | validate <path>`

- `list`: enumerates the 10 built-ins (id, entity, confidence, pattern
  count) as a table or JSON envelope (under `extensions.pii_recognizers`,
  following the `query`/`sql`-result precedent of living under
  `extensions` rather than a governance `Sections` field, since this
  is metadata about the tool, not a scan result).
- `validate <path>`: structurally validates a recognizer YAML file per
  PRD §10.9 (regex compiles, confidence in range, validator names
  known). Exit 0 with a confirmation on success; exit 2 naming the
  offending recognizer id and field on failure — this is the exact
  scenario already written in the `pii-detection` spec delta.

## `datagov pii scan <path> [--sample N] [--field a,b] [--recognizers path] [--fail-on 0.x]`

Standard envelope wiring, same as `inspect`/`profile`: `input.uri`,
`input.format`, `input.content_hash`. `pii` section populated per
above. Human-readable rendering: a table of findings
(column/entity/confidence/match%/masked evidence), never raw values.

## Fixtures

`examples/customers.*` (already committed) naturally covers
`EMAIL_ADDRESS` and `PHONE_NUMBER` — use it for those. The other 8
entities need a **new, clearly-synthetic fixture** —
`examples/pii-fixture.csv` (or similar; your naming) with realistic-
shaped but obviously-fake values: use the well-known industry-standard
Luhn-valid test credit card numbers (e.g. `4111111111111111`,
`5555555555554444` — these are universally recognized sandbox test
numbers, safe to use, not real cards), clearly-placeholder SSNs
outside any real assigned range, real (but generic/example) IPv4/IPv6
addresses (e.g. documentation ranges like `192.0.2.0/24` / `2001:db8::/32`
are IANA-reserved specifically for this purpose — prefer them), a
generated UUID, a MAC address, a URL, and a US ZIP. Document the
fixture's provenance and entity coverage in `examples/README.md`,
matching the existing documentation convention for `examples/`.

## Tests

`crates/datagov-cli/tests/pii.rs`:
- **The masking eval (HARD, this bolt's top priority)**: run
  `pii scan` against `examples/customers.parquet` and
  `examples/pii-fixture.csv`, capture the complete JSON output, human
  output, and any error output, and grep all of it for every known
  raw fixture value (the real emails/phones from `customers.*`, the
  test credit card numbers, the SSN, etc.) — zero hits, no exceptions.
- Per-entity detection: each of the 10 entities produces at least one
  finding on the appropriate fixture, with `confidence`, `match_count`,
  `match_percentage` all present and sane (e.g. confidence in
  `[0,1]`, match_percentage in `[0,100]`).
- Confidence model: a golden test pinning the exact confidence value
  for at least 2 findings (verify your `base + bonus` arithmetic
  produces what you documented).
- `--sample N` restricts scanning; `--field` restricts columns; unknown
  field → exit 2.
- `--recognizers`: a custom recognizer overrides a built-in by id; a
  malformed recognizer file → exit 2 naming the bad id/field.
- `--fail-on`: a threshold below an actual finding's confidence → exit
  12; a threshold above all findings → exit 0. Report is present in
  both cases.
- `pii recognizers list` and `validate` (valid + invalid file cases).
- Missing input file → exit 3; unsupported format (`.json`) → exit 4.

## Report back

What was built per crate; the exact confidence-model arithmetic
verified against your golden tests; any deviations as **Proposed
Corrections**; the masking-eval result explicitly (this is the one
thing that must not have any caveats); the three gate commands' final
output; total test count and status; and the fixture entity-coverage
table (which entity, which fixture file, whether detection succeeded
cleanly).

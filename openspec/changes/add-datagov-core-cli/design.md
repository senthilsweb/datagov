# Design — `add-datagov-core-cli`

> Per PRD §21–§24. Engine decisions locked at the inception gate
> (owner, 2026-07-26) — see the resolved questions in proposal.md. The
> planned Q1/Q2 spikes were waived by owner decision; the Bolt 4
> conformance corpus and Bolt 3 benchmarks serve as the after-the-fact
> evidence checks.

## Workspace

```
crates/
├── datagov-cli/      # clap command tree; thin adapters only — parses
│                     #   flags, calls a domain crate, renders the envelope
├── datagov-core/     # report envelope (serde + JSON Schema), exit codes,
│                     #   config resolution (PRD §28), tracing setup,
│                     #   masking utilities (single implementation)
├── datagov-data/     # format detection, readers (csv / json / jsonl /
│                     #   parquet), inspection, profiling, file query —
│                     #   engine: DataFusion (locked 2026-07-26)
├── datagov-sql/      # parse / format / transpile —
│                     #   engine: sqlglot-rust (locked 2026-07-26)
├── datagov-pii/      # recognizer registry, validators (Luhn, ABA,
│                     #   mod-97, SSN), YAML recognizer loader, scanner
└── datagov-report/   # consolidation + YAML/Markdown/terminal renderings
```

Rules:

- No business logic in `datagov-cli` — every command body is
  parse-flags → call-crate → render. Domain crates are usable as
  libraries (the future `datagovd`/MCP surfaces reuse them unchanged).
- Masking lives in `datagov-core` and is the **only** path by which a
  sampled value reaches any output. There is no unmasked accessor on the
  evidence type.
- The envelope is constructed by `datagov-core::Report` builder; commands
  populate domain sections (`dataset`, `profile`, `pii`, …) with typed
  structs, never raw `serde_json::Value`.

## Report envelope and exit codes

- Envelope exactly per PRD §23; `schema_version: "1.0"`. JSON Schema
  generated from the serde types (schemars) and committed at
  `docs/schema/report-v1.json`; a test fails if the generated schema
  drifts from the committed one.
- Exit codes: a single `ExitCode` enum in `datagov-core` mirroring PRD
  §24; `main` maps every error path through it. Clap usage errors map
  to 2; missing inputs to 3; unsupported format/dialect to 4; PII
  threshold to 12.

## Key decisions (locked at the inception gate, 2026-07-26)

| Decision | Resolution |
|---|---|
| SQL engine | **sqlglot-rust** — Bolt 4 conformance corpus is the evidence check; fallback via Correction + ADR if coverage fails |
| Profiling/query engine | **DataFusion** — binary-size impact measured with Bolt 3 benchmarks |
| `query` command in 0.1 | **In scope** (proposal revision 1) — bounded output, `--limit`, execution stats |
| PII entity set for 0.1 | email, phone, IPv4, IPv6, URL, US_SSN, credit card (Luhn), UUID, MAC, US ZIP |
| Arrow IPC | Deferred to the quality/schema change |
| Report run id | UUIDv7 `run.id` + SHA-256 `input.content_hash` |
| License | Apache 2.0 (LICENSE replaced at the gate) |
| Release versioning | Manual `v*` tags for 0.1; release-plz revisited later |

**Correction (2026-07-26, Bolt 1 Construction):** `schemars` pinned to
**0.8** (resolved 0.8.22), not 1.x — the 1.x line changes the schema
generation API and default JSON Schema draft, and the committed
`docs/schema/report-v1.json` is load-bearing (drift test + published
with releases). Migrating to 1.x is a deliberate future change, not a
silent dependency bump. Flagged by the implementation agent; accepted
by the architect.

**Correction (2026-07-26, Bolt 2 Construction):** Parquet sample rows
are produced via the `parquet` crate's own `Row::to_json_value()`
(feature `json`, added alongside `snap` in the workspace `Cargo.toml`)
rather than a hand-written `RowAccessor` physical-type dispatch — it's
the crate's supported JSON rendering, already the exact shape
`sample_rows` needs, and avoids duplicating the schema/type mapping
already derived from metadata. `RowAccessor` is still exercised
directly in the CLI integration test's independent oracle for the
masking assertion. Flagged by the implementation agent; accepted by
the architect.

**Correction (2026-07-26, Bolt 2 Construction):** content-sniffing's
comma-vs-tab tie-break (including 0-vs-0, e.g. prose with neither) is
"still undetermined" → `UnsupportedInput` (exit 4) — this is what
correctly routes `examples/README.md` to the unsupported-format test
rather than a silent CSV misclassification. The brief named the
comparison but not the tie-break; this is the natural reading.
Accepted by the architect.

**Correction (2026-07-26, Bolt 2 Construction):** Parquet's
`BYTE_ARRAY`/`FIXED_LEN_BYTE_ARRAY` physical types both map to
`DataType::String` — Bolt 2's `DataType` enum has no binary/decimal/
date variant. Correct for the committed fixture (all VARCHAR columns);
not yet exercised against a decimal- or date-typed Parquet column.
Revisit when a fixture with those types is introduced. Accepted by the
architect.

**Correction (2026-07-26, Bolt 3 Construction):** DataFusion's
`AVG`/`STDDEV`/`APPROX_PERCENTILE_CONT` aggregates are not perfectly
bit-reproducible run-to-run on an unchanged input (streaming
floating-point accumulation, non-associative addition) — observed
differences in the last 1-2 ULPs even with a single target partition.
`profile.rs::round_stat` rounds `mean`/`median`/`stddev`/`quantiles`/
`string_length.mean` to 9 decimal places before they enter the
envelope, which absorbs the noise; verified stable across 20+ repeated
runs. The brief's "byte-identical envelopes" requirement holds at this
precision, not at full `f64` precision — a correction to an implicit
assumption in the brief, not a spec violation (9 decimal places is far
beyond any real governance use of these statistics). Accepted by the
architect; independently re-verified (two runs of `datagov profile
examples/customers.parquet`, `run` block excepted, byte-identical).

**Decision (2026-07-26, Bolt 3 Construction):** for `datagov query`,
quoted file-path references in the SQL text are resolved via an
explicit regex scan + registration (design's "approach 2"), **not**
DataFusion 54.1.0's native `enable_url_table()` — verified working,
but it silently pulls in JSON datasource support, which would let a
query against `examples/customers.json` (a JSON *array*, explicitly
out of scope per PRD §10.3) succeed instead of failing with
`UnsupportedInput`. The explicit scan is what enforces the CSV/Parquet-
only contract with the correct exit codes (3 missing / 4 unsupported)
before execution. Architect-verified: `query` against
`examples/customers.json` correctly exits 4.

**Correction (2026-07-26, Bolt 4 Construction — inception gate Q1
outcome):** the Phase-1 verification the brief required actually
happened, retroactively covering the spike the owner waived at the
inception gate. **`sqlglot-rust` v0.10.23** (crates.io,
`protegrity/sql-glot-rust`) is real, maintained, and confirmed working
across all 11 priority dialects — no fallback to `sqlparser-rs`
needed, no ADR required. Architect-verified independently: a genuine
dialect transform (T-SQL `TOP 10` → Postgres `LIMIT 10`, not a literal
passthrough) and all 5 committed transpile golden pairs byte-match a
fresh build.

**Correction (2026-07-26, Bolt 4 Construction):** the first
`transpile()` implementation called the crate's `generate()` on the
already-parsed AST directly, which skips the crate's own
`dialects::transform` step — the part that actually rewrites
`TOP n` ↔ `LIMIT n` ↔ `FETCH FIRST n ROWS ONLY` and similar
dialect-specific rewrites. Caught by the implementer's own stdin
integration test before this reached review. Fixed by calling the
crate's `transpile()` entry point for the output SQL (accepting one
extra re-parse), while still using the separately-parsed pre-transform
AST for lossy-construct warning detection. A regression test pins this.

**Decision (2026-07-26, Bolt 4 Construction):** `sql format`'s
`--output json` envelope shape (`extensions.sql_format: {dialect,
output_sql, written}`) and putting `input.uri`/`content_hash` on
`parse`/`format`/`transpile` (unlike `query`, which omits `input`
since it can span zero-to-many files) both follow `inspect`/`profile`'s
existing precedent, not `query`'s. Transpile warnings render to stderr
on the human path (stdout stays pipeable, matching `query`'s CSV-bare
precedent) — architect-verified: `stdout` is exactly the transpiled
SQL, `stderr` carries the warning, on the same lossy `QUALIFY` case
used for the golden test. Accepted.

**Known limitation (2026-07-26, Bolt 4 Construction — not a datagov
defect, documented for transparency):** `sqlglot-rust` has its own
parser/generator fidelity gaps independent of any dialect target —
observed even regenerating in the *same* source dialect: Spark
`LATERAL VIEW EXPLODE(...) AS alias` loses the column alias
(architect-reproduced: `AS tag` vanishes, the clause is even rewritten
to a `CROSS JOIN` form); SQLite `GLOB` is silently coerced to `LIKE`
(changes case-sensitivity/wildcard semantics); SQLite
`INSERT OR REPLACE` loses its conflict clause; `UNNEST(...) AS
t(cols...)` loses its column-alias list. None of these constructs are
used in the committed corpus. `datagov-sql::warnings` implements its
own small, explicitly non-exhaustive capability matrix for
`QUALIFY`/`PIVOT`/`UNPIVOT` since the crate has no general warnings API
of its own (only a date/time-format-specific one) — this covers this
bolt's corpus, not a claim of complete dialect-compatibility knowledge.
Revisit if a future change needs any of the excluded constructs.

## Fixtures and evals

**Decision (2026-07-26, pre-Bolt-2 planning):** rather than a
from-scratch generator script, `examples/customers.*` is sourced from
the owner's own public synthetic-dataset repos (a TICKIT-shaped users
table — synthetic identities, Lorem-Ipsum content, no real people) and
committed across all five Milestone-0.1 formats — CSV, TSV, JSON,
JSONL, and the original Parquet file (full row count, real Snappy
compression/row-group metadata). See `examples/README.md` for exact
provenance, row counts, and regeneration commands. `claimwise-*.csv`
(synthetic healthcare RCM data with cross-file foreign keys) is also
committed now for later lineage/quality bolts, out of scope for 0.1.

- `examples/` — the `customers.*` fixture set above, plus a SQL
  conformance corpus `examples/sql/<dialect>/*.sql` (still to author in
  Bolt 4).
- Golden tests snapshot the JSON envelope (with `run` block normalized)
  for inspect/profile/report on the fixtures.
- The masking eval greps the complete output surface (stdout, stderr,
  report files) of a `pii scan` run for every known fixture value — any
  hit is a HARD failure.
- Benchmarks under `benchmarks/` produce the numbers for acceptance
  criterion 8 and are published in release notes.

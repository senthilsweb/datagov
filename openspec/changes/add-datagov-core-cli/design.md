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

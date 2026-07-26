# Proposal: Add `datagov` Core CLI (Milestone 0.1)

> Status: **APPROVED** — 2026-07-26. Inception gate passed: all eight
> open questions resolved by the owner (see below). Construction starts
> with Bolt 1.
> Revision: 1 (owner, 2026-07-26: **`datagov query` pulled into scope**
> — with DataFusion locked as the data engine (Q2), the embedded SQL
> engine comes at near-zero marginal cost, so the PRD §38.6 question
> resolves to "include": bounded output defaults and explicit `--limit`
> from day one. Same gate: Q1/Q2 resolved by **owner decision without
> the planned spikes** — sqlglot-rust and DataFusion chosen directly;
> the Bolt 4 dialect-conformance corpus remains the evidence check, and
> if sqlglot-rust coverage fails it, the fallback is logged as a dated
> Correction + ADR, not a silent swap. License switched from MIT to
> Apache 2.0 per PRD §39.)
> (PROPOSED 2026-07-26)
> Owner: @senthilsweb
> Source: `docs/prd.md` §37 (Milestone 0.1), §33 (Phase 1 priorities)

## Why

The PRD's core premise — *"a useful DataGovOps workflow can be delivered
as one portable, deterministic, agent-ready executable"* — is unproven
until a user can download one binary and run inspect → profile → sql →
pii scan → report on a real dataset with no Python, Java, Node.js,
Docker, or Cargo installed. Milestone 0.1 is deliberately narrower than
the full Core phase: it validates the premise, the report envelope, the
exit-code contract, and the cross-platform release pipeline before the
remaining Core capabilities (quality, schema, policy) build on them.

## What changes

Stand up the Rust workspace and deliver the Milestone 0.1 command
surface, exactly as scoped in PRD §37:

- **Workspace + CLI frame** — `crates/` workspace (`datagov-cli`,
  `datagov-core`, `datagov-data`, `datagov-sql`, `datagov-pii`,
  `datagov-report` for this milestone), `clap`-based
  `datagov <domain> <action> [target] [flags]` surface, global
  `--output json|table`, `--quiet`, `--verbose`, stdin (`-`) where
  applicable, `tracing` diagnostics on stderr.
- **Normalized report envelope + exit codes** — the versioned JSON
  envelope (PRD §23) and the exit-code table (PRD §24) implemented in
  `datagov-core` and used by every command from day one. The envelope's
  JSON Schema is committed and published with releases.
- **`datagov inspect`** — CSV, TSV, JSON, JSONL, Parquet (Arrow IPC per
  open question 4): format detection, size, row/column counts, schema,
  types, nullability, Parquet row groups/compression, masked sample rows.
- **`datagov profile`** — per-column statistics per PRD §10.2 (nulls,
  distinct, min/max/mean/median/stddev, quantiles, string lengths, top
  values, inferred semantic type, possible identifiers), `--columns`,
  `--sample`.
- **`datagov query`** *(revision 1)* — SQL over local CSV/Parquet files
  through the embedded DataFusion engine (PRD §10.3): JSON/table/CSV
  output, bounded output by default, explicit `--limit`, query execution
  statistics in the envelope.
- **`datagov sql parse | format | transpile`** — PRD §10.4–10.6 over the
  priority dialects (ANSI, PostgreSQL, DuckDB, Spark, Databricks,
  Snowflake, BigQuery, Trino, MySQL, SQLite, T-SQL — final list confirmed
  by the Q1 spike). `format` writes to stdout unless `--write`.
  Unsupported/lossy transpilations are reported explicitly, never silent.
- **`datagov pii scan`** — deterministic engine per PRD §10.8: the
  initial entity list (final mandatory set per open question 3) detected
  via column names, sampled values, regex, checksums (Luhn, ABA, IBAN
  mod-97, SSN structure), and context terms; user-defined recognizers via
  the PRD §10.9 YAML (`pii recognizers list|validate`). Every finding
  carries entity, column, confidence, recognizer, masked evidence, match
  count/percentage, and reason. **Raw values never appear in output.**
- **`datagov report`** — consolidated run of profile + pii over one
  dataset into the single envelope (JSON canonical; YAML, Markdown,
  terminal summary renderings).
- **`datagov version` / `capabilities` / `doctor`** — trivial once the
  frame exists; `capabilities` lists compiled features and formats.
- **Release automation** — GitHub Actions: CI (fmt, clippy, test) on
  every push/PR; tag-driven release building `datagov-darwin-arm64` and
  `datagov-linux-x86_64` (required), remaining PRD §29 targets
  best-effort, each release carrying an SPDX SBOM and SHA-256 checksums.
  The workflows are committed with this scaffolding; the release path is
  proven in the final bolt.

## Out of scope (this change)

Everything PRD §37 excludes from 0.1: anonymization, Presidio, dbt,
OpenLineage, plugins, MCP, remote execution, catalogs — plus the rest of
Core that 0.1 defers: `quality check`, `schema infer|validate|diff`,
`policy check`, `sql lineage`, and Homebrew/install-script distribution
(GitHub Releases only for 0.1). ~~`datagov query` is in/out per open
question 5~~ — **in scope per revision 1.**

## Acceptance criteria

1. On a clean macOS ARM64 or Linux x86-64 machine with no Rust, Python,
   Java, Node.js, or Docker, the downloaded binary runs
   `datagov inspect examples/customers.parquet` successfully.
2. `inspect` and `profile` produce stable, schema-valid JSON envelopes
   for the committed CSV and Parquet example datasets (golden tests).
3. `sql transpile` round-trips the committed dialect-conformance corpus
   for the priority dialects; lossy or unsupported constructs produce an
   explicit warning in the envelope, never silent output (HARD).
4. `pii scan` detects every entity type in the final mandatory list on
   the synthetic fixture dataset, and no raw matched value appears
   anywhere in the report, logs, or error output — enforced by an eval
   that greps the full output surface for the known fixture values (HARD).
5. Every command supports `--output json` and returns the versioned
   envelope; `schema_version` validates against the committed JSON Schema.
6. Every documented exit code in PRD §24 that applies to 0.1 commands
   (0, 1, 2, 3, 4, 12) is produced by an integration test.
7. `pii scan --fail-on <threshold>` (or equivalent documented flag)
   makes a CI job fail with exit code 12 — demonstrated in a workflow.
8. Startup (`datagov version`) completes in <150 ms; profiling 1M rows
   of the benchmark schema completes in <10 s on a modern laptop
   (SOFT — measured and published, misses reviewed not blocking).
9. A `v0.1.0` tag produces a GitHub Release with the two required
   binaries, SPDX SBOM, and checksums, built entirely by CI.
10. `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test`
    pass in CI on every bolt merge.
11. *(revision 1)* `datagov query "SELECT state, COUNT(*) FROM
    'examples/customers.parquet' GROUP BY state"` returns correct
    results in JSON, table, and CSV renderings; output is bounded by
    default; `--limit` is honoured; the envelope carries execution
    statistics (HARD).

## Open questions for the inception gate

All resolved 2026-07-26 — gate passed.

1. ~~**SQL engine** (PRD §38.1)~~ — **Resolved (owner, 2026-07-26):**
   **`sqlglot-rust` directly**, chosen without the planned spike. The
   Bolt 4 dialect-conformance corpus stays in place as the evidence
   check; if coverage fails it, the fallback (`sqlparser-rs` or hybrid)
   is adopted via a dated Correction + ADR.
2. ~~**Profiling engine** (PRD §38.2)~~ — **Resolved (owner,
   2026-07-26):** **DataFusion directly** (PRD §39 default), chosen
   without the planned spike. Binary-size impact is measured and
   published with the Bolt 3 benchmarks instead.
3. ~~**Mandatory PII entities for 0.1**~~ — **Resolved (owner,
   2026-07-26):** the recommended subset — **email, phone, IPv4, IPv6,
   URL, US SSN, credit card (Luhn), UUID, MAC, US ZIP**. DOB candidates,
   IBAN (mod-97), and ABA routing follow in the next Core change.
4. ~~**Arrow IPC in 0.1?**~~ — **Resolved (owner, 2026-07-26):**
   **deferred.** 0.1 formats are CSV/TSV/JSON/JSONL/Parquet; Arrow IPC
   arrives with the quality/schema change.
5. ~~**Include `datagov query` in 0.1?**~~ — **Resolved (owner,
   2026-07-26):** **included** (revision 1) — DataFusion makes the
   engine free; bounded output + `--limit` from day one.
6. ~~**Report IDs**~~ — **Resolved (owner, 2026-07-26):** **UUIDv7 for
   `run.id`** (time-ordered, unique per run) **plus SHA-256
   `input.content_hash`** for reproducibility/provenance.
7. ~~**License**~~ — **Resolved (owner, 2026-07-26):** **Apache 2.0**,
   switched in this same gate (LICENSE replaced; was MIT from repo
   initialization).
8. ~~**Release versioning**~~ — **Resolved (owner, 2026-07-26):**
   **manual `v*` tags for 0.1** driving the committed release workflow;
   release-plz automation revisited as its own change once the
   workspace stabilizes.

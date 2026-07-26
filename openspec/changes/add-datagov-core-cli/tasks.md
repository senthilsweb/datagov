# Tasks — `add-datagov-core-cli`

Bolts follow AI-DLC: evals pinned before or alongside the code they gate.
Nothing starts until the proposal's open questions are resolved and status
moves to **APPROVED**.

## Bolt 0 — Inception gate ✅ (passed 2026-07-26)

- [x] ~~Spike: SQL engine (Q1)~~ — **waived by owner decision
      (2026-07-26): sqlglot-rust chosen directly.** The Bolt 4
      conformance corpus remains the evidence check; fallback via
      Correction + ADR if coverage fails
- [x] ~~Spike: profiling engine (Q2)~~ — **waived by owner decision
      (2026-07-26): DataFusion chosen directly.** Binary-size impact
      measured with the Bolt 3 benchmarks
- [x] Resolve Q3 — **resolved by owner (2026-07-26):** subset — email,
      phone, IPv4, IPv6, URL, US SSN, credit card, UUID, MAC, US ZIP
- [x] Resolve Q4 — **resolved by owner (2026-07-26):** Arrow IPC
      deferred to the quality/schema change
- [x] Resolve Q5 — **resolved by owner (2026-07-26):** `datagov query`
      in scope (proposal revision 1)
- [x] Resolve Q6 — **resolved by owner (2026-07-26):** UUIDv7 run.id +
      SHA-256 content hash
- [x] Resolve Q7 — **resolved by owner (2026-07-26):** Apache 2.0;
      LICENSE replaced at the gate
- [x] Resolve Q8 — **resolved by owner (2026-07-26):** manual `v*` tags
      for 0.1
- [x] Proposal status → **APPROVED** (2026-07-26)

## Bolt 1 — Workspace skeleton, envelope, exit codes

- [ ] Cargo workspace + the six milestone crates; `datagov version`
      and `datagov capabilities` working end-to-end
- [ ] `datagov-core`: report envelope types + schemars JSON Schema
      committed at `docs/schema/report-v1.json` + drift test
- [ ] `datagov-core`: `ExitCode` enum per PRD §24; clap error mapping
      (2/3/4) covered by integration tests
- [ ] `tracing` subscriber (stderr, JSON lines under `--verbose`,
      silent under `--quiet`); global `--output` plumbing
- [ ] Config resolution chain per PRD §28 with unit tests
- [ ] File-header convention applied; CI (fmt, clippy, test) green

## Bolt 2 — `inspect` + fixtures

- [ ] Synthetic fixture generator + committed `examples/customers.csv`
      / `.parquet` (fictional identities only)
- [ ] Format detection (extension + content sniffing); CSV/TSV/JSON/
      JSONL/Parquet readers behind one trait
- [ ] `inspect`: schema, counts, types, nullability, Parquet row
      groups/compression, masked samples; stdin via `-` with `--type`
- [ ] Golden tests for JSON envelopes on both fixtures; exit code 3/4
      integration tests

## Bolt 3 — `profile` + `query` (DataFusion)

- [ ] Column statistics per PRD §10.2 on DataFusion;
      `--columns`, `--sample`
- [ ] Semantic-type inference + possible-identifier flagging
- [ ] Golden tests; determinism eval (two runs → byte-identical
      envelopes modulo `run` block)
- [ ] 1M-row benchmark wired into `benchmarks/` (SOFT criterion 8);
      binary-size impact of DataFusion recorded alongside
- [ ] `datagov query` *(revision 1)*: SQL over local CSV/Parquet,
      JSON/table/CSV renderings, bounded output by default, `--limit`,
      execution statistics in the envelope
- [ ] Golden tests for `query` incl. bounded-output and `--limit`
      behaviour; exit code 4 on unsupported input

## Bolt 4 — `sql parse | format | transpile`

- [ ] Dialect conformance corpus committed under `examples/sql/`
      (per-dialect statements + expected transpilations)
- [ ] `parse`: statement type, tables, columns, joins, filters, CTEs,
      normalized AST in the envelope
- [ ] `format`: stdout by default; file modified only with `--write`
      (HARD eval: no `--write`, no mtime change)
- [ ] `transpile` across the confirmed dialect list; lossy/unsupported
      constructs reported explicitly in the envelope (HARD)
- [ ] Exit code 4 on unsupported dialect; corpus round-trip suite green

## Bolt 5 — `pii scan` + recognizers

- [ ] Recognizer registry: built-ins for the confirmed entity set
      (regex + validators: Luhn, SSN structure, and the rest per Q3)
- [ ] Context-term and column-name scoring; confidence model documented
      in the spec
- [ ] Recognizer YAML loader + `pii recognizers list|validate`
      (PRD §10.9 schema published)
- [ ] Masked evidence type in `datagov-core` — no unmasked accessor
- [ ] HARD masking eval: full output surface greps clean of every
      fixture value; per-entity detection eval on the fixture set
- [ ] `--fail-on` threshold → exit code 12, integration-tested

## Bolt 6 — `report` + `doctor`

- [ ] `report`: consolidated profile + pii run into one envelope;
      YAML / Markdown / terminal renderings derived from the JSON
- [ ] `doctor`: version, OS/arch, memory, format support, config
      validity
- [ ] Golden test: consolidated report on the Parquet fixture matches
      the PRD §36 demo shape

## Bolt 7 — Release proving + verification

- [ ] CI release path proven: `v0.1.0-rc` tag → GitHub Release with
      `datagov-darwin-arm64` + `datagov-linux-x86_64`, SPDX SBOM,
      SHA-256 checksums (remaining PRD §29 targets best-effort)
- [ ] Clean-machine eval: binaries run the §36 demo sequence on hosts
      with no toolchains installed
- [ ] Startup benchmark <150 ms measured and published (SOFT)
- [ ] Demo workflow: a sample GitHub Actions job failing on a PII
      threshold (acceptance criterion 7)
- [ ] Full HARD sweep green; SOFT observations reviewed and logged
- [ ] Proposal status → **IMPLEMENTED**, then **VERIFIED**; archive the
      change and merge spec deltas into `openspec/specs/`

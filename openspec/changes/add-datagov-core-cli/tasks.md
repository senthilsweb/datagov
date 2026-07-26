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

## Bolt 1 — Workspace skeleton, envelope, exit codes ✅ (2026-07-26)

Built by a Sonnet 5 agent from `briefs/bolt-1.md`; architect-reviewed
(gates re-run independently, envelope/mask/exit spot-checked, binary
smoke-tested). 30 tests green (23 unit + 7 integration).

- [x] Cargo workspace + the six milestone crates; `datagov version`
      and `datagov capabilities` working end-to-end
- [x] `datagov-core`: report envelope types + schemars JSON Schema
      committed at `docs/schema/report-v1.json` + drift test.
      **Correction:** schemars pinned 0.8, not 1.x (see design.md)
- [x] `datagov-core`: `ExitCode` enum per PRD §24 (all 13 discriminants
      pinned by unit test); clap error mapping — 2 integration-tested;
      3/4 unit-tested via `DatagovError::exit_code()`, integration
      coverage lands with `inspect` in Bolt 2 (no file commands exist yet)
- [x] `tracing` subscriber (stderr-only, JSON lines under `--verbose`,
      ERROR-only under `--quiet`, `DATAGOV_LOG` override); global
      `--output` plumbing (quiet/verbose conflict → exit 2)
- [x] Config resolution chain per PRD §28 with 10 unit tests (each
      layer, override order, malformed YAML → exit 2)
- [x] File-header convention applied; CI (fmt, clippy, test) green

## Bolt 2 — `inspect` + fixtures ✅ (2026-07-26)

Built by a Sonnet 5 agent from `briefs/bolt-2.md`; architect-reviewed
(gates re-run independently, masking verified by grepping full
JSON+human output for raw fixture values, all five formats and both
error-path exit codes manually smoke-tested). 64 tests green (46 unit +
18 integration, cumulative with Bolt 1).

- [x] Fixtures committed at `examples/` — `customers.{csv,tsv,json,
      jsonl,parquet}` (one logical TICKIT-shaped dataset across all
      five formats, sourced from the owner's public dataset repos, see
      `examples/README.md`) plus `claimwise-*.csv` for later bolts
- [x] Format detection (extension + `--type` override + content
      sniffing: Parquet magic, JSON array/object heuristics, comma-vs-
      tab frequency with tie → `UnsupportedInput`, see design.md
      Correction); CSV/TSV/JSON/JSONL/Parquet readers behind one
      `DatasetReader` trait
- [x] `inspect`: schema, counts, types (Boolean→Integer→Float→String
      narrowing for delimited/JSON; metadata-derived for Parquet),
      nullability, Parquet row groups/compression (from metadata, no
      full scan), masked sample rows (heuristic sensitive-column list
      in `datagov-core::sensitivity`, pre-dating Bolt 5's recognizer
      engine); stdin via `-` with `--type` (required — hard exit 2
      otherwise)
- [x] Golden tests for JSON envelopes on all five `customers.*`
      fixtures (`run` block normalized); exit code 2/3/4 integration
      tests

## Bolt 3 — `profile` + `query` (DataFusion) ✅ (2026-07-26)

Built by a Sonnet 5 agent from `briefs/bolt-3.md`; architect-reviewed
(gates re-run independently, hands-on tested against real fixtures:
profile human/JSON output, masked top-values, `--sample`/`--columns`,
determinism across two runs, query aggregation/CSV output/bounded
truncation/`--limit`, all four query exit paths, and the CSV-rejection
guard on `inspect`/`version`). 100 tests green (91 run + 1 informational
`#[ignore]`, cumulative with Bolts 1-2), 0 clippy warnings.

- [x] Column statistics per PRD §10.2 on DataFusion (count, nulls,
      distinct, min/max, mean/median/stddev/quantiles for numeric,
      string-length stats, top-values); `--columns`, `--sample N`
      (first N rows, deterministic — verified: `--sample 10` on
      `customers.csv` gives min=1/max=10, not a random subset)
- [x] Semantic-type inference + possible-identifier flagging (verified:
      `phone` in `customers.parquet` — 100% distinct — flagged
      `possible_identifier: true`; `email` — 99.9% distinct — correctly
      not flagged, still semantic_type `"email"` via the heuristic)
- [x] Golden tests; determinism eval. **Correction:** DataFusion's
      float aggregates aren't bit-reproducible run-to-run; results are
      rounded to 9 decimal places before entering the envelope (see
      design.md). Independently re-verified: two full runs on
      `customers.parquet`, `run` block excepted, byte-identical.
- [x] Lightweight informational 1M-row timing check (Bolt 7 owns the
      real `benchmarks/` + criterion pass): ~450ms observed, well under
      the 10s SOFT target. Release binary is now 135.2 MiB (up from a
      pre-DataFusion baseline in the low tens of MB) — DataFusion's own
      SQL planner/optimizer + `sqlparser` + object_store account for
      the jump; flagged as the new baseline for Bolt 7 to track.
      `parquet` crate coexistence confirmed clean: v59.1.0 (Bolt 2's
      direct pin, `inspect`-only) alongside v58.4.0 (DataFusion's
      transitive dependency) — no version-resolution conflicts.
- [x] `datagov query` *(revision 1)*: SQL over local CSV/Parquet only
      (PRD §10.3 is explicit — JSON/TSV/JSONL correctly rejected, exit
      4, verified against `examples/customers.json`), JSON/table/CSV
      renderings (CSV output verified bare — no envelope wrapper),
      bounded output by default (1000, verified: unbounded query over
      8,682-row parquet returns 1000 rows, `truncated: true`,
      `total_row_count: 8682`), `--limit` override (verified),
      execution statistics in the envelope. **Decision:** file-path
      resolution uses an explicit regex scan + registration, not
      DataFusion's native `enable_url_table()` — the latter would
      silently accept JSON files, undermining the PRD §10.3 boundary
      (see design.md).
- [x] Golden tests for `query`; exit codes verified: 2 (invalid SQL),
      3 (missing file), 4 (unsupported format). New `Csv` output
      variant rejected by every other command (`inspect`/`version`/
      `capabilities`), verified: `--output csv` on each → exit 2.

## Bolt 4 — `sql parse | format | transpile` ✅ (2026-07-26)

Built by a Sonnet 5 agent from `briefs/bolt-4.md`; architect-reviewed
(gates re-run independently, hands-on verified: a real dialect
transform T-SQL `TOP 10` → Postgres `LIMIT 10` not a passthrough, the
lossy `QUALIFY` warning on stderr with stdout staying pipeable, the
`--write` byte/mtime guard, and all three exit codes 2/3/4). 134 tests
green (133 run + 1 pre-existing informational `#[ignore]`).

The Phase 1 verification the brief mandated **succeeded** —
`sqlglot-rust` v0.10.23 works across all 11 priority dialects,
including genuine dialect-specific rewrites. The inception gate's
waived spike effectively happened anyway, retroactively; no fallback
to `sqlparser-rs`, no ADR needed (see proposal.md Q1 update and
design.md for the full coverage matrix and the crate's own known
fidelity gaps, none affecting the committed corpus).

- [x] Dialect conformance corpus committed under `examples/sql/` — 3
      statements × 11 priority dialects (33 files) + 5 cross-dialect
      transpile pairs with byte-verified `expected.sql`
- [x] `parse`: statement type, tables, columns, joins, filters, CTEs,
      normalized AST (the crate's own serializable `Statement`, per
      design.md's "expose the real parse result" guidance) in the
      envelope
- [x] `format`: stdout by default; file modified only with `--write`
      (HARD eval verified: mtime and SHA-256 both unchanged without
      `--write`)
- [x] `transpile` across all 11 dialects; lossy/unsupported constructs
      reported explicitly (HARD — verified: `QUALIFY` into ANSI
      surfaces a warning naming the construct, never silent)
- [x] Exit codes verified: 2 (malformed SQL), 3 (missing file), 4
      (unknown dialect); corpus round-trip suite green

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

- [x] `install.sh` authored ahead of schedule (2026-07-26, owner
      request) — OS/arch detection, latest-or-pinned release
      resolution, checksum-verified download, no-sudo install; see the
      `release-distribution` requirement.
- [x] CI release path proven ahead of schedule (2026-07-26, owner
      request) — `v0.1.0-rc.1` through `rc.4` (four iterations; see the
      dated `fix(ci):` commits on `main` for what each one caught: a
      `macos-13` runner with no available capacity, then a matrix
      structural issue where a best-effort leg's expected failure
      silently skipped the whole release job rather than letting it
      proceed). `v0.1.0-rc.4` published clean: required targets
      (`datagov-darwin-arm64`, `datagov-linux-x86_64`) succeeded,
      `datagov-darwin-x86_64` and `datagov-linux-arm64` (best-effort)
      succeeded, `datagov-windows-x86_64.exe` (best-effort) timed out
      without blocking the release, SPDX SBOM + SHA-256 checksums
      attached, correctly flagged `prerelease: true`. `install.sh
      DATAGOV_VERSION=v0.1.0-rc.4` verified end-to-end against the live
      release: checksum-verified download, install, and a working
      `datagov inspect` against a real fixture, all independent of the
      dev build. **What's left for this bolt to formally close:** the
      full PRD §36 demo sequence (needs `pii scan`/`report`, Bolts 5-6),
      the SOFT startup-time measurement, and the CI-failing-on-PII-
      threshold demo workflow (acceptance criterion 7) — those still
      wait on later bolts, this task is not fully done, just started
      early and substantially de-risked.
- [ ] Clean-machine eval: binaries run the §36 demo sequence on hosts
      with no toolchains installed
- [ ] Startup benchmark <150 ms measured and published (SOFT)
- [ ] Demo workflow: a sample GitHub Actions job failing on a PII
      threshold (acceptance criterion 7)
- [ ] Full HARD sweep green; SOFT observations reviewed and logged
- [ ] Proposal status → **IMPLEMENTED**, then **VERIFIED**; archive the
      change and merge spec deltas into `openspec/specs/`

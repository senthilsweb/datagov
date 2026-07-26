# datagov

A DataGovOps platform whose first deliverable is a single-download,
agent-ready command-line interface for data inspection, SQL analysis,
data profiling, PII detection, data quality, lineage, policy evaluation,
and governance reporting.

```bash
datagov inspect customers.parquet
datagov profile customers.csv
datagov sql transpile query.sql --from spark --to duckdb
datagov pii scan customers.parquet
datagov report customers.parquet --profile --pii --output report.json
```

One portable Rust binary — no Python, Java, Node.js, Docker, or Cargo on
the end-user machine. Deterministic first, local first, masked by
default, agent- and CI-friendly.

## Status

**Pre-implementation.** The project follows AI-DLC: specs before code.
The first change — [`add-datagov-core-cli`](openspec/changes/add-datagov-core-cli/proposal.md)
(Milestone 0.1: inspect, profile, sql parse/format/transpile, pii scan,
report, cross-platform releases) — is **PROPOSED**, awaiting its
inception gate.

## Where things live

| Path | Purpose |
|---|---|
| `docs/prd.md` | The PRD — source of truth for product scope |
| `openspec/` | AI-DLC change specs (proposal → design → tasks → spec) |
| `AGENTS.md` | Conventions for every agent and human in this repo |
| `crates/` | Rust workspace (arrives with the first approved change) |
| `.github/workflows/` | CI quality gates + tag-driven binary releases |

## Process

Every non-trivial change goes through `openspec/changes/<name>/` before
and during implementation — see [`openspec/project.md`](openspec/project.md)
for the lifecycle (`proposed → approved → implemented → verified →
archived`), bolt structure, and HARD/SOFT eval contract.

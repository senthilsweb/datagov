---
layout: default
title: datagov — DataGovOps CLI
---

# datagov

> 🚧 **Work in progress.** Milestone 0.1 is under active construction.
> `inspect`, `profile`, `query`, and `sql parse/format/transpile` are
> built and tested; `pii scan` and `report` are still being built.
> This page tracks what's actually working today — updated as each
> bolt lands, not a finished-product page.

A single-download, agent-ready command-line interface for data
inspection, SQL analysis, data profiling, PII detection, data quality,
lineage, policy evaluation, and governance reporting — one Rust
binary, no Python, Java, Node.js, or Docker required on the end-user
machine.

```bash
datagov inspect customers.parquet
datagov profile customers.csv
datagov sql transpile query.sql --from spark --to duckdb
datagov query "SELECT state, COUNT(*) FROM 'customers.parquet' GROUP BY state"
```

## Get started

- [Installation](installation.html)
- [Command reference](commands.html)

## Reference

- [Product Requirements Document](https://github.com/senthilsweb/datagov/blob/main/docs/prd.md) — full product scope across all three phases (Core, Medium, Full)
- [Report JSON Schema](https://github.com/senthilsweb/datagov/blob/main/docs/schema/report-v1.json) — the canonical envelope every command emits with `--output json`
- [Source on GitHub](https://github.com/senthilsweb/datagov)
- [Releases](https://github.com/senthilsweb/datagov/releases) — pre-release snapshots for testing (no final `v0.1.0` yet)

## Build status

| Bolt | What | Status |
|---|---|---|
| 1 | CLI frame, JSON report envelope, exit codes | ✅ Done |
| 2 | `inspect` — format detection, schema, masked samples | ✅ Done |
| 3 | `profile`, `query` (DataFusion) | ✅ Done |
| 4 | `sql parse \| format \| transpile` (sqlglot-rust) | ✅ Done |
| 5 | `pii scan` + recognizers | 🚧 In progress |
| 6 | `report`, `doctor` | ⬜ Not started |
| 7 | Release proving | 🚧 Pipeline proven, full demo pending Bolts 5–6 |

Full bolt-by-bolt build log:
[`openspec/changes/add-datagov-core-cli/tasks.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/tasks.md).

## Design principles

- **Deterministic first** — rules, parsers, checksums, and statistics
  over model inference.
- **Local first** — no data leaves the machine unless explicitly
  configured.
- **Masked by default** — PII evidence is never shown raw, in any
  output format.
- **Agent-ready** — every command supports `--output json`, stable
  exit codes, non-interactive execution, and stdin.

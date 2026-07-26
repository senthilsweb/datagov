# datagov

!!! note "Work in progress"
    Milestone 0.1 is under active construction. `inspect`, `profile`,
    `query`, `sql parse`/`format`/`transpile`, and `pii scan` (plus
    recognizer management) are built and tested; `report` and `doctor`
    are still being built. This page tracks what actually works today,
    updated as each bolt lands — not a finished-product page.

`datagov` is a single-download, agent-ready command-line interface for
data inspection, SQL analysis, data profiling, PII detection, data
quality, lineage, policy evaluation, and governance reporting — one
Rust binary, no Python, Java, Node.js, or Docker required on the
end-user machine.

```bash
datagov inspect examples/customers.parquet
datagov profile examples/customers.parquet --columns email,state
datagov sql transpile query.sql --from tsql --to postgres
datagov pii scan examples/pii-fixture.csv
```

## Build status

| Bolt | What | Status |
|---|---|---|
| 1 | CLI frame, JSON report envelope, exit codes | Done |
| 2 | `inspect` — format detection, schema, masked samples | Done |
| 3 | `profile`, `query` (DataFusion) | Done |
| 4 | `sql parse \| format \| transpile` (sqlglot-rust) | Done |
| 5 | `pii scan` + recognizers | Done |
| 6 | `report`, `doctor` | Not started |
| 7 | Release proving | `install.sh` and the tag-driven release pipeline proven end to end (see [CI/CD](ci-cd.md)); full closure waits on `report`/`doctor` from Bolt 6 |

Full bolt-by-bolt build log:
[`openspec/changes/add-datagov-core-cli/tasks.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/tasks.md).

## Reference

- [Product Requirements Document](https://github.com/senthilsweb/datagov/blob/main/docs/prd.md) — full product scope across all three phases (Core, Medium, Full)
- [Report JSON Schema](https://github.com/senthilsweb/datagov/blob/main/docs/schema/report-v1.json) — the canonical envelope every command emits with `--output json`
- [Source on GitHub](https://github.com/senthilsweb/datagov)
- [Releases](https://github.com/senthilsweb/datagov/releases) — pre-release snapshots for testing (no final `v0.1.0` yet)

## Design principles

- **Deterministic first** — rules, parsers, checksums, and statistics
  over model inference.
- **Local first** — no data leaves the machine unless explicitly
  configured.
- **Masked by default** — PII evidence is never shown raw in `inspect`
  or `pii scan` output, in any format.
- **Agent-ready** — every command supports `--output json`, stable
  exit codes, non-interactive execution, and stdin where practical.

Next: [Getting Started](getting-started.md).

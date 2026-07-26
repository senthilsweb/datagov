---
layout: default
title: Command reference
---

# Command reference

Every command below is built, tested, and working today. `pii scan`
and `report` are still under construction (Bolts 5–6) and aren't
listed here yet — check back as they land, or see the
[build status](index.html#build-status).

All commands share these global flags:

- `--output json|table` — `table` (default) is a human-readable
  summary; `json` is the full, versioned report envelope (stable
  schema, safe for scripts and agents).
- `--quiet` / `--verbose` — control diagnostic output on stderr;
  stdout is always reserved for command output.

Sample commands below use the fixtures committed in
[`examples/`](https://github.com/senthilsweb/datagov/tree/main/examples)
— clone the repo to run them yourself, or point them at your own data.

## `datagov inspect`

Format detection, schema, row/column counts, and masked sample rows
across CSV, TSV, JSON, JSONL, and Parquet.

```bash
datagov inspect examples/customers.parquet
datagov inspect examples/customers.csv --output json
cat examples/customers.jsonl | datagov inspect - --type jsonl
```

PII-shaped columns (email, phone, etc.) are masked in sample rows even
at this stage, ahead of the dedicated PII engine — see the
[design notes](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/design.md)
for why.

Exit codes: `3` if the input file doesn't exist, `4` if the format
can't be determined or isn't supported.

## `datagov profile`

Column statistics — nulls, distinct counts, min/max/mean/median/
stddev/quantiles for numeric columns, string-length stats, masked
top-values — computed via an embedded DataFusion query engine.

```bash
datagov profile examples/customers.parquet
datagov profile examples/customers.parquet --columns email,state --output json
datagov profile examples/customers.csv --sample 1000
```

`--sample N` profiles the first N rows in source order (deterministic
— not a random sample, so repeated runs are reproducible).

## `datagov query`

SQL over local CSV and Parquet files — no server, no daemon.

```bash
datagov query "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state ORDER BY total DESC"
datagov query "SELECT * FROM 'examples/customers.parquet'" --limit 50 --output csv
```

Output is bounded to 1,000 rows by default (`--limit` overrides); a
truncated result says so explicitly in the JSON envelope rather than
silently dropping rows. `--output csv` emits bare CSV with no envelope
wrapper, for piping into other tools.

Only CSV and Parquet are supported (per the product scope) — a query
against a JSON or JSONL file exits with code `4`.

## `datagov sql parse | format | transpile`

SQL parsing, formatting, and cross-dialect transpilation across 11
priority dialects: ANSI, PostgreSQL, DuckDB, Spark, Databricks,
Snowflake, BigQuery, Trino, MySQL, SQLite, and T-SQL.

```bash
datagov sql parse query.sql --dialect spark --output json

# stdout only by default — the source file is untouched
datagov sql format query.sql --dialect ansi
# add --write to format in place
datagov sql format query.sql --dialect ansi --write

# a genuine dialect rewrite, not a text substitution:
# "SELECT TOP 10 id, name FROM customers" (T-SQL) becomes:
datagov sql transpile query.sql --from tsql --to postgres
# → SELECT id, name FROM customers LIMIT 10
```

A construct with no equivalent in the target dialect produces an
explicit warning (on stderr for the human-readable path, in the JSON
envelope's `warnings` field otherwise) — transpilation never silently
changes meaning without telling you.

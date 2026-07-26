# Commands

At the end you will know every command `datagov` supports today, every
flag it accepts, and what each exit code means — one reference page,
not a terse flag table (an explicit owner decision for this project:
split into one page per command only once the surface passes roughly
eight commands).

## Global flags

Every command accepts these three, defined once on the root parser:

- `--output <json|table|csv>` — rendering format. `table` (default) is
  a human-readable summary. `json` is the full, versioned report
  envelope (stable schema, safe for scripts and agents). `csv` is
  accepted **only by `query`** — every other command rejects it with
  exit code `2` (verified: `datagov inspect examples/customers.parquet --output csv` →
  `error: 'inspect' does not support --output csv`, exit `2`).
- `--quiet` — suppress all diagnostics except errors on stderr.
- `--verbose` — emit DEBUG-level JSON-lines diagnostics on stderr.
  `--quiet` and `--verbose` are mutually exclusive; passing both exits
  `2` (clap's own usage-error code).

stdout is always reserved for command output; diagnostics never mix
into it.

Sample commands below use the fixtures committed in
[`examples/`](https://github.com/senthilsweb/datagov/tree/main/examples) —
clone the repo to run them yourself, or point them at your own data.

## Inspect (datagov inspect)

```text
datagov inspect <path|-> [--type <csv|tsv|json|jsonl|parquet>]
```

Format detection, schema, row/column counts, nullability, Parquet row
groups/compression (read from metadata, no full scan), and masked
sample rows — across CSV, TSV, JSON, JSONL, and Parquet.

Flags:

- `--type <FORMAT>` — explicit format when it can't be inferred from
  the path. **Required** when reading from stdin (`-`); omitting it
  there is a hard exit `2`.

Exit codes:

- `2` — `--type` missing on a stdin read; or `--output csv` requested
  (rejected — only `query` supports CSV output).
- `3` — the input file does not exist.
- `4` — the format can't be detected from extension/content, or isn't
  one of the five supported formats.

Tested example:

```bash
datagov inspect examples/customers.parquet
```

```text
format:       parquet
file size:    474.7 KiB
row count:    8682
column count: 18
...
row groups: 1, compression: SNAPPY
```

PII-shaped columns (`email`, `phone`) are masked in sample rows even at
this stage, ahead of the dedicated PII engine — see the
[FAQ](faq.md#why-does-inspect-mask-pii-columns-before-pii-scan-existed).

## Profile (datagov profile)

```text
datagov profile <path> [--type <FORMAT>] [--columns a,b,c] [--sample N]
```

Column statistics — nulls, distinct counts, min/max/mean/median/
stddev/quantiles for numeric columns, string-length stats, masked
top-values, inferred semantic type, and possible-identifier flagging —
computed via the embedded DataFusion engine. **CSV and Parquet only**
(the same boundary as `query`); a JSON/TSV/JSONL input exits `4`.

Flags:

- `--type <FORMAT>` — explicit format when it can't be inferred.
- `--columns a,b,c` — profile only these column names, comma-separated.
- `--sample N` — profile only the first N rows in source order
  (deterministic, not a random sample — repeated runs are reproducible).

Exit codes:

- `2` — `--output csv` requested (rejected).
- `3` — the input file does not exist.
- `4` — the input format is not CSV or Parquet.

Tested example:

```bash
datagov profile examples/customers.parquet --columns email,state
```

```text
+--------+--------+---------+----------+----------+------+--------+---------------+------------+
| column | type   | nulls % | distinct | unique % | mean | stddev | semantic type | identifier |
+==============================================================================================+
| email  | string | 0.0     | 8673     | 99.9     | -    | -      | email         | false      |
| state  | string | 0.0     | 63       | 0.7      | -    | -      | text          | false      |
+--------+--------+---------+----------+----------+------+--------+---------------+------------+
```

## Query (datagov query)

```text
datagov query "<sql>" [--limit N]
```

SQL over local CSV and Parquet files — no server, no daemon. **This is
the one command that supports `--output csv`.** Query results are
**not masked** — unlike `inspect`/`profile`, which specifically mask
sample rows, `query` returns exactly what the SQL asks for, same as a
raw database client would.

Flags:

- `--limit N` — override the default row bound
  (`datagov_data::query::DEFAULT_QUERY_LIMIT`, 1000). Output is bounded
  to 1,000 rows by default; a truncated result says so explicitly in
  the JSON envelope (`extensions.query.truncated: true`,
  `total_row_count`) rather than silently dropping rows.

Exit codes:

- `2` — invalid SQL syntax.
- `3` — a referenced file does not exist.
- `4` — a referenced file is not CSV or Parquet (verified:
  `datagov query "SELECT * FROM 'examples/customers.json'"` → exit `4`,
  "query only supports CSV and Parquet files").

Tested example:

```bash
datagov query "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state ORDER BY total DESC LIMIT 5"
```

```text
+-------+-------+
| state | total |
+===============+
| NT    | 351   |
| NL    | 341   |
| BC    | 341   |
| QC    | 338   |
| PE    | 333   |
+-------+-------+
```

`--output csv` emits bare CSV — header + bounded rows, no envelope —
for piping into other tools:

```bash
datagov query "SELECT * FROM 'examples/customers.parquet'" --limit 3 --output csv
```

## SQL parse (datagov sql parse)

```text
datagov sql parse <path|-> [--dialect <name>]
```

Parses one SQL statement into: statement type, tables, columns, joins,
filters, grouping, ordering, CTEs, and the full normalized AST (the
underlying `sqlglot-rust` crate's own serializable `Statement`) — under
`extensions.sql_parse` in JSON output.

Flags:

- `--dialect <name>` — source dialect, one of `ansi` (default),
  `postgres`, `duckdb`, `spark`, `databricks`, `snowflake`, `bigquery`,
  `trino`, `mysql`, `sqlite`, `tsql`.

Exit codes:

- `2` — malformed SQL (parser error).
- `3` — the input file does not exist.
- `4` — the named dialect isn't one of the 11 supported.

Tested example:

```bash
datagov sql parse examples/sql/postgres/select_where.sql --dialect postgres --output json
```

Returns the full AST under `extensions.sql_parse.ast` — confirmed
against `SELECT id, name FROM customers WHERE state = 'CA';`.

## SQL format (datagov sql format)

```text
datagov sql format <path|-> [--dialect <name>] [--write]
```

Pretty-prints a SQL statement. Writes to **stdout by default** — the
source file is untouched unless `--write` is given (verified: file
SHA-256 and mtime both unchanged after a plain `sql format` run).

Flags:

- `--dialect <name>` — same 11-dialect set as `sql parse`.
- `--write` — modify the source file in place instead of printing to
  stdout. Not compatible with stdin (`-`) — there is no source file to
  modify.

Exit codes:

- `2` — malformed SQL.
- `3` — the input file does not exist.
- `4` — the named dialect isn't supported.

Tested example:

```bash
datagov sql format examples/sql/postgres/select_where.sql --dialect ansi
```

```text
SELECT
  id,
  name
FROM
  customers
WHERE
  state = 'CA'
```

## SQL transpile (datagov sql transpile)

```text
datagov sql transpile <path|-> --from <dialect> --to <dialect>
```

Rewrites a SQL statement from one dialect to another — a genuine
dialect transform, not a text substitution. A construct with no
equivalent in the target dialect produces an explicit warning (stderr
for the human-readable path, the JSON envelope's `warnings` field
otherwise) rather than silently changing meaning.

Flags:

- `--from <dialect>` — required, source dialect.
- `--to <dialect>` — required, target dialect.

Exit codes:

- `2` — malformed SQL in the source file.
- `3` — the input file does not exist.
- `4` — either dialect isn't one of the 11 supported.

Tested example — T-SQL's `TOP` clause becomes Postgres's `LIMIT`:

```bash
cat examples/sql/tsql/idiomatic.sql
# SELECT TOP 10 id, name FROM customers;

datagov sql transpile examples/sql/tsql/idiomatic.sql --from tsql --to postgres
# SELECT id, name FROM customers LIMIT 10
```

A lossy example — Snowflake's `QUALIFY` has no ANSI equivalent:

```bash
datagov sql transpile examples/sql/snowflake/idiomatic.sql --from snowflake --to ansi
```

```text
SELECT id, ROW_NUMBER() OVER (PARTITION BY state ORDER BY id) AS rn FROM customers QUALIFY rn = 1
warning: [QUALIFY] the QUALIFY clause was carried into the output unchanged, but ANSI SQL does not natively support QUALIFY — the generated SQL may not execute as-is on ANSI SQL
```

## PII scan (datagov pii scan)

```text
datagov pii scan <path> [--type <FORMAT>] [--sample N] [--field a,b] [--recognizers <path>] [--fail-on <confidence>]
```

Deterministic PII detection over CSV, TSV, JSONL, or Parquet. Every
finding carries entity type, column, confidence, recognizer id, masked
evidence, match count/percentage, and reason — **never a raw value**,
in any output mode or error path. Requires a real file path; stdin
(`-`) is not supported (DataFusion table registration needs random
file access).

Flags:

- `--type <FORMAT>` — explicit format when it can't be inferred.
- `--sample N` — scan only the first N rows in source order
  (deterministic).
- `--field a,b` — scan only these column names, comma-separated;
  default is all columns.
- `--recognizers <path>` — load additional recognizers from a PRD
  §10.9 YAML file. A custom recognizer whose `id` matches a built-in
  overrides it; otherwise it's added.
- `--fail-on <confidence>` — after the full report is printed (never
  suppressed), exit code `12` if any finding's confidence is at or
  above this threshold.

Exit codes:

- `2` — stdin was given as the path; or the `--recognizers` file is
  malformed.
- `3` — the input file does not exist.
- `4` — the input isn't CSV/TSV/JSONL/Parquet (verified:
  `datagov pii scan examples/customers.json` → exit `4`, "pii scan
  supports CSV, TSV, JSONL, and Parquet").
- `12` — `--fail-on` threshold breached by at least one finding.

Tested example:

```bash
datagov pii scan examples/pii-fixture.csv
```

```text
scanned 10 column(s), 10 row(s)
+--------------------+---------------+---------------+------------+---------+---------------------+
| column             | entity        | recognizer    | confidence | match % | evidence            |
+=================================================================================================+
| ipv4_address       | IP_ADDRESS_V4 | ip_address_v4 | 0.70       | 100.0   | 19….1, 19….2, 19….3 |
| ssn                | US_SSN        | us_ssn        | 0.90       | 100.0   | 55…01, 55…02, 55…03 |
| credit_card_number | CREDIT_CARD   | credit_card   | 0.85       | 100.0   | 41…11, 55…44, 37…05 |
...
```

`--fail-on` verified both sides of a real finding's confidence:
`--fail-on 0.5` against this fixture exits `12`; `--fail-on 0.99` exits
`0`.

## PII recognizers list (datagov pii recognizers list)

```text
datagov pii recognizers list
```

Enumerates the 10 built-in recognizers: id, entity, confidence, and
pattern count. No flags beyond the global set.

Tested example:

```bash
datagov pii recognizers list
```

```text
+---------------+---------------+------------+----------+
| id            | entity        | confidence | patterns |
+=======================================================+
| email_address | EMAIL_ADDRESS | 0.75       | 1        |
| phone_number  | PHONE_NUMBER  | 0.60       | 1        |
| us_ssn        | US_SSN        | 0.75       | 1        |
| credit_card   | CREDIT_CARD   | 0.70       | 1        |
...
```

## PII recognizers validate (datagov pii recognizers validate)

```text
datagov pii recognizers validate <path>
```

Validates a PRD §10.9 recognizers YAML file: every regex compiles,
every confidence is in range, and every validator name is known.

Exit codes:

- `0` — valid; prints the recognizer count loaded.
- `2` — invalid (regex parse failure, out-of-range confidence, or
  unknown validator), naming the offending recognizer id and field.

Tested example:

```bash
datagov pii recognizers validate recognizers/example-custom.yaml
```

```text
'recognizers/example-custom.yaml' is valid: 2 recognizer(s) loaded.
```

Against a deliberately broken file:

```bash
datagov pii recognizers validate recognizers/example-invalid.yaml
```

```text
error: recognizer 'broken_pattern': field 'patterns' has an invalid regex '(unclosed': regex parse error: ...
remediation: fix the recognizer file and try again — see PRD §10.9 for the expected schema
```

Next: [Tutorials](tutorials.md) for end-to-end recipes chaining these
commands together.

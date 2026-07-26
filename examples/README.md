# Example fixtures

## Try them against the CLI

From the repo root, after `cargo build --release` (see the root
[README](../README.md#try-it-now)):

```bash
./target/release/datagov inspect examples/customers.parquet
./target/release/datagov inspect examples/customers.csv --output json | jq .
./target/release/datagov inspect examples/customers.tsv
./target/release/datagov inspect examples/customers.jsonl --output json | jq '.dataset.schema'
cat examples/customers.jsonl | ./target/release/datagov inspect - --type jsonl

# email/phone are masked even in raw sample rows — compare against the
# actual first row of customers.csv to see it in action
./target/release/datagov inspect examples/customers.parquet --output json \
  | jq '.dataset.sample_rows[0] | {email, phone}'

# pii scan: entity-typed, confidence-scored findings — never raw values
./target/release/datagov pii scan examples/customers.parquet
./target/release/datagov pii scan examples/pii-fixture.csv --output json | jq '.pii.findings'
./target/release/datagov pii scan examples/pii-fixture.csv --field notes
./target/release/datagov pii recognizers list
```

All datasets here are **synthetic** — generated identities and Lorem
Ipsum-style content, never real people or organizations. They are
sourced from the owner's own public dataset repos, so provenance is
external and auditable rather than invented for this project.

## `customers.*` — one logical dataset, five formats

`customers.csv`, `customers.tsv`, `customers.json`, `customers.jsonl`,
and `customers.parquet` all share the same schema (a TICKIT-shaped
users table: `userid, username, firstname, lastname, city, state,
email, phone` + ten `like*` booleans, several with null values —
18 columns total, confirmed by Bolt 2's `inspect` output across all
five formats). Using one dataset across every supported format lets
`inspect` golden tests compare format-specific parsing without also
varying the data.

| File | Rows | Source |
|---|---|---|
| `customers.parquet` | 8,682 (full file, Snappy-compressed) | [`senthilsweb/apache-iceberg`](https://github.com/senthilsweb/apache-iceberg) `output/sports_fans.parquet` |
| `customers.csv` | 2,000 (trimmed) | [`senthilsweb/datasets`](https://github.com/senthilsweb/datasets) `ticket/users.csv` (AWS TICKIT-shaped sample, synthetic identities) |
| `customers.tsv` | 200 (trimmed) | same source as `customers.csv`, re-delimited |
| `customers.jsonl` | 200 (trimmed) | same source, one JSON object per line |
| `customers.json` | 50 (trimmed) | same source, single JSON array |

`customers.parquet` is committed at its full row count and original
Snappy compression so `inspect` exercises real Parquet metadata
(row groups, compression codec) rather than a hand-rolled stand-in.
The CSV/TSV/JSON/JSONL files are trimmed subsets of the same source to
keep the repo lean — they are fixtures for format-parsing and
golden-output tests, not for performance benchmarking (see
`benchmarks/` for that).

`email` and `phone` are real-shaped PII columns (synthetic values) —
this fixture set is reused as the `pii scan` masking-eval target from
Bolt 5 onward; no separate PII fixture is needed for the entities it
covers.

## `pii-fixture.csv` — synthetic fixture for the 8 non-`customers.*` PII entities (Bolt 5)

`customers.*` naturally covers `EMAIL_ADDRESS` and `PHONE_NUMBER` (real,
synthetic-identity values already in that dataset). The other 8 entities
in the inception-gate-confirmed set (proposal.md Q3) need dedicated
columns, so `pii-fixture.csv` is a small (10-row), hand-authored,
clearly-synthetic dataset — no generator script, no external download.
Every value is either a universally-recognized public test/placeholder
constant or drawn from an IANA/IETF documentation-reserved range, never
a real, uniquely-identifying value:

| Column | Entity | Provenance |
|---|---|---|
| `ipv4_address` | `IP_ADDRESS_V4` | IANA TEST-NET-1/2/3 ranges (RFC 5737): `192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24` — reserved specifically for documentation/examples, never routable on the real internet. |
| `ipv6_address` | `IP_ADDRESS_V6` | IANA documentation prefix (RFC 3849): `2001:db8::/32`. |
| `website_url` | `URL` | IANA-reserved example domains (RFC 2606): `example.com`, `example.org`, `example.net`. |
| `ssn` | `US_SSN` | Area number `555` — never issued by the SSA (the same convention that makes `555` telephone numbers fictional), so every value is guaranteed non-real while still passing this bolt's own SSA-area validator. |
| `credit_card_number` | `CREDIT_CARD` | The universally-recognized Luhn-valid sandbox test numbers published by major payment processors' own docs: `4111111111111111` (Visa), `5555555555554444` (Mastercard), `378282246310005` (Amex), `6011111111111117` (Discover). |
| `record_uuid` | `UUID` | Well-known public example UUIDs from RFC 4122 and the OpenAPI/Swagger docs (`3fa85f64-5717-4562-…`, `550e8400-e29b-41d4-…`, `6ba7b810-9dad-11d1-…`), reused across rows — not generated per-record, since these are illustrative constants, not identities. |
| `mac_address` | `MAC_ADDRESS` | Locally-administered addresses (the U/L bit set in the first octet, e.g. `02:00:00:00:00:01`) — the standard way to construct a MAC address that is guaranteed not to collide with any real vendor-assigned OUI. |
| `zip_code` | `US_ZIP_CODE` | Generic, widely-reused example ZIPs (`12345`, `90210`, …) — district-level codes, not street-level addresses. |
| `notes` | (prose) | Free text with an `IP_ADDRESS_V4`/`URL` embedded mid-sentence (e.g. `"Reported via https://example.com/incidents/42 from host 192.0.2.10…"`) — exercises `pii scan`'s `find_iter`-based embedded-match detection (PRD §10.8's `--field text` example) against entities beyond email/phone. |

## `claimwise-*.csv` — synthetic healthcare RCM data

`claimwise-claims.csv`, `claimwise-encounters.csv`, and
`claimwise-activities.csv` are synthetic revenue-cycle-management
records (claims, encounters, activities) with foreign-key
relationships (`claimID`, `patientID`, `encounterID`) across files —
useful for later multi-file lineage and referential-integrity rule
testing (Core `quality check` referential membership, Medium lineage).
Not used by Milestone 0.1; committed now so later changes don't need a
second fixture-sourcing round trip.

Source: [`senthilsweb/claimwise`](https://github.com/senthilsweb/claimwise)
`dbt-pipeline/seeds/healthcare_data/`.

## `sql/` — dialect conformance corpus (Bolt 4)

`sql/<dialect>/` (one directory per priority dialect: `ansi`,
`postgres`, `duckdb`, `spark`, `databricks`, `snowflake`, `bigquery`,
`trino`, `mysql`, `sqlite`, `tsql`) each contain three statements:
`select_where.sql` and `join_group_by.sql` (identical, dialect-portable
SQL across all 11 — the same statement genuinely parses/generates
identically in every dialect) and `idiomatic.sql` (one real,
dialect-specific construct per dialect — e.g. Snowflake/Databricks
`QUALIFY`, T-SQL `TOP n`, MySQL/BigQuery backtick identifiers, DuckDB
`PIVOT`, Spark `LATERAL VIEW`, Trino `APPROX_DISTINCT`, Postgres
`ILIKE`, SQLite `ON CONFLICT ... DO NOTHING`, ANSI `FETCH FIRST n ROWS
ONLY`). All 33 files parse successfully via `datagov sql parse` under
their own dialect — see the Bolt 4 report for the full per-dialect
coverage matrix, including constructs that parse but don't round-trip
perfectly (Spark `LATERAL VIEW`'s output-column alias, for one).

`sql/transpile/<pair-name>/{source.sql,expected.sql}` are five
cross-dialect pairs (`spark_to_duckdb`, `tsql_to_postgres`,
`snowflake_to_bigquery`, `mysql_to_ansi`, and the deliberately lossy
`lossy_qualify_snowflake_to_ansi`) used as golden round-trip fixtures
by `crates/datagov-cli/tests/sql.rs` — `expected.sql` is the exact,
byte-verified output of `datagov sql transpile source.sql --from <a>
--to <b>`.

## Regenerating

These are one-time downloads, not build outputs — there is no
generator script to run. To refresh from source:

```bash
curl -fsSL -o examples/customers.parquet \
  https://raw.githubusercontent.com/senthilsweb/apache-iceberg/main/output/sports_fans.parquet

curl -fsSL https://raw.githubusercontent.com/senthilsweb/datasets/master/ticket/users.csv \
  | duckdb -c "COPY (SELECT * FROM read_csv_auto('/dev/stdin') LIMIT 2000) TO 'examples/customers.csv' (HEADER);"
```

# Example fixtures

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

## Regenerating

These are one-time downloads, not build outputs — there is no
generator script to run. To refresh from source:

```bash
curl -fsSL -o examples/customers.parquet \
  https://raw.githubusercontent.com/senthilsweb/apache-iceberg/main/output/sports_fans.parquet

curl -fsSL https://raw.githubusercontent.com/senthilsweb/datasets/master/ticket/users.csv \
  | duckdb -c "COPY (SELECT * FROM read_csv_auto('/dev/stdin') LIMIT 2000) TO 'examples/customers.csv' (HEADER);"
```

# Use Cases

At the end you will know which `datagov` command to reach for, given a
real governance task — scenario framing adapted from the PRD's
[Primary Use Cases](https://github.com/senthilsweb/datagov/blob/main/docs/prd.md#6-primary-use-cases)
(§6), narrowed to only what's actually built today. The PRD lists 20
use cases across all three product phases; the scenarios below cover
the subset Milestone 0.1 addresses.

## I've been handed a file I've never seen and need to trust it fast

I don't know the schema, the row count, or whether it's even the
format the filename claims. I run:

```bash
datagov inspect customers.parquet
```

and get format, size, row/column counts, schema with types and
nullability, and a masked sample — enough to decide whether this is
the file I think it is, without opening it in a spreadsheet tool or
writing a script.

## I want to understand a column's shape before I build anything on it

Before I write a quality rule or trust a "clean" label, I want actual
statistics — nulls, distinct counts, distribution, possible identifier
flags:

```bash
datagov profile customers.parquet --columns email,state
```

`profile` tells me `email` is 99.9% distinct and semantically an email
address, without me having to write that logic myself.

## I have an ad hoc question and don't want to stand up a database

I need one answer — a group-by, a filter, a count — from a CSV or
Parquet file sitting on disk, and starting DuckDB or loading it into
pandas is overkill for one query:

```bash
datagov query "SELECT state, COUNT(*) AS total FROM 'customers.parquet' GROUP BY state ORDER BY total DESC"
```

## I inherited SQL I didn't write and need to understand or normalize it

A teammate's SQL doesn't follow our formatting convention, or I need
to know exactly what tables and columns it touches before I change
anything:

```bash
datagov sql parse query.sql --dialect postgres --output json
datagov sql format query.sql --dialect postgres --write
```

`parse` gives me the structure (tables, joins, filters, CTEs) as data
I can act on; `format` normalizes the style, and only touches the file
when I explicitly say `--write`.

## I'm migrating queries between SQL engines

We're moving workloads from one warehouse dialect to another, and I
need to know which queries transfer cleanly and which don't:

```bash
datagov sql transpile query.sql --from tsql --to postgres
```

If a construct doesn't exist in the target dialect, `datagov` tells me
explicitly instead of generating SQL that silently changes meaning —
see [Tutorials](tutorials.md#compare-sql-across-two-dialects-for-a-migration)
for a worked example.

## I need to know if a file is safe to hand to someone else

Before sharing a dataset outside my team, I want a determination of
what sensitive entities it contains — not a guess based on column
names alone:

```bash
datagov pii scan customers.parquet --fail-on 0.8
```

Every finding carries a confidence score and masked evidence, never a
raw value; `--fail-on` gives me an exit code I can gate a script or CI
job on. See
[Tutorials](tutorials.md#check-a-dataset-for-pii-before-sharing-it) for
the full recipe.

## I want to know exactly what PII detection is checking for

Before I trust a scan's results, or before I add a detector for
something specific to my data (an internal account ID format, for
example), I want to see the built-in recognizer set and validate my
own:

```bash
datagov pii recognizers list
datagov pii recognizers validate my-recognizers.yaml
```

Next: [Configuration](configuration.md) for how these commands resolve
their settings.

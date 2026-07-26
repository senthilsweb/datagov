# Getting Started

At the end you will have run three independent 5-minute paths through
`datagov` — each complete on its own, using the fixtures already
committed in [`examples/`](https://github.com/senthilsweb/datagov/tree/main/examples).
Start with whichever path matches what you want to see first; none
depends on the others.

All three assume you have the binary built (see
[Installation](installation.md)) and are running from the repo root.

## Path 1 — inspect a bundled fixture (zero setup)

The cheapest path: no flags, no config, just point `inspect` at a file
that already ships in the repo.

```bash
./target/release/datagov inspect examples/customers.parquet
```

```text
format:       parquet
file size:    474.7 KiB
row count:    8682
column count: 18
+---------------+---------+----------+
| name          | type    | nullable |
+====================================+
| userid        | integer | true     |
| username      | string  | true     |
| ...           | ...     | ...      |
+---------------+---------+----------+
row groups: 1, compression: SNAPPY
```

Sample rows are printed too — `email` and `phone` are masked even
here, ahead of the dedicated PII engine (see the
[FAQ](faq.md#why-does-inspect-mask-pii-columns-before-pii-scan-existed)
for why).

## Path 2 — profile and query the same fixture

`profile` computes column statistics; `query` runs SQL over the same
file — both via the embedded DataFusion engine, no server involved.

```bash
./target/release/datagov profile examples/customers.parquet --columns email,state
```

```text
+--------+--------+---------+----------+----------+------+--------+---------------+------------+
| column | type   | nulls % | distinct | unique % | mean | stddev | semantic type | identifier |
+==============================================================================================+
| email  | string | 0.0     | 8673     | 99.9     | -    | -      | email         | false      |
| state  | string | 0.0     | 63       | 0.7      | -    | -      | text          | false      |
+--------+--------+---------+----------+----------+------+--------+---------------+------------+
```

Now query the same file directly with SQL:

```bash
./target/release/datagov query "SELECT state, COUNT(*) AS total FROM 'examples/customers.parquet' GROUP BY state ORDER BY total DESC LIMIT 5"
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

`profile` tells you what the column looks like; `query` lets you ask
your own question of it — same underlying engine, two different jobs.

## Path 3 — transpile one file across two SQL dialects

`sql transpile` performs a real dialect rewrite, not a text
substitution. `examples/sql/tsql/idiomatic.sql` contains T-SQL's `TOP`
clause, which has no ANSI/Postgres equivalent syntax — `datagov`
rewrites it to `LIMIT`:

```bash
cat examples/sql/tsql/idiomatic.sql
```

```sql
SELECT TOP 10 id, name FROM customers;
```

```bash
./target/release/datagov sql transpile examples/sql/tsql/idiomatic.sql --from tsql --to postgres
```

```text
SELECT id, name FROM customers LIMIT 10
```

Next: [Installation](installation.md) for the full set of ways to get
the binary, or [Commands](commands.md) for the complete reference.

# Tutorials

At the end you will have chained multiple `datagov` commands into two
real, end-to-end recipes — distinct from [Getting Started](getting-started.md)'s
atomic, single-command paths. Only commands that exist today are used
here.

## Check a dataset for PII before sharing it

You have a CSV you're about to hand to someone outside your team.
First, see its shape without looking at raw values:

```bash
datagov inspect examples/pii-fixture.csv --output json
```

The envelope's `dataset.row_count` and `dataset.column_count` confirm
this is a small, 10-row, 10-column file — cheap to scan in full. Now
run the actual PII scan on the same file:

```bash
datagov pii scan examples/pii-fixture.csv
```

Every finding is entity-typed and confidence-scored, with masked
evidence only — `record_uuid`, `ssn`, `credit_card_number`,
`ipv4_address`, and more all surface as findings without ever printing
a raw value. If you want the check to fail loudly in a script or CI
job rather than just reporting, add a threshold:

```bash
datagov pii scan examples/pii-fixture.csv --fail-on 0.8
echo "exit code: $?"
```

Against this fixture, `--fail-on 0.8` exits `12` (`mac_address` and
`credit_card_number` findings score at or above 0.8) — a scriptable
gate: "don't share this file until these findings are resolved."

## Compare SQL across two dialects for a migration

You're migrating a query from SQL Server (T-SQL) to Postgres and want
to understand what changes before you commit to it. Start by parsing
the source under its real dialect, to confirm `datagov` understands
the construct you're migrating:

```bash
cat examples/sql/transpile/tsql_to_postgres/source.sql
```

```sql
SELECT t.state, COUNT(*) AS total FROM customers t INNER JOIN orders o ON t.id = o.customer_id GROUP BY t.state ORDER BY t.state;
```

```bash
datagov sql parse examples/sql/transpile/tsql_to_postgres/source.sql --dialect tsql --output json
```

The AST under `extensions.sql_parse.ast` confirms the statement type,
tables (`customers t`, `orders o`), the join, and the grouping — before
you transform anything. Now transpile it:

```bash
datagov sql transpile examples/sql/transpile/tsql_to_postgres/source.sql --from tsql --to postgres
```

```text
SELECT t.state, COUNT(*) AS total FROM customers AS t INNER JOIN orders AS o ON t.id = o.customer_id GROUP BY t.state ORDER BY t.state
```

The rewrite adds explicit `AS` before table aliases — implicit aliasing
(`customers t`) is valid T-SQL but Postgres's canonical form spells out
`AS`. Confirm the target parses cleanly under its own dialect too:

```bash
datagov sql parse examples/sql/transpile/tsql_to_postgres/expected.sql --dialect postgres
```

No errors, exit `0` — the transpiled query is valid Postgres. For a
construct with no equivalent in the target dialect (Snowflake's
`QUALIFY` into ANSI, for example), `transpile` prints an explicit
warning instead of silently producing SQL that won't run — see
[Commands](commands.md#sql-transpile-datagov-sql-transpile) for that example.

Next: [Use Cases](use-cases.md) for the broader scenarios these
commands address.

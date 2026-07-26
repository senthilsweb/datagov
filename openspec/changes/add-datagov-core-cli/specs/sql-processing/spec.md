# Spec delta — `sql-processing` (add-datagov-core-cli)

### Requirement: SQL parsing

`datagov sql parse` SHALL expose statement type, tables, aliases,
selected columns, joins, filters, grouping, ordering, functions, CTEs,
subqueries, and a normalized AST representation (PRD §10.4), with the
dialect selectable via `--dialect`.

#### Scenario: Parse from stdin with a dialect

**When** the user runs `cat q.sql | datagov sql parse - --dialect spark`
**Then** the envelope lists the referenced tables and columns
**And** records the dialect used.

### Requirement: Formatting is non-destructive by default

`datagov sql format` SHALL write formatted SQL to stdout and SHALL
modify the source file only when `--write` is supplied.

#### Scenario: Explicit write

**When** the user runs `datagov sql format query.sql --write`
**Then** `query.sql` contains the formatted SQL
**And** the exit code is 0.

### Requirement: Transpilation across priority dialects

`datagov sql transpile --from <a> --to <b>` SHALL support the priority
dialect list confirmed at the inception gate (PRD §10.6), and SHALL
report unsupported or lossy transformations explicitly in the envelope.

#### Scenario: Lossy construct

**Given** a source statement using a construct with no target-dialect
equivalent
**When** the user transpiles it
**Then** the envelope contains an explicit warning identifying the
construct
**And** the output is never silently altered semantics.

#### Scenario: Unsupported dialect

**When** the user runs `datagov sql transpile q.sql --from foo --to bar`
**Then** the process exits with code 4.

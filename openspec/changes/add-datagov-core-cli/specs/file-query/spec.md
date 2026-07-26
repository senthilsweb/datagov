# Spec delta — `file-query` (add-datagov-core-cli, revision 1)

### Requirement: SQL over local files

`datagov query "<sql>"` SHALL execute SQL against local CSV and Parquet
files through the embedded DataFusion engine (PRD §10.3), with no
external service, daemon, or runtime involved.

#### Scenario: Aggregate a Parquet file

**When** the user runs
`datagov query "SELECT state, COUNT(*) AS total FROM 'customers.parquet' GROUP BY state"`
**Then** the result is computed locally
**And** the envelope records query execution statistics (rows scanned,
duration).

### Requirement: Bounded output by default

Query output SHALL be bounded by a documented default row limit;
`--limit <n>` overrides it explicitly. Truncation is reported in the
envelope, never silent.

#### Scenario: Result exceeds the default bound

**Given** a query whose result set exceeds the default limit
**When** the user runs it without `--limit`
**Then** only the bounded rows are emitted
**And** the envelope marks the result as truncated with the total count
where cheaply available.

### Requirement: Output renderings

`query` SHALL render results as JSON (canonical envelope), table, and
CSV via `--output`.

#### Scenario: CSV rendering

**When** the user runs a query with `--output csv`
**Then** stdout contains only the CSV result rows and header, suitable
for piping.

### Requirement: Query failure exit codes

Invalid SQL SHALL exit with code 2; a referenced file that does not
exist SHALL exit with code 3; an unsupported file format SHALL exit
with code 4.

#### Scenario: Missing file in FROM clause

**When** the user queries `'missing.parquet'`
**Then** the process exits with code 3
**And** the error names the missing file.

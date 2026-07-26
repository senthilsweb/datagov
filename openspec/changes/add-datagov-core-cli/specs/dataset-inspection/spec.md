# Spec delta — `dataset-inspection` (add-datagov-core-cli)

### Requirement: Supported formats

`datagov inspect` SHALL support CSV, TSV, JSON, JSONL, and Parquet
inputs (Arrow IPC per inception-gate Q4), detecting the format from
extension and content, overridable with `--type`.

#### Scenario: Unsupported format

**When** the user runs `datagov inspect image.png`
**Then** the process exits with code 4
**And** the error names the detected/attempted format.

### Requirement: Inspection payload

Inspection SHALL report detected format, file size, row count, column
count, schema (names, data types, nullability), approximate in-memory
size, and — for Parquet — row groups and compression codecs (PRD §10.1).

#### Scenario: Inspect a Parquet file

**When** the user runs `datagov inspect customers.parquet --output json`
**Then** the envelope's `dataset` section contains the schema and
Parquet row-group and compression details
**And** row count is obtained from metadata without a full scan.

### Requirement: Sample rows are masked

Sample rows included in inspection output SHALL pass through the shared
masking layer before emission.

#### Scenario: Samples from a column matching a PII recognizer

**Given** a dataset whose `email` column matches the email recognizer
**When** sample rows appear in inspect output
**Then** the sampled email values are masked.

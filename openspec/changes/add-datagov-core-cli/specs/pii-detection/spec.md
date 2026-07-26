# Spec delta — `pii-detection` (add-datagov-core-cli)

### Requirement: Deterministic detection

`datagov pii scan` SHALL detect the inception-gate-confirmed entity set
(from PRD §10.8) using only deterministic inputs: column names, sampled
values, regular expressions, checksums/validators, value distributions,
and context terms. No network calls, no model inference in Core.

#### Scenario: Scan a structured file

**When** the user runs `datagov pii scan customers.parquet`
**Then** each finding includes entity type, column, confidence,
recognizer id, masked evidence, match count, match percentage, and
reason.

### Requirement: PII values are masked by default

The system SHALL prevent raw detected PII values from appearing in
normal output — reports, logs, and errors included.

#### Scenario: Scan a column containing Social Security numbers

**Given** a dataset contains values matching an SSN recognizer
**When** the user runs `datagov pii scan customers.csv`
**Then** the result identifies the column as potentially containing
`US_SSN`
**And** the report contains masked evidence
**And** the report does not contain the complete source value.

### Requirement: Custom recognizers

The system SHALL load user-defined recognizers from the PRD §10.9 YAML
schema, and SHALL provide `datagov pii recognizers list` and
`datagov pii recognizers validate <file>`.

#### Scenario: Invalid recognizer file

**Given** a recognizers file with a malformed pattern
**When** the user runs `datagov pii recognizers validate recognizers.yaml`
**Then** the process exits with code 2
**And** the error names the offending recognizer id and field.

### Requirement: CI threshold enforcement

`pii scan` SHALL support a documented fail-on threshold that terminates
with exit code 12 when exceeded.

#### Scenario: Pipeline gate

**Given** a dataset with findings above the configured threshold
**When** the scan runs in CI with the threshold flag
**Then** the process exits with code 12 and the job fails.

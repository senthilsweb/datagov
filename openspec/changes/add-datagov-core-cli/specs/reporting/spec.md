# Spec delta — `reporting` (add-datagov-core-cli)

### Requirement: Normalized report envelope

Every Milestone 0.1 command SHALL emit results in the versioned
envelope of PRD §23 (`schema_version`, `tool`, `run`, `input`,
`summary`, domain sections, `extensions`), whose JSON Schema is
committed at `docs/schema/report-v1.json` and published with releases.

#### Scenario: Envelope validates

**When** any command runs with `--output json`
**Then** the output validates against the committed schema
**And** `input.content_hash` is present for file inputs.

### Requirement: Consolidated report

`datagov report <dataset> --profile --pii` SHALL produce one envelope
combining profile and PII sections, renderable as JSON (canonical),
YAML, Markdown, and a terminal summary.

#### Scenario: Milestone demo report

**When** the user runs
`datagov report customers.parquet --profile --pii --output report.json`
**Then** `report.json` contains dataset schema, profile statistics,
detected sensitive columns, tool version, and evidence in one document
**And** `summary.status` aggregates severity across sections.

### Requirement: Renderings never diverge

Non-JSON renderings SHALL be derived from the same envelope data — a
fact present in the Markdown or terminal output MUST exist in the JSON.

#### Scenario: Cross-format consistency

**Given** one report run emitted as JSON and Markdown
**Then** every finding in the Markdown appears in the JSON envelope.

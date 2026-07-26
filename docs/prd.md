# PRD: DataGov

**Document type:** Product Requirements Document  
**Product name:** DataGov  
**Repository name:** `datagov`  
**Primary executable:** `datagov`  
**Working description:** A DataGovOps platform whose first and primary deliverable is a single-download, agent-ready command-line interface for data inspection, SQL analysis, data profiling, PII detection, data quality, lineage, policy evaluation, and governance reporting.  
**Status:** Draft  
**Target specification format:** OpenSpec  
**Primary implementation language:** Rust  
**Primary distribution model:** Precompiled standalone binaries  
**License recommendation:** Apache License 2.0  

---

## 1. Executive Summary

DataGov is a DataGovOps platform whose first deliverable is the standalone `datagov` command-line interface. It consolidates common data-governance and data-engineering tasks behind one consistent interface.

The product is designed for:

- data engineers,
- data governance engineers,
- AI engineers,
- analytics engineers,
- platform engineers,
- quality engineers,
- security and privacy teams,
- LLM coding agents,
- autonomous engineering agents,
- and CI/CD pipelines.

Users should download one executable and run commands such as:

```bash
datagov inspect customers.parquet
datagov profile customers.csv
datagov sql transpile query.sql --from spark --to duckdb
datagov pii scan customers.parquet
datagov quality check customers.parquet --rules rules/customer.yaml
datagov policy check metadata.json --policy policies/
datagov lineage extract ./dbt-project
datagov run datagov.yaml
```

The project follows a deterministic-first approach.

Native Rust capabilities should perform inspection, profiling, SQL processing, deterministic PII detection, data-quality validation, policy checks, and report generation without requiring Python, Docker, Java, Node.js, or Cargo on the end-user machine.

Advanced NLP-based PII detection and selected ecosystem integrations may use optional external backends such as Microsoft Presidio running in a container or remote service.

The product will be delivered in three phases:

1. **Core** — single native binary for local deterministic governance checks.
2. **Medium** — richer lineage, policy, pipeline, plugin, and agent integration.
3. **Full** — enterprise-grade orchestration, advanced PII engines, catalogs, observability, distributed execution, and governance evidence.

---


## 1.1 Product and Repository Naming

The product, GitHub repository, OpenSpec project, and primary executable should use the name **DataGov** / `datagov`.

Recommended naming:

```text
Product:          DataGov
GitHub repository: datagov
OpenSpec project:  datagov
Executable:        datagov
Rust workspace:    datagov
```

The repository should not be named `datagov-cli`, because the project is intended to grow beyond a command-line wrapper into a broader DataGovOps platform.

The initial product interface is the CLI:

```bash
datagov inspect customers.parquet
```

Future capabilities may be added without renaming the repository:

```text
datagov          Primary CLI
datagovd         Optional daemon or server
datagov-mcp      MCP integration
datagov-ui       Optional web interface
datagov-sdk      SDK packages
```

Internal Rust crates should use descriptive workspace names:

```text
datagov-cli
datagov-core
datagov-data
datagov-sql
datagov-pii
datagov-quality
datagov-policy
datagov-lineage
datagov-report
datagov-plugin-sdk
```

This distinction keeps the public product name concise while preserving clear internal module boundaries.


## 2. Problem Statement

Data governance work is fragmented across many tools.

A typical workflow may require:

- `qsv` or `csvkit` for CSV inspection,
- DuckDB for querying files,
- Parquet inspection tools,
- SQLGlot for SQL parsing and transpilation,
- SQL linters for quality checks,
- Presidio for PII detection,
- OpenLineage for lineage events,
- OPA or Conftest for policy checks,
- dbt artifacts for transformation lineage,
- custom Python scripts for reporting,
- and shell scripts to connect everything.

This fragmentation creates several problems:

1. Users must install and learn many unrelated tools.
2. Tool output formats are inconsistent.
3. Automation requires fragile shell scripts.
4. LLM agents must discover tool-specific syntax.
5. CI/CD pipelines become difficult to reproduce.
6. Python, Java, Node.js, and Docker dependencies increase installation friction.
7. Governance results are not emitted in one normalized schema.
8. Advanced tools are often designed as libraries rather than agent-friendly CLIs.
9. Data profiling, PII classification, policy checks, and lineage analysis are disconnected.
10. There is no lightweight, deterministic, single-binary DataGovOps CLI.

`datagov` addresses this by providing one command surface, one configuration model, one report schema, and one distribution mechanism.

---

## 3. Product Vision

Create the Unix-style governance primitive for modern data and AI engineering.

`datagov` should make common governance operations:

- composable,
- deterministic,
- scriptable,
- inspectable,
- explainable,
- agent-friendly,
- CI-friendly,
- local-first,
- and portable.

The intended experience is similar to:

- `jq` for JSON,
- `qsv` for CSV,
- `rg` for search,
- `sqlglot` for SQL,
- and `terraform` for declarative execution,

but focused on data governance workflows.

---

## 4. Product Principles

### 4.1 Single-download experience

The end user should download one precompiled executable.

The end user should not need:

- Rust,
- Cargo,
- Python,
- pip,
- Java,
- Node.js,
- or Docker

for Core capabilities.

### 4.2 Deterministic first

Rules, parsers, checksums, schemas, SQL ASTs, statistics, and explicit policies should be preferred over LLM inference.

AI-based interpretation may enrich deterministic results but must not silently replace them.

### 4.3 Local first

Core processing should run locally without sending datasets to external services.

### 4.4 Agent ready

Every command should support:

- non-interactive execution,
- stable exit codes,
- JSON output,
- machine-readable errors,
- deterministic behavior,
- stdin/stdout,
- and bounded output.

### 4.5 Explainable results

Every finding should include evidence.

Examples:

- matched pattern,
- column statistics,
- source column,
- SQL expression,
- violated rule,
- policy identifier,
- recognizer name,
- confidence score,
- and remediation guidance.

### 4.6 Composable command surface

Commands should follow a predictable hierarchy:

```text
datagov <domain> <action> [target] [flags]
```

### 4.7 Progressive capability

The same CLI should support:

- lightweight local use,
- team pipelines,
- CI/CD,
- enterprise integrations,
- and advanced engines.

### 4.8 No hidden mutations

Commands should not modify source data unless an explicit write, fix, anonymize, or export action is requested.

---

## 5. Target Users

### 5.1 Data engineer

Needs to inspect CSV and Parquet files, validate schemas, run quality checks, and understand transformations.

### 5.2 Analytics engineer

Needs to lint SQL, transpile dialects, inspect dbt artifacts, and determine model and column lineage.

### 5.3 Data governance engineer

Needs to classify sensitive data, enforce metadata policies, generate governance evidence, and integrate with catalogs.

### 5.4 AI engineer

Needs to validate training and evaluation datasets, detect sensitive content, inspect skew, and generate machine-readable reports for AI pipelines.

### 5.5 Quality engineer

Needs repeatable tests for datasets, schemas, SQL, PII detectors, and governance controls.

### 5.6 Platform engineer

Needs a portable binary that can run in CI, containers, Kubernetes jobs, developer laptops, and restricted environments.

### 5.7 LLM or coding agent

Needs deterministic tools for inspecting files, validating generated SQL, extracting lineage, checking policies, and producing bounded JSON output.

---

## 6. Primary Use Cases

1. Inspect a CSV, JSON, JSONL, Arrow, or Parquet dataset.
2. Profile columns and produce statistical summaries.
3. Query local structured files using SQL.
4. Parse and format SQL.
5. Transpile SQL between dialects.
6. Extract table and column references from SQL.
7. Validate SQL against style and safety rules.
8. Detect PII in structured datasets.
9. Classify sensitive columns using names, values, patterns, checksums, and context.
10. Anonymize or tokenize selected fields.
11. Evaluate PII detectors using ground-truth datasets.
12. Validate schemas and detect schema drift.
13. Run declarative data-quality rules.
14. Evaluate governance policies.
15. Extract lineage from SQL, dbt artifacts, and source code.
16. Emit OpenLineage-compatible events.
17. Generate consolidated governance reports.
18. Run repeatable multi-step governance pipelines.
19. Execute the same controls locally and in CI.
20. Provide a stable tool interface to LLM agents.

---

## 7. Non-Goals

The initial product will not attempt to be:

- a complete enterprise data catalog,
- a full replacement for DataHub or OpenMetadata,
- a general-purpose workflow orchestration platform,
- a distributed query warehouse,
- a complete replacement for Microsoft Presidio,
- a full replacement for dbt,
- an ETL design studio,
- a general-purpose BI platform,
- an ML training platform,
- or an autonomous governance decision-maker.

`datagov` should generate evidence and enforce explicit controls. Human review remains necessary for material governance decisions.

---

## 8. Product Editions and Delivery Phases

The product will evolve through three phases.

| Phase | Working name | Primary objective |
|---|---|---|
| Phase 1 | Core | One native binary for local deterministic governance |
| Phase 2 | Medium | Integrated lineage, policies, pipelines, agents, and extensibility |
| Phase 3 | Full | Enterprise integrations, advanced engines, observability, and distributed operation |

The product and repository remain named `datagov`, and the primary executable remains named `datagov`, in every phase.

The phase names describe capability maturity, not separate incompatible products.

---

# Phase 1: Core

## 9. Core Phase Objective

Deliver a reliable standalone Rust binary that provides immediate value for structured-data inspection, SQL processing, deterministic PII detection, data quality, and normalized reporting.

Core must run without Python or Docker.

---

## 10. Core Functional Scope

### 10.1 Dataset inspection

Supported initial formats:

- CSV,
- TSV,
- JSON,
- JSONL,
- Parquet,
- Arrow IPC where practical.

Commands:

```bash
datagov inspect customers.parquet
datagov inspect customers.csv --format json
datagov inspect - --type jsonl
```

The command should return:

- detected format,
- file size,
- row count,
- column count,
- schema,
- nullability,
- data types,
- approximate memory size,
- Parquet row groups,
- Parquet compression,
- and selected sample rows.

Example:

```bash
datagov inspect customers.parquet --output json
```

### 10.2 Dataset profiling

Commands:

```bash
datagov profile customers.csv
datagov profile customers.parquet --columns email,state,income
datagov profile customers.parquet --sample 10000
```

Metrics should include, where applicable:

- row count,
- null count,
- null percentage,
- distinct count,
- uniqueness percentage,
- minimum,
- maximum,
- mean,
- median,
- standard deviation,
- quantiles,
- string length statistics,
- top values,
- frequency distribution,
- inferred semantic type,
- and possible identifiers.

### 10.3 File querying

Provide embedded SQL querying through DataFusion or another Rust-native engine.

```bash
datagov query \
  "SELECT state, COUNT(*) AS total FROM 'customers.parquet' GROUP BY state"
```

Requirements:

- CSV and Parquet support,
- JSON output,
- table output,
- CSV output,
- stdin support where practical,
- bounded output defaults,
- explicit `--limit`,
- and query execution statistics.

### 10.4 SQL parsing

```bash
datagov sql parse query.sql
datagov sql parse - --dialect spark
```

Output should expose:

- statement type,
- tables,
- aliases,
- selected columns,
- joins,
- filters,
- grouping,
- ordering,
- functions,
- CTEs,
- subqueries,
- and a normalized AST representation.

### 10.5 SQL formatting

```bash
datagov sql format query.sql
datagov sql format query.sql --write
```

Default behavior should write formatted SQL to stdout.

Source files must only be modified when `--write` is explicitly supplied.

### 10.6 SQL transpilation

```bash
datagov sql transpile query.sql --from spark --to duckdb
datagov sql transpile - --from tsql --to postgres
```

Initial dialect priority:

- ANSI,
- PostgreSQL,
- DuckDB,
- Spark,
- Databricks,
- Snowflake,
- BigQuery,
- Trino,
- MySQL,
- SQLite,
- T-SQL.

The product should clearly report unsupported or lossy transformations.

### 10.7 SQL dependency extraction

```bash
datagov sql lineage query.sql
```

Core output:

- input tables,
- output tables,
- referenced columns,
- aliases,
- expressions,
- statement-level dependencies.

Column lineage may be partial in Core.

### 10.8 Deterministic PII detection

```bash
datagov pii scan customers.parquet
datagov pii scan customers.csv --sample 1000
datagov pii scan notes.jsonl --field text
```

Initial entity types:

- email address,
- phone number,
- IPv4 address,
- IPv6 address,
- URL,
- US Social Security number,
- credit card number,
- bank routing number,
- IBAN,
- UUID,
- MAC address,
- date of birth candidates,
- US ZIP code,
- and configurable custom identifiers.

Detection inputs:

- column names,
- sampled values,
- regular expressions,
- checksums,
- value distributions,
- context terms,
- and user-defined recognizers.

Each result must include:

- entity type,
- column or field,
- confidence,
- recognizer,
- sample evidence with masking,
- match count,
- match percentage,
- and reason.

Sensitive raw values must not appear in reports by default.

### 10.9 PII recognizer configuration

```yaml
recognizers:
  - id: us_ssn
    entity: US_SSN
    confidence: 0.85
    patterns:
      - '\b\d{3}-\d{2}-\d{4}\b'
    context:
      - ssn
      - social_security
      - taxpayer_id
    validators:
      - us_ssn
```

Commands:

```bash
datagov pii recognizers list
datagov pii recognizers validate recognizers.yaml
```

### 10.10 Data-quality checks

```bash
datagov quality check customers.parquet --rules customer-rules.yaml
```

Initial rule types:

- not null,
- unique,
- accepted values,
- regex match,
- type match,
- numeric range,
- date range,
- row-count bounds,
- distinct-count bounds,
- referential membership using a local file,
- SQL assertion,
- and custom expression.

Example:

```yaml
version: 1

dataset: customers

rules:
  - id: customer-id-required
    type: not_null
    column: customer_id
    severity: error

  - id: valid-state
    type: accepted_values
    column: state
    values: [NJ, NY, PA, CT]
    severity: warning

  - id: positive-income
    type: range
    column: annual_income
    minimum: 0
    severity: error
```

### 10.11 Schema validation and drift

```bash
datagov schema infer customers.parquet --output customer.schema.json
datagov schema validate customers.parquet --schema customer.schema.json
datagov schema diff previous.schema.json current.schema.json
```

The diff should detect:

- added columns,
- removed columns,
- type changes,
- nullability changes,
- ordering changes,
- and compatible versus incompatible changes.

### 10.12 Basic governance policies

Core should provide a simplified native policy language.

```yaml
version: 1

policies:
  - id: pii-owner-required
    description: Datasets containing PII must have an owner.
    when:
      pii_detected: true
    require:
      metadata.owner:
        not_empty: true
    severity: error

  - id: retention-required
    when:
      classification:
        in: [PII, CONFIDENTIAL]
    require:
      metadata.retention:
        not_empty: true
    severity: warning
```

Command:

```bash
datagov policy check metadata.json --policy governance-policy.yaml
```

### 10.13 Consolidated report

```bash
datagov report customers.parquet \
  --profile \
  --pii \
  --quality customer-rules.yaml \
  --metadata metadata.json \
  --policy governance-policy.yaml
```

Outputs:

- JSON,
- YAML,
- Markdown,
- terminal summary.

The canonical report format must be JSON.

### 10.14 Doctor command

```bash
datagov doctor
```

Checks:

- executable version,
- operating system,
- CPU architecture,
- available memory,
- input format support,
- configuration validity,
- optional backend availability,
- and model availability.

Core should not require external executables.

### 10.15 Version and capabilities

```bash
datagov version
datagov capabilities
```

`capabilities` should list compiled features and supported formats.

---

## 11. Core Command Model

```text
datagov
├── inspect
├── profile
├── query
├── sql
│   ├── parse
│   ├── format
│   ├── transpile
│   └── lineage
├── pii
│   ├── scan
│   └── recognizers
│       ├── list
│       └── validate
├── quality
│   └── check
├── schema
│   ├── infer
│   ├── validate
│   └── diff
├── policy
│   └── check
├── report
├── doctor
├── capabilities
└── version
```

---

## 12. Core Acceptance Criteria

Core is complete when:

1. A user can download one binary and run it without installing a runtime.
2. The binary supports macOS and Linux on ARM64 and x86-64.
3. CSV and Parquet inspection work on representative datasets.
4. Profiling returns stable machine-readable JSON.
5. SQL can be parsed, formatted, and transpiled for the priority dialects.
6. Deterministic PII scanning supports the initial entity list.
7. Reports mask sensitive values by default.
8. Quality rules return stable pass/fail results.
9. Schema drift can be identified.
10. Governance policies can evaluate normalized metadata and scan results.
11. Every command supports `--output json`.
12. Every command uses documented exit codes.
13. CI can fail based on quality or policy violations.
14. No Core command requires Python, Java, Node.js, or Docker.
15. The binary includes an SPDX software bill of materials in release artifacts.

---

# Phase 2: Medium

## 13. Medium Phase Objective

Extend `datagov` from a local inspection tool into an integrated DataGovOps CLI for repositories, dbt projects, CI/CD pipelines, agent workflows, richer policies, lineage, and optional backends.

---

## 14. Medium Functional Scope

### 14.1 Declarative pipeline execution

Introduce:

```bash
datagov run datagov.yaml
```

Example:

```yaml
version: 1

project:
  name: customer-governance

inputs:
  - id: customers
    path: data/customers.parquet

steps:
  - id: inspect
    uses: dataset.inspect
    input: customers

  - id: profile
    uses: dataset.profile
    input: customers
    with:
      sample_size: 10000

  - id: pii
    uses: pii.scan
    input: customers
    with:
      engine: native

  - id: quality
    uses: quality.check
    input: customers
    with:
      rules: rules/customers.yaml

  - id: policy
    uses: policy.check
    with:
      metadata: metadata/customers.json
      policies: policies/

outputs:
  directory: reports/
  formats: [json, markdown]
```

Requirements:

- dependency ordering,
- step outputs,
- conditional execution,
- reusable variables,
- environment overlays,
- deterministic caching,
- resumable runs,
- and manifest generation.

### 14.2 Repository scanning

```bash
datagov scan repo .
```

The scanner should discover:

- SQL files,
- dbt projects,
- CSV and Parquet assets,
- schema files,
- pipeline configurations,
- Python file reads and writes,
- Java or JavaScript data-access patterns where supported,
- and governance metadata.

### 14.3 dbt integration

```bash
datagov dbt inspect ./analytics
datagov dbt lineage ./analytics
datagov dbt validate ./analytics
```

Inputs:

- `manifest.json`,
- `catalog.json`,
- `run_results.json`,
- model SQL,
- source definitions,
- tests,
- exposures,
- metrics where available.

Outputs:

- model lineage,
- source lineage,
- column lineage where derivable,
- test coverage,
- ownership gaps,
- documentation gaps,
- and policy findings.

### 14.4 Rich SQL linting

Add SQL rule validation beyond formatting:

- prohibited `SELECT *`,
- missing aliases,
- unqualified columns,
- Cartesian joins,
- unsafe delete or update,
- non-deterministic functions,
- dialect restrictions,
- query-complexity thresholds,
- missing row limits for agent-generated queries,
- and organization-defined rules.

```bash
datagov sql lint models/
```

### 14.5 Expanded SQL lineage

Medium should support:

- CTE propagation,
- nested query dependencies,
- derived columns,
- expression-level lineage,
- selected dbt macros,
- statement sets,
- and lineage confidence.

### 14.6 OpenLineage output

```bash
datagov lineage emit report.json --format openlineage
```

The CLI should generate OpenLineage-compatible events for:

- datasets,
- jobs,
- runs,
- inputs,
- outputs,
- schemas,
- and custom governance facets.

### 14.7 Source-code lineage

```bash
datagov lineage extract ./src
```

Initial supported source patterns:

- pandas `read_csv`,
- pandas `read_parquet`,
- pandas `to_csv`,
- pandas `to_parquet`,
- PySpark reads and writes,
- DuckDB file reads,
- database connection calls,
- and configurable AST patterns.

The implementation may use Tree-sitter or integrated AST libraries.

### 14.8 Optional Presidio backend

Microsoft Presidio is not available as a native Rust CLI.

Medium should support Presidio as an optional backend through:

1. a local container,
2. an HTTP service,
3. or a user-managed endpoint.

Commands:

```bash
datagov pii scan customers.parquet --engine presidio
datagov pii compare customers.parquet --engines native,presidio
```

The CLI remains the single user-facing command.

The CLI must clearly display external runtime requirements.

### 14.9 Detector evaluation

```bash
datagov pii evaluate ground-truth.jsonl \
  --engine native \
  --output evaluation.json
```

Metrics:

- true positives,
- false positives,
- false negatives,
- precision,
- recall,
- F1,
- per-entity metrics,
- latency,
- throughput,
- and confidence distribution.

Comparison:

```bash
datagov pii evaluate ground-truth.jsonl \
  --engines native,presidio \
  --compare
```

### 14.10 Anonymization

```bash
datagov pii anonymize customers.parquet \
  --strategy-config anonymization.yaml \
  --output customers.masked.parquet
```

Strategies:

- redact,
- mask,
- hash,
- tokenize,
- replace,
- preserve format where supported,
- and deterministic pseudonymization.

No source file may be overwritten without `--write` or explicit output.

### 14.11 Advanced policy evaluation

Medium should add:

- Rego support through an embedded Rust engine or optional OPA backend,
- policy bundles,
- policy tests,
- policy explanations,
- policy severity,
- waivers,
- expiration dates,
- and policy result aggregation.

```bash
datagov policy test policies/
datagov policy check report.json --bundle policies/
```

### 14.12 Baselines and change detection

```bash
datagov baseline create report.json --name customer-v1
datagov compare customer-v1 report-current.json
```

Detect:

- schema drift,
- quality regression,
- new PII entities,
- classification changes,
- row-count anomalies,
- distribution shifts,
- and policy regressions.

### 14.13 Agent mode

```bash
datagov agent schema
datagov agent tools
datagov agent execute --request request.json
```

Agent mode should provide:

- JSON Schema definitions,
- bounded output,
- explicit side-effect metadata,
- command safety classification,
- and predictable error payloads.

Optional MCP server:

```bash
datagov mcp serve
```

Initial MCP tools:

- inspect dataset,
- profile dataset,
- parse SQL,
- transpile SQL,
- scan PII,
- check quality,
- check policy,
- and extract lineage.

### 14.14 Plugin architecture

Plugins should support external capabilities without bloating the core executable.

```bash
datagov plugin list
datagov plugin install datagov-openmetadata
datagov plugin run openmetadata.publish
```

Preferred plugin mechanisms:

- subprocess protocol using JSON over stdin/stdout,
- WebAssembly components where feasible,
- signed plugin manifests,
- version compatibility checks,
- and capability declarations.

### 14.15 CI integrations

Provide examples and native output for:

- GitHub Actions,
- GitLab CI,
- Jenkins,
- Azure DevOps,
- and generic shell pipelines.

Example:

```yaml
- name: Governance checks
  run: |
    datagov run datagov.yaml --ci
```

Medium should support:

- JUnit XML,
- SARIF,
- JSON,
- Markdown job summaries,
- and configurable failure thresholds.

---

## 15. Medium Command Additions

```text
datagov
├── run
├── scan
│   └── repo
├── dbt
│   ├── inspect
│   ├── lineage
│   └── validate
├── sql
│   └── lint
├── pii
│   ├── compare
│   ├── evaluate
│   └── anonymize
├── lineage
│   ├── extract
│   └── emit
├── baseline
│   ├── create
│   └── compare
├── policy
│   └── test
├── agent
│   ├── schema
│   ├── tools
│   └── execute
├── mcp
│   └── serve
└── plugin
    ├── list
    ├── install
    ├── remove
    └── run
```

---

## 16. Medium Acceptance Criteria

Medium is complete when:

1. Users can execute a multi-step YAML pipeline.
2. dbt artifacts can be inspected and converted into lineage.
3. SQL linting supports configurable safety and style rules.
4. OpenLineage-compatible output is available.
5. Native and Presidio PII results can be compared.
6. Detector evaluation produces precision, recall, and F1.
7. Selected PII can be anonymized.
8. Baselines identify governance regressions.
9. GitHub Actions can publish SARIF or JUnit results.
10. Agent mode exposes stable schemas.
11. The MCP server supports read-only governance tools.
12. Plugins can be installed and invoked safely.
13. External backends are always explicit.
14. Core workflows continue to work without optional backends.
15. Pipeline runs produce a reproducible manifest.

---

# Phase 3: Full

## 17. Full Phase Objective

Make `datagov` an enterprise-grade governance execution plane capable of integrating advanced detection engines, catalogs, observability systems, lineage platforms, remote runners, and organizational control frameworks.

---

## 18. Full Functional Scope

### 18.1 Advanced native PII and NPI detection

Add optional native or downloaded ONNX models for:

- person names,
- locations,
- organizations,
- medical identifiers,
- account identifiers,
- mortgage-related NPI,
- and domain-specific entities.

Architecture:

```text
deterministic recognizers
        +
column semantic inference
        +
ONNX NER model
        +
context scoring
        +
policy classification
```

Commands:

```bash
datagov models list
datagov models install pii-ner-small
datagov pii scan notes.parquet --model pii-ner-small
```

Models must be versioned separately from the executable.

### 18.2 Multi-engine privacy evaluation

Supported engine categories:

- native deterministic engine,
- native ONNX engine,
- Presidio,
- remote REST detectors,
- user-defined plugins,
- and LLM-based evaluators where explicitly configured.

```bash
datagov pii benchmark benchmark.yaml
```

The product should support:

- accuracy comparisons,
- cost metrics,
- latency metrics,
- token usage where relevant,
- model version tracking,
- and reproducible evaluation manifests.

### 18.3 Catalog integrations

Initial integration targets:

- OpenMetadata,
- DataHub,
- Marquez,
- Apache Atlas where feasible,
- and generic REST or file-based catalog export.

Commands:

```bash
datagov catalog publish report.json --target openmetadata
datagov catalog pull dataset-id --target datahub
```

Supported publishing objects:

- datasets,
- schemas,
- classifications,
- owners,
- glossary terms,
- quality results,
- PII findings,
- policies,
- lineage,
- and governance evidence.

### 18.4 Knowledge graph export

```bash
datagov graph export report.json --format cypher
datagov graph export report.json --format rdf
datagov graph export report.json --format graphml
```

Graph entities:

- datasets,
- columns,
- SQL models,
- source files,
- transformations,
- tests,
- business terms,
- owners,
- classifications,
- policies,
- systems,
- and runtime jobs.

Relationships:

- reads,
- writes,
- derives from,
- transforms,
- tested by,
- governed by,
- classified as,
- owned by,
- documents,
- and impacts.

### 18.5 Enterprise policy bundles

Full policy capabilities:

- policy package registry,
- signed policies,
- policy inheritance,
- organization and domain overlays,
- exceptions,
- approvals,
- waiver workflows,
- evidence requirements,
- version history,
- and policy impact reports.

### 18.6 Governance evidence packs

```bash
datagov evidence build datagov.yaml \
  --framework internal-ai-governance \
  --output evidence/
```

Evidence packs may contain:

- dataset inventory,
- schema,
- lineage,
- quality results,
- PII findings,
- detector evaluation results,
- policy decisions,
- waivers,
- run manifests,
- software versions,
- signatures,
- and audit logs.

The product should not claim formal regulatory compliance.

It should generate evidence that organizations may use within their compliance processes.

### 18.7 Observability

`datagov` should emit OpenTelemetry signals.

Telemetry:

- command duration,
- step duration,
- rows scanned,
- bytes scanned,
- error count,
- quality pass/fail count,
- PII findings,
- detector latency,
- model version,
- policy pass/fail count,
- cache hits,
- and external backend calls.

Supported outputs:

- OTLP,
- JSON logs,
- local trace file,
- and OpenTelemetry Collector.

Sensitive data must never be included in telemetry by default.

### 18.8 Remote and distributed execution

Introduce an optional execution service.

```bash
datagov remote submit datagov.yaml --runner governance-cluster
datagov remote status <run-id>
datagov remote logs <run-id>
```

Possible environments:

- Kubernetes Jobs,
- CI runners,
- managed internal execution workers,
- and isolated sandbox workers.

Local mode remains supported.

### 18.9 Large dataset operation

Add:

- streaming scans,
- predicate pushdown,
- partition pruning,
- object storage,
- S3-compatible storage,
- Azure Blob Storage,
- Google Cloud Storage,
- sampling strategies,
- distributed partition plans,
- and resumable jobs.

### 18.10 Database connectors

Read-only inspection initially:

- PostgreSQL,
- MySQL,
- SQL Server,
- Snowflake,
- BigQuery,
- Databricks SQL,
- Trino,
- ClickHouse,
- DuckDB,
- and SQLite.

Commands:

```bash
datagov source inspect postgres://...
datagov source profile postgres://... --table public.customers
datagov source lineage postgres://... --query query.sql
```

Credentials must be supplied through environment variables, operating-system secret stores, or approved secret providers.

Credentials must never appear in reports or logs.

### 18.11 AI-assisted explanation

Optional LLM capability:

```bash
datagov explain report.json --provider configured-provider
```

Permitted uses:

- summarize findings,
- explain policy failures,
- suggest remediation,
- group related issues,
- and generate human-readable reports.

LLMs must not alter deterministic findings.

The report must distinguish:

- observed facts,
- deterministic conclusions,
- model-generated interpretations,
- and recommendations.

### 18.12 Enterprise server mode

```bash
datagov serve
```

Possible interfaces:

- REST API,
- gRPC,
- MCP,
- event queue,
- and webhook receiver.

Server mode should reuse the same execution engine and report schema as the CLI.

### 18.13 Signed reports and provenance

Reports should optionally include:

- content hashes,
- input hashes,
- executable version,
- model versions,
- policy versions,
- configuration hash,
- run timestamp,
- execution environment,
- and digital signature.

---

## 19. Full Command Additions

```text
datagov
├── models
│   ├── list
│   ├── install
│   ├── remove
│   └── verify
├── pii
│   └── benchmark
├── catalog
│   ├── publish
│   └── pull
├── graph
│   └── export
├── evidence
│   └── build
├── remote
│   ├── submit
│   ├── status
│   ├── logs
│   └── cancel
├── source
│   ├── inspect
│   ├── profile
│   └── lineage
├── explain
├── serve
└── sign
```

---

## 20. Full Acceptance Criteria

Full is complete when:

1. Advanced PII models can be installed and verified.
2. Multiple PII engines can be benchmarked reproducibly.
3. Governance metadata can be published to at least two catalog platforms.
4. Lineage and governance relationships can be exported as a graph.
5. Evidence packs include immutable run manifests.
6. OpenTelemetry traces and metrics are emitted.
7. Large Parquet datasets can be scanned using streaming or pushdown.
8. At least three database connectors support read-only inspection.
9. Remote execution can run the same pipeline definition as local execution.
10. LLM explanations are clearly separated from deterministic evidence.
11. Reports can be cryptographically signed.
12. Server and CLI execution produce compatible report schemas.
13. Sensitive data is excluded from telemetry and logs by default.
14. Plugin and model artifacts can be signature verified.
15. Backward compatibility is maintained for stable Core commands.

---

## 21. Consolidated Architecture

```text
                         ┌──────────────────────────┐
                         │       datagov CLI        │
                         │ clap + async runtime     │
                         └────────────┬─────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          │                           │                           │
          ▼                           ▼                           ▼
┌──────────────────┐       ┌──────────────────┐       ┌──────────────────┐
│ Dataset Engine   │       │ Governance Engine│       │ Integration Layer│
│                  │       │                  │       │                  │
│ CSV              │       │ PII recognizers  │       │ Presidio         │
│ JSON/JSONL       │       │ Quality rules    │       │ OpenLineage      │
│ Arrow            │       │ Policy engine    │       │ dbt              │
│ Parquet          │       │ Classification   │       │ Catalogs         │
│ DataFusion       │       │ Evidence         │       │ OpenTelemetry    │
└──────────────────┘       └──────────────────┘       └──────────────────┘
          │                           │                           │
          └───────────────────────────┼───────────────────────────┘
                                      ▼
                         ┌──────────────────────────┐
                         │ Normalized Report Model  │
                         │ JSON / YAML / Markdown   │
                         │ SARIF / JUnit / OL       │
                         └──────────────────────────┘
```

---

## 22. Recommended Rust Technology Stack

| Concern | Recommended technology |
|---|---|
| CLI | `clap` |
| Error handling | `anyhow`, `thiserror` |
| Serialization | `serde`, `serde_json`, `serde_yaml` |
| Async runtime | `tokio` |
| Logging | `tracing` |
| HTTP | `reqwest` |
| CSV | `csv` |
| Arrow | Apache Arrow Rust |
| Parquet | Apache Parquet Rust |
| Query engine | Apache DataFusion |
| DataFrames | Polars, only where justified |
| SQL parsing | SQLGlot Rust implementation or `sqlparser-rs`, based on capability validation |
| SQL linting | integrated rules; evaluate `sqruff` crate reuse |
| AST parsing | Tree-sitter |
| Regex | `regex` |
| Multi-pattern matching | `aho-corasick` |
| Parallelism | `rayon` |
| Hashing | `sha2`, `blake3` |
| Policy | native DSL; evaluate `regorus` for Rego |
| ONNX | `ort` or suitable ONNX Runtime binding |
| Plugins | JSON subprocess protocol and/or Wasmtime |
| Telemetry | OpenTelemetry Rust |
| Terminal output | `comfy-table` or equivalent |
| Progress | `indicatif` |
| Testing | Rust unit/integration tests, golden tests, snapshot tests |
| Packaging | GitHub Actions, Homebrew tap, release archives |

Technology selection must be validated through spikes before being locked.

---

## 23. Canonical Report Model

Every major command should be able to emit a normalized envelope.

```json
{
  "schema_version": "1.0",
  "tool": {
    "name": "datagov",
    "version": "0.1.0"
  },
  "run": {
    "id": "run-identifier",
    "started_at": "ISO-8601 timestamp",
    "completed_at": "ISO-8601 timestamp",
    "duration_ms": 1200
  },
  "input": {
    "uri": "customers.parquet",
    "format": "parquet",
    "content_hash": "sha256-value"
  },
  "summary": {
    "status": "failed",
    "errors": 1,
    "warnings": 2,
    "info": 5
  },
  "dataset": {},
  "profile": {},
  "pii": {},
  "quality": {},
  "schema": {},
  "lineage": {},
  "policy": {},
  "evidence": {},
  "extensions": {}
}
```

The schema must be versioned and published.

---

## 24. Exit Codes

| Code | Meaning |
|---:|---|
| 0 | Success; no threshold violations |
| 1 | Execution or internal error |
| 2 | Invalid arguments or configuration |
| 3 | Input not found or unreadable |
| 4 | Unsupported input or dialect |
| 10 | Quality threshold failed |
| 11 | Policy threshold failed |
| 12 | PII threshold failed |
| 13 | Schema validation failed |
| 14 | Lineage extraction incomplete beyond configured threshold |
| 20 | Optional backend unavailable |
| 21 | Authentication or authorization failure |
| 22 | Plugin or model verification failed |

Commands should allow configurable fail conditions.

---

## 25. Security and Privacy Requirements

1. Data must remain local by default.
2. External calls require explicit configuration.
3. Raw PII must be masked in logs and reports.
4. Sample values must be excluded by default or masked.
5. Temporary files must use secure permissions.
6. Credentials must not be accepted as plain CLI flags where shell history may expose them.
7. Plugin and model downloads should support signature verification.
8. Container and remote backends must be explicitly identified in output.
9. Reports must record whether external services were used.
10. Telemetry must not contain source data.
11. Anonymization must require an explicit output target or write flag.
12. Destructive commands must support dry-run mode.
13. The CLI should support offline operation for Core.
14. Dependency releases should include an SBOM.
15. Security advisories should follow a documented disclosure process.

---

## 26. Performance Requirements

Initial targets for Core on a modern developer laptop:

- startup under 150 milliseconds where practical,
- inspect Parquet metadata without scanning all rows,
- profile one million rows in under 10 seconds for common schemas,
- stream data instead of loading entire files where practical,
- bounded memory use,
- parallel scanning configurable by `--threads`,
- and cancellation on `SIGINT`.

Performance metrics must be benchmarked and published by release.

---

## 27. Usability Requirements

1. Commands must have consistent flags.
2. `--help` must include examples.
3. Human-readable output must be concise.
4. JSON must be stable and complete.
5. Commands must support `--quiet`.
6. Commands must support `--verbose`.
7. Commands must support `--output`.
8. Commands must support stdin where applicable.
9. Errors must include remediation guidance.
10. Potentially destructive actions must be explicit.
11. Default output must avoid overwhelming terminals.
12. Full results should be available through JSON files.

---

## 28. Configuration

Configuration precedence:

1. CLI flags,
2. environment variables,
3. project configuration,
4. user configuration,
5. built-in defaults.

Suggested locations:

```text
./datagov.yaml
./.datagov/config.yaml
$XDG_CONFIG_HOME/datagov/config.yaml
~/.config/datagov/config.yaml
```

Secrets must not be stored in normal project configuration.

---

## 29. Distribution

Release targets:

```text
datagov-darwin-arm64
datagov-darwin-x86_64
datagov-linux-arm64
datagov-linux-x86_64
datagov-windows-x86_64.exe
```

Installation options:

```bash
brew install senthilsweb/tap/datagov
```

```bash
curl -fsSL https://example.org/datagov/install.sh | sh
```

```bash
docker pull ghcr.io/senthilsweb/datagov
```

Docker is a distribution option, not a Core runtime requirement.

Cargo is needed by contributors building from source, not by normal end users.

---

## 30. OpenSpec Project Structure

Recommended repository structure:

```text
datagov/
├── openspec/
│   ├── project.md
│   ├── specs/
│   │   ├── cli-interface/
│   │   │   └── spec.md
│   │   ├── dataset-inspection/
│   │   │   └── spec.md
│   │   ├── dataset-profiling/
│   │   │   └── spec.md
│   │   ├── sql-processing/
│   │   │   └── spec.md
│   │   ├── pii-detection/
│   │   │   └── spec.md
│   │   ├── data-quality/
│   │   │   └── spec.md
│   │   ├── schema-management/
│   │   │   └── spec.md
│   │   ├── governance-policy/
│   │   │   └── spec.md
│   │   ├── lineage/
│   │   │   └── spec.md
│   │   ├── pipeline-runtime/
│   │   │   └── spec.md
│   │   ├── reporting/
│   │   │   └── spec.md
│   │   ├── agent-integration/
│   │   │   └── spec.md
│   │   └── plugin-system/
│   │       └── spec.md
│   └── changes/
├── crates/
│   ├── datagov-cli/
│   ├── datagov-core/
│   ├── datagov-data/
│   ├── datagov-sql/
│   ├── datagov-pii/
│   ├── datagov-quality/
│   ├── datagov-policy/
│   ├── datagov-lineage/
│   ├── datagov-report/
│   └── datagov-plugin-sdk/
├── policies/
├── recognizers/
├── examples/
├── tests/
├── benchmarks/
├── docs/
├── Cargo.toml
├── justfile
├── Dockerfile
└── README.md
```

---

## 31. Initial OpenSpec Capabilities

The PRD should be decomposed into the following OpenSpec capabilities:

1. `cli-interface`
2. `dataset-inspection`
3. `dataset-profiling`
4. `file-query`
5. `sql-processing`
6. `sql-lineage`
7. `pii-detection`
8. `pii-recognizers`
9. `data-quality`
10. `schema-management`
11. `governance-policy`
12. `reporting`
13. `pipeline-runtime`
14. `dbt-integration`
15. `source-lineage`
16. `openlineage-export`
17. `presidio-backend`
18. `detector-evaluation`
19. `anonymization`
20. `agent-integration`
21. `mcp-server`
22. `plugin-system`
23. `catalog-integration`
24. `observability`
25. `remote-execution`
26. `evidence-pack`
27. `model-management`
28. `report-signing`

---

## 32. Example OpenSpec Requirement Style

### Requirement: Single-binary Core execution

The system SHALL provide precompiled standalone binaries for supported operating systems and architectures.

#### Scenario: Run dataset inspection without a language runtime

**Given** a user has downloaded the supported `datagov` binary  
**And** Python, Java, Node.js, Docker, and Cargo are not installed  
**When** the user runs:

```bash
datagov inspect customers.parquet
```

**Then** the command completes using only capabilities embedded in the binary  
**And** the command emits a human-readable summary  
**And** `--output json` emits a valid normalized JSON response.

### Requirement: PII values are masked by default

The system SHALL prevent raw detected PII values from appearing in normal output.

#### Scenario: Scan a column containing Social Security numbers

**Given** a dataset contains values matching an SSN recognizer  
**When** the user runs:

```bash
datagov pii scan customers.csv
```

**Then** the result identifies the column as potentially containing `US_SSN`  
**And** the report contains masked evidence  
**And** the report does not contain the complete source value.

### Requirement: Optional Presidio execution is explicit

The system SHALL not invoke Presidio unless the user or project configuration explicitly selects the Presidio engine.

#### Scenario: Use the native engine by default

**Given** Presidio is configured as an available backend  
**When** the user runs:

```bash
datagov pii scan customers.parquet
```

**Then** the native engine is used  
**And** Presidio is not called.

#### Scenario: Explicitly select Presidio

**When** the user runs:

```bash
datagov pii scan customers.parquet --engine presidio
```

**Then** the CLI invokes the configured Presidio backend  
**And** the report records the backend name and version  
**And** the report records that an external execution backend was used.

---

## 33. Delivery Priorities

### Phase 1 priority order

1. CLI and normalized report envelope
2. CSV and Parquet inspection
3. Dataset profiling
4. SQL parse, format, and transpile
5. Deterministic PII detection
6. Data-quality checks
7. Schema validation and drift
8. Basic policy engine
9. Consolidated report
10. Cross-platform release automation

### Phase 2 priority order

1. Declarative pipeline runtime
2. dbt inspection and lineage
3. Rich SQL linting
4. OpenLineage export
5. Detector evaluation
6. Optional Presidio backend
7. Anonymization
8. Baselines
9. Agent and MCP interfaces
10. Plugin system

### Phase 3 priority order

1. Native ONNX PII models
2. Multi-engine benchmarks
3. Catalog integrations
4. Observability
5. Governance evidence packs
6. Database connectors
7. Graph export
8. Remote execution
9. Enterprise policy bundles
10. Signed reports and provenance

---

## 34. Key Risks

### 34.1 SQL dialect coverage

A Rust SQL transpiler may not match Python SQLGlot’s dialect depth.

Mitigation:

- perform a technical spike,
- create dialect conformance tests,
- support explicit fallback plugins,
- and communicate unsupported transformations.

### 34.2 PII accuracy

A deterministic engine may have lower recall for names, locations, and context-dependent entities.

Mitigation:

- focus Core on structured-data entities,
- use column-level context,
- expose confidence and evidence,
- support custom recognizers,
- provide evaluation tooling,
- and add ONNX and Presidio engines later.

### 34.3 Binary size

Arrow, Parquet, DataFusion, ONNX, and plugin runtimes may create a large executable.

Mitigation:

- use Cargo features,
- keep advanced engines optional,
- distribute models separately,
- and evaluate a compact Core build.

### 34.4 Tool duplication

Reimplementing every capability from qsv, DuckDB, SQLGlot, Presidio, and OPA would create excessive scope.

Mitigation:

- implement only governance-focused capabilities,
- reuse stable Rust crates,
- use plugins for broad ecosystem functionality,
- and maintain clear non-goals.

### 34.5 Policy-language complexity

A custom policy language could become difficult to maintain.

Mitigation:

- begin with a small governance-specific DSL,
- publish a JSON Schema,
- add Rego compatibility in Medium,
- and avoid building a general programming language.

### 34.6 False perception of compliance

Users may treat reports as proof of regulatory compliance.

Mitigation:

- state clearly that `datagov` produces evidence and checks,
- avoid compliance certification claims,
- and make policy ownership explicit.

---

## 35. Success Metrics

### Core

- installation success rate,
- time to first successful command,
- number of supported file formats,
- SQL transpilation success rate,
- PII detector precision and recall by entity,
- profiling throughput,
- binary startup time,
- and CI adoption.

### Medium

- number of executed pipelines,
- dbt projects analyzed,
- lineage edges generated,
- detector comparisons completed,
- MCP tool calls,
- plugin installations,
- and policy regressions detected.

### Full

- datasets published to catalogs,
- governance evidence packs generated,
- remote runs completed,
- observability signals emitted,
- models installed,
- signed reports produced,
- and enterprise policy bundles adopted.

Telemetry must be opt-in unless running in a user-controlled observability environment.

---

## 36. Core MVP Demonstration

The first public demonstration should show:

```bash
datagov inspect customers.parquet
```

```bash
datagov profile customers.parquet --output json
```

```bash
datagov sql transpile query.sql --from spark --to duckdb
```

```bash
datagov pii scan customers.parquet
```

```bash
datagov quality check customers.parquet --rules rules.yaml
```

```bash
datagov report customers.parquet \
  --profile \
  --pii \
  --quality rules.yaml \
  --output report.json
```

The demo should conclude with one consolidated governance report containing:

- dataset schema,
- profile statistics,
- detected sensitive columns,
- quality results,
- policy results,
- tool version,
- and evidence.

---

## 37. Recommended Initial Milestone

The first implementation milestone should be narrower than the complete Core phase.

### Milestone 0.1

Deliver:

- `datagov inspect`,
- `datagov profile`,
- `datagov sql parse`,
- `datagov sql format`,
- `datagov sql transpile`,
- `datagov pii scan`,
- `datagov report`,
- JSON output,
- macOS ARM64 binary,
- Linux x86-64 binary,
- and GitHub release automation.

Exclude from Milestone 0.1:

- anonymization,
- Presidio,
- dbt,
- OpenLineage,
- plugins,
- MCP,
- remote execution,
- and catalogs.

This milestone validates the core premise:

> A useful DataGovOps workflow can be delivered as one portable, deterministic, agent-ready executable.

---

## 38. Open Questions

1. Should the Rust SQL implementation use `sqlglot-rust`, `sqlparser-rs`, or a hybrid abstraction?
2. Should DataFusion or Polars be the primary profiling engine?
3. Should the Core policy DSL be YAML-only or support JSON as well?
4. Which PII entity types are mandatory for the first release?
5. Should the binary include Arrow IPC support in Milestone 0.1?
6. Should `datagov query` be included in the first milestone?
7. Should plugin execution prefer Wasm or JSON subprocesses?
8. Should report IDs be random UUIDs or content-derived identifiers?
9. Should model downloads use Hugging Face, GitHub Releases, or an independent registry?
10. Which catalog integration should be implemented first?
11. Should the MCP server ship in the main binary behind a feature flag?
12. What backward-compatibility policy should apply before version 1.0?

---

## 39. Recommended Decisions

Unless a technical spike contradicts them, use the following defaults:

- Rust for the product implementation.
- `clap` for the CLI.
- DataFusion and Arrow for native file querying.
- Native deterministic PII in Core.
- Presidio as an optional Medium backend.
- ONNX models as an optional Full capability.
- YAML for user configuration.
- JSON as the canonical report format.
- A governance-specific policy DSL in Core.
- Rego compatibility in Medium.
- JSON subprocess plugins first.
- Wasm plugins after the protocol stabilizes.
- GitHub Releases and Homebrew for initial distribution.
- Apache License 2.0.
- No external telemetry by default.

---

## 40. Final Product Definition

`datagov` is a portable DataGovOps command-line platform that consolidates data inspection, profiling, SQL analysis, sensitive-data detection, quality validation, lineage extraction, governance policy evaluation, and evidence generation.

Its defining properties are:

- one user-facing CLI,
- one normalized report model,
- native Core execution,
- deterministic-first governance,
- optional advanced engines,
- explicit external dependencies,
- agent-friendly interfaces,
- and progressive adoption from laptop to enterprise.

The three-phase strategy is:

```text
Core
  Native, standalone and deterministic

Medium
  Integrated, extensible and agent-ready

Full
  Enterprise-connected, observable and scalable
```

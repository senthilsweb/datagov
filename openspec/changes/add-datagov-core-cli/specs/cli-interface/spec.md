# Spec delta — `cli-interface` (add-datagov-core-cli)

### Requirement: Composable command surface

The CLI SHALL follow the `datagov <domain> <action> [target] [flags]`
hierarchy (PRD §4.6, §11), and every command SHALL support
`--output json`, `--quiet`, and `--verbose`.

#### Scenario: JSON output on any command

**Given** any Milestone 0.1 command
**When** it is invoked with `--output json`
**Then** stdout contains exactly one JSON document conforming to the
versioned report envelope
**And** all diagnostics are on stderr.

### Requirement: Single-binary Core execution

The system SHALL provide precompiled standalone binaries for supported
operating systems and architectures.

#### Scenario: Run dataset inspection without a language runtime

**Given** a user has downloaded the supported `datagov` binary
**And** Python, Java, Node.js, Docker, and Cargo are not installed
**When** the user runs `datagov inspect customers.parquet`
**Then** the command completes using only capabilities embedded in the
binary
**And** emits a human-readable summary
**And** `--output json` emits a valid normalized JSON response.

### Requirement: Documented exit codes

Every command SHALL terminate with an exit code from the PRD §24 table;
no other codes are permitted.

#### Scenario: Missing input file

**Given** `customers.parquet` does not exist
**When** the user runs `datagov inspect customers.parquet`
**Then** the process exits with code 3
**And** stderr contains a machine-readable error with remediation
guidance.

### Requirement: Stdin as input

Commands operating on a single input SHALL accept `-` as the target,
reading from stdin, with the format supplied via `--type` when it cannot
be inferred.

#### Scenario: Inspect a JSONL stream

**When** the user runs `cat events.jsonl | datagov inspect - --type jsonl`
**Then** inspection completes without a temporary file requirement in
the interface contract.

### Requirement: No hidden mutations

No command SHALL modify a source file unless `--write` or an explicit
output target is supplied.

#### Scenario: Formatting without --write

**Given** a file `query.sql`
**When** the user runs `datagov sql format query.sql`
**Then** the formatted SQL is written to stdout
**And** `query.sql` is byte-identical to its state before the command.

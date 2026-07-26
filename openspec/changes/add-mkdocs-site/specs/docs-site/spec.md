# Spec delta — `docs-site` (add-mkdocs-site)

### Requirement: Task-organized docs site

`docs/` SHALL contain Home, Getting Started, Installation, Commands,
Tutorials, Use Cases, Configuration, CI/CD, and FAQ pages following the
shared senthilsweb style guide (plain English, "at the end you will
have" openers, copy-pasteable tested commands, relative links inside
`docs/`, absolute GitHub URLs outside), published independently at the
project's GitHub Pages URL.

#### Scenario: Five-minute path

**WHEN** a reader with the binary installed follows Getting Started
**THEN** they reach a working `inspect` (or `profile`/`query`/`sql`)
result without reading any other page.

### Requirement: Single-owner topics

Each topic SHALL have exactly one home page; the PRD and JSON Schema
are linked (absolute GitHub URLs), never duplicated into site prose.

#### Scenario: No drift

**WHEN** a command's flags or exit codes change
**THEN** exactly one docs page (`commands.md`) needs editing.

### Requirement: Safe publishing

The docs workflow SHALL trigger only on docs-related paths, build with
`mkdocs build --strict` (broken links fail the build), deploy via
`actions/deploy-pages`, and SHALL NOT trigger the release or CI
workflows — a `docs:`-scoped commit releases nothing.

#### Scenario: Docs-only push

**WHEN** a page changes on `main`
**THEN** the docs site rebuilds and deploys
**AND** no version tag, GitHub Release, or `cargo test` CI run is
triggered by that push.

### Requirement: Documented, not aspirational

Every command example in the docs site SHALL have been actually run
against a real fixture before being committed; no invented output,
filenames, or numbers.

#### Scenario: A command that doesn't exist yet

**WHEN** a PRD-scoped command (e.g. `report`) has not yet landed
**THEN** it does not appear in Commands, Tutorials, or Getting Started
**AND** the build-status table on Home states its actual status.

# Spec delta — `release-distribution` (add-datagov-core-cli)

### Requirement: CI quality gates

Every push and pull request touching Rust sources SHALL run
`cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test`; a
failure blocks merge.

#### Scenario: Clippy warning introduced

**Given** a commit introducing a clippy warning
**When** CI runs
**Then** the CI check fails.

### Requirement: Tag-driven cross-platform release

Pushing a `v*` tag SHALL produce a GitHub Release containing, at
minimum, `datagov-darwin-arm64` and `datagov-linux-x86_64` (remaining
PRD §29 targets best-effort for 0.1), each named per the PRD §29
convention.

#### Scenario: Release v0.1.0

**When** the tag `v0.1.0` is pushed
**Then** CI builds the release binaries without manual steps
**And** attaches them to the GitHub Release.

### Requirement: SBOM and checksums

Every release SHALL include an SPDX software bill of materials and
SHA-256 checksums for all binaries (PRD §12.15, §25.14).

#### Scenario: Verify a downloaded binary

**Given** a published release
**When** a user downloads `datagov-darwin-arm64` and its checksum file
**Then** the SHA-256 matches
**And** an SPDX document covering the binary's dependencies is attached
to the release.

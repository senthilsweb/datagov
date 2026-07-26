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

### Requirement: One-line install script

The repository SHALL provide `install.sh` (PRD §29) that detects the
caller's OS/architecture, resolves a release (latest by default, or a
pinned tag via `DATAGOV_VERSION`), downloads the matching binary and
its checksum from that release, refuses to install on a checksum
mismatch, and installs to a user-writable directory with no implicit
`sudo`.

#### Scenario: Install the latest release

**Given** a published GitHub Release exists
**When** a user runs
`curl -fsSL https://raw.githubusercontent.com/senthilsweb/datagov/main/install.sh | sh`
**Then** the script installs the binary matching the caller's OS/arch
to `~/.local/bin` (or `$DATAGOV_INSTALL_DIR`)
**And** `datagov version` succeeds afterward.

#### Scenario: Checksum mismatch aborts installation

**Given** a downloaded binary whose SHA-256 does not match its
published checksum file
**When** the install script runs
**Then** it exits non-zero without installing anything
**And** the error names the expected and actual checksums.

#### Scenario: No release published yet

**Given** no GitHub Release exists
**When** the install script runs
**Then** it fails with a clear, actionable message rather than a raw
HTTP error, pointing at the Releases page.

# datagov

A DataGovOps platform whose first deliverable is a single-download,
agent-ready command-line interface for data inspection, SQL analysis,
data profiling, PII detection, data quality, lineage, policy evaluation,
and governance reporting.

```bash
datagov inspect customers.parquet
datagov profile customers.csv
datagov sql transpile query.sql --from spark --to duckdb
datagov pii scan customers.parquet
datagov report customers.parquet --profile --pii --output report.json
```

One portable Rust binary — no Python, Java, Node.js, Docker, or Cargo on
the end-user machine. Deterministic first, local first, masked by
default, agent- and CI-friendly.

## Status

**In construction — Milestone 0.1, Bolt 5 of 7 done.** The project
follows AI-DLC: specs before code, tracked in
[`add-datagov-core-cli`](openspec/changes/add-datagov-core-cli/proposal.md).
Landed so far: the CLI frame and JSON report envelope (Bolt 1),
`inspect` (Bolt 2), `profile`/`query` (Bolt 3), `sql parse|format|
transpile` (Bolt 4), and `pii scan` + recognizers (Bolt 5). `report`
and `doctor` (Bolt 6) are still to come; release proving (Bolt 7) is
in progress.

**No final `v0.1.0` release yet**, but `v0.1.0-rc.*` pre-release
snapshots are published as each bolt lands — see
[Try it now](#try-it-now) below to install one and run what exists
today.

## Try it now

```bash
DATAGOV_VERSION=v0.1.0-rc.5 curl -fsSL \
  https://raw.githubusercontent.com/senthilsweb/datagov/main/install.sh | sh
```

(`install.sh` detects your OS/arch, downloads the matching binary from
GitHub Releases, verifies its SHA-256 checksum, and installs to
`~/.local/bin` — no sudo. Drop `DATAGOV_VERSION` once a final `v0.1.0`
ships to always get the latest stable release.)

Or build from source (needs a Rust toolchain — `rustup.rs` if you
don't have one):

```bash
git clone https://github.com/senthilsweb/datagov.git
cd datagov
cargo build --release
./target/release/datagov version
```

### Take it for a spin

Real synthetic fixtures ship in [`examples/`](examples/README.md) — no
need to bring your own data to try the first command:

```bash
./target/release/datagov inspect examples/customers.parquet
./target/release/datagov inspect examples/customers.csv --output json | jq .

# PII-shaped columns (email, phone) are masked even in raw sample rows
./target/release/datagov inspect examples/customers.parquet --output json \
  | jq '.dataset.sample_rows[0] | {email, phone}'
```

## Documentation

The full wiki is published at
[senthilsweb.github.io/datagov](https://senthilsweb.github.io/datagov/):
Getting Started, Installation, Commands, Tutorials, Use Cases,
Configuration, CI/CD, Deployment, and FAQ. Source lives under
[`docs/`](docs/), built with MkDocs Material.

## Where things live

| Path | Purpose |
|---|---|
| `docs/prd.md` | The PRD — source of truth for product scope |
| `openspec/` | AI-DLC change specs (proposal → design → tasks → spec) |
| `AGENTS.md` | Conventions for every agent and human in this repo |
| `crates/` | Rust workspace — `datagov-cli`, `datagov-core`, `datagov-data`, … |
| `examples/` | Real synthetic fixtures for trying commands and golden tests |
| `install.sh` | One-line installer, works today against pre-release tags |
| `.github/workflows/` | CI quality gates + tag-driven binary releases |

## Process

Every non-trivial change goes through `openspec/changes/<name>/` before
and during implementation — see [`openspec/project.md`](openspec/project.md)
for the lifecycle (`proposed → approved → implemented → verified →
archived`), bolt structure, and HARD/SOFT eval contract.

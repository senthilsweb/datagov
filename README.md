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

**In construction — Milestone 0.1, Bolt 2 of 7 done.** The project
follows AI-DLC: specs before code, tracked in
[`add-datagov-core-cli`](openspec/changes/add-datagov-core-cli/proposal.md).
Landed so far: the CLI frame, the normalized JSON report envelope,
documented exit codes (Bolt 1), and `datagov inspect` — format
detection, schema/type inference, and masked sample rows across
CSV/TSV/JSON/JSONL/Parquet (Bolt 2). `profile`, `query`, `sql`,
`pii scan`, and `report` are still to come.

**No tagged release yet** — see [Try it now](#try-it-now) below to
build from source and run what exists today.

## Try it now

No release is published yet, so install from source (needs a Rust
toolchain — `rustup.rs` if you don't have one):

```bash
git clone https://github.com/senthilsweb/datagov.git
cd datagov
cargo build --release
./target/release/datagov version
```

Once a release is published, the same binary will install with:

```bash
curl -fsSL https://raw.githubusercontent.com/senthilsweb/datagov/main/install.sh | sh
```

(`install.sh` detects your OS/arch, downloads the matching binary from
GitHub Releases, verifies its SHA-256 checksum, and installs to
`~/.local/bin` — no sudo. Pin a version with `DATAGOV_VERSION=v0.1.0`.)

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

## Where things live

| Path | Purpose |
|---|---|
| `docs/prd.md` | The PRD — source of truth for product scope |
| `openspec/` | AI-DLC change specs (proposal → design → tasks → spec) |
| `AGENTS.md` | Conventions for every agent and human in this repo |
| `crates/` | Rust workspace — `datagov-cli`, `datagov-core`, `datagov-data`, … |
| `examples/` | Real synthetic fixtures for trying commands and golden tests |
| `install.sh` | One-line installer (active once a release is published) |
| `.github/workflows/` | CI quality gates + tag-driven binary releases |

## Process

Every non-trivial change goes through `openspec/changes/<name>/` before
and during implementation — see [`openspec/project.md`](openspec/project.md)
for the lifecycle (`proposed → approved → implemented → verified →
archived`), bolt structure, and HARD/SOFT eval contract.

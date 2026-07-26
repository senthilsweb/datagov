# AGENTS — datagov

Repo-level conventions. Every coding agent (and human) working anywhere in
this repo follows these. Every non-trivial change goes through
`openspec/changes/<name>/` first — see `openspec/project.md` for the AI-DLC
lifecycle. The product definition lives in `docs/prd.md`; specs are derived
from it, code is derived from specs — never the other way around.

## Layout

```
datagov/
├── docs/
│   └── prd.md        # the PRD — source of truth for product scope
├── openspec/         # AI-DLC change specs; check status with /openspec-status
│   ├── project.md    # project context + lifecycle rules
│   ├── specs/        # living capability specs (merged from verified changes)
│   ├── changes/      # in-flight changes (proposal → design → tasks → specs)
│   └── adr/          # architecture decision records
├── crates/           # Rust workspace (created by the first approved change)
│   ├── datagov-cli/  #   thin clap adapter — no business logic in commands
│   ├── datagov-core/ #   shared engine: config, report envelope, exit codes
│   └── datagov-*/    #   one crate per domain (data, sql, pii, quality, …)
├── recognizers/      # built-in + example PII recognizer configs (YAML)
├── policies/         # example governance policies (YAML)
├── examples/         # sample datasets + SQL used in docs and golden tests
├── benchmarks/       # performance benchmarks (published per release)
└── .github/workflows # CI, cross-platform release, SBOM
```

## Common engineering requirements (all code)

1. **Deterministic first, local first, no hidden mutations.** Rules,
   parsers, checksums, ASTs, and statistics over inference. Core commands
   never call the network and never modify source data unless an explicit
   `--write` / `--output` target is given. Destructive actions support
   dry-run. (PRD §4, §25)
2. **PII is masked everywhere by default.** Raw detected values must not
   appear in reports, logs, errors, telemetry, test snapshots, or fixtures.
   Evidence is masked samples + match statistics. (PRD §10.8, §25)
3. **One normalized report envelope.** Every major command emits the
   versioned JSON envelope in PRD §23 via `--output json`. JSON is the
   canonical format; human output is a rendering of it, never a separate
   code path with different facts. Schema changes bump `schema_version`.
4. **Documented, stable exit codes.** The table in PRD §24 is a contract.
   New failure modes map to an existing code or extend the table through a
   spec change — never ad-hoc `std::process::exit` values. Exit codes are
   covered by integration tests.
5. **Structured logging via `tracing` only.** No bare `println!`/`eprintln!`
   for diagnostics — stdout belongs to command output (a machine may be
   parsing it). Diagnostics go to stderr through the shared `tracing`
   subscriber (JSON lines with `--verbose`, quiet by default). Every
   command honours `--quiet`, `--verbose`, `--output`.
6. **File-header comment convention.** Every source file opens with a
   `//!` (module) or header comment block: file name, description with a
   numbered list of what the file does, and — for binaries/entry points —
   the environment variables it reads.
7. **No secrets or config in source.** Configuration follows the
   precedence chain in PRD §28 (flags → env → project → user → defaults).
   No hard-coded endpoints, model ids, or credentials — not even as
   "example" fallbacks. Missing required config is a clear startup error
   (exit code 2).
8. **Typed contracts.** Data crossing any boundary (CLI output, config
   files, recognizer/rule/policy YAML, report JSON) is a serde-validated
   schema with a published JSON Schema where the PRD requires one.
9. **Evals from the spec, before or alongside the code.** Acceptance
   criteria become executable tests (unit, golden, snapshot, integration)
   in the same bolt as the feature. HARD criteria (deterministic —
   output shape, exit codes, masking, transpile fidelity) block
   `implemented → verified`; SOFT criteria (e.g. detector recall bands)
   are logged and reviewed, not blocking.
10. **Conventional Commits.** `type(scope): subject` (`feat:`, `fix:`,
    `docs:`, `chore:`, …). Release automation derives semantic versions
    from them (`fix` → patch, `feat` → minor, `BREAKING CHANGE:` → major).
11. **Rust hygiene gates.** `cargo fmt --check`, `cargo clippy -D warnings`,
    and `cargo test` must pass locally before pushing to `main` — the same
    three gates CI enforces.

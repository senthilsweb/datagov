# Bolt 1 implementation brief — workspace skeleton, envelope, exit codes

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). Contract: implement exactly this; where reality forces a
> deviation, stop and report it as a proposed **Correction** — do not
> silently improvise. Read first: `AGENTS.md`, `openspec/project.md`,
> `openspec/changes/add-datagov-core-cli/{proposal,design,tasks}.md`,
> `docs/prd.md` §23, §24, §27, §28.

## Ground rules

- Toolchain: stable Rust 1.97 (Homebrew, no rustup). Edition 2024.
- Do NOT commit, do NOT edit anything under `openspec/` or `docs/prd.md`.
- Every source file opens with the AGENTS.md §6 header comment (`//!`
  module docs: file name, description, numbered list of what it does;
  entry points also list env vars read).
- Diagnostics only via `tracing` to stderr. `println!` is allowed solely
  for command output on stdout.
- Finish only when `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass.

## Workspace

Root `Cargo.toml`: workspace resolver "2" (or "3"), members `crates/*`.
`[workspace.package]`: version `0.1.0`, edition `2024`, license
`Apache-2.0`, repository `https://github.com/senthilsweb/datagov`.
Shared deps under `[workspace.dependencies]`.

Crates (all inherit workspace package fields):

| Crate | Bolt 1 content |
|---|---|
| `crates/datagov-core` | everything below |
| `crates/datagov-cli` | binary `datagov` (set `[[bin]] name = "datagov"`) |
| `crates/datagov-data` | stub: header docs + empty lib, no deps |
| `crates/datagov-sql` | stub |
| `crates/datagov-pii` | stub |
| `crates/datagov-report` | stub |

Dependencies (workspace-level): `clap` 4 (derive), `serde` (derive),
`serde_json`, `serde_yaml` 0.9, `thiserror`, `anyhow` (cli only),
`tracing`, `tracing-subscriber` (env-filter + json features), `uuid`
(v7 + serde), `time` (formatting + serde, RFC 3339), `schemars`,
`sha2`. Dev: `assert_cmd`, `predicates`, `tempfile`, `jsonschema`.

## `datagov-core`

### `report` module — the PRD §23 envelope

Typed structs, serde + schemars derives on all of them:

- `Report { schema_version: String ("1.0"), tool: Tool, run: Run,
  input: Option<Input>, summary: Summary, sections: Sections,
  extensions: BTreeMap<String, serde_json::Value> }` — serialize
  `sections` flattened so the JSON shape matches PRD §23 exactly
  (`dataset`, `profile`, `pii`, … as top-level keys).
- `Tool { name: "datagov", version: env!("CARGO_PKG_VERSION") }`.
- `Run { id, started_at, completed_at, duration_ms }` — `id` is UUIDv7
  (Q6); timestamps RFC 3339 via `time`.
- `Input { uri, format, content_hash: Option<String> }` — provide
  `content_hash_sha256(path)` helper using `sha2` (used from Bolt 2 on).
- `Summary { status: Status (success|warning|failed), errors, warnings,
  info: u64 }`.
- `Sections`: one `Option<T>` field per PRD §23 domain key. For Bolt 1
  each `T` is an empty `#[non_exhaustive]` placeholder struct
  (`DatasetSection`, `ProfileSection`, `PiiSection`, `QualitySection`,
  `SchemaSection`, `LineageSection`, `PolicySection`,
  `EvidenceSection`) — domain bolts flesh them out. `None` fields are
  skipped in serialization. Command code never touches
  `serde_json::Value` except via `extensions`.
- `ReportBuilder`: constructs `Run` (start on new, finish on build),
  computes `duration_ms`, defaults summary to success.

### JSON Schema + drift test

- Generate the schema for `Report` with `schemars` and commit it at
  `docs/schema/report-v1.json` (pretty-printed, trailing newline).
- Test: regenerate in-memory, compare to the committed file;
  mismatch → failing test with a message telling the developer to
  regenerate and to bump `schema_version` on breaking changes.

### `exit` module — PRD §24 contract

`ExitCode` enum with explicit discriminants: Success=0, Internal=1,
InvalidArgs=2, InputNotFound=3, UnsupportedInput=4, QualityFailed=10,
PolicyFailed=11, PiiFailed=12, SchemaFailed=13, LineageIncomplete=14,
BackendUnavailable=20, AuthFailed=21, VerificationFailed=22. Unit test
pins every discriminant (the table is a contract).

### `error` module

`thiserror` enum `DatagovError` with variants carrying remediation
text; `impl DatagovError { fn exit_code(&self) -> ExitCode }` — unit
test the complete mapping (Internal=1, InvalidArgs=2, InputNotFound=3,
UnsupportedInput=4 for Bolt 1; add variants only as domains land).
Errors render to stderr: human one-liner with remediation by default,
single JSON object `{"code", "exit_code", "message", "remediation"}`
when `--output json`.

### `logging` module

`init(quiet: bool, verbose: bool)`: tracing-subscriber writing to
**stderr only**; `--verbose` → DEBUG level, JSON lines; default → WARN,
compact human; `--quiet` → ERROR only. Respect `DATAGOV_LOG` env filter
override. Configure-once guard (second init is a no-op).

### `config` module

`Config` (serde): Bolt 1 fields `output: Option<OutputFormat>`,
`threads: Option<usize>` — small on purpose; the chain matters, not
the surface. Resolution precedence (PRD §28), first hit wins per field:
CLI flags (applied by caller) → env `DATAGOV_*` → `./datagov.yaml` →
`./.datagov/config.yaml` → `$XDG_CONFIG_HOME/datagov/config.yaml` →
`~/.config/datagov/config.yaml` → defaults. Loader takes the start dir
and env as parameters for testability. Unit tests with `tempfile`
cover: each layer alone, override order, malformed YAML → error
mapping to exit 2. Never read secrets from these files.

### `mask` module

`Masked` newtype: wraps a sensitive `String`; `Display`/`Serialize`
emit only the masked form (≤2 leading chars + `…` + ≤2 trailing for
len ≥ 8; fully `●●●` below that); **no method returns the inner
value**. Unit test: serialized/displayed output never contains a
sampled full value. (Per-entity mask styles arrive in Bolt 5 — keep
this generic.)

## `datagov-cli`

- clap derive. Global flags on the root: `--output <json|table>`
  (default `table`), `--quiet`, `--verbose` (conflicting quiet/verbose
  → clap error). Subcommands for Bolt 1: `version`, `capabilities`.
- Thin adapters only: parse → call a function in a lib crate or
  `datagov-core` → render. No logic in `main.rs` beyond wiring.
- `version`: human → `datagov <version>`; `--output json` → full
  envelope (no `input`, no sections; summary success).
- `capabilities`: reports compiled commands (`version`,
  `capabilities`), supported formats (empty list in Bolt 1 — honest),
  and enabled cargo features; human table + envelope with the payload
  under `extensions.capabilities`.
- Exit path: all errors funnel through `DatagovError` →
  `ExitCode`; clap usage errors exit 2 (clap 4 default is 2 — pin it
  with a test anyway).

## Tests (pinned before/alongside code — AI-DLC)

Integration (`crates/datagov-cli/tests/cli.rs`, assert_cmd):

1. `datagov version` → exit 0, stdout contains the crate version.
2. `datagov version --output json` → exit 0, stdout parses as JSON,
   validates against `docs/schema/report-v1.json` (jsonschema crate),
   `run.id` parses as UUID, `schema_version == "1.0"`.
3. `datagov --bogus-flag` and `datagov nonexistent-cmd` → exit 2.
4. `datagov capabilities --output json` → exit 0, envelope valid,
   `extensions.capabilities.commands` contains both commands.
5. `--quiet` + `--verbose` together → exit 2.
6. stdout/stderr separation: with `--output json`, stdout is exactly
   one JSON document (diagnostics, if any, on stderr).

Unit: exit-code discriminants, error→code mapping, config precedence,
masking, schema drift, logging double-init.

## Report back

Summary of what was built, deviations proposed as Corrections (if
any), the three gate commands' final output, and the test count.

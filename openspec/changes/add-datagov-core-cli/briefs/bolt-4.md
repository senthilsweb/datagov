# Bolt 4 implementation brief — `datagov sql parse | format | transpile`

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). Contract: implement exactly this; where reality forces a
> deviation, stop and report it as a proposed **Correction** — do not
> silently improvise. Read first: `AGENTS.md`, `openspec/project.md`,
> `openspec/changes/add-datagov-core-cli/{proposal,design,tasks}.md`,
> `openspec/changes/add-datagov-core-cli/specs/sql-processing/spec.md`,
> `docs/prd.md` §10.4–§10.6, §34.1, §38.1.
> This builds on Bolts 1–3 — read the existing `datagov-core` (report
> envelope, exit codes, error mapping) and how `datagov query`
> (Bolt 3) put its results under `extensions.query` before writing
> anything; follow the same pattern here, don't invent a new one.

## This bolt carries real technical risk — verify before you build

The inception gate locked **`sqlglot-rust`** as the SQL engine
*without* the spike that was originally planned (see proposal.md's
Bolt 0 record). The explicit, pre-agreed contingency: **if
`sqlglot-rust` can't do the job, the fallback (`sqlparser-rs` or a
hybrid) requires an architect-authored Correction + ADR — you do not
have authority to silently switch SQL engines.** Your job in this
bolt has two phases. Do not skip straight to Phase 2.

### Phase 1 — Verify (do this first, budget real time for it)

1. Search crates.io / the wider Rust ecosystem for a crate that
   actually implements sqlglot's parse/transpile model in Rust (the
   exact crate name may not literally be `sqlglot-rust` — check what's
   actually published and maintained). Do not assume a name from
   memory; confirm it exists, what version is current, and read its
   docs/examples.
2. Write a throwaway smoke test: parse, format, and transpile **one
   trivial statement** (`SELECT a, b FROM t WHERE a > 1`) across at
   least 3 of the 11 priority dialects (ANSI, PostgreSQL, DuckDB,
   Spark, Databricks, Snowflake, BigQuery, Trino, MySQL, SQLite,
   T-SQL) — enough to confirm the crate's basic dialect-parameterized
   API actually works end-to-end before investing in the full command
   surface.
3. **If this fails** (no viable crate exists, or it fundamentally
   can't do dialect-parameterized parse/transpile): **stop. Do not
   build a fallback yourself.** Report exactly what you found —
   crate(s) considered, what broke, error messages — as your final
   report, and do not proceed to Phase 2. This is not a failure on
   your part; it's exactly the scenario the inception gate already
   planned for.
4. **If it works**, proceed to Phase 2, and note in your final report
   which crate/version you're using and what its basic capability
   looks like.

### Phase 2 — Build (only if Phase 1 succeeds)

## Ground rules (same as Bolts 1–3)

- Do NOT run any git command. Leave the tree dirty for review.
- Do NOT edit anything under `openspec/` or `docs/prd.md`.
- Every source file opens with the AGENTS.md file-header comment.
- Done only when `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass.

## New crate: `datagov-sql`

Add the verified crate as a dependency here (not in `datagov-data` —
SQL processing is its own domain crate per the workspace layout in
design.md). Public API roughly:

```rust
pub fn parse(sql: &str, dialect: &str) -> Result<SqlParseResult, DatagovError>;
pub fn format(sql: &str, dialect: &str) -> Result<String, DatagovError>;
pub fn transpile(sql: &str, from: &str, to: &str) -> Result<SqlTranspileResult, DatagovError>;
```

An unrecognized dialect name (not one of the 11 priority dialects, or
a name the underlying crate itself rejects) → `DatagovError::
UnsupportedInput` (exit 4). Malformed/unparseable SQL → `DatagovError::
InvalidArgs` (exit 2) — same categorization Bolt 3 used for bad SQL in
`query`, stay consistent.

### Result types (in `datagov-sql`, not `datagov-core` — these are
SQL-specific, following the same precedent as Bolt 3's `QueryResult`
living in `datagov-data` rather than the shared report envelope)

```rust
pub struct SqlParseResult {
    pub dialect: String,
    pub statement_type: String,     // "select" | "insert" | "update" | "delete" | "create_table" | "other" | ...
    pub tables: Vec<TableRef>,
    pub columns: Vec<String>,       // top-level selected columns, best-effort
    pub joins: Vec<JoinRef>,
    pub filters: Vec<String>,       // rendered filter expressions as text
    pub group_by: Vec<String>,
    pub order_by: Vec<String>,
    pub ctes: Vec<String>,          // CTE names
    pub ast: serde_json::Value,     // see note below
}
pub struct TableRef { pub name: String, pub alias: Option<String> }
pub struct JoinRef { pub kind: String, pub table: String, pub on: Option<String> }

pub struct SqlTranspileResult {
    pub from_dialect: String,
    pub to_dialect: String,
    pub output_sql: String,
    pub warnings: Vec<TranspileWarning>,  // lossy/unsupported constructs — never silent
}
pub struct TranspileWarning { pub construct: String, pub message: String }
```

**On `ast`**: don't invent a universal AST schema — that's a
significant undertaking outside this bolt's scope. Use whatever the
underlying crate's own parsed-statement representation is: if it
implements `Serialize`, serialize it directly; if not, capture its
`Debug`/native string rendering as a JSON string value. Document
whichever you do in the module header comment. The PRD's "normalized
AST representation" requirement is satisfied by exposing the crate's
real parse result, not a hand-rolled abstraction over it.

## `datagov-cli` commands

- **`datagov sql parse <path|-> [--dialect <name>]`**: envelope with
  the result under `extensions.sql_parse` (same pattern as Bolt 3's
  `extensions.query`). No `--dialect` → default to `ansi`.
- **`datagov sql format <path> [--dialect <name>] [--write]`**:
  stdout by default; the source file is modified **only** with
  `--write` (HARD — a test must assert byte-identical file + unchanged
  mtime when `--write` is absent).
- **`datagov sql transpile <path|-> --from <dialect> --to <dialect>`**:
  writes the transpiled SQL to stdout (or the full envelope with
  `--output json`, result under `extensions.sql_transpile`, including
  `warnings`). Warnings are surfaced in **both** renderings — never
  swallowed in the human/stdout path.
- All three read from stdin via `-`, same convention as `inspect`.

## Dialect conformance corpus

Commit `examples/sql/<dialect>/*.sql` for **all 11 priority
dialects** — at least 3 statements per dialect: a plain `SELECT`
with a `WHERE`, a `JOIN` + `GROUP BY`, and one dialect-idiomatic
construct that actually differs across engines (e.g. Snowflake's
`QUALIFY`, BigQuery's backtick-quoted identifiers, T-SQL's `TOP n`,
MySQL's backtick identifiers, Spark's `LATERAL VIEW` — pick something
real and testable, not contrived). Add a handful of **cross-dialect
transpile pairs** mirroring the PRD's own examples (`spark → duckdb`,
`tsql → postgres`, plus 2-3 more of your choosing) with the expected
output committed alongside for round-trip golden tests.

**Report a full per-dialect coverage matrix in your final report** —
works / partial / fails, with the specific error for anything that
isn't clean — even for dialects you don't end up building golden
tests for. Do not silently narrow scope to only the dialects that
happened to work without telling the architect exactly what you
found and where.

## Tests

`crates/datagov-cli/tests/sql.rs`:
- Golden tests for `parse` on at least the dialects confirmed working
  in Phase 1, plus as many of the remaining corpus dialects as prove
  viable — normalize the `run` block as usual.
- `format` without `--write`: source file byte-identical + mtime
  unchanged after the command runs; with `--write`: file updated to
  the formatted SQL.
- `transpile` round-trips the corpus's cross-dialect pairs; a
  deliberately lossy/unsupported construct produces a `warnings` entry
  in the envelope (not silent), and the human-readable path also
  surfaces it.
- Exit codes: malformed SQL → 2; missing input file → 3; unknown
  dialect name → 4.
- stdin (`-`) works for all three subcommands.

## Report back

Phase 1 findings first and clearly (crate chosen, version, what you
verified) — if Phase 1 failed, that section IS your report, stop
there. If Phase 2 happened: what was built per crate; the full
per-dialect coverage matrix; any deviations as **Proposed
Corrections**; the three gate commands' final output; total test
count and status.

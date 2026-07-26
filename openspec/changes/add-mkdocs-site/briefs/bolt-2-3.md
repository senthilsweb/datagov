# Bolt 2+3 implementation brief — MkDocs site migration and content

> Authored by the architect (Fable) for the implementation agent
> (Sonnet 5). This is a docs-only change: **do not touch any Rust
> source, `crates/`, `Cargo.toml`, or anything under `openspec/`**.
> Read first: `openspec/changes/add-mkdocs-site/{proposal,design,
> tasks}.md` (the full plan and every resolved decision is there —
> don't re-derive it), `~/.claude/skills/mkdocs-site/SKILL.md` (the
> packaged pattern this change follows), and the canonical style guide
> at `~/work/ai-agents/agents/job-scout/docs/style-guide.md` (voice,
> page conventions, link rules — follow it, don't copy it into this
> repo).

## Ground rules

- Do NOT run any git command (no add/commit). Leave the tree dirty
  for architect review.
- Do NOT touch `openspec/`, `docs/prd.md`, or `docs/schema/`.
- Do NOT touch any Rust source or `Cargo.toml`.
- **Every command shown in every page must be actually run** against
  the real fixtures in `examples/` (build the release binary first:
  `cargo build --release` from the repo root) before being written
  down. Never invent output, numbers, or filenames — this is a style
  guide rule, not optional.
- Done only when `mkdocs build --strict` passes locally with zero
  broken links (`pip install "mkdocs-material==9.*" "mkdocs<2"` first).

## What gets removed

`docs/_config.yml` (Jekyll config from the earlier, superseded setup)
and the current `docs/index.md`, `docs/installation.md`,
`docs/commands.md` — all replaced by the structure below, not merged.
`docs/prd.md` and `docs/schema/report-v1.json` stay exactly as they
are, untouched, linked from the new site via absolute GitHub blob
URLs (`https://github.com/senthilsweb/datagov/blob/main/docs/prd.md`
etc.) — never duplicated or reformatted into a page.

## `mkdocs.yml` (repo root) and `.github/workflows/docs.yml`

Copy both **exactly** from `openspec/changes/add-mkdocs-site/
design.md`'s templates — `site_name: datagov`, `site_url:
https://senthilsweb.github.io/datagov/` (this is the only URL this
project documents or promotes — see proposal.md open question 3 for
why, a real GitHub Pages platform constraint the owner already
resolved; don't second-guess it), `repo_url`/`repo_name` for
`senthilsweb/datagov`, and the nav below. The workflow file is a
verbatim copy of `~/work/agent-job-matcher/.github/workflows/
docs.yml`'s pattern (mkdocs<2 pin, `mkdocs build --strict`,
`actions/upload-pages-artifact` → `actions/deploy-pages`, triggered
only on `docs/**`/`mkdocs.yml`/its own path so a docs-only commit
never runs `ci.yml` or `release.yml`).

## Nav and page-by-page content (see design.md for the full
rationale — this is the actionable summary)

```yaml
nav:
  - Home: index.md
  - Getting Started: getting-started.md
  - Installation: installation.md
  - Commands: commands.md
  - Tutorials: tutorials.md
  - Use Cases: use-cases.md
  - Configuration: configuration.md
  - CI/CD: ci-cd.md
  - Deployment: deployment.md
  - FAQ: faq.md
```

- **`index.md`** — what datagov is, the in-progress status (currently:
  `version`, `capabilities`, `inspect`, `profile`, `query`, `sql
  parse/format/transpile`, `pii scan`, `pii recognizers list/validate`
  are built — check `openspec/changes/add-datagov-core-cli/tasks.md`
  for the exact current bolt status before writing this, don't assume
  it's still Bolt 4), a quickstart snippet, links to the PRD/schema/
  repo, ends with `Next:` → Getting Started.
- **`getting-started.md`** — 3 independent "5-minute paths", cheapest
  first, each ending in a real result: (1) `inspect` a bundled fixture
  (zero setup beyond the binary); (2) `profile` + `query` together on
  the same fixture; (3) `sql transpile` one file across two dialects.
  Opens with "At the end you will have...".
- **`installation.md`** — pre-release curl-install
  (`DATAGOV_VERSION=v0.1.0-rc.N curl -fsSL .../install.sh | sh` — use
  the actual latest `v0.1.0-rc.*` tag, check `git tag -l` or GitHub
  Releases for the current one) and build-from-source. Note clearly
  no final release exists yet.
- **`commands.md`** — **one reference page, every command explained
  in full** (this was an explicit owner decision — not a table, not
  terse): for each of `inspect`, `profile`, `query`, `sql parse`,
  `sql format`, `sql transpile`, `pii scan`, `pii recognizers list`,
  `pii recognizers validate`: full signature, every flag with what it
  does, exit codes specific to that command, and one tested example.
- **`tutorials.md`** — end-to-end recipes chaining commands (distinct
  from Getting Started's atomic paths): e.g. "check a dataset for PII
  before sharing it" (`inspect` → `pii scan`, same file), "compare
  SQL across two dialects for a migration" (`sql parse` both, `sql
  transpile` between them). Only commands that exist today.
- **`use-cases.md`** — scenario framing adapted from PRD §6 (Primary
  Use Cases), first-person/practitioner voice, each scenario naming
  the specific commands that address it *today* — do not describe
  PRD-scoped capabilities that aren't built yet (no `report`, no
  `quality check`, etc.).
- **`configuration.md`** — PRD §28's precedence chain (flags → env →
  project config → user config → defaults), described accurately
  including the known gap: the CLI's global `--output` flag does not
  yet consult `config::load` for its default (check
  `crates/datagov-cli/src/cli.rs` and `datagov-core::config` to
  confirm this is still true before stating it — don't assume it's
  unchanged since Bolt 1).
- **`ci-cd.md`** — a mermaid diagram of `ci.yml` (push/PR →
  fmt/clippy/test) and `release.yml` (tag `v*` → `build-required`/
  `build-optional` → SBOM → Publish Release), plus the real story:
  the `macos-13` runner capacity issue and the `continue-on-error`+
  matrix `needs` bug found while proving the release pipeline
  (2026-07-26) — pull the actual details from the dated commits on
  `main` (`git log --oneline --grep="fix(ci)"`) rather than
  paraphrasing from memory.
- **`deployment.md`** — how *this documentation site itself* gets
  built and deployed (this is the resolved meaning of "Deployment" —
  distinct from `ci-cd.md`, which covers the product's own pipeline):
  `mkdocs build --strict`, the Pages workflow, `mkdocs serve` for
  local preview, where GitHub Pages settings live. Include the
  custom-domain-inheritance note from `~/.claude/skills/mkdocs-site/
  SKILL.md` briefly, since it explains why the site is reachable at
  more than one URL.
- **`faq.md`** — titled exactly "FAQ". Seed with real, already-answered
  questions from this project's own history, each a short answer
  linking to the spec/design doc that recorded the full reasoning
  (never duplicate the reasoning into the FAQ answer itself): why
  Apache 2.0 not MIT (`openspec/changes/add-datagov-core-cli/
  proposal.md`'s inception gate), why `sqlglot-rust` not
  `sqlparser-rs` (`design.md`'s Bolt 4 verification), why `query` only
  supports CSV/Parquet (PRD §10.3), why PII masking exists in
  `inspect` before `pii scan` was built (design.md's Bolt 2 note on
  `sensitivity::is_heuristically_sensitive`).

## README

No change needed — datagov's README is already a front door (intro,
status, install, layout, links out) per the style guide's own
definition. Confirm this is still true; if the README has drifted
into explaining something a new wiki page now owns, trim it, but
don't restructure it wholesale.

## Report back

What was built; the exact current bolt/command status you found and
used for Home's status table; confirmation every command shown was
actually run (list any you couldn't verify and why); the
`mkdocs build --strict` output (must be clean); and any deviations
from this brief as **Proposed Corrections** with reasoning.

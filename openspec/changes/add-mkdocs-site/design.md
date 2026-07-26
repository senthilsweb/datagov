# Design — `add-mkdocs-site`

## `mkdocs.yml` (repo root) — copied from `agent-job-matcher`, only
the identifying fields change

```yaml
# Wiki site, published to GitHub Pages by .github/workflows/docs.yml.
# Pattern shared with senthilsweb/agent-job-matcher and
# senthilsweb/ai-agents; writing standard:
# https://senthilsweb.github.io/ai-agents/style-guide/
# Spec: openspec/changes/add-mkdocs-site/
site_name: datagov
site_url: https://www.senthilsweb.com/datagov/   # pending open question 3
repo_url: https://github.com/senthilsweb/datagov
repo_name: senthilsweb/datagov
edit_uri: edit/main/docs/

theme:
  name: material
  features: [navigation.sections, navigation.footer, navigation.top,
             content.code.copy, search.suggest]
  palette:
    - media: "(prefers-color-scheme: light)"
      scheme: default
      primary: blue grey
      accent: indigo
      toggle: { icon: material/brightness-7, name: Switch to dark mode }
    - media: "(prefers-color-scheme: dark)"
      scheme: slate
      primary: blue grey
      accent: indigo
      toggle: { icon: material/brightness-4, name: Switch to light mode }

nav:
  - Home: index.md
  - Getting Started: getting-started.md
  - Installation: installation.md
  - Commands: commands.md
  - Tutorials: tutorials.md
  - Use Cases: use-cases.md
  - Configuration: configuration.md
  - CI/CD: ci-cd.md
  - FAQ: faq.md

markdown_extensions:
  - admonition
  - attr_list
  - tables
  - footnotes
  - toc: { permalink: true }
  - pymdownx.details
  - pymdownx.tabbed: { alternate_style: true }
  - pymdownx.superfences:
      custom_fences:
        - { name: mermaid, class: mermaid,
            format: !!python/name:pymdownx.superfences.fence_code_format }

extra_javascript:
  - https://unpkg.com/tablesort@5.3.0/dist/tablesort.min.js
  - javascripts/tablesort.js
```

## `.github/workflows/docs.yml` — copied verbatim from
`agent-job-matcher/.github/workflows/docs.yml`

Same triggers (`push: branches: [main], paths: [docs/**, mkdocs.yml,
.github/workflows/docs.yml]` + `workflow_dispatch`), same
`concurrency: {group: docs, cancel-in-progress: true}`, same build
(`pip install "mkdocs-material==9.*" "mkdocs<2"` then
`mkdocs build --strict`), same `actions/upload-pages-artifact@v3` →
`actions/deploy-pages@v4`. No datagov-specific changes needed beyond
the header comment.

## GitHub Pages source switch

Current state (from the earlier Jekyll setup this change replaces):
`build_type: legacy`, `source: {branch: main, path: /docs}`. Target
state, matching sibling repos: `build_type: workflow`. This is a repo
settings change (`PUT /repos/senthilsweb/datagov/pages` with
`{"build_type": "workflow"}`, or Settings → Pages → Source → "GitHub
Actions" in the UI) made once this change is approved — not yet done.

## Page-by-page content plan

- **`index.md`** — mostly today's content (in-progress banner,
  quickstart snippet, build-status table), restructured to end with a
  `Next:` link to Getting Started instead of being the catch-all.
- **`getting-started.md`** (new) — style guide's "5-minute paths":
  each path complete on its own, cheapest first. Proposed paths: (1)
  `inspect` a bundled fixture, zero setup beyond having the binary;
  (2) `profile` + `query` together on the same fixture; (3)
  `sql transpile` one file across two dialects. Every command tested
  before being written down, per the style guide's rule.
- **`installation.md`** — today's content, restyled (admonition for
  "no final release yet", the same curl one-liner and build-from-source
  paths).
- **`commands.md`** — today's content, one page (see open question 2),
  reference-style rather than tutorial-style (that's what Tutorials is
  for) — signature, flags, exit codes, one example per command.
- **`tutorials.md`** (new) — end-to-end recipes distinct from Getting
  Started's atomic paths, e.g. "check a dataset for PII before sharing
  it" (inspect → pii scan, chained), "compare two SQL dialects for a
  migration" (parse both, diff the AST, transpile). Only built from
  commands that exist today.
- **`use-cases.md`** (new) — scenario framing pulled from PRD §6
  (Primary Use Cases), rewritten in first person / practitioner voice
  per the style guide, each scenario linking to the specific commands
  that address it today (not the full PRD list — only what's built).
- **`configuration.md`** (new) — PRD §28's precedence chain (flags →
  env → project config → user config → defaults), documented
  accurately including the known gap noted in Bolt 1's review (the
  CLI's `--output` flag doesn't yet consult `config::load` for its
  default) — docs describe actual behavior, not aspiration.
- **`ci-cd.md`** (new) — the style guide's mandatory page for 2+
  workflows: a left-to-right mermaid diagram (`push tag v*` →
  `release.yml` → `build-required`/`build-optional` →
  `Publish GitHub Release`; `push main` → `ci.yml` →
  `fmt/clippy/test`), plus the real story of the `macos-13` capacity
  issue and the `continue-on-error`+matrix `needs` bug found while
  proving the release pipeline (2026-07-26) — the style guide asks for
  "failures seen so far" in operational pages, and this project has a
  concrete, instructive one.
- **`faq.md`** (new) — seeded from real questions already answered in
  this project's own history: why Apache 2.0 not MIT (links the
  inception-gate Correction), why `sqlglot-rust` not `sqlparser-rs`
  (links design.md's Bolt 4 verification), why `query` only supports
  CSV/Parquet (links PRD §10.3), why PII masking exists even in
  `inspect` before `pii scan` was built (links the Bolt 2 heuristic
  note). Each answer short, linking to the spec/design doc that
  recorded the full reasoning — never duplicating it.

## Skill design: `~/.claude/skills/mkdocs-site/SKILL.md`

Frontmatter `description` triggers on: building or restructuring a
docs site, "mkdocs", "docs site", "publish docs", "GitHub Pages" for
markdown documentation (distinct from this session's Jekyll attempt,
which existed only because this skill didn't yet exist).

Body (target: page-set decision rules + templates, not prose duplicate
of the style guide):
1. When to use / when not to use (a project needs *any* published
   docs site; not for single-file READMEs).
2. The mandatory-page rules (FAQ always; Configuration if a config
   file/precedence chain exists; CI/CD if 2+ workflows exist; Runbook
   if anything runs on a schedule/automatically) and the flexible ones
   (Getting Started, Installation, Commands/Surfaces, Tutorials, Use
   Cases — adapt names to the product's shape: a CLI gets Commands, a
   service gets Surfaces/API).
3. The `mkdocs.yml` and `docs.yml` templates verbatim (from this
   design doc), with a note on the two fields that change per repo
   (`site_name`, `repo_url`/`repo_name`, `nav`).
4. Pages setup steps (source = GitHub Actions).
5. A link to the canonical style guide
   (https://senthilsweb.github.io/ai-agents/style-guide/, and the
   local path `ai-agents/agents/job-scout/docs/style-guide.md` as a
   fallback) as the single source of truth for voice/writing rules —
   not duplicated here, per the style guide's own stated principle.

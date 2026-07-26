# Proposal: `add-mkdocs-site` — replace the ad hoc docs site with the shared MkDocs pattern

> Status: **PROPOSED** — 2026-07-26. Awaiting owner resolution of the
> open questions below, then status → APPROVED.
> Owner: @senthilsweb
> Source: owner request ("I need it in mkdocs which is what we have
> been following... standard navigation system... skill file for docs
> structure"), 2026-07-26.

## Why

Bolt "docs kickoff" (2026-07-26, same day) stood up a minimal docs
site using plain Jekyll and GitHub Pages' legacy branch/folder build,
because at the time no docs convention had been located for this
project. It turns out one already exists and is live in two sibling
repos:

- **`agent-job-matcher`** — `openspec/changes/project-wiki/` (APPROVED
  2026-07-14): MkDocs Material, `docs/` with a task-organized page set,
  a GitHub Actions workflow (`mkdocs build --strict` →
  `actions/deploy-pages`), Pages source set to "GitHub Actions".
- **`ai-agents`** — the same pattern, plus a canonical
  [style guide](https://senthilsweb.github.io/ai-agents/style-guide/)
  ("the writing standard for every senthilsweb repo... other repos
  link here instead of copying it") covering voice, page structure,
  command/example discipline, and link conventions.

Neither the pattern nor the style guide had been packaged as a
reusable Claude Skill — each repo's `AGENTS.md`/`mkdocs.yml` just
carries a comment pointing at the sibling repo to copy from. That's
exactly why it didn't surface automatically when datagov's docs site
was first built this session. This change (a) replaces datagov's
Jekyll site with the real convention, (b) adapts the page set to a CLI
product's shape per the owner's requested navigation, and (c) closes
the gap by packaging the pattern as a Skill so it's found automatically
next time, in this repo or any other.

## What changes

1. **Replace Jekyll with MkDocs Material**, matching
   `agent-job-matcher`/`ai-agents` exactly: `mkdocs.yml` at the repo
   root (Material theme, explicit `nav`, `markdown_extensions` for
   admonitions/tabs/mermaid/tablesort), `.github/workflows/docs.yml`
   (`mkdocs build --strict`, `mkdocs<2` pinned, `deploy-pages`,
   triggered only on docs paths so `docs:` commits release nothing),
   GitHub Pages source switched from "legacy branch/folder" to
   "GitHub Actions". `docs/_config.yml` (Jekyll) is removed.
2. **Reconciled navigation** — the owner's requested set (Getting
   Started, Installation, Commands, Tutorials, Deployment, Use Cases)
   plus the style guide's non-negotiable pages for a project this
   shape (Configuration — required whenever there's a config file,
   which PRD §28 defines; FAQ — always required; CI/CD — required
   once a project has 2+ workflows, which datagov already does):

   | Page | Source |
   |---|---|
   | Home (`index.md`) | style guide baseline |
   | Getting Started | owner request |
   | Installation | owner request (content largely relocates from the current site) |
   | Commands | owner request — one reference page for now (5 commands; split per-command once past ~8) |
   | Tutorials | owner request |
   | Use Cases | owner request |
   | Configuration | style guide: required (PRD §28 config precedence exists) |
   | CI/CD | style guide: required (2+ workflows: `ci.yml`, `release.yml`) |
   | FAQ | style guide: always required, titled exactly "FAQ" |

   Proposed nav order: Home → Getting Started → Installation →
   Commands → Tutorials → Use Cases → Configuration → CI/CD → FAQ.
   "Deployment" from the owner's original list is folded into CI/CD
   pending open question 1 below.
3. **Content migrates, doesn't duplicate**: today's `index.md`/
   `commands.md`/`installation.md` content relocates into the new
   structure (restyled to the shared conventions — "at the end you
   will have" openers, `Next:` footers, copy-pasteable commands only
   if actually run). The PRD and JSON Schema stay linked via absolute
   GitHub URLs, never reformatted into the site, matching both the
   existing datagov convention and the style guide's "specs record
   why, docs record how — never duplicate a spec into prose" rule.
4. **New Skill**: `~/.claude/skills/mkdocs-site/SKILL.md` (global,
   since the pattern already spans 3 repos) — packages the nav
   decision rules (which pages are mandatory and why), the mkdocs.yml/
   docs.yml templates, the Pages-setup steps, and a condensed pointer
   to the canonical style guide (linked, not copied — same
   single-source-of-truth rule the style guide itself states). Loads
   automatically whenever a docs site is being built or restructured
   in any repo.
5. **README stays a front door** (already true for datagov — no
   change needed there, unlike `agent-job-matcher` which had to trim
   an overgrown README).

## Out of scope

- Any Rust code change — this is a docs-only change; per the style
  guide's own rule, `docs:` commits release nothing.
- Rewriting `AGENTS.md` (already follows the standard's spirit).
- Cross-repo site aggregation (each repo publishes independently, per
  the established convention).
- Building the *content* of Bolt-5/6-dependent pages (`pii scan`,
  `report`) before those commands exist — pages describe what's
  actually built, updated per bolt, same discipline as today's site.

## Acceptance criteria

1. `mkdocs build --strict` passes locally and in CI (broken internal
   links fail the build).
2. The nine pages above exist, render on the published site, and
   follow the style guide (openers, `Next:` footers, tested commands
   only, relative links inside `docs/`, absolute GitHub URLs outside).
3. GitHub Pages source is "GitHub Actions" (not legacy branch/folder);
   `docs.yml` triggers only on `docs/**`, `mkdocs.yml`, and its own
   path — verified by a docs-only commit producing no release/CI-build
   side effects.
4. The published site is reachable and serves the same content
   whether visited at the account's custom domain or the plain
   `senthilsweb.github.io/datagov/` URL (matching confirmed live
   behavior on both sibling repos).
5. `~/.claude/skills/mkdocs-site/SKILL.md` exists, loads on relevant
   requests, and a fresh session (no prior context) could stand up an
   equivalent site in a new repo using only the skill.
6. No Rust source, `openspec/` change content, or `docs/prd.md` is
   touched by this change.

## Open questions for the inception gate

1. **What does "Deployment" mean for a CLI tool?** No running service
   to deploy. Candidates: (a) using `datagov` inside CI/CD pipelines
   (GitHub Actions example, exit-code gating on `pii scan --fail-on`)
   — this is what the style guide's "CI/CD" page already covers, so
   "Deployment" and "CI/CD" would just be the same page under one of
   the two names; (b) binary distribution/installation across
   environments (Docker, air-gapped, package managers) — mostly
   overlaps with Installation; (c) something else you have in mind.
   Recommended: fold into one "CI/CD" page (style guide's exact name)
   unless you mean something distinct from both (a) and (b).
2. **Commands: one page or one-per-command?** Recommended: one page
   while the surface is small (5 commands today); revisit once
   `pii scan`/`report` land and it gets crowded (~8 commands).
3. **Published URL**: keep `www.senthilsweb.com/datagov/` (confirmed
   this is what `agent-job-matcher` and `ai-agents` actually serve
   today too, despite their own `mkdocs.yml` `site_url` fields still
   saying `senthilsweb.github.io/...` — likely stale since the custom
   domain was added after those sites were first set up), or force
   plain `senthilsweb.github.io/datagov` only? Recommended: keep the
   custom domain (matches real sibling behavior) and set `site_url`
   correctly this time rather than copying the stale value.
4. **Skill placement**: global (`~/.claude/skills/mkdocs-site/`, my
   recommendation, since the pattern already spans 3 repos) or
   datagov-repo-local (`.claude/skills/`)? Recommended: global.

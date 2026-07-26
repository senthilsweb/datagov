# Tasks — `add-mkdocs-site`

## Bolt 0 — Inception gate

- [ ] Resolve open question 1 — Deployment vs. CI/CD naming (owner)
- [ ] Resolve open question 2 — Commands: one page or per-command (owner)
- [ ] Resolve open question 3 — published URL / `site_url` (owner)
- [ ] Resolve open question 4 — Skill placement, global vs. local (owner)
- [ ] Proposal status → **APPROVED**

## Bolt 1 — Skill

- [ ] `~/.claude/skills/mkdocs-site/SKILL.md` per design.md, including
      the mkdocs.yml/docs.yml templates and the mandatory-page rules
- [ ] Dry-run: confirm the skill's frontmatter description actually
      surfaces it for a "set up a docs site" style request

## Bolt 2 — Site migration

- [ ] `mkdocs.yml` at repo root per design.md
- [ ] `.github/workflows/docs.yml` copied from `agent-job-matcher`
- [ ] Remove `docs/_config.yml` (Jekyll) and the current ad hoc
      `index.md`/`installation.md`/`commands.md` in favor of the new
      structure (content relocates, nothing is silently dropped)
- [ ] GitHub Pages source switched to "GitHub Actions"
      (`build_type: workflow`)
- [ ] `mkdocs build --strict` passes locally

## Bolt 3 — Content

- [ ] `index.md`, `getting-started.md`, `installation.md`,
      `commands.md` (relocated + restyled)
- [ ] `tutorials.md`, `use-cases.md`, `configuration.md`, `ci-cd.md`,
      `faq.md` (new, per design.md's content plan)
- [ ] Every command shown was actually run against the real fixtures
      (style guide rule) — no invented output

## Bolt 4 — Verify + archive

- [ ] `docs:` commit produces no release/CI-build side effect
      (acceptance criterion 3)
- [ ] Site reachable at the published URL, content matches the repo
- [ ] Proposal status → **IMPLEMENTED**, then **VERIFIED**; archive
      the change and merge the spec delta into `openspec/specs/`

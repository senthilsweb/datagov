# Tasks — `add-mkdocs-site`

## Bolt 0 — Inception gate

- [x] Resolve open question 1 — **resolved (owner, 2026-07-26):**
      Deployment = docs-publishing page, distinct from CI/CD
- [x] Resolve open question 2 — **resolved (owner, 2026-07-26):** one
      Commands reference page, every command explained in full
- [ ] Resolve open question 3 — published URL. **Investigated, not
      resolved**: GitHub Pages' custom-domain inheritance is
      unconditional per account, confirmed no per-repo API override
      exists; `senthilsweb.github.io/datagov/` is independently live
      today regardless. Owner to choose how to proceed (see
      proposal.md's options a/b/c) — this is the only remaining gate
      item; it blocks Bolt 2/4 (going live) but not Bolt 1 (the skill)
- [x] Resolve open question 4 — **resolved (owner, 2026-07-26):**
      master copy in `my-agent-task-register`, symlinked globally
- [ ] Proposal status → **APPROVED** (pending question 3)

## Bolt 1 — Skill ✅ unblocked, proceeding now

- [ ] `~/work/my-agent-task-register/skills/mkdocs-site/SKILL.md`
      (master) per design.md — frontmatter `name`, `owner:
      Senthilnathan`, `github: https://github.com/senthilsweb`,
      `description`, `user-invocable: true`; includes the mkdocs.yml/
      docs.yml templates, mandatory-page rules, and the custom-domain-
      inheritance finding
- [ ] `~/.claude/skills/mkdocs-site` symlinked to the master
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

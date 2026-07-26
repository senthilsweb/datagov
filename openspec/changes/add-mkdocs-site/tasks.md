# Tasks — `add-mkdocs-site`

## Bolt 0 — Inception gate

- [x] Resolve open question 1 — **resolved (owner, 2026-07-26):**
      Deployment = docs-publishing page, distinct from CI/CD
- [x] Resolve open question 2 — **resolved (owner, 2026-07-26):** one
      Commands reference page, every command explained in full
- [x] Resolve open question 3 — **resolved (owner, 2026-07-26):
      proceed anyway (option a).** GitHub Pages' custom-domain
      inheritance confirmed unconditional (3 attempts, incl. a full
      Pages-site delete+recreate); `senthilsweb.github.io/datagov/` is
      the only URL this project documents/promotes; owner handles
      main-webapp routing collision risk separately. `build_type:
      workflow` already set as a side effect.
- [x] Resolve open question 4 — **resolved (owner, 2026-07-26):**
      master copy in `my-agent-task-register`, symlinked globally
- [x] Proposal status → **APPROVED** (2026-07-26)

## Bolt 1 — Skill ✅ (2026-07-26)

- [x] `~/work/my-agent-task-register/skills/mkdocs-site/SKILL.md`
      (master) — frontmatter `name`, `owner: Senthilnathan`, `github:
      https://github.com/senthilsweb`, `description`, `user-invocable:
      true`; includes the mkdocs.yml/docs.yml templates,
      mandatory-page rules, and the custom-domain-inheritance finding.
      Committed to `my-agent-task-register` (not yet pushed — owner to
      decide, per that repo's own push cadence)
- [x] `~/.claude/skills/mkdocs-site` symlinked to the master
- [x] Dry-run: skill appeared in the available-skills listing
      immediately after the symlink was created, description intact

## Bolt 2 — Site migration ✅ (2026-07-26)

Built by a Sonnet 5 agent from `briefs/bolt-2-3.md`; architect-reviewed
(`mkdocs build --strict` re-run independently — clean, exit 0 — plus
confirmed `docs/prd.md`, `docs/schema/`, `openspec/`, and all Rust
source untouched via `git status`).

- [x] `mkdocs.yml` at repo root per design.md, `site_url:
      https://senthilsweb.github.io/datagov/`
- [x] `.github/workflows/docs.yml` copied from `agent-job-matcher`
- [x] Removed `docs/_config.yml` (Jekyll); `index.md`/
      `installation.md`/`commands.md` replaced with the new structure
- [x] `build_type: workflow` (already set during the Q3 investigation)
- [x] `mkdocs build --strict` passes locally — independently
      re-verified by the architect, zero broken links

## Bolt 3 — Content ✅ (2026-07-26)

Architect spot-checked documented commands against the real release
binary: `inspect --output csv` (exit 2, exact error text matches),
`query` against a `.json` file (exit 4, exact error text matches), and
the `profile --columns email,state` table (byte-for-byte match). Three
implementer-flagged Proposed Corrections accepted: README updated to
reflect Bolt 5 (the brief's "no change needed" assumption predated
this session's own progress), one FAQ citation corrected to point at
the actual Bolt 2/5 implementation briefs instead of design.md, and
several headings fixed to the style guide's plain-word-anchor rule.

- [x] All ten pages built per design.md's content plan
- [x] Every command shown was actually run against real fixtures — no
      invented output (spot-checked independently, see above)
- [x] **Additional architect fix, found during review**: `datagov
      capabilities` was missing `pii scan`/`pii recognizers` from its
      command list (same class of staleness bug as the earlier Bolt 4
      gap) — fixed in `crates/datagov-cli/src/commands/
      capabilities.rs` (docs-content agent correctly left this alone
      per its own "no Rust source" scope; architect fixed it directly,
      197/197 tests still green); `installation.md`'s note updated to
      say "fixed on `main`, not yet in a tagged release" instead of
      "not yet fixed"

## Bolt 4 — Verify + archive

- [ ] `docs:` commit produces no release/CI-build side effect
      (acceptance criterion 3) — verify after this commit lands and
      the docs workflow runs
- [ ] Site reachable at the published URL, content matches the repo
- [ ] Proposal status → **IMPLEMENTED**, then **VERIFIED**; archive
      the change and merge the spec delta into `openspec/specs/`

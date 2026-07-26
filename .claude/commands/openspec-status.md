---
description: Show every openspec change with its status and task progress
---

Report the status of all OpenSpec changes in this repo. For each directory
under `openspec/changes/` (and `openspec/archive/` if present):

1. Read the `> Status:` line from its `proposal.md` header (PROPOSED /
   APPROVED / IMPLEMENTED / VERIFIED / ARCHIVED, plus any revision note).
2. Count checked (`- [x]`) vs unchecked (`- [ ]`) items in its `tasks.md`,
   overall and per `## Bolt`/section heading.

Then output:

- A compact table: change name | status | tasks done/total | progress bar.
- Under the table, for each change that is not fully done, list the
  **unchecked tasks grouped by their section heading**, so the next
  action is obvious.
- If a change's proposal has an "Open questions" section with unresolved
  items (questions not struck through or marked Resolved), flag them as
  blockers before any construction tasks.

Read only the proposal headers and tasks files — do not summarize designs
or specs. Keep the whole report short enough to scan in one screen.

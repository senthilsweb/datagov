---
name: mkdocs-site
owner: Senthilnathan
github: https://github.com/senthilsweb
description: Stand up or restructure a project's published documentation site — MkDocs Material published to GitHub Pages via GitHub Actions, task-organized navigation, and Senthil's shared writing style guide. Use whenever Senthil asks to create, fix, or reorganize project docs, a "docs site", a "wiki", or mentions "mkdocs" or "GitHub Pages" for markdown documentation — this is the established pattern across his repos (ai-agents, agent-job-matcher, datagov), not a new choice to make each time.
---

<!--
  This is a repo-local mirror of the master copy at
  ~/work/my-agent-task-register/skills/mkdocs-site/SKILL.md (also
  symlinked globally to ~/.claude/skills/mkdocs-site). It's a plain
  file here, not a symlink, because datagov is a public repo and a
  symlink into a private personal repo wouldn't resolve for anyone
  else cloning it. Edit the master first; re-copy here if it changes.
-->

# MkDocs documentation sites

The established pattern across every senthilsweb repo that publishes
docs: MkDocs Material, published to GitHub Pages via GitHub Actions
(never the legacy branch/folder Jekyll build — that predates this
pattern and should be migrated away from if found). Originated in
`ai-agents`/`agent-job-matcher`; first packaged as a skill during the
`datagov` project (2026-07-26), after its absence caused a Jekyll site
to get built by mistake before this pattern was found.

## When to use / when not to

Use this whenever a project needs a published docs site — anything
beyond a single README that a reader would want to browse, search, or
link to page-by-page. Don't use it for a project whose only public
face is its README; that's a front door, not a wiki (see below).

## Non-negotiable pages, and when each is required

- **FAQ** — always present, titled exactly "FAQ".
- **Configuration** — required whenever the product has any config
  file, env vars, or a settings-precedence chain.
- **CI/CD** — required once a project has 2+ GitHub Actions workflows:
  a left-to-right mermaid diagram (triggers → workflows → outputs)
  plus a short write-up per workflow, including real failures seen so
  far, not just the happy path.
- **Deployment** — required whenever the docs site's own publish
  mechanism isn't obvious from the CI/CD page (e.g. the CI/CD page
  covers the *product's* pipeline; Deployment covers how *the docs
  site itself* gets built and deployed). For a project whose CI/CD
  page already covers this, don't duplicate a second page — fold it
  in, one topic one home.
- **Runbook** — required if anything runs on a schedule or
  automatically: minimum content is what runs automatically (a
  table), any procedure with cost or risk (with its guard), and a
  failure → fix list of things that actually happened.

## Flexible pages — adapt names to the product's shape

Getting Started, Installation, and a small number of subject-deep
pages are close to universal, but their *names* should match what the
product actually is:

- A CLI tool: **Commands** (one reference page while the surface is
  small — every command explained in full, not just a flag table;
  split into one page per command only once the list gets crowded,
  roughly 8+ commands) plus **Tutorials** (end-to-end recipes chaining
  commands) and **Use Cases** (scenario framing, first person).
- A service/API: **Surfaces** or **API** instead of Commands.
- A data-heavy project: one or two **subject-deep pages** (data
  model, API, whatever the domain's real nouns are) instead of a
  generic "features" page.

Every task page opens with one sentence of the form *"At the end you
will have/know …"* and ends with a `Next:` link to the natural
following page. Getting Started offers independent "5-minute paths" —
cheapest first, nothing to install if avoidable.

## The site pattern (`mkdocs.yml`)

```yaml
# Wiki site, published to GitHub Pages by .github/workflows/docs.yml.
# Pattern shared across senthilsweb repos; writing standard:
# https://senthilsweb.github.io/ai-agents/style-guide/
site_name: <repo-name>
site_url: https://<actual-serving-domain>/<repo-name>/  # verify before hardcoding, see the domain note below
repo_url: https://github.com/senthilsweb/<repo-name>
repo_name: senthilsweb/<repo-name>
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
  # ... the page set decided above, in reading order

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

Only `site_name`, `site_url`, `repo_url`, `repo_name`, and `nav`
change per repo — everything else is copied verbatim.

## The publish workflow (`.github/workflows/docs.yml`)

```yaml
name: Docs site

on:
  push:
    branches: [main]
    paths: ["docs/**", "mkdocs.yml", ".github/workflows/docs.yml"]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: docs
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: "3.12" }
      - name: Build site (strict — broken links fail)
        # mkdocs pinned <2: the 2.0 rewrite drops the plugin system
        # mkdocs-material 9.x depends on.
        run: |
          pip install --quiet "mkdocs-material==9.*" "mkdocs<2"
          mkdocs build --strict
      - uses: actions/upload-pages-artifact@v3
        with: { path: site }

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Trigger only on docs-related paths so a `docs:`-scoped commit never
releases anything or runs the product's own CI — check this holds for
whatever release/CI workflow triggers already exist in the repo.

## GitHub Pages setup

Settings → Pages → Source → "GitHub Actions" (or
`gh api repos/<owner>/<repo>/pages -X PATCH -f build_type=workflow`
if Pages is already enabled the legacy way and needs switching).

**Custom-domain inheritance — verify before promising a URL.** If the
account's user/org Pages site (`<user>.github.io`) has a custom domain
configured, **every project site inherits it automatically**, serving
project docs at `<custom-domain>/<repo>/` with no supported per-repo
opt-out — confirmed empirically on `datagov` (2026-07-26): explicitly
clearing the project repo's `cname` via the Pages API had no effect on
`html_url`, and neither did a full delete-and-recreate of the Pages
site with only `build_type: workflow` set. The plain
`<user>.github.io/<repo>/` URL is still independently live in parallel
(GitHub serves both simultaneously) — there's no way to make only one
respond. If asked to keep a project's docs off the account's custom
domain, say this plainly rather than attempting a fix that doesn't
exist at the API/settings level; the only lever available is a GitHub
Support request, or accepting both URLs serve the same content (which
is what `datagov` did — see
`openspec/changes/add-mkdocs-site/proposal.md` open question 3).

## README stays a front door

A README contains only: what the project is (a few sentences, one
diagram if it earns its place), an "I want to… → run this" table, a
Documentation section listing the wiki pages, and Layout. Anything
deeper is a wiki page's job — if a topic has a wiki page, the README
must not also explain it.

## Writing rules

Full detail lives in the canonical style guide — link to it, don't
copy it into this file or into project docs (same rule the guide
itself states): https://senthilsweb.github.io/ai-agents/style-guide/
(local fallback:
`~/work/ai-agents/agents/job-scout/docs/style-guide.md`). Covers
voice, command/example discipline (every command shown must have
actually been run), and link conventions (relative inside `docs/`,
absolute GitHub URLs outside, plain-word headings since GitHub and
MkDocs generate different anchors for punctuation).

## Reference implementations

- `~/work/agent-job-matcher/mkdocs.yml` +
  `.github/workflows/docs.yml` — the original template.
- `~/work/ai-agents/mkdocs.yml` — the monorepo variant (mkdocs-
  monorepo plugin, one `!include` per sub-project, redirect map for
  moved pages) — reach for this only if a repo genuinely has multiple
  independently-published sub-projects sharing one site.
- This repo's own `mkdocs.yml`, `.github/workflows/docs.yml`, and
  `openspec/changes/add-mkdocs-site/` — the change that packaged this
  skill, including the full reconciliation of a CLI-shaped nav against
  these rules and the custom-domain finding above.

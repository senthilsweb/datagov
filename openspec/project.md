# Project — datagov

A DataGovOps platform whose first deliverable is `datagov`: a
single-download, agent-ready Rust CLI for data inspection, SQL analysis,
profiling, deterministic PII detection, data quality, lineage, policy
evaluation, and governance reporting.

Source of truth for product scope: `docs/prd.md` (three phases —
Core → Medium → Full; first milestone is **0.1**, PRD §37).

Lineage / reference projects:

- [`agent-job-matcher`](https://github.com/senthilsweb/agent-job-matcher) —
  the reference implementation of this AI-DLC lifecycle (bolts, HARD/SOFT
  evals, revision-logged proposals, ADRs).
- `privacyshield` — prior art for PII detection product thinking.
- `templrgo` — prior art for single-binary distribution and
  standards-check CI.

Target shape (PRD §30):

```
datagov/
├── openspec/           # this directory — every non-trivial change
├── docs/prd.md         # the PRD
├── crates/             # Rust workspace: datagov-cli, datagov-core,
│                       #   datagov-data, datagov-sql, datagov-pii,
│                       #   datagov-quality, datagov-policy,
│                       #   datagov-lineage, datagov-report, …
├── recognizers/        # PII recognizer YAML
├── policies/           # governance policy YAML
├── examples/           # sample datasets + SQL
└── benchmarks/         # performance benchmarks
```

## Process — AI-DLC via OpenSpec

Every non-trivial change goes through `openspec/changes/<name>/`
(proposal → design → tasks → spec) **before and during** implementation.

Status lifecycle: `proposed → approved → implemented → verified → archived`.
Archived changes move to `openspec/archive/<date>-<name>/`; on archive,
their capability spec deltas merge into the living specs under
`openspec/specs/<capability>/spec.md`.

Conventions carried over from the reference projects:

- Work is organized into **bolts** in `tasks.md`. Bolt 0 is always the
  inception gate: no construction starts until every open question in the
  proposal is resolved by the owner and status moves to **approved**.
- Evals are written from the spec, before or alongside the code —
  executable acceptance criteria, not after-the-fact tests.
- Eval criteria are **HARD** (objective, deterministic; any violation
  blocks `implemented → verified`) or **SOFT** (directional expectations,
  e.g. detector recall on a ground-truth set; misses are logged and
  reviewed, not blocking).
- Corrections discovered during Construction are logged in place with a
  dated `**Correction:**` entry — specs follow evidence, never silently
  rewritten.
- Scope changes are appended to the proposal header as numbered, dated
  `Revision:` entries with the reasoning — the proposal is an audit log,
  not a snapshot.
- Cross-cutting or reversal-prone decisions get an ADR under
  `openspec/adr/`.
- Requirement style: RFC-2119 `SHALL` requirements with
  Given/When/Then scenarios (PRD §32 shows the house style).
- Technology selections named in the PRD are defaults, not decisions —
  they must be validated through spike tasks before being locked
  (PRD §22).
- No secrets, credentials, or raw PII in any spec, fixture, or eval
  artifact. PII fixtures are synthetic.

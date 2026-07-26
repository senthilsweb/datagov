# FAQ

Short answers to questions already resolved in this project's own
history — each links to the spec or design doc that recorded the full
reasoning rather than repeating it here.

## Why Apache 2.0 and not MIT

The repository started under MIT, but the license was switched to
Apache 2.0 at the `add-datagov-core-cli` inception gate, before any
Core code was written. See open question 7 in
[`openspec/changes/add-datagov-core-cli/proposal.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/proposal.md)
for the owner's resolution.

## Why sqlglot-rust and not sqlparser-rs

`sqlglot-rust` was chosen directly at the inception gate, then
verified against a real dialect-conformance corpus during Bolt 4
construction rather than left as an untested assumption — v0.10.23
works across all 11 priority dialects, including genuine
dialect-specific rewrites (not literal passthroughs). No fallback to
`sqlparser-rs` was needed. See open question 1 in
[`openspec/changes/add-datagov-core-cli/proposal.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/proposal.md)
and the coverage matrix in
[`openspec/changes/add-datagov-core-cli/design.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/design.md).

## Why query only supports CSV and Parquet

This is the product's defined scope, not a current limitation waiting
to be lifted — see PRD
[§10.3, File querying](https://github.com/senthilsweb/datagov/blob/main/docs/prd.md#103-file-querying).
A query against a JSON, JSONL, or TSV file exits with code `4` rather
than attempting a best-effort read.

## Why does inspect mask PII columns before pii scan existed

`inspect` and `profile` mask sample rows using
`datagov-core::sensitivity::is_heuristically_sensitive` — a simple
column-name heuristic introduced in Bolt 2, before the real recognizer
engine existed. It was explicitly documented as a stand-in for the
dedicated PII engine, and it stays in place as `inspect`/`profile`'s
own independent masking even now that `pii scan` (Bolt 5) has real,
entity-typed, confidence-scored detection — the two are deliberately
separate code paths, not layered on top of each other. See
[`openspec/changes/add-datagov-core-cli/briefs/bolt-2.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/briefs/bolt-2.md)
(where the heuristic was introduced) and
[`briefs/bolt-5.md`](https://github.com/senthilsweb/datagov/blob/main/openspec/changes/add-datagov-core-cli/briefs/bolt-5.md)
(confirming it stays as-is, untouched by the recognizer engine).

# Configuration

At the end you will know the configuration precedence `datagov`
defines (PRD §28), where each layer lives on disk, and a known gap
between that design and what the binary actually does today.

## Precedence chain

Highest priority first — the first layer that sets a value wins:

1. CLI flags
2. Environment variables (`DATAGOV_*`)
3. Project configuration
4. User configuration
5. Built-in defaults

## File locations

```text
./datagov.yaml
./.datagov/config.yaml
$XDG_CONFIG_HOME/datagov/config.yaml
~/.config/datagov/config.yaml
```

`datagov-core`'s `config::load` resolves these layers in reverse
priority order (defaults first, then user, then XDG, then project,
then environment overrides last), so each later layer overwrites only
the fields it explicitly sets — an absent layer or an unset field
never clobbers a value set by a lower-priority one. The chain is
covered by 10 unit tests (each layer individually, the full override
order, and malformed YAML mapping to exit code `2`).

!!! note "Secrets never belong here"
    `Config` carries no credential-shaped field — the loader never
    reads secrets from these files, by design.

## Known gap: the output flag does not consult this chain

As of this writing, `datagov`'s configuration resolution and its
actual CLI dispatch are two separate things that haven't been wired
together yet. Checked directly against the current source
(`crates/datagov-cli/src/main.rs` and
`crates/datagov-core/src/config.rs`, 2026-07-26):

- `datagov_core::config::load` exists, is fully implemented, and is
  unit-tested in isolation — it resolves `output` and `threads` through
  the full five-layer chain above.
- `crates/datagov-cli/src/main.rs` never calls it. The global
  `--output` flag's default comes only from clap's own
  `default_value_t = OutputFormat::Table` — there is no code path
  today where an environment variable, `datagov.yaml`, or
  `~/.config/datagov/config.yaml` changes what `--output` defaults to.

In practice: setting `DATAGOV_OUTPUT=json` or `output: json` in
`datagov.yaml` has **no effect** on the binary right now. The
precedence chain above describes the intended design and is fully
built at the library level; it just isn't consulted by the CLI's
dispatch path yet. This page will be updated the moment that wiring
lands.

Next: [CI/CD](ci-cd.md) for how the product's own quality gates and
release pipeline work.

# Installation

At the end you will have a working `datagov` binary, either installed
from a pre-release snapshot or built from source.

!!! note "No final release yet"
    `v0.1.0` has not shipped. `v0.1.0-rc.*` pre-release snapshots are
    published as each bolt lands, for testing only — see
    [Releases](https://github.com/senthilsweb/datagov/releases) for
    the current tag.

## From a pre-release (fastest way to try it today)

```bash
DATAGOV_VERSION=v0.1.0-rc.5 curl -fsSL \
  https://raw.githubusercontent.com/senthilsweb/datagov/main/install.sh | sh
```

This detects your OS/architecture (macOS or Linux, arm64 or x86_64),
downloads the matching binary, verifies its SHA-256 checksum against
the one published in the release, and installs to `~/.local/bin` — no
`sudo`. Set `DATAGOV_INSTALL_DIR` to install elsewhere. Once a final
`v0.1.0` ships, dropping `DATAGOV_VERSION` will install the latest
stable release automatically.

Windows binaries (`datagov-windows-x86_64.exe`) are published on each
release but aren't installed by this script — download directly from
the [Releases page](https://github.com/senthilsweb/datagov/releases).

## From source

Requires a Rust toolchain ([rustup.rs](https://rustup.rs) if you don't
have one):

```bash
git clone https://github.com/senthilsweb/datagov.git
cd datagov
cargo build --release
./target/release/datagov version
```

```text
datagov 0.1.0
```

## Verify it worked

```bash
./target/release/datagov version
./target/release/datagov capabilities --output json
```

`capabilities` lists compiled commands and supported formats — useful
for confirming what's available in whatever snapshot you installed.

!!! note "Pre-release snapshots may lag this list"
    `capabilities` is generated from a fixed list in the CLI source, so
    it's only as current as the binary you installed. `v0.1.0-rc.5` and
    earlier don't list `pii scan`/`pii recognizers` even though both
    ship in that release — fixed on `main` for the next tag. Check
    [Commands](commands.md) for the authoritative, always-current list.

Next: [Commands](commands.md) for the full command reference, or back
to [Getting Started](getting-started.md) for three 5-minute paths.

---
layout: default
title: Installation
---

# Installation

No final release has shipped yet — `v0.1.0-rc.*` pre-release
snapshots are published as each bolt lands, for testing only. See
[Releases](https://github.com/senthilsweb/datagov/releases) for the
latest tag.

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

## Verify it worked

```bash
datagov version
datagov capabilities --output json
```

`capabilities` lists every command the binary was actually built with
— useful for confirming what's available in whatever snapshot you
installed, since this project is still mid-construction.

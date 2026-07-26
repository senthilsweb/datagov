#!/bin/sh
# install.sh — Download and install the datagov CLI.
#
# 1. Detects OS/architecture (macOS/Linux, arm64/x86_64).
# 2. Resolves the release to install: the latest GitHub Release by
#    default, or a specific tag via DATAGOV_VERSION (e.g. "v0.1.0").
# 3. Downloads the matching binary and its committed SHA-256 checksum
#    from the same release, and refuses to install unless they match.
# 4. Installs to DATAGOV_INSTALL_DIR (default: $HOME/.local/bin),
#    creating it if needed — no sudo, no system-directory writes.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/senthilsweb/datagov/main/install.sh | sh
#   DATAGOV_VERSION=v0.1.0 curl -fsSL .../install.sh | sh   # pin a version
#   DATAGOV_INSTALL_DIR=/usr/local/bin curl -fsSL .../install.sh | sudo sh   # system-wide
#
# Windows is not supported by this script — download
# datagov-windows-x86_64.exe directly from the Releases page.

set -eu

REPO="senthilsweb/datagov"
INSTALL_DIR="${DATAGOV_INSTALL_DIR:-$HOME/.local/bin}"

log()  { printf '%s\n' "$*"; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command '$1' not found"
}

need_cmd curl
need_cmd uname
need_cmd mktemp

detect_artifact() {
  os=$(uname -s)
  arch=$(uname -m)

  case "$os" in
    Darwin) os_part="darwin" ;;
    Linux)  os_part="linux" ;;
    *) die "unsupported OS '$os' — download a binary manually from https://github.com/${REPO}/releases" ;;
  esac

  case "$arch" in
    arm64|aarch64) arch_part="arm64" ;;
    x86_64|amd64)  arch_part="x86_64" ;;
    *) die "unsupported architecture '$arch' — download a binary manually from https://github.com/${REPO}/releases" ;;
  esac

  printf 'datagov-%s-%s' "$os_part" "$arch_part"
}

resolve_version() {
  if [ -n "${DATAGOV_VERSION:-}" ]; then
    printf '%s' "$DATAGOV_VERSION"
    return
  fi
  need_cmd sed
  tag=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  [ -n "$tag" ] || die "could not resolve the latest release — set DATAGOV_VERSION to install a specific tag (e.g. DATAGOV_VERSION=v0.1.0)"
  printf '%s' "$tag"
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "need either 'sha256sum' or 'shasum' to verify the download"
  fi
}

main() {
  artifact=$(detect_artifact)
  version=$(resolve_version)
  base_url="https://github.com/${REPO}/releases/download/${version}"

  log "datagov: installing ${version} (${artifact})"

  workdir=$(mktemp -d)
  trap 'rm -rf "$workdir"' EXIT

  bin_url="${base_url}/${artifact}"
  sum_url="${base_url}/${artifact}.sha256"

  curl -fsSL -o "${workdir}/${artifact}" "$bin_url" \
    || die "download failed: $bin_url (no release published yet? see https://github.com/${REPO}/releases)"
  curl -fsSL -o "${workdir}/${artifact}.sha256" "$sum_url" \
    || die "checksum download failed: $sum_url"

  expected=$(awk '{print $1}' "${workdir}/${artifact}.sha256")
  actual=$(sha256_of "${workdir}/${artifact}")
  [ "$expected" = "$actual" ] || die "checksum mismatch for ${artifact} — expected ${expected}, got ${actual}. Refusing to install a corrupted or tampered binary."

  mkdir -p "$INSTALL_DIR"
  install_path="${INSTALL_DIR}/datagov"
  cp "${workdir}/${artifact}" "$install_path"
  chmod +x "$install_path"

  log "datagov: installed to ${install_path}"

  case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *) log "note: ${INSTALL_DIR} is not on your PATH — add it, e.g.:"
       log "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
  esac

  if command -v "$install_path" >/dev/null 2>&1; then
    "$install_path" version
  fi
}

main "$@"

#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
#  Syncara — curl-pipe installer
#  Usage:
#    curl -fsSL https://syncara.sh/install.sh | sh
#    curl -fsSL https://syncara.sh/install.sh | sh -s -- v0.1.0
#
#  Installs the latest (or specified) release to /usr/local/bin.
# ─────────────────────────────────────────────────────────────

REPO="anomalyco/syncara"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# ── helpers ──────────────────────────────────────────────────
info()  { printf "\033[36m•\033[0m %s\n" "$*"; }
warn()  { printf "\033[33m⚠\033[0m %s\n" "$*" >&2; }
err()   { printf "\033[31m✗\033[0m %s\n" "$*" >&2; exit 1; }

# ── detect architecture ──────────────────────────────────────
detect_arch() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64)       echo "x86_64" ;;
    aarch64|arm64)      echo "aarch64" ;;
    *) err "unsupported architecture: $arch" ;;
  esac
}

detect_os() {
  local os
  os="$(uname -s)"
  case "$os" in
    Linux)  echo "unknown-linux-gnu" ;;
    Darwin) echo "apple-darwin" ;;
    *)      err "unsupported operating system: $os" ;;
  esac
}

# ── resolve version ──────────────────────────────────────────
resolve_version() {
  if [ -n "${1:-}" ]; then
    echo "${1#v}"
    return
  fi
  # fetch latest tag from GitHub API
  if command -v curl >/dev/null 2>&1; then
    curl -fsS "https://api.github.com/repos/$REPO/releases/latest" |
      grep '"tag_name"' |
      sed 's/.*"v\(.*\)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "https://api.github.com/repos/$REPO/releases/latest" |
      grep '"tag_name"' |
      sed 's/.*"v\(.*\)".*/\1/'
  else
    err "need curl or wget to find latest release"
  fi
}

# ── download ──────────────────────────────────────────────────
download() {
  local url="$1" dest="$2"
  info "downloading $url"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$dest" "$url"
  else
    err "need curl or wget to download"
  fi
}

verify_checksum() {
  local archive="$1" checksum_url="$2"
  if ! command -v sha256sum >/dev/null 2>&1; then
    warn "sha256sum not found — skipping verification"
    return
  fi
  local expected actual
  expected="$(curl -fsSL "$checksum_url" | awk '{print $1}')"
  actual="$(sha256sum "$archive" | awk '{print $1}')"
  if [ "$expected" != "$actual" ]; then
    err "checksum mismatch — expected $expected, got $actual"
  fi
  info "checksum verified"
}

# ── main ─────────────────────────────────────────────────────
main() {
  local version target artifact archive_url checksum_url
  version="$(resolve_version "${1:-}")"
  : "${version:?could not resolve version}"

  local os_arch target
  target="$(detect_arch)-$(detect_os)"
  artifact="syncara-$version-$target.tar.gz"

  archive_url="https://github.com/$REPO/releases/download/v$version/$artifact"
  checksum_url="https://github.com/$REPO/releases/download/v$version/${artifact}.sha256"

  info "Syncara v$version — $target"

  # ── temporary directory ──
  local tmpdir
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  download "$archive_url" "$tmpdir/$artifact"
  verify_checksum "$tmpdir/$artifact" "$checksum_url"

  # ── extract ──
  tar xzf "$tmpdir/$artifact" -C "$tmpdir"

  # ── install ──
  mkdir -p "$INSTALL_DIR"
  if [ -w "$INSTALL_DIR" ]; then
    mv "$tmpdir/syncara" "$INSTALL_DIR/syncara"
  else
    info "need sudo to install to $INSTALL_DIR"
    sudo mv "$tmpdir/syncara" "$INSTALL_DIR/syncara"
  fi

  chmod +x "$INSTALL_DIR/syncara"
  info "installed to $INSTALL_DIR/syncara"

  # ── verify ──
  "$INSTALL_DIR/syncara" --version
}

main "$@"

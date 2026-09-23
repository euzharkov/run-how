#!/bin/sh
# rhow installer — https://github.com/euzharkov/run-how
#
#   curl -fsSL https://raw.githubusercontent.com/euzharkov/run-how/main/install.sh | sh
#
# Environment overrides:
#   RHOW_VERSION      version to install (default: latest release)
#   RHOW_INSTALL_DIR  install directory  (default: ~/.local/bin)
#   RHOW_REPO         GitHub repo         (default: euzharkov/run-how)
#   RHOW_DOWNLOAD_BASE  where release archives live (default: https://github.com);
#   RHOW_API_BASE       where "latest release" is asked (default: https://api.github.com).
#                       Both exist so tests/install can point this script at a local mirror.
set -eu

REPO="${RHOW_REPO:-euzharkov/run-how}"
DOWNLOAD_BASE="${RHOW_DOWNLOAD_BASE:-https://github.com}"
API_BASE="${RHOW_API_BASE:-https://api.github.com}"
INSTALL_DIR="${RHOW_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${RHOW_VERSION:-}"

say() { printf '%s\n' "$*" >&2; }
die() { say "error: $*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

need uname
need tar
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  fetch_stdout() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
  fetch_stdout() { wget -qO- "$1"; }
else
  die "curl or wget is required"
fi

# 1. OS
os="$(uname -s)"
case "$os" in
  Darwin) os_target="apple-darwin" ;;
  Linux)  os_target="unknown-linux-musl" ;;
  MINGW*|MSYS*|CYGWIN*) die "on Windows use: winget install rhow, scoop install rhow, or download the .zip from GitHub Releases" ;;
  *) die "unsupported OS: $os" ;;
esac

# 2. CPU architecture
arch="$(uname -m)"
case "$arch" in
  x86_64|amd64)   arch_target="x86_64" ;;
  arm64|aarch64)  arch_target="aarch64" ;;
  *) die "unsupported architecture: $arch" ;;
esac
target="${arch_target}-${os_target}"

# 3. Release artifact
if [ -z "$VERSION" ]; then
  VERSION="$(fetch_stdout "${API_BASE}/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$VERSION" ] || die "could not determine the latest release of ${REPO}"
fi
VERSION="${VERSION#v}"
name="rhow-${VERSION}-${target}"
base="${DOWNLOAD_BASE}/${REPO}/releases/download/v${VERSION}"

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t rhow)"
trap 'rm -rf "$tmp"' EXIT

# 4. Download
say "Downloading ${name}.tar.gz"
fetch "${base}/${name}.tar.gz" "${tmp}/${name}.tar.gz"
fetch "${base}/SHA256SUMS" "${tmp}/SHA256SUMS"

# 5. Verify SHA256
expected="$(grep " ${name}.tar.gz\$" "${tmp}/SHA256SUMS" | awk '{print $1}')"
[ -n "$expected" ] || die "no checksum for ${name}.tar.gz in SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${tmp}/${name}.tar.gz" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${tmp}/${name}.tar.gz" | awk '{print $1}')"
else
  die "sha256sum or shasum is required to verify the download"
fi
[ "$expected" = "$actual" ] || die "checksum mismatch for ${name}.tar.gz (expected ${expected}, got ${actual})"
say "Checksum verified"

# 6. Install (no sudo)
tar -xzf "${tmp}/${name}.tar.gz" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 755 "${tmp}/${name}/rhow" "${INSTALL_DIR}/rhow"
say "Installed rhow ${VERSION} to ${INSTALL_DIR}/rhow"

# 7. PATH
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    say ""
    say "${INSTALL_DIR} is not on your PATH. Add it with:"
    case "${SHELL:-}" in
      */zsh)  say "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc" ;;
      */fish) say "  fish_add_path ${INSTALL_DIR}" ;;
      *)      say "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc" ;;
    esac
    ;;
esac
say "Run \`rhow\` inside any repository to get started."

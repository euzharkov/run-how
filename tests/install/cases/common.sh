# Shared helpers for the installer cases. Sourced with `. /src/tests/install/cases/common.sh`.
set -eu
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' /src/Cargo.toml | head -n1)"
MIRROR="http://release/euzharkov/run-how/releases/download/v$VERSION"

say() { printf '\n== %s\n' "$*" >&2; }

# The checksum the mirror publishes for a target's archive.
sum_for() {
  wget -qO- "$MIRROR/SHA256SUMS" 2>/dev/null || curl -fsSL "$MIRROR/SHA256SUMS"
}
sha_of_target() {
  sum_for | grep " rhow-$VERSION-$1.tar.gz\$" | awk '{print $1}'
}

# Prove an installed binary is the one we built and that it actually works.
smoke() {
  bin="$1"
  say "smoke test: $bin"
  [ -x "$bin" ] || { echo "not executable: $bin" >&2; exit 1; }
  got="$("$bin" --version)"
  [ "$got" = "rhow $VERSION" ] || { echo "expected 'rhow $VERSION', got '$got'" >&2; exit 1; }
  tmp="$(mktemp -d)"
  printf '{"scripts":{"dev":"vite","test":"vitest run"}}\n' > "$tmp/package.json"
  out="$(cd "$tmp" && "$bin" --no-runtime --color never)"
  echo "$out"
  echo "$out" | grep -q "Start Vite development server" || { echo "discovery output missing dev action" >&2; exit 1; }
  echo "$out" | grep -q "Run Vitest tests" || { echo "discovery output missing test action" >&2; exit 1; }
  say "OK: $bin is rhow $VERSION"
}

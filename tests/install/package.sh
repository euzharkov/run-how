#!/bin/sh
# Build and package rhow like the release workflow, into a GitHub-Releases-shaped tree:
#   <out>/euzharkov/run-how/releases/download/v<ver>/rhow-<ver>-<target>.tar.gz
#   <out>/euzharkov/run-how/releases/download/v<ver>/SHA256SUMS
#   <out>/repos/euzharkov/run-how/releases/latest          (what api.github.com answers)
set -eu
out="${1:?output directory}"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
host="$(rustc -vV | sed -n 's/^host: //p')"
targets="$host"
# Homebrew on Linux and Arch only ship amd64 images; cross-link for them from an arm64 host.
case "$host" in
  aarch64-*) targets="$host x86_64-unknown-linux-musl" ;;
esac

dl="$out/euzharkov/run-how/releases/download/v$version"
mkdir -p "$dl" "$out/repos/euzharkov/run-how/releases"
for target in $targets; do
  rustup target add "$target" >/dev/null 2>&1 || true
  if [ "$target" != "$host" ]; then
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
  fi
  cargo build --release --locked --target "$target"
  name="rhow-$version-$target"
  mkdir -p "dist/$name"
  cp README.md LICENSE "target/$target/release/rhow" "dist/$name/"
  (cd dist && tar -czf "$dl/$name.tar.gz" "$name")
  rm -rf "dist/$name"
done
(cd "$dl" && sha256sum rhow-* > SHA256SUMS && cat SHA256SUMS)
printf '{"tag_name": "v%s"}\n' "$version" > "$out/repos/euzharkov/run-how/releases/latest"

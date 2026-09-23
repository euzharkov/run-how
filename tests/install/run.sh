#!/bin/sh
# Run the installer tests in Docker. Usage:
#   tests/install/run.sh              # every channel
#   tests/install/run.sh npm brew     # only these
#
# Builds the release mirror from the working tree, then installs rhow through each channel in
# its own container. Nothing is installed on the host. Homebrew and AUR images are amd64 only,
# so on an arm64 host they run under emulation (slower, but real).
set -eu
cd "$(dirname "$0")"
all="curl-debian curl-alpine npm cargo brew aur"
cases="${*:-$all}"
compose="docker compose -f compose.yml"

echo "== building the release mirror"
if ! $compose build --progress quiet release; then
  echo "mirror build failed; nothing was tested" >&2
  exit 1
fi
$compose up -d --wait --quiet-pull release

passed=""; failed=""
for c in $cases; do
  echo
  echo "================ $c ================"
  if $compose run --rm "$c"; then passed="$passed $c"; else failed="$failed $c"; fi
done
$compose down --remove-orphans >/dev/null 2>&1 || true

echo
echo "passed:${passed:- (none)}"
echo "failed:${failed:- (none)}"
[ -z "$failed" ]

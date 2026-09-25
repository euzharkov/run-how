#!/bin/sh
# Build the integration image if rhow's source changed, drop the untagged image that a
# rebuild leaves behind, and run the check with the product list, must-have commands,
# expected outputs and logs mounted from this directory. Arguments select stacks or products (see check.py).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
image=run-how/integration

docker build -q -f "$here/Dockerfile" -t "$image" "$root" >/dev/null
# A rebuild retags the name and leaves the previous build as <none>; keep only the current.
docker images -q --filter dangling=true --filter "label=org.opencontainers.image.title=$image" \
  | xargs docker rmi >/dev/null 2>&1 || true

mkdir -p "$here/logs" "$here/expected"
exec docker run --rm --tmpfs /work:size=2g \
  -v "$here/repos.txt:/integration/repos.txt:ro" \
  -v "$here/must.txt:/integration/must.txt:ro" \
  -v "$here/expected:/integration/expected" \
  -v "$here/logs:/logs" \
  -e UPDATE_EXPECTED="${UPDATE_EXPECTED:-0}" \
  "$image" "$@"

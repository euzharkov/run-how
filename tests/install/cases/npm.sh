#!/bin/sh
# npm: pack the publishable tarball, install it globally, let postinstall fetch the binary.
. /src/tests/install/cases/common.sh
cp -r /src/packaging/npm /tmp/npm && cd /tmp/npm
say "npm pack"
tgz="$(npm pack --silent)"
say "npm install -g $tgz"
npm install -g "./$tgz"
smoke "$(npm prefix -g)/bin/rhow"
say "launcher forwards arguments and the exit code"
cd "$(mktemp -d)" && printf '{"scripts":{"x":"true"}}\n' > package.json
rhow --no-runtime --color never . | grep -q "npm run x"
if rhow does-not-exist >/dev/null 2>&1; then echo "expected non-zero exit" >&2; exit 1; fi
say "OK: npm channel"

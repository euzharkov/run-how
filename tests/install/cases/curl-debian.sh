#!/bin/sh
# curl | sh on Debian: "latest" is resolved through the API mirror, no version given.
. /src/tests/install/cases/common.sh
apt-get update -qq >/dev/null && apt-get install -y -qq curl ca-certificates >/dev/null
say "install.sh via curl, latest"
sh /src/install.sh
smoke "$HOME/.local/bin/rhow"

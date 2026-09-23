#!/bin/bash
# Homebrew on Linux: install the tap formula with its URL and sha pointed at the mirror.
. /src/tests/install/cases/common.sh
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_ANALYTICS=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1
sha="$(sha_of_target x86_64-unknown-linux-musl)"
[ -n "$sha" ] || { echo "no x86_64 archive on the mirror" >&2; exit 1; }
sed -e "s#https://github.com/#http://release/#" \
    -e "s/REPLACE_WITH_SHA256/$sha/" /src/packaging/homebrew/rhow.rb > /tmp/rhow.rb
# Homebrew only installs formulae that live in a tap, so make a throwaway local one.
export HOMEBREW_GIT_NAME=test HOMEBREW_GIT_EMAIL=test@example.com
say "brew tap-new local/tap"
brew tap-new --no-git local/tap >/dev/null
cp /tmp/rhow.rb "$(brew --repository local/tap)/Formula/rhow.rb"
say "brew install local/tap/rhow"
brew install local/tap/rhow
say "brew test local/tap/rhow (the formula's own test block)"
brew test local/tap/rhow
smoke "$(brew --prefix)/bin/rhow"

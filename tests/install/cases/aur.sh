#!/bin/bash
# AUR: build the rhow-bin PKGBUILD with makepkg as an unprivileged user, install with pacman.
. /src/tests/install/cases/common.sh
useradd -m builder
sha="$(sha_of_target x86_64-unknown-linux-musl)"
[ -n "$sha" ] || { echo "no x86_64 archive on the mirror" >&2; exit 1; }
sed -e 's#^url="https://github.com/euzharkov/run-how"#url="http://release/euzharkov/run-how"#' \
    -e "s/REPLACE_WITH_SHA256/$sha/" /src/packaging/aur/PKGBUILD > /home/builder/PKGBUILD
chown builder /home/builder/PKGBUILD
say "makepkg"
su builder -c "cd ~ && makepkg -f"
say "pacman -U"
pacman -U --noconfirm /home/builder/rhow-bin-*.pkg.tar.zst
smoke /usr/bin/rhow
pacman -Qi rhow-bin | grep -q "^Version.*: $VERSION" || { echo "package version mismatch" >&2; exit 1; }

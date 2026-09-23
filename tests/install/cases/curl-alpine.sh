#!/bin/sh
# install.sh on Alpine: no curl, so the wget fallback; explicit version and install dir.
. /src/tests/install/cases/common.sh
command -v curl >/dev/null && { echo "expected no curl on alpine base" >&2; exit 1; }
say "install.sh via wget, RHOW_VERSION=$VERSION, RHOW_INSTALL_DIR=/usr/local/bin"
RHOW_VERSION="v$VERSION" RHOW_INSTALL_DIR=/usr/local/bin sh /src/install.sh
smoke /usr/local/bin/rhow
say "a corrupted checksum is rejected"
mkdir -p /tmp/bad/euzharkov/run-how/releases/download/v$VERSION
cd /tmp/bad/euzharkov/run-how/releases/download/v$VERSION
wget -q "$MIRROR/rhow-$VERSION-$(uname -m)-unknown-linux-musl.tar.gz"
printf '%s  %s\n' "0000000000000000000000000000000000000000000000000000000000000000" "rhow-$VERSION-$(uname -m)-unknown-linux-musl.tar.gz" > SHA256SUMS
apk add --no-cache busybox-extras >/dev/null   # the base image's busybox has no httpd applet
httpd -p 8099 -h /tmp/bad
if RHOW_VERSION="$VERSION" RHOW_DOWNLOAD_BASE=http://localhost:8099 RHOW_INSTALL_DIR=/tmp/bad-install sh /src/install.sh 2>/tmp/err; then
  echo "installer accepted a bad checksum" >&2; exit 1
fi
grep -q "checksum mismatch" /tmp/err || { cat /tmp/err >&2; exit 1; }
[ ! -e /tmp/bad-install/rhow ] || { echo "binary installed despite bad checksum" >&2; exit 1; }
say "OK: bad checksum rejected"

#!/bin/sh
# cargo install from the source tree, the same thing `cargo install rhow` does from crates.io.
. /src/tests/install/cases/common.sh
apk add --no-cache musl-dev >/dev/null
say "cargo install --path"
CARGO_TARGET_DIR=/tmp/target cargo install --path /src --locked --root /tmp/cargo
smoke /tmp/cargo/bin/rhow

# Super integration test: rhow against real repositories, three per supported stack.
#
#   tests/integration/run.sh                  # every product
#   tests/integration/run.sh go               # one stack
#   tests/integration/run.sh sharkdp/bat      # one product
#   UPDATE_EXPECTED=1 tests/integration/run.sh ever-co/ever-gauzy   # accept its new output
#
# run.sh builds this image (cached: only rhow's source and check.py are in it), removes the
# untagged image a rebuild leaves behind, mounts repos.txt, expected/ and logs/, and runs.
#
# The image holds only rhow, git and python. Each repository is fetched at run time at its
# pinned commit with depth 1, checked, and deleted before the next. Mounting /work as tmpfs
# caps it hard and keeps Docker's disk untouched.
# Each repository's default view is compared with tests/integration/expected/, its hand-picked
# commands in must.txt must stay in it, and --all, --ci and --json must agree with it.
# Dependencies are never installed: rhow reads structure only.

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock support.toml README.md ./
COPY src src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends python3 git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/rhow /usr/local/bin/rhow
COPY tests/integration/check.py /integration/check.py
# repos.txt, must.txt and expected/ are mounted at run time (see run.sh), so editing the
# product list or accepting new output never rebuilds the image: it only changes with rhow's
# own source.
LABEL org.opencontainers.image.title="run-how/integration"
ENV LOGS=/logs
ENTRYPOINT ["python3", "/integration/check.py"]

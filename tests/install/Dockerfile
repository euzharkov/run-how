# A local stand-in for GitHub Releases, built from the working tree.
#
# Stage 1 builds rhow for the container's own architecture (plus x86_64 when the host is
# arm64, so the amd64-only Homebrew and Arch images have something to install) and packages
# it exactly like .github/workflows/release.yml does. Stage 2 serves those files under the
# same paths GitHub uses, so every installer can be pointed at it with RHOW_DOWNLOAD_BASE.
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
# Cache the crates registry and build outputs across runs; only rhow itself recompiles.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    sh tests/install/package.sh /dist

FROM nginx:alpine
COPY --from=builder /dist /usr/share/nginx/html

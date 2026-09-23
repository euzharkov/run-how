# Distribution

GitHub Releases are the source of truth. Tagging `vX.Y.Z` runs `.github/workflows/release.yml`,
which builds:

```text
rhow-<version>-aarch64-apple-darwin.tar.gz
rhow-<version>-x86_64-apple-darwin.tar.gz
rhow-<version>-aarch64-unknown-linux-musl.tar.gz
rhow-<version>-x86_64-unknown-linux-musl.tar.gz
rhow-<version>-aarch64-pc-windows-msvc.zip
rhow-<version>-x86_64-pc-windows-msvc.zip
SHA256SUMS
```

Each archive contains a `rhow-<version>-<target>/` directory with the binary, `README.md` and
`LICENSE`. The workflow also publishes to crates.io when `CARGO_REGISTRY_TOKEN` is set.

## Channels

| Channel | Mechanism | Files |
|---|---|---|
| `curl … \| sh` | `install.sh`: detects OS/arch, downloads, verifies SHA256, installs to `~/.local/bin`, explains PATH | `install.sh` |
| `cargo install rhow` | crates.io source build | `Cargo.toml` |
| `cargo binstall rhow` | Reads the `[package.metadata.binstall]` section below and downloads the release archive | `Cargo.toml` |
| Homebrew | Tap formula pointing at the release archives | `packaging/homebrew/rhow.rb` |
| Scoop | Bucket manifest with `autoupdate` from `SHA256SUMS` | `packaging/scoop/rhow.json` |
| WinGet | Manifest generated with `wingetcreate` at release time | `packaging/winget/` |
| AUR | `rhow-bin` PKGBUILD from the musl archives | `packaging/aur/PKGBUILD` |
| Nix | `buildRustPackage` derivation | `packaging/nix/default.nix` |
| npm | `@euzharkov/rhow`: `postinstall` downloads and verifies the native binary; `bin/rhow.js` execs it. Node is only a launcher | `packaging/npm/` |

## Testing the installers without touching your machine

Every Linux channel above downloads a release archive, checks it against `SHA256SUMS`, and
installs it. `tests/install/` exercises exactly that inside Docker, from the working tree, with
no network dependency on a real GitHub release:

```bash
tests/install/run.sh              # all channels
tests/install/run.sh npm brew     # a subset
```

`tests/install/Dockerfile` builds and packages rhow like the release workflow, then serves
the files from an nginx container under the same paths GitHub uses. `install.sh` and the npm
`install.js` accept `RHOW_DOWNLOAD_BASE` / `RHOW_API_BASE` so they can be pointed at that
mirror; the Homebrew formula and PKGBUILD are copied with their URL and checksum rewritten.
Each channel runs in its own throwaway container and ends with the same smoke test: the
installed binary reports the version from `Cargo.toml` and discovers a sample project.

| Case | Image | What it proves |
|---|---|---|
| `curl-debian` | `debian:bookworm-slim` | `curl \| sh`, "latest" resolved through the API |
| `curl-alpine` | `alpine:3` | the wget fallback, explicit version and install dir, a bad checksum is refused |
| `npm` | `node:22-alpine` | `npm pack` + `npm install -g`, postinstall download, launcher exit codes |
| `cargo` | `rust:1-alpine` | `cargo install --path` with the locked dependencies |
| `brew` | `homebrew/brew` | the tap formula installs and its `test do` block passes |
| `aur` | `archlinux:base-devel` | the PKGBUILD builds with `makepkg` and installs with `pacman` |

Homebrew and Arch publish amd64 images only, so on an Apple Silicon host those two run under
emulation and the mirror also carries an x86_64 archive cross-linked with `rust-lld`. Not
covered: the Windows channels (Scoop, WinGet), Nix, and `cargo binstall`. CI runs the whole set
on every push through `.github/workflows/ci.yml`.

Release checklist:

1. Bump `version` in `Cargo.toml` and `packaging/npm/package.json`; run `cargo test`.
2. Tag `vX.Y.Z` and push. Wait for the release workflow.
3. Copy hashes from `SHA256SUMS` into the Homebrew, Scoop, WinGet and AUR files.
4. `npm publish` from `packaging/npm`.

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
| npm | `@line-19/rhow`: `postinstall` downloads and verifies the native binary; `bin/rhow.js` execs it. Node is only a launcher | `packaging/npm/` |

Release checklist:

1. Bump `version` in `Cargo.toml` and `packaging/npm/package.json`; run `cargo test`.
2. Tag `vX.Y.Z` and push. Wait for the release workflow.
3. Copy hashes from `SHA256SUMS` into the Homebrew, Scoop, WinGet and AUR files.
4. `npm publish` from `packaging/npm`.

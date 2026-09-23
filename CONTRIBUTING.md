# Contributing

Thanks for helping. Pull requests are welcome from anyone; `main` only changes through a
reviewed pull request with green checks, merged by a maintainer.

## Before you open a PR

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same three commands on Linux, macOS and Windows, then the installer tests in
Docker (`tests/install/run.sh`). You can run those locally too if you have Docker.

## What a good change looks like

- **A new ecosystem** implements `discover::Discoverer` in its own file under `src/discover/`,
  is registered in `discover::all()`, and comes with a fixture repository under `fixtures/`
  and a snapshot test entry in `tests/fixtures.rs`. See `docs/ARCHITECTURE.md`.
- **A new command explanation** goes into the matching file under `src/analyze/tools/`,
  not into an adapter.
- **A version-support change** edits `support.toml`; `cargo test` then tells you what to
  paste into the README table.
- Discovery must stay read-only: no subprocesses, no writes, no network.
- Snapshot updates are reviewed like code. Run `cargo insta review` and include the
  `.snap` changes in the PR with a sentence on why they moved.

## Reporting a bug

Open an issue with the smallest repository layout that reproduces it, ideally as a fixture
directory, plus the output of `rhow --json --no-runtime`.

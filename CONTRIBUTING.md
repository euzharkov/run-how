# Contributing

Thanks for helping. Pull requests are welcome from anyone; `main` only changes through a
reviewed pull request with green checks, merged by a maintainer.

## Before you open a PR

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same three commands on Linux, macOS and Windows, then the installer tests and the
real-product integration test in Docker (`tests/install/run.sh`, `tests/integration/run.sh`). You can run those locally too if you have Docker.

Before changing discovery or explanations, also run the real-product integration test
(`tests/integration/run.sh`, Docker only): it checks rhow against ~100 real repositories
pinned to commits and diffs each default view against `tests/integration/expected/`. CI runs
it too, but locally you see the diff sooner. A moved expected output is a behaviour change to
explain in the commit, accepted with `UPDATE_EXPECTED=1 tests/integration/run.sh <owner/repo>`.
Re-recording never touches `tests/integration/must.txt`: if a command there leaves the
default view, rhow regressed, whatever the new expected output says. See the README's
"Tests" section.

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

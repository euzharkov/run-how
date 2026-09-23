## What

<!-- One or two sentences: what changes and why. -->

## Checks

- [ ] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass
- [ ] Snapshot changes (`tests/snapshots/*.snap`) were reviewed and are intended
- [ ] A new ecosystem or command comes with a fixture under `fixtures/` and a snapshot
- [ ] Discovery stays read-only: nothing added spawns a process, writes, or uses the network
- [ ] `support.toml` and the README table were updated together, if a version baseline changed

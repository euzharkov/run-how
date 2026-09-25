# rhow: purpose, principles, architecture, and working rules

This file is the entry point for anyone (human or agent) changing this repository. It states
what the project is, what it deliberately is not, how the code is organised, and how changes
are made and verified. `docs/ARCHITECTURE.md` holds the detailed pipeline description;
`docs/DISTRIBUTION.md` covers packaging and release.

## 1. What rhow is

`rhow` answers one question inside any repository: **how do I run and operate this?**

It reads the files a project already has (`package.json`, `Makefile`, `Cargo.toml`,
`pyproject.toml`, `Taskfile.yml`, compose files, Kubernetes manifests, and so on), normalises
everything it finds into one model, and prints:

- the **actions** the project offers, grouped by category (Development, Testing, Quality,
  Build, Database, Infrastructure, Release);
- the exact **native command** behind each action (`pnpm --filter api test`);
- a short deterministic **explanation** of what that command does (`yarn start` in an Expo
  project is "Start the Expo development server");
- whether the action was **declared** by the project or **inferred** from convention, with a
  confidence level;
- a **risk** label when the command talks to something external or destroys data;
- which **tool versions** the project declares, compared against what this build of rhow has
  been verified with (`rhow support`).

The user then runs the command with the project's own tool.

## 2. What rhow is not

These are decisions, not gaps. Do not reopen them in a pull request without a discussion first.

- **Not a task runner.** rhow never executes an action, forwards arguments, manages
  environment variables, or asks for confirmation. The listing shows the command. This keeps
  the tool free of shell quoting, exit-code plumbing and per-runner argument rules.
- **Not a writer.** Discovery never modifies the repository, installs dependencies, starts
  services, spawns a project tool (`npm`, `cargo metadata`, `kubectl`, `docker`) or touches
  the network. `tests/fixtures.rs::discovery_is_read_only` (every fixture, before and after)
  and `nothing_spawns_a_process_or_opens_a_socket` (the source) guard this.
- **Not an LLM product.** Explanations are produced by a static knowledge table and a
  deterministic tokenizer. Same input, same output, offline, in milliseconds.
- **Not configurable.** No `.rhow.yml`, no plugins, no favourites, no history, no dashboard.
  Zero configuration is the feature.
- **Not a guesser.** When unsure, rhow prefers a missing description or a missing warning over
  a wrong one. Unknown commands are shown as `Run <program>`; ambiguous version constraints
  are reported as "unclear", never as a verdict. Notes state only what the command text says:
  no guessing at environment variables or credentials a program reads internally, and no
  dependency graph between actions.

## 3. Principles for changes

1. **Read-only, always.** Any change that reads a file through anything other than
   `repo::DirInfo::read` / `read_text` / the `Context` caches, or that spawns a process, is
   wrong by construction.
2. **Nothing ecosystem-specific leaves its adapter.** Adapters may use any internal
   representation, but only `model::Action`, `Script` and `ToolVersion` come out.
3. **Declared beats inferred.** A command the project wrote down wins id collisions and is
   shown by default; an inferred one that loses its natural name to an action that can stand
   in for it (same ecosystem, or a generic runner such as `make`) is not reported.
   **A project is its directory.** Nested projects are named and titled by their path, never
   by a manifest name, so no tool's naming takes priority over another's.
4. **Explanations state observable behaviour.** "Run Vitest tests", not "Runs the unit test
   suite to validate correctness". Under about sixty characters. No articles: "Build app with
   Vite", not "Build the app with Vite". `explain::terse` enforces this on every description,
   so write knowledge-table text naturally and let the pass drop `the`, `a` and `an`.
5. **Risk is a maximum, never lowered.** Tool knowledge, textual patterns and adapter hints
   combine with `max()`. New patterns go in with a test that also shows a safe near-miss.
6. **Every behaviour change is visible in a snapshot.** Fixture repositories under `fixtures/`
   and snapshots under `tests/snapshots/` are the specification of the output. A change with
   no snapshot movement changed nothing user-visible; a snapshot that moved must be explained.
7. **One source of truth per fact.** The support matrix lives in `support.toml` and nowhere
   else; the README table is generated from it and a test enforces that.
8. **Fast by construction.** One directory walk, an index for path lookups, a cache for parsed
   manifests, static parsing only. Discovery of a normal repository takes milliseconds.
9. **The listing is everything rhow reports, and nothing is said twice.** There is no hidden
   tier and no `--all`: what another tool or an ancestor project already covers is not
   reported at all, so the list stays the one a developer would type. The rules that keep it
   short are general, never per-repository:
   - a workspace root offers `build`/`test`/`lint`/… once; the same inferred action is not
     repeated for every member (`tokio-util:test`, `api:clippy`);
   - a declared script that every workspace package repeats (`compile`, `test` in 200
     packages) is boilerplate: shown once at the root as "run in all packages", not per package;
   - in a repository with many projects, an inferred convention action that repeats across
     nested projects (`cargo test` in every crate) is boilerplate; one that only a project or
     two have (a Pulumi stack's `pulumi up`) is that project's own and stays;
   - two tools describing the same thing show once (`test` from `package.json` and the Nx
     `test` target; `ios` and `run-ios`); declared beats inferred, earlier adapter wins, and
     Bazel or Pants beat a language convention at the root they build. Two tools of different
     ecosystems do not describe the same thing: `yarn test` leaves `bundle exec rspec`;
   - inherited Nx targets (`"lint": {}` filled in by `targetDefaults`) show once at the root,
     not per project, unless they are core developer targets;
   - projects under `test/`, `examples/`, `fixtures/`, `playground/`, `benches/`, … are
     demoted to low confidence and never claim Docker, Kubernetes or Terraform directories;
   - `android/`, `ios/`, `linux/`, … inside a Flutter, Expo or React Native app are build
     scaffolding, not projects;
   - an Expo or React Native app is recognised by its app config, never by a hoisted
     dependency in a workspace root;
   - five or more Terraform roots under one project are a module library, not deployments.
10. **Real repositories are the test that matters.** Fixture snapshots pin the rules; before
    changing discovery, run `rhow` on a few large real repositories (an Nx workspace, a Cargo
    workspace, a pnpm monorepo, a Flutter samples repo, a Go module tree) and read the output
    as its developer would. If a line makes no sense to them, it is a bug.
    `tests/integration/` makes this repeatable: a Docker run checks rhow against three real
    products per supported stack (listed in the README), each pinned to a commit, and compares
    the default view with `tests/integration/expected/`. Those files are snapshots of real
    repositories: a change that moves one must be explained, like any fixture snapshot.
    Snapshots only catch change, so `tests/integration/must.txt` pins by hand the commands a
    developer of each product would type; it is never re-recorded, and a command leaving the
    default view is a regression. `--group`, `--ci` and `--json` are checked against the
    listing: the same rows reordered, one row per CI step, the same actions.

## 4. Architecture in one screen

```text
cwd ─► repo::find_root ─► repo::scan ─► discover (adapters) ─► explain + risk ─► ids ─► ui
```

| Module          | Responsibility                                                                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/repo/`     | Find the git root; one read-only walk producing `DirInfo` listings; glob matching                                                                                   |
| `src/discover/` | One adapter per ecosystem implementing `Discoverer`; the orchestrator that detects, attributes helper directories to projects, finalises actions and assigns ids    |
| `src/analyze/`  | Shell-aware tokenizer, wrapper peeling (`npx`, `uv run`, `cross-env`, …), script-reference resolution, and the tool knowledge table split per family under `tools/` |
| `src/explain/`  | Descriptions: explicit → command analysis → name heuristics → `Run <program>`                                                                                       |
| `src/risk/`     | `safe` / `external` / `destructive` from tool evidence plus textual patterns                                                                                        |
| `src/support/`  | Embeds `support.toml`; compares a repository's declared versions against it                                                                                         |
| `src/techs.rs`  | Names the technologies a project visibly uses (kind, tool families, recognised tools, Compose images) for the title tag                                            |
| `src/runtime/`  | Suggestions about the local machine (Colima, OrbStack, Podman, Docker Desktop)                                                                                      |
| `src/ui/`       | Terminal rendering, `--json`, `rhow support`                                                                                                                        |
| `src/model.rs`  | `Repo → Project → Action`, plus `ToolVersion`                                                                                                                       |

Key `Action` fields: `id` (unique CLI name, `api:test`), `command`, `working_directory`,
`source` (declared/inferred), `confidence`, `risk`, `category`.

Detailed rules for id assignment, attribution, analysis depth limits and the explanation
grammar are in `docs/ARCHITECTURE.md`.

## 5. How to make common changes

**Add an ecosystem.** Create `src/discover/<name>.rs` implementing `Discoverer` (`detect` on a
`DirInfo`, `discover` producing `Discovery`), register it in `discover::all()` in priority
order, add a fixture directory under `fixtures/<name>/`, add the name to the
`fixture_tests!` list in `tests/fixtures.rs`, run `cargo test`, review the new snapshot with
`cargo insta review`, and add a row to the README ecosystem table. If the ecosystem declares a
version worth checking, add a `[[tool]]` to `support.toml` and call `out.version(...)`.

**Teach rhow a command.** Add a match arm in the right file under `src/analyze/tools/`
(`js_frameworks`, `db`, `python`, `docker`, `infra`, …) returning `s(tool, text, Kind, Risk)`.
Add a unit test next to the existing ones in `src/analyze/tools/mod.rs` and, if a fixture
uses that command, accept the snapshot change.

**Change a risk rule.** Edit `src/risk/mod.rs` (textual patterns) or the tool's arm (tool
evidence). Extend `destructive_patterns`, `external_patterns` and `safe_when_unsure` so a
near-miss stays safe.

**Raise a supported version.** Edit `max` and `verified` for the tool in `support.toml`, run
`cargo test`, paste the printed table into the README block between the
`support-matrix` markers.

**Change packaging or an installer.** Edit `install.sh`, `packaging/*` or the release
workflow, then run `tests/install/run.sh` (Docker). It builds a local mirror of GitHub
Releases from the working tree and installs through each channel in a throwaway container.

## 6. Verification

Every change must pass, in this order:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
tests/install/run.sh        # when packaging, installers or the release layout changed
```

CI (`.github/workflows/ci.yml`) runs the first three on Linux, macOS and Windows, and the
installer suite and the real-product integration run (`tests/integration/run.sh`) in Docker
on Linux. `main` is protected by a ruleset: changes land only
through a pull request with those checks green, merged by a maintainer by hand. Auto-merge
is off.

Performance sanity: fixtures discover in under a millisecond; a synthetic tree of 8,000
directories takes about a quarter of a second, almost all of it filesystem time.

## 7. Conventions

- Rust 2021, MSRV in `Cargo.toml`. No new dependencies without a reason in the PR.
- Adapters never call `std::fs`, `Path::is_file` or `Path::exists`; they go through `DirInfo`
  and `Context` (`ctx.has_file`, `ctx.text`, `ctx.toml`, …). Ignored directories are listed
  shallowly for exactly this reason.
- Terminal glyphs must be single width and drawn by every default monospace font on macOS,
  Linux and Windows: Mathematical Operators (`∞ ∆ ⊙`), basic Arrows (`↓ ↑`) and the common
  Geometric Shapes (`● ■ ▲ ◆`) only. No emoji (double width, boxes on the legacy Windows
  console), nothing from Miscellaneous Technical, OCR or the rarer shape ranges (`▯ ◷ ⑂`
  showed as unknown symbols on a stock Mac). When colour is off, the word carries the
  meaning, never the glyph.
- Keep `README.md` truthful: examples there are run by hand before release. The `rhow support`
  table is generated; the sample discovery output is illustrative.
- Snapshots are reviewed diffs, not noise. Never regenerate them blindly.
- Commit messages say what changed and why in one line; details go in the body.

## 8. Decision log

| Date    | Decision                                                           | Why                                                                                                                                                     |
| ------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-09 | Removed action execution (`rhow test` used to run the command)     | rhow is discovery and explanation; execution pulled in quoting, argument forwarding, prompts and per-runner separators for no gain over the native tool |
| 2026-09 | Support matrix moved to `support.toml`, embedded at compile time   | The binary must carry the matrix it was verified with; the file is readable on GitHub without Rust; the README table is generated from it               |
| 2026-09 | Tool knowledge table split into `src/analyze/tools/<family>.rs`    | A 4,000-line match could not be reviewed or tested per ecosystem                                                                                        |
| 2026-09 | Path index and manifest cache in `discover::Context`               | Ancestor lookups were quadratic; 8,000 directories took up to a second                                                                                  |
| 2026-09 | Reserved `why` and `support` as action ids                         | A script with either name was listed but unreachable                                                                                                    |
| 2026-09 | Installer channels tested in Docker against a local release mirror | Installers were only ever exercised by real users after a release; the harness found a PKGBUILD bug on its first run                                    |
| 2026-09 | Noise rules: workspace fan-out, repeated package scripts, cross-tool duplicates, inherited Nx targets, demoted test/example trees, shadowed platform runners, Make variable expansion | Running on real monorepos produced 200 to 2,000 lines where a developer wants 20; each rule is general and pinned by `fixtures/noise` |
| 2026-09 | Projects named by directory, listing grouped by project, rows are commands | Manifest names disagree across tools and force a priority list; the directory is the identity every tool shares, and developers ask "what can I do here", not "all tests everywhere" |
| 2026-09 | Notes (`long-running`, `device`, `download`, `git`) and `--ci` pipeline structure; no environment-variable or credential guessing, no action dependency graph | Notes must be facts the command text states; requirements hidden in code are unknowable and a wrong note is worse than none. CI is shown as the structure it is, not as marks on actions |
| 2026-09 | A declared action only shadows an inferred one of the same ecosystem (or a generic runner); Bazel/Pants beat language conventions at their root; an app that is the only project of its ecosystem keeps its convention actions in large repos | Real products hid their canonical commands: a Rails app's `yarn test` hid `bundle exec rspec`, `pnpm run build` hid `bazel build //...`, firezone's Elixir backend lost `mix test` |
| 2026-09 | Listing rows are numbered; `rhow <row>` opens one | Ids like `build-prod-assemble-debug` are long to retype; a number is two keystrokes. Numbers are per invocation and follow the flags (`--all`, `--group`), ids stay the stable handle for scripts and `--json` |
| 2026-09 | Removed the per-action view (`rhow <row>`, `rhow <id>`, `why`), the row numbers that fed it and the reserved ids; `support` is the only argument | The listing already shows the command, what it does, its risk and notes; the view added the directory twice and debug fields (source, confidence, `risk safe`) a developer has no use for |
| 2026-09 | The directory is the positional argument (`rhow path/to/dir`, `rhow support path/to/dir`); `-C` stays as a hidden alias | Inspecting tools take the directory as an argument (`ls`, `tree`, `tokei`); `-C` is for tools whose arguments mean something else, which rhow's no longer do. A directory named `support` is `./support` |
| 2026-09 | No `--all`, no hidden actions: what another action covers is not reported, everything else is listed (JSON schema 2 drops `hidden`) | rhow is for developers; a hidden tier meant deliberately keeping real commands (lifecycle hooks, `_private` recipes, low-confidence guesses) out of sight. Repeats (per-package copies, cross-tool twins) are dropped instead of hidden |

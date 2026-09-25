# Architecture

`rhow` is a pipeline of small, independently testable stages.

```text
cwd ─► repo::find_root ─► repo::scan ─► discover (adapters) ─► explain + risk ─► ids ─► ui
```

## Model

Everything normalises into `Repo → Project → Action` (`src/model.rs`). Adapters may use any
internal representation (`make::Target`, `just::Recipe`, …) but only `Action` leaves them.

Key `Action` fields:

| Field | Meaning |
|---|---|
| `id` | Globally unique name, for `--json` consumers. Root actions keep their name; nested project actions are prefixed (`api:test`). A nested project's `run`/`dev` collapses to the project name when free |
| `command` | The exact native command the project would run; rhow shows it, never runs it |
| `working_directory` | Relative to the repo root. A workspace member's scripts are shown as run inside the member (`pnpm run dev` in `apps/web`), which is how the per-project listing reads |
| `source` | `declared` (the project wrote it down) or `inferred` (ecosystem convention) |
| `confidence` | `exact` for declared; `high`/`medium`/`low` for inferred. `low` marks a guess (an unknown target, a project under `examples/`); it is still listed |
| `risk` | `safe`, `external`, `destructive` |
| `raw` (internal) | The text to analyse when it differs from `command` (the npm script body, the Make recipe) |

## Discovery

`repo::scan` performs one read-only, depth-first walk (max depth 10) and caches every
directory listing in a `DirInfo`. Directories that never hold a project (`node_modules`,
`target`, `dist`, `bin`, `build`, hidden directories, …) are listed but not descended into
and carry `ignored = true`: adapters can ask whether `bin/rails` or
`.config/dotnet-tools.json` exists without touching the filesystem, and detection never
runs inside them. `env/` and `venv/` are only skipped when they hold `pyvenv.cfg`, so
Terraform environments under `env/` are found. Because the walk is depth-first, a
directory's subtree is one contiguous slice of the listing (`repo::subtree`).

`Context` indexes the listing by relative path, so ancestor walks are O(depth) and
`descendants()` is O(subtree), and caches file text and parsed JSON, TOML and YAML so
workspace lookups parse each root manifest once (`ctx.text`, `ctx.json`, `ctx.toml`,
`ctx.yaml`, `ctx.has_file`, `ctx.has_dir`, `ctx.child`).

`discover::discover` then:

1. Runs every adapter's `detect(dir)` over every directory. Adapters that `claims_subtree()`
   (Kubernetes) suppress detection inside a subtree they already own.
2. Marks a directory as a **project** when a non-attachable adapter detects it (or it is the root).
3. Attributes **attachable** adapters (Docker, Kubernetes, scripts) to the nearest ancestor
   project, so `docker/compose.yml` and `k8s/overlays/*` become actions of the project that
   contains them instead of separate projects.
4. Calls `discover(ctx, base, dirs)` per (project, adapter) and collects actions plus a
   *script table* (`family:name → body`) used to resolve references such as `npm run build`
   or `make test` during analysis.
5. Finalises each action: description (unless explicit), category (unless set), risk (max of
   adapter, analysis and textual heuristics).
6. Drops what another action already covers; nothing is hidden, everything left is listed:
   - same-name or same-meaning actions from two tools in one project (declared beats inferred,
     earlier adapter wins, except that Bazel and Pants win over a language convention at the
     root they build; same-tool variants such as `test:watch` are never touched). The keeper
     must be able to stand in for the other: same ecosystem, or a generic runner (`make`,
     `just`, `task`, Bazel) in front of a language's command. A Rails app's `yarn test` leaves
     `bundle exec rspec` listed, its `make test` does not; `dotnet build` leaves the project's
     own `build.ps1`, and `build.cmd` and `build.ps1` are two commands;
   - an inferred command that is exactly the body of a declared script next to it
     (`"services": "docker compose up -d"` and Compose's own `docker compose up -d`);
   - inferred non-Development actions of a nested project that its nearest ancestor project
     already offers with the same name and tool (workspace fan-out);
   - a declared script repeated with the same explanation in five or more nested packages;
     the root gains one inferred "run it in all packages" action for it instead;
   - when more than twelve nested projects have actions, an inferred non-Development action
     whose tool and name repeat in three or more of them (`cargo test` in 200 crates); one
     that is a project's own (the one Pulumi stack's `pulumi up`, the Phoenix backend's
     `mix test`) stays;
   - a Taskfile `internal` task (`task` refuses to run it), an Nx `nx:noop` target, a Make
     `.SPECIAL` or file target.
   Projects under a demoted directory (`test/`, `examples/`, `fixtures/`, `playground/`,
   `benches/`, `*-tests/`, …) keep their actions at low confidence; they are listed.
   Directories named `android/`, `ios/`, `macos/`, `linux/`, `windows/`, `web/` inside a
   Flutter, Expo or React Native app are shadowed entirely: nothing below them is detected.
7. Assigns ids. Declared actions win collisions; an inferred action that loses its natural id is
   dropped when the holder could stand in for it (same ecosystem or a generic runner), because
   the project already exposes that name.
8. Optionally probes the local machine for runtime suggestions (Colima etc.).

### Adding an ecosystem

Implement `discover::Discoverer`:

```rust
impl Discoverer for Gradle {
    fn id(&self) -> &'static str { "gradle" }
    fn kind(&self) -> ProjectKind { ProjectKind::Jvm }
    fn detect(&self, dir: &DirInfo) -> bool { dir.has_any(&["build.gradle", "build.gradle.kts"]) }
    fn discover(&self, ctx: &Context, base: &DirInfo, dirs: &[&DirInfo]) -> Discovery { … }
}
```

and register it in `discover::all()`. Never spawn a process in `detect`/`discover`. Projects
are named after their directory, never after a manifest: manifests disagree with each other
and the file structure is the identity every tool shares.
Add tool knowledge to `analyze/tools/<family>.rs` (not a new adapter) when a *command* needs
explaining; `tools::summarize` chains the per-family tables in order.
Not covered yet: Rake as its own adapter, Pants, moon, NUKE, Cake.

## Identity and presentation

**A project is its directory.** Every nested project is named after the directory it lives in
(`owner` for `apps/owner`) and shown under its path. rhow never names a project after a manifest.
Manifests disagree with each other (`package.json` says `boatsetterowners`, Nx's `project.json`
says `owner`, a `.csproj` says something else again), each tool has its own idea of a name, and
any rule that picks one becomes a priority list that people argue about. The file structure is
the one identity every tool and every developer already shares, and it is what you `cd` into.
The root project is named after the repository directory.

**The listing is grouped by project, not by action type.** The question a developer asks is
"what can I do in this project", so the terminal view is one block per project, root first,
each headed by its path. Inside a block the actions are ordered by type (development, testing,
quality, build, database, infrastructure, release, other) so related lines sit together, with
no sub-headers. A repository with a single project shows no headers at all: the actions,
ordered by type, with a blank line between types.

**The line is the command.** Each row is the native command exactly as the developer would
type it inside that project's directory (`pnpm run dev` in `apps/web`, not
`pnpm --filter web dev` from the root), followed by the explanation and, when it applies, the
risk marker. Column width is computed per block so one long command does not push every block's
text right. Action ids (`web:test`) exist for `--json`, but they are not what the listing
shows.

## Analysis

`analyze::analyze(cmd, resolver)`:

1. **Tokenise** with shell semantics (quotes, escapes, `&&`, `||`, `;`, `|`, redirections,
   env assignments). Windows paths survive.
2. **Peel wrappers**: `npx`, `bunx`, `pnpm exec`, `yarn dlx`, `uv run`, `poetry run`,
   `python -m`, `cross-env`, `dotenv`, `env`, `sh -c`, `concurrently`, `npm-run-all`, ….
3. **Resolve references** (`npm run x`, `pnpm x`, `make x`, `just x`, `task x`, `cargo <alias>`)
   through the project's script table, with cycle and depth guards.
4. **Summarise** each invocation with `tools::summarize` into text + `Kind` + `Risk`.

## Explanation

Precedence: explicit description → command analysis → action-name heuristics → `Run <program>`.

Several steps combine to `Run linting, type checks, and tests` when every step has a noun,
otherwise `A, then b`, otherwise the most significant step plus `(+N more steps)`.

Every description, whatever its source, passes through `explain::terse`, which drops the
articles `the`, `a` and `an` (`Build app with Vite`). Explicit descriptions additionally go
through `explain::tidy` (first line, capitalised, no trailing period). Duplicate detection
compares the final text, so two tools describing the same thing still match.

## Labels

Two independent classifiers run on every action's analysis text (the script body, the Make
recipe, or the command itself) and on CI steps. Both read only the command text and the tools
recognised during analysis. Neither infers what a program does at run time.

**Risk** (`risk::classify`, `model::Risk`) is one of `Safe < External < Destructive`, ordered so
`max()` picks the most severe. Evidence combines from three sources: the knowledge table
(`tools::summarize` returns a risk per recognised invocation), token-sequence patterns on the
raw text (`risk::textual`, e.g. `["down", "-v"]`, `["terraform", "destroy"]`, `["git", "push"]`),
and adapter hints (an Expo `eas build` action is external by construction). An adapter can raise
risk, never lower it. `rm -rf` of a known build output (`tools::is_build_output`) is `Safe`
cleanup; any other recursive delete is `Destructive`. `kubectl apply --dry-run` and
`helm --dry-run` are exempt from the external patterns. A name-based hint (`deploy`, `publish`)
applies only when the command is opaque and declared. The classifier prefers silence to a wrong
label.

**Notes** (`notes::classify`, `model::Note`) are zero or more of, in this order:

| Note | Evidence |
|---|---|
| `LongRunning` | any recognised step of `Kind::Dev`; `--watch`/`-w` on build and test tools; `watch` subcommands; `tail -f`, `logs -f`; watchers (`nodemon`, `watchexec`, `air`); `compose up` without `-d`; `storybook dev`; `flutter run`; `expo start`/`run:*` |
| `Device` | `expo run:*`, `flutter run/install/drive`, `react-native run-*`, Gradle `installDebug`/`connectedAndroidTest`, `xcodebuild` with a simulator destination, `maestro`, `detox test`, `adb`, `xcrun simctl` |
| `Download` | package installs and adds across ecosystems, `docker pull`, `git fetch/pull/clone`, `curl`/`wget`, system package managers, `terraform init`, `helm repo/pull`, `pre-commit install`, `playwright install`; only when the risk is below `External` |
| `Git` | `git` subcommands that change the tree, history or tags; `gh pr`/`release`; `npm version`; `cargo release`; `changeset version/publish`; release tools; hook installers (`husky`, `lefthook`, `pre-commit`) |

Runners in front of the tool (`npx`, `bunx`, `pnpm exec`, `bun x`) are peeled before matching.
`npx`/`bunx` themselves are not a download: they run the installed binary when present.

**Rendering.** `ui::trailer` prints the risk (a coloured `●` plus the word, or `[word]` without
colour) followed by each note as `glyph word`. Glyphs come only from Unicode blocks every
default monospace font draws (Mathematical Operators, basic Arrows, common Geometric Shapes);
see AGENTS.md. A description whose meaningful words all appear in the command is omitted
(`ui::restates`), so a line never explains what it already shows.

**Not labelled, by decision:** environment variables or credentials read inside code,
dependencies between actions, duration.

## CI

`ci::discover` parses `.github/workflows/*.yml` and `.gitlab-ci.yml` into
`CiPipeline → CiJob → CiStep`. A step is a `run` block (or a GitLab `script` line) explained by
the same analysis as an action, with script references resolved through the root project's
scripts. `rhow --ci` renders the structure; `--json` includes it under `ci`. Hidden directories
are otherwise skipped by the scan, so this reads those two locations directly, read-only.

## Presentation flags

By default a project's commands are listed in the order the project declares them. `--group`
orders them by type (`ui::GROUP_ORDER`: development, build, release, testing, quality, database,
infrastructure, other), with a blank line between types in a single-project repository.

## Safety

Discovery reads files. It never writes, never runs `npm`, `gradle`, `cargo metadata`,
`kubectl`, `docker` or anything else. `tests/fixtures.rs::discovery_is_read_only` and
`nothing_spawns_a_process_or_opens_a_socket` guard this.

## Performance

One directory walk, static parsing only, a path index and a manifest cache per scan. Fixture
repositories discover in well under a millisecond each. On a synthetic tree of 8,000
directories the whole run takes about 0.25 s, almost all of it in the filesystem walk itself;
CPU time stays around 20 ms, and the cost grows linearly with the number of directories.

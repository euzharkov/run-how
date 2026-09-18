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
| `id` | Globally unique CLI name. Root actions keep their name; nested project actions are prefixed (`api:test`). A nested project's `run`/`dev` collapses to the project name when free |
| `command` | The exact native command `rhow <id>` executes |
| `working_directory` | Relative to the repo root; workspace members run from the workspace root with a filter |
| `source` | `declared` (the project wrote it down) or `inferred` (ecosystem convention) |
| `confidence` | `exact` for declared; `high`/`medium`/`low` for inferred. `low` is hidden without `--all` |
| `risk` | `safe`, `external`, `destructive` |
| `raw` (internal) | The text to analyse when it differs from `command` (the npm script body, the Make recipe) |

## Discovery

`repo::scan` performs one read-only walk (max depth 10, ignoring `node_modules`, `target`,
`dist`, hidden directories, …) and caches every directory listing in a `DirInfo`.

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
6. Assigns ids. Declared actions win collisions; an inferred action that loses its natural id is
   hidden because the project already exposes that name.
7. Optionally probes the local machine for runtime suggestions (Colima etc.).

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

and register it in `discover::all()`. Never spawn a process in `detect`/`discover`.
Add tool knowledge to `analyze/tools.rs` (not a new adapter) when a *command* needs explaining.
Planned next: Gradle, Maven, Rake, Composer, Nx/Turborepo/Lerna/Rush project graphs, Bazel,
Pants, moon, Terraform/OpenTofu, Pulumi, Ansible, NUKE, Cake.

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

## Risk

`risk::classify` takes the maximum of tool-level evidence and token-sequence patterns on the
raw text. `rm -rf` of build outputs (`dist`, `coverage`, `.turbo`, …) is cleanup; anything
else recursive is destructive. Name-based hints (`deploy`) apply only to opaque commands.

## Safety

Discovery reads files. It never writes, never runs `npm`, `gradle`, `cargo metadata`,
`kubectl`, `docker` or anything else. `tests/fixtures.rs::discovery_is_read_only` guards this.

## Performance

One directory walk, static parsing only, no caching. Fixture repositories discover in
well under a millisecond each; a directory of several large projects in ~60 ms.

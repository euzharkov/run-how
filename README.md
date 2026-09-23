# rhow

**How do I run or operate this repository?**

Enter any unfamiliar repository and run:

```bash
rhow
```

You immediately see the useful actions the project offers, regardless of whether it uses
JavaScript, Python, Go, Rust, .NET, Make, Just, Taskfile, Docker, Kubernetes or a mix of all of them.

```text
$ rhow

polyglot

  pnpm run dev                                         Run dev in web, then run the api project  ∞ long-running
  pnpm run test
  pnpm run lint
  pnpm run typecheck
  pnpm run services                                    Start Docker services in the background
  pnpm run kafka:topics                                List Kafka topics  [external]
  pnpm run kafka:reset                                 Delete Kafka topics  [destructive]
  pnpm run k8s:render                                  Render Kustomize manifests from k8s
  pnpm run k8s:apply                                   Apply Kubernetes resources  [external]
  pnpm run migrate                                     Apply EF Core migrations to the database
  pnpm run reset                                       Stop Docker services and delete their volumes (+2 more steps)  [destructive]
  docker compose -f docker/compose.yml up -d postgres  Start PostgreSQL
  docker compose -f docker/compose.yml up -d kafka     Start local Kafka broker
  docker compose -f docker/compose.yml logs -f         Follow Docker service logs  ∞ long-running
  docker compose -f docker/compose.yml down            Stop and remove Docker services
  ./scripts/setup.sh                                   Bootstrap a fresh development machine

apps/api
  dotnet run                 Start the .NET Api web app
  dotnet build               Build the Api project
  dotnet publish -c Release  Publish Api for deployment

apps/api.tests
  dotnet test  Run Api.Tests tests

apps/web
  pnpm run dev        Start the Vite development server  ∞ long-running
  pnpm run build      Build the app with Vite
  pnpm run test       Run Vitest tests
  pnpm run test:e2e   Run Playwright end-to-end tests
  pnpm run lint       Check source code with ESLint
  pnpm run typecheck  Check TypeScript types

packages/ui
  pnpm run build  Build the package with tsup
  pnpm run test   Run Vitest tests
  pnpm run lint   Check source code with ESLint

services/worker
  go build ./...
  go test ./...   Run Go tests
  go vet ./...    Check Go source with go vet
  go fmt ./...    Format Go source
  go run .        Run the Go worker program
```

`rhow` is not another task runner. It:

1. **Discovers** project actions automatically, across ecosystems and inside monorepos.
2. **Normalises** them into one model (`Action`), whatever tool declared them.
3. **Explains** each action in short plain English, deterministically, without an LLM.
4. **Distinguishes** explicitly declared commands from conventional inferred ones.
5. **Flags** destructive and external actions so you know before you run them.
6. **Shows** the exact native command behind each action (`rhow test` → `pnpm --filter api test`)
   for you to run with the project's own tool. rhow itself never runs anything.

Zero config. Read-only, always. Fast (a normal repository is discovered in a few milliseconds).

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/euzharkov/run-how/main/install.sh | sh
```

The installer detects your OS and CPU, downloads the matching GitHub release, verifies its
SHA256, and installs to `~/.local/bin` without sudo.

Other channels (see [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md)):

```bash
cargo install rhow            # from crates.io
cargo binstall rhow           # prebuilt binary via cargo-binstall
brew install euzharkov/tap/rhow # Homebrew tap
scoop install rhow            # Scoop (Windows)
winget install euzharkov.rhow   # WinGet (Windows)
npx @euzharkov/rhow             # npm launcher around the native binary
```

Prebuilt binaries for macOS (arm64, x64), Linux (arm64, x64, musl) and Windows (arm64, x64)
are published on [GitHub Releases](https://github.com/euzharkov/run-how/releases) with a `SHA256SUMS` file.

## Usage

```bash
rhow                  # list actions grouped by category
rhow --all            # include internal / low-confidence actions
rhow --group          # order each project's commands by type: run, build, deploy, test, …
rhow --ci             # CI pipelines (GitHub Actions, GitLab CI) as workflows, jobs and steps
rhow --json           # the normalised model, for editors and scripts
rhow --ci --json      # only the CI pipelines, as JSON
rhow why 'make test'  # where that command comes from and why it is flagged
rhow why db:reset     # the same, by action id
rhow db:reset --json  # the same, as JSON
rhow support          # which tool versions this build has been verified against
rhow -C path/to/dir   # inspect exactly that directory
```

Without `-C`, rhow walks up from the current directory to the enclosing git repository. When
there is none it inspects the current directory alone and says so on stderr (one line, only
for the terminal listing). `--group` is a terminal ordering and is refused with `--json`.

Every JSON document starts with `"schema": 1` and carries every discovered action: the ones
the listing keeps behind `--all` are marked `"hidden": true` or `"confidence": "low"`, and
an action whose description is a bare `Run <program>` fallback is marked `"opaque": true`,
so consumers filter the same way the terminal does. `root` is absolute; every other path is
relative to it with `/` separators on every platform.

`rhow` never modifies the repository, installs dependencies, starts services, contacts a
cluster or runs project scripts. It is not a task runner: it shows you the command and what it
does, and you run it with the tool the project already uses. That keeps rhow free of shell
quoting, argument forwarding, environment handling and confirmation prompts.

Output uses ANSI colour only on a TTY and stays plain when piped. `--color always|never`
wins; otherwise `NO_COLOR` turns colour off, and `CLICOLOR_FORCE=1` or `FORCE_COLOR`
(non-empty, not `0`) turns it on even through a pipe.

## Supported ecosystems

| Ecosystem | Detected from | Notes |
|---|---|---|
| JavaScript / TypeScript | `package.json`, lockfiles, `pnpm-workspace.yaml` | npm, pnpm, Yarn, Bun; workspaces become separate projects with correct filter commands |
| Make | `Makefile`, `makefile`, `GNUmakefile` | `.PHONY` and `##` comments; file-like and internal targets hidden |
| Just | `justfile` | Comments, `[doc]`, `[private]`, `[group]`, aliases |
| Taskfile | `Taskfile.yml` | `desc`/`summary`, `internal`, one level of `includes` |
| Python | `pyproject.toml`, `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile`, `tox.ini`, `noxfile.py`, `manage.py`, … | uv/Poetry/PDM/Pipenv runners, pytest, Ruff, mypy, tox, nox, Django, `[project.scripts]`, PDM scripts, Poe tasks. `pip` is never a task |
| Go | `go.mod`, `go.work` | build/test/vet/fmt, `run` per `cmd/*`, `generate` only when `//go:generate` exists |
| Rust | `Cargo.toml`, `.cargo/config.toml` | run/build/test/check/clippy/fmt/bench, workspaces (`-p`), Cargo aliases |
| Ruby / Rails | `Gemfile`, `Rakefile`, `bin/rails`, `config.ru` | Rake tasks with `desc`, `bin/dev`, server/console, db tasks, RSpec or Minitest, RuboCop, Brakeman |
| .NET | `*.sln`, `*.slnx`, `*.csproj`, `*.fsproj` | Solution vs project hierarchy, web/worker/exe/test flavours, EF Core, `*.ps1`/`*.cmd`/`*.bat` scripts, Cake and NUKE hooks |
| Docker | `Dockerfile`, `compose.yml`, `docker-compose.yml` | Compose services become actions with image-aware descriptions (PostgreSQL, Kafka, Redis, …) |
| Kubernetes | `Chart.yaml`, `kustomization.yaml`, manifest directories | Helm, Kustomize overlays, plain manifests. Discovery never contacts a cluster |

Command analysis recognises well over a hundred tools (Vite, Next, Vitest, Jest, Playwright,
Cypress, Detox, Maestro, ESLint, Prettier, tsc, Prisma, Drizzle, pytest, Ruff, mypy, uvicorn, Django, go, cargo, dotnet,
docker, docker compose, kubectl, helm, terraform, pulumi, git, curl, rm, …) so that

```json
"check": "eslint . && tsc --noEmit && vitest run"
```

becomes `Run linting, type checks, and tests`, and

```json
"xyz": "node ./scripts/foo.mjs"
```

stays honest: `Run scripts/foo.mjs`.

## Labels

Every command can carry up to two kinds of label after its explanation. Both come from the
command text and the tools rhow recognises in it, never from running anything or guessing at
what a program does internally.

### Risk: what it changes

Exactly one of three levels, shown only when it is not `safe`.

| Label | Meaning | Examples |
|---|---|---|
| *(none)* `safe` | local, reversible or read-only | `vitest run`, `rm -rf dist`, `docker compose down`, `terraform plan` |
| `● external` | changes something outside your working copy: a registry, a cluster, a remote, a cloud account | `npm publish`, `docker push`, `terraform apply`, `kubectl apply`, `git push`, `helm upgrade` |
| `● destructive` | deletes data or state you cannot trivially get back | `rm -rf src`, `docker compose down -v`, `prisma migrate reset`, `terraform destroy`, `kubectl delete`, `git push --force` |

Risk is a maximum: tool knowledge, textual patterns and adapter hints combine and the most
severe wins. Deleting a known build output (`dist`, `coverage`, `.turbo`, …) is cleanup, not
destruction. When unsure the classifier stays silent; a missing warning beats a wrong one.

### Notes: what running it involves

Any number of these, in this order.

| Label | Meaning | Examples |
|---|---|---|
| `∞ long-running` | keeps running until you stop it | `vite`, `tsc --watch`, `docker compose up`, `docker compose logs -f`, `flutter run` |
| `⊙ device` | needs a phone, tablet, simulator or emulator attached | `expo run:ios`, `flutter run`, `./gradlew installDebug`, `maestro test` |
| `↓ download` | fetches packages, images or providers; changes nothing remote | `npm install`, `pip install`, `docker pull`, `terraform init`, `pre-commit install` |
| `∆ git` | changes the git working tree, history or tags | `git commit`, `git clean -fdx`, `npm version`, `changeset version`, `husky install` |

`download` and `external` never appear together: external already implies the network, and
the note is only for reads. A remote git read (`git fetch`) counts as external under the risk
model, so it carries the risk label rather than the note.

### What is deliberately not labelled

- Environment variables or credentials a program needs. They are usually read inside code,
  where rhow cannot see them, and a wrong note is worse than none.
- Dependencies between actions ("run install first"). Out of scope.
- Duration. Nothing in the files says how long a test suite takes.

### Rendering

In a colour terminal the risk is a yellow or red dot, `● external` / `● destructive`; without
colour (`--color never`, `NO_COLOR`, a pipe) it is `[external]` / `[destructive]`, so the word
carries the meaning, never the glyph. Note glyphs are single-width characters that every
default monospace font on macOS, Linux and Windows draws. In `--json` the fields are
`risk` (`safe`, `external`, `destructive`) and `notes` (`long-running`, `device`, `download`,
`git`).

## CI as a structure

```bash
rhow --ci
```

GitHub Actions workflows and `.gitlab-ci.yml` are shown as they are written: workflow, trigger,
jobs, then each `run` step as a command with the same explanation, risk and notes as an action.
`npm test` inside a workflow resolves through the root project's scripts, so it reads
"Run Vitest tests" rather than "the test script". Nothing is evaluated: matrix expressions and
shell conditionals are shown as text.

## Version support

```bash
rhow support         # what this build has been verified against, and how the repo compares
rhow support --json  # the same, as data
```

Ecosystems change: a new Rust edition, a new Taskfile schema, a .NET target framework or a
Terraform major version can all show up in a repository before `rhow` has been checked against
it. `rhow support` reads the same declared versions the normal discovery pass already sees — a
Cargo edition, a `go.mod` directive, `engines.node`, a `packageManager` pin, `required_version`,
a Gradle wrapper, `.bazelversion`, `.ruby-version`, `require.php` — compares each one against a
static baseline, and flags anything past it as "newer than verified" instead of silently
explaining it as if nothing had changed. This is static and read-only like everything else in
`rhow`: no version check ever runs a subprocess or contacts a registry.

The matrix below is generated from [`support.toml`](support.toml), the single source of truth
that the binary embeds and `rhow support` prints. A test fails if this table and the file
disagree.

<!-- support-matrix:start -->
| Tool | Verified against | Read from |
|---|---|---|
| Rust / Cargo | editions 2015-2024 | Cargo.toml `edition` (own or inherited from `[workspace.package]`) |
| Go | up to Go 1.26 | the `go` directive in go.mod / go.work |
| .NET | net5.0-net10.0 (net48 and netstandard* recognised but not version-checked) | `<TargetFramework(s)>` in .csproj/.fsproj/.vbproj |
| Python | up to Python 3.14 (compound constraints like `>=3.9,<4` are left unclear) | `project.requires-python` in pyproject.toml |
| Taskfile | schema version 3 | `version:` in Taskfile.yml |
| Node.js | up to Node 24 | `engines.node` in package.json |
| npm | up to npm 11 | the `packageManager` field in package.json |
| pnpm | up to pnpm 10 | the `packageManager` field in package.json, pnpm-workspace.yaml |
| Yarn | up to Yarn 4 (Berry) | the `packageManager` field in package.json |
| Bun | up to Bun 1 | the `packageManager` field in package.json, bun.lock(b) |
| Ruby | up to Ruby 4.0 | .ruby-version |
| PHP | up to PHP 8.5 | `require.php` in composer.json |
| Terraform | up to Terraform 1.13 | `required_version` in a `terraform {}` block |
| OpenTofu | up to OpenTofu 1.10 | `required_version` in a `terraform {}` block |
| Gradle | up to Gradle 9.1 | gradle/wrapper/gradle-wrapper.properties `distributionUrl` |
| Bazel | up to Bazel 8.3 | .bazelversion |
<!-- support-matrix:end -->

**Maintaining this**: when you've verified `rhow` against a newer version of a tool, bump that
tool's `max` and `verified` in [`support.toml`](support.toml), run `cargo test`, and paste the
table it prints into the block above. That file is the entire support matrix. A value the comparator can't safely reduce to a number (a compound
range like `>=3.9,<4`, a pre-release tag) is reported as "unclear" rather than guessed at, the
same false-negative-over-false-positive preference the risk classifier uses.

## Architecture

```text
src/
  repo/       repository root detection and read-only directory scan
  discover/   one adapter per ecosystem, all producing normalised `Action`s
  analyze/    shell tokenising, wrapper peeling (npx, uv run, cross-env…), tool knowledge
  explain/    deterministic descriptions: explicit → analysis → name heuristics → fallback
  risk/       destructive / external classification
  runtime/    local runtime suggestions (Colima, OrbStack, Podman, Docker Desktop)
  support/    the version-support registry `rhow support` checks a repo against
  ui/         terminal and JSON rendering
```

Adding an ecosystem means implementing the `Discoverer` trait (detect files, emit actions);
nothing ecosystem-specific escapes the adapter. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Development

```bash
cargo test                   # unit tests + fixture snapshots (tests/snapshots)
cargo insta review           # review changed snapshots (cargo install cargo-insta)
cargo run -- -C fixtures/polyglot
```

Fixture repositories for every ecosystem live in `fixtures/`, including monorepos, duplicate
script names, shell chains, quoted arguments, Windows paths, PowerShell scripts, recursive npm
scripts, unknown scripts and destructive commands.

## Non-goals

Not a task runner: rhow never executes an action, forwards arguments or manages environment
variables. No LLM explanations, telemetry, cloud accounts, remote execution, plugins,
favourites, history, configuration UI or full-screen dashboard. No `.rhow.yml` required, ever.

## License

MIT

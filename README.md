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

polyglot  ·  5 projects: Api, Api.Tests, web, ui, worker

Development
  dev             Run dev in web, then run the api project
  api             Start the .NET Api web app
  web             Start the Vite development server
  worker          Run the Go worker program

Testing
  test            Run test in all workspace packages
  api.tests:test  Run Api.Tests tests
  web:test        Run Vitest tests
  web:test:e2e    Run Playwright end-to-end tests

Quality
  lint            Run lint in all workspace packages
  web:typecheck   Check TypeScript types

Database
  migrate         Apply EF Core migrations to the database

Infrastructure
  services        Start Docker services in the background
  postgres        Start PostgreSQL
  kafka           Start local Kafka broker
  k8s:render      Render Kustomize manifests from k8s
  reset           Stop Docker services and delete their volumes (+2 more steps)  ⚠ destructive

Release
  k8s:apply       Apply Kubernetes resources                                     ⚠ external
```

`rhow` is not another task runner. It:

1. **Discovers** project actions automatically, across ecosystems and inside monorepos.
2. **Normalises** them into one model (`Action`), whatever tool declared them.
3. **Explains** each action in short plain English, deterministically, without an LLM.
4. **Distinguishes** explicitly declared commands from conventional inferred ones.
5. **Flags** destructive and external actions before you run them.
6. **Runs** the native underlying command on request (`rhow test` → `pnpm --filter api test`).

Zero config. Read-only by default. Fast (a normal repository is discovered in a few milliseconds).

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/line-19/rhow/main/install.sh | sh
```

The installer detects your OS and CPU, downloads the matching GitHub release, verifies its
SHA256, and installs to `~/.local/bin` without sudo.

Other channels (see [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md)):

```bash
cargo install rhow            # from crates.io
cargo binstall rhow           # prebuilt binary via cargo-binstall
brew install line-19/tap/rhow # Homebrew tap
scoop install rhow            # Scoop (Windows)
winget install line-19.rhow   # WinGet (Windows)
npx @line-19/rhow             # npm launcher around the native binary
```

Prebuilt binaries for macOS (arm64, x64), Linux (arm64, x64, musl) and Windows (arm64, x64)
are published on [GitHub Releases](https://github.com/line-19/rhow/releases) with a `SHA256SUMS` file.

## Usage

```bash
rhow                  # list actions grouped by category
rhow --all            # include internal / low-confidence actions
rhow --json           # the normalised model, for editors and scripts
rhow test             # run the `test` action with its native tool
rhow api:test -- -v   # append arguments to the underlying command
rhow --dry-run deploy # print what would run
rhow why db:reset     # where an action comes from and why it is flagged
rhow -C path/to/repo  # inspect another directory
```

Running `rhow` with no action never modifies the repository, installs dependencies, starts
services, contacts a cluster or runs project scripts. Execution only happens when you name an
action, and external or destructive actions ask for confirmation first (`--yes` skips it).

Output respects `NO_COLOR`, uses ANSI colour only on a TTY, and stays plain when piped.

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

## Risk classification

| Command | Risk |
|---|---|
| `rm -rf src`, `docker compose down -v`, `prisma migrate reset`, `terraform destroy`, `kubectl delete`, `git push --force` | destructive |
| `npm publish`, `docker push`, `terraform apply`, `kubectl apply`, `git push`, `helm upgrade` | external |
| `rm -rf dist`, `docker compose down`, `terraform plan`, `vitest run` | safe |

The classifier prefers a missing warning over a wrong one when confidence is low.

## Architecture

```text
src/
  repo/       repository root detection and read-only directory scan
  discover/   one adapter per ecosystem, all producing normalised `Action`s
  analyze/    shell tokenising, wrapper peeling (npx, uv run, cross-env…), tool knowledge
  explain/    deterministic descriptions: explicit → analysis → name heuristics → fallback
  risk/       destructive / external classification
  runtime/    local runtime suggestions (Colima, OrbStack, Podman, Docker Desktop)
  exec/       running an action through its native tool
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

No LLM explanations, telemetry, cloud accounts, remote execution, plugins, favourites, history,
configuration UI or full-screen dashboard. No `.rhow.yml` required, ever.

## License

MIT

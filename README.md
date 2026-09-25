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

storefront  [Node.js, Docker, Turborepo, Playwright, PostgreSQL]

  pnpm run dev                   Run dev across packages with Turborepo  ∞ long-running
  pnpm run build                 Run build across packages with Turborepo
  pnpm run test                  Run test across packages with Turborepo
  pnpm run lint                  Run lint across packages with Turborepo
  pnpm run typecheck             Run typecheck in all workspace packages
  pnpm run services              Start postgres and redis services in background
  pnpm run e2e                   Run Playwright end-to-end tests
  docker compose up -d           Start all Docker services
  docker compose up -d postgres  Start PostgreSQL
  docker compose up -d redis     Start Redis
  docker compose up -d orders    Start orders service (built from source)
  docker compose up -d payments  Start payments service (built from source)
  docker compose logs -f         Follow Docker service logs  ∞ long-running
  docker compose build           Build Docker service images
  docker compose down            Stop and remove Docker services
  docker compose down -v         Stop Docker services and delete their volumes  [destructive]

apps/bff  [Node.js, NestJS, Jest, Prisma]
  pnpm run dev         Start NestJS app in watch mode  ∞ long-running
  pnpm run build       Build NestJS app
  pnpm run start       Run dist/main
  pnpm run test        Run Jest tests
  pnpm run test:e2e    Run Jest tests
  pnpm run lint        Check source code with ESLint
  pnpm run typecheck   Check TypeScript types
  pnpm run db:migrate  Apply pending Prisma migrations
  pnpm run db:reset    Reset database and re-apply migrations  [destructive]

apps/mobile  [Node.js, Expo, Jest]
  pnpm exec expo prebuild  Generate native iOS and Android projects
  pnpm exec expo-doctor    Check Expo project for common issues
  pnpm run start           Start Expo development server  ∞ long-running
  pnpm run ios             Build and run app on iOS  ∞ long-running  ⊙ device
  pnpm run android         Build and run app on Android  ∞ long-running  ⊙ device
  pnpm run test            Run Jest tests
  pnpm run lint            Check source code with ESLint

apps/web  [Node.js, Next.js, Vitest, Playwright]
  pnpm run dev        Start Next.js development server  ∞ long-running
  pnpm run build      Build Next.js app
  pnpm run start      Start Next.js production server  ∞ long-running
  pnpm run test       Run Vitest tests
  pnpm run test:e2e   Run Playwright end-to-end tests
  pnpm run lint       Check source code with ESLint
  pnpm run typecheck  Check TypeScript types

packages/ui  [Node.js, Vitest]
  pnpm run build  Build package with tsup
  pnpm run test   Run Vitest tests
  pnpm run lint   Check source code with ESLint

services/orders  [Go]
  go build ./...
  go test ./...        Run Go tests
  go vet ./...         Check Go source with go vet
  go fmt ./...         Format Go source
  go mod tidy          Tidy Go module dependencies  ↓ download
  go run ./cmd/orders  Run orders command

services/payments  [Go]
  go build ./...
  go test ./...          Run Go tests
  go vet ./...           Check Go source with go vet
  go fmt ./...           Format Go source
  go mod tidy            Tidy Go module dependencies  ↓ download
  go run ./cmd/payments  Run payments command
```

`rhow` is not another task runner. It:

1. **Discovers** project actions automatically, across ecosystems and inside monorepos.
2. **Normalises** them into one model (`Action`), whatever tool declared them.
3. **Explains** each action in short plain English, deterministically, without an LLM.
4. **Distinguishes** explicitly declared commands from conventional inferred ones.
5. **Flags** destructive and external actions so you know before you run them.
6. **Shows** the exact native command behind each action (`pnpm --filter api test`) for you
   to run with the project's own tool. rhow itself never runs anything.

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
rhow --group          # order each project's commands by type: run, build, deploy, test, …
rhow --ci             # CI pipelines (GitHub Actions, GitLab CI) as workflows, jobs and steps
rhow --json           # the normalised model, for editors and scripts
rhow --ci --json      # only the CI pipelines, as JSON
rhow support          # which tool versions this build has been verified against
rhow path/to/dir      # inspect exactly that directory (also: rhow support path/to/dir)
```

Without a directory, rhow walks up from the current directory to the enclosing git
repository. When there is none it inspects the current directory alone and says so on stderr
(one line, only for the terminal listing). `--group` is a terminal ordering and is refused with `--json`.

Every JSON document starts with `"schema": 2` and carries exactly the actions the listing
shows: rhow hides nothing. An action whose description is a bare `Run <program>` fallback is
marked `"opaque": true`, and a guess (an unknown target, a project under `examples/`) carries
`"confidence": "low"`. `root` is absolute; every other path is
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
| Make | `Makefile`, `makefile`, `GNUmakefile` | `.PHONY` and `##` comments, `include`d files; file-like and `.SPECIAL` targets skipped |
| Just | `justfile` | Comments, `[doc]`, `[private]`, `[group]`, aliases |
| Taskfile | `Taskfile.yml` | `desc`/`summary`, `internal`, one level of `includes` (`flatten` honoured) |
| Python | `pyproject.toml`, `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile`, `tox.ini`, `noxfile.py`, `manage.py`, … | uv/Poetry/PDM/Pipenv runners, pytest, Ruff, mypy, tox, nox, Django, `[project.scripts]`, PDM scripts, Poe tasks. `pip` is never a task |
| Go | `go.mod`, `go.work` | build/test/vet/fmt, `run` per `cmd/*`, `generate` only when `//go:generate` exists |
| Rust | `Cargo.toml`, `.cargo/config.toml` | run/build/test/check/clippy/fmt/bench, workspaces (`-p`), Cargo aliases |
| Ruby / Rails | `Gemfile`, `Rakefile`, `bin/rails`, `config.ru` | Rake tasks with `desc` (including `lib/tasks/*.rake`), `bin/dev`, server/console, db tasks, RSpec or Minitest, RuboCop, Brakeman, gem build |
| .NET | `*.sln`, `*.slnx`, `*.csproj`, `*.fsproj` | Solution vs project hierarchy, web/worker/exe/test flavours, EF Core, `*.ps1`/`*.cmd`/`*.bat` scripts, Cake and NUKE hooks |
| Docker | `Dockerfile`, `compose.yml`, `docker-compose.yml` | Compose services become actions with image-aware descriptions (PostgreSQL, Kafka, Redis, …) |
| Kubernetes | `Chart.yaml`, `kustomization.yaml`, manifest directories | Helm, Kustomize overlays, plain manifests, Skaffold profiles. Discovery never contacts a cluster |
| Terraform / Terragrunt | `*.tf`, `terragrunt.hcl` | One root per environment directory (`env/`, `envs/`, `stacks/`); Terragrunt units and `run-all` at the config root; five or more roots are a module library |

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

## Tested on real products

Besides the fixture snapshots, rhow is checked against real products built with each stack
it supports: applications and deployments that use the technology, not the technology's own
repository or its examples. Each was chosen for how much of rhow's rule set it exercises
(projects, declared scripts, tools, categories) within a 300 MB checkout, and
each is pinned to a commit, so the run is reproducible and tied to the versions in
`support.toml`. A Docker run fetches exactly that commit (depth 1, no history) one product
at a time into a tmpfs, runs `rhow`, `--group`, `--ci` and `--json`, compares the default view
with `tests/integration/expected/<owner>__<repo>.txt`, checks that the hand-picked commands in
`tests/integration/must.txt` are still in it, checks the other modes against it, grades the
output and deletes the clone. Nothing is installed or executed. The list lives in
`tests/integration/repos.txt`; this table is generated from it and a test keeps the two in
step.

<!-- real-repos:start -->
| Stack | Products |
|---|---|
| Node.js (npm) | [Kong/insomnia](https://github.com/Kong/insomnia), [excalidraw/excalidraw](https://github.com/excalidraw/excalidraw), [louislam/uptime-kuma](https://github.com/louislam/uptime-kuma) |
| pnpm | [hoppscotch/hoppscotch](https://github.com/hoppscotch/hoppscotch), [dubinc/dub](https://github.com/dubinc/dub), [Dokploy/dokploy](https://github.com/Dokploy/dokploy) |
| Yarn | [actualbudget/actual](https://github.com/actualbudget/actual), [rainbow-me/rainbow](https://github.com/rainbow-me/rainbow), [outline/outline](https://github.com/outline/outline) |
| Bun | [openstatusHQ/openstatus](https://github.com/openstatusHQ/openstatus), [midday-ai/midday](https://github.com/midday-ai/midday), [sst/opencode](https://github.com/sst/opencode) |
| Nx | [ever-co/ever-gauzy](https://github.com/ever-co/ever-gauzy), [TryGhost/Ghost](https://github.com/TryGhost/Ghost), [infinitered/reactotron](https://github.com/infinitered/reactotron) |
| Turborepo | [formbricks/formbricks](https://github.com/formbricks/formbricks), [unkeyed/unkey](https://github.com/unkeyed/unkey), [documenso/documenso](https://github.com/documenso/documenso) |
| Deno | [louislam/its-mytabs](https://github.com/louislam/its-mytabs), [bewcloud/bewcloud](https://github.com/bewcloud/bewcloud), [weise25/LocalSite-ai](https://github.com/weise25/LocalSite-ai) |
| Expo / React Native | [mattermost/mattermost-mobile](https://github.com/mattermost/mattermost-mobile), [bluesky-social/social-app](https://github.com/bluesky-social/social-app), [BlueWallet/BlueWallet](https://github.com/BlueWallet/BlueWallet) |
| Rust / Cargo | [zed-industries/zed](https://github.com/zed-industries/zed), [sharkdp/bat](https://github.com/sharkdp/bat), [helix-editor/helix](https://github.com/helix-editor/helix) |
| Go | [go-gitea/gitea](https://github.com/go-gitea/gitea), [traefik/traefik](https://github.com/traefik/traefik), [grafana/k6](https://github.com/grafana/k6) |
| Python | [mealie-recipes/mealie](https://github.com/mealie-recipes/mealie), [open-webui/open-webui](https://github.com/open-webui/open-webui), [home-assistant/core](https://github.com/home-assistant/core) |
| Django | [saleor/saleor](https://github.com/saleor/saleor), [paperless-ngx/paperless-ngx](https://github.com/paperless-ngx/paperless-ngx), [netbox-community/netbox](https://github.com/netbox-community/netbox) |
| Ruby / Rails | [chatwoot/chatwoot](https://github.com/chatwoot/chatwoot), [mastodon/mastodon](https://github.com/mastodon/mastodon), [rubygems/rubygems.org](https://github.com/rubygems/rubygems.org) |
| PHP / Laravel | [BookStackApp/BookStack](https://github.com/BookStackApp/BookStack), [monicahq/monica](https://github.com/monicahq/monica), [koel/koel](https://github.com/koel/koel) |
| .NET | [bitwarden/server](https://github.com/bitwarden/server), [jellyfin/jellyfin](https://github.com/jellyfin/jellyfin), [files-community/Files](https://github.com/files-community/Files) |
| Gradle | [halo-dev/halo](https://github.com/halo-dev/halo), [signalapp/Signal-Android](https://github.com/signalapp/Signal-Android), [thunderbird/thunderbird-android](https://github.com/thunderbird/thunderbird-android) |
| Maven | [keycloak/keycloak](https://github.com/keycloak/keycloak), [apache/dolphinscheduler](https://github.com/apache/dolphinscheduler), [openmrs/openmrs-core](https://github.com/openmrs/openmrs-core) |
| Bazel | [pixie-io/pixie](https://github.com/pixie-io/pixie), [envoyproxy/envoy](https://github.com/envoyproxy/envoy), [kythe/kythe](https://github.com/kythe/kythe) |
| Terraform | [kubernetes/k8s.io](https://github.com/kubernetes/k8s.io), [ministryofjustice/cloud-platform-infrastructure](https://github.com/ministryofjustice/cloud-platform-infrastructure), [cds-snc/notification-terraform](https://github.com/cds-snc/notification-terraform) |
| Kubernetes | [peer-calls/peer-calls](https://github.com/peer-calls/peer-calls), [zwave-js/zwave-js-ui](https://github.com/zwave-js/zwave-js-ui), [szinn/k8s-homelab](https://github.com/szinn/k8s-homelab) |
| Docker Compose | [immich-app/immich](https://github.com/immich-app/immich), [nextcloud/all-in-one](https://github.com/nextcloud/all-in-one), [getsentry/self-hosted](https://github.com/getsentry/self-hosted) |
| Pulumi | [featurehub-io/featurehub](https://github.com/featurehub-io/featurehub), [passportxyz/passport](https://github.com/passportxyz/passport), [OpenPipe/OpenPipe](https://github.com/OpenPipe/OpenPipe) |
| Ansible | [khuedoan/homelab](https://github.com/khuedoan/homelab), [debops/debops](https://github.com/debops/debops), [kubernetes-sigs/kubespray](https://github.com/kubernetes-sigs/kubespray) |
| Taskfile | [direktiv/direktiv](https://github.com/direktiv/direktiv), [browser-use/desktop](https://github.com/browser-use/desktop), [PierreBeucher/cloudypad](https://github.com/PierreBeucher/cloudypad) |
| Just | [tareqimbasher/NetPad](https://github.com/tareqimbasher/NetPad), [pamburus/hl](https://github.com/pamburus/hl), [GitoxideLabs/gitoxide](https://github.com/GitoxideLabs/gitoxide) |
| Make | [redis/redis](https://github.com/redis/redis), [prometheus/prometheus](https://github.com/prometheus/prometheus), [neovim/neovim](https://github.com/neovim/neovim) |
| Swift / Xcode | [wordpress-mobile/WordPress-iOS](https://github.com/wordpress-mobile/WordPress-iOS), [signalapp/Signal-iOS](https://github.com/signalapp/Signal-iOS), [exelban/stats](https://github.com/exelban/stats) |
| Flutter | [AppFlowy-IO/AppFlowy](https://github.com/AppFlowy-IO/AppFlowy), [hiddify/hiddify-app](https://github.com/hiddify/hiddify-app), [localsend/localsend](https://github.com/localsend/localsend) |
| Elixir / Mix | [firezone/firezone](https://github.com/firezone/firezone), [plausible/analytics](https://github.com/plausible/analytics), [supabase/realtime](https://github.com/supabase/realtime) |
| Zig | [tigerbeetle/tigerbeetle](https://github.com/tigerbeetle/tigerbeetle), [ghostty-org/ghostty](https://github.com/ghostty-org/ghostty), [lightpanda-io/browser](https://github.com/lightpanda-io/browser) |
| CMake | [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp), [obsproject/obs-studio](https://github.com/obsproject/obs-studio), [mumble-voip/mumble](https://github.com/mumble-voip/mumble) |
| Clojure | [logseq/logseq](https://github.com/logseq/logseq), [athensresearch/athens](https://github.com/athensresearch/athens), [cljdoc/cljdoc](https://github.com/cljdoc/cljdoc) |
| Haskell | [simonmichael/hledger](https://github.com/simonmichael/hledger), [IntersectMBO/cardano-node](https://github.com/IntersectMBO/cardano-node), [jgm/pandoc](https://github.com/jgm/pandoc) |
<!-- real-repos:end -->

```bash
tests/integration/run.sh                     # every product
tests/integration/run.sh deno                # one stack
tests/integration/run.sh sharkdp/bat         # one product
UPDATE_EXPECTED=1 tests/integration/run.sh sharkdp/bat   # accept its new output
```

`run.sh` builds the image only when rhow's source changed (the product list and expected
outputs are mounted, not baked in), removes the untagged image a rebuild leaves behind, and
runs the check.

The run fails when rhow crashes, times out, emits invalid JSON, misses the stack a product
was chosen for (by whole name: `cargo` does not count as Go), when its default view differs
from the expected output (the diff is saved next to the logs), or when a command in
`must.txt` has left the default view. Expected outputs are recorded by rhow itself, so they
only catch change; `must.txt` is the part chosen by hand (the command a developer of that
product would type to run, test or build it) and is never re-recorded. The three other modes
each get one check: `--group` lists the same rows as the default view, only reordered,
`--ci` prints one row per CI step, and `--json` holds exactly the actions the two listings
show. CI runs the whole suite on every pull request. Long default views, unexplained commands and thin test cases are reported
as warnings. Logs land in `tests/integration/logs/`: one directory per product plus
`summary.md` with each product's checkout size and complexity score. Clones go to a 2 GB
tmpfs and are deleted one by one, so the run never grows Docker's disk. Expected outputs are
the specification of rhow's behaviour on those commits, like the fixture snapshots: a change
that moves one must be explained, and bumping a product's commit means regenerating its
expected output with it.

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
| Go | up to Go 1.26 | the `go` / `toolchain` directives in go.mod / go.work, .tool-versions |
| .NET | net5.0-net10.0 (net48 and netstandard* recognised but not version-checked) | `<TargetFramework(s)>` in .csproj/.fsproj/.vbproj |
| Python | up to Python 3.14 (compound constraints like `>=3.9,<4` are left unclear) | `project.requires-python` or `tool.poetry.dependencies.python` in pyproject.toml, .python-version, .tool-versions |
| Taskfile | schema version 3 | `version:` in Taskfile.yml |
| Node.js | up to Node 24 | `engines.node` in package.json, .nvmrc, .node-version, .tool-versions |
| npm | up to npm 11 | the `packageManager` field in package.json |
| pnpm | up to pnpm 10 | the `packageManager` field in package.json, pnpm-workspace.yaml |
| Yarn | up to Yarn 4 (Berry) | the `packageManager` field in package.json |
| Bun | up to Bun 1 | the `packageManager` field in package.json, bun.lock(b) |
| Ruby | up to Ruby 4.0 | .ruby-version, .tool-versions |
| PHP | up to PHP 8.5 | `require.php` in composer.json |
| Terraform | up to Terraform 1.13 | `required_version` in a `terraform {}` block, .terraform-version, .tool-versions |
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
cargo run -- fixtures/storefront       # try rhow on a fixture repository
```

### Tests

Three layers, from fastest to slowest:

```bash
cargo test                             # unit tests + fixture snapshots + README/support checks
INSTA_UPDATE=always cargo test         # accept changed snapshots (or: cargo insta review)
tests/integration/run.sh               # real products in Docker (see "Tested on real products")
```

- **Unit tests** live next to the code (`src/**`): tokenizer, wrapper peeling, knowledge table,
  Make/Just/Taskfile parsers, id assignment.
- **Fixture snapshots** (`fixtures/` + `tests/snapshots/`) are the specification of the
  output. Every ecosystem has a fixture, including monorepos, duplicate script names, shell
  chains, quoted arguments, Windows paths, PowerShell scripts, recursive npm scripts, unknown
  scripts and destructive commands. A behaviour change must move a snapshot; a moved snapshot
  must be explained in the commit.
- **Integration in Docker** (`tests/integration/`) runs the release binary against ~100 real
  products pinned to commits, compares each default view with `tests/integration/expected/`,
  requires the commands in `tests/integration/must.txt` to stay in it, and checks `--group`,
  `--ci` and `--json` against it. CI runs it on every pull request. Needs Docker; nothing else. `run.sh` builds the image (only
  rhow's source is baked in; the product list and expected outputs are mounted), runs the
  products you name or all of them, writes `tests/integration/logs/` and exits non-zero on
  any problem:

  ```bash
  tests/integration/run.sh                          # all products (~10 min, one clone at a time)
  tests/integration/run.sh pnpm nx                  # only these stacks
  tests/integration/run.sh TryGhost/Ghost           # one product
  UPDATE_EXPECTED=1 tests/integration/run.sh TryGhost/Ghost   # accept its new default view
  ```

  `logs/summary.md` has one row per product (checkout size, complexity score, projects,
  visible/all actions, `rhow` wall-clock, warnings) plus a speed section; each product's
  directory holds `default.txt`, `all.txt`, `ci.txt`, `model.json`, `timing.txt` and, on a
  mismatch, `default.diff`. To move a product to a newer commit, change its sha in
  `repos.txt` and re-record its expected output with `UPDATE_EXPECTED=1`. The installer
  tests in `tests/install/run.sh` also use Docker.

## Non-goals

Not a task runner: rhow never executes an action, forwards arguments or manages environment
variables. No LLM explanations, telemetry, cloud accounts, remote execution, plugins,
favourites, history, configuration UI or full-screen dashboard. No `.rhow.yml` required, ever.

## License

MIT

# Sourced by demo.tape before recording. Builds a small monorepo with CI from two fixtures in a
# temporary directory, points PATH at the release binary and cds into it. Nothing here is shown.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
demo="${TMPDIR:-/tmp}/rhow-demo/storefront"
rm -rf "$demo" && mkdir -p "$demo"
cp -R "$root/fixtures/pnpm-monorepo/." "$demo/"
mkdir -p "$demo/.github/workflows"
cat > "$demo/.github/workflows/ci.yml" <<'YML'
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - run: pnpm install --frozen-lockfile
      - run: pnpm run lint
      - run: pnpm run test
      - run: pnpm run build
  deploy:
    name: Deploy
    needs: test
    runs-on: ubuntu-latest
    steps:
      - run: npx wrangler deploy
YML
git -C "$demo" init -q
export PATH="$root/target/release:$PATH"
export PS1='\[\e[38;5;215m\]$\[\e[0m\] '
cd "$demo" && clear

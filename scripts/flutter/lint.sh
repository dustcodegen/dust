#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# Read from the root pubspec workspace, so adding a package is one edit.
TARGETS=()
while IFS= read -r package; do
  TARGETS+=("$package")
done < <("$ROOT_DIR/scripts/workspace_packages.sh" flutter)

if [[ "${1:-}" == "--examples" ]]; then
  TARGETS=(
    "examples/benchmark_project"
    "examples/shopping_app"
  )
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--examples]" >&2
  exit 2
fi

for target in "${TARGETS[@]}"; do
  echo "==> Flutter pub get: $target"
  (cd "$target" && flutter pub get >/dev/null)
done

if [[ "${1:-}" == "--examples" ]]; then
  echo "==> Generate benchmark sources: examples/benchmark_project"
  (cd examples/benchmark_project && dart run tool/generate.dart --count 64)
  echo "==> Dust build --clean: examples/benchmark_project"
  cargo run --quiet -p dust_cli -- build --clean --root examples/benchmark_project
  echo "==> Dust check: examples/benchmark_project"
  cargo run --quiet -p dust_cli -- check --root examples/benchmark_project
  echo "==> Dust build --clean: examples/shopping_app"
  cargo run --quiet -p dust_cli -- build --clean --root examples/shopping_app
  echo "==> Dust db build --offline: examples/shopping_app"
  cargo run --quiet -p dust_cli -- db build --offline --root examples/shopping_app
  echo "==> Dust check: examples/shopping_app"
  cargo run --quiet -p dust_cli -- check --root examples/shopping_app
  echo "==> Dust check --db --offline: examples/shopping_app"
  cargo run --quiet -p dust_cli -- check --db --offline --root examples/shopping_app
fi

for target in "${TARGETS[@]}"; do
  echo "==> Flutter analyze: $target"
  (cd "$target" && flutter analyze --no-pub)
done

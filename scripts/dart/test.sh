#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# Read from the root pubspec workspace, so adding a package is one edit.
TARGETS=()
while IFS= read -r package; do
  TARGETS+=("$package")
done < <("$ROOT_DIR/scripts/workspace_packages.sh" dart)
# Independent Pub resolution is intentional; keep this consumer outside workspace.
TARGETS+=("fixtures/sqlite_sdk_consumer")
# Workspace discovery lists packages only, so include the executable fixture.
TARGETS+=("fixtures/server_codegen_app")

if [[ "${1:-}" == "--examples" ]]; then
  TARGETS=("examples/product_showcase")
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--examples]" >&2
  exit 2
fi

for target in "${TARGETS[@]}"; do
  echo "==> Dart pub get: $target"
  (cd "$target" && dart pub get)
done

if [[ "${1:-}" != "--examples" ]]; then
  for target in fixtures/server_codegen_app fixtures/server_client_server examples/server_codegen; do
    echo "==> Dust build --clean: $target"
    cargo run --quiet -p dust_cli -- build --clean --root "$target"
    echo "==> Dust check: $target"
    cargo run --quiet -p dust_cli -- check --root "$target"
  done
fi

if [[ "${1:-}" == "--examples" ]]; then
  for example in examples/product_showcase; do
    echo "==> Dust build --clean: $example"
    cargo run --quiet -p dust_cli -- build --clean --root "$example"
    echo "==> Dust check: $example"
    cargo run --quiet -p dust_cli -- check --root "$example"
  done

fi

for target in "${TARGETS[@]}"; do
  echo "==> Dart test: $target"
  (cd "$target" && dart test)
done

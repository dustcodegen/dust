#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

CHECK_MODE=false
# Read from the root pubspec workspace, so adding a package is one edit.
TARGETS=()
while IFS= read -r package; do
  TARGETS+=("$package")
done < <("$ROOT_DIR/scripts/workspace_packages.sh" dart)
# Independent Pub resolution is intentional; keep this consumer outside workspace.
TARGETS+=("fixtures/sqlite_sdk_consumer")
# Workspace discovery lists packages only, so include executable proof fixtures.
TARGETS+=(
  "fixtures/server_codegen_app"
  "fixtures/server_client_server"
  "fixtures/server_client_app"
  "fixtures/server_admin_client_app"
)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) CHECK_MODE=true ;;
    --examples) TARGETS=("examples/product_showcase") ;;
    *)
      echo "Usage: $0 [--check] [--examples]" >&2
      exit 2
      ;;
  esac
  shift
done

for target in "${TARGETS[@]}"; do
  # The independent consumer needs its own language metadata before formatting.
  if [[ "$target" == fixtures/sqlite_sdk_consumer ]]; then
    (cd "$target" && dart pub get)
  fi
  echo "==> Dart format: $target"
  if [[ "$CHECK_MODE" == true ]]; then
    (
      cd "$target"
      find . -name "*.dart" \
        ! -path "*/.dart_tool/*" \
        ! -name "*.g.dart" \
        ! -name "*.freezed.dart" \
        ! -path "*/generated/*" \
        ! -path "*/generated_models/*" \
        -exec dart format --output=none --set-exit-if-changed {} +
    )
  else
    (
      cd "$target"
      find . -name "*.dart" \
        ! -path "*/.dart_tool/*" \
        ! -name "*.g.dart" \
        ! -name "*.freezed.dart" \
        ! -path "*/generated/*" \
        ! -path "*/generated_models/*" \
        -exec dart format {} +
    )
  fi
done

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "==> Rust format"

if [[ "${1:-}" == "--check" ]]; then
  cargo fmt --all -- --check
  cargo fmt --manifest-path vendor/tree-sitter-dart/Cargo.toml -- --check
elif [[ $# -eq 0 ]]; then
  cargo fmt --all
  cargo fmt --manifest-path vendor/tree-sitter-dart/Cargo.toml
else
  echo "Usage: $0 [--check]" >&2
  exit 2
fi

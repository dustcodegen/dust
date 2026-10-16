#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "==> Rust tests"
cargo nextest run --workspace --target-dir target/nextest

echo "==> Pinned listed Dart grammar"
python3 -m unittest discover -s scripts -p 'test_listed_dart_grammar.py'
cargo test --manifest-path vendor/tree-sitter-dart/Cargo.toml --locked
python3 scripts/check_listed_class_parity.py

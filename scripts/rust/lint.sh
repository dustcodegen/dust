#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "==> Rust clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::missing_docs_in_private_items -D missing_docs

echo "==> Pinned listed Dart grammar"
python3 scripts/check_listed_dart_grammar.py
cargo clippy --manifest-path vendor/tree-sitter-dart/Cargo.toml --locked --all-targets -- -D warnings -D clippy::missing_docs_in_private_items -D missing_docs

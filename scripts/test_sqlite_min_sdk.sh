#!/usr/bin/env bash
set -euo pipefail

DUST_ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Preserve the old local entrypoint, with the same tested source consumer.
exec python3 "$DUST_ROOT_DIR/scripts/sqlite_sdk_consumer.py" \
  --dart "$(command -v dart)" "$@"

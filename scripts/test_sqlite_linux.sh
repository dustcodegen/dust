#!/usr/bin/env bash
set -euo pipefail

DUST_ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$DUST_ROOT_DIR/.dart_tool"
DUST_LINUX_EVIDENCE="$(mktemp -d "$DUST_ROOT_DIR/.dart_tool/sqlite-linux.XXXXXX")"
echo "Local Linux evidence: $DUST_LINUX_EVIDENCE"

docker build --platform=linux/arm64 \
  -f "$DUST_ROOT_DIR/scripts/sqlite_linux.Dockerfile" \
  -t dust-sqlite-linux-local:3.12.1 "$DUST_ROOT_DIR/scripts"
docker image inspect dust-sqlite-linux-local:3.12.1 \
  --format '{{json .Id}}' > "$DUST_LINUX_EVIDENCE/image-id.json"
git -C "$DUST_ROOT_DIR" rev-parse HEAD > "$DUST_LINUX_EVIDENCE/source-commit.txt"
docker run --rm --platform=linux/arm64 --cpus=4 --memory=4g --pids-limit=512 \
  --cap-drop=ALL --security-opt=no-new-privileges \
  --mount "type=bind,source=$DUST_ROOT_DIR,target=/source,readonly" \
  --mount "type=bind,source=$DUST_LINUX_EVIDENCE,target=/evidence" \
  dust-sqlite-linux-local:3.12.1 --evidence /evidence/result \
  2>&1 | tee "$DUST_LINUX_EVIDENCE/run.log"
# A second container has no source mount, Pub cache or staging from the first.
docker run --rm --platform=linux/arm64 --network=none --cap-drop=ALL \
  --security-opt=no-new-privileges \
  --mount "type=bind,source=$DUST_LINUX_EVIDENCE,target=/proof,readonly" \
  --entrypoint /proof/result/consumer-bundle/bin/sqlite_sdk_consumer \
  dust-sqlite-linux-local:3.12.1 > "$DUST_LINUX_EVIDENCE/clean-container-proof.json"
PYTHONPATH="$DUST_ROOT_DIR/scripts" python3 - "$DUST_LINUX_EVIDENCE" <<'PY'
import json
import sys
from pathlib import Path
from sqlite_sdk_consumer import compare_proofs
evidence = Path(sys.argv[1])
compare_proofs(
    json.loads((evidence / "result/consumer-proof.json").read_text())["relocatedAot"],
    json.loads((evidence / "clean-container-proof.json").read_text()),
)
PY
echo "Linux gate passed: $DUST_LINUX_EVIDENCE"

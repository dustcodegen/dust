#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if ! cargo audit --version >/dev/null 2>&1; then
  echo "error: cargo-audit 0.22.1 is required" >&2
  echo "Run: cargo install cargo-audit --version 0.22.1 --locked" >&2
  exit 1
fi

# rsa is an inactive optional SQLx lockfile entry and has no patched release.
cargo audit \
  --deny unsound \
  --deny unmaintained \
  --ignore RUSTSEC-2023-0071

rust_report="$(mktemp)"
trap 'rm -f "$rust_report"' EXIT
cargo audit --json --no-fetch --ignore RUSTSEC-2023-0071 >"$rust_report"
python3 - "$rust_report" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    report = json.load(source)

allowed_yanked = {("spin", "0.9.8")}
yanked = {
    (warning["package"]["name"], warning["package"]["version"])
    for warning in report["warnings"].get("yanked", [])
}
unexpected = yanked - allowed_yanked
if unexpected:
    names = ", ".join(f"{name} {version}" for name, version in sorted(unexpected))
    raise SystemExit("unexpected yanked Rust dependencies: " + names)
PY

report="$(mktemp)"
trap 'rm -f "$rust_report" "$report"' EXIT
dart pub outdated --json >"$report"
python3 - "$report" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    packages = json.load(source)["packages"]

unsafe = [
    package["package"]
    for package in packages
    if package["isDiscontinued"]
    or package["isCurrentRetracted"]
    or package["isCurrentAffectedByAdvisory"]
]
if unsafe:
    raise SystemExit("unsafe Dart dependencies: " + ", ".join(sorted(unsafe)))
PY

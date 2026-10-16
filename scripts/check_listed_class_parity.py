#!/usr/bin/env python3
"""Compare complete class facts in separately linked grammar test binaries."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CASES = {"traditional", "modifiers", "metadata", "methods", "factories", "parameters", "operators"}
MARKER = "DUST_CLASS_PARITY:"


def records(output: str) -> dict[str, str]:
    """Reject missing, duplicated or empty facts instead of comparing empty runs."""
    found = {}
    for line in output.splitlines():
        if MARKER not in line:
            continue
        name, separator, facts = line.partition(MARKER)[2].partition(":")
        if not separator or name not in CASES or name in found or not facts.startswith("[ParsedClassSurface {"):
            raise ValueError(f"Invalid class parity record: {name!r}")
        found[name] = facts
    if set(found) != CASES:
        raise ValueError(f"Missing class parity cases: {sorted(CASES - found.keys())}")
    return found


def compare(existing: str, listed: str) -> None:
    """Require exact facts, ordering, defaults, annotations, types and byte spans."""
    baseline, candidate = records(existing), records(listed)
    different = sorted(name for name in CASES if baseline[name] != candidate[name])
    if different:
        raise ValueError(f"Listed class extraction differs: {', '.join(different)}")


def main() -> None:
    """Build each grammar alone; duplicate native exports cannot alias the test."""
    output = ROOT / ".dart_tool/listed-class-parity"
    output.mkdir(parents=True, exist_ok=True)
    logs = []
    for name, selector in (
        ("existing", ["-p", "dust_parser_dart_ts"]),
        ("listed", ["--manifest-path", "crates/dust_parser_dart_ts/parity/Cargo.toml"]),
    ):
        result = subprocess.run(
            ["cargo", "test", *selector, "--locked", "--lib", "classes::parity_tests", "--",
             "--nocapture", "--test-threads=1"],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900,
        )
        log = output / f"{name}.log"
        log.write_text(result.stdout, encoding="utf-8")
        if result.returncode:
            raise RuntimeError(f"{name} class contracts failed; see {log}")
        logs.append(result.stdout)
    compare(*logs)
    print(f"Listed class parity: {len(CASES)} exact contracts; evidence: {output}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Compare complete parser facts in separately linked grammar test binaries."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLASS_CASES = {"traditional", "modifiers", "metadata", "methods", "factories", "parameters", "operators"}
I18N_CASES = {"i18n-static", "i18n-runtime", "i18n-hardcoded", "i18n-ignored", "i18n-shopping"}
CASES = CLASS_CASES | {"directives", "conditional-directives", "uri-part", "unnamed-library",
                       "enums", "declarations", "variables", "functions",
                       "query-terminals", "query-context", "query-unsafe", "query-dynamic"} | I18N_CASES
MARKER = "DUST_PARSER_PARITY:"


def records(output: str) -> dict[str, str]:
    """Reject missing, duplicated or empty facts instead of comparing empty runs."""
    found = {}
    for line in output.splitlines():
        if MARKER not in line:
            continue
        name, separator, facts = line.partition(MARKER)[2].partition(":")
        prefix = ("I18nScanResult {" if name in I18N_CASES else
                  "[ParsedClassSurface {" if name in CLASS_CASES else "ParsedDartFileSurface {")
        if not separator or name not in CASES or name in found or not facts.startswith(prefix):
            raise ValueError(f"Invalid parser parity record: {name!r}")
        found[name] = facts
    if set(found) != CASES:
        raise ValueError(f"Missing parser parity cases: {sorted(CASES - found.keys())}")
    return found


def compare(existing: str, listed: str) -> None:
    """Require exact facts, ordering, defaults, annotations, types and byte spans."""
    baseline, candidate = records(existing), records(listed)
    different = sorted(name for name in CASES if baseline[name] != candidate[name])
    if different:
        raise ValueError(f"Listed parser extraction differs: {', '.join(different)}")


def main() -> None:
    """Build each grammar alone; duplicate native exports cannot alias the test."""
    output = ROOT / ".dart_tool/listed-parser-parity"
    output.mkdir(parents=True, exist_ok=True)
    logs = []
    for name, selector in (
        ("existing", ["-p", "dust_parser_dart_ts"]),
        ("listed", ["--manifest-path", "crates/dust_parser_dart_ts/parity/Cargo.toml"]),
    ):
        result = subprocess.run(
            ["cargo", "test", *selector, "--locked", "--lib", "--",
             "--nocapture", "--test-threads=1"],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900,
        )
        log = output / f"{name}.log"
        log.write_text(result.stdout, encoding="utf-8")
        if result.returncode:
            raise RuntimeError(f"{name} parser contracts failed; see {log}")
        logs.append(result.stdout)
    compare(*logs)
    print(f"Listed parser parity: {len(CASES)} exact contracts; evidence: {output}")


if __name__ == "__main__":
    main()

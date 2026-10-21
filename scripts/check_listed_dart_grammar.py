#!/usr/bin/env python3
"""Verify the exact native grammar bytes and notices before compiling them."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = {
    "LICENSE", "grammar.js", "src/parser.c", "src/scanner.c", "src/node-types.json",
    "src/tree_sitter/alloc.h", "src/tree_sitter/array.h", "src/tree_sitter/parser.h",
    "upstream/Cargo.toml", "upstream/package.json", "upstream/build.rs", "upstream/lib.rs",
}


def check(root: Path) -> list[str]:
    """Reject missing, changed, escaping or incompletely recorded source files."""
    try:
        manifest = json.loads((root / "UPSTREAM.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError) as error:
        return [f"Unable to read UPSTREAM.json: {error}"]
    if not isinstance(manifest, dict):
        return ["UPSTREAM.json must contain an object"]
    if (manifest.get("repository") != "https://github.com/dustcodegen/tree-sitter-dart"
            or not re.fullmatch(r"[0-9a-f]{40}", str(manifest.get("revision", "")))
            or manifest.get("abi") != 15):
        return ["Expected the Dust Dart fork, an immutable revision and ABI 15"]
    files = manifest.get("files")
    if not isinstance(files, dict) or set(files) != FILES:
        return ["The complete pinned source and notice file set must be recorded"]
    problems = []
    for relative, expected in sorted(files.items()):
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()):
            problems.append(f"{relative}: source escapes the vendored directory")
            continue
        try:
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
        except OSError as error:
            problems.append(f"{relative}: {error}")
            continue
        if actual != expected:
            problems.append(f"{relative}: differs from the recorded upstream bytes")
    return problems


if __name__ == "__main__":
    errors = check(ROOT / "vendor/tree-sitter-dart")
    for error in errors:
        print(f"error: {error}")
    raise SystemExit(bool(errors))

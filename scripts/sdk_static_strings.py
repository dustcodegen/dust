"""Compare Dust's shared literal corpus against real Dart evaluation and imports."""

from __future__ import annotations

import json
from pathlib import Path

from sqlite_sdk_consumer import ROOT


def literal_cases() -> list[tuple[str, str, list[list[int]]]]:
    """The Rust decoder test reads these same exact literals and UTF-16 values."""
    result = []
    for filename in ("values.txt", "utf16_only.txt"):
        records = (ROOT / "fixtures/static_strings" / filename).read_text().rstrip("\n").split("\n---\n")
        for record in records:
            header, literal = record.split("\n", 1)
            name, *variants = header.split("\t")
            result.append((name, literal, [[int(unit, 16) for unit in units.split()] for units in variants]))
    return result


def check_static_strings(run, dart: Path, app: Path, output: Path,
                         env: dict[str, str]) -> dict[str, int]:
    """Keep malformed compiler probes outside the app's baseline analysis surface."""
    if output.exists():
        raise ValueError("Static-string probe requires fresh owned staging")
    output.mkdir(parents=True)
    cases = literal_cases()
    test = app / "test/static_strings_test.dart"
    if test.exists():
        raise ValueError("Static-string probe would overwrite an existing test")
    try:
        for index, (newline, label) in enumerate([("\n", "lf"), ("\r\n", "crlf"), ("\r", "cr")]):
            assertions = [f"  test('{name}', () => expect(({literal}).codeUnits, <int>{units[index] if len(units) > 1 else units[0]}));"
                          for name, literal, units in cases]
            source = "import 'package:test/test.dart';\n\nvoid main() {\n" + "\n".join(assertions) + "\n}\n"
            test.write_bytes(source.replace("\n", newline).encode("utf-8"))
            run([str(dart), "test", "--reporter=expanded", str(test)], app, env=env)
            (output / f"{label}_test.dart").write_bytes(test.read_bytes())
        # An escaped URI must resolve the same real file, not a stand-in string.
        (output / "a.dart").write_text("const marker = 'actual a.dart';\n", encoding="utf-8")
        (output / "main.dart").write_text(
            "import '\\u0061.dart' as unicode;\nimport '\\x61.dart' as hex;\n"
            "void main() {\n  if (unicode.marker != 'actual a.dart' || hex.marker != unicode.marker) {\n"
            "    throw StateError('Wrong escaped URI identity');\n  }\n}\n", encoding="utf-8")
        run([str(dart), "run", str(output / "main.dart")], app, env=env)
        result = {"literals": len(cases), "lineEndings": 3, "escapedImports": 2}
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return result
    finally:
        test.unlink(missing_ok=True)

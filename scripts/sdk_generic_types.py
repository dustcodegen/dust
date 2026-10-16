"""Compile expected-failing calls against freshly generated generic consumers."""

from __future__ import annotations

import json
from pathlib import Path


CASES = {
    "replacement": ("void main() { GenericBox<int>(1).copyWith(value: 'wrong'); }",
                    ("String", "int?", "can't be assigned")),
    "bound": ("void main() { BoundedBox<int>.named(1).copyWith(value: 2.5); }",
              ("double", "int?", "can't be assigned")),
    "result": ("GenericBox<int> wrong() => GenericBox<num>(1).copyWith();\nvoid main() { wrong(); }",
               ("GenericBox<num>", "GenericBox<int>", "can't be returned")),
    "result-binder": ("void main() { ResultNamed<String>('before').copyWith(value: 1); }",
                     ("int", "String?", "can't be assigned")),
    "imported-helper": ("void main() { print(ImportedEnvelope<int>(GenericBox(1)).copyWith.child); }",
                        ("getter 'child'", "isn't defined")),
}


def check_generic_types(run, dart: Path, app: Path, output: Path,
                        package: str, env: dict[str, str]) -> list[str]:
    """Reject unrelated failures and unexpected success; never edit the consumer."""
    output.mkdir(parents=True, exist_ok=True)
    packages = app / ".dart_tool/package_config.json"
    for name, (body, markers) in CASES.items():
        source = output / f"{name}.dart"
        source.write_text(
            f"import 'package:{package}/generic_models.dart';\n"
            f"import 'package:{package}/generic_imported.dart';\n{body}\n",
            encoding="utf-8",
        )
        result = run([str(dart), "compile", "kernel", f"--packages={packages}",
                      str(source), "-o", str(output / f"{name}.dill")], app,
                     expected=254, env=env)
        message = result.stdout + result.stderr
        if result.returncode != 254 or "Error:" not in message or not all(
                marker in message for marker in markers):
            raise ValueError(f"{name}: missing the expected generic type error")
    names = list(CASES)
    (output / "results.json").write_text(json.dumps({
        "package": package, "cases": names,
    }, indent=2) + "\n", encoding="utf-8")
    return names

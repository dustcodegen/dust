"""Compiler-negative contracts for effective inherited public types."""

from pathlib import Path


def check_inheritance_types(run, dart: Path, app: Path, output: Path, env: dict[str, str]) -> list[str]:
    """Reject only the intended compiler errors and restore all source bytes."""
    cases = {
        "scalar": ("concrete", "const Concrete(1).copyWith(amount: 'wrong');", ("can't be assigned", "String", "int?")),
        "nested": ("nested", "const Nested().copyWith(items: <int, List<String?>>{1: ['bad']});", ("can't be assigned", "Map<int", "Map<String")),
        "generic": ("generic", "const GenericChild<int, List<int>>([1]).copyWith(values: <String>['bad']);", ("can't be assigned", "List<String>", "List<int>?")),
        "bound": ("generic", "_$GenericChildCopyWith<Object?, int, List<String>>? value = null;", ("doesn't conform", "List<String>", "List<A>")),
        "validation": ("generic", "_GenericChildValidation._validateValues<int, List<int>>(const GenericChild<int, List<int>>([1]), <String>['bad'], <ValidationError>[]);", ("can't be assigned", "List<String>", "List<int>?")),
        "callback": ("override", "String wrong(String value) => value; OverrideModel.create((value) => value).copyWith(transform: wrong);", ("can't be assigned", "String Function(String)", "int Function(int)")),
        "result": ("generic", "GenericChild<String, List<String>> value = const GenericChild<int, List<int>>([1]).copyWith();", ("can't be assigned", "GenericChild<int", "GenericChild<String")),
    }
    for name, (kind, body, markers) in cases.items():
        source = app / f"lib/inheritance_{kind}.dart"
        original = source.read_bytes()
        try:
            source.write_bytes(original + f"\nvoid main() {{ {body} }}\n".encode())
            result = run([str(dart), "compile", "kernel", str(source), "-o", str(output / f"{name}.dill")],
                         app, expected=254, env=env)
            message = result.stdout + result.stderr
            if result.returncode != 254 or "Error:" not in message or not all(marker in message for marker in markers):
                raise ValueError(f"Missing expected inherited {name} type error")
        finally:
            source.write_bytes(original)
    return list(cases)

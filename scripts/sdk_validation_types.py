"""Compiler rejection proof for generated private generic validation helpers."""

from pathlib import Path


def check_validation_types(run, dart: Path, app: Path, output: Path,
                           env: dict[str, str]) -> list[str]:
    """Only append to the caller-owned probe library; restore its exact source bytes."""
    source = app / "lib/validation_basic.dart"
    original = source.read_bytes()
    cases = {
        "receiver": ("Holder<num>(1), 1", ("Holder<num>", "Holder<int>")),
        "candidate": ("Holder<int>(1), 'wrong'", ("String", "int?")),
    }
    try:
        for name, (arguments, markers) in cases.items():
            source.write_bytes(original + (
                "\nvoid main() { _HolderValidation._validateValue<int>("
                f"{arguments}, <ValidationError>[]); }}\n").encode())
            result = run([str(dart), "compile", "kernel", str(source), "-o", str(output / f"{name}.dill")],
                         app, expected=254, env=env)
            message = result.stdout + result.stderr
            if result.returncode != 254 or "Error:" not in message or "can't be assigned" not in message or not all(
                    marker in message for marker in markers):
                raise ValueError(f"Missing expected generic validation {name} type error")
    finally:
        source.write_bytes(original)
    return list(cases)

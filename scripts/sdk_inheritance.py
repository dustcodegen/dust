"""Fresh inherited field contracts on minimum/latest and primary SDK consumers."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from sdk_codegen_fixtures import generated_hashes, generated_library_coverage
from sdk_generic_analysis import check_generic_analysis
from sdk_inheritance_types import check_inheritance_types
from sqlite_sdk_consumer import ROOT

KINDS = ("concrete", "nested", "generic", "override")
STEMS = tuple(f"inheritance_{kind}" for kind in KINDS)


def require_override_infos(message: str) -> list[str]:
    """Record only intentional explicit-override style infos, never warnings."""
    infos = []
    for line in message.splitlines():
        if not line.strip():
            continue
        fields = line.split("|", 7)
        if (len(fields) != 8 or fields[:2] != ["INFO", "LINT"]
                or fields[2] not in {"OVERRIDDEN_FIELDS", "USE_SUPER_PARAMETERS"}
                or not fields[3].replace("\\", "/").endswith("/inheritance_override.dart")):
            raise ValueError(f"Unexpected inheritance-consumer diagnostic: {line}")
        infos.append(line)
    return infos


def require_inheritance_parity(rows: list[dict], primary: dict[str, dict]) -> None:
    """Neither constructor spelling nor SDK choice may change effective field types."""
    for target in ("dart", "flutter"):
        outputs = [row["genericInheritance"]["generated"] for row in rows if row["target"] == target]
        if (len(outputs) != 2 or not outputs[0] or outputs[0] != outputs[1]
                or outputs[0] != primary[target]["generated"]):
            raise ValueError(f"Missing or different traditional/primary {target} inheritance output")


def check_generic_inheritance(run, dust: Path, dart: Path, app: Path, output: Path,
                              options: Path, package: str, env: dict[str, str], *,
                              primary: bool = False, flutter: list[str] | None = None) -> dict[str, object]:
    """Preserve source/baseline bytes and fail closed on every staged proof gate."""
    output.mkdir(parents=True, exist_ok=True)
    sources = [app / f"lib/{stem}.dart" for stem in STEMS]
    generated = [path.with_suffix(".g.dart") for path in sources]
    tests = [app / f"test/{stem}_test.dart" for stem in STEMS]
    if flutter:
        tests.append(app / "test/inheritance_form_test.dart")
    owned = [*sources, *generated, *tests]
    if any(path.exists() for path in owned):
        raise ValueError("Generic inheritance requires fresh owned staging filenames")
    (output / "results.json").unlink(missing_ok=True)
    baseline = generated_hashes(app)
    fixtures = ROOT / "fixtures/generic_inheritance"
    style = "primary" if primary else "traditional"
    infos = []

    def analyze(command, cwd, env=None):
        result = run([str(dart), "analyze", "--format=machine", *command[3:], *map(str, tests)], cwd, env=env)
        infos.extend(require_override_infos(result.stdout + result.stderr))

    try:
        for source, test, kind in zip(sources, tests, KINDS):
            source.write_bytes((fixtures / f"{kind}_{style}.dart.snapshot").read_bytes())
            content = (fixtures / f"{kind}_test.dart.snapshot").read_text().replace(
                "package:inheritance_probe/", f"package:{package}/")
            if flutter:
                content = content.replace("package:test/test.dart", "package:flutter_test/flutter_test.dart")
            test.write_text(content, encoding="utf-8")
        if flutter:
            tests[-1].write_text((fixtures / "form_test.dart.snapshot").read_text().replace(
                "package:inheritance_probe/", f"package:{package}/"), encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        snapshots = {path: path.read_bytes() for path in generated}
        for path in owned:
            (output / path.name).write_bytes(path.read_bytes())
        # Keep legacy non-generic JSON lint headers, not any type-check exceptions.
        check_generic_analysis(analyze, dart, app, options, "lib", env, stems=STEMS,
                               legacy_lint_stems=("inheritance_concrete", "inheritance_nested", "inheritance_override"))
        test_paths = [path.relative_to(app).as_posix() for path in tests]
        report = output / "generated.lcov"
        try:
            if flutter:
                for path, source in snapshots.items():
                    path.write_bytes(source.replace(b"// coverage:ignore-file\n", b""))
                run([*flutter, "test", "--no-pub", "--coverage", f"--coverage-path={report}",
                     "--reporter=expanded", *test_paths], app, env=env)
            else:
                run([str(dart), "test", f"--coverage={output / 'coverage'}", "--reporter=expanded",
                     *test_paths], app, env=env)
                run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov",
                     "-i", str(output / "coverage"), "-o", str(report), "--report-on=lib"], app, env=env)
        finally:
            for path, source in snapshots.items():
                path.write_bytes(source)
        coverage = generated_library_coverage(report, STEMS)
        negatives = check_inheritance_types(run, dart, app, output, env)
        for path, source in snapshots.items():
            stem = path.stem.removesuffix('.g')
            kind = "flutter_" if flutter and stem == "inheritance_concrete" else ""
            expected = (fixtures / f"{stem}_{kind}generated.dart.snapshot").read_bytes()
            if source != expected:
                raise ValueError(f"Inheritance differs from the complete contract: {path.name}")
        current = generated_hashes(app)
        if any(current.get(name) != digest for name, digest in baseline.items()):
            raise ValueError("Generic inheritance changed baseline generated output")
        result = {"generated": {path.name: hashlib.sha256(source).hexdigest() for path, source in snapshots.items()},
                  "generatedLineCoverage": coverage, "typeErrors": negatives, "overrideStyleInfos": infos}
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return result
    finally:
        for path in owned:
            path.unlink(missing_ok=True)

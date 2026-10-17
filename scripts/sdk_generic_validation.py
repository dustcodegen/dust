"""Typed generic validation proof using fresh, owned SDK consumer files."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from sdk_codegen_fixtures import generated_hashes
from sdk_generic_analysis import check_generic_analysis
from sdk_generic_names import require_only_name_infos
from sdk_validation_types import check_validation_types
from sqlite_sdk_consumer import ROOT

STEMS = ("validation_basic", "validation_shapes", "validation_scope")


def require_validation_parity(rows: list[dict], primary: dict[str, dict]) -> None:
    """SDK choice and constructor spelling cannot change the same validation output."""
    for target in ("dart", "flutter"):
        outputs = [row["genericValidation"]["generated"] for row in rows if row["target"] == target]
        if (len(outputs) != 2 or not outputs[0] or outputs[0] != outputs[1]
                or outputs[0] != primary[target]["generated"]):
            raise ValueError(f"Missing or different traditional/primary {target} validation output")


def validation_coverage(report: Path) -> dict[str, dict[str, int]]:
    """Require each owned generated library, including its private helpers, to execute."""
    result = {}
    for record in report.read_text().split("end_of_record"):
        lines = record.splitlines()
        names = [stem for stem in STEMS if any(
            line.startswith("SF:") and line.endswith(f"/{stem}.g.dart") for line in lines)]
        if not names:
            continue
        name, = names
        hits = [int(line.split(",")[1]) for line in lines if line.startswith("DA:")]
        if name in result or not hits or any(hit <= 0 for hit in hits):
            raise ValueError(f"Incomplete or duplicate generic validation coverage: {name}")
        result[name] = {"hit": len(hits), "found": len(hits)}
    if set(result) != set(STEMS):
        raise ValueError("Missing generic validation generated coverage")
    return result


def check_generic_validation(run, dust: Path, dart: Path, app: Path, output: Path,
                             options: Path, package: str, env: dict[str, str], *,
                             primary: bool = False, flutter: list[str] | None = None) -> dict[str, object]:
    """Preserve baseline output; never analyze or change a source checkout in place."""
    output.mkdir(parents=True, exist_ok=True)
    sources = [app / "lib" / f"{stem}.dart" for stem in STEMS]
    generated = [app / "lib" / f"{stem}.g.dart" for stem in STEMS]
    tests = [app / "test/validation_runtime_test.dart"]
    if flutter:
        tests.append(app / "test/validation_form_test.dart")
    owned = [*sources, *generated, *tests]
    if any(path.exists() for path in owned):
        raise ValueError("Generic validation requires fresh owned staging filenames")
    baseline = generated_hashes(app)
    fixtures = ROOT / "fixtures/generic_validation"
    style = "primary" if primary else "traditional"
    fixture_names = (style, f"shapes_{style}", f"scope_{style}")
    infos = []

    def analyze(command, cwd, env=None):
        result = run([str(dart), "analyze", "--format=machine", *command[3:], *map(str, tests)], cwd, env=env)
        infos.extend(require_only_name_infos(result.stdout + result.stderr))

    try:
        for source, name in zip(sources, fixture_names):
            source.write_text((fixtures / f"{name}.dart.snapshot").read_text().replace(
                "part 'model.g.dart';", "part 'validation_basic.g.dart';"), encoding="utf-8")
        for test, name in zip(tests, ("runtime", "form")):
            source = (fixtures / f"{name}_test.dart.snapshot").read_text().replace(
                "package:validation_probe/", f"package:{package}/")
            if flutter:
                source = source.replace("package:test/test.dart", "package:flutter_test/flutter_test.dart")
            test.write_text(source, encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        snapshots = {path: path.read_bytes() for path in generated}
        expected = (fixtures / "generated.dart.snapshot").read_text().replace(
            "part of 'model.dart';", "part of 'validation_basic.dart';")
        # Flutter adds input helpers only for concrete supported input types, not T.
        if snapshots[generated[0]].decode() != expected:
            raise ValueError("Generic validation differs from the complete basic contract")
        check_generic_analysis(analyze, dart, app, options, "lib", env, stems=STEMS)
        report = output / "generated.lcov"
        test_paths = [path.relative_to(app).as_posix() for path in tests]
        try:
            if flutter:
                # Flutter's collector honors this marker; remove it only in owned staging.
                for path, source in snapshots.items():
                    path.write_bytes(source.replace(b"// coverage:ignore-file\n", b""))
                run([*flutter, "test", "--no-pub", "--coverage", f"--coverage-path={report}",
                     "--reporter=expanded", *test_paths], app, env=env)
            else:
                run([str(dart), "test", f"--coverage={output / 'coverage'}", "--reporter=expanded",
                     *test_paths], app, env=env)
                run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov",
                     "-i", str(output / "coverage"), "-o", str(report),
                     "--report-on=lib"], app, env=env)
        finally:
            for path, source in snapshots.items():
                path.write_bytes(source)
        for path in [*sources, *generated, *tests]:
            (output / path.name).write_bytes(path.read_bytes())
        coverage = validation_coverage(report)
        negatives = check_validation_types(run, dart, app, output, env)
        current = generated_hashes(app)
        if any(current.get(name) != digest for name, digest in baseline.items()):
            raise ValueError("Generic validation changed baseline generated output")
        result = {"generated": {path.name: hashlib.sha256(source).hexdigest()
                                for path, source in snapshots.items()},
                  "nameStyleInfos": infos, "generatedLineCoverage": coverage, "typeErrors": negatives}
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return result
    finally:
        for path in owned:
            path.unlink(missing_ok=True)

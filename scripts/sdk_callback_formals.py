"""Execute fresh callback copies without treating expected style infos as success."""

from __future__ import annotations

import hashlib
from collections import Counter
from pathlib import Path

from sdk_codegen_fixtures import generated_library_coverage
from sqlite_sdk_consumer import ROOT


def check_callback_formals(run, dust: Path, dart: Path, app: Path, output: Path,
                           options: Path, package: str, env: dict[str, str], *, primary: bool = False,
                           flutter: list[str] | None = None) -> dict[str, object]:
    """Stage only new probe files and restore analyzer configuration on every exit."""
    source = app / "lib/callback_probe.dart"
    generated = app / "lib/callback_probe.g.dart"
    test = app / "test/callback_probe_test.dart"
    config = app / "analysis_options.yaml"
    if output.exists() or any(path.exists() for path in (source, generated, test)):
        raise ValueError("Callback probe requires fresh owned staging filenames")
    original_config = config.read_bytes()
    output.mkdir(parents=True)
    fixtures = ROOT / "fixtures/primary_callback_formals"
    try:
        source.write_bytes((fixtures / f"generated_{'primary' if primary else 'traditional'}.dart.snapshot").read_bytes())
        test.write_text((fixtures / "runtime_test.dart.snapshot").read_text().replace(
            "package:callback_probe/", f"package:{package}/").replace(
            "package:test/test.dart", "package:flutter_test/flutter_test.dart" if flutter else "package:test/test.dart"), encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        snapshot = generated.read_bytes()
        analysis = options.read_text().replace('    - "**/*.g.dart"\n', "")
        for rule in ("strict_raw_type", "inference_failure_on_function_invocation", "inference_failure_on_instance_creation"):
            analysis = analysis.replace(f"    {rule}: ignore\n", "")
        config.write_text(analysis.replace("analyzer:\n", "analyzer:\n  language:\n"
                          "    strict-casts: true\n    strict-inference: true\n"
                          "    strict-raw-types: true\n"), encoding="utf-8")
        generated.write_text(snapshot.decode().replace("// ignore_for_file: type=lint\n", ""), encoding="utf-8")
        analysis = run([str(dart), "analyze", "--format=machine", str(source), str(generated), str(test)], app, env=env)
        infos = expected_style_infos(analysis.stdout + analysis.stderr, source, primary=primary)
        # Flutter honors the generated coverage marker; strip it only in staging.
        generated.write_bytes(snapshot.replace(b"// coverage:ignore-file\n", b"") if flutter else snapshot)
        runner = [*flutter, "test", "--no-pub", "--coverage"] if flutter else [str(dart), "test"]
        coverage_arg = f"--coverage-path={output / 'generated.lcov'}" if flutter else f"--coverage={output / 'coverage'}"
        run([*runner, coverage_arg, "--reporter=expanded", "test/callback_probe_test.dart"], app, env=env)
        report = output / "generated.lcov"
        if not flutter:
            run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov",
                 "-i", str(output / "coverage"), "-o", str(report),
                 "--report-on=lib/callback_probe.g.dart"], app, env=env)
        coverage = generated_library_coverage(report, ("callback_probe",))
        generated.write_bytes(snapshot)
        for path in (source, generated, test):
            (output / path.name).write_bytes(path.read_bytes())
        return {"generated": hashlib.sha256(snapshot).hexdigest(),
                "generatedLineCoverage": coverage, "expectedStyleInfos": infos}
    finally:
        config.write_bytes(original_config)
        for path in (source, generated, test):
            path.unlink(missing_ok=True)


def expected_style_infos(output: str, source: Path, *, primary: bool) -> int:
    """No suppressed diagnostics, errors, warnings or unrelated infos are allowed."""
    lines = output.splitlines()
    allowed = {"USE_FUNCTION_TYPE_SYNTAX_FOR_PARAMETERS", "PREFER_INITIALIZING_FORMALS"}
    if not lines or any(len(parts := line.split("|")) < 8 or parts[:2] != ["INFO", "LINT"]
                        or parts[2] not in allowed or Path(parts[3]).resolve() != source.resolve()
                        for line in lines):
        raise ValueError(f"Unexpected callback analyzer diagnostics: {output}")
    counts = Counter(line.split("|")[2] for line in lines)
    expected = {"USE_FUNCTION_TYPE_SYNTAX_FOR_PARAMETERS": 7}
    if not primary:
        expected["PREFER_INITIALIZING_FORMALS"] = 6
    if counts != expected:
        raise ValueError(f"Missing or extra callback style infos: {counts}")
    return len(lines)

"""Real compiler/runtime checks of generated models with lexical metadata shadows."""

from __future__ import annotations

import hashlib
from pathlib import Path

from sdk_codegen_fixtures import generated_library_coverage
from sdk_generic_analysis import check_generic_analysis
from sqlite_sdk_consumer import ROOT


def check_lexical_scopes(run, dust: Path, dart: Path, app: Path, output: Path,
                         options: Path, package: str, env: dict[str, str], *,
                         primary: bool = False) -> dict[str, object]:
    """Add only owned staging files; restore the consumer even when a check fails."""
    source = app / "lib/lexical_probe.dart"
    generated = app / "lib/lexical_probe.g.dart"
    test = app / "test/lexical_probe_test.dart"
    if any(path.exists() for path in (source, generated, test)):
        raise ValueError("Lexical probe requires fresh owned staging filenames")
    output.mkdir(parents=True, exist_ok=True)
    fixtures = ROOT / "fixtures/namespace_lexical_shadowing"
    try:
        source.write_bytes((fixtures / f"generated_{'primary' if primary else 'traditional'}.dart.snapshot").read_bytes())
        test.write_text((fixtures / "runtime_test.dart.snapshot").read_text().replace(
            "package:lexical_probe/", f"package:{package}/"), encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        check_generic_analysis(run, dart, app, options, "lib", env, stems=("lexical_probe",))
        run([str(dart), "test", f"--coverage={output / 'coverage'}", "--reporter=expanded",
             "test/lexical_probe_test.dart"], app, env=env)
        report = output / "generated.lcov"
        run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov",
             "-i", str(output / "coverage"), "-o", str(report),
             "--report-on=lib/lexical_probe.g.dart"], app, env=env)
        coverage = generated_library_coverage(report, ("lexical_probe",))
        snapshot = generated.read_bytes()
        if snapshot != (fixtures / "generated.dart.snapshot").read_bytes():
            raise ValueError("Generated lexical output differs from the complete contract")
        for path in (source, generated, test):
            (output / path.name).write_bytes(path.read_bytes())
        return {"generated": hashlib.sha256(snapshot).hexdigest(),
                "generatedLineCoverage": coverage}
    finally:
        for path in (source, generated, test):
            path.unlink(missing_ok=True)

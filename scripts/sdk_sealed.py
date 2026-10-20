"""Fresh generic sealed contracts on minimum/latest and primary SDK consumers."""

from __future__ import annotations

import hashlib
import json
import tempfile
from pathlib import Path

from sdk_codegen_fixtures import generated_hashes, generated_library_coverage
from sdk_generic_analysis import check_generic_analysis
from sdk_generic_names import require_only_name_infos
from sdk_sealed_types import check_sealed_types
from sqlite_sdk_consumer import ROOT

KINDS = ("declared", "generated", "modes", "scope", "plain")
STEMS = tuple(f"sealed_{kind}" for kind in KINDS)


def require_sealed_parity(rows: list[dict], primary: dict[str, dict]) -> None:
    """Require all SDK/header/package combinations, not merely two passing probes."""
    outputs = []
    for target in ("dart", "flutter"):
        matches = [row["genericSealed"]["generated"] for row in rows if row["target"] == target]
        if len(matches) != 2:
            raise ValueError(f"Missing sealed {target} SDK targets")
        outputs.extend([*matches, primary[target]["generated"]])
    if not outputs[0] or any(output != outputs[0] for output in outputs[1:]):
        raise ValueError("Different or empty sealed generated output")


def require_sealed_infos(message: str) -> list[str]:
    """Record intentional source-binder styles only in the collision fixture."""
    infos = require_only_name_infos(message)
    for line in infos:
        path = line.split("|", 7)[3].replace("\\", "/")
        if not path.endswith(("/sealed_scope.dart", "/sealed_scope.g.dart")):
            raise ValueError(f"Unexpected sealed diagnostic location: {line}")
    return infos


def check_generic_sealed(run, dust: Path, dart: Path, app: Path, output: Path,
                         options: Path, package: str, env: dict[str, str], *,
                         primary: bool = False, flutter: list[str] | None = None) -> dict[str, object]:
    """Preserve caller/baseline bytes and publish proof only after every gate passes."""
    output.mkdir(parents=True, exist_ok=True)
    sources = [app / f"lib/{stem}.dart" for stem in STEMS]
    generated = [path.with_suffix(".g.dart") for path in sources]
    tests = [app / f"test/{stem}_test.dart" for stem in STEMS]
    owned = [*sources, *generated, *tests]
    if any(path.exists() for path in owned):
        raise ValueError("Generic sealed proof requires fresh owned staging filenames")
    (output / "results.json").unlink(missing_ok=True)
    baseline = generated_hashes(app)
    fixtures = ROOT / "fixtures/generic_sealed"
    style = "primary" if primary else "traditional"
    infos = []

    def analyze(command, cwd, env=None):
        result = run([str(dart), "analyze", "--format=machine", *command[3:], *map(str, tests)], cwd, env=env)
        infos.extend(require_sealed_infos(result.stdout + result.stderr))

    try:
        for path, kind in zip(sources, KINDS):
            path.write_bytes((fixtures / f"{kind}_{style}.dart.snapshot").read_bytes())
        for test, kind in zip(tests, KINDS):
            content = (fixtures / f"{kind}_test.dart.snapshot").read_text().replace("package:sealed_probe/", f"package:{package}/")
            if flutter:
                content = content.replace("package:test/test.dart", "package:flutter_test/flutter_test.dart")
            test.write_text(content, encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        snapshots = {path: path.read_bytes() for path in generated}
        for path in owned:
            (output / path.name).write_bytes(path.read_bytes())
        check_generic_analysis(analyze, dart, app, options, "lib", env, stems=STEMS, legacy_lint_stems=("sealed_plain",))
        report = output / "generated.lcov"
        test_paths = [path.relative_to(app).as_posix() for path in tests]
        try:
            if flutter:
                for path, source in snapshots.items():
                    path.write_bytes(source.replace(b"// coverage:ignore-file\n", b""))
                run([*flutter, "test", "--no-pub", "--coverage", f"--coverage-path={report}", "--reporter=expanded", *test_paths], app, env=env)
            else:
                coverage_root = Path(tempfile.mkdtemp(prefix="coverage-", dir=output))
                run([str(dart), "test", f"--coverage={coverage_root}", "--reporter=expanded", *test_paths], app, env=env)
                run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov", "-i", str(coverage_root), "-o", str(report), "--report-on=lib"], app, env=env)
        finally:
            for path, source in snapshots.items():
                path.write_bytes(source)
        coverage = generated_library_coverage(report, STEMS)
        negatives = check_sealed_types(run, dart, app, output, env)
        for path, source in snapshots.items():
            expected = (fixtures / f"{path.stem.removesuffix('.g')}_generated.dart.snapshot").read_bytes()
            if source != expected:
                raise ValueError(f"Sealed output differs from the complete contract: {path.name}")
        current = generated_hashes(app)
        if any(current.get(name) != digest for name, digest in baseline.items()):
            raise ValueError("Sealed proof changed baseline generated output")
        result = {"generated": {path.name: hashlib.sha256(source).hexdigest() for path, source in snapshots.items()},
                  "generatedLineCoverage": coverage, "typeErrors": negatives, "nameStyleInfos": infos}
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return result
    finally:
        for path in owned:
            path.unlink(missing_ok=True)

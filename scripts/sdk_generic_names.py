"""Compiler/runtime proof for legal but intentionally nonrecommended Dart binders."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from sdk_generic_analysis import check_generic_analysis
from sqlite_sdk_consumer import ROOT


def require_generated_coverage(report: Path) -> dict[str, int]:
    """Check the single generated-file scope, without honoring coverage-ignore comments."""
    records = report.read_text().split("end_of_record")
    selected = [record for record in records if any(
        line.startswith("SF:") and line.endswith("/generic_names.g.dart")
        for line in record.splitlines())]
    if len(selected) != 1:
        raise ValueError("Missing or duplicate generated collision coverage")
    hits = [int(line.split(",")[1]) for line in selected[0].splitlines() if line.startswith("DA:")]
    if not hits or any(hit <= 0 for hit in hits):
        raise ValueError("Generated collision paths are not fully covered")
    return {"hit": sum(hit > 0 for hit in hits), "found": len(hits)}


def require_only_name_infos(message: str) -> list[str]:
    """No warning, error, unrelated lint or arbitrary tool output may count as success."""
    infos = []
    for line in message.splitlines():
        if not line.strip():
            continue
        fields = line.split("|", 7)
        if (len(fields) != 8 or fields[:2] != ["INFO", "LINT"] or fields[2] not in
                {"AVOID_TYPES_AS_PARAMETER_NAMES", "CAMEL_CASE_TYPES"}):
            raise ValueError(f"Unexpected collision-consumer diagnostic: {line}")
        infos.append(line)
    return infos


def check_member_binder_rejection(run, dust: Path, dart: Path, app: Path,
                                  env: dict[str, str], *, primary: bool) -> list[str]:
    """Prove Dart accepts the binders before Dust rejects conflicting generation."""
    source = app / "lib/member_binders.dart"
    generated = app / "lib/member_binders.g.dart"
    if source.exists() or generated.exists():
        raise ValueError("Member-binder probe requires fresh owned staging filenames")
    parameters = "copyWith, toString, hashCode"
    plain = f"""/// Legal Dart member-named binder probe.
class Probe<{parameters}> {{
  /// Creates a probe.
  const Probe(this.value);
  /// Chosen value.
  final copyWith value;
}}
"""
    if primary:
        declaration = (
            f"class const Probe<{parameters}>(final copyWith value) "
            f"with _$Probe<{parameters}> {{ this; }}")
    else:
        declaration = (
            f"class Probe<{parameters}> with _$Probe<{parameters}> {{\n"
            "  const Probe(this.value);\n  final copyWith value;\n}")
    annotated = (
        "import 'package:dust_dart/derive.dart' as d;\n"
        "part 'member_binders.g.dart';\n"
        "@d.Derive([d.CopyWith(), d.ToString(), d.Eq()])\n"
        f"{declaration}\n")
    try:
        source.write_text(plain, encoding="utf-8")
        accepted = run([str(dart), "analyze", "--format=machine", "lib/member_binders.dart"],
                       app, env=env)
        require_only_name_infos(accepted.stdout + accepted.stderr)
        source.write_text(annotated, encoding="utf-8")
        for command in ("build", "check"):
            result = run([str(dust), command, "--root", str(app)], app, expected=1, env=env)
            output = result.stdout + result.stderr
            for member in ("copyWith", "toString", "hashCode"):
                if f"generic parameter `{member}`" not in output:
                    raise ValueError(f"Missing member-binder diagnostic for {member}")
            if generated.exists():
                raise ValueError("Rejected member binders wrote generated output")
        return ["copyWith", "toString", "hashCode"]
    finally:
        source.unlink(missing_ok=True)
        generated.unlink(missing_ok=True)


def check_generic_names(run, dust: Path, dart: Path, app: Path, output: Path,
                        options: Path, package: str, env: dict[str, str], *,
                        primary: bool = False) -> dict[str, object]:
    """Use owned staging only; keep baseline generated files and options byte-identical."""
    output.mkdir(parents=True, exist_ok=True)
    source = app / "lib/generic_names.dart"
    generated = app / "lib/generic_names.g.dart"
    test = app / "test/generic_names_test.dart"
    member_source = app / "lib/member_binders.dart"
    member_generated = app / "lib/member_binders.g.dart"
    if any(path.exists() for path in (source, generated, test, member_source, member_generated)):
        raise ValueError("Collision probe requires fresh owned staging filenames")
    fixtures = ROOT / "fixtures/generic_names"
    infos = []

    def analyze(command, cwd, env=None):
        result = run([str(dart), "analyze", "--format=machine", *command[3:]], cwd, env=env)
        infos.extend(require_only_name_infos(result.stdout + result.stderr))

    try:
        source.write_bytes((fixtures / f"{'primary' if primary else 'traditional'}.dart.snapshot").read_bytes())
        test.write_text((fixtures / "runtime_test.dart.snapshot").read_text().replace(
            "package:generic_names_probe/", f"package:{package}/"), encoding="utf-8")
        run([str(dust), "build", "--root", str(app)], app, env=env)
        run([str(dust), "check", "--root", str(app)], app, env=env)
        check_generic_analysis(analyze, dart, app, options, "lib", env, stems=("generic_names",))
        run([str(dart), "test", f"--coverage={output / 'coverage'}", "--reporter=expanded",
             "test/generic_names_test.dart"], app, env=env)
        report = output / "generated.lcov"
        run([str(dart), "run", "coverage:format_coverage", "--package=.", "--lcov",
             "-i", str(output / "coverage"), "-o", str(report),
             "--report-on=lib/generic_names.g.dart"], app, env=env)
        coverage = require_generated_coverage(report)
        snapshot = generated.read_bytes()
        if snapshot != (fixtures / "generated.dart.snapshot").read_bytes():
            raise ValueError("Generated collision output differs from the complete contract")
        rejected = check_member_binder_rejection(
            run, dust, dart, app, env, primary=primary)
        (output / "generic_names.dart").write_bytes(source.read_bytes())
        (output / "generic_names.g.dart").write_bytes(snapshot)
        (output / "generic_names_test.dart").write_bytes(test.read_bytes())
        result = {"generated": hashlib.sha256(snapshot).hexdigest(), "nameStyleInfos": infos,
                  "rejectedMemberBinders": rejected,
                  "generatedLineCoverage": coverage}
        (output / "results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return result
    finally:
        for path in (source, generated, test):
            path.unlink(missing_ok=True)

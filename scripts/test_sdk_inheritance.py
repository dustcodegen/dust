"""Inheritance probes must fail closed and preserve caller-owned staging."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_inheritance import ROOT, STEMS, check_generic_inheritance, require_inheritance_parity, require_override_infos
from sdk_inheritance_types import check_inheritance_types

ERROR = "Error: can't be assigned doesn't conform String int? Map<int Map<String List<String> List<int>? List<A> String Function(String) int Function(int) GenericChild<int GenericChild<String"
CASES = ["scalar", "nested", "generic", "bound", "validation", "callback", "result"]


class InheritanceSdkTests(unittest.TestCase):
    def test_parity_requires_both_sdk_ends_and_primary_output(self):
        primary = {target: {"generated": {"part": "same"}} for target in ("dart", "flutter")}
        rows = [{"target": target, "genericInheritance": primary[target]}
                for target in ("dart", "flutter") for _ in range(2)]
        require_inheritance_parity(rows, primary)
        for invalid in (rows[:3], [*rows[:3], {"target": "flutter", "genericInheritance": {"generated": {}}}],
                        [{"target": "dart", "genericInheritance": {"generated": {}}}, *rows[1:]],
                        [{"target": "dart", "genericInheritance": {"generated": {"part": "different"}}}, *rows[1:]]):
            with self.assertRaises(ValueError):
                require_inheritance_parity(invalid, primary)
        with self.assertRaises(ValueError):
            require_inheritance_parity(rows, {**primary, "dart": {"generated": {"part": "different"}}})

    def test_only_explicit_override_source_style_infos_are_recorded(self):
        allowed = "INFO|LINT|OVERRIDDEN_FIELDS|/app/lib/inheritance_override.dart|1|1|1|explicit override"
        primary = allowed.replace("OVERRIDDEN_FIELDS", "USE_SUPER_PARAMETERS")
        self.assertEqual(require_override_infos("\n" + allowed + "\n" + primary), [allowed, primary])
        windows = allowed.replace("/app/lib/", "C:\\app\\lib\\")
        self.assertEqual(require_override_infos(windows), [windows])
        for invalid in ("tool failure", allowed.replace("INFO", "WARNING"), allowed.replace("LINT", "ERROR"),
                        allowed.replace("OVERRIDDEN_FIELDS", "OTHER"), allowed.replace("inheritance_override", "other")):
            with self.assertRaises(ValueError):
                require_override_infos(invalid)

    def test_negative_compiler_cases_restore_all_source_bytes(self):
        for failure in (None, "success", "unrelated", "wrong-error", "exception"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                (app / "lib").mkdir()
                originals = {}
                for stem in STEMS:
                    source = app / f"lib/{stem}.dart"
                    source.write_bytes(b"// original\n")
                    originals[source] = source.read_bytes()
                calls = []

                def run(command, cwd, expected, env):
                    self.assertEqual((cwd, expected, env), (app, 254, {"SDK": "selected"}))
                    source = Path(command[3])
                    self.assertEqual(source.read_text().count("void main()"), 1)
                    self.assertTrue(all(path == source or path.read_bytes() == value for path, value in originals.items()))
                    calls.append(command)
                    if failure == "exception":
                        raise RuntimeError("compiler failed")
                    message = "unrelated" if failure == "unrelated" else ("Error: missing import" if failure == "wrong-error" else ERROR)
                    return subprocess.CompletedProcess(command, 0 if failure == "success" else 254, message, "")

                if failure:
                    with self.assertRaises(RuntimeError if failure == "exception" else ValueError):
                        check_inheritance_types(run, Path("dart"), app, app, {"SDK": "selected"})
                else:
                    self.assertEqual(check_inheritance_types(run, Path("dart"), app, app, {"SDK": "selected"}), CASES)
                    self.assertEqual(len(calls), len(CASES))
                self.assertEqual({path: path.read_bytes() for path in originals}, originals)

    def test_success_and_failure_paths_keep_staging_and_analysis_boundaries(self):
        failures = (None, "build", "check", "analyze", "test", "run", "missing-output",
                    "coverage", "snapshot", "baseline", "info", "compiler")
        fixtures = ROOT / "fixtures/generic_inheritance"
        for flutter in (None, ["flutter"]):
            for primary in (False, True):
                for failure in failures:
                    if flutter and failure == "run":
                        continue
                    with self.subTest(flutter=flutter, primary=primary, failure=failure), tempfile.TemporaryDirectory() as directory:
                        root = Path(directory)
                        app = root / "app"
                        (app / "lib").mkdir(parents=True)
                        (app / "test").mkdir()
                        options = root / "options.yaml"
                        options.write_text("analyzer:\n  errors:\n    strict_raw_type: ignore\n")
                        config = app / "analysis_options.yaml"
                        original = b"include: ../options.yaml\n"
                        config.write_bytes(original)
                        baseline = app / "lib/old.g.dart"
                        baseline.write_bytes(b"// unchanged\n")
                        output = root / "proof"
                        output.mkdir()
                        (output / "results.json").write_text('{"stale": true}')

                        def golden(stem):
                            kind = "flutter_" if flutter and stem == "inheritance_concrete" else ""
                            return fixtures / f"{stem}_{kind}generated.dart.snapshot"

                        def report():
                            records = "".join(f"SF:/app/lib/{stem}.g.dart\nDA:1,2\nend_of_record\n" for stem in STEMS)
                            if failure == "coverage":
                                records = records.replace("DA:1,2", "DA:1,0", 1)
                            (output / "generated.lcov").write_text(records)

                        def run(command, cwd, env=None, expected=0):
                            self.assertEqual((cwd, env), (app, {"SDK": "selected"}))
                            self.assertEqual("class const Concrete" in (app / "lib/inheritance_concrete.dart").read_text(), primary)
                            content = (app / "test/inheritance_concrete_test.dart").read_text()
                            self.assertIn("package:selected/inheritance_concrete.dart", content)
                            self.assertEqual("package:flutter_test/flutter_test.dart" in content, bool(flutter))
                            self.assertEqual((app / "test/inheritance_form_test.dart").exists(), bool(flutter))
                            if command[1] == failure:
                                raise RuntimeError("controlled failure")
                            if command[1] == "build":
                                for stem in STEMS:
                                    (app / f"lib/{stem}.g.dart").write_bytes(golden(stem).read_bytes())
                                if failure == "missing-output":
                                    (app / "lib/inheritance_override.g.dart").unlink()
                                if failure == "snapshot":
                                    with (app / "lib/inheritance_override.g.dart").open("a") as changed:
                                        changed.write("// unexpected output\n")
                                if failure == "baseline":
                                    baseline.write_bytes(b"// unexpected baseline change\n")
                            if command[1] == "analyze":
                                self.assertIn("strict-casts: true", config.read_text())
                                for stem in STEMS:
                                    self.assertEqual("// ignore_for_file: type=lint" in (app / f"lib/{stem}.g.dart").read_text(), stem != "inheritance_generic")
                                message = "INFO|LINT|OTHER|/app/lib/inheritance_override.dart|1|1|1|unrelated" if failure == "info" else "INFO|LINT|OVERRIDDEN_FIELDS|/app/lib/inheritance_override.dart|1|1|1|intentional"
                                return subprocess.CompletedProcess(command, 0, message, "")
                            if command[1] == "test":
                                self.assertEqual("--no-pub" in command, bool(flutter))
                                if flutter:
                                    for stem in STEMS:
                                        self.assertNotIn("// coverage:ignore-file", (app / f"lib/{stem}.g.dart").read_text())
                                    report()
                            if command[1] == "run":
                                report()
                            if command[1] == "compile":
                                return subprocess.CompletedProcess(command, expected, "unrelated" if failure == "compiler" else ERROR, "")
                            return subprocess.CompletedProcess(command, 0, "", "")

                        if failure:
                            kind = FileNotFoundError if failure == "missing-output" else (
                                RuntimeError if failure in ("build", "check", "analyze", "test", "run") else ValueError)
                            with self.assertRaises(kind):
                                check_generic_inheritance(run, Path("dust"), Path("dart"), app, output, options,
                                                          "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                        else:
                            result = check_generic_inheritance(run, Path("dust"), Path("dart"), app, output, options,
                                                               "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                            self.assertEqual(result["generatedLineCoverage"], {stem: {"hit": 1, "found": 1} for stem in STEMS})
                            self.assertEqual(result["typeErrors"], CASES)
                            self.assertEqual(json.loads((output / "results.json").read_text()), result)
                            for stem in STEMS:
                                self.assertEqual((output / f"{stem}.g.dart").read_bytes(), golden(stem).read_bytes())
                        self.assertEqual(config.read_bytes(), original)
                        self.assertEqual(sorted(path.name for path in (app / "lib").iterdir()), ["old.g.dart"])
                        self.assertEqual(list((app / "test").iterdir()), [])
                        if failure != "baseline":
                            self.assertEqual(baseline.read_bytes(), b"// unchanged\n")
                        if failure:
                            self.assertFalse((output / "results.json").exists())

    def test_preexisting_input_is_never_overwritten_or_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            (app / "lib").mkdir()
            source = app / "lib/inheritance_concrete.dart"
            source.write_bytes(b"// caller-owned\n")
            with self.assertRaisesRegex(ValueError, "fresh owned"):
                check_generic_inheritance(None, Path("dust"), Path("dart"), app, app / "proof", app / "options", "selected", {})
            self.assertEqual(source.read_bytes(), b"// caller-owned\n")


if __name__ == "__main__":
    unittest.main()

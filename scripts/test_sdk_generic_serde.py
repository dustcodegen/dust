"""SerDe SDK probes must fail closed and preserve caller-owned staging boundaries."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_generic_serde import ROOT, STEMS, check_generic_serde, check_serde_types, require_serde_parity


class GenericSerdeTests(unittest.TestCase):
    def test_requires_sdk_and_spelling_parity(self):
        primary = {target: {"generated": {"part": "same"}} for target in ("dart", "flutter")}
        rows = [{"target": target, "genericSerde": primary[target]}
                for target in ("dart", "flutter") for _ in range(2)]
        require_serde_parity(rows, primary)
        for invalid in (rows[:3], [*rows[:3], {"target": "flutter", "genericSerde": {"generated": {}}}]):
            with self.assertRaises(ValueError):
                require_serde_parity(invalid, primary)
        with self.assertRaises(ValueError):
            require_serde_parity(rows, {**primary, "dart": {"generated": {"part": "different"}}})

    def test_negative_compiler_probes_require_intended_errors_and_restore_source(self):
        for failure in (None, "success", "unrelated", "wrong-error", "exception"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                (app / "lib").mkdir()
                source = app / "lib/serde_basic.dart"
                original = b"// original input\n"
                source.write_bytes(original)
                calls = []

                def run(command, cwd, expected, env):
                    self.assertEqual((cwd, expected, env), (app, 254, {"SDK": "selected"}))
                    self.assertEqual(source.read_text().count("void main()"), 1)
                    calls.append(command)
                    if failure == "exception":
                        raise RuntimeError("compiler failed")
                    message = "Error: can't be assigned Phantom<num> Phantom<int> doesn't conform String num"
                    if failure == "unrelated":
                        message = "unrelated tool failure"
                    if failure == "wrong-error":
                        message = "Error: missing an import"
                    return subprocess.CompletedProcess(command, 0 if failure == "success" else 254, message, "")

                if failure:
                    with self.assertRaises(RuntimeError if failure == "exception" else ValueError):
                        check_serde_types(run, Path("dart"), app, app, {"SDK": "selected"})
                else:
                    self.assertEqual(check_serde_types(run, Path("dart"), app, app, {"SDK": "selected"}),
                                     ["result", "receiver", "bound", "codec"])
                    self.assertEqual(len(calls), 4)
                self.assertEqual(source.read_bytes(), original)

    def test_success_and_failures_clean_owned_files_and_restore_options(self):
        failures = (None, "build", "check", "analyze", "test", "run", "missing-output",
                    "coverage", "snapshot", "baseline", "name-info", "compiler")
        fixtures = ROOT / "fixtures/generic_serde"
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
                        calls = []

                        def report():
                            records = "".join(f"SF:/app/lib/{stem}.g.dart\nDA:1,2\nend_of_record\n" for stem in STEMS)
                            if failure == "coverage":
                                records = records.replace("DA:1,2", "DA:1,0", 1)
                            (output / "generated.lcov").write_text(records)

                        def run(command, cwd, env=None, expected=0):
                            calls.append(command)
                            self.assertEqual((cwd, env), (app, {"SDK": "selected"}))
                            self.assertEqual("class const Phantom" in (app / "lib/serde_basic.dart").read_text(), primary)
                            test = (app / "test/serde_runtime_test.dart").read_text()
                            self.assertIn("package:selected/serde_basic.dart", test)
                            self.assertEqual("package:flutter_test/flutter_test.dart" in test, bool(flutter))
                            if command[1] == failure:
                                raise RuntimeError("controlled execution failure")
                            if command[1] == "build":
                                for stem in STEMS:
                                    (app / f"lib/{stem}.g.dart").write_bytes((fixtures / f"{stem}_generated.dart.snapshot").read_bytes())
                                if failure == "missing-output":
                                    (app / "lib/serde_scope.g.dart").unlink()
                                if failure == "snapshot":
                                    (app / "lib/serde_scope.g.dart").write_bytes(b"// unexpected output\n")
                                if failure == "baseline":
                                    baseline.write_bytes(b"// unexpected baseline change\n")
                            if command[1] == "analyze":
                                self.assertIn("strict-casts: true", config.read_text())
                                for stem in STEMS:
                                    self.assertNotIn("// ignore_for_file: type=lint", (app / f"lib/{stem}.g.dart").read_text())
                                if failure == "name-info":
                                    return subprocess.CompletedProcess(command, 0, "INFO|LINT|OTHER|file|1|1|1|unrelated", "")
                            if command[1] == "test":
                                self.assertEqual("--no-pub" in command, bool(flutter))
                                if flutter:
                                    for stem in STEMS:
                                        self.assertNotIn("// coverage:ignore-file", (app / f"lib/{stem}.g.dart").read_text())
                                    report()
                            if command[1] == "run":
                                report()
                            if command[1] == "compile":
                                message = "Error: can't be assigned Phantom<num> Phantom<int> doesn't conform String num"
                                return subprocess.CompletedProcess(command, expected, "unrelated" if failure == "compiler" else message, "")
                            return subprocess.CompletedProcess(command, 0, "", "")

                        if failure:
                            kind = FileNotFoundError if failure == "missing-output" else (
                                RuntimeError if failure in ("build", "check", "analyze", "test", "run") else ValueError)
                            with self.assertRaises(kind):
                                check_generic_serde(run, Path("dust"), Path("dart"), app, output, options,
                                                    "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                        else:
                            result = check_generic_serde(run, Path("dust"), Path("dart"), app, output, options,
                                                         "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                            self.assertEqual(result["generatedLineCoverage"], {stem: {"hit": 1, "found": 1} for stem in STEMS})
                            self.assertEqual(result["typeErrors"], ["result", "receiver", "bound", "codec"])
                            self.assertEqual(json.loads((output / "results.json").read_text()), result)
                            for stem in STEMS:
                                self.assertEqual((output / f"{stem}.g.dart").read_bytes(), (fixtures / f"{stem}_generated.dart.snapshot").read_bytes())
                        self.assertEqual(config.read_bytes(), original)
                        self.assertEqual(sorted(path.name for path in (app / "lib").iterdir()), ["old.g.dart"])
                        self.assertEqual(list((app / "test").iterdir()), [])
                        if failure != "baseline":
                            self.assertEqual(baseline.read_bytes(), b"// unchanged\n")

    def test_freshness_check_never_overwrites_a_preexisting_input(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "lib").mkdir()
            source = root / "lib/serde_basic.dart"
            source.write_bytes(b"// caller-owned\n")
            with self.assertRaisesRegex(ValueError, "fresh owned"):
                check_generic_serde(None, Path("dust"), Path("dart"), root, root / "proof", root / "options", "selected", {})
            self.assertEqual(source.read_bytes(), b"// caller-owned\n")


if __name__ == "__main__":
    unittest.main()

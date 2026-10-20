"""Generic validation probes fail closed and preserve caller-owned staging."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_generic_validation import ROOT, STEMS, check_generic_validation, require_validation_parity, validation_coverage


class GenericValidationTests(unittest.TestCase):
    def test_traditional_primary_and_sdk_output_must_match(self):
        primary = {target: {"generated": {"part": "hash"}} for target in ("dart", "flutter")}
        rows = [{"target": target, "genericValidation": primary[target]}
                for target in ("dart", "flutter") for _ in range(2)]
        require_validation_parity(rows, primary)
        with self.assertRaises(ValueError):
            require_validation_parity(rows[:3], primary)
        with self.assertRaises(ValueError):
            require_validation_parity(rows, {**primary, "dart": {"generated": {"part": "wrong"}}})
        with self.assertRaises(ValueError):
            require_validation_parity([*rows[:3], {"target": "flutter", "genericValidation": {"generated": {}}}], primary)

    def test_requires_exact_nonempty_fully_hit_libraries(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "coverage.lcov"
            valid = "".join(f"SF:/app/lib/{stem}.g.dart\nDA:1,2\nend_of_record\n" for stem in STEMS)
            report.write_text(valid)
            self.assertEqual(validation_coverage(report), {stem: {"hit": 1, "found": 1} for stem in STEMS})
            for invalid in ("", valid + valid, valid.replace("DA:1,2", "DA:1,0", 1),
                            valid.replace("DA:1,2", "DA:1,-1", 1),
                            valid.replace("DA:1,2\n", "", 1)):
                with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                    report.write_text(invalid)
                    validation_coverage(report)

    def test_success_and_failures_restore_and_clean_owned_files(self):
        for flutter in (None, ["flutter"]):
            for fail in (None, "build", "check", "analyze", "test", "run", "snapshot", "baseline"):
                for primary in (False, True):
                    with self.subTest(flutter=flutter, fail=fail, primary=primary), tempfile.TemporaryDirectory() as directory:
                        root = Path(directory)
                        app = root / "app"
                        (app / "lib").mkdir(parents=True)
                        (app / "test").mkdir()
                        config = app / "analysis_options.yaml"
                        config.write_text("include: ../analysis_options.yaml\n")
                        options = root / "analysis_options.yaml"
                        options.write_text("analyzer:\n  errors:\n    strict_raw_type: ignore\n")
                        baseline = app / "lib/old.g.dart"
                        baseline.write_text("// unchanged")
                        calls = []

                        def run(command, cwd, env=None, expected=0):
                            calls.append(command)
                            self.assertEqual(cwd, app)
                            self.assertEqual(env, {"SDK": "selected"})
                            self.assertEqual("class const Holder" in (app / "lib/validation_basic.dart").read_text(), primary)
                            self.assertIn("package:selected/validation_basic.dart", (app / "test/validation_runtime_test.dart").read_text())
                            if command[1] == fail:
                                raise RuntimeError("controlled failure")
                            if command[1] == "build":
                                expected = (ROOT / "fixtures/generic_validation/generated.dart.snapshot").read_text().replace(
                                    "part of 'model.dart';", "part of 'validation_basic.dart';")
                                for stem in STEMS:
                                    (app / "lib" / f"{stem}.g.dart").write_text(expected)
                                if fail == "snapshot":
                                    (app / "lib/validation_basic.g.dart").write_text("// wrong")
                            if command[1] == "analyze":
                                self.assertIn("strict-casts: true", config.read_text())
                                self.assertNotIn("type=lint", (app / "lib/validation_basic.g.dart").read_text())
                            if command[1] == "test" and flutter:
                                self.assertNotIn("coverage:ignore-file", (app / "lib/validation_basic.g.dart").read_text())
                                report = Path(next(arg.split("=", 1)[1] for arg in command if arg.startswith("--coverage-path=")))
                                report.write_text("".join(f"SF:{app}/lib/{stem}.g.dart\nDA:1,1\nend_of_record\n" for stem in STEMS))
                            if command[1] == "run":
                                self.assertNotIn("--check-ignore", command)
                                Path(command[command.index("-o") + 1]).write_text("".join(
                                    f"SF:{app}/lib/{stem}.g.dart\nDA:1,1\nend_of_record\n" for stem in STEMS))
                            if fail == "baseline" and command[1] == "test":
                                baseline.write_text("// changed")
                            if command[1] == "compile":
                                self.assertEqual(expected, 254)
                                return subprocess.CompletedProcess(command, 254, "", "Error: Holder<num> String can't be assigned to Holder<int> int?")
                            return subprocess.CompletedProcess(command, 0, "", "")

                        def probe():
                            return check_generic_validation(run, Path("dust"), Path("dart"), app,
                                                            root / "proof", options, "selected", {"SDK": "selected"},
                                                            primary=primary, flutter=flutter)

                        # Flutter's own collector replaces the Dart coverage formatter.
                        expected_failure = fail and not (flutter and fail == "run")
                        if expected_failure:
                            with self.assertRaises(ValueError if fail in ("snapshot", "baseline") else RuntimeError):
                                probe()
                            self.assertFalse((root / "proof/results.json").exists())
                        else:
                            result = probe()
                            self.assertEqual(set(result["generated"]), {f"{stem}.g.dart" for stem in STEMS})
                            self.assertEqual(result["nameStyleInfos"], [])
                        self.assertEqual(config.read_text(), "include: ../analysis_options.yaml\n")
                        self.assertEqual(list((app / "lib").iterdir()), [baseline])
                        self.assertEqual(list((app / "test").iterdir()), [])

    def test_existing_owned_names_are_preserved(self):
        for name in ("lib/validation_basic.dart", "lib/validation_shapes.g.dart", "test/validation_runtime_test.dart"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                source = app / name
                source.parent.mkdir()
                source.write_text("keep me")
                with self.assertRaises(ValueError):
                    check_generic_validation(None, Path("dust"), Path("dart"), app, app / "proof", app / "options", "selected", {})
                self.assertEqual(source.read_text(), "keep me")

"""Collision probes must expose unrelated diagnostics and clean up owned files."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_generic_names import ROOT, check_generic_names, require_generated_coverage, require_only_name_infos


class GenericNameTests(unittest.TestCase):
    def test_only_intentional_name_infos_are_accepted(self):
        info = "INFO|LINT|CAMEL_CASE_TYPES|lib/generic_names.dart|1|1|4|name"
        self.assertEqual(require_only_name_infos(info + "\n"), [info])
        self.assertEqual(require_only_name_infos("\n"), [])
        for message in ["noise", info.replace("INFO", "WARNING"),
                        info.replace("CAMEL_CASE_TYPES", "UNUSED_FIELD"),
                        info.replace("LINT", "COMPILE_TIME_ERROR")]:
            with self.subTest(message=message), self.assertRaises(ValueError):
                require_only_name_infos(message)

    def test_success_and_each_failure_restore_owned_staging(self):
        for fail in [None, "build", "check", "analyze", "test", "run", "snapshot"]:
            for primary in [False, True]:
                with self.subTest(fail=fail, primary=primary), tempfile.TemporaryDirectory() as name:
                    root = Path(name)
                    app = root / "app"
                    (app / "lib").mkdir(parents=True)
                    (app / "test").mkdir()
                    config = app / "analysis_options.yaml"
                    config.write_text("include: ../analysis_options.yaml\n")
                    options = root / "analysis_options.yaml"
                    options.write_text("analyzer:\n  errors:\n    strict_raw_type: ignore\n")
                    baseline = config.read_bytes()
                    calls = []

                    def run(command, cwd, env=None):
                        calls.append(command)
                        self.assertEqual(cwd, app)
                        self.assertEqual(env, {"SDK": "selected"})
                        source = (app / "lib/generic_names.dart").read_text()
                        self.assertEqual("class const CoreNames" in source, primary)
                        self.assertIn("package:selected/generic_names.dart", (app / "test/generic_names_test.dart").read_text())
                        if command[1] == fail:
                            raise RuntimeError("controlled failure")
                        if command[1] == "build":
                            content = (ROOT / "fixtures/generic_names/generated.dart.snapshot").read_bytes()
                            (app / "lib/generic_names.g.dart").write_bytes(b"// wrong contract\n" if fail == "snapshot" else content)
                        if command[1] == "analyze":
                            self.assertEqual(command[:3], ["dart", "analyze", "--format=machine"])
                            self.assertIn("strict-casts: true", config.read_text())
                            self.assertNotIn("type=lint", (app / "lib/generic_names.g.dart").read_text())
                        if command[1] == "run":
                            self.assertNotIn("--check-ignore", command)
                            Path(command[command.index("-o") + 1]).write_text(
                                f"SF:{app}/lib/generic_names.g.dart\nDA:1,1\nend_of_record\n")
                        return subprocess.CompletedProcess(command, 0, "", "")

                    def probe():
                        return check_generic_names(run, Path("dust"), Path("dart"), app,
                                                   root / "evidence", options, "selected", {"SDK": "selected"}, primary=primary)

                    if fail:
                        with self.assertRaises(ValueError if fail == "snapshot" else RuntimeError):
                            probe()
                        self.assertFalse((root / "evidence/results.json").exists())
                    else:
                        result = probe()
                        self.assertEqual(result["nameStyleInfos"], [])
                        self.assertEqual(len(result["generated"]), 64)
                        self.assertEqual([call[1] for call in calls], ["build", "check", "analyze", "test", "run"])
                        self.assertEqual(result["generatedLineCoverage"], {"hit": 1, "found": 1})
                        self.assertTrue((root / "evidence/results.json").is_file())
                    self.assertEqual(config.read_bytes(), baseline)
                    self.assertEqual(list((app / "lib").iterdir()), [])
                    self.assertEqual(list((app / "test").iterdir()), [])

    def test_existing_files_are_never_overwritten_or_removed(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "lib").mkdir()
            source = root / "lib/generic_names.dart"
            source.write_text("keep me")
            with self.assertRaises(ValueError):
                check_generic_names(None, Path("dust"), Path("dart"), root,
                                    root / "evidence", root / "options", "selected", {})
            self.assertEqual(source.read_text(), "keep me")

    def test_coverage_requires_one_nonempty_fully_hit_generated_file(self):
        with tempfile.TemporaryDirectory() as name:
            report = Path(name) / "generated.lcov"
            valid = "SF:/app/lib/generic_names.g.dart\nDA:1,2\nDA:2,1\nend_of_record\n"
            report.write_text(valid)
            self.assertEqual(require_generated_coverage(report), {"hit": 2, "found": 2})
            for invalid in ["", valid + valid, valid.replace("DA:1,2", "DA:1,0"),
                            valid.replace("DA:1,2", "DA:1,-1"),
                            "SF:/app/lib/generic_names.g.dart\nend_of_record\n"]:
                report.write_text(invalid)
                with self.assertRaises(ValueError):
                    require_generated_coverage(report)

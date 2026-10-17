"""Lexical SDK probes must reject incomplete proof and restore owned staging."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_lexical_scopes import ROOT, check_lexical_scopes


class LexicalScopeTests(unittest.TestCase):
    def test_success_and_failures_restore_staging_and_analysis(self):
        for primary in (False, True):
            for fail in (None, "build", "check", "analyze", "test", "run", "coverage", "snapshot"):
                with self.subTest(primary=primary, fail=fail), tempfile.TemporaryDirectory() as name:
                    root = Path(name)
                    app = root / "app"
                    (app / "lib").mkdir(parents=True)
                    (app / "test").mkdir()
                    config = app / "analysis_options.yaml"
                    baseline = b"include: ../analysis_options.yaml\n"
                    config.write_bytes(baseline)
                    options = root / "analysis_options.yaml"
                    options.write_text('analyzer:\n  exclude:\n    - "**/*.g.dart"\n')
                    calls = []

                    def run(command, cwd, env=None):
                        calls.append(command)
                        self.assertEqual(cwd, app)
                        self.assertEqual(env, {"SDK": "selected"})
                        self.assertEqual("class const LexicalProbe" in (app / "lib/lexical_probe.dart").read_text(), primary)
                        self.assertIn("package:selected/lexical_probe.dart", (app / "test/lexical_probe_test.dart").read_text())
                        if command[1] == fail:
                            raise RuntimeError("controlled failure")
                        if command[1] == "build":
                            snapshot = (ROOT / "fixtures/namespace_lexical_shadowing/generated.dart.snapshot").read_bytes()
                            (app / "lib/lexical_probe.g.dart").write_bytes(b"wrong" if fail == "snapshot" else snapshot)
                        if command[1] == "analyze":
                            self.assertIn("strict-casts: true", config.read_text())
                            self.assertNotIn('"**/*.g.dart"', config.read_text())
                            self.assertNotIn("type=lint", (app / "lib/lexical_probe.g.dart").read_text())
                        if command[1] == "run":
                            self.assertNotIn("--check-ignore", command)
                            Path(command[command.index("-o") + 1]).write_text(
                                f"SF:{app}/lib/lexical_probe.g.dart\nDA:1,{0 if fail == 'coverage' else 1}\nend_of_record\n")
                        return subprocess.CompletedProcess(command, 0, "", "")

                    def probe():
                        return check_lexical_scopes(run, Path("dust"), Path("dart"), app,
                                                    root / "evidence", options, "selected", {"SDK": "selected"}, primary=primary)

                    if fail:
                        with self.assertRaises(ValueError if fail in ("coverage", "snapshot") else RuntimeError):
                            probe()
                    else:
                        result = probe()
                        self.assertEqual(result["generatedLineCoverage"], {"lexical_probe": {"hit": 1, "found": 1}})
                        self.assertEqual(len(result["generated"]), 64)
                        self.assertEqual([call[1] for call in calls], ["build", "check", "analyze", "test", "run"])
                    self.assertEqual(config.read_bytes(), baseline)
                    self.assertEqual(list((app / "lib").iterdir()), [])
                    self.assertEqual(list((app / "test").iterdir()), [])

    def test_existing_source_output_or_test_is_never_overwritten(self):
        for relative in ("lib/lexical_probe.dart", "lib/lexical_probe.g.dart", "test/lexical_probe_test.dart"):
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                existing = root / relative
                existing.parent.mkdir(parents=True)
                existing.write_text("keep me")
                with self.assertRaises(ValueError):
                    check_lexical_scopes(None, Path("dust"), Path("dart"), root,
                                        root / "evidence", root / "options", "selected", {})
                self.assertEqual(existing.read_text(), "keep me")

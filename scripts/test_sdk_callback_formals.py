"""Callback SDK probes must keep staging safe and reject unproven success."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_callback_formals import check_callback_formals, expected_style_infos


class CallbackTests(unittest.TestCase):
    def test_only_expected_style_infos_are_accepted(self):
        line = "INFO|LINT|USE_FUNCTION_TYPE_SYNTAX_FOR_PARAMETERS|probe.dart|1|1|4|Use Function"
        self.assertEqual(expected_style_infos((line + "\n") * 7, Path("probe.dart"), primary=True), 7)
        for output in ("", "No issues found", line.replace("INFO", "ERROR"),
                       line.replace("USE_FUNCTION_TYPE_SYNTAX_FOR_PARAMETERS", "OTHER"),
                       line, line.replace("probe.dart", "generated.g.dart")):
            with self.assertRaises(ValueError):
                expected_style_infos(output, Path("probe.dart"), primary=True)

    def test_every_failure_restores_configuration_and_owned_probe_files(self):
        for primary in (False, True):
            for flutter in (None, ["flutter"]):
                for failure in (None, "build", "check", "analyze", "test", "run", "coverage", "diagnostic"):
                    if flutter and failure == "run":
                        continue
                    with self.subTest(primary=primary, flutter=flutter, failure=failure), tempfile.TemporaryDirectory() as name:
                        root = Path(name)
                        app = root / "app"
                        (app / "lib").mkdir(parents=True)
                        (app / "test").mkdir()
                        config = app / "analysis_options.yaml"
                        config.write_bytes(b"original config\r\n")
                        options = root / "analysis_options.yaml"
                        options.write_text('include: package:lints/recommended.yaml\nanalyzer:\n  exclude:\n    - "**/*.g.dart"\n')
                        output = root / "proof"

                        def run(command, cwd, env=None):
                            self.assertEqual(cwd, app)
                            self.assertEqual(env, {"SDK": "selected"})
                            verb = command[1]
                            if verb == failure:
                                raise RuntimeError("controlled failure")
                            if verb == "build":
                                (app / "lib/callback_probe.g.dart").write_text("// ignore_for_file: type=lint\noutput\n")
                            if verb == "analyze":
                                self.assertIn("strict-casts: true", config.read_text())
                                self.assertNotIn("type=lint", (app / "lib/callback_probe.g.dart").read_text())
                                info = f"INFO|LINT|USE_FUNCTION_TYPE_SYNTAX_FOR_PARAMETERS|{app}/lib/callback_probe.dart|1|1|4|Use Function\n" * 7
                                if not primary:
                                    info += f"INFO|LINT|PREFER_INITIALIZING_FORMALS|{app}/lib/callback_probe.dart|1|1|4|Use field\n" * 6
                                return subprocess.CompletedProcess(command, 0, "unexpected" if failure == "diagnostic" else info, "")
                            if verb == "run" or (flutter and verb == "test"):
                                (output / "generated.lcov").write_text(f"SF:{app}/lib/callback_probe.g.dart\nDA:1,{0 if failure == 'coverage' else 1}\nend_of_record\n")
                            return subprocess.CompletedProcess(command, 0, "", "")

                        def probe():
                            return check_callback_formals(run, Path("dust"), Path("dart"), app, output,
                                                          options, "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)

                        if failure:
                            with self.assertRaises(ValueError if failure in ("coverage", "diagnostic") else RuntimeError):
                                probe()
                        else:
                            self.assertEqual(probe()["generatedLineCoverage"], {"callback_probe": {"hit": 1, "found": 1}})
                        self.assertEqual(config.read_bytes(), b"original config\r\n")
                        self.assertEqual(list((app / "lib").iterdir()), [])
                        self.assertEqual(list((app / "test").iterdir()), [])

    def test_existing_staging_files_are_never_overwritten(self):
        for name in ("lib/callback_probe.dart", "lib/callback_probe.g.dart", "test/callback_probe_test.dart", "proof/keep"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                path = app / name
                path.parent.mkdir(parents=True)
                path.write_bytes(b"keep")
                with self.assertRaises(ValueError):
                    check_callback_formals(None, Path("dust"), Path("dart"), app, app / "proof", Path("options"), "selected", {})
                self.assertEqual(path.read_bytes(), b"keep")


if __name__ == "__main__":
    unittest.main()

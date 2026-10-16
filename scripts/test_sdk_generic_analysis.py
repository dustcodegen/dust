"""Restoration and strict-option controls for generated generic SDK checks."""

import tempfile
import unittest
from pathlib import Path

from sdk_generic_analysis import check_generic_analysis


class GenericAnalysisTests(unittest.TestCase):
    def test_strict_analysis_restores_staged_bytes_on_success_and_failure(self):
        for fail in [False, True]:
            with self.subTest(fail=fail), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                app = root / "app"
                (app / "lib").mkdir(parents=True)
                config = app / "analysis_options.yaml"
                config.write_text("include: ../analysis_options.yaml\n")
                options = root / "analysis_options.yaml"
                options.write_text('include: package:very_good_analysis/analysis_options.yaml\n'
                                   'analyzer:\n  exclude:\n    - "**/*.g.dart"\n'
                                   '  errors:\n    strict_raw_type: ignore\n'
                                   '    inference_failure_on_function_invocation: ignore\n'
                                   '    inference_failure_on_instance_creation: ignore\n')
                for stem in ["generic_models", "generic_imported"]:
                    (app / f"lib/{stem}.dart").write_text("// source\n")
                    (app / f"lib/{stem}.g.dart").write_text(
                        "// ignore_for_file: type=lint\n"
                        "// ignore_for_file: unused_element\n// generated\n")
                originals = {path: path.read_bytes() for path in app.rglob("*") if path.is_file()}

                def run(command, cwd, env=None):
                    self.assertEqual(cwd, app)
                    self.assertEqual(env, {"SDK": "selected"})
                    self.assertEqual(command[:3], ["dart", "analyze", "--fatal-infos"])
                    self.assertEqual(len(command), 7)
                    analysis = config.read_text()
                    self.assertIn("strict-casts: true", analysis)
                    self.assertIn("strict-inference: true", analysis)
                    self.assertIn("strict-raw-types: true", analysis)
                    self.assertNotIn("ignore", analysis)
                    self.assertNotIn('"**/*.g.dart"', analysis)
                    for target in ["generic_models", "generic_imported"]:
                        text = (app / f"lib/{target}.g.dart").read_text()
                        self.assertNotIn("type=lint", text)
                        self.assertIn("unused_element", text)
                    if fail:
                        raise RuntimeError("analyzer failure")

                if fail:
                    with self.assertRaises(RuntimeError):
                        check_generic_analysis(run, Path("dart"), app, options, "lib", {"SDK": "selected"})
                else:
                    check_generic_analysis(run, Path("dart"), app, options, "lib", {"SDK": "selected"})
                self.assertEqual({path: path.read_bytes() for path in originals}, originals)

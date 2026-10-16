"""Offline controls for compiler-negative generic acceptance probes."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_generic_types import CASES, check_generic_types


class GenericTypeProbeTests(unittest.TestCase):
    def test_uses_the_consumer_packages_and_expected_diagnostic_for_each_case(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            calls = []

            def run(command, cwd, expected=0, env=None):
                calls.append(command)
                case = Path(command[-3]).stem
                self.assertEqual(cwd, root / "app")
                self.assertEqual(expected, 254)
                self.assertEqual(env, {"SDK": "chosen"})
                self.assertIn(f"--packages={root / 'app/.dart_tool/package_config.json'}", command)
                return subprocess.CompletedProcess(command, 254, "Error: " + " ".join(CASES[case][1]), "")

            result = check_generic_types(run, Path("dart"), root / "app", root / "probes",
                                         "consumer/models", {"SDK": "chosen"})
            self.assertEqual(result, list(CASES))
            self.assertEqual(len(calls), len(CASES))
            manifest = json.loads((root / "probes/results.json").read_text())
            self.assertEqual(manifest, {"package": "consumer/models", "cases": list(CASES)})
            self.assertFalse((root / "app").exists())
            self.assertIn("package:consumer/models/generic_models.dart",
                          (root / "probes/replacement.dart").read_text())

    def test_unrelated_failures_and_success_are_rejected_without_a_pass_manifest(self):
        for status, message in [(0, ""), (254, "Error: missing import"),
                                (254, "String int? can't be assigned"),
                                (1, "Error: String int? can't be assigned")]:
            with self.subTest(status=status, message=message), tempfile.TemporaryDirectory() as name:
                root = Path(name)

                def run(command, cwd, expected=0, env=None):
                    return subprocess.CompletedProcess(command, status, "", message)

                with self.assertRaises(ValueError):
                    check_generic_types(run, Path("dart"), root / "app", root / "probes", "consumer", {})
                self.assertFalse((root / "probes/results.json").exists())


if __name__ == "__main__":
    unittest.main()

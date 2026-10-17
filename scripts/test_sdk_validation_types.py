"""Expected compiler failures must prove the intended helper contract and restore source."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_validation_types import check_validation_types


class ValidationTypeTests(unittest.TestCase):
    def test_expected_errors_and_failures_restore_exact_bytes(self):
        for failure in (None, "command", "unrelated", "success"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                source = app / "lib/validation_basic.dart"
                source.parent.mkdir()
                source.write_bytes(b"// original\r\n")
                calls = []

                def run(command, cwd, expected=0, env=None):
                    calls.append(command)
                    self.assertEqual(cwd, app)
                    self.assertEqual(expected, 254)
                    self.assertEqual(env, {"SDK": "selected"})
                    self.assertIn("_validateValue<int>", source.read_text())
                    if failure == "command":
                        raise RuntimeError("compiler failed")
                    message = "unrelated failure" if failure == "unrelated" else "Error: Holder<num> String can't be assigned to Holder<int> int?"
                    return subprocess.CompletedProcess(command, 0 if failure == "success" else 254, "", message)

                if failure:
                    with self.assertRaises(RuntimeError if failure == "command" else ValueError):
                        check_validation_types(run, Path("dart"), app, app, {"SDK": "selected"})
                else:
                    self.assertEqual(check_validation_types(run, Path("dart"), app, app, {"SDK": "selected"}),
                                     ["receiver", "candidate"])
                    self.assertEqual(len(calls), 2)
                self.assertEqual(source.read_bytes(), b"// original\r\n")

"""The compiler probe must restore owned staging and never overwrite user files."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_static_strings import check_static_strings, literal_cases


class StaticStringTests(unittest.TestCase):
    def test_shared_corpus_includes_line_endings_and_escaped_uri(self):
        cases = {name: (literal, variants) for name, literal, variants in literal_cases()}
        self.assertEqual(cases["unicode-uri"], (r"'\u0061.dart'", [[97, 46, 100, 97, 114, 116]]))
        self.assertEqual(cases["triple-escaped-lf"][1], [[97, 10, 98], [97, 13, 10, 98], [97, 13, 98]])
        self.assertEqual(cases["unpaired-high"], (r"'\uD800'", [[0xd800]]))

    def test_success_and_every_failure_remove_only_the_owned_test(self):
        for fail_at in [None, 1, 2, 3, 4]:
            with self.subTest(fail_at=fail_at), tempfile.TemporaryDirectory() as name:
                app = Path(name)
                (app / "test").mkdir()
                baseline = app / "test/keep.dart"
                baseline.write_bytes(b"keep untouched\r\n")
                output = app / "probe"
                calls = []

                def run(command, cwd, env=None):
                    calls.append(command)
                    self.assertEqual(cwd, app)
                    self.assertEqual(env, {"SDK": "selected"})
                    if len(calls) <= 3:
                        self.assertEqual(command[:4], ["dart", "test", "--reporter=expanded", str(app / "test/static_strings_test.dart")])
                        source = (app / "test/static_strings_test.dart").read_bytes()
                        self.assertIn(b"import 'package:test/test.dart'", source)
                        expected = [b"\n", b"\r\n", b"\r"][len(calls) - 1]
                        self.assertIn(b"'''a" + expected + b"b'''", source)
                    else:
                        self.assertEqual(command, ["dart", "run", str(output / "main.dart")])
                        self.assertIn(r"import '\u0061.dart'", (output / "main.dart").read_text())
                        self.assertIn(r"import '\x61.dart'", (output / "main.dart").read_text())
                        self.assertEqual((output / "a.dart").read_text(), "const marker = 'actual a.dart';\n")
                    if len(calls) == fail_at:
                        raise RuntimeError("controlled failure")
                    return subprocess.CompletedProcess(command, 0, "", "")

                def probe():
                    return check_static_strings(run, Path("dart"), app, output, {"SDK": "selected"})

                if fail_at:
                    with self.assertRaises(RuntimeError):
                        probe()
                    self.assertFalse((output / "results.json").exists())
                else:
                    result = probe()
                    self.assertEqual(result, {"literals": len(literal_cases()), "lineEndings": 3, "escapedImports": 2})
                    self.assertEqual(json.loads((output / "results.json").read_text()), result)
                self.assertFalse((app / "test/static_strings_test.dart").exists())
                self.assertEqual(baseline.read_bytes(), b"keep untouched\r\n")

    def test_existing_test_and_output_are_preserved(self):
        for existing in ["test/static_strings_test.dart", "probe/keep.txt"]:
            with self.subTest(existing=existing), tempfile.TemporaryDirectory() as name:
                app = Path(name)
                path = app / existing
                path.parent.mkdir(parents=True)
                path.write_bytes(b"keep me")
                with self.assertRaises(ValueError):
                    check_static_strings(None, Path("dart"), app, app / "probe", {})
                self.assertEqual(path.read_bytes(), b"keep me")

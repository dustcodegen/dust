"""Sealed SDK proof must fail closed without changing caller-owned staging."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_sealed import ROOT, STEMS, check_generic_sealed, require_sealed_infos, require_sealed_parity
from sdk_sealed_types import check_sealed_types

ERROR = "Error: can't be assigned doesn't conform Event<num Event<int Done<List<num> List<String> List<A> Done<List<int> Done<List<String> GeneratedDone<List<int> GeneratedDone<List<String> GeneratedAt<List<int> GeneratedAt<List<String> String int? Envelope<int> Envelope<String> Choice<int> Choice<String> Map<int Map<String Required named parameter 'value' Required named parameter 'requested' must be provided"
CASES = ["result", "receiver", "bound", "selected", "generated-bound", "generated-result", "codec", "nullable", "required-nullable", "required-factory", "generic-required", "adjacent", "untagged", "json-key"]


class SealedSdkTests(unittest.TestCase):
    def test_parity_requires_all_six_nonempty_equal_outputs(self):
        primary = {target: {"generated": {"part": "same"}} for target in ("dart", "flutter")}
        rows = [{"target": target, "genericSealed": primary[target]} for target in ("dart", "flutter") for _ in range(2)]
        require_sealed_parity(rows, primary)
        for invalid in (rows[:3], [{"target": "dart", "genericSealed": {"generated": {}}}, *rows[1:]], [{"target": "dart", "genericSealed": {"generated": {"part": "different"}}}, *rows[1:]]):
            with self.assertRaises(ValueError):
                require_sealed_parity(invalid, primary)
        with self.assertRaises(ValueError):
            require_sealed_parity(rows, {**primary, "flutter": {"generated": {"part": "different"}}})

    def test_only_intentional_collision_fixture_name_infos_are_recorded(self):
        allowed = "INFO|LINT|AVOID_TYPES_AS_PARAMETER_NAMES|/app/lib/sealed_scope.dart|1|1|1|intentional"
        generated = allowed.replace("AVOID_TYPES_AS_PARAMETER_NAMES", "CAMEL_CASE_TYPES").replace("scope.dart", "scope.g.dart")
        self.assertEqual(require_sealed_infos(allowed + "\n" + generated), [allowed, generated])
        windows = allowed.replace("/app/lib/", "C:\\app\\lib\\")
        self.assertEqual(require_sealed_infos(windows), [windows])
        for invalid in ("tool error", allowed.replace("INFO", "WARNING"), allowed.replace("LINT", "ERROR"), allowed.replace("AVOID_TYPES_AS_PARAMETER_NAMES", "OTHER"), allowed.replace("sealed_scope", "other")):
            with self.assertRaises(ValueError):
                require_sealed_infos(invalid)

    def test_compiler_checks_restore_every_source_after_success_or_failure(self):
        for failure in (None, "success", "unrelated", "wrong-error", "exception"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                app = Path(directory)
                (app / "lib").mkdir()
                originals = {app / f"lib/{stem}.dart": b"// original\n" for stem in STEMS}
                for path, content in originals.items():
                    path.write_bytes(content)
                calls = []

                def run(command, cwd, expected, env):
                    self.assertEqual((cwd, expected, env), (app, 254, {"SDK": "selected"}))
                    selected = Path(command[3])
                    self.assertEqual(selected.read_text().count("void main()"), 1)
                    self.assertTrue(all(path == selected or path.read_bytes() == content for path, content in originals.items()))
                    calls.append(command)
                    if failure == "exception":
                        raise RuntimeError("compiler failed")
                    message = "unrelated" if failure == "unrelated" else ("Error: missing import" if failure == "wrong-error" else ERROR)
                    return subprocess.CompletedProcess(command, 0 if failure == "success" else 254, message, "")

                if failure:
                    with self.assertRaises(RuntimeError if failure == "exception" else ValueError):
                        check_sealed_types(run, Path("dart"), app, app, {"SDK": "selected"})
                else:
                    self.assertEqual(check_sealed_types(run, Path("dart"), app, app, {"SDK": "selected"}), CASES)
                    self.assertEqual(len(calls), len(CASES))
                self.assertEqual({path: path.read_bytes() for path in originals}, originals)

    def test_proof_gates_restore_bytes_clean_owned_files_and_invalidate_stale_results(self):
        failures = (None, "build", "check", "analyze", "test", "run", "missing-output", "coverage", "snapshot", "baseline", "info", "compiler")
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
                        (output / "coverage").mkdir()
                        (output / "coverage/old.json").write_text('{"stale": true}')

                        def golden(stem):
                            return ROOT / f"fixtures/generic_sealed/{stem}_generated.dart.snapshot"

                        def report():
                            records = "".join(f"SF:/app/lib/{stem}.g.dart\nDA:1,2\nend_of_record\n" for stem in STEMS)
                            (output / "generated.lcov").write_text(records.replace("DA:1,2", "DA:1,0", 1) if failure == "coverage" else records)

                        def run(command, cwd, env=None, expected=0):
                            self.assertEqual((cwd, env), (app, {"SDK": "selected"}))
                            self.assertEqual("sealed class const Event" in (app / "lib/sealed_declared.dart").read_text(), primary)
                            content = (app / "test/sealed_declared_test.dart").read_text()
                            self.assertIn("package:selected/sealed_declared.dart", content)
                            self.assertEqual("package:flutter_test/flutter_test.dart" in content, bool(flutter))
                            if command[1] == failure:
                                raise RuntimeError("controlled failure")
                            if command[1] == "build":
                                for stem in STEMS:
                                    (app / f"lib/{stem}.g.dart").write_bytes(golden(stem).read_bytes())
                                if failure == "missing-output":
                                    (app / "lib/sealed_scope.g.dart").unlink()
                                if failure == "snapshot":
                                    with (app / "lib/sealed_scope.g.dart").open("a") as changed:
                                        changed.write("// unexpected output\n")
                                if failure == "baseline":
                                    baseline.write_bytes(b"// unexpected baseline change\n")
                            if command[1] == "analyze":
                                self.assertIn("strict-casts: true", config.read_text())
                                self.assertEqual(command[-len(STEMS):], [str(app / f"test/{stem}_test.dart") for stem in STEMS])
                                for stem in STEMS:
                                    self.assertEqual("// ignore_for_file: type=lint" in (app / f"lib/{stem}.g.dart").read_text(), stem == "sealed_plain")
                                message = "INFO|LINT|OTHER|/app/lib/sealed_scope.dart|1|1|1|unrelated" if failure == "info" else "INFO|LINT|CAMEL_CASE_TYPES|/app/lib/sealed_scope.dart|1|1|1|intentional"
                                return subprocess.CompletedProcess(command, 0, message, "")
                            if command[1] == "test":
                                self.assertEqual("--no-pub" in command, bool(flutter))
                                if flutter:
                                    for stem in STEMS:
                                        self.assertNotIn("// coverage:ignore-file", (app / f"lib/{stem}.g.dart").read_text())
                                    report()
                                else:
                                    coverage = next(value for value in command if value.startswith("--coverage="))
                                    self.assertTrue(coverage.startswith(f"--coverage={output}/coverage-"))
                            if command[1] == "run":
                                report()
                            if command[1] == "compile":
                                return subprocess.CompletedProcess(command, expected, "unrelated" if failure == "compiler" else ERROR, "")
                            return subprocess.CompletedProcess(command, 0, "", "")

                        if failure:
                            kind = FileNotFoundError if failure == "missing-output" else (RuntimeError if failure in ("build", "check", "analyze", "test", "run") else ValueError)
                            with self.assertRaises(kind):
                                check_generic_sealed(run, Path("dust"), Path("dart"), app, output, options, "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                        else:
                            result = check_generic_sealed(run, Path("dust"), Path("dart"), app, output, options, "selected", {"SDK": "selected"}, primary=primary, flutter=flutter)
                            self.assertEqual(result["generatedLineCoverage"], {stem: {"hit": 1, "found": 1} for stem in STEMS})
                            self.assertEqual(result["typeErrors"], CASES)
                            self.assertEqual(json.loads((output / "results.json").read_text()), result)
                            for stem in STEMS:
                                self.assertEqual((output / f"{stem}.g.dart").read_bytes(), golden(stem).read_bytes())
                        self.assertEqual(config.read_bytes(), original)
                        self.assertEqual(sorted(path.name for path in (app / "lib").iterdir()), ["old.g.dart"])
                        self.assertEqual(list((app / "test").iterdir()), [])
                        self.assertEqual((output / "coverage/old.json").read_text(), '{"stale": true}')
                        if failure != "baseline":
                            self.assertEqual(baseline.read_bytes(), b"// unchanged\n")
                        if failure:
                            self.assertFalse((output / "results.json").exists())

    def test_caller_owned_source_is_never_overwritten_or_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            (app / "lib").mkdir()
            source = app / "lib/sealed_declared.dart"
            source.write_bytes(b"// caller-owned\n")
            with self.assertRaisesRegex(ValueError, "fresh owned"):
                check_generic_sealed(None, Path("dust"), Path("dart"), app, app / "proof", app / "options", "selected", {})
            self.assertEqual(source.read_bytes(), b"// caller-owned\n")


if __name__ == "__main__":
    unittest.main()

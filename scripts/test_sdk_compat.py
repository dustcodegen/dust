"""Offline safety regressions for the explicit minimum/latest compatibility gate."""

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from sdk_compat import (
    ROOT, check_unsupported, core_floor, core_lints, generated_hashes,
    latest_flutter_version, require_deterministic_generation, require_version,
    run_command, stable_dart_version,
    stage_example,
)


class SdkCompatibilityTests(unittest.TestCase):
    """No SDK, compiler, process or network is needed for these checks."""

    def test_requires_exact_stable_sdk_versions(self) -> None:
        self.assertEqual(stable_dart_version("Dart SDK version: 3.6.0 (stable) on macos"), "3.6.0")
        for text in ("unknown", "Dart SDK version: 3.6.0-290.beta (beta)", "Dart SDK version: 3.13.5"):
            with self.subTest(text=text), self.assertRaises(ValueError):
                stable_dart_version(text)
        require_version("3.6.0", "3.6.0")
        for actual in ("3.6.1", "3.13.5", "3.6.0-beta"):
            with self.subTest(actual=actual), self.assertRaises(ValueError):
                require_version(actual, "3.6.0")

    def test_latest_listing_never_accepts_beta_or_failed_refresh(self) -> None:
        valid = {"success": True, "messages": [{"message": "Latest stable releases:\nFlutter 3.47.7 | now\n\nLatest beta releases:\nFlutter 3.50.0"}]}
        self.assertEqual(latest_flutter_version(valid), "3.47.7")
        for value in (None, {}, {"success": False}, {"success": True, "messages": None},
                      {"success": True, "messages": [{"message": "Latest beta releases:\nFlutter 3.50.0"}]}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                latest_flutter_version(value)

    def test_reads_the_declared_floor_and_varies_only_lint_tooling(self) -> None:
        self.assertEqual(core_floor(), "3.6.0")
        for version, expected in (("3.6.0", "7.0.0"), ("3.10.0", "10.1.0"), ("3.13.5", "10.3.0")):
            self.assertEqual(core_lints(f"Dart SDK version: {version} (stable)"), expected)
        with self.assertRaises(ValueError):
            core_lints("Dart SDK version: 3.5.0 (stable)")
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "packages/dust_dart").mkdir(parents=True)
            (root / "packages/dust_dart/pubspec.yaml").write_text("name: invalid\n")
            with self.assertRaises(ValueError):
                core_floor(root)

    def test_staging_cannot_reuse_generated_output_or_change_source(self) -> None:
        source = ROOT / "examples/product_showcase/lib/models/product.g.dart"
        previous = source.read_bytes()
        with tempfile.TemporaryDirectory() as name:
            output = Path(name)
            app, expected = stage_example(output, "examples/product_showcase", "7.0.0")
            self.assertIn("lib/models/product.g.dart", expected)
            self.assertIn("test/generated/api/todo_api_test.dart", expected)
            self.assertFalse(list(app.rglob("*.g.dart")))
            self.assertFalse((app / "test/generated/api/todo_api_test.dart").exists())
            expected_analysis = (ROOT / "analysis_options.yaml").read_text()
            for rule in ("remove_deprecations_in_breaking_versions", "unnecessary_ignore"):
                expected_analysis = expected_analysis.replace(f"    {rule}: false\n", "")
            self.assertEqual((output / "analysis_options.yaml").read_text(), expected_analysis)
            self.assertTrue((app / "lib/models/product.dart").is_file())
            with self.assertRaises(ValueError):
                generated_hashes(app)
            (app / "lib/proof.g.dart").write_text("fresh")
            self.assertEqual(set(generated_hashes(app)), {"lib/proof.g.dart"})
        self.assertEqual(source.read_bytes(), previous)

    def test_benchmark_declares_the_test_package_used_by_generated_http_tests(self) -> None:
        manifest = (ROOT / "examples/benchmark_project/pubspec.yaml").read_text()
        self.assertIn("\n  test: ^1.25.15\n", manifest)

    def test_latest_analysis_stays_identical_and_enabled_rules_are_not_removed(self) -> None:
        original = (ROOT / "analysis_options.yaml").read_text()
        with tempfile.TemporaryDirectory() as name:
            output = Path(name)
            stage_example(output, "examples/product_showcase", "10.3.0")
            self.assertEqual((output / "analysis_options.yaml").read_text(), original)
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "analysis_options.yaml").write_text(
                "linter:\n  rules:\n    unnecessary_ignore: true\n"
            )
            with patch("sdk_compat.ROOT", root), patch("sdk_compat.stage_consumer", return_value=root):
                stage_example(root, "examples/product_showcase", "7.0.0")
            self.assertIn("unnecessary_ignore: true", (root / "analysis_options.yaml").read_text())

    def test_failed_process_and_unexpected_success_never_pass(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            for actual, expected in ((1, 0), (0, 1), (2, 1)):
                result = subprocess.CompletedProcess(["tool"], actual, "stdout", "stderr")
                with patch("sdk_compat.subprocess.run", return_value=result), self.assertRaises(RuntimeError):
                    run_command(["tool"], root, root / "log", expected)
                self.assertEqual((root / "log").read_text(), "stdoutstderr")
            result = subprocess.CompletedProcess(["tool"], 0, "ok", "")
            with patch("sdk_compat.subprocess.run", return_value=result):
                self.assertIs(run_command(["tool"], root, root / "log"), result)

    def test_generation_must_match_across_both_sdk_ends(self) -> None:
        rows = [{"target": target, "generated": {"model.g.dart": "hash"}}
                for target in ("dart", "flutter") for _ in range(2)]
        require_deterministic_generation(rows)
        for invalid in ([], rows[:3]):
            with self.assertRaises(ValueError):
                require_deterministic_generation(invalid)
        rows[-1]["generated"] = {"model.g.dart": "different"}
        with self.assertRaises(ValueError):
            require_deterministic_generation(rows)

    def test_negative_probe_requires_sdk_analysis_and_parser_failure(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            calls = []

            def run(command, cwd=ROOT, expected=0):
                calls.append(command)
                return subprocess.CompletedProcess(command, expected, "", "syntax error" if expected else "")

            self.assertEqual(check_unsupported(run, Path("dust"), Path("dart"), Path(name)),
                             ["primary", "named-concise"])
            self.assertEqual(sum("analyze" in call for call in calls), 2)
            self.assertEqual(sum("build" in call or "check" in call for call in calls), 4)
        with tempfile.TemporaryDirectory() as name:
            def bad(command, cwd=ROOT, expected=0):
                return subprocess.CompletedProcess(command, expected, "", "unrelated failure")
            with self.assertRaises(ValueError):
                check_unsupported(bad, Path("dust"), Path("dart"), Path(name))

    def test_negative_probe_rejects_generated_files_and_analyzer_failures(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)

            def emits(command, cwd=ROOT, expected=0):
                if expected:
                    (root / "unsupported/lib/probe.g.dart").write_text("unexpected")
                return subprocess.CompletedProcess(command, expected, "", "syntax error")

            with self.assertRaises(ValueError):
                check_unsupported(emits, Path("dust"), Path("dart"), root)
        with tempfile.TemporaryDirectory() as name:
            calls = []

            def analyzer_fails(command, cwd=ROOT, expected=0):
                calls.append(command)
                if "analyze" in command:
                    raise RuntimeError("sample is not valid Dart")
                return subprocess.CompletedProcess(command, 0, "", "")

            with self.assertRaises(RuntimeError):
                check_unsupported(analyzer_fails, Path("dust"), Path("dart"), Path(name))
            self.assertFalse(any("build" in call or "check" in call for call in calls))


if __name__ == "__main__":
    unittest.main()

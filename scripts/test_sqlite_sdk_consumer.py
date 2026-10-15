"""Regression checks for SDK isolation and native proof comparison."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

from sqlite_sdk_consumer import (
    ROOT, compare_proofs, compatible_lints, native_assets, parse_jit_proof, stage_consumer,
    validate_proof, validate_sdk_pair,
)


def proof() -> list[dict[str, object]]:
    """Return a minimal pair of valid, independently timestamped proofs."""
    return [
        {"mode": mode, "rows": 1, "integrity": "ok", "sqliteVersion": "3.53.4",
         "sqliteSourceId": "source", "migrationChecksum": "checksum",
         "migrationAppliedAt": str(index)}
        for index, mode in enumerate(("sync", "worker"))
    ]


class ConsumerTests(unittest.TestCase):
    """Do not require an SDK, network or native compiler."""

    def test_cannot_attribute_different_compilers_to_one_sdk_run(self) -> None:
        validate_sdk_pair("Dart SDK version: 3.10.0 (stable)", {"dartSdkVersion": "3.10.0"})
        for value in ({}, {"dartSdkVersion": 3}, {"dartSdkVersion": ""},
                      {"dartSdkVersion": " "}, {"dartSdkVersion": "3.12.1"}):
            with self.assertRaises(ValueError):
                validate_sdk_pair("Dart SDK version: 3.10.0 (stable)", value)

    def test_lint_tooling_matches_the_compiler(self) -> None:
        for version in ("3.10.0", "3.10.1", "3.11.0"):
            self.assertEqual(compatible_lints(f"Dart SDK version: {version}"), "10.1.0")
        for version in ("3.12.1", "3.13.5"):
            self.assertEqual(compatible_lints(f"Dart SDK version: {version}"), "10.3.0")
        for version in ("unknown", "Dart SDK version: 3.9.0"):
            with self.assertRaises(ValueError):
                compatible_lints(version)

    def test_stage_has_source_but_no_workspace_or_stale_resolution(self) -> None:
        for relative in ("fixtures/sqlite_sdk_consumer", "examples/shopping_app"):
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as name:
                output = Path(name)
                consumer = stage_consumer(ROOT, output, relative)
                self.assertTrue((consumer / "lib").is_dir())
                self.assertTrue((consumer / "test").is_dir())
                self.assertFalse((consumer / "pubspec.lock").exists())
                self.assertFalse((consumer / ".dart_tool").exists())
                self.assertFalse((output / "pubspec.yaml").exists())
                self.assertEqual((output / "packages/dust_dart").resolve(),
                                 ROOT / "packages/dust_dart")
                self.assertEqual((consumer / "pubspec.yaml").read_bytes(),
                                 (ROOT / relative / "pubspec.yaml").read_bytes())
                overrides = json.loads(
                    (consumer / "pubspec_overrides.yaml").read_text()
                )["dependency_overrides"]
                self.assertEqual(overrides["sqlite3"], "3.5.2")
                self.assertEqual(overrides["dust_db_sqlite3"]["path"],
                                 str(ROOT / "packages/dust_db_sqlite3"))
                if relative.startswith("examples/"):
                    self.assertIn("dust_flutter", overrides)
                    self.assertTrue((consumer / "integration_test").is_dir())
                    self.assertEqual(
                        (consumer / "migrations/0001_shopping_cache.sql").read_bytes(),
                        (ROOT / relative / "migrations/0001_shopping_cache.sql").read_bytes(),
                    )

    def test_accepts_complete_proof(self) -> None:
        self.assertEqual(validate_proof(proof()), proof())

    def test_native_bundle_records_exact_bytes_and_rejects_missing_library(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            bundle = Path(name)
            with self.assertRaises(ValueError):
                native_assets(bundle)
            (bundle / "lib").mkdir()
            (bundle / "lib/fixture").write_bytes(b"abc")
            self.assertEqual(native_assets(bundle), {
                "lib/fixture": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            })

    def test_only_known_compiler_progress_can_precede_json(self) -> None:
        for prefix in ("", "Running build hooks..." * 2):
            self.assertEqual(parse_jit_proof(prefix + json.dumps(proof())), proof())
        with self.assertRaises(ValueError):
            parse_jit_proof("unexpected warning" + json.dumps(proof()))

    def test_rejects_missing_wrong_or_empty_proof(self) -> None:
        for value in (None, {}, [], proof()[:1], list(reversed(proof()))):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_proof(value)
        for key, invalid in (("rows", 0), ("integrity", "corrupt"),
                             ("sqliteVersion", ""), ("sqliteSourceId", None),
                             ("migrationChecksum", 5)):
            value = proof()
            value[1][key] = invalid
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_proof(value)

    def test_rejects_native_or_migration_disagreement(self) -> None:
        for key in ("sqliteVersion", "sqliteSourceId", "migrationChecksum"):
            value = proof()
            value[1][key] = "different"
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_proof(value)
            value[0][key] = "different"
            with self.subTest(aot=key), self.assertRaises(ValueError):
                compare_proofs(proof(), value)

    def test_compare_ignores_only_per_database_timestamp(self) -> None:
        other = copy.deepcopy(proof())
        other[0]["migrationAppliedAt"] = "new database"
        compare_proofs(proof(), other)


if __name__ == "__main__":
    unittest.main()

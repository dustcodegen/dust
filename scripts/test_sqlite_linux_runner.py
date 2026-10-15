"""Check Linux staging and native-engine attribution without Docker or network."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from sqlite_linux_runner import (
    ROOT, configure_runtime_engine, require_fixed_sqlite, select_library,
    runtime_identity, stage_runtime, system_library, validate_runtime_engine,
)


class LinuxRunnerTests(unittest.TestCase):
    """Never let workspace simplification alter production source/constraints."""

    def test_identity_comes_from_all_four_actual_dart_connection_pairs(self) -> None:
        line = "native SQLite: 3.53.4 / source\n"
        self.assertEqual(runtime_identity(line * 4), {
            "sqliteVersion": "3.53.4", "sqliteSourceId": "source",
        })
        for output in ("", line * 3, line * 4 + line, line * 3 + line.replace("source", "other"),
                       line.replace("3.53.4", "3.51.2") * 4):
            with self.subTest(output=output), self.assertRaises(ValueError):
                runtime_identity(output)

    def test_system_package_must_identify_one_existing_shared_library(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            library = root / "libsqlite3.so.0"
            library.write_bytes(b"system fixture")
            line = str(library)
            self.assertEqual(system_library(line), library.resolve())
            for output in ("", "libsqlite3.so.0", line + "\n" + line):
                with self.assertRaises(ValueError):
                    system_library(output)

    def test_requires_a_known_wal_reset_fixed_release_not_an_older_minor(self) -> None:
        for version in ("3.44.6", "3.44.7", "3.50.7", "3.51.3", "3.53.4", "3.54.0"):
            require_fixed_sqlite(version)
        for version in ("3.44.5", "3.45.0", "3.50.6", "3.51.2", "", "vendor-patched"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                require_fixed_sqlite(version)

    def test_runtime_copies_source_and_keeps_workspace_markers(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            workspace = Path(name) / "runtime"
            library = Path(name) / "libsqlite3.so"
            stage_runtime(ROOT, workspace, library)
            pubspec = json.loads((workspace / "pubspec.yaml").read_text())
            self.assertEqual(pubspec["workspace"], [
                "packages/dust_dart", "packages/dust_db_sqlite3",
            ])
            self.assertEqual(pubspec["dependency_overrides"], {"sqlite3": "3.5.2"})
            self.assertEqual(pubspec["hooks"]["user_defines"]["sqlite3"], {
                "source": "system", "name": str(library),
            })
            configure_runtime_engine(workspace, None)
            bundled = json.loads((workspace / "pubspec.yaml").read_text())
            self.assertEqual(bundled["hooks"]["user_defines"]["sqlite3"], {"source": "sqlite3"})
            for package in pubspec["workspace"]:
                self.assertEqual((workspace / package / "pubspec.yaml").read_bytes(),
                                 (ROOT / package / "pubspec.yaml").read_bytes())
                self.assertFalse((workspace / package / ".dart_tool").exists())
                for source in (ROOT / package / "lib").rglob("*.dart"):
                    copied = workspace / source.relative_to(ROOT)
                    self.assertEqual(copied.read_bytes(), source.read_bytes())
            runtime = workspace / "packages/dust_db_sqlite3"
            self.assertTrue((runtime / "test/process_crash_recovery_test.dart").exists())
            self.assertTrue((runtime / "integration_test/support/fault_vfs.c").exists())
            self.assertTrue((runtime / "example/wal_checkpoint.dart").exists())
            with self.assertRaises(FileExistsError):
                stage_runtime(ROOT, workspace, library)

    def test_only_verified_bundled_sqlite_can_feed_the_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            sdk = Path(name)
            library = sdk / "relocated/bundle/lib/libsqlite3.so"
            library.parent.mkdir(parents=True)
            library.write_bytes(b"sqlite fixture")
            proof = {"nativeAssetsSha256": {
                "lib/libsqlite3.so": hashlib.sha256(library.read_bytes()).hexdigest(),
            }}
            self.assertEqual(select_library(sdk, proof), library.resolve())
            library.write_bytes(b"changed")
            with self.assertRaises(ValueError):
                select_library(sdk, proof)
            for invalid in ({}, {"nativeAssetsSha256": {}}):
                with self.assertRaises(ValueError):
                    select_library(sdk, invalid)

    def test_native_link_cannot_escape_the_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            sdk = Path(name)
            outside = sdk / "outside.so"
            outside.write_bytes(b"sqlite fixture")
            library = sdk / "relocated/bundle/lib/libsqlite3.so"
            library.parent.mkdir(parents=True)
            library.symlink_to(outside)
            with self.assertRaises(ValueError):
                select_library(sdk, {"nativeAssetsSha256": {
                    "lib/libsqlite3.so": hashlib.sha256(outside.read_bytes()).hexdigest(),
                }})

    def test_all_four_runtime_mode_pairs_must_match_the_selected_engine(self) -> None:
        proof = {"sqliteVersion": "3.53.4", "sqliteSourceId": "source"}
        line = "native SQLite: 3.53.4 / source\n"
        validate_runtime_engine(line * 4, proof)
        for output in ("", line * 3, line * 5, line * 3 + line.replace("source", "other")):
            with self.subTest(output=output), self.assertRaises(ValueError):
                validate_runtime_engine(output, proof)


if __name__ == "__main__":
    unittest.main()

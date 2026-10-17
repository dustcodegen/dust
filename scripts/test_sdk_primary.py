"""Offline regressions for fresh primary-constructor SDK and SQLx proofs."""

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from sdk_compat import ROOT, check_primary
from sqlite_sdk_consumer import stage_consumer


PARTS = ("models", "private_models", "language_models", "api",
         "generic_models", "generic_imported", "database")
MODES = ("all", "execute", "one", "one", "optional")


def emit_dart(command, app):
    """Model separate ordinary and DB builds, never a successful no-op."""
    if "build" not in command:
        return
    if "db" not in command:
        for part in PARTS[:-1]:
            (app / f"lib/{part}.g.dart").write_text("fresh")
        return
    (app / "lib/database.g.dart").write_text("fresh DB")
    cache = app / ".dust_sql"
    cache.mkdir()
    for name in ("checked_queries", "database"):
        entries = [{"fetch_mode": mode, "driver": "sqlite3"} for mode in MODES]
        (cache / f"{name}-hash.json").write_text(json.dumps({"entries": entries}))


class PrimarySdkTests(unittest.TestCase):
    """No SDK, compiler, database or network is needed for these checks."""

    def test_requires_fresh_generation_sqlx_metadata_and_strict_execution(self):
        with tempfile.TemporaryDirectory() as name:
            output = Path(name)
            app = output / "primary/fixtures/primary_sdk_consumer"
            calls = []

            def run(command, cwd=ROOT, env=None):
                calls.append((command, cwd, env))
                if "build" in command and "db" not in command:
                    self.assertFalse(list(app.rglob("*.g.dart")))
                emit_dart(command, app)
                return subprocess.CompletedProcess(command, 0, "", "")

            def stale_copy(*args, **kwargs):
                staged = stage_consumer(*args, **kwargs)
                (staged / "lib/models.g.dart").write_text("stale copied output")
                return staged

            with patch.dict(os.environ, {"DUST_DATABASE_URL": "sqlite:/must-not-open"}), \
                    patch("sdk_codegen_fixtures.stage_consumer", side_effect=stale_copy), \
                    patch("sdk_compat.check_generic_types", return_value=["checked"]) as negatives, \
                    patch("sdk_compat.check_generic_analysis") as strict, \
                    patch("sdk_compat.check_generic_names") as names, \
                    patch("sdk_compat.check_generic_validation") as validation, \
                    patch("sdk_compat.check_generic_serde") as serde, \
                    patch("sdk_compat.check_generic_inheritance") as inheritance, \
                    patch("sdk_compat.check_generic_sealed") as sealed, \
                    patch("sdk_compat.check_lexical_scopes", return_value={"generated": "verified"}) as lexical:
                generated = check_primary(run, Path("dust"), Path("/sdk/bin/dart"), output)
                self.assertEqual(os.environ["DUST_DATABASE_URL"], "sqlite:/must-not-open")
            negatives.assert_called_once()
            strict.assert_called_once()
            names.assert_called_once()
            lexical.assert_called_once()
            self.assertEqual(lexical.call_args.kwargs, {"primary": True})
            self.assertEqual(json.loads((output / "primary-lexical-scopes.json").read_text()), {"generated": "verified"})
            validation.assert_called_once()
            serde.assert_called_once()
            inheritance.assert_called_once()
            sealed.assert_called_once()
            self.assertEqual(sealed.call_args.kwargs, {"primary": True, "flutter": None})
            self.assertEqual(inheritance.call_args.kwargs, {"primary": True, "flutter": None})
            self.assertEqual(serde.call_args.kwargs, {"primary": True, "flutter": None})
            self.assertEqual(validation.call_args.kwargs, {"primary": True, "flutter": None})
            self.assertEqual(names.call_args.kwargs, {"primary": True})
            self.assertEqual(negatives.call_args.args[1:5],
                             (Path("/sdk/bin/dart"), app, output / "primary-generic-negative",
                              "primary_sdk_consumer"))
            self.assertEqual(set(generated), {f"lib/{part}.g.dart" for part in PARTS})
            self.assertEqual([call[0][1] for call in calls],
                             ["pub", "pub", "build", "check", "db", "check", "check",
                              "format", "analyze", "test"])
            self.assertEqual([call[0] for call in calls[4:7]], [
                ["dust", "db", "build", "--root", str(app)],
                ["dust", "check", "--db", "--root", str(app)],
                ["dust", "check", "--db", "--offline", "--root", str(app)],
            ])
            self.assertEqual(calls[-2][0], ["/sdk/bin/dart", "analyze", "--fatal-infos"])
            expected_sources = [f"lib/{part}.dart" for part in
                                ("api", "checked_queries", "database", "generic_imported", "generic_models", "language_models",
                                 "models", "private_models")]
            expected_sources += [f"test/{part}_test.dart" for part in
                                 ("api", "checked_queries", "generic_models", "language_models", "models",
                                  "private_models", "sqlite_runtime")]
            self.assertEqual(calls[-3][0][4:], expected_sources)
            self.assertEqual(calls[-1][0], ["/sdk/bin/dart", "test", "--reporter=expanded"])
            self.assertEqual(calls[-1][1], app)
            self.assertTrue(all(call[2]["PATH"].startswith("/sdk/bin") for call in calls))
            self.assertTrue(all("DUST_DATABASE_URL" not in call[2] for call in calls))
            overrides = json.loads((app / "pubspec_overrides.yaml").read_text())
            self.assertEqual(overrides["dependency_overrides"]["dust_db_sqlite3"],
                             {"path": str(ROOT / "packages/dust_db_sqlite3")})
            self.assertTrue((app / "migrations/0001_accounts.sql").is_file())

    def test_rejects_each_missing_output_and_execution_failure(self):
        for omitted in PARTS:
            with self.subTest(output=omitted), tempfile.TemporaryDirectory() as name:
                app = Path(name) / "primary/fixtures/primary_sdk_consumer"
                calls = []

                def run(command, cwd=ROOT, env=None):
                    calls.append(command)
                    emit_dart(command, app)
                    path = app / f"lib/{omitted}.g.dart"
                    if path.exists():
                        path.unlink()
                    return subprocess.CompletedProcess(command, 0, "", "")

                with self.assertRaisesRegex(ValueError, "Missing fresh primary outputs"):
                    check_primary(run, Path("dust"), Path("dart"), Path(name))
                self.assertFalse(any("test" in call for call in calls))
        for failure in ("build", "check", "db", "--offline", "format", "analyze", "test"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                app = Path(name) / "primary/fixtures/primary_sdk_consumer"

                def run(command, cwd=ROOT, env=None):
                    if failure in command:
                        raise RuntimeError("failed execution")
                    emit_dart(command, app)
                    return subprocess.CompletedProcess(command, 0, "", "")

                with self.assertRaises(RuntimeError):
                    check_primary(run, Path("dust"), Path("dart"), Path(name))

    def test_each_query_style_requires_complete_sqlite_metadata(self):
        for library in ("checked_queries", "database"):
            for invalid in ("missing", "empty", "shape", "driver", "duplicate"):
                with self.subTest(library=library, invalid=invalid), tempfile.TemporaryDirectory() as name:
                    app = Path(name) / "primary/fixtures/primary_sdk_consumer"
                    calls = []

                    def run(command, cwd=ROOT, env=None):
                        calls.append(command)
                        emit_dart(command, app)
                        if command[1:3] == ["db", "build"]:
                            path = app / f".dust_sql/{library}-hash.json"
                            if invalid == "missing":
                                path.unlink()
                            else:
                                entries = json.loads(path.read_text())["entries"]
                                if invalid == "empty":
                                    entries = []
                                elif invalid == "shape":
                                    entries[0]["fetch_mode"] = "optional"
                                elif invalid == "driver":
                                    entries[0]["driver"] = "postgres"
                                else:
                                    entries += entries
                                path.write_text(json.dumps({"entries": entries}))
                        return subprocess.CompletedProcess(command, 0, "", "")

                    with self.assertRaisesRegex(ValueError, f"metadata: {library}"):
                        check_primary(run, Path("dust"), Path("dart"), Path(name))
                    self.assertFalse(any("test" in call for call in calls))

    def test_flutter_requires_route_output_and_uses_only_flutter_commands(self):
        for complete in (False, True):
            with self.subTest(complete=complete), tempfile.TemporaryDirectory() as name:
                output = Path(name)
                app = output / "primary-flutter/fixtures/primary_flutter_consumer"
                calls = []
                flutter = ["puro", "-e", "latest", "flutter"]

                def run(command, cwd=ROOT, env=None):
                    calls.append(command)
                    if "build" in command:
                        (app / "lib/counter.g.dart").write_text("fresh state")
                        (app / "lib/generic_value.g.dart").write_text("fresh generic model")
                        if complete:
                            (app / "lib/route").mkdir()
                            for part in ("routes", "paths", "navigation", "metadata", "runtime"):
                                (app / f"lib/route/{part}.g.dart").write_text("fresh route")
                    return subprocess.CompletedProcess(command, 0, "", "")

                if not complete:
                    with self.assertRaises(ValueError):
                        check_primary(run, Path("dust"), Path("dart"), output, flutter=flutter)
                    self.assertFalse(any("analyze" in call or "test" in call for call in calls))
                    continue
                with patch("sdk_compat.check_generic_validation") as validation, \
                        patch("sdk_compat.check_generic_serde") as serde, \
                        patch("sdk_compat.check_generic_inheritance") as inheritance, \
                        patch("sdk_compat.check_generic_sealed") as sealed:
                    generated = check_primary(run, Path("dust"), Path("dart"), output, flutter=flutter)
                validation.assert_called_once()
                serde.assert_called_once()
                inheritance.assert_called_once()
                sealed.assert_called_once()
                self.assertEqual(sealed.call_args.kwargs, {"primary": True, "flutter": flutter})
                self.assertEqual(inheritance.call_args.kwargs, {"primary": True, "flutter": flutter})
                self.assertEqual(serde.call_args.kwargs, {"primary": True, "flutter": flutter})
                self.assertEqual(validation.call_args.kwargs, {"primary": True, "flutter": flutter})
                self.assertEqual(len(generated), 7)
                self.assertEqual(calls[-2], [*flutter, "analyze", "--no-pub", "--fatal-infos"])
                self.assertEqual(calls[-1], [*flutter, "test", "--no-pub", "--reporter=expanded"])
                self.assertFalse(any("db" in call or "--offline" in call for call in calls))
                self.assertEqual((app / "analysis_options.yaml").read_bytes(),
                                 (ROOT / "fixtures/primary_flutter_consumer/analysis_options.yaml").read_bytes())


if __name__ == "__main__":
    unittest.main()

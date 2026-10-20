"""Offline provenance checks; no SDK, compiler, network or symlink privileges."""

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from check_listed_dart_grammar import FILES, ROOT, check
from check_listed_parser_parity import CASES, CLASS_CASES, I18N_CASES, MARKER, compare, records


class ListedGrammarTests(unittest.TestCase):
    """Source tampering must fail before a candidate grammar is compiled."""

    def stage(self, root: Path) -> dict:
        """Create a small synthetic pinned tree instead of copying megabytes."""
        for name in FILES:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"source\n")
        value = {
            "repository": "https://github.com/UserNobody14/tree-sitter-dart",
            "revision": "acfb453395e25cf6a1018e064df3c92e68e38782", "abi": 15,
            "files": {name: hashlib.sha256(b"source\n").hexdigest() for name in FILES},
        }
        self.write(root, value)
        return value

    def write(self, root: Path, value: object) -> None:
        """Write the synthetic provenance document under the owned temp root."""
        (root / "UPSTREAM.json").write_text(json.dumps(value), encoding="utf-8")

    def test_active_parser_is_pinned_and_legacy_harness_is_separate(self) -> None:
        self.assertEqual(check(ROOT / "vendor/tree-sitter-dart"), [])
        manifest = (ROOT / "crates/dust_parser_dart_ts/Cargo.toml").read_text()
        self.assertIn('tree-sitter-dart = { path = "../../vendor/tree-sitter-dart" }', manifest)
        harness = (ROOT / "crates/dust_parser_dart_ts/parity/Cargo.toml").read_text()
        self.assertIn('tree-sitter-dart = "=0.1.0"', harness)
        self.assertIn('name = "dust_legacy_parser_parity"', harness)

    def test_parser_parity_requires_all_complete_matching_records(self) -> None:
        output = "\n".join(
            f"test prefix {MARKER}{name}:" +
            ("I18nScanResult { name: User }" if name in I18N_CASES else
             "[ParsedClassSurface { name: User }]" if name in CLASS_CASES else "ParsedDartFileSurface { name: User }")
            for name in sorted(CASES)
        )
        self.assertEqual(set(records(output)), CASES)
        compare(output, output)
        with self.assertRaisesRegex(ValueError, "differs"):
            compare(output, output.replace("User", "Other", 1))
        for invalid in ("", output.split("\n", 1)[1], output + "\n" + output,
                        output.replace("[ParsedClassSurface { name: User }]", "[]", 1),
                        output.replace("ParsedDartFileSurface { name: User }", "[]", 1),
                        output.replace("ParsedDartFileSurface { name: User }", "[ParsedClassSurface { name: User }]", 1),
                        output.replace("I18nScanResult { name: User }", "[]", 1),
                        output.replace("I18nScanResult { name: User }", "ParsedDartFileSurface { name: User }", 1),
                        output + f"\n{MARKER}unknown:invalid", output + f"\n{MARKER}broken"):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                records(invalid)

    def test_pinned_bytes_survive_crlf_checkout_configuration(self) -> None:
        snapshot = ROOT / "vendor/tree-sitter-dart"
        expected = json.loads((snapshot / "UPSTREAM.json").read_text())["files"]
        with tempfile.TemporaryDirectory() as name:
            prefix = Path(name).as_posix() + "/"
            subprocess.run([
                "git", "-C", str(ROOT), "-c", "core.autocrlf=true", "checkout-index",
                f"--prefix={prefix}", "--",
                *(f"vendor/tree-sitter-dart/{path}" for path in sorted(FILES)),
            ], check=True, capture_output=True)
            for path, digest in expected.items():
                copied = Path(name) / "vendor/tree-sitter-dart" / path
                self.assertEqual(hashlib.sha256(copied.read_bytes()).hexdigest(), digest, path)

    def test_changed_or_missing_source_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            self.stage(root)
            self.assertEqual(check(root), [])
            (root / "src/parser.c").write_bytes(b"changed")
            self.assertIn("src/parser.c: differs", "\n".join(check(root)))
            (root / "LICENSE").unlink()
            self.assertIn("LICENSE:", "\n".join(check(root)))

    def test_incomplete_source_set_and_mutable_revision_are_rejected(self) -> None:
        for change in ("missing", "escape", "revision", "repository", "abi", "not-object"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                value = self.stage(root)
                if change == "missing":
                    value["files"].pop("LICENSE")
                elif change == "escape":
                    value["files"]["../secret"] = "bad"
                elif change == "not-object":
                    value = []
                else:
                    value[change] = "wrong"
                self.write(root, value)
                self.assertTrue(check(root))

    def test_unreadable_metadata_and_escaping_sources_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            self.assertTrue(check(root))
            self.stage(root)
            (root / "UPSTREAM.json").write_text("invalid", encoding="utf-8")
            self.assertTrue(check(root))
            self.stage(root)
            with patch.object(Path, "is_relative_to", return_value=False):
                self.assertEqual(len(check(root)), len(FILES))


if __name__ == "__main__":
    unittest.main()

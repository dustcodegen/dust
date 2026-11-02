"""Offline regressions for frozen SDK evidence identity."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from sdk_source_identity import FrozenSourceIdentity, invalidate_proof


class SourceIdentityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.source = self.root / "lib/model.dart"
        self.dust = self.root / "target/dust"
        self.source.parent.mkdir()
        self.dust.parent.mkdir()
        self.source.write_text("class Model {}\n")
        self.dust.write_bytes(b"dust-one")
        (self.root / ".gitignore").write_text("target/\n")
        self.git("init", "-q")
        self.git("config", "user.name", "Dust Test")
        self.git("config", "user.email", "dust@example.com")
        self.git("add", ".")
        self.git("commit", "-qm", "base")

    def tearDown(self) -> None:
        self.directory.cleanup()

    def git(self, *arguments: str) -> None:
        subprocess.run(
            ["git", *arguments],
            cwd=self.root,
            check=True,
            capture_output=True,
        )

    def test_clean_source_and_ignored_build_artifacts_keep_identity(self) -> None:
        frozen = FrozenSourceIdentity(self.root, self.dust)
        self.assertEqual(frozen.identity.evidence()["sourceDirty"], False)
        (self.root / "target/cache").write_text("ignored")
        frozen.verify()
        self.assertEqual(frozen.identity.evidence()["sourceCommit"], self.head())

    def test_source_edit_fails_even_after_the_edit_is_restored(self) -> None:
        frozen = FrozenSourceIdentity(self.root, self.dust)
        original = self.source.read_text()
        self.source.write_text("class Changed {}\n")
        with self.assertRaises(ValueError):
            frozen.verify()
        self.source.write_text(original)
        with self.assertRaises(ValueError):
            frozen.verify()
        FrozenSourceIdentity(self.root, self.dust).verify()

    def test_untracked_content_change_is_not_hidden_by_the_same_status(self) -> None:
        draft = self.root / "lib/draft.dart"
        draft.write_text("class Draft {}\n")
        frozen = FrozenSourceIdentity(self.root, self.dust)
        draft.write_text("class ChangedDraft {}\n")
        with self.assertRaises(ValueError):
            frozen.verify()

    def test_commit_change_fails(self) -> None:
        frozen = FrozenSourceIdentity(self.root, self.dust)
        (self.root / "README.md").write_text("new commit\n")
        self.git("add", "README.md")
        self.git("commit", "-qm", "next")
        with self.assertRaises(ValueError):
            frozen.verify()

    def test_cli_replacement_fails(self) -> None:
        frozen = FrozenSourceIdentity(self.root, self.dust)
        self.dust.write_bytes(b"dust-two")
        with self.assertRaises(ValueError):
            frozen.verify()

    def test_stale_proof_is_invalidated_before_a_new_run(self) -> None:
        proof = self.root / "proof.json"
        proof.write_text('{"stale": true}\n')
        invalidate_proof(proof)
        self.assertFalse(proof.exists())
        invalidate_proof(proof)

    def head(self) -> str:
        return subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()


if __name__ == "__main__":
    unittest.main()

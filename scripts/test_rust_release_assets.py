"""Regression checks for the Rust release publication gate."""

import io
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from rust_release_assets import ASSETS, MANIFEST, verify


class ReleaseAssetsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ASSETS:
            self.archive(name)

    def archive(self, name, binary=None, *, mode=0o755, symlink=False):
        if name.endswith(".zip"):
            with zipfile.ZipFile(self.root / name, "w") as handle:
                handle.writestr(binary or "dust.exe", b"binary")
        else:
            with tarfile.open(self.root / name, "w:gz") as handle:
                entry = tarfile.TarInfo(binary or "dust")
                entry.size = 6
                entry.mode = mode
                if symlink:
                    entry.type = tarfile.SYMTYPE
                    entry.linkname = "elsewhere"
                handle.addfile(entry, io.BytesIO(b"binary"))

    def test_complete_release_round_trip(self):
        verify(self.root, prepare=True)
        verify(self.root)

    def test_missing_archive(self):
        (self.root / ASSETS[0]).unlink()
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_unexpected_file(self):
        (self.root / "extra.txt").write_text("unexpected")
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_checksum_tampering(self):
        verify(self.root, prepare=True)
        (self.root / MANIFEST).write_text("bad checksum\n")
        with self.assertRaises(ValueError):
            verify(self.root)

    def test_tar_path_traversal(self):
        self.archive(ASSETS[0], "../dust")
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_zip_path_traversal(self):
        self.archive(ASSETS[1], "../dust.exe")
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_tar_symlink(self):
        self.archive(ASSETS[0], symlink=True)
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_non_executable_binary(self):
        self.archive(ASSETS[0], mode=0o644)
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)

    def test_published_manifest_is_not_overwritten(self):
        verify(self.root, prepare=True)
        with self.assertRaises(ValueError):
            verify(self.root, prepare=True)


if __name__ == "__main__":
    unittest.main()

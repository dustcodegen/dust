"""Offline checks for current documentation and package repository links."""

import re
import tomllib
import unittest
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent
REPOSITORY = "https://github.com/dustcodegen/dust"


class RepositoryMetadataTests(unittest.TestCase):
    def test_rust_workspace_repository(self):
        package = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]
        self.assertEqual(package["repository"], REPOSITORY)
        self.assertEqual(package["homepage"], REPOSITORY)

    def test_package_repository_and_support_links(self):
        for manifest in sorted((ROOT / "packages").glob("*/pubspec.yaml")):
            with self.subTest(package=manifest.parent.name):
                metadata = dict(re.findall(r"^(\w+): (https://\S+)$", manifest.read_text(), re.M))
                self.assertEqual(metadata["repository"], REPOSITORY)
                self.assertEqual(metadata["issue_tracker"], f"{REPOSITORY}/issues")
                if "homepage" in metadata:
                    self.assertEqual(metadata["homepage"], REPOSITORY)
                if "documentation" in metadata:
                    self.assertEqual(metadata["documentation"],
                                     f"{REPOSITORY}/tree/main/packages/{manifest.parent.name}#readme")

    def test_documentation_links_use_current_repository_and_existing_paths(self):
        documents = list(ROOT.glob("*.md"))
        for directory in ("docs", "packages", "skills"):
            documents.extend((ROOT / directory).rglob("*.md"))
        for document in documents:
            with self.subTest(document=str(document.relative_to(ROOT))):
                text = document.read_text()
                self.assertFalse("y3l1n4ung/dust" in text, "Historical repository link")
                self.assertFalse("codecov.io/gh/" in text, "Unmigrated coverage badge")
                for url in re.findall(r"https://github\.com/dustcodegen/dust[^\s)<>\"`|]*", text):
                    path = unquote(urlsplit(url).path)
                    for prefix in ("/dustcodegen/dust/blob/main/", "/dustcodegen/dust/tree/main/"):
                        if path.startswith(prefix):
                            self.assertTrue((ROOT / path.removeprefix(prefix)).exists(), url)


if __name__ == "__main__":
    unittest.main()

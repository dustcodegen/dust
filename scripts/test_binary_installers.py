"""Test the installer destination and fail-closed checks without network or Rust."""

import hashlib
import io
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class BinaryInstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "commands"
        self.bin.mkdir()
        self.log = self.root / "requests.jsonl"
        self.env = os.environ.copy()
        self.env.pop("DUST_REPO", None)
        self.env.pop("DUST_VERSION", None)
        self.env.update({"PATH": f"{self.bin}:/usr/bin:/bin", "DUST_INSTALL_DIR": str(self.root / "installed")})
        self.command("curl", f"""#!{sys.executable}
import json, shutil, sys
from pathlib import Path
url = next(arg for arg in sys.argv if arg.startswith('https://'))
with open({str(self.log)!r}, 'a') as log:
    log.write(json.dumps(url) + '\\n')
shutil.copyfile(Path({str(self.root)!r}) / url.rsplit('/', 1)[1], sys.argv[sys.argv.index('-o') + 1])
""")

    def command(self, name, contents):
        target = self.bin / name
        target.write_text(contents)
        target.chmod(0o755)

    def fixture(self, os_name="Darwin", arch="arm64", *, checksum=True):
        self.command("uname", f"#!/bin/sh\ncase \"$1\" in -s) echo {os_name};; -m) echo {arch};; esac\n")
        targets = {("Darwin", "arm64"): "aarch64-apple-darwin", ("Darwin", "x86_64"): "x86_64-apple-darwin",
                   ("Linux", "aarch64"): "aarch64-unknown-linux-gnu", ("Linux", "x86_64"): "x86_64-unknown-linux-gnu"}
        asset = f"dust-{targets[(os_name, arch)]}.tar.gz"
        data = b"#!/bin/sh\necho 'dust 0.2.1'\n"
        with tarfile.open(self.root / asset, "w:gz") as archive:
            entry = tarfile.TarInfo("dust")
            entry.size = len(data)
            entry.mode = 0o755
            archive.addfile(entry, io.BytesIO(data))
        digest = hashlib.sha256((self.root / asset).read_bytes()).hexdigest() if checksum else "0" * 64
        (self.root / "SHA256SUMS.txt").write_text(f"{digest}  {asset}\n")
        return asset

    def run_installer(self):
        return subprocess.run(["bash", str(ROOT / "install.sh")], env=self.env, text=True, capture_output=True, timeout=30)

    def test_all_unix_architectures_install_without_rust(self):
        for platform in [("Darwin", "arm64"), ("Darwin", "x86_64"), ("Linux", "aarch64"), ("Linux", "x86_64")]:
            with self.subTest(platform=platform):
                asset = self.fixture(*platform)
                result = self.run_installer()
                self.assertEqual(result.returncode, 0, result.stderr)
                binary = self.root / "installed/dust"
                self.assertEqual(subprocess.check_output([str(binary)], text=True).strip(), "dust 0.2.1")
                urls = [json.loads(line) for line in self.log.read_text().splitlines()]
                self.assertIn(f"https://github.com/dustcodegen/dust/releases/latest/download/{asset}", urls)

    def test_explicit_repository_and_version(self):
        asset = self.fixture()
        self.env.update({"DUST_REPO": "example/mirror", "DUST_VERSION": "v0.2.1"})
        self.assertEqual(self.run_installer().returncode, 0)
        self.assertIn(f"https://github.com/example/mirror/releases/download/v0.2.1/{asset}", self.log.read_text())

    def test_checksum_failure_preserves_existing_installation(self):
        self.fixture(checksum=False)
        binary = self.root / "installed/dust"
        binary.parent.mkdir()
        binary.write_text("existing")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertEqual(binary.read_text(), "existing")

    def test_missing_checksum_rejects_installation(self):
        self.fixture()
        (self.root / "SHA256SUMS.txt").write_text("")
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assertFalse((self.root / "installed/dust").exists())

    def test_unsupported_architecture_does_not_download(self):
        self.command("uname", "#!/bin/sh\ncase \"$1\" in -s) echo Linux;; -m) echo sparc;; esac\n")
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assertFalse(self.log.exists())

    def test_powershell_default_destination_and_checksum_protection(self):
        contents = (ROOT / "install.ps1").read_text()
        self.assertIn('else { "dustcodegen/dust" }', contents)
        self.assertIn("Get-FileHash -Algorithm SHA256", contents)
        self.assertIn("if ($ActualHash -ne $ExpectedHash)", contents)
        self.assertNotIn("cargo", contents)


if __name__ == "__main__":
    unittest.main()

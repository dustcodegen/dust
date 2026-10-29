"""Run real formatting entrypoints against an isolated, offline fake SDK."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONSUMER = "fixtures/sqlite_sdk_consumer"
FAKE_DART = '''#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path
root = Path(os.environ["DUST_FORMAT_TEST_ROOT"])
target = Path.cwd().relative_to(root).as_posix()
args = sys.argv[1:]
with (root / "commands.jsonl").open("a") as output:
    output.write(json.dumps([target, args]) + "\\n")
if args == ["pub", "get"]:
    status = int(os.environ.get("DUST_FORMAT_TEST_PUB_EXIT", "0"))
    if status:
        sys.exit(status)
    Path(".dart_tool").mkdir(exist_ok=True)
    Path(".dart_tool/package_config.json").write_text("{}")
elif args[0] == "format":
    if target == "fixtures/sqlite_sdk_consumer":
        if not Path(".dart_tool/package_config.json").is_file():
            sys.exit(23)
        sys.exit(int(os.environ.get("DUST_FORMAT_TEST_FORMAT_EXIT", "0")))
else:
    sys.exit(24)
'''


class FormatBootstrapTests(unittest.TestCase):
    """No network, real Pub cache, generated source or SDK is touched."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory(prefix="dust-format-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        for relative in ("scripts/dart", "scripts/rust", "bin"):
            (self.root / relative).mkdir(parents=True)
        for relative in ("scripts/dart/format.sh", "scripts/format.sh"):
            shutil.copy2(ROOT / relative, self.root / relative)
        scripts = {
            "scripts/workspace_packages.sh": "#!/bin/sh\necho packages/runtime\n",
            "scripts/rust/format.sh": "#!/bin/sh\nexit 0\n",
            "bin/dart": FAKE_DART,
        }
        for relative, source in scripts.items():
            path = self.root / relative
            path.write_text(source)
            path.chmod(0o755)
        for relative in (
            "packages/runtime",
            CONSUMER,
            "fixtures/server_codegen_app",
            "fixtures/server_client_server",
            "fixtures/server_client_app",
            "fixtures/server_admin_client_app",
            "examples/product_showcase",
        ):
            target = self.root / relative
            target.mkdir(parents=True)
            (target / "example.dart").write_text("void main() {}\n")
        self.env = {
            **os.environ, "PATH": f"{self.root / 'bin'}:{os.environ['PATH']}",
            "DUST_FORMAT_TEST_ROOT": str(self.root),
        }

    def run_format(self, *args: str, entrypoint: str = "scripts/dart/format.sh") -> int:
        """Invoke the checked-in shell script, not a Python reimplementation."""
        return subprocess.run(
            ["bash", entrypoint, *args], cwd=self.root, env=self.env,
            capture_output=True, text=True, timeout=10, check=False,
        ).returncode

    def commands(self) -> list[list[object]]:
        """Read only SDK operations from the disposable workspace."""
        path = self.root / "commands.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def test_cold_consumer_resolves_before_format_in_both_modes(self) -> None:
        for args in ((), ("--check",)):
            with self.subTest(args=args):
                (self.root / CONSUMER / ".dart_tool/package_config.json").unlink(missing_ok=True)
                self.assertEqual(self.run_format(*args), 0)
                commands = [item for item in self.commands() if item[0] == CONSUMER]
                self.assertEqual(commands[-2], [CONSUMER, ["pub", "get"]])
                self.assertEqual(commands[-1][1][0], "format")
                if args:
                    self.assertIn("--set-exit-if-changed", commands[-1][1])

    def test_pub_failure_stops_consumer_formatting(self) -> None:
        self.env["DUST_FORMAT_TEST_PUB_EXIT"] = "31"
        self.assertEqual(self.run_format("--check"), 31)
        self.assertEqual([item for item in self.commands() if item[0] == CONSUMER],
                         [[CONSUMER, ["pub", "get"]]])

    def test_formatter_failure_is_not_swallowed(self) -> None:
        self.env["DUST_FORMAT_TEST_FORMAT_EXIT"] = "32"
        self.assertNotEqual(self.run_format("--check"), 0)

    def test_example_scope_does_not_prepare_consumer(self) -> None:
        self.assertEqual(self.run_format("--check", "--examples"), 0)
        commands = self.commands()
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][0], "examples/product_showcase")
        self.assertEqual(commands[0][1][0], "format")

    def test_rust_scope_does_not_invoke_dart(self) -> None:
        self.assertEqual(self.run_format("--check", "--scope=rust",
                                         entrypoint="scripts/format.sh"), 0)
        self.assertEqual(self.commands(), [])


if __name__ == "__main__":
    unittest.main()

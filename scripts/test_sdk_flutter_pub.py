"""Offline checks for preserving repository analysis rules around Flutter Pub."""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import strict_generated_analyze as strict


WRAPPER = Path(__file__).with_name("flutter_pub_get.py")


class FlutterPubTests(unittest.TestCase):
    """Exercise the wrapper CLI in real temporary Git roots and nested members."""

    def run_wrapper(
        self, root: Path, exit_code: int, *, delete_policy: bool = False,
        cwd: Path | None = None,
    ):
        """Model only Flutter's file mutation; Git discovery and CLI exits are real."""
        binary = root / "fake-bin/flutter"
        binary.parent.mkdir()
        binary.write_text(
            f"#!{sys.executable}\n"
            "import os, sys\n"
            "from pathlib import Path\n"
            "assert sys.argv[1:] == ['pub', 'get']\n"
            "root = Path(os.environ['DUST_PUB_TEST_ROOT'])\n"
            "for policy in root.rglob('analysis_options.yaml'):\n"
            "    policy.write_bytes(b'analyzer: {exclude: [build/**]}\\n')\n"
            "if os.environ['DUST_PUB_TEST_DELETE'] == '1':\n"
            "    (root / 'analysis_options.yaml').unlink()\n"
            "sys.exit(int(os.environ['DUST_PUB_TEST_EXIT']))\n",
            encoding="utf-8",
        )
        binary.chmod(0o700)
        env = {
            **os.environ,
            "PATH": str(binary.parent) + os.pathsep + os.environ["PATH"],
            "DUST_PUB_TEST_ROOT": str(root),
            "DUST_PUB_TEST_EXIT": str(exit_code),
            "DUST_PUB_TEST_DELETE": "1" if delete_policy else "0",
        }
        return subprocess.run(
            [sys.executable, str(WRAPPER)], cwd=cwd or root / "packages/member",
            env=env, capture_output=True, check=False,
        )

    def fixture(self, root: Path) -> dict[Path, bytes]:
        """Keep staged baselines distinct from existing user-edited policy bytes."""
        subprocess.run(["git", "init", "--quiet", str(root)], check=True)
        (root / ".gitignore").write_text("ignored/\n", encoding="utf-8")
        policies = {
            root / "analysis_options.yaml": b"include: original.yaml\n",
            root / "packages/member/analysis_options.yaml": b"# member\r\n",
        }
        for path, contents in policies.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(contents)
        subprocess.run(["git", "-C", str(root), "add", "."], check=True)
        policies[root / "analysis_options.yaml"] += b"# existing user edit\n"
        policies[root / "untracked member/analysis_options.yaml"] = "# café\n".encode()
        policies[root / "empty/analysis_options.yaml"] = b""
        for path, contents in policies.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(contents)
        ignored = root / "ignored/analysis_options.yaml"
        ignored.parent.mkdir()
        ignored.write_bytes(b"# ignored build artifact\n")
        return policies

    def test_root_members_and_untracked_user_edits_survive_success_and_failure(self):
        for exit_code in (0, 23):
            with self.subTest(exit_code=exit_code), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                policies = self.fixture(root)
                result = self.run_wrapper(root, exit_code, cwd=root if exit_code == 0 else None)
                self.assertEqual(result.returncode, exit_code, result.stderr.decode())
                for path, contents in policies.items():
                    self.assertEqual(path.read_bytes(), contents, path)
                self.assertEqual(subprocess.check_output([
                    "git", "-C", str(root), "show", ":analysis_options.yaml",
                ]), b"include: original.yaml\n")
                self.assertEqual((root / "ignored/analysis_options.yaml").read_bytes(),
                                 b"analyzer: {exclude: [build/**]}\n")

    def test_failed_pub_restores_an_existing_policy_even_if_it_was_removed(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            policies = self.fixture(root)
            result = self.run_wrapper(root, 37, delete_policy=True)
            self.assertEqual(result.returncode, 37, result.stderr.decode())
            for path, contents in policies.items():
                self.assertEqual(path.read_bytes(), contents, path)

    def test_strict_analyzer_sees_original_repository_and_temporary_policies(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name).resolve()
            policies = self.fixture(root)
            package = root / "fixtures/app"
            (package / "lib").mkdir(parents=True)
            (package / "pubspec.yaml").write_text("name: app\n")
            policy = package / "analysis_options.yaml"
            policies[policy] = b"include: ../../analysis_options.yaml\n# app policy\n"
            policy.write_bytes(policies[policy])
            (package / "lib/model.g.dart").write_text(
                f"{strict.GENERATED_MARKER}\n{strict.TYPE_LINT_IGNORE}\nclass Model {{}}\n"
            )
            temporary = {}
            calls = []

            def command(args, cwd):
                calls.append(args)
                paths = (cwd.parents[1] / "analysis_options.yaml", cwd / "analysis_options.yaml")
                if args == ["flutter", "pub", "get"]:
                    temporary.update({path: path.read_bytes() for path in paths})
                    for path in {*policies, *paths}:
                        path.write_bytes(b"analyzer: {exclude: [build/**]}\n")
                    return
                self.assertEqual(args, ["dart", "analyze", "--fatal-warnings"])
                for path, contents in {**policies, **temporary}.items():
                    self.assertEqual(path.read_bytes(), contents, path)
                self.assertNotIn(strict.TYPE_LINT_IGNORE,
                                 (cwd / "lib/model.g.dart").read_text())

            with patch.object(strict, "repo_root", return_value=root), \
                    patch.object(strict, "run_checked", side_effect=command), \
                    patch.object(sys, "argv", [
                        "strict_generated_analyze.py", "--package", "fixtures/app",
                        "--tool", "flutter",
                    ]):
                self.assertEqual(strict.main(), 0)
            self.assertEqual(calls, [
                ["flutter", "pub", "get"], ["dart", "analyze", "--fatal-warnings"],
            ])


if __name__ == "__main__":
    unittest.main()

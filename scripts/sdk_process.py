"""Logged subprocess execution for local SDK compatibility probes."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def sdk_environment(dart: Path, flutter_root: Path | None = None) -> dict[str, str]:
    """Bind direct Dart commands to this SDK row, never an inherited Flutter root."""
    env = os.environ.copy()
    inherited_path = env.get("PATH", "")
    env["PATH"] = str(dart.parent) + (os.pathsep + inherited_path if inherited_path else "")
    if flutter_root is None:
        env.pop("FLUTTER_ROOT", None)
    else:
        env["FLUTTER_ROOT"] = str(flutter_root)
    return env


def run_command(command: list[str], cwd: Path, log: Path, expected: int = 0,
                env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    """Persist both streams; unexpected success also fails negative probes."""
    result = subprocess.run(command, cwd=cwd, env=env, text=True,
                            capture_output=True, timeout=900)
    log.write_text(result.stdout + result.stderr, encoding="utf-8")
    if result.returncode != expected:
        raise RuntimeError(
            f"Expected exit {expected}, got {result.returncode}: {command}; see {log}")
    return result

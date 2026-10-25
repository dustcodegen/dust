"""Logged subprocess execution for local SDK compatibility probes."""

from __future__ import annotations

import subprocess
from pathlib import Path


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

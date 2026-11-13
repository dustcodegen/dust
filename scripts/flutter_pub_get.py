#!/usr/bin/env python3
"""Run Flutter Pub without retaining SDK migrations of repository analysis rules."""

from __future__ import annotations

import os
import subprocess
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path


@contextmanager
def preserve_analysis_options(
    root: Path, additional_paths: tuple[Path, ...] = (),
) -> Iterator[None]:
    """Preserve repository and explicit temporary policies through SDK migrations."""
    listed = subprocess.check_output([
        "git", "-C", str(root), "ls-files", "--cached", "--others",
        "--exclude-standard", "-z", "--", "analysis_options.yaml",
        "**/analysis_options.yaml",
    ])
    paths = {root / os.fsdecode(name) for name in listed.split(b"\0") if name}
    paths.update(additional_paths)
    policies = {path: path.read_bytes() for path in paths if path.is_file()}
    try:
        yield
    finally:
        for path, contents in policies.items():
            if not path.is_file() or path.read_bytes() != contents:
                path.write_bytes(contents)


def run_pub_get() -> int:
    """Restore exact existing policies, including local edits, even if Pub fails."""
    root = Path(subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True,
    ).strip())
    with preserve_analysis_options(root):
        return subprocess.run(["flutter", "pub", "get"], check=False).returncode


if __name__ == "__main__":
    raise SystemExit(run_pub_get())

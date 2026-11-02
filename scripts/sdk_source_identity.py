"""Freeze the source and CLI identity used by an SDK compatibility run."""

from __future__ import annotations

import hashlib
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class SourceIdentity:
    """One exact commit, non-ignored source tree and CLI binary."""

    commit: str
    status: bytes
    source_sha256: str
    dust_sha256: str

    def evidence(self) -> dict[str, object]:
        """Return the stable fields persisted in the compatibility proof."""
        return {
            "sourceCommit": self.commit,
            "sourceDirty": bool(self.status),
            "sourceSha256": self.source_sha256,
            "dustSha256": self.dust_sha256,
        }


def capture_source_identity(root: Path, dust: Path) -> SourceIdentity:
    """Hash tracked and non-ignored untracked files; ignored build output stays external."""
    commit = _git(root, "rev-parse", "HEAD").decode().strip()
    status = _git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    paths = _git(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard")
    source = hashlib.sha256()
    for raw_path in paths.split(b"\0"):
        if not raw_path:
            continue
        source.update(len(raw_path).to_bytes(8, "big"))
        source.update(raw_path)
        path = root / os.fsdecode(raw_path)
        if path.is_symlink():
            source.update(b"link\0" + os.fsencode(os.readlink(path)))
        elif path.is_file():
            source.update(b"file\0" + path.read_bytes())
        else:
            source.update(b"missing\0")
    return SourceIdentity(
        commit=commit,
        status=status,
        source_sha256=source.hexdigest(),
        dust_sha256=hashlib.sha256(dust.read_bytes()).hexdigest(),
    )


class FrozenSourceIdentity:
    """Fail permanently after any checkpoint differs from the initial identity."""

    def __init__(self, root: Path, dust: Path) -> None:
        self.root = root
        self.dust = dust
        self.identity = capture_source_identity(root, dust)
        self._changed = False

    def verify(self) -> None:
        """Reject a changed or previously changed source/CLI pair."""
        changed = capture_source_identity(self.root, self.dust) != self.identity
        self._changed = self._changed or changed
        if self._changed:
            raise ValueError("Dust source, commit, or CLI changed during the SDK matrix")


def invalidate_proof(path: Path) -> None:
    """Remove earlier success evidence before a new matrix can fail."""
    path.unlink(missing_ok=True)


def verify_frozen(frozen: FrozenSourceIdentity | None) -> None:
    """Verify a frozen identity after it has been established."""
    if frozen is not None:
        frozen.verify()


def _git(root: Path, *arguments: str) -> bytes:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return result.stdout

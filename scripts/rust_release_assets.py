"""Validate the complete Rust release set before and after uploading it."""

import argparse
import hashlib
import tarfile
import zipfile
from pathlib import Path

ASSETS = (
    "dust-aarch64-apple-darwin.tar.gz",
    "dust-aarch64-pc-windows-msvc.zip",
    "dust-aarch64-unknown-linux-gnu.tar.gz",
    "dust-x86_64-apple-darwin.tar.gz",
    "dust-x86_64-pc-windows-msvc.zip",
    "dust-x86_64-unknown-linux-gnu.tar.gz",
)
MANIFEST = "SHA256SUMS.txt"


def verify(directory: Path, *, prepare: bool = False) -> None:
    """Reject incomplete, unexpected, malformed or checksum-mismatched assets."""
    expected = set(ASSETS) | (set() if prepare else {MANIFEST})
    if {entry.name for entry in directory.iterdir()} != expected:
        raise ValueError("Release must contain exactly six archives and, when verifying, SHA256SUMS.txt")
    lines = []
    for name in ASSETS:
        archive = directory / name
        if not archive.is_file() or archive.is_symlink():
            raise ValueError(f"Not a regular archive: {name}")
        if name.endswith(".zip"):
            with zipfile.ZipFile(archive) as handle:
                entries = handle.infolist()
                if len(entries) != 1 or entries[0].filename != "dust.exe" or entries[0].is_dir():
                    raise ValueError(f"Unexpected archive contents: {name}")
                if (entries[0].external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError(f"Symlink in archive: {name}")
                if not handle.read(entries[0]):
                    raise ValueError(f"Empty binary: {name}")
        else:
            with tarfile.open(archive, "r:gz") as handle:
                entries = handle.getmembers()
                if len(entries) != 1 or entries[0].name != "dust" or not entries[0].isfile():
                    raise ValueError(f"Unexpected archive contents: {name}")
                if entries[0].size == 0 or not entries[0].mode & 0o111:
                    raise ValueError(f"Empty or non-executable binary: {name}")
                if not handle.extractfile(entries[0]).read():
                    raise ValueError(f"Empty binary: {name}")
        with archive.open("rb") as handle:
            digest = hashlib.file_digest(handle, "sha256").hexdigest()
        lines.append(f"{digest}  {name}\n")
    contents = "".join(lines)
    manifest = directory / MANIFEST
    if prepare:
        manifest.write_text(contents, encoding="ascii")
    elif manifest.is_symlink() or manifest.read_text(encoding="ascii") != contents:
        raise ValueError("Release checksum manifest does not match the archives")
    print("Verified six Rust archives and SHA-256 checksums")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    verify(args.directory, prepare=args.prepare)

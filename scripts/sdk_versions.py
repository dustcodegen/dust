"""Version and lint policy for independent minimum/latest SDK consumers."""

from __future__ import annotations

import re
from pathlib import Path

from sqlite_sdk_consumer import ROOT, compatible_lints


def stable_dart_version(output: str) -> str:
    """Reject prerelease or unrecognized SDK output rather than claiming a floor pass."""
    match = re.match(r"Dart SDK version: (\d+\.\d+\.\d+) \(stable\)", output)
    if not match:
        raise ValueError(f"Expected stable Dart, got: {output}")
    return match[1]


def latest_flutter_version(value: object) -> str:
    """Read only Puro's refreshed stable listing, never the beta section."""
    if not isinstance(value, dict) or value.get("success") is not True:
        raise ValueError("Unable to refresh Flutter releases")
    messages = value.get("messages")
    if not isinstance(messages, list):
        raise ValueError("Refreshed Flutter listing has no messages")
    for message in messages:
        if isinstance(message, dict) and isinstance(message.get("message"), str):
            match = re.match(r"Latest stable releases:\nFlutter (\d+\.\d+\.\d+)\s", message["message"])
            if match:
                return match[1]
    raise ValueError("Refreshed Flutter listing has no stable release")


def core_floor(root: Path = ROOT) -> str:
    """Use the declared runtime floor, not a nearby installed SDK."""
    manifest = (root / "packages/dust_dart/pubspec.yaml").read_text(encoding="utf-8")
    match = re.search(r'sdk:\s*["\']?>=(\d+\.\d+\.\d+)\s+<4\.0\.0', manifest)
    if not match:
        raise ValueError("Unrecognized core SDK constraint; review the minimum SDK gate")
    return match[1]


def require_version(actual: str, expected: str) -> None:
    """A nearby SDK is not a passing minimum or latest execution target."""
    if actual != expected:
        raise ValueError(f"Expected SDK {expected}, got {actual}; supply the matching SDK")


def core_lints(version: str) -> str:
    """Vary dev lint tooling only, keeping enabled analysis rules intact."""
    actual = stable_dart_version(version)
    if tuple(map(int, actual.split("."))) < (3, 6, 0):
        raise ValueError("Core Dust consumers require Dart >=3.6.0")
    return "7.0.0" if tuple(map(int, actual.split("."))) < (3, 10, 0) else compatible_lints(version)

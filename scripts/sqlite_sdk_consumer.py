#!/usr/bin/env python3
"""Run independent SQLite consumers locally; never select an SDK or run CI."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def stage_consumer(
    root: Path, output: Path, relative: str, lint_version: str = "10.3.0"
) -> Path:
    """Copy consumer source without its lock, workspace or cached resolution."""
    target = output / relative
    target.mkdir(parents=True)
    source = root / relative
    for name in (
        "bin", "lib", "test", "assets", "migrations", "integration_test", "test_driver"
    ):
        if (source / name).is_dir():
            shutil.copytree(source / name, target / name)
    for name in ("pubspec.yaml", "analysis_options.yaml"):
        shutil.copy2(source / name, target / name)
    packages = ("dust_dart", "dust_db_sqlite3")
    if relative.startswith("examples/"):
        packages += ("dust_flutter",)
    (output / "packages").mkdir(exist_ok=True)
    for package in packages:
        link = output / "packages" / package
        if not link.exists():
            link.symlink_to(root / "packages" / package, target_is_directory=True)
    overrides: dict[str, object] = {
        package: {"path": str(root / "packages" / package)} for package in packages
    }
    overrides["sqlite3"] = "3.5.2"
    # Only lint tooling varies with the SDK, never production constraints.
    overrides["very_good_analysis"] = lint_version
    (target / "pubspec_overrides.yaml").write_text(
        json.dumps({"dependency_overrides": overrides}, indent=2) + "\n",
        encoding="utf-8",
    )
    return target


def compatible_lints(version: str) -> str:
    """Use the floor analyzer's lint release, not an SDK-incompatible override."""
    match = re.search(r"Dart SDK version: (\d+)\.(\d+)\.", version)
    if not match:
        raise ValueError(f"Unrecognized Dart version: {version}")
    major, minor = map(int, match.groups())
    if (major, minor) < (3, 10):
        raise ValueError("SQLite requires Dart >=3.10.0")
    return "10.1.0" if (major, minor) < (3, 12) else "10.3.0"


def validate_sdk_pair(dart_version: str, flutter: dict[str, object]) -> None:
    """Do not accidentally attribute two different compilers to one SDK run."""
    version = flutter.get("dartSdkVersion")
    parts = version.split() if isinstance(version, str) else []
    if not parts or not dart_version.startswith(f"Dart SDK version: {parts[0]} "):
        raise ValueError("Explicit Dart binary does not match the Flutter SDK")


def validate_proof(value: object) -> list[dict[str, object]]:
    """Reject incomplete CLI output; ordinary exceptions remain failures in AOT."""
    if not isinstance(value, list) or len(value) != 2:
        raise ValueError("Expected sync and worker proof")
    for entry, mode in zip(value, ("sync", "worker")):
        if not isinstance(entry, dict) or entry.get("mode") != mode:
            raise ValueError(f"Missing {mode} proof")
        if entry.get("rows") != 1 or entry.get("integrity") != "ok":
            raise ValueError(f"Invalid {mode} data/integrity proof")
        for key in ("sqliteVersion", "sqliteSourceId", "migrationChecksum"):
            if not isinstance(entry.get(key), str) or not entry[key]:
                raise ValueError(f"Missing {mode} {key}")
    for key in ("sqliteVersion", "sqliteSourceId", "migrationChecksum"):
        if value[0][key] != value[1][key]:
            raise ValueError(f"Sync/worker disagree on {key}")
    return value


def parse_jit_proof(output: str) -> object:
    """Dart 3.10 emits hook progress to stdout despite error-only verbosity."""
    return json.loads(re.sub(r"^(?:Running build hooks\.\.\.)+", "", output))


def native_assets(bundle: Path) -> dict[str, str]:
    """Record the shipped native bytes rather than only the package version."""
    assets = {
        path.relative_to(bundle).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted((bundle / "lib").rglob("*"))
        if path.is_file()
    }
    if not assets:
        raise ValueError("Native bundle has no library assets")
    return assets


def compare_proofs(jit: object, aot: object) -> None:
    """Compare portable facts, not timestamps from separate owned databases."""
    left, right = validate_proof(jit), validate_proof(aot)
    for before, after in zip(left, right):
        for key in (
            "mode", "rows", "integrity", "sqliteVersion", "sqliteSourceId",
            "migrationChecksum"
        ):
            if before[key] != after[key]:
                raise ValueError(f"JIT/AOT disagree on {key}")


def run_matrix(dart: Path, flutter_env: str | None, output: Path) -> None:
    """Analyze/test JIT and relocated native bundles in one isolated SDK run."""
    shutil.copy2(ROOT / "analysis_options.yaml", output / "analysis_options.yaml")
    log_number = 0

    def run(command: list[str], cwd: Path) -> str:
        nonlocal log_number
        log_number += 1
        print(f"==> {' '.join(command)} ({cwd})", flush=True)
        result = subprocess.run(
            command, cwd=cwd, text=True, capture_output=True, timeout=900,
            check=False,
        )
        (output / f"{log_number:02d}.log").write_text(
            result.stdout + result.stderr, encoding="utf-8"
        )
        if command[-2:] == ["deps", "--json"]:
            print(f"Dependency resolution saved to {log_number:02d}.log", flush=True)
        else:
            print(result.stdout + result.stderr, end="", flush=True)
        result.check_returncode()
        return result.stdout

    dart_version = run([str(dart), "--version"], output)
    lint_version = compatible_lints(dart_version)
    flutter = (
        ["puro", "--no-update-check", "-e", flutter_env, "flutter"]
        if flutter_env else None
    )
    if flutter:
        metadata = json.loads(run([*flutter, "--version", "--machine"], output))
        validate_sdk_pair(dart_version, metadata)
    consumer = stage_consumer(
        ROOT, output, "fixtures/sqlite_sdk_consumer", lint_version
    )
    run([str(dart), "pub", "get"], consumer)
    run([str(dart), "pub", "deps", "--json"], consumer)
    run([
        str(dart), "format", "--output=none", "--set-exit-if-changed",
        "bin", "lib", "test",
    ], consumer)
    run([str(dart), "analyze", "--fatal-infos"], consumer)
    run([str(dart), "test"], consumer)
    jit = parse_jit_proof(run([
        str(dart), "run", "--verbosity=error", "bin/sqlite_sdk_consumer.dart",
    ], consumer))
    validate_proof(jit)
    build = output / "native-build"
    run([
        str(dart), "build", "cli", f"--output={build}",
        "--target=bin/sqlite_sdk_consumer.dart",
    ], consumer)
    bundle = output / "relocated" / "bundle"
    shutil.move(str(build / "bundle"), bundle)
    # A cached absolute hook path into the source tree must not rescue the AOT run.
    consumer.rename(consumer.with_name("source-after-build"))
    executable = bundle / "bin" / "sqlite_sdk_consumer"
    if not executable.is_file():
        executable = executable.with_suffix(".exe")
    aot = json.loads(run([str(executable)], output))
    compare_proofs(jit, aot)
    (output / "proof.json").write_text(
        json.dumps({
            "jit": jit, "relocatedAot": aot,
            "nativeAssetsSha256": native_assets(bundle),
        }, indent=2) + "\n",
        encoding="utf-8",
    )
    if flutter:
        app = stage_consumer(ROOT, output, "examples/shopping_app", lint_version)
        run([*flutter, "pub", "get"], app)
        # Newer Flutter may migrate options; analyze the original, unchanged rules.
        shutil.copy2(
            ROOT / "examples/shopping_app/analysis_options.yaml",
            app / "analysis_options.yaml",
        )
        run([*flutter, "pub", "deps", "--json"], app)
        run([*flutter, "analyze", "--no-pub", "--fatal-infos"], app)
        run([*flutter, "test", "--no-pub"], app)
    print(f"Passed; local evidence retained in {output}", flush=True)


def main() -> None:
    """Require an explicit Dart binary; optional Puro env never changes defaults."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dart", required=True, type=Path)
    parser.add_argument("--flutter-env")
    args = parser.parse_args()
    dart = args.dart.resolve(strict=True)
    output = Path(tempfile.mkdtemp(prefix="dust-sdk-consumer-"))
    print(f"Local evidence: {output}", flush=True)
    run_matrix(dart, args.flutter_env, output)


if __name__ == "__main__":
    main()

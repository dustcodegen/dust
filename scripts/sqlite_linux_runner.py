#!/usr/bin/env python3
"""Run unchanged SQLite sources in Linux-owned staging, never remote CI."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from sqlite_sdk_consumer import ROOT, run_matrix


def require_fixed_sqlite(version: str) -> None:
    """Enforce upstream's WAL-reset fixed releases; vendor backports need review."""
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError(f"Unrecognized SQLite version: {version}")
    major, minor, patch = map(int, version.split("."))
    if (major, minor, patch) >= (3, 51, 3):
        return
    if (major, minor) == (3, 44) and patch >= 6:
        return
    if (major, minor) == (3, 50) and patch >= 7:
        return
    raise ValueError(f"SQLite {version} requires verified WAL-reset backport evidence")


def select_library(sdk: Path, proof: dict[str, object]) -> Path:
    """Reject a changed/missing library or a link outside the native bundle."""
    assets = proof.get("nativeAssetsSha256")
    relative = "lib/libsqlite3.so"
    expected = assets.get(relative) if isinstance(assets, dict) else None
    if not isinstance(expected, str):
        raise ValueError("Missing bundled Linux SQLite checksum")
    bundle = sdk / "relocated/bundle"
    library = (bundle / relative).resolve()
    library.relative_to(bundle.resolve())
    if not library.is_file() or hashlib.sha256(library.read_bytes()).hexdigest() != expected:
        raise ValueError("Bundled Linux SQLite bytes changed")
    return library


def stage_runtime(root: Path, workspace: Path, library: Path | None) -> Path:
    """Keep actual package manifests/source; only the test workspace is smaller."""
    workspace.mkdir(parents=True)
    shutil.copy2(root / "analysis_options.yaml", workspace / "analysis_options.yaml")
    packages = ["packages/dust_dart", "packages/dust_db_sqlite3"]
    for package in packages:
        source, target = root / package, workspace / package
        target.mkdir(parents=True)
        for name in ("pubspec.yaml", "analysis_options.yaml"):
            if (source / name).is_file():
                shutil.copy2(source / name, target / name)
        for name in ("lib", "test", "integration_test", "example"):
            if (source / name).is_dir():
                shutil.copytree(source / name, target / name)
    (workspace / "pubspec.yaml").write_text(json.dumps({
        "name": "dust_sqlite_linux_validation", "publish_to": "none",
        "environment": {"sdk": ">=3.10.0 <4.0.0"}, "workspace": packages,
        "dependency_overrides": {"sqlite3": "3.5.2"},
    }, indent=2) + "\n", encoding="utf-8")
    configure_runtime_engine(workspace, library)
    return workspace / "packages/dust_db_sqlite3"


def configure_runtime_engine(workspace: Path, library: Path | None) -> None:
    """Change only owned workspace metadata, never the source package manifests."""
    path = workspace / "pubspec.yaml"
    pubspec = json.loads(path.read_text(encoding="utf-8"))
    engine = {"source": "sqlite3"} if library is None else {
        "source": "system", "name": str(library),
    }
    pubspec["hooks"] = {"user_defines": {"sqlite3": engine}}
    path.write_text(json.dumps(pubspec, indent=2) + "\n", encoding="utf-8")


def system_library(output: str) -> Path:
    """Select the actual SONAME file owned by the signed distribution package."""
    paths = [Path(line) for line in output.splitlines()
             if Path(line).is_absolute() and Path(line).name == "libsqlite3.so.0"]
    if len(paths) != 1:
        raise ValueError("Cannot identify the system SQLite shared library")
    library = paths[0].resolve(strict=True)
    if not library.is_file():
        raise ValueError("System SQLite is not a shared-library file")
    return library


def runtime_identity(output: str) -> dict[str, str]:
    """Attribute the full-API engine through the actual Dart connection pairs."""
    observed = re.findall(r"^native SQLite: (.+)$", output, flags=re.MULTILINE)
    if len(observed) != 4 or len(set(observed)) != 1 or " / " not in observed[0]:
        raise ValueError("Missing or inconsistent Dart runtime engine identity")
    version, source = observed[0].split(" / ", 1)
    require_fixed_sqlite(version)
    if not source:
        raise ValueError("Missing runtime source ID")
    return {"sqliteVersion": version, "sqliteSourceId": source}


def validate_runtime_engine(output: str, proof: dict[str, object]) -> None:
    """Require both connections in every sync/worker pair to match the bundle."""
    observed = re.findall(r"^native SQLite: (.+)$", output, flags=re.MULTILINE)
    expected = f"{proof['sqliteVersion']} / {proof['sqliteSourceId']}"
    if observed != [expected] * 4:
        raise ValueError(f"Runtime engine attribution failed: {observed}")


def run(evidence: Path) -> None:
    """Run portable tests plus the Linux-supported native VFS fault matrix."""
    if platform.system() != "Linux" or platform.machine() not in ("aarch64", "arm64"):
        raise ValueError("This pinned local gate requires Linux arm64")
    evidence.mkdir()
    staging = Path(tempfile.mkdtemp(prefix="dust-linux-"))
    if shutil.disk_usage(staging).free < 512 * 1024 * 1024:
        raise ValueError("At least 512 MiB container staging headroom is required")
    sdk = staging / "sdk"
    sdk.mkdir()
    workspace = staging / "runtime"
    command_number = 0

    def command(args: list[str], cwd: Path, env: dict[str, str] | None = None) -> str:
        nonlocal command_number
        command_number += 1
        print(f"==> {' '.join(args)} ({cwd})", flush=True)
        result = subprocess.run(
            args, cwd=cwd, env=env, capture_output=True, text=True,
            timeout=900, check=False,
        )
        (evidence / f"runtime-{command_number:02d}.log").write_text(
            result.stdout + result.stderr, encoding="utf-8"
        )
        print(result.stdout + result.stderr, end="", flush=True)
        result.check_returncode()
        return result.stdout

    try:
        command(["uname", "-a"], staging)
        shutil.copy2("/etc/os-release", evidence / "os-release")
        command(["dpkg-query", "-W", "python3", "clang", "libsqlite3-0", "libsqlite3-dev", "binutils"], staging)
        dart = shutil.which("dart")
        if dart is None:
            raise ValueError("Dart executable is missing")
        run_matrix(Path(dart), None, sdk)
        proof = json.loads((sdk / "proof.json").read_text(encoding="utf-8"))
        require_fixed_sqlite(proof["jit"][0]["sqliteVersion"])
        library = select_library(sdk, proof)
        command(["readelf", "-d", str(library)], staging)
        shared = system_library(command(["dpkg-query", "-L", "libsqlite3-0"], staging))
        native = None
        runtime = stage_runtime(ROOT, workspace, None)
        coverage = staging / "coverage"
        for mode, selected in (("bundled", None), ("system", shared)):
            configure_runtime_engine(workspace, selected)
            command([dart, "pub", "get"], runtime)
            command([dart, "pub", "deps", "--json"], runtime)
            command([dart, "analyze", "--fatal-infos"], runtime)
            tests = [dart, "test", "test"]
            if mode == "system":
                tests.append("integration_test/native_storage_fault_test.dart")
            output = command([
                *tests, f"--coverage={coverage / mode}", "--reporter=expanded",
            ], runtime)
            if mode == "bundled":
                validate_runtime_engine(output, proof["jit"][0])
            else:
                native = runtime_identity(output)
                native["librarySha256"] = hashlib.sha256(shared.read_bytes()).hexdigest()
        report = evidence / "runtime.lcov"
        command([
            dart, "run", "coverage:format_coverage", "--lcov", f"--in={coverage}",
            f"--out={report}", "--report-on=lib",
        ], runtime)
        command(["python3", str(ROOT / "scripts/check_lcov_floor.py"), str(report), "100"], runtime)
        shutil.copytree(coverage, evidence / "raw-coverage")
        shutil.copytree(sdk / "relocated/bundle", evidence / "consumer-bundle")
        shutil.copy2(sdk / "proof.json", evidence / "consumer-proof.json")
        (evidence / "result.json").write_text(json.dumps({
            "status": "passed", "os": platform.system(), "arch": platform.machine(),
            "bundledEngine": proof["jit"][0], "faultTestEngine": native,
            "nativeAssetsSha256": proof["nativeAssetsSha256"],
            "scope": "portable SQLite on both engines; native I/O on full-API system build only",
        }, indent=2) + "\n", encoding="utf-8")
    finally:
        for log in sdk.glob("*.log"):
            shutil.copy2(log, evidence / f"consumer-{log.name}")
        for source, name in (
            (workspace / "pubspec.yaml", "runtime-workspace.yaml"),
            (workspace / "pubspec.lock", "runtime-pubspec.lock"),
            (sdk / "fixtures/source-after-build/pubspec.lock", "consumer-pubspec.lock"),
        ):
            if source.is_file():
                shutil.copy2(source, evidence / name)
    print(f"Linux runtime/coverage passed; evidence retained in {evidence}", flush=True)


def main() -> None:
    """Only create a fresh result directory under the caller's evidence mount."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", required=True, type=Path)
    run(parser.parse_args().evidence)


if __name__ == "__main__":
    main()

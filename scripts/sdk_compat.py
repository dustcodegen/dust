#!/usr/bin/env python3
"""Check fresh Dust generation on explicit minimum/latest local Dart/Flutter SDKs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import urllib.request
from pathlib import Path

from sqlite_sdk_consumer import ROOT, compatible_lints, validate_sdk_pair
from sdk_codegen_fixtures import generated_files, generated_hashes, stage_example
from sdk_generic_types import check_generic_types
from sdk_generic_analysis import check_generic_analysis
from sdk_generic_names import check_generic_names


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


def require_deterministic_generation(rows: list[dict[str, object]]) -> None:
    """The same supported source must produce identical bytes on both SDK ends."""
    for target in ("dart", "flutter"):
        generated = [row["generated"] for row in rows if row["target"] == target]
        if len(generated) != 2 or generated[0] != generated[1]:
            raise ValueError(f"Missing or different minimum/latest {target} generated output")


def run_command(command: list[str], cwd: Path, log: Path, expected: int = 0,
                env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    """Persist both streams; unexpected success is also a failure for negative probes."""
    result = subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True, timeout=900)
    log.write_text(result.stdout + result.stderr, encoding="utf-8")
    if result.returncode != expected:
        raise RuntimeError(f"Expected exit {expected}, got {result.returncode}: {command}; see {log}")
    return result


def run_matrix(args: argparse.Namespace, output: Path) -> None:
    """Execute independent consumers without changing selected SDKs or production locks."""
    log_number = 0

    def run(command: list[str], cwd: Path = ROOT, expected: int = 0,
            env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        nonlocal log_number
        log_number += 1
        print(f"==> {' '.join(command)} ({cwd})", flush=True)
        return run_command(command, cwd, output / f"{log_number:02d}.log", expected, env)

    with urllib.request.urlopen(
        "https://storage.googleapis.com/dart-archive/channels/stable/release/latest/VERSION",
        timeout=30,
    ) as response:
        latest_dart = json.load(response)
    refreshed = json.loads(run(["puro", "--no-update-check", "--json", "ls-versions", "--force"]).stdout)
    latest_flutter = latest_flutter_version(refreshed)
    (output / "latest-metadata.json").write_text(json.dumps({
        "dart": latest_dart, "flutterListing": refreshed,
    }, indent=2) + "\n", encoding="utf-8")
    run(["cargo", "build", "--quiet", "-p", "dust_cli"])
    cargo = json.loads(run(["cargo", "metadata", "--no-deps", "--format-version=1"]).stdout)
    dust = Path(cargo["target_directory"]) / "debug" / ("dust.exe" if os.name == "nt" else "dust")
    rows = []
    for role in ("minimum", "latest"):
        dart = getattr(args, f"{role}_dart").resolve(strict=True)
        version_text = run([str(dart), "--version"]).stdout.strip()
        version = stable_dart_version(version_text)
        require_version(version, core_floor() if role == "minimum" else latest_dart["version"])
        app, required = stage_example(output / role / "dart", "examples/product_showcase", core_lints(version_text))
        env = {**os.environ, "PATH": str(dart.parent) + os.pathsep + os.environ.get("PATH", "")}
        run([str(dart), "pub", "get"], app, env=env)
        run([str(dart), "pub", "deps", "--json"], app, env=env)
        run([str(dust), "build", "--clean", "--root", str(app)], env=env)
        run([str(dust), "check", "--root", str(app)], env=env)
        hashes = generated_hashes(app)
        if not required.issubset(hashes):
            raise ValueError(f"Missing fresh Dart outputs: {required - hashes.keys()}")
        run([str(dart), "analyze", "--fatal-infos"], app, env=env)
        run([str(dart), "test", "--reporter=expanded"], app, env=env)
        check_generic_analysis(run, dart, app, output / role / "dart/analysis_options.yaml",
                               "lib/models", env)
        negatives = check_generic_types(run, dart, app, output / role / "generic-negative",
                                        "product_showcase/models", env)
        names = check_generic_names(run, dust, dart, app, output / role / "generic-names",
                                    output / role / "dart/analysis_options.yaml", "product_showcase", env)
        rows.append({"role": role, "target": "dart", "version": version,
                     "generated": hashes, "genericNegative": negatives, "genericNames": names})

        flutter = ["puro", "--no-update-check", "-e", getattr(args, f"{role}_flutter_env"), "flutter"]
        metadata = json.loads(run([*flutter, "--version", "--machine"]).stdout)
        # Flutter 3.27 pins meta 1.15, below Dust's existing meta ^1.16 constraint.
        require_version(metadata["frameworkVersion"], "3.29.0" if role == "minimum" else latest_flutter)
        flutter_dart = Path(metadata["flutterRoot"]) / "bin/cache/dart-sdk/bin" / ("dart.exe" if os.name == "nt" else "dart")
        flutter_dart_text = run([str(flutter_dart), "--version"]).stdout.strip()
        validate_sdk_pair(flutter_dart_text, metadata)
        if role == "minimum":
            require_version(stable_dart_version(flutter_dart_text), "3.7.0")
        env = {**os.environ, "PATH": str(flutter_dart.parent) + os.pathsep + os.environ.get("PATH", "")}
        app, required = stage_example(output / role / "flutter", "examples/benchmark_project", core_lints(flutter_dart_text))
        run([*flutter, "pub", "get"], app, env=env)
        shutil.copy2(ROOT / "examples/benchmark_project/analysis_options.yaml", app / "analysis_options.yaml")
        run([*flutter, "pub", "deps", "--json"], app, env=env)
        run([str(flutter_dart), "run", "tool/generate.dart", "--count", "64"], app, env=env)
        run([str(dust), "build", "--clean", "--root", str(app)], env=env)
        run([str(dust), "check", "--root", str(app)], env=env)
        hashes = generated_hashes(app)
        if not required.issubset(hashes):
            raise ValueError(f"Missing fresh Flutter outputs: {required - hashes.keys()}")
        run([*flutter, "analyze", "--no-pub", "--fatal-infos"], app, env=env)
        run([*flutter, "test", "--no-pub", "--reporter=expanded"], app, env=env)
        rows.append({"role": role, "target": "flutter", "sdk": metadata, "generated": hashes})

    require_deterministic_generation(rows)
    primary = {
        "dart": check_primary(run, dust, args.latest_dart.resolve(), output),
        "flutter": check_primary(run, dust, flutter_dart, output, flutter=flutter),
    }
    negatives = check_unsupported(run, dust, args.latest_dart.resolve(), output)
    proof = {
        "sourceCommit": run(["git", "rev-parse", "HEAD"]).stdout.strip(),
        "sourceDirty": bool(run(["git", "status", "--porcelain=v1"]).stdout),
        "dustSha256": hashlib.sha256(dust.read_bytes()).hexdigest(),
        "rows": rows, "primary": primary, "unsupported": negatives,
        "primaryGenericNames": json.loads((output / "primary-generic-names/results.json").read_text()),
    }
    (output / "proof.json").write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    print(f"SDK compatibility gate passed; evidence: {output}", flush=True)


def check_primary(run, dust: Path, dart: Path, output: Path,
                  *, flutter: list[str] | None = None) -> dict[str, str]:
    """Generate and execute primary constructors independently of traditional fixtures."""
    relative = "fixtures/primary_flutter_consumer" if flutter else "fixtures/primary_sdk_consumer"
    app, required = stage_example(output / ("primary-flutter" if flutter else "primary"), relative, "10.3.0")
    sources = sorted(path.relative_to(app).as_posix() for folder in ("lib", "test")
                     for path in (app / folder).rglob("*.dart"))
    env = {**os.environ, "PATH": str(dart.parent) + os.pathsep + os.environ.get("PATH", "")}
    sdk = flutter or [str(dart)]
    no_pub = ["--no-pub"] if flutter else []
    env.pop("DUST_DATABASE_URL", None)
    run([*sdk, "pub", "get"], app, env=env)
    shutil.copy2(ROOT / relative / "analysis_options.yaml", app / "analysis_options.yaml")
    run([*sdk, "pub", "deps", "--json"], app, env=env)
    run([str(dust), "build", "--clean", "--root", str(app)], env=env)
    run([str(dust), "check", "--root", str(app)], env=env)
    if not flutter:
        for arguments in (("db", "build"), ("check", "--db"), ("check", "--db", "--offline")):
            run([str(dust), *arguments, "--root", str(app)], env=env)
        for name in ("checked_queries", "database"):
            entries = [entry for path in (app / ".dust_sql").glob(f"{name}-*.json")
                       for entry in json.loads(path.read_text(encoding="utf-8"))["entries"]]
            if (sorted(entry["fetch_mode"] for entry in entries) !=
                    ["all", "execute", "one", "one", "optional"]
                    or any(entry["driver"] != "sqlite3" for entry in entries)):
                raise ValueError(f"Missing checked primary query metadata: {name}")
    hashes = generated_hashes(app)
    if flutter:
        required.update(("lib/counter.g.dart", "lib/generic_value.g.dart"))
        required.update(f"lib/route/{name}.g.dart" for name in (
            "routes", "paths", "navigation", "metadata", "runtime"))
    else:
        required.update(("lib/models.g.dart", "lib/private_models.g.dart",
                         "lib/language_models.g.dart", "lib/api.g.dart", "lib/database.g.dart",
                         "lib/generic_models.g.dart", "lib/generic_imported.g.dart"))
    if not required.issubset(hashes):
        raise ValueError(f"Missing fresh primary outputs: {required - hashes.keys()}")
    run([str(dart), "format", "--output=none", "--set-exit-if-changed", *sources], app, env=env)
    run([*sdk, "analyze", *no_pub, "--fatal-infos"], app, env=env)
    run([*sdk, "test", *no_pub, "--reporter=expanded"], app, env=env)
    if not flutter:
        check_generic_analysis(run, dart, app, output / "primary/analysis_options.yaml", "lib", env)
        check_generic_types(run, dart, app, output / "primary-generic-negative",
                            "primary_sdk_consumer", env)
        check_generic_names(run, dust, dart, app, output / "primary-generic-names",
                            output / "primary/analysis_options.yaml", "primary_sdk_consumer", env, primary=True)
    return hashes


def check_unsupported(run, dust: Path, dart: Path, output: Path) -> list[str]:
    """The latest SDK must accept each sample before Dust's rejection is counted."""
    app = output / "unsupported"
    (app / "lib").mkdir(parents=True)
    shutil.copy2(ROOT / "analysis_options.yaml", app / "analysis_options.yaml")
    (app / "pubspec.yaml").write_text(json.dumps({
        "name": "dust_unsupported_syntax_probe", "publish_to": "none",
        "environment": {"sdk": ">=3.13.0 <4.0.0"},
        "dependencies": {"dust_dart": {"path": str(ROOT / "packages/dust_dart")}},
        "dev_dependencies": {"very_good_analysis": "10.3.0"},
    }) + "\n", encoding="utf-8")
    run([str(dart), "pub", "get"], app)
    sources = {
        "named-concise": """class Probe {
  /// Creates a probe.
  const new create(this.name);
  /// Name.
  final String name;
}
""",
    }
    for name, declaration in sources.items():
        (app / "lib/probe.dart").write_text(
            "import 'package:dust_dart/derive.dart';\n\n/// Unsupported syntax probe.\n@Derive([ToString()])\n" + declaration.rstrip() + "\n",
            encoding="utf-8",
        )
        run([str(dart), "analyze", "--fatal-infos", "lib/probe.dart"], app)
        for command in ("build", "check"):
            result = run([str(dust), command, "--root", str(app)], expected=1)
            if "Dust does not support concise constructors yet" not in result.stderr or (app / "lib/probe.g.dart").exists():
                raise ValueError(f"{name}: missing parser failure or unexpected generated output")
    return list(sources)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for role in ("minimum", "latest"):
        parser.add_argument(f"--{role}-dart", type=Path, required=True)
        parser.add_argument(f"--{role}-flutter-env", required=True)
    args = parser.parse_args()
    output = Path(tempfile.mkdtemp(prefix="dust-sdk-compat-"))
    print(f"Local SDK compatibility evidence: {output}", flush=True)
    run_matrix(args, output)


if __name__ == "__main__":
    main()

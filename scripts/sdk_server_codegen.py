"""Minimum/latest SDK checks for the Dart-only server-codegen consumer."""

from __future__ import annotations

from pathlib import Path

from sdk_codegen_fixtures import generated_hashes, stage_example


def check_server_codegen(run, dust: Path, dart: Path, output: Path,
                         lint_version: str, env: dict[str, str]) -> dict[str, str]:
    """Regenerate and execute the Dart-only server consumer on one SDK."""
    app, required = stage_example(output, "examples/server_codegen", lint_version)
    run([str(dart), "pub", "get"], app, env=env)
    run([str(dart), "pub", "deps", "--json"], app, env=env)
    run([str(dust), "build", "--clean", "--root", str(app)], env=env)
    run([str(dust), "check", "--root", str(app)], env=env)
    hashes = generated_hashes(app)
    if not required.issubset(hashes):
        raise ValueError(f"Missing fresh server outputs: {required - hashes.keys()}")
    run([str(dart), "analyze", "--fatal-infos"], app, env=env)
    run([str(dart), "test", "--reporter=expanded"], app, env=env)
    return hashes


def require_server_determinism(rows: list[dict[str, object]]) -> None:
    """Require identical generated server bytes at both standalone Dart ends."""
    generated = [row["serverGenerated"] for row in rows if row["target"] == "dart"]
    if len(generated) != 2 or generated[0] != generated[1]:
        raise ValueError("Missing or different minimum/latest server generated output")

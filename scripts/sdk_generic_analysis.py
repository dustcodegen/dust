"""Analyze new generic output without broad lint suppression in owned staging."""

from __future__ import annotations

from pathlib import Path


def check_generic_analysis(run, dart: Path, app: Path, options: Path,
                           library: str, env: dict[str, str], *,
                           stems: tuple[str, ...] = ("generic_models", "generic_imported")) -> None:
    """Restore every staged byte even on failure; never touch source checkout files."""
    config = app / "analysis_options.yaml"
    targets = [app / library / f"{name}{suffix}.dart"
               for name in stems for suffix in ("", ".g")]
    originals = {path: path.read_bytes() for path in [config, *targets[1::2]]}
    analysis = options.read_text(encoding="utf-8")
    analysis = analysis.replace('    - "**/*.g.dart"\n', "")
    analysis = analysis.replace("    strict_raw_type: ignore\n", "")
    analysis = analysis.replace("    inference_failure_on_function_invocation: ignore\n", "")
    analysis = analysis.replace("    inference_failure_on_instance_creation: ignore\n", "")
    analysis = analysis.replace("analyzer:\n", "analyzer:\n  language:\n"
                                "    strict-casts: true\n    strict-inference: true\n"
                                "    strict-raw-types: true\n")
    try:
        config.write_text(analysis, encoding="utf-8")
        for path in targets[1::2]:
            path.write_text(originals[path].decode("utf-8").replace(
                "// ignore_for_file: type=lint\n", ""), encoding="utf-8")
        run([str(dart), "analyze", "--fatal-infos", *map(str, targets)], app, env=env)
    finally:
        for path, source in originals.items():
            path.write_bytes(source)

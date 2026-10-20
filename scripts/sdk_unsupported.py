"""SDK-accepted syntax must fail clearly when Dust does not yet support it."""

import json
import shutil
from pathlib import Path

from sqlite_sdk_consumer import ROOT


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

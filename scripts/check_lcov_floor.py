#!/usr/bin/env python3
"""Print LCOV line coverage and fail when it falls below a floor."""

from __future__ import annotations

import argparse
import tempfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path, nargs="?")
    parser.add_argument("floor", type=float, nargs="?")
    parser.add_argument(
        "--include-prefix",
        help="Count only LCOV source paths with this prefix.",
    )
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.self_test:
        run_self_test()
        return
    if args.report is None or args.floor is None:
        raise SystemExit("report and floor are required unless --self-test is set")

    check(args.report, args.floor, args.include_prefix)


def check(report: Path, floor: float, include_prefix: str | None = None) -> None:
    total = 0
    covered = 0
    uncovered: dict[str, list[str]] = {}
    current: str | None = None
    missing: list[str] = []

    included = False
    for line in report.read_text().splitlines():
        if line.startswith("SF:"):
            current = line[3:]
            missing = []
            included = include_prefix is None or current.startswith(include_prefix)
        elif line.startswith("DA:") and included:
            number, count, *_ = line[3:].split(",")
            total += 1
            if int(count) > 0:
                covered += 1
            else:
                missing.append(number)
        elif line == "end_of_record" and current and missing:
            uncovered[current] = missing

    if total == 0:
        raise SystemExit("no coverage data; did the suite run?")

    percent = covered * 100 / total
    print(f"{covered}/{total} lines = {percent:.2f}%")

    for path, lines in uncovered.items():
        print(f"  uncovered {path}: {', '.join(lines)}")

    if percent + 1e-9 < floor:
        raise SystemExit(
            f"coverage {percent:.2f}% is below the {floor:.2f}% floor"
        )


def run_self_test() -> None:
    report = """SF:lib/src/router/router.dart
DA:1,1
DA:2,0
end_of_record
SF:lib/src/state/state.dart
DA:1,1
end_of_record
"""
    with tempfile.TemporaryDirectory(prefix="dust-lcov-floor-") as temp:
        path = Path(temp) / "lcov.info"
        path.write_text(report)
        check(path, 100, "lib/src/state/")
        try:
            check(path, 100, "lib/src/router/")
        except SystemExit as error:
            assert "50.00%" in str(error)
        else:
            raise AssertionError("filtered floor should reject missing router line")
        try:
            check(path, 0, "lib/src/missing/")
        except SystemExit as error:
            assert str(error) == "no coverage data; did the suite run?"
        else:
            raise AssertionError("missing prefix should reject empty coverage")
    print("lcov floor self-test passed")


if __name__ == "__main__":
    main()

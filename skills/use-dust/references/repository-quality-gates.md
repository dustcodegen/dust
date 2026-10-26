# Dust Repository Quality Gates

Use this reference only when changing the Dust repository itself. The scripts
are the enforcement source of truth; do not duplicate their logic in an agent
hook or hosted CI workflow.

## Local-first policy

- GitHub Actions is for manually triggered Rust releases only.
- Do not add or trigger pull-request, push, scheduled, or general hosted CI.
- External checks are allowed only when they consume no GitHub Actions runner
  time.
- Run focused checks while developing and the complete local pre-commit gate
  before committing. Never bypass it with `--no-verify`.
- Configure the repository hook once with:

```bash
git config core.hooksPath .githooks
```

The complete gate is:

```bash
.githooks/pre-commit
```

It runs formatting, linting, source-size checks, security audit, compatibility
validation, generated-output checks, Rust/Dart/Flutter tests, and coverage.
Generated proof outputs changed by the gate must be reviewed and staged before
the commit is retried.

The source-size enforcement path is explicit:

```text
.githooks/pre-commit
  -> scripts/lint.sh
  -> python3 scripts/check_source_size.py
```

The hook and checker stay in the Dust repository so every contributor runs the
same executable policy. This skill describes and invokes that policy; it must
not install or replace Git hooks in unrelated application repositories.

## Source and test size

Every handwritten Rust, Dart, Python, shell, native test, example, fixture, and
test source must stay at or below 300 lines. Generated Dart is excluded.
`scripts/check_source_size.py` enforces the rule, and its `BASELINE` must remain
empty. Split a growing file by responsibility; do not add an exemption.

## Coverage floors

The pre-commit hook enforces these current minimums:

| Surface | Floor |
| :--- | ---: |
| Rust workspace | 90% |
| `dust_dart` | 100% |
| `dust_db_sqlite3` | 96% |
| `dust_server` | 100% |
| `dust_flutter` | 90% |
| Flutter router source | 100% |
| Flutter state source | 100% |

Add focused regression tests for every changed branch or failure mode. Prefer
exact generated snapshots over substring assertions. A passing focused test is
not a substitute for the full hook before commit.

## Handoff evidence

Report the exact focused checks and full gate result. Keep these states
separate: edited, formatted, tested, committed, pushed, reviewed, merged,
released, and published. Do not call work complete while a required local gate
is red or while generated output is unstaged.

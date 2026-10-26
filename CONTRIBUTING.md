# Contributing to Dust

Thank you for your interest in contributing to Dust! This project aims to provide the fastest Dart code generation engine, and we welcome contributions that help us maintain high performance and code quality.

---

## 🚀 Getting Started

To contribute to Dust, you'll need both the Rust and Dart toolchains installed on your machine.

### Prerequisites
- **Rust:** Latest stable version (via [rustup](https://rustup.rs/))
- **Dart:** SDK 3.0 or later (via [dart.dev](https://dart.dev/get-dart))
- **Flutter:** (Optional) For running mobile examples
- **SQLite:** System SQLite library for `dust_db_sqlite3` tests.

### Initial Setup
1. Fork and clone the repository.
2. Initialize the project:
   ```bash
   # Install Rust dependencies and run tests
   cargo test --workspace --quiet
   
   # Fetch Dart dependencies
   flutter pub get
   ```
3. Enable the repository pre-commit hook:
   ```bash
   git config core.hooksPath .githooks
   ```

   The hook delegates formatting, analysis, source-size, generated-output, and
   test work to the shared scripts:
   ```bash
   scripts/lint.sh
   scripts/test.sh
   ```

   It then runs the repository security, compatibility, and coverage gates.
   Use `--scope packages` or `--scope rust` only for focused local development
   checks; a commit still requires the complete hook.

---

## 📂 Repository Map

| Directory | Purpose |
| :--- | :--- |
| `crates/` | Rust crates for parsing, IR, plugins, emitter, and the CLI. |
| `packages/` | Publishable Dart annotation and runtime packages. |
| `examples/` | Real-world example projects and scale/perf fixtures. |
| `docs/` | Comprehensive usage guides and internal architecture docs. |

---

## 🛠️ Development Workflow

### Local Build Commands
Generate the **Product Showcase**:
```bash
cargo run -p dust_cli -- build --root examples/product_showcase
```

Generate the **Benchmark Project** (Scale Testing):
```bash
cd examples/benchmark_project
./generate.sh --count 5000
cd ../..
cargo run -p dust_cli -- build --root examples/benchmark_project
```

### Verification & Testing
Before submitting a PR, ensure all checks pass:

```bash
# Full local gate used by pre-commit and release preparation
scripts/lint.sh
scripts/test.sh

# Focused local checks while developing
scripts/lint.sh --scope rust
scripts/test.sh --scope packages
```

The Dart formatter prepares `fixtures/sqlite_sdk_consumer` with `dart pub get`
before formatting it: this independent consumer needs its own language metadata.
An initial run needs Pub access or a populated cache. Example-only and Rust-only
formatting do not resolve the consumer.

### Package Version Rule

`packages/` contains publishable Dart and Flutter runtime packages. If a PR
changes a package's `lib/` source, public API, dependency constraints, runtime
behavior, or compatibility with generated code, record it under that package's
`[Unreleased]` changelog section. Do not bump one package by itself. The CLI
and every publishable package move together in an explicitly approved
release-preparation change.

Docs-only and tests-only edits do not need a changelog entry unless they change
the documented public contract.

When generated code depends on a package version, also update
[`compatibility/dust-cli-packages.json`](compatibility/dust-cli-packages.json)
and the [compatibility guide](docs/usage/compatibility.md).

---

## 📚 Documentation Rules

Documentation PRs should keep the page's primary reader intent clear:

- **Tutorials:** give beginners a prescriptive path to a first successful result.
- **How-to guides:** solve one named task and assume the reader knows the basics.
- **Reference:** provide dry, structured facts such as flags, types, config, generated files, and schemas.
- **Explanation:** describe design reasons, trade-offs, and background context.

Before opening a docs PR:

- Verify the README quick start can be completed in about 5 minutes.
- Copy-paste each README command or code block into a real shell/project before claiming it works.
- Use literal placeholders such as `your-api-token-here`; avoid ambiguous placeholders such as `<token>`.
- Document breaking syntax or API changes in `CHANGELOG.md` or in a clearly linked migration section.
- Keep changelogs based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
  The root `CHANGELOG.md` is the source of truth for GitHub release notes; do not add separate release-note files.
- Keep docs changes scoped and reviewable. If a page mixes tutorial, how-to, reference, and explanation content, split the content or make the primary role explicit.

---

## ⚖️ Engineering Rules

To keep Dust fast and maintainable, please follow these core principles:

- **Single Pipeline:** Use the shared 4-pass build pipeline. Do not add plugin-specific hacks to the driver.
- **Shared Analysis:** Use Pass 2 (`collect_workspace_analysis`) for cross-file state.
- **No Panics:** Avoid `.expect()` or `.unwrap()` in plugin code. Use `Diagnostic::error` instead.
- **Deterministic:** Generated output must be deterministic and byte-for-byte identical on every run.
- **Small & Clean:** Keep generated Dart code small, readable, and analyzer-clean.
- **One List:** A publishable package is named once, in the root `pubspec.yaml`
  workspace. `scripts/workspace_packages.sh` derives the lint, format and test
  targets from it, `scripts/check_coverage_reports.py` fails if Sonar has no
  coverage report for a package the gate tests, and
  `scripts/check_workflow_paths.py` fails if a workflow names a file that no
  longer exists.
- **Lockstep Versions:** Every publishable package carries the CLI's version,
  even when its own changes would only be a patch. All five publish from one
  `v{version}` tag and pub.dev refuses a package whose version differs from it,
  so a package on its own track cannot publish at all.
  `scripts/check_release_versions.py` enforces this.
- **First Publish By Hand:** A new package's first version has to be published
  manually. Automated publishing is configured on a package's pub.dev page, and
  that page does not exist until the package does, so a tag cannot publish a
  name pub.dev has never seen. `scripts/check_publish_readiness.py` checks this
  in the release job before anything is published.
- **Small Files:** Every handwritten Rust, Dart, Python, shell, test, example,
  fixture, and native test source stays at or below 300 lines.
  `scripts/check_source_size.py` enforces this in the local lint gate.
  Generated Dart is excluded. The script's `BASELINE` stays empty;
  split a growing file by responsibility instead of adding an exemption.

---

## 📬 Pull Requests

1.  **Conventional Commits:** Use prefixes like `feat:`, `fix:`, `perf:`, or `docs:`.
2.  **Small Scopes:** Prefer smaller, focused PRs over giant refactors.
3.  **Include Commands:** In your PR description, list the exact commands you used to verify your changes.
4.  **Performance Impact:** If your change affects the build path, report both **Cold** and **Warm** build times using the benchmark project.

### AI-assisted contributions

AI tools are allowed as assistants, but contributors must own the final diff. Do not open blind agent-created PRs, automated bulk rewrites, or changes with generated output you have not reviewed.

Before opening an AI-assisted PR, be ready to answer:

- What Dust feature, package, or example does this touch?
- What user problem, failing test, or minimal example proves the change is needed?
- What generated Dart output changes, if any?
- What local commands did you run?
- What risk or behavior change should maintainers review?

For every AI-assisted PR, confirm that you personally reviewed the changed code, ran the stated checks, and can explain the design tradeoff. If you cannot answer those questions yet, open an issue first with a minimal example project, reproduction, or expected generated output.

This policy is a review-quality gate only. CLA, DCO, or sign-off automation is tracked separately.

---

## 🐞 Reporting Issues

- **Bugs:** Provide a minimal Dart snippet that reproduces the issue.
- **Features:** Describe the desired Dart API and the expected generated code.
- **Performance:** Include environment details (OS, CPU, RAM) and the scale of the project (file count).

---

## 📜 License

By contributing to Dust, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).

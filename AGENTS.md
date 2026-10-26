# Dust Agent Rules

## Scope

- These rules apply to the complete Dust repository.
- Preserve unrelated work and use a clean worktree for implementation.
- Prefer the smallest complete change and keep handwritten application APIs
  stable unless a separately reviewed correctness fix requires a migration.

## CI Policy

- GitHub Actions is reserved for release workflows only.
- Do not add or trigger Actions for ordinary pushes, pull requests, scheduled
  validation, or development checks.
- GitHub pull requests may show external checks such as SonarQube or SonarCloud.
  Those checks must run on the provider's infrastructure, consume no GitHub
  Actions runner time, and never be invoked by an Actions job.
- Run `./.githooks/pre-commit` locally before every commit and fix every
  failure before pushing.
- The release workflows in `.github/workflows/release.yml` and
  `.github/workflows/rust-release.yml` are the only allowed workflow files.
- Do not change this policy unless the repository owner explicitly approves
  the change.

## Local Quality Gate

- `./.githooks/pre-commit` is the authoritative local commit gate. It delegates to
  `./scripts/lint.sh` and `./scripts/test.sh`, then runs security, compatibility,
  coverage, and generated-output checks.
- Every handwritten Rust, Dart, Python, shell, test, example, fixture, and
  native test source stays at or below 300 lines. Generated Dart is excluded.
- Keep `scripts/check_source_size.py`'s `BASELINE` empty. Split by
  responsibility instead of hiding growth behind an exemption.
- Do not suppress warnings, exclude production files from coverage, or lower a
  threshold to make a change pass.
- Regenerate output from handwritten source. Never edit `.g.dart` files.

## Architecture

- Parser syntax stays in parser crates; normalized meaning stays in the
  resolver and IR; feature behavior stays in the owning plugin.
- Plugins consume structured facts and shared emitters. Do not parse source
  strings or add a second workspace scan inside a plugin.
- Keep `dust_dart` Dart-only, `dust_flutter` Flutter-only, and database/server
  runtimes in their dedicated packages.
- Generated output must be deterministic and analyzer-safe without using
  `dart format` as a repair step. Use exact `.dart.snapshot` contracts.

## Flutter Routing

- Routing uses Navigator 2.0 without another routing package and exposes one
  application `route.dart` entrypoint.
- The router owns typed route stacks and navigation lifecycle. Applications
  own platform link configuration, host allow-lists, and durable auth state.
- Preserve protected-by-default routes, typed parameters/results, guarded deep
  links, branch stacks, browser history, and system-back behavior.

## Flutter State

- Keep dependencies in typed `args` and UI values in immutable `state`/`value`.
- Do not generate mirror getters for `args.*` or `state.*`; use generated
  watch, read, selector, and listener APIs.
- Created scopes own initialization and disposal. `.value` scopes never dispose
  externally owned ViewModels.
- Action tokens suppress stale emissions but do not claim to cancel underlying
  I/O. Effects are at-most-once events, not durable state.

## Release

- Record package changes under `[Unreleased]`; versions move together only in
  the explicitly approved release preparation.
- Do not bump versions, tag, publish, or dispatch a release workflow without
  direct user approval.

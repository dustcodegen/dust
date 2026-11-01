# Developing Dust

This index is for contributors and maintainers. Application setup and public
APIs are documented separately in [Using Dust](../usage/README.md).

## Start here

1. Follow [Contributing](../../CONTRIBUTING.md) to prepare the checkout and
   enable the repository hooks.
2. Read the [Developer guide](../developer.md) for the build pipeline and
   ownership boundaries.
3. Use the [Plugin guide](../plugin-guide.md) when changing generation
   behavior.
4. Run `./.githooks/pre-commit` before every commit.

## Engine and plugin architecture

- [Developer guide](../developer.md): workspace map, build pipeline, ownership
  boundaries, and validation commands.
- [Plugin guide](../plugin-guide.md): plugin registration, IR input, workspace
  analysis, emission, diagnostics, and tests.
- [Crate map](../../crates/README.md): responsibility of each Rust crate.
- [Design notes](../design/README.md): active implementation designs and
  retained architectural decisions.

## Package validation

These are maintainer gates, not application setup instructions. Their existing
paths are retained because package READMEs, fixtures, and external links use
them.

### Dust CLI and release set

- [Minimum and latest SDK validation](../usage/sdk-validation.md)
- [Package production readiness](../production-readiness.md)

### `dust_dart` and parser

- [Listed Dart grammar migration](../usage/listed-grammar-migration.md)
- [Primary-constructor consumer validation](../usage/primary-constructors.md#executable-checks)
- [Minimum and latest SDK validation](../usage/sdk-validation.md)

### `dust_flutter`

- [Minimum and latest SDK validation](../usage/sdk-validation.md)
- [Package status and deployment boundary](../production-readiness.md#current-package-status)

### `dust_db_sqlite3`

- [Independent SDK and native consumer](../usage/db-sdk-validation.md)
- [Crash and reopen recovery](../usage/db-recovery.md)
- [Injected native storage faults](../usage/db-native-storage.md)
- [Actual filesystem capacity failure](../usage/db-filesystem-full.md)
- [Flutter lifecycle validation](../usage/db-flutter-validation.md)
- [Linux validation](../usage/db-linux-validation.md)

### `dust_db_postgres`

- [Package tests and disposable-database integration](../../packages/dust_db_postgres/README.md#testing)
- [Release-candidate database evidence](../production-readiness.md#release-candidate-evidence)

### `dust_server`

- [Server production readiness](../dust_server/production-readiness.md)
- [Server testing](../dust_server/testing.md)
- [Server security](../dust_server/security.md)

## Release evidence and policy

- [Release rebuild evidence](../releases/v0.2.1-rebuild.md)
- [Security policy](../../SECURITY.md)
- [Code of Conduct](../../CODE_OF_CONDUCT.md)
- [Roadmap and milestones](https://github.com/dustcodegen/dust/milestones)

GitHub Actions is reserved for the repository's release workflows. Ordinary
development validation runs locally through `./.githooks/pre-commit`.

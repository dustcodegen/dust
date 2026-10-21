# dust

**Rust-powered, batteries-included code generation for full-stack Dart and
Flutter.**

Generate data classes, JSON, validation, HTTP clients and servers, routing,
state management, localization, and SQL—without `build_runner`.

[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

[Website](https://dustcodegen.github.io/) · [Source](https://github.com/dustcodegen/dust)

This source snapshot comes from cached mainline revision
`45349afab66f25b5092b4a0e7189a785ff578020` of the original repository.
The original remote is unavailable, so its current state cannot be verified.
Versions are unchanged. The original `v0.2.1` source was rebuilt through
[Rust-only release CI](https://github.com/dustcodegen/dust/actions/workflows/rust-release.yml)
with new binaries and checksums; see [rebuild provenance](docs/releases/v0.2.1-rebuild.md).
General CI and OpenSSF Scorecard run in the organization repository. Production
Dart/Flutter publishing remains disabled until a release is prepared and
approved. Original CI history and external quality integrations are not
migrated. Current documentation and package metadata use `dustcodegen/dust`;
already-published pub.dev metadata stays unchanged until a future package
release.

Dust is Rust-powered code generation for Dart and Flutter, built for human
developers and AI coding agents. It handles repetitive and complex code so you
can focus on your product.

## How Dust Works

1. Add Dust annotations to normal Dart or Flutter code.
2. Run `dust build` to generate the typed implementation.
3. Use the generated API from your application code.

## Installation

macOS and Linux (no Rust toolchain required):

```bash
curl -fsSL https://raw.githubusercontent.com/dustcodegen/dust/main/install.sh | bash
```

Windows PowerShell (no Rust toolchain required):

```powershell
irm https://raw.githubusercontent.com/dustcodegen/dust/main/install.ps1 | iex
```

The installers verify SHA-256 checksums before installing into `$HOME/.local/bin`.
Add that directory to `PATH`, then verify:

```bash
dust --version
```

The unchanged `v0.2.1` binary still has its historical `dust upgrade` destination.
Current source uses `dustcodegen/dust`, but existing binaries are not replaced.
Until a future binary release includes this fix, rerun the new installer to reinstall.

Dust packages have their own pub versions. When changing the CLI or packages,
check the [compatibility guide](docs/usage/compatibility.md).

## Quick Start

Dust has two main runtime packages:

- `dust_dart` provides data classes, JSON, validation, HTTP clients, and
  database annotations.
- `dust_flutter` provides Flutter routing, state management, and i18n.

Start with `dust_dart` from your application directory:

```bash
dart pub add dust_dart
```

Annotate a model:

```dart
import 'package:dust_dart/derive.dart';

part 'user.g.dart';

@Derive([ToString(), CopyWith()])
class User with _$User {
  const User(this.name);

  final String name;
}
```

Generate `user.g.dart`, then verify generated files are current:

```bash
dust build
dust check
```

Flutter apps can add `dust_flutter` too:

```bash
flutter pub add dust_flutter
```

See the [Flutter package guide](packages/dust_flutter/README.md) for routing,
state, and i18n setup.

## Agent Skill

Install the [use-dust Agent Skill](skills/use-dust/SKILL.md):

```bash
npx skills add dustcodegen/dust --skill use-dust
```

## Features

| Feature | Status | Documentation |
| :--- | :--- | :--- |
| Data classes | Stable | [Derive guide](docs/usage/derive.md) |
| JSON serialization | Stable | [JSON guide](docs/usage/serde.md) |
| Validation | Stable | [Validation guide](docs/usage/validation.md) |
| HTTP clients | Stable | [HTTP guide](docs/usage/http.md) |
| Primary constructors | Supported in source (Dart 3.13+) | [Usage and limits](docs/usage/primary-constructors.md) |
| Routing | Beta | [Routing guide](docs/usage/routing.md) |
| State management | Beta | [State guide](docs/usage/state.md) |
| i18n | Beta | [i18n guide](docs/usage/i18n.md) |
| Database | Beta | [Database guide](docs/usage/db.md) |
| HTTP servers | Beta, runtime only | [Server guide](docs/dust_server/README.md) |
| Firebase | Planned | — |
| Supabase | Planned | — |

Stable features keep their documented authoring APIs compatible throughout
`0.1.x`. Beta features may still receive API refinements.

Primary-constructor support is available from the current source checkout, not
the published 0.2.1 binaries or packages. Traditional constructors remain
supported on the documented minimum Dart and Flutter SDKs.

`dust_server` is a runtime, not a generator. Server code is written by hand
today; `dust build` produces nothing for it.

## Documentation

- [Usage guides](docs/usage/README.md)
- [CLI and package compatibility](docs/usage/compatibility.md)
- [Dart package](packages/dust_dart/README.md)
- [Flutter package](packages/dust_flutter/README.md)
- [Database runtime](packages/dust_db_sqlite3/README.md)
- [Server runtime](packages/dust_server/README.md)
- [Contributor guide](CONTRIBUTING.md)
- [Architecture and internals](docs/developer.md)

## Examples

- [Product showcase](examples/product_showcase/README.md): Dart models, JSON,
  validation, HTTP clients, and row mapping.
- [Shopping app](examples/shopping_app/README.md): end-to-end Flutter routing,
  state, i18n, and database integration.
- [Benchmark project](examples/benchmark_project/README.md): scale and
  performance regression fixture.

## Support and Contributing

- [Open an issue](https://github.com/dustcodegen/dust/issues) for bugs and feature
  requests.
- Follow [CONTRIBUTING.md](CONTRIBUTING.md) to build and test Dust from source.
- Report vulnerabilities through [private security reporting](SECURITY.md).

## License

[MIT License](LICENSE).

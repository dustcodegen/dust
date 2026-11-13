<h1 align="center">dust</h1>

<p align="center">
  <strong>Rust-powered, full-stack code generation for Dart and Flutter—from apps to servers.</strong>
</p>

Dust's packages bring Rust-inspired, batteries-included tooling to data
classes, JSON, validation, HTTP clients, routing, state management,
localization, SQL, and Dart servers—without `build_runner`.

<p align="center">
  <a href="https://github.com/dustcodegen/dust/releases/latest"><img alt="GitHub release" src="https://img.shields.io/github/v/release/dustcodegen/dust?style=flat-square&amp;logo=github&amp;color=2563eb"></a>
  <a href="https://github.com/dustcodegen/dust/releases"><img alt="Release downloads" src="https://img.shields.io/github/downloads/dustcodegen/dust/total?style=flat-square&amp;logo=github"></a>
  <a href="https://app.codecov.io/gh/dustcodegen/dust"><img alt="Coverage" src="https://codecov.io/gh/dustcodegen/dust/graph/badge.svg?branch=main"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-16a34a?style=flat-square"></a>
</p>

<p align="center">
  <a href="https://pub.dev/packages/dust_dart"><img alt="dust_dart on pub.dev" src="https://img.shields.io/pub/v/dust_dart?style=flat-square&amp;logo=dart&amp;label=dust_dart"></a>
  <a href="https://pub.dev/packages/dust_flutter"><img alt="dust_flutter on pub.dev" src="https://img.shields.io/pub/v/dust_flutter?style=flat-square&amp;logo=flutter&amp;label=dust_flutter"></a>
  <a href="https://pub.dev/packages/dust_db_sqlite3"><img alt="dust_db_sqlite3 on pub.dev" src="https://img.shields.io/pub/v/dust_db_sqlite3?style=flat-square&amp;logo=sqlite&amp;label=dust_db_sqlite3"></a>
  <a href="https://pub.dev/packages/dust_db_postgres"><img alt="dust_db_postgres on pub.dev" src="https://img.shields.io/pub/v/dust_db_postgres?style=flat-square&amp;logo=postgresql&amp;label=dust_db_postgres"></a>
  <a href="https://pub.dev/packages/dust_server"><img alt="dust_server on pub.dev" src="https://img.shields.io/pub/v/dust_server?style=flat-square&amp;logo=dart&amp;label=dust_server"></a>
</p>

<p align="center">
  <a href="https://dustcodegen.github.io/">Website</a> ·
  <a href="docs/README.md">Documentation</a> ·
  <a href="https://github.com/dustcodegen/dust/releases/tag/v0.3.1">v0.3.1 release</a> ·
  <a href="examples/">Examples</a>
</p>

## How Dust Works

1. Add Dust annotations to normal Dart or Flutter code.
2. Run `dust build` to generate the typed implementation.
3. Use the generated API from your application code.

## v0.3.1 Highlights

- **Generated server clients:** target one package or split route groups into
  tagged packages, with separate typed Dio clients and a flat generated barrel.
- **Typed payload contracts:** shared request and response models use Dust
  `Serialize()` and `Deserialize()` derives, retaining Serde names, aliases,
  defaults, nested values, enums, and validation.
- **Server performance:** precompiled route-prefix candidates and leaner request
  body handling reduce work on common routing and JSON paths.

Read the complete [v0.3.1 release notes](https://github.com/dustcodegen/dust/releases/tag/v0.3.1).

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

`dust upgrade` and both installers resolve releases from `dustcodegen/dust`.

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

Generate `user.g.dart`:

```bash
dust build
```

Use the generated API immediately:

```dart
final user = User('Ada');
final renamed = user.copyWith(name: 'Grace');

print(renamed);
```

For continuous regeneration while editing:

```bash
dust watch
```

Flutter apps can add `dust_flutter` too:

```bash
flutter pub add dust_flutter material_ui cupertino_ui
```

See the [Flutter package guide](packages/dust_flutter/README.md) for routing,
state, and i18n setup.

## Agent Skill And Plugin

Install the portable [use-dust Agent Skill](skills/use-dust/SKILL.md) in an
Agent Skills-compatible client:

```bash
npx skills add dustcodegen/dust --skill use-dust
```

Install the plugin for Codex CLI and the ChatGPT/Codex desktop marketplace:

```bash
codex plugin marketplace add dustcodegen/dust
codex plugin add dust@dustcodegen
```

Install the same skill as a Claude Code plugin:

```bash
claude plugin marketplace add dustcodegen/dust
claude plugin install dust@dustcodegen
```

The standard `skills/use-dust/SKILL.md` is the single instruction source for
every client; platform manifests only make that skill discoverable. The plugin
adds no `CLAUDE.md`/`CODEX.md`, MCP server, credentials, or hosted CI. In
application repositories it guides Dust setup and diagnosis; in this repository
it also loads the contributor architecture, 300-line source rule, coverage
floors, local pre-commit gate, and release-only CI policy.

## Features

| Feature | Status | Documentation |
| :--- | :--- | :--- |
| Data classes | Stable | [Derive guide](docs/usage/derive.md) |
| JSON serialization | Stable | [JSON guide](docs/usage/serde.md) |
| Validation | Stable | [Validation guide](docs/usage/validation.md) |
| HTTP clients | Stable | [HTTP guide](docs/usage/http.md) |
| Primary constructors | Supported (Dart 3.13+) | [Usage and limits](docs/usage/primary-constructors.md) |
| Routing | Stable | [Routing guide](docs/usage/routing.md) |
| State management | Stable | [State guide](docs/usage/state.md) |
| i18n | Beta | [i18n guide](docs/usage/i18n.md) |
| Database | Beta | [Database guide](docs/usage/db.md) |
| HTTP servers | Stable | [Server guide](docs/dust_server/README.md) |
| Firebase | Planned | — |
| Supabase | Planned | — |

## Documentation

- [Documentation map](docs/README.md)
- [Using Dust](docs/usage/README.md)
- [CLI and package compatibility](docs/usage/compatibility.md)
- [Dart package](packages/dust_dart/README.md)
- [Flutter package](packages/dust_flutter/README.md)
- [Database runtime](packages/dust_db_sqlite3/README.md)
- [Server runtime](packages/dust_server/README.md)
- [Developing Dust](docs/development/README.md)

## Examples

- [Server codegen](examples/server_codegen/README.md): generated HTTP routes,
  extraction, validation, state, authentication, and uploads.
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

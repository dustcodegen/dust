# Developer Guide

This page describes Dust's internal architecture and the boundaries contributors
should preserve. For checkout, toolchain, and pull-request setup, start with
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Workspace Map

### Core Engine

| Crate | Responsibility |
| :--- | :--- |
| `dust_workspace` | Finds the package root, reads configuration, and discovers candidate Dart libraries. |
| `dust_cache` | Stores persistent per-library fingerprints and workspace-analysis snapshots. |
| `dust_text` | Owns source text, file IDs, ranges, and line indexes. |
| `dust_diagnostics` | Builds source-labelled errors and warnings. |
| `dust_dart_syntax` | Parses shared Dart literal and syntax fragments without owning a full parser. |
| `dust_parser_dart` | Defines the parser backend contract and parser-facing Dart surface. |
| `dust_parser_dart_ts` | Implements that contract with tree-sitter. Tree-sitter nodes stay inside this crate. |
| `dust_resolver` | Resolves imports, annotation identities, symbols, and normalized feature configuration. |
| `dust_ir` | Defines the canonical `DartFileIr` consumed by plugins. |
| `dust_plugin_api` | Defines plugin ownership, workspace analysis, symbol planning, and generated contributions. |
| `dust_dart_emit` | Provides shared Dart names, type rendering, rename rules, and emission helpers. |
| `dust_emitter` | Validates plugins, merges contributions, formats output, and assembles deterministic `.g.dart` files. |
| `dust_driver` | Orchestrates build, check, watch, caching, workers, i18n, and DB modes. |
| `dust_cli` | Parses commands and renders user-facing results. |

Feature generation lives in:

- `dust_plugin_derive`
- `dust_plugin_serde`
- `dust_http_client_plugin`
- `dust_route_plugin`
- `dust_state_plugin`
- `dust_db_plugin`

### Dart and Flutter Packages

| Package | Responsibility |
| :--- | :--- |
| `dust_dart` | Dart-only annotations and runtime APIs for derives, JSON, validation, HTTP, and Database. |
| `dust_flutter` | Flutter-only state, routing, and i18n APIs. |
| `dust_db_sqlite3` | Native SQLite `Executor` implementation for generated Database code. |
| `dust_db_postgres` | PostgreSQL `Executor` and pool implementation for generated Database code. |
| `dust_server` | Dart HTTP server runtime consumed by generated server wiring. |

## Build Pipeline

### 1. Discover and fingerprint

The workspace layer finds Dart libraries containing supported annotations and
assigns their `.g.dart` output paths. The driver fingerprints source text,
`pubspec.yaml`, package configuration, `dust.yaml`, active codegen sources, and
the previous primary and auxiliary outputs.

The codegen fingerprint also includes the active vendored Dart grammar's
native sources and headers, node definitions, Rust binding/build inputs,
grammar definition, Cargo manifest and upstream provenance. A native-only
grammar update therefore invalidates generation caches without a version bump.
Vendor tests, preserved upstream comparison copies and machine-local build or
package-manager artifacts are not inputs. Cargo tracks each registered input
for rebuilds; the manifest uses sorted repository-relative paths.

A library is a cache hit only when those inputs and its generated output set
still match.

Dust prepares each root's parser facts and output plan from its discovery read,
then loads imported namespaces and verifies generated output ownership before
checking library caches. Prepared facts retain exact input bytes and parser options;
canonical loading rereads the source before reusing them.
Root preparation starts larger snapshots first while retaining the original
merge order. Each parser worker can reuse a bounded set of small native trees
through bounded edits to changed text. Facts, source labels, language gates and
namespace projections are extracted from the current input; syntax errors use a
fresh parse.
Root selection and ownership validation assign metadata work as workers become
available, then merge by the original root order and source paths.
Imports needed by constant and transparent typedef aliases are loaded when
annotation lookup follows those aliases. Unused aliases do not expand their
implementation imports. Output ownership waits for pending alias imports.

Successful parser facts have a separate optional snapshot under `.dart_tool/dust`.
Reuse requires the exact source, parser implementation, options and annotation
policy; import targets are resolved again. Codegen cache misses still run analysis
and emission. `dust clean` removes this snapshot, and its writes finish before
the command reports elapsed time.

A successful, diagnostic-free normal build can save an optional unchanged-build
proof. Reuse requires matching tool and cache identities, current package/SDK
metadata, the discovered file set, and complete input/output content checks.
An invalid proof or changed cache takes the ordinary pipeline. DB validation,
configured server destinations, and i18n retain their ordinary pipelines.

### 2. Collect workspace analysis

Cache hits contribute their saved analysis snapshots. Pending libraries are
parsed, resolved, and lowered to `DartFileIr` in parallel. Plugins collect
cross-file facts from that canonical IR into one immutable workspace analysis.

Routing adds one dependency rule: when route declarations change, a cached
router library is rebuilt even when the router source itself did not change.

### 3. Process pending libraries

Workers reuse the pre-lowered IR, build a deterministic symbol plan, run every
plugin's semantic validation, and ask each plugin for generated contributions.
`--fail-fast` stops after the first observed worker error while preserving
parallel execution.

### 4. Assemble and persist

The emitter merges contributions in registry order, formats the generated Dart,
and hashes the primary plus auxiliary output set. `dust build` writes changed
outputs; `dust check` runs the same generation path without writing and reports
stale files.

Successful results update the persistent cache at:

```text
.dart_tool/dust/build_cache_v1.json
```

## Plugin Contract

A plugin should:

1. Claim the fully qualified traits and configuration annotations it owns.
2. Read normalized `DartFileIr`, not raw source strings or tree-sitter nodes.
3. Collect cross-file facts through `collect_workspace_analysis_ir` when needed.
4. Return source-labelled diagnostics for invalid user code.
5. Reserve shared helper names through the symbol plan.
6. Return `PluginContribution` values for the shared emitter to assemble.

See the [Plugin Guide](./plugin-guide.md) for a focused implementation path.

> [!IMPORTANT]
> Add new parser or resolver facts before adding plugin-local source parsing.
> Parser syntax belongs in the parser crates; normalized meaning belongs in the
> resolver and IR; feature behavior belongs in a plugin.

Dust currently uses normalized IR as the feature-lowering boundary. There is no
general plugin lowering hook. Add one only when a concrete feature cannot be
modeled as parser facts, resolver normalization, and plugin
validation/generation.

## Engineering Boundaries

### Public API ownership

- Keep `dust_dart` free of Flutter imports.
- Keep Flutter-only annotations and runtime code in `dust_flutter`.
- Keep database drivers in separate packages such as `dust_db_sqlite3`.
- Prefer generated-code or internal changes before changing stable handwritten
  application APIs.

### Diagnostics and failures

Malformed Dart, invalid annotations, unsupported types, and filesystem failures
must become diagnostics or returned errors. Do not unwrap user-controlled
input. Use `expect` only for an internal invariant whose violation is a Dust
bug and whose contract is covered by tests.

### Deterministic output

- Sort data whose source order is not part of the public contract.
- Use shared emitters or templates instead of plugin-local formatting.
- Keep generated output analyzer-safe without running `dart format` as a repair
  step.
- Use `.dart.snapshot` for generator fixtures and exact snapshot assertions for
  generated contracts.

The local test gate also runs `scripts/strict_generated_analyze.py` against
`fixtures/serde_json_app`, `fixtures/http_client_app`, and
`examples/shopping_app`. That temporary pass includes `.g.dart` files, removes
Dust's broad `type=lint` generated-file ignore, and treats analyzer warnings as
failures so generated-code regressions are not hidden by the normal user-facing
header.

### Cross-file features

Do not add a second workspace scan inside a plugin. Extend shared workspace
analysis and persist the minimum facts needed by cached libraries. Any new
cross-file dependency also needs an explicit cache invalidation rule.

### Flutter routing and state

Routing source annotations are resolved into structured route facts before
`dust_route_plugin` validates names, parameters, guards, shells, branches, and
imports. The plugin emits route data, parsing, navigation, and page assembly;
`dust_flutter` owns only the Navigator 2.0 runtime and shared contracts. The
application owns platform link setup, incoming-host policy, authentication
state, and deployment rewrites.

State annotations follow the same pipeline through `dust_state_plugin`.
Generated bases and scopes connect application ViewModels to the small
`dust_flutter` runtime. A created scope owns `init` and `dispose`; a `.value`
scope observes but never disposes its external ViewModel. Dependencies remain
in typed args, UI values remain in immutable state, and generated watch/select
APIs are the subscription boundary.

Neither runtime invents persistence or I/O cancellation. Route locations are
the durable navigation representation. Applications decide what state survives
process death, and resource-specific clients cancel network, database, stream,
or isolate work.

## Where to Make a Change

| Change | Start here |
| :--- | :--- |
| New Dart syntax support | `dust_parser_dart_ts`, then resolver/IR tests. |
| New annotation or option | Runtime package, resolver/IR, then the owning plugin. |
| Generated Dart behavior | Owning plugin and exact snapshots. |
| Shared Dart rendering | `dust_dart_emit` or `dust_emitter`. |
| Cross-file feature facts | Plugin analysis plus driver cache dependency review. |
| Scheduling, watch, or cache behavior | `dust_driver`, `dust_cache`, and benchmark tests. |
| CLI command or output | `dust_cli` and driver request/result types. |
| Runtime behavior | The package under `packages/` plus Dart or Flutter tests. |

## Validation

Use focused commands while developing:

```bash
cargo nextest run -p dust_driver
cargo clippy -p dust_driver --all-targets -- -D warnings
cargo fmt --all -- --check
```

Run the authoritative local gate before every commit:

```bash
./.githooks/pre-commit
```

Changes affecting discovery, caching, workers, parsing, resolution, or emission
must also run the ignored release benchmark:

```bash
cargo test -p dust_cli benchmark_project_release_build_benchmark \
  -- --ignored --nocapture
```

The benchmark creates a 5,000-file fixture and checks cold, warm, and
tool-invalidated builds. The last phase invalidates every cached tool hash and
runs with `--fail-fast`. The default budgets remain 2,000 ms, 800 ms, and
1,400 ms respectively. See the
[benchmark project](../examples/benchmark_project/README.md) for manual setup
and fixture restoration.

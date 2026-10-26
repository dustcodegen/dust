# Dust Contributor Architecture

Use this reference only when changing the Dust repository itself.

## Read before editing

1. Read the repository `AGENTS.md` for the current scope.
2. Read `CONTRIBUTING.md` for workflow and release rules.
3. Read `docs/developer.md` for the pipeline and ownership boundaries.
4. Read `docs/plugin-guide.md` when changing or adding generation behavior.
5. Search every caller, fixture, snapshot, export, config entry, and example
   touched by the contract before editing it.

## Keep one ownership path

Dust's normal generation path is:

```text
Dart source
  -> parser facts
  -> resolver and canonical IR
  -> owning feature plugin
  -> shared Dart emitters
  -> deterministic .g.dart output
  -> Dart or Flutter runtime
  -> real example proof
```

Put each change in the layer that owns it:

| Concern | Owner |
| :--- | :--- |
| New Dart syntax facts | `dust_parser_dart_ts` |
| Import, annotation, and symbol identity | `dust_resolver` |
| Shared normalized meaning | `dust_ir` |
| Cross-file feature facts | shared workspace analysis and cache dependencies |
| Feature validation and generation | the owning `dust_plugin_*` crate |
| Shared Dart rendering | `dust_dart_emit` or `dust_emitter` |
| CLI orchestration and output | `dust_driver` and `dust_cli` |
| Handwritten public behavior | the owning package under `packages/` |
| End-to-end proof | the matching fixture and `examples/` app |

Do not parse Dart source strings inside a feature plugin. Add structured parser,
resolver, or IR facts instead. Do not add a second workspace scan for a
cross-file feature. Prefer generated or internal changes before changing stable
handwritten application APIs.

## Preserve generated-code contracts

- Keep output deterministic and analyzer-safe without using `dart format` as a
  repair step.
- Use shared emitters and templates instead of ad hoc string assembly.
- Use `.dart.snapshot` for generator fixtures and exact snapshot assertions for
  generated contracts.
- Update public docs and a real example for public behavior changes.
- Review cache invalidation when discovery, syntax, resolution, IR, cross-file
  analysis, or output sets change.
- Return source-labelled diagnostics for invalid user input. Never unwrap
  user-controlled data.

Keep `dust_dart` Dart-only, `dust_flutter` Flutter-only, and database drivers in
their own packages. Follow the repository route, state, DB, and release rules
instead of recreating those policies in a plugin.

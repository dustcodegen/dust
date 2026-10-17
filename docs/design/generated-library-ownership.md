# Producer-owned generated library inputs

Status: user-approved design for [#114](https://github.com/dustcodegen/dust/issues/114).
The Rust contract and production loader are implemented in the #114 worktree;
full acceptance is still pending. This is not a generation pass or a new
handwritten Dart API.

## Cold-build problem

The router source imports `route/routes.g.dart`, which its plugin has not emitted
yet. Loading this as an ordinary missing library blocks annotation ownership before
the router can run. A filename suffix or matching annotation short name is not
enough to decide that a missing library is a legitimate generated input.

## Rust contract

The optional `DustPlugin::potential_generated_libraries` hook, empty by default, declares potential standalone
generated library paths for an absolute source path. Each claim names the
canonical **class-level producer annotation ID** that must own a handwritten
annotation on that source's class. Initially only the route plugin needs this
contract, using `dust_flutter::AppRouter` and its existing five output paths.
`GeneratedLibrary` retains the output path and canonical `SymbolId`; registry
metadata planning rejects relative paths and unclaimed producers, and excludes
shadowed fallback claims without executing a plugin.

Do not introduce a generic site/configuration language. Class metadata matches
the route plugin's actual producer site; an annotation on a variable, method,
parameter or otherwise ignored site cannot authorize these outputs. Names and
applied arguments remain parser-owned facts, not source-text matching.

Path planning and emission must share the same route output-path computation.
The driver must not hardcode route filenames or infer paths from annotation
names. Registry entries that supply symbols in a focused validation mode still
declare their potential libraries; this metadata does not enable their emission.

## Loading and verification

1. Resolve candidate paths to source identities using the same package/URI index
   as every other source. Reject conflicting ownership claims deterministically.
2. An actually absent candidate may temporarily have an empty namespace. No
   source file is written. Current/stale marked Dust output is also empty and
   cannot provide annotation declarations.
3. Bind the handwritten class annotation against this provisional graph. Accept
   a claim only when its actual canonical owner matches the declared producer ID.
   Aliases and prefixes follow normal namespace binding, not special rules.
4. Restore required-input failures for unverified claims and validate against the
   final graph before any emission/cache update. Recheck surviving producers in
   that graph: provisional inputs cannot legitimize each other or hide a failed
   ownership dependency.

A local foreign `AppRouter`, an annotation declared only in generated output,
or a missing/ambiguous producer does not authorize a placeholder. Valid binding
does not bypass route argument/shape validation or promise that focused DB mode
will emit router code. Reading an ordinary generated model type remains outside
annotation provenance and is still left to Dart's type checker.

An existing unmarked file is not automatically Dust-owned. Keep its ordinary
source/parse failures and fail a verified output-path collision clearly before
overwriting it; neither suffix matching nor a candidate claim may erase user or
another generator's contents. Ordinary noncandidate missing imports still fail.

## Required regressions

- Cold and warm route generation/check/watch preserve existing exact snapshots.
- Every declared route output path matches the emitter's actual path, including
  nested/relative package roots and alternate primary suffix configuration.
- Real aliases/re-exports/prefixes of the producer work; foreign, hidden, deferred,
  conditional or ambiguous producers cannot authorize outputs.
- Class-only verification rejects producer-looking annotations on ignored sites.
- Orphan candidate imports and ordinary missing `.g.dart` libraries still fail.
- Marked stale generated annotations never participate in ownership; unmarked
  output collisions preserve all existing file and successful cache bytes.
- Conflicting claims and provisional/final graph ownership changes fail safely.
- Focused validation gets namespace metadata without enabling other generators.

Run scoped coverage, full strict local lint/test gates and real minimum/latest
consumers. The separate [#142 parser-role design](namespace-interface-diagnostics.md)
must also pass before claiming Flutter or server production readiness.

## Current evidence and boundary

The new loader tests cover cold prefix/alias/re-export producers, alternate
primary suffixes, warm checks, ignored metadata sites, foreign/hidden/deferred
producers, marked generated annotations, unmarked collisions, focused DB mode,
and final-graph revalidation after another claim is removed. Existing route
snapshots, check/watch and multi-router diagnostics are retained.
The current local Rust suite passes 1,188 tests in both default and all-feature
nextest runs (one existing ignored performance test); isolated all-feature
doctests also pass. New graph/output-planning modules have 146/146 executable
lines covered, while the complete loader plus those modules is 1,163/1,196.
These are line counts, not whole-project or branch coverage.

These regressions do not replace the complete local or frozen SDK gates. Parser
projection is reviewed separately in [PR #144](https://github.com/dustcodegen/dust/pull/144);
the valid SQLite generic-call regression remains [#143](https://github.com/dustcodegen/dust/issues/143).
Neither implementation is a claim that server code generation is enabled.

The full #114 local lint gate currently fails fresh Flutter benchmark generation:
Flutter's `AnimationController` has a valid `.forward` instance-field initializer,
outside #142's approved method/function-body projection. Its namespace error must
remain enforced until the narrow follow-up design is approved in
[#145](https://github.com/dustcodegen/dust/issues/145). Producer verification correctly
rejects placeholders when that required dependency fails. The development-only
primary Dart consumer passes fresh generation, normal/DB/offline checks, analysis,
runtime and generic probes; this is not the frozen full minimum/latest #114 matrix.

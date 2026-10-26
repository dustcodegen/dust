# Flutter Routing and State Contract

Routing and state management are stable Dust authoring surfaces for v0.3.0.
This document defines the boundary behind that status; it does not turn app or
deployment responsibilities into generated behavior.

## Architecture

```text
handwritten annotations
        |
        v
parser facts -> shared IR -> route/state plugins -> deterministic .g.dart
                                                   |
                                                   v
                                      dust_flutter runtime
                                                   |
                                                   v
                                         Flutter application
```

- Parser and IR facts own source meaning. Plugins do not parse Dart source
  strings independently.
- Generated code owns typed route data, parsers, navigation helpers, ViewModel
  bases, scopes, selectors, listeners, and builders.
- `dust_flutter` owns Navigator 2.0 stack behavior and ViewModel lifecycle.
- The application owns widgets, repositories, authentication policy, durable
  persistence, analytics, host allow-lists, web rewrites, domain association,
  and platform signing.

## Routing Contract

- One handwritten `lib/route.dart` imports and exports
  `route/routes.g.dart` and declares `@AppRouter`.
- Route widgets declare `@AppRoute` and use an unnamed generative constructor.
- URL parameters use supported URL primitives; domain objects are loaded after
  navigation.
- Routes are protected by default. `guards: []` explicitly makes one public.
- `go`, `push`, `replace`, `pop`, typed results, redirects, guards, shells,
  independent branch stacks, transitions, restoration, and diagnostics are
  covered by runtime and generated-output tests.
- Incoming URLs remain untrusted. Apps validate hosts and stored redirect
  locations before converting them into internal navigation.

## State Contract

- ViewModels extend their generated base. UI-changing values stay in immutable
  state; repositories and clients stay in typed `ViewModelArgs`.
- `watchXViewModel().value`, generated selectors, and builders subscribe.
  `readXViewModel()` is the non-subscribing API for callbacks and factories.
- A creating scope owns, initializes, and disposes its ViewModel. `.value`
  scopes never dispose the externally owned instance.
- Effects are at-most-once notifications for currently mounted listeners, not
  durable state.
- Async state preserves typed failures and optionally visible data during
  refresh. Action tokens prevent stale emissions but do not cancel underlying
  I/O; apps use the client's cancellation mechanism when required.

## Local Release Evidence

The repository pre-commit hook is the authoritative non-release gate. It:

- runs formatting, lint, analysis, and package tests locally;
- requires 100% line coverage for `dust_flutter` router and state runtime
  sources;
- checks deterministic generated outputs and analyzer-safe examples;
- rejects native Rust and Dart source/test files over 300 lines.

The shopping-app integration smoke runs generated routing and state together on
native Flutter hosts. Web readiness additionally requires `flutter build web`.
Release evidence must name the exact hosts used; simulator and emulator results
must not be reported as physical-device proof.

## Compatibility Policy

Stable means the documented handwritten authoring contract remains compatible.
Internal generated code may change. A necessary breaking handwritten change
requires a migration note and a versioned release decision. i18n and Database
keep their separately documented beta status.

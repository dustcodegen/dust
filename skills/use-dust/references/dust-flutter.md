# `dust_flutter` Guide

Use `dust_flutter` only for Flutter routing, ViewModel state, or i18n. Read
[flutter-routing-state.md](flutter-routing-state.md) before routing or state
changes.

## Choose the focused library

| Need | Import | Required authoring shape |
| :--- | :--- | :--- |
| Routing | `package:dust_flutter/route.dart` | one `route.dart` entrypoint and `@AppRoute` widgets |
| State | `package:dust_flutter/state.dart` | `@ViewModel`, generated base, typed args, immutable state |
| i18n | `package:dust_flutter/i18n.dart` | `dust.yaml`, literal keys, locale ARB files, generated app setup |

Most state models also need `dust_dart` derives for equality and `copyWith`.
Do not generate mirror getters for args or state. Use subscribing watch/select
helpers only while building UI, and non-subscribing reads for callbacks,
lifecycle code, and factories.

Routing uses Navigator 2.0 without an external routing package. Preserve the
single public route entrypoint, URL/deep-link behavior, guard defaults, and
application-owned platform link registration.

For i18n, preview discovery with `dust i18n scan`; use `dust i18n build` only
when writing locale output, then verify with `dust i18n check`.

Run normal generation with `dust build` and `dust check`, followed by
`flutter analyze` and `flutter test`.

Source documentation:

- [Routing](https://github.com/dustcodegen/dust/blob/main/docs/usage/routing.md)
- [State](https://github.com/dustcodegen/dust/blob/main/docs/usage/state.md)
- [i18n](https://github.com/dustcodegen/dust/blob/main/docs/usage/i18n.md)
- [Readiness contract](https://github.com/dustcodegen/dust/blob/main/docs/design/flutter-routing-state.md)

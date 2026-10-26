# Flutter Routing and State

Use these stable contracts without inventing another facade.

## Routing

- Keep one `lib/route.dart` entrypoint. It imports and exports
  `route/routes.g.dart`, declares `@AppRouter`, and is what route pages import.
- Annotate widgets with `@AppRoute`. They need an unnamed generative
  constructor. Route fields are URL primitives; load domain objects from app
  state after navigation.
- Routes are protected unless `guards: []` explicitly marks them public.
- Use generated `go`, `push`, `replace`, and `pop`; `push` returns the declared
  route result type.
- Let the app own redirect policy, external-host rejection, Android/iOS link
  registration, HTTPS association files, web rewrites, and signing.
- Use shells and branch stacks from the generated contract. Do not add another
  router package or handwritten route mirror.

## State

- Put changing UI facts in immutable state. Put repositories, clients, storage,
  and observers in a typed `ViewModelArgs` subclass.
- Use `watchXViewModel().value`, selectors, or builders where UI rebuilds. Use
  `readXViewModel()` in callbacks, lifecycle methods, and dependency factories.
- A normal generated scope owns, initializes, and disposes the ViewModel. A
  `.value` scope initializes and listens but never disposes the external value.
- Effects are at-most-once and not replayed. Durable UI facts belong in state.
- `runAction` prevents stale results from emitting. It does not cancel the
  underlying request; use the client's cancellation API for that.
- Do not add generated mirror getters for `args` or state fields.

## Validation

Run `dust build`, `dust check`, `flutter analyze`, and `flutter test`. For a
shipping app, also build every supported platform and run its routing/state
integration smoke on the available native hosts. Report simulator/emulator
evidence separately from physical-device evidence.

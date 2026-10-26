# `dust_server` Guide

Use `dust_server` for native Dart HTTP servers on `shelf`. It is modelled on
Rust's axum but does not embed or depend on axum.

Choose one authoring style:

- reuse the handwritten `Router` API for runtime-only projects;
- use top-level `@Routes` functions for generated modules without shared class
  dependencies;
- use `@Controller` only when a class is the clearer home for shared
  dependencies.

Generated routes keep handler bodies and deployment policy handwritten. Use
one HTTP verb annotation per handler, explicit path/body/query extraction, and
`@Query() Model` only for a same-library flat model deriving `Deserialize()`.
Derive `Validate()` when decoded input must be validated.

At the application boundary, set explicit body, concurrency, queue, timeout,
rate-limit, logging, and error policies. Validate WebSocket origins, constrain
static-file roots, and test graceful shutdown. Timeout cancellation is
cooperative; it cannot forcibly stop an arbitrary Dart future.

For generated modules, run `dust build` and `dust check`, then `dart analyze`
and `dart test`. Handwritten-only server projects need no Dust generation.
Check installed versions first: published 0.2.1 has the stable runtime but not
the current source generator.

Source documentation:

- [Server guide](https://github.com/dustcodegen/dust/blob/main/docs/dust_server/README.md)
- [Generated example](https://github.com/dustcodegen/dust/blob/main/examples/server_codegen/README.md)
- [Handwritten examples](https://github.com/dustcodegen/dust/blob/main/packages/dust_server/example/README.md)
- [Security](https://github.com/dustcodegen/dust/blob/main/docs/dust_server/security.md)

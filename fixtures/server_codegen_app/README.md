# Server codegen fixture

This package is the executable Phase 1 exit gate for Dust server generation.
Its controller source is handwritten; `todo_controller.g.dart` is produced by
the Rust plugin.

```bash
cargo run --quiet -p dust_cli -- build --clean --root fixtures/server_codegen_app
cargo run --quiet -p dust_cli -- check --root fixtures/server_codegen_app
cd fixtures/server_codegen_app
dart analyze --fatal-infos
dart test
```

The tests exercise generated controller and top-level GET and POST routes,
path/query/header extraction, JSON body decoding, an explicit success status,
void-to-204 and explicit `Response` dispatch, and a typed path rejection.
Later-phase custom extractors, state, raw/form/multipart bodies, middleware, and
typed `Result` responses remain in `fixtures/server_app` as runtime contracts.

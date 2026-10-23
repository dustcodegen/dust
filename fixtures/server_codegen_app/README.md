# Server codegen fixture

This package is the executable Phase 5 exit gate for Dust server generation.
`basic_routes.dart` is the smallest function-based example: generated GET,
POST, DELETE, path, and query wiring without a controller. The larger
`todo_controller.dart` fixture keeps proving the complete surface.

```bash
cargo run --quiet -p dust_cli -- build --clean --root fixtures/server_codegen_app
cargo run --quiet -p dust_cli -- check --root fixtures/server_codegen_app
cd fixtures/server_codegen_app
dart analyze --fatal-infos
dart test
```

The tests exercise basic function routes, generated controller routes, every Phase 3
extractor, one-pass form and multipart bodies, validation, extractor wrappers,
typed response dispatch, guarded failures, and the 400/413/415/422/500
rejection taxonomy. Phase 4 adds generated controller/route layers and a real
loopback server proving merge/nest, prefix normalization, root body limits,
request-scoped error reporting, and graceful shutdown.

Phase 5 adds inherited operation metadata, typed `describe()` recovery after
mounting, and the workspace facts consumed by future documentation tooling.

Run the production-shaped entrypoint with:

```bash
dart run bin/server.dart 8080
```

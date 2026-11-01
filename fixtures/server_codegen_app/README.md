# Server codegen fixture

This package is the executable regression fixture for Dust server generation.
`basic_routes.dart` is the smallest function-based example: generated GET,
POST, DELETE, path, and query wiring without a controller. The larger
`todo_controller.dart` fixture proves the complete generated surface.

```bash
cargo run --quiet -p dust_cli -- build --clean --root fixtures/server_codegen_app
cargo run --quiet -p dust_cli -- check --root fixtures/server_codegen_app
cd fixtures/server_codegen_app
dart analyze --fatal-infos
dart test
```

The tests exercise basic function and controller routes, every supported
extractor, one-pass form and multipart bodies, validation, extractor wrappers,
typed response dispatch, guarded failures, and the 400/413/415/422/500
rejection taxonomy. They also cover generated controller/route layers, a real
loopback server, merge/nest composition, prefix normalization, root body
limits, request-scoped error reporting, graceful shutdown, inherited operation
metadata, and typed `describe()` recovery after mounting.

Run the fixture server with:

```bash
dart run bin/server.dart 8080
```

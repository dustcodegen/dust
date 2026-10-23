# Public server-entrypoint namespace control

This fixture exercises the production source loader after Phase-1 server
generation. `dust_dart` owns `ToString`, while the actual `dust_server`
annotations keep their own owners and never acquire Dio meaning merely because
their short names overlap.

The driver regression copies actual runtime source trees into an owned temporary
workspace and resolves `package:dust_server/server.dart` through package config.
Its SDK and external dependency namespaces are synthetic, so it is not compiler
evidence. A source-loader unit contract checks exact default server ownership
and verifies that resolved configs never acquire Dio normalization. Command
tests prove invalid server declarations and dependency failures cannot replace
successful output or cache state, then verify controller route generation.

The executable fixture for analyzer and request-level proof lives in
`fixtures/server_codegen_app`.

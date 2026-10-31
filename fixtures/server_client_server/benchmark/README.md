# Server request-path benchmark

These benchmarks exercise the generated validated JSON endpoint in
`client_edges.dart`. They vary the payload so the compiler cannot replace the
work with a constant result.

Compile and run the in-memory breakdown:

```bash
dart compile exe benchmark/server_path_benchmark.dart \
  -o .dart_tool/benchmark/server_path_benchmark
.dart_tool/benchmark/server_path_benchmark
```

For real HTTP, start the server in one process:

```bash
dart compile exe benchmark/server_http_benchmark.dart \
  -o .dart_tool/benchmark/server_http_benchmark
.dart_tool/benchmark/server_http_benchmark
```

Then drive it from another process:

```bash
wrk -t2 -c32 -d10s --latency \
  -s benchmark/server_payload_wrk.lua \
  http://127.0.0.1:8080/transport/typed
```

Pin the Dart SDK, CPU and memory limits before comparing revisions. Report the
median of repeated runs and keep the raw latency output; one local run is not a
production capacity claim.

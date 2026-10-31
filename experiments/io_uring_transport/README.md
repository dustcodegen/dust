# Linux `io_uring` transport evaluation

This directory contains the rejected transport experiment for server epic
[#211](https://github.com/dustcodegen/dust/issues/211). It is not a
production runtime and is not part of `dust_server`.

## Decision

Keep Shelf as Dust's only production HTTP runtime. The `io_uring` prototype
missed the required throughput gate on the same constrained Linux harness.

| Path | Median requests/second | Versus Shelf |
| --- | ---: | ---: |
| Optimized generated Dust/Shelf | 18,942 | baseline |
| Required go threshold | 24,625 | +30.0% |
| Generated Dust over the `io_uring` probe | 13,600 | -28.2% |

The native-to-Dart bridge plus the exact generated handler reached about
41,200 operations/second with 8 to 128 requests in flight. The handler was not
the limiting stage. Adding native socket reads, request delivery, Dart response
delivery, wakeups, copies, and native socket writes reduced the five-run median
to 13,600 requests/second.

The transport is 44.8% below the go threshold. Protocol-parity work therefore
stops here; streaming, multipart, SSE, WebSocket, backpressure, cancellation,
fuzzing, and release packaging were intentionally not implemented.

## Harness

- Apple Silicon Docker Linux, ARM64 kernel 6.12.76-linuxkit
- Dart 3.12.1 AOT
- one CPU and 2 GB memory for the server container
- `wrk`, two threads, 32 keep-alive connections, five 5-second runs
- generated `POST /transport/typed` route with varied validated JSON
- response status and body checked before the load run

The five transport runs were 13,535, 13,463, 13,600, 13,631, and 13,735
requests/second. This is local comparative evidence, not a production capacity
claim.

Docker's default seccomp profile rejected `io_uring_setup` in this environment.
The probe required `--security-opt seccomp=unconfined`; a production design
would need a narrower reviewed policy rather than this benchmark-only setting.

## Contents

- `bridge/`: bounded native-thread to Dart callback feasibility probe
- `native_server/`: minimal HTTP/1.1 `io_uring` socket probe
- `bridge_benchmark.dart`: callback-only benchmark
- fixture benchmarks: generated handler bridge and live server entrypoints in
  `fixtures/server_client_server/benchmark/`

The probe handles only bounded Content-Length HTTP/1.1 requests and responses.
It exists to make the rejected result reproducible, not as reusable server
code.

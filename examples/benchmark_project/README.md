# Dust Benchmark Project

Large local Flutter fixture project for Dust build/watch scale testing.

## What this project is for

- generate a large number of annotated Dart and Flutter source files
- exercise `ToString()`, `Eq()`, `CopyWith()`, `Serialize()`, and
  `Deserialize()` in one mixed corpus
- exercise `Validate()` with email, URL, length, range, contains,
  does-not-contain, regex, must-match, nested, custom, required, nullable, and
  form-input helper rules
- exercise the `HttpClient` plugin, including auxiliary `.test.g.dart` outputs
- keep all future annotation perf work on the same shared pipeline shape
- benchmark `dust build` and `dust watch` on a large input set

## Generate 5000 source files

```bash
cd examples/benchmark_project
flutter pub get
dart run tool/generate.dart --count 5000
```

This writes the generated sources into `lib/generated_models/`.

## Run Dust

From the Dust repository root:

```bash
cargo run -p dust_cli -- build --root examples/benchmark_project
```

## Analyze And Test

The runtime tests import selected generated models, so generate sources and run
Dust first.

```bash
cd examples/benchmark_project
flutter pub get
dart run tool/generate.dart --count 64

cd ../..
cargo run -p dust_cli -- build --root examples/benchmark_project

cd examples/benchmark_project
flutter analyze
flutter test
```

## Notes

- `lib/generated_models/` is ignored by Git through the local `.gitignore`
- the same folder is excluded from Dart analyzer through `analysis_options.yaml`
- the generator emits a stable mixed matrix of derive-only, nested, linked,
  codec-backed, and serde-configured models
- linked templates intentionally import earlier generated files so Dust keeps
  exercising shared workspace analysis, not only same-file generation
- the generator source is split by derive vs serde patterns so future benchmark
  additions follow the same cleanliness rules as the Rust workspace
- static `lib/support/http_*.dart` fixtures keep HTTP generation in the same
  benchmark package without bloating the file generator
- the local gate runs a smaller generated corpus for analyzer and runtime smoke
  coverage; the ignored perf test validates the full 5k corpus

## Perf Test

Run the ignored release benchmark test:

```bash
cargo test -p dust_cli benchmark_project_release_build_benchmark -- --ignored --nocapture
```

The default budgets are 2,000 ms cold, 800 ms warm, and 1,400 ms after
invalidating every cached tool hash. The invalidated build uses `--fail-fast`.
Run acceptance checks with these defaults.
The cold phase clears all Dust caches, including parser facts. Tool-hash
invalidation retains valid parser facts while rerunning analysis and emission.

After the performance run, restore the smaller fixture before normal checks:

```bash
cd examples/benchmark_project
./generate.sh --count 64
cd ../..
cargo run -p dust_cli -- clean --root examples/benchmark_project
cargo run -p dust_cli -- build --root examples/benchmark_project
```

See [../../CONTRIBUTING.md](../../CONTRIBUTING.md) for the normal contributor
workflow and [../../docs/developer.md](../../docs/developer.md) for the shared
pipeline rules this fixture is meant to protect.

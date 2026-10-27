# Manifold benchmarking guide

The project has three different measurement tools. They answer different
questions and their numbers are not interchangeable:

| Tool | Subject | Primary result |
| --- | --- | --- |
| Native `rbench` suite | `manifold-core` check, repair, and metrics kernels | Wall-clock median in `ns/op`; regression gate |
| `tools/manifold-bench` CPU sidecar | External `manifold-3d` fixture construction | Per-fixture `min`, `p50`, `p95`, and `max` in milliseconds |
| `tools/manifold-bench` memory sidecar | External `manifold-3d` dense-sphere fixture | `process.memoryUsage()` samples after explicit GC |

The sidecar package is deliberately not a product dependency. Its reports say
`product: "not-open-scad-viewer"`; do not present its CPU or memory numbers as
measurements of this project's kernel, TypeScript transport, workers, or UI.
The differential and native/WASM runners in the same directory are correctness
checks, not performance benchmarks.

## Native rbench suite

### Run it directly

From the repository root, use a release build and a fresh output path:

```sh
cargo run --locked --release --manifest-path crates/Cargo.toml \
  -p manifold-core --example bench_manifold -- \
  --profile normal --json --output tmp/manifold-rbench-normal
```

The output directory must not already exist. The structured report is
`tmp/manifold-rbench-normal/run.json`. Supported profiles are `quick`,
`normal`, and `thorough`. Use `quick` for the same short measurement contract
as CI; use a longer profile for local investigation, but record and compare
baselines with the same profile. Changing the profile changes the benchmark
contract, including its sample count, so mixed-profile reports are rejected.

The suite currently covers five cases: `check` at 1,280 and 5,120 triangles,
Conservative repair of an unwelded 1,280-triangle sphere, Full repair of its
defected variant, and `metrics` at 5,120 triangles. Each measured operation
asserts properties of its result, so returning a faster invalid answer fails
instead of producing an apparently good timing. Input creation and input drop
are outside the measured lifecycle; output drop is included.

### Record and check a local baseline

From `tools/manifold-bench`:

```sh
npm run test:rbench
npm run bench:kernel:record -- --baseline rbench-baseline.macos-aarch64.json
npm run bench:kernel:check -- --baseline rbench-baseline.macos-aarch64.json --max-regression 0.25
```

Both commands run the release suite with the default `quick` profile. Add
`--profile normal` or `--profile thorough` to both commands when deliberately
using another contract. Replace a stored baseline only after reviewing an
intentional performance change; never update it merely to make an unexplained
regression pass.

To compare reports that were already measured, without running either suite
again:

```sh
node rbench-regression.mjs compare \
  --baseline /path/to/base/run.json \
  --current /path/to/current/run.json \
  --max-regression 0.25 \
  --report tmp/rbench-comparison.json
```

The default report path for `check` and `compare` is
`tools/manifold-bench/tmp/rbench-comparison.json`. A failing regression check
still writes its report and exits nonzero.

### How the median and regression are calculated

An rbench observation is a **batch total**, not necessarily one operation. The
comparator first normalizes every available candidate wall-clock observation:

```text
sample_ns_per_op = observation.value / observation.operations
```

It then calculates:

1. the median of normalized samples within each process;
2. the median of those process medians for each case;
3. `regression = currentNs / baselineNs - 1` for matching cases.

Thus a positive `regression` is a slowdown, `0.10` means 10% slower, and a
negative value is an improvement. The comparison uses the normalized medians,
not raw batch totals. It rejects unavailable or incomplete samples, missing or
duplicate cases, changed case sets or contracts, invalid values, and OS or
architecture mismatches.

The allowed local and CI regression budget is 25% (`--max-regression 0.25`).
This is a regression-screening threshold, not proof that a smaller change is
statistically significant or irrelevant.

## What CI compares

The `manifold-performance` job checks out the base revision and the current
revision, builds both in release mode, and measures both sequentially with
`--profile quick` on the **same `macos-latest` runner**. It then compares the
two `run.json` reports with the same comparator and blocks a regression over
25%. The comparison JSON is uploaded as the `manifold-rbench-comparison`
artifact, including when a later step fails.

A stored workstation baseline is meaningful only for repeated measurements on
the same machine. Matching `os` and `arch` is necessary for the comparator but
is not sufficient to make different physical machines comparable. Never gate a
change by comparing absolute times from a developer workstation with a hosted
runner, or from two different runners.

For a suspicious result, stop competing builds, tests, and other heavy jobs;
keep power and thermal conditions stable; and repeat the complete base/current
comparison. Warmups and multiple samples reduce transient noise but do not
remove scheduler, thermal, or background-load effects. Investigate repeated
changes rather than selecting the fastest run.

## External manifold-3d CPU sidecar

From `tools/manifold-bench`:

```sh
npm ci
npm run bench:cpu -- --out tmp/cpu-baseline.json
npm run bench:cpu -- --quick --out tmp/cpu-quick.json
```

The default is two warmups followed by nine measured iterations per fixture.
`--quick` uses one warmup and three measured iterations. Explicit controls are
also available:

```sh
npm run bench:cpu -- \
  --iterations 15 --warmups 3 \
  --fixtures small-bracket,medium-csg \
  --out tmp/cpu-focused.json
```

The four fixture IDs are `small-bracket`, `medium-csg`, `many-bodies`, and
`dense-sphere`. The timed region includes direct `manifold-3d` fixture
construction, `getMesh()`, and `solid.delete()`; it does not parse OpenSCAD or
execute this project's manifold library. Warmups are excluded. Reported `p50`
is the median elapsed time per fixture in milliseconds; `p95` describes the
tail of this small sample, while `min` and `max` are individual extremes. The
CPU sidecar has no automatic cross-revision regression gate.

Use `--out` with a new filename: report creation uses exclusive file creation
and does not overwrite an existing report. The JSON records the package
version, options, Node version, OS, architecture, OS release, and CPU model so
that the measurement context remains visible.

## External manifold-3d memory sidecar

From `tools/manifold-bench`:

```sh
npm run bench:memory -- tmp/memory.json
```

The script runs with Node's `--expose-gc`, builds the `dense-sphere` fixture 15
times, releases the solid, performs explicit garbage collection around an
event-loop turn, and records `process.memoryUsage()` after each iteration. It
prints the first and last rows and writes the complete report. The output path
is optional; without one, a timestamped file is created under `tmp/`.

These rows are diagnostic samples, not a leak proof, a steady-state browser
heap measurement, or a peak-memory measurement. Explicit GC does not model
browser collection timing, and one fixture cannot characterize other topology
or allocation patterns. Compare like-for-like environments and investigate
monotonic retained growth with additional profiling before drawing a leak
conclusion. This sidecar is not a CI memory-regression gate.

## Choosing evidence for a change

- For a native `manifold-core` optimization, use the rbench base/current
  comparison and retain the comparison JSON.
- For a change to `manifold-ops`, ABI, TypeScript transport, or workers, do not
  claim coverage from the core microbenchmark alone; add a representative
  end-to-end measurement if performance is part of the change contract.
- Use the external CPU and memory sidecars only for their stated
  `manifold-3d` comparison purpose.
- Treat differential tests, ABI tests, and native/WASM precision comparisons
  as correctness evidence, never as timing evidence.

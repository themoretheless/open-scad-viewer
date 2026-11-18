# Manifold benchmarking guide

The project has three different measurement tools. They answer different
questions and their numbers are not interchangeable:

| Tool | Subject | Primary result |
| --- | --- | --- |
| Native `rbench` suites | `manifold-core` check, repair, metrics; `manifold-ops` boolean, diagnostics, hull, decimate | Wall-clock median in `ns/op`; regression gate |
| `profile_large` example | Same operations on 1M+ triangle meshes | Wall time and peak heap per operation |
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

The `manifold-core` suite covers five cases: `check` at 1,280 and 5,120
triangles, Conservative repair of an unwelded 1,280-triangle sphere, Full
repair of its defected variant, and `metrics` at 5,120 triangles. The
companion `manifold-ops` suite (`-p manifold-ops --example bench_manifold_ops`)
covers union and difference of two overlapping 320-triangle spheres,
self-intersection and mass-properties diagnostics, `hull_mesh`, and
decimation from 5,120 to 1,280 triangles, all on 5,120-triangle spheres. The
`polygon-core` suite (`-p polygon-core --example bench_csg_fixtures`) holds
the native `medium-csg` (panel minus 64 cylinders through the bridge policy)
and `dense-sphere` cases (overlapping `$fn=32` spheres through the BSP,
three separated `$fn=128` spheres); `--example csg_ladder` prints the wider
scaling ladder used by ADR 0013. Each measured operation
asserts properties of its result, so returning a faster invalid answer fails
instead of producing an apparently good timing. Input creation and input drop
are outside the measured lifecycle; output drop is included.

### Record and check a local baseline

From `tools/manifold-bench`:

```sh
npm run test:rbench
npm run bench:kernel:check      # compare with rbench-baseline.<os>-<arch>.json
npm run bench:kernel:record     # replace that platform's baseline
```

Both commands run both release suites with the default `thorough` profile
(100 x 10 ms observations, the CI contract) and pick the stored baseline
for the host, `rbench-baseline.<os>-<arch>.json` (for example
`rbench-baseline.macos-aarch64.json`); `--baseline FILE` overrides it.
`check --record-missing FILE` records into `FILE` instead of failing when the
host has no stored baseline. Add `--profile quick` or `--profile normal` to
both commands when deliberately using another contract. Cases that are new
since the baseline are listed as `NEW` and not gated; cases missing from the
current run are errors. Replace a stored baseline only after reviewing an
intentional performance change; never update it merely to make an unexplained
regression pass.

To compare reports that were already measured, without running either suite
again:

```sh
node rbench-regression.mjs compare \
  --baseline /path/to/base/run.json \
  --current /path/to/current/run.json \
  --max-regression 0.15 \
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
duplicate cases, baseline cases absent from the current run, changed
contracts, invalid values, and OS or architecture mismatches.

The allowed local and CI regression budget is 15% (`--max-regression 0.15`).
This is a regression-screening threshold, not proof that a smaller change is
statistically significant or irrelevant.

## What CI compares

The `manifold-performance` job checks out the base revision and the current
revision, builds both in release mode, and measures every suite present in
each (`rbench-regression.mjs measure --root CHECKOUT`) with `--profile
thorough` on the **same `macos-latest` runner**. It then compares the two
reports with the same comparator and blocks a regression over 15%; suites the
base lacks are reported as new. The comparison JSON is uploaded as the
`manifold-rbench-comparison` artifact, including when a later step fails.

The `manifold-performance-linux` job runs `check --record-missing` on
`ubuntu-latest`. When `tools/manifold-bench/rbench-baseline.linux-x86_64.json`
is in the tree it compares against it and fails over 15%; until then it only
records a baseline and uploads it as the `manifold-rbench-linux` artifact,
without failing. Hosted runners vary between jobs, so add a Linux baseline
only after checking that repeated artifacts agree within the budget.

### Stored baselines

| File | Machine | Profile | Recorded under |
| --- | --- | --- | --- |
| `rbench-baseline.macos-aarch64.json` | Apple M4 Max, 16 cores, 128 GiB | thorough | heavy background load (load average 26-35 from concurrent builds) |
| `rbench-baseline.linux-x86_64.json` | not stored yet | thorough | CI artifact pending review |

The macOS medians (ns/op): check 1,280 = 185k, check 5,120 = 901k, metrics
5,120 = 1.06M, conservative repair 1,280 = 378k, full repair 1,281 = 1.01M,
union 640 = 12.6M, difference 640 = 10.6M, self-intersections 5,120 = 6.89M,
mass properties 5,120 = 615k, hull 5,120 = 7.00M, decimate 5,120 = 5.41M. An
immediate re-check under the same load stayed within 2.3% on every case.

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

## Large-mesh profile (1M+ triangles)

```sh
cargo run --locked --release --manifest-path crates/Cargo.toml \
  -p manifold-ops --example profile_large -- 8 --json tmp/profile-large.json
```

The argument is the icosphere subdivision level (8 = 1,310,720 triangles,
655,362 vertices; 9 = 5.2M). Fixtures are deterministic. Each operation is
timed once with a wall clock; peak heap is the high-water mark of live heap
bytes above the level at operation start, counted by a global allocator in
the example (inputs excluded, outputs included). `--only PREFIX` runs one
operation, which is convenient under `sample` or Instruments. Booleans run on
two 320-triangle spheres: the BSP kernel exceeds its default work budget on
two overlapping 1,280-triangle spheres, so it is not a 1M+ operation today.

Level 8 on the machine above (Apple M4 Max, background load average 26-44,
best of three alternating runs of the old and new build):

| Operation | Before ms | Before peak MiB | After ms | After peak MiB |
| --- | ---: | ---: | ---: | ---: |
| check | 667 | 461 | 327 | 265 |
| metrics | 671 | 293 | 378 | 166 |
| repair, conservative (unwelded soup) | 1,552 | 661 | 641 | 391 |
| repair, full (unwelded soup) | 2,236 | 661 | 1,200 | 391 |
| mass properties | 178 | 60 | 180 | 60 |
| self-intersections | 2,466 | 956 | 2,466 | 956 |
| hull_mesh | 3,479 | 769 | 3,237 | 769 |
| decimate to 10% | 4,498 | 461 | 4,043 | 320 |

The dominant hot spots were hash maps and ordered maps keyed by edge, often
with a heap-allocated vector per edge: `check` built two such maps (connected
and orientable components), `metrics` built three hash sets per component, and
repair built a `BTreeMap<EdgeKey, Vec<_>>` in orientation unification, outward
orientation and non-manifold edge splitting, plus a hash map of per-triangle
adjacency vectors. Each is now one sorted flat list of edge uses with the same
iteration order, so outputs are bit-identical (checked by hashing check,
metrics, repair and decimate results of the old and new build on 45 clean and
defected meshes). The repair convergence check compares buffers in place
instead of copying the input and every pass output, and the exact weld no
longer reserves the whole unwelded input for its output. Decimate and hull
gain only through their internal `check`.

Remaining hot spots: decimation spends most of its time in the lazy-deletion
binary heap (5.7M pops for 590k collapses at level 8) and per-collapse
neighbour vectors; self-intersection keeps every broadphase candidate pair
(the largest peak, about 730 bytes per triangle); quickhull on a sphere is its
worst case because every point is a hull vertex.

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

- For a native `manifold-core` or `manifold-ops` optimization, use the rbench
  base/current comparison and retain the comparison JSON; for large-mesh
  behaviour add a `profile_large` before/after.
- For a change to the ABI, TypeScript transport, or workers, do not claim
  coverage from the native microbenchmarks alone; add a representative
  end-to-end measurement if performance is part of the change contract.
- Use the external CPU and memory sidecars only for their stated
  `manifold-3d` comparison purpose.
- Treat differential tests, ABI tests, and native/WASM precision comparisons
  as correctness evidence, never as timing evidence.

# CPU, allocation and WebGPU benchmarks

The benchmark suites exercise the current browser geometry evaluator and the
real viewport renderer. They are development tools, run separately from unit
tests. Machine-dependent timing thresholds do not gate correctness tests.

Measured findings and changes: [2026-09-07 results](performance-results-2026-09-07.md).
Retained-solid analysis and identical-workload CPU comparison:
[2026-09-19 results](solid-analysis-2026-09-19.md).
Continuous animation and draw-call optimization: [FPS results](performance-fps-2026-09-07.md).

## Run

```sh
npm run bench:cpu -- --out tmp/performance/cpu-baseline
npm run bench:cpu -- --profiles build,stl --out tmp/performance/cpu-profiles
npm run bench:gpu -- --out tmp/performance/gpu-baseline
npm run bench:gpu -- --heap-sampling --out tmp/performance/gpu-profiles
npm run bench:gpu -- --fps --fps-diagnostics --frames 100 --warmup 20 --out tmp/performance/fps
npm run bench:memory -- tmp/performance/cad-memory.json
npm run bench:workers -- tmp/performance/worker-lifecycle.json 5
npm run bench:nurbs-process -- tmp/performance/nurbs-process.json 5
npm run bench:bootstrap -- --out tmp/performance/wasm-bootstrap.json --samples 9
npm run bench:profiles -- --out tmp/performance/profile-triangulation.json
npm run bench:analysis -- --out tmp/performance/solid-analysis.json
node --import tsx benchmarks/own-cad/bench-csg-scaling.mts --out tmp/performance/csg-scaling.json

# Native Rust rbench examples (release binaries; `--profile quick` is a
# reproducible smoke profile and can be replaced with the desired profile).
cargo rbench run --program crates/target/release/examples/bench_boolean --protocol \
  --repetitions 8 -o .rbench/boolean -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_mesh_kernels --protocol \
  --repetitions 8 -o .rbench/mesh-kernels -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_sdf_cpu --protocol \
  --repetitions 8 -o .rbench/sdf-cpu -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_print_export --protocol \
  --repetitions 8 -o .rbench/print-export -- --profile quick --json
cargo rbench run --program crates/target/release/examples/bench_point_bounds --protocol \
  --repetitions 8 -o .rbench/point-bounds -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_point_moments --protocol \
  --repetitions 8 -o .rbench/point-moments -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_nearest_two --protocol \
  --repetitions 8 -o .rbench/nearest-two -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_chamfer --protocol \
  --repetitions 8 -o .rbench/chamfer -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_validation --protocol \
  --repetitions 8 -o .rbench/brep-validation -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_decode --protocol \
  --repetitions 8 -o .rbench/brep-decode -- --profile quick
cargo rbench run --program crates/target/release/examples/bench_matching --protocol \
  --repetitions 8 -o .rbench/matching -- --profile quick
```

Each output directory must be new. CPU defaults to four fixtures, two warmups
and nine measured repetitions; `--quick` is a smoke check. GPU defaults to
30 measured frames per case after eight warmups, eight scene replacements and
three half-second idle windows. Use `--help` for bounded workload controls.

`bench:workers` takes an optional new JSON output path and iteration count
(default 5, range 1..50). Each iteration starts three fresh production Node
workers: an expected assertion refusal, a successful cube build and a capability
probe. It checks that all workers are joined before settlement, the next request
works after a refusal, and the supervisor is neither busy nor quarantined.
Timings include worker startup, evaluation and teardown; the production join
deadline is unchanged. This is not browser edit latency. Run it without other
tests or benchmarks competing for CPU. Without an output path it prints the
full JSON report; an existing output file is never overwritten.

`bench:nurbs-process` has the same output-path/iteration arguments. It measures
the production disposable NURBS subprocess boundary on union, intersection,
difference and a 3.8 MB STL response. It includes parent validation, fresh child
startup, calculation, response transfer and child close. Each sample verifies
volume, closed topology and deterministic whole-response hashes. Compare the
fixture hash and response hashes before comparing timings across revisions.
It does not isolate native CSG throughput or browser interaction latency.

`bench:bootstrap` measures the compression-only Rust WASM decoder in a fresh
Node worker/isolate per sample. It separates unpacking the decoder itself,
compilation, instantiation, upload, base85/Brotli decode and owned output copy.
It verifies the exact decoded geometry artifact outside the measured interval.
Use repeated `--decoder /absolute/variant.wasm` options to compare up to four
decoder builds on identical input. Variant order alternates each iteration;
workers run sequentially. Import/startup/join and geometry compilation are not
included; process-wide V8 and OS caches can warm. The report fingerprints the
actual WASM variants and host inputs, and never overwrites an existing report.

`bench:profiles` builds the locked native release triangulation example. Three
warmups and nine timed samples cover synthetic hole grids and actual profiles
constructed by the prism Boolean path. Fixture construction and validation are
outside timing. Validation checks area, authored coordinates, every oriented
boundary segment and paired interior edges, not only volume. Reports contain
the full fixture arrays/hash, selected source and executable fingerprints,
raw samples and explicit refusals. This is not the whole Boolean or WASM path.

`bench-csg-scaling.mts` measures that production WASM path through
`parseOpenSCAD`, including evaluation and mesh analysis. It accepts case IDs,
`--samples` (default 7, range 1..50), `--warmups` (default 2, range 0..10), and a
new `--out` file. It records actual medians, raw phase samples, source hashes,
artifact fingerprints and environment. Volume/topology validation is outside
timing. Failures remain report rows, not fast successful samples; the runner
can finish successfully with refused cases. See the
[profile results and limits](profile-triangulation-2026-09-19.md).

`bench:analysis` isolates warm retained-solid export, display preparation, BVH,
semantic edges and the combined analysis call. Schema v2 also measures uncached
kernel inspection and host asset hashing as separate phases. Defaults are three warmups/nine
samples; `--warmups` accepts 0..20 and `--samples` 1..100. All returned buffer
bytes must be deterministic, and the combined result must match separate
calls. Reports retain input/output SHA-256, selected source/artifact hashes,
environment and every timing sample. Verification hashing/validation is outside
timing; only the explicit `assetHash` phase times product identity hashing.
Individual replays include their own transport/copies (and BVH/edge uploads),
so their times are not an additive breakdown of the combined call. Solid
construction, startup, provenance, workers and GPU are excluded. The combined
call excludes metrics and host hashing; the new probes measure them separately.
Inspection bypasses the `CadSolid` wrapper's cached report, since a repeated
`volume()` call on one wrapper would only measure cache access. Use `bench:cpu`
separately for the whole parser/build path.
Measured [dense display preparation results](mesh-render-2026-09-19.md) include
the byte-parity contract and limitations of comparing isolated replays.
The [inspection follow-up](mesh-inspection-2026-09-19.md) compares the original
edge tree, a sorted pair array and the retained packed-index implementation.

The GPU runner uses the repository's isolated Playwright package under
`tools/browser-qualification`, builds a separate production Vite entry, serves
it on loopback and launches a dedicated Chromium process. It does not install
packages or browsers. If the browser expected by the locked Playwright package
is absent, provision that browser or supply `--executable /absolute/browser/path`.
The executable and actual browser version are recorded in the report. A headed
hardware run is the default; `--headless` must be treated as a different setup.

For an A/B renderer comparison, preserve the original renderer file and pass
`--renderer-source /absolute/snapshot/webgpuRenderer.ts`. The runner substitutes
only that module at its original import path, saves the executed source and
bundle, and verifies the source-map content hash. This avoids modifying the
working tree between runs. Other imported modules must remain equivalent.

CPU `report.json` contains raw samples, phase summaries, output signatures,
machine metadata and source manifests. Optional `.cpuprofile` and `.heapprofile`
files can be opened in the corresponding profiler. GPU `results.json` separates
CPU submission, native GPU pass time, queue completion and GPU buffer counters;
`summary.tsv`, `viewport.png` and the immutable bundle accompany it. Optional
browser heap samples run after timings with a separate renderer without the
latency/counter wrappers. Profiling covers steady camera/section activity;
renderer initialization and mesh generation are excluded from those samples.

The memory soak repeats the dense fixture 15 times with forced JavaScript GC
between builds. It records the actual WebAssembly memory capacity, process
memory and whether transformed/normal-bearing kernel handles were deleted by
session cleanup. WASM capacity cannot shrink after growth; compare continued
growth after warmup against a plateau, rather than expecting it to return to the
initial allocation. A JSON report alone does not replace the handle-lifetime
correctness tests.

Synthetic GPU spheres deliberately include all tessellation edges to stress the
edge pass. Their index count includes degenerate pole triangles. This workload
is a rendering stress case, not a claim about edge counts produced by OpenSCAD.

The CPU `dense-sphere` fixture uses three `$fn=128` spheres (48,384 output
triangles). This intentionally remains below the kernel's documented
100,000-triangle per-mesh budget while keeping BVH, edge extraction, hashing
and export large enough to profile. Unsupported capacity probes, including
`$fn=192` and `$fn=256`, belong in the separate CSG scaling benchmark and are
recorded as expected refusals rather than making the standard CPU suite partial.

## Continuous FPS and separate GPU diagnostics

`--fps` runs continuous camera rotation, close zoom and section animation with
128 dense instances, 4096/16384 small instances, and a single dense mesh, in
shaded/edges/xray modes. Geometry and edge buffers are prepared before timing.
There are no per-frame GPU fences, timestamp readbacks or API counter wrappers
in this mode. A wrapper records CPU render duration and render-start intervals;
an idle requestAnimationFrame calibration records the browser refresh cadence.
The reported FPS is **frame submission cadence**, not physical display
presentation or whole-application FPS. GPU work is drained between scenarios.
Compilation, Vue updates and pointer picking are outside this measurement.

`--fps-diagnostics` adds a separate pass after FPS timing: native GPU timestamps,
queue-completion latency, draw counts and image captures for mixed materials,
mirrored transforms, selection/hover, clipping, isolation, scene replacement and
unique-geometry render bundles. `--diagnostics-only` runs just this pass.
Counters include draw commands replayed by bundles and instance counts; encoding
a reusable bundle does not itself count as an executed draw.

`results.json` contains the final report; `timing-results.json` preserves the
primary measurement before optional diagnostics. `fps-*.png` and
`validation-*.png` can be compared byte-for-byte between versions. GPU timestamp
diagnostics intentionally serialize samples and must not be converted into FPS.

## Measurement rules

- Run CPU and GPU suites sequentially, with other builds and timed workloads
  stopped. Repeat comparisons on the same machine, browser, viewport and power
  conditions. A single run is exploratory evidence.
- Preserve the working tree when taking a baseline. The revision alone is not
  sufficient for a dirty checkout: keep the source manifest and its digest with
  each report. Benchmark inputs and output checksums belong to the evidence.
- Separate cold initialization, warmups, steady samples and profiling passes.
  Heap/CPU sampling adds overhead and must not silently contaminate the primary
  timing distributions.
- Compiler phase durations are parts of the compiler invocation. Replayed
  helper stages are independent diagnostic experiments; do not add their times
  to the compiler phases or present them as an exact decomposition.
- Node evaluator measurements exclude browser Worker transport, Vue updates,
  application publication and GPU execution. Renderer measurements exclude
  source compilation and the application UI. Neither is end-to-end edit latency;
  the existing Build measurements panel covers the application boundaries.
- Heap and resident memory deltas are not allocation counts or proof of a leak.
  Forced-GC snapshots estimate retained memory; allocation sampling estimates
  sampled JavaScript allocations. Typed-array backing stores, WASM memory,
  driver allocations and GPU buffers require separate accounting.
- GPU queue completion includes queueing, submission and host notification.
  Render-pass timestamps, when supported and enabled, measure the GPU pass.
  Neither measures physical display presentation. Record unavailable timestamps
  explicitly, and identify software adapters rather than presenting them as
  hardware GPU evidence.

## Questions covered

| Workload | Diagnostic question |
| --- | --- |
| Small parametric part | How much does initialization cost relative to a warm edit? |
| Perforated CSG panel | Does boolean evaluation or mesh analysis dominate? |
| Repeated bodies | How much work scales per entity rather than per geometry asset? |
| Dense curved mesh | What do BVH, edge extraction, hashing, inspection and STL export cost? |
| Retained scene publication | Are unchanged geometry buffers reused, and how much CPU/GPU API work remains? |
| Shaded, transparent and edge rendering | Is the frame limited by draw submission, vertex/fragment work or extra passes? |
| Camera motion and scanning plane | Does animation allocate or upload geometry unnecessarily? |
| Settled idle renderer | Does rendering stop after pending work is complete? |

## Optimization decisions

For emitted-code duplication and the exact-solid main-thread route, see
[bundle attribution](bundle-audit-2026-09-19.md). Run `npm run audit:bundle` for
source-map attribution; rebuild without source maps before checking delivery size.
The [exact-solid worker check](exact-solid-worker-2026-09-19.md) exercises the built
browser worker, group replacement and hard cancellation, and records remaining
main-thread callback gaps separately from end-to-end wall time.
Its [cooperative validation follow-up](solid-receive-2026-09-19.md) documents the
CPU profile, shared sync/async validator, cancellation boundary and remaining
single-body latency.
Use `npm run bench:brep-validation` for isolated native B-rep inspection; the
[validated face sampler report](brep-validation-2026-09-19.md) separates those
measurements from the browser's complete source-to-body path.
`npm run bench:brep-decode` isolates owned value-tree decoding; its
[report](brep-decode-2026-09-19.md) includes canonical output hashes and explicitly
excludes input preparation from the measured interval.
`npm run bench:brep-dispatch` measures owned request decoding, validation and
response encoding in the production native dispatcher. The
[owned request report](owned-brep-request-2026-09-19.md) distinguishes that
interval from isolated decoding and browser/WASM performance.

Mechanical preview isolation: `node --import tsx benchmarks/mechanical-preview.mts`.
The [report](mechanical-preview-2026-09-19.md) separates full-detail geometry from
bounded display-only tessellation and measures the extra image-generation cost.

Direct history: `node --import tsx benchmarks/direct-history.mts [output-json]`.
The [report](direct-history-2026-09-19.md) measures undo/redo with defensive copies
and checks identical document hashes before and after snapshot-size retention.

Direct validation: `node --import tsx benchmarks/direct-validation.mts [output-json]`.
The [cache report](brep-inspection-cache-2026-09-19.md) measures warm validation
of the 20-body spinner and states the exact retained-key bound and its limits.
The [owned JSON follow-up](owned-json-2026-09-19.md) removes a redundant full-tree
copy while preserving number normalization and caller isolation.

The [ModelGraph runtime/schema split](modelgraph-runtime-split-2026-09-19.md)
records delivery-byte savings, compatibility hashes and real browser checks;
the existing bundle audit and exact-solid browser commands reproduce its checks.

WASM packaging: `npm run audit:wasm-package -- baseline.wasm candidate.wasm --out report.json`.
The [convergence experiment](wasm-package-audit-2026-09-19.md) records a rejected
size-only candidate and distinguishes compressed delivery bytes from raw WASM.

Changes should be selected from recorded baseline results and validated with the
same workloads and output checksums. Prioritize costs visible at realistic scene
sizes. Keep cancellation, resource ownership, mesh validation, mirrored winding,
camera depth and transparent ordering intact. Cross-frame caches need explicit
invalidation; persistent scratch storage needs bounded ownership.

The upstream official OpenSCAD runtime and experimental geometry-compute pilot
are separate execution paths and are outside these benchmark results.

Foreign `manifold-3d` comparison timings are also outside the product. Install
and run them only from [`tools/manifold-bench`](../../tools/manifold-bench/README.md);
do not add that package to the application lockfile.

The remaining manual Rust examples are `bench_profile_triangulation`,
`bench_point_cloud_stats`, `bench_distance_pairs`, `bench_local_planes`,
`bench_nearest_four`, `bench_transformed_bounds`, `bench_transformed_stats`,
`bench_transform_error`, `bench_point_plane`, `math_backend_report`,
`sdf_bench_gpu`,
`bench_lattice`, and `bench_printer_lan`. The first nine retain bespoke
multi-phase/diagnostic output and are not yet protocolized. GPU examples are
left manual because timing includes real device queues and backend selection;
`bench_printer_lan` is left manual because it measures real LAN/network
behavior. These should not be interpreted as rbench results.

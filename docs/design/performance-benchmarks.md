# CPU, allocation and WebGPU benchmarks

The benchmark suites exercise the current browser geometry evaluator and the
real viewport renderer. They are development tools, run separately from unit
tests. Machine-dependent timing thresholds do not gate correctness tests.

Measured findings and changes: [2026-09-07 results](performance-results-2026-09-07.md).
Continuous animation and draw-call optimization: [FPS results](performance-fps-2026-09-07.md).

## Run

```sh
npm run bench:cpu -- --out tmp/performance/cpu-baseline
npm run bench:cpu -- --profiles build,stl --out tmp/performance/cpu-profiles
npm run bench:gpu -- --out tmp/performance/gpu-baseline
npm run bench:gpu -- --heap-sampling --out tmp/performance/gpu-profiles
npm run bench:gpu -- --fps --fps-diagnostics --frames 100 --warmup 20 --out tmp/performance/fps
npm run bench:memory -- tmp/performance/cad-memory.json
```

Each output directory must be new. CPU defaults to four fixtures, two warmups
and nine measured repetitions; `--quick` is a smoke check. GPU defaults to
30 measured frames per case after eight warmups, eight scene replacements and
three half-second idle windows. Use `--help` for bounded workload controls.

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

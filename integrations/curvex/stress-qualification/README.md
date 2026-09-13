# Curvex complex strokes, numerical robustness and native GPU qualification

This follow-up extends the 2026-09-13 simple-stroke optimization. The kernel
still depends only on our `osv-math` crate. The original Curvex checkout is
read-only; `prepare.py` now includes the bounded render-cache migration.

## Production changes

- Arrangement construction classifies internal segments before allocating
  boundary vertex identities. Its broad phase chooses X or Y using exact
  projected candidate counts, avoiding a quadratic X sweep for long horizontal
  stroke strips. More than eight million candidates returns an explicit error
  before pair processing. Intersection, output and memory budgets also remain.
- Stroke offsets use local coordinates when document position dwarfs the path
  extent. This fixes tiny round strokes and retracing around coordinates 1e7.
- Fill bands without a representable binary64 interior sample are skipped;
  numerical endpoint inversions within 16 rounding units collapse to their
  shared endpoint. Larger inversions remain errors. This fixes an actual
  retraced-contour regression without accepting crossed triangles.
- The isolated Curvex renderer retains up to 8192 cache entries instead of 512,
  subject to a 64 MiB payload weight limit. Mesh weights use allocated buffer
  capacities; heterogeneous shadow ghosts use a conservative serialized size
  estimate. This is not a bound on total application RAM. LRU touches use a
  bounded generation queue with amortized constant work. Document revisions
  still invalidate the entire cache; oversized entries render without caching.

## Measured complex stroke mesh construction

| Workload | Before, ms | After, ms | Speedup |
| --- | ---: | ---: | ---: |
| crossing_80 | 1.366 | 1.234 | 1.11× |
| crossing_240_wide | 9.898 | 6.914 | 1.43× |
| wide_wave_400 | 66.605 | 55.353 | 1.20× |
| consumed_square | 0.238 | 0.168 | 1.42× |
| retraced_200 | 43.186 | 28.165 | 1.53× |
| dashed_wave | 10.341 | 9.620 | 1.07× |

[All paired samples and executable hashes](results/benchmark.json) are retained.
The wide 400-point wave still takes about 55 ms uncached on this run; complex
geometry is not universally real-time. Cache reuse addresses repeated frames.
Simple-stroke controls show about 3–8% (0.4–4.4 µs) extra cost from coordinate
validation, and fill controls range from 14% faster to 4% slower; see
[render-controls.json](results/render-controls.json). No universal speedup is claimed.

## Independent geometry oracle

`src/main.rs` generates 1024 deterministic random open/closed paths, including
retracing and self-intersections, widths up to 72 units, five scale factors
from 1e-6 through 1e6, and translated microgeometry at 1e7. For round joins and
caps, the exact filled region is the union of segment capsules. Point-to-segment
distance supplies an independent oracle. Samples within twice the requested
chord tolerance of the analytic boundary are excluded. Triangles use top-left
edge ownership, so shared edges count once. The final run checks **261987
samples without failures**; see [stress-1024.json](results/stress-1024.json).

Four native regression tests cover the discovered numerical failures, invalid
input and dense coincident inputs exceeding the pair budget. Development runs,
including the original failures and a corrected edge-ownership defect in the
test harness, remain explicitly labelled in `results/diagnostic`.

## Native GPU and large documents

`gpu_qualification.rs` uses the actual `Document`, `draw_shape_cached`, egui
and eframe/wgpu pipeline in a visible native window on Apple M5 / Metal. Three
synthetic documents contain 100, 1000 and 5000 visible shapes: rectangles,
wide self-crossing strokes, cubic strokes and gradient cubic strokes. Each
scene runs 110 frames, including a moving pan/zoom interval and restoration.
The test bypasses editor menus, selection and document I/O; it does not measure
full editing latency. First-paint preparation and per-frame CPU paint time are
separate from wall-clock frame intervals; there are no GPU timestamp queries.

| Visible shapes | Warm paint before, ms | Warm paint after, ms | Frame interval after, ms |
| --- | ---: | ---: | ---: |
| 100 | 0.088 | 0.090 | 16.694 |
| 1000 | 0.983 | 0.862 | 16.686 |
| 5000 | 38.139 | 4.454 | 16.652 |

The 5000-shape first frame still prepares geometry in about 38 ms. Its warm
paint time improves about 8.6×, and the observed frame interval returns to the
60 Hz presentation cadence. These are two observed runs, not a statistical
whole-editor FPS guarantee. [Raw frames](results/gpu-after.json) and
[before/after summaries](results/gpu-summary.json) separate the metrics.

All six final GPU captures were received. At 2000×2000 pixels, before/after
cache optimization and restored-view comparisons have **zero differing pixels**
for all three documents. Screenshots were also visually inspected. See
[gpu-pixel-comparison.json](results/gpu-pixel-comparison.json) and
[5000-shape capture](results/gpu-5000.png).

The initial run that launched a renamed executable did not return screenshots
and is not used for final GPU evidence; retrying from the build's normal
executable path completed all captures. Measurements are synthetic workloads
on one GPU, not an exhaustive GPU/driver compatibility or arbitrary-document
performance guarantee.

## Final verification

- **1494 Curvex tests pass**, with one existing ignored timing test also run
  explicitly. Doctests, all-target checking and release build pass. All 33
  qualified source hashes stayed unchanged and match the final workspace.
- **226 native tests pass** (including one unrelated BRep triangulation test in
  the shared checkout), and the rebuilt workspace WASM passes **six transport
  tests**. Build and test logs are retained under `results/logs`.
- **36 software-render captures** have exactly the same covered pixels as the
  preceding implementation, no changed active color exceeding 3/255, identical
  cached frames and analytic gradient error at most 2/255. Original Curvex
  coverage IoU remains at least 0.999754. The full native GPU comparisons above
  are separate from this software raster check.
- The latest [migration patch](results/curvex-migration.patch) applies to the
  clean original Curvex checkout. That checkout has not been switched over.
  [summary.json](results/summary.json) records the complete result and limits;
  [report.json](results/report.json) retains actual application test names,
  commands, timings and source hashes.

## Reproduction

Build both stress executables before benchmarking and stop other compilers:

```sh
cargo build --offline --release --manifest-path integrations/curvex/stress-qualification/Cargo.toml
integrations/curvex/stress-qualification/target/release/curvex-stress-qualification
python3 integrations/curvex/stress-qualification/compare.py /path/to/before /path/to/after results.json
cargo test --offline --manifest-path crates/Cargo.toml -p planar-geometry -p polygon-core
```

The before snapshot uses the preceding simple-stroke implementation: the files
qualified in `../stroke-optimization/report.json`, before this follow-up's
`rings.rs`, `stroke.rs` and `tessellation.rs` edits. Both stress executables use
the same harness. Benchmarks are two A/B/B/A cycles, with three warmups and
fifteen timed samples per case in each execution. Uncontrolled other machine
activity remains a limitation; every sample and executable hash is retained.

For a native GPU run, prepare a fresh isolated Curvex migration, copy
`gpu_qualification.rs` to `examples/osv_gpu_qualification.rs`, then run:

```sh
cargo run --offline --release --example osv_gpu_qualification -- /tmp/curvex-gpu-result
```

The window must be permitted to connect to the macOS window services. Run from
the normal build path and bound the process externally (e.g. a 120-second
subprocess timeout); verify six captures in the resulting JSON before accepting
its metrics. The standard `../qualify.py` runner verifies actual Curvex tests,
release, original/migrated render captures and unchanged kernel sources.

Finite precision and work limits are intentional. The new corpus substantially
extends coverage; it cannot prove correctness for every possible input.

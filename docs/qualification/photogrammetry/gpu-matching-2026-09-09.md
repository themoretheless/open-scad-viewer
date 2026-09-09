# GPU descriptor matching — qualification (2026-09-09)

Phase 0+1 of the GPU support plan: optional `gpu` crate feature (wgpu; Metal on
macOS, Vulkan elsewhere — NVIDIA included). The CPU path remains the default and
is byte-identical to the previous build; GPU matching is opt-in via
`FeatureOptions::acceleration = Acceleration::Gpu` and falls back to the CPU scan
when no adapter is present.

## Method

One thread per candidate pair computes the full 128-component squared distance;
workgroup reductions reproduce the CPU scan's tie-breaking (strict `<`, first
index wins; NaN never wins). GPU float contraction (fma) means distances are not
bit-identical to CPU — on synthetic 300×280 descriptors 132/300 rows were
bit-exact while all best/second selections agreed within contraction noise
(`gpu_matching_agrees_with_cpu_on_synthetic_descriptors`, skipped without an
adapter).

## Measured (Apple M4 Max, Metal, frozen PPM inputs, alternating CPU/GPU runs)

| Case | Limit | CPU, ms | GPU, ms | Speedup | Cameras | Points | RMSE |
| --- | --- | ---: | ---: | ---: | --- | --- | --- |
| monstree6 | 900 | 859 | 261 | 3.3× | 6/6 | 570 = 570 | 0.2604 = 0.2604 |
| monstree6 | 4000 | 12,604 | 666 | 18.9× | 6/6 | 2915 = 2915 | 0.2973 = 0.2973 |
| shell12 | 900 | 1,239 | 267 | 4.6× | 12/12 | 977 = 977 | 0.4081 = 0.4081 |
| shell12 | 4000 | 1,249 | 276 | 4.5× | 12/12 | 977 = 977 | 0.4081 = 0.4081 |

On these real datasets the GPU match sets produced identical registered cameras,
point counts and reprojection RMSE — no borderline flips observed. shell12 at
limit 4000 is unchanged because its images yield fewer than 2000 retained corners;
monstree6 benefits: raising the limit to 4000 reaches 2915 points (COLMAP
reference on the same inputs: 3281 points in 1.90 s) at 0.67 s instead of 12.6 s.

## Verification

- `cargo test --offline -p photogrammetry-kernel`: 115 passed (CPU default).
- `cargo test --offline -p photogrammetry-kernel --features gpu`: 116 passed
  (adds the GPU agreement test; M4 Max adapter present).
- `cargo test --offline -p photogrammetry-wasm`: 16 passed; the WASM build has no
  `gpu` feature and stays import-free.
- Photogrammetry vitest set (47 tests), `vue-tsc`, production build and
  `verify-dist` pass. The packed kernel grew ~22 kB from round-3 accuracy code;
  the total dist budget moved 2,990,000 → 3,010,000 with a documented comment.

## Limits

- GPU results are not guaranteed bit-identical to CPU (fma contraction); no flip
  was observed on the three real datasets or the synthetic suite above.
- Vulkan compatibility is unverified (no such machine here); shaders use only
  core WGSL (no f64, no atomics beyond none, workgroup size 256).
- Phase 2 (dense NCC sweep) and browser WebGPU enablement are separate phases.

## Rejected experiment: cascade hashing (approximate NN), 2026-09-09

A COLMAP-style cascade-hashing matcher (16 groups × 8 components, 8 sign bits per
group over fixed-seed LCG projections, exact distances only for bucket-sharing
candidates, same tie-breaking as the exhaustive scan) was implemented as an
opt-in `FeatureOptions::approximate_matching` and measured on the frozen real
sets. On shell6/monstree6/shell12 it produced **identical** cameras, points and
RMSE at limits 900 and 4000 — but was uniformly **slower**: ~2× at limit 900 and
3.3× at limit 4000 (monstree6: 39.9 s vs 12.2 s exhaustive). The round-2
early-exit cutoff already rejects most pairs within the first 16–32 components,
so the exhaustive scan costs ~O(N·32) per row, which hashing overhead (per-query
projection hashes plus candidate sort/dedup, plus full 128-component evaluation
of ~hundreds of candidates without cutoff momentum) cannot beat at these sizes.
The experiment was reverted; the measurement is recorded here as negative
evidence. The scaling lever for large feature limits is GPU matching (above),
not approximate NN on this descriptor distribution.

## Phase 2: GPU NCC depth sweep (2026-09-09)

Opt-in `DenseOptions::acceleration = Acceleration::Gpu` evaluates the
frontoparallel NCC sweep on the GPU (one thread per depth-map pixel, all 64
hypotheses × sources × patch taps, f32 arithmetic). Hypothesis selection,
per-pixel ranges, consistency, fusion and meshing remain on the CPU, as do the
SlantedPlane passes. Without an adapter the sweep silently falls back to CPU.

A unit test (`gpu_sweep_matches_cpu_on_shifted_plane`) reproduces a known
10 px-disparity plane: 1058/1058 valid pixels agree within 0.15 depth units.
An early draft read the reference patch from the source buffer; the unit test
caught it (F1 ≈ 0 on all scenes) and the fix was verified before any timing was
trusted.

### Analytic scenes (dense fixture, 5 scenes, patch 5x5, tolerance 0.04)

GPU vs CPU per scene: precision/completeness/F1/mean/p90 identical to 4 decimal
places on all five scenes; vertex counts differ by at most 6 (f32 contraction
at threshold margins). Sweep section time per scene: 315-377 ms CPU →
17-24 ms GPU (15-22×).

### Real frozen PPM sets (default dense preset, whole densify stage)

| Case | CPU dense, ms | GPU dense, ms | Speedup | Vertices | Faces |
| --- | --- | ---: | ---: | ---: | --- |
| shell6 | 368-397 | 76-90 | ~4.6× | 10,507 → 10,499 | 6,139 → 6,133 |
| monstree6 | 474-477 | 83-85 | ~5.6× | 19,422 → 19,423 | 18,210 = 18,210 |
| shell12 | 745 | 151-162 | ~4.8× | 17,158 → 17,147 | 10,782 → 10,759 |

The whole-stage speedup is lower than the sweep's because consistency, fusion
and meshing still run on the CPU. GPU-mode work counters honestly report the
full evaluation (no early exits), so they are larger than the CPU counters;
geometry is unaffected.

Verification: 117 kernel tests with `--features gpu`, 115 without,
16 WASM adapter tests, photogrammetry vitest subset, regenerated embedded WASM.

## Phase 3: browser WebGPU sweep (2026-09-09)

The browser now runs the depth sweep on the GPU without changing the
import-free synchronous WASM module: the kernel splits dense into
`prepare_host_sweep` / `densify_with_host_scores`, the adapter exposes
`photo_dense_prepare` (binary payload + the kernel's own WGSL text) and
`photo_dense_finish`, and the photogrammetry Worker dispatches the shared
shader through WebGPU (`src/services/photoGpuSweep.ts`). Async lives only in
TypeScript; the CPU path is unchanged and remains the fallback on any GPU
failure (the worker posts a warning when falling back). Eligibility: baseline
preset, resolution ≤ 128, adapter present.

### Verification

- Kernel: the prepare/finish split reproduces the native GPU sweep bit-for-bit
  on the analytic 30° scene (validated end-to-end) and the CPU sweep within
  tolerance in the permanent `host_sweep_split_matches_cpu_sweep` test.
- A browser probe (`tools/browser-qualification/photo-gpu-probe.ts`, headless
  Chromium, WebGPU in a Worker) reproduces the known shifted-plane answer
  exactly (best bin 10/16, score 1.0000). It caught a real integration bug —
  staging buffers missing COPY_DST — before the full smoke.
- Full browser smoke (`photo-gpu-smoke.mjs`, shell6 PNGs, focal equivalent
  85 mm): 6/6 registered, 597 sparse points, surface 10,506 vertices / 6,136
  triangles with no fallback warning — against the native GPU result of
  10,499/6,133 (±0.07%, cross-driver f32 noise) and the native CPU reference
  10,507/6,139 (the CPU fallback itself was verified on the same page).
- 115 kernel + 118 `--features gpu` + 16 WASM adapter tests, full vitest
  (2,233; the three MCP suites pass in isolation and remain timing-flaky under
  full-suite load — pre-existing), vue-tsc, production build, verify-dist
  (total budget 3,020,000 with a documented note).

## Geometry GPU phase A/B (2026-09-10)

Shared `crates/gpu-compute` (wgpu context, readback, bind helpers) extracted;
photogrammetry-kernel migrated onto it (115/118 tests green, no behavior change).
polygon-kernel now builds at opt-level=3: own-cad skadis warm median 310 → 240 ms
(-22.5%, identical triangles, oracle hash test green) at +68 kB packed; sdf/nurbs/
bridge at opt-level=3 added size without speed and stayed on the size profile.
Budgets documented in scripts/verify-dist.mjs.

SDF kernel gained an opt-in GPU grid sampler (feature `gpu`): flat postorder
field encoding (Translate folded into leaf parameters), a stack-machine WGSL
interpreter shared as `SDF_WGSL`, mesh-distance nodes (brute-force closest point
and solid-angle sign in scan order per grid point). Snap-to-zero, boundary
validation and marching-tetrahedra remain on the CPU.

Qualified (M4 Max, Metal): CSG smooth-union field values agree with CPU to
<0.01 absolute and extraction keeps the identical 80,818 triangles; signed
mesh-distance classification matches CPU exactly on a tetrahedron grid.
Measured: mesh-distance field 16^3 grid 1088 triangles — 405.8 ms CPU → 4.1 ms
GPU (99x); primitive smooth-union 64^3 — 51.1 → 45.1 ms (marching-tets
dominates cheap fields; native polygonize 51.7 ms vs 143 ms through the JS
dispatch path shows the Value transport costs ~90 ms for an 80k-triangle mesh,
a separate known finding). CPU defaults are bit-identical; GPU tests skip
without an adapter. Baseline record: output/geometry-stage-baseline.json.

## Geometry GPU phase C (2026-09-10): browser SDF sweep + warm CAD worker

The browser runs the same SDF grid sampler through WebGPU: geometry-bridge
gained `sdf_prepare`/`sdf_finish` dispatch ops (pending handles in the session;
finish reuses `polygonize_with_values`, so snap/validation/marching stay on the
kernel CPU path), and the modelgraph-text scene builder prefetches eligible
pure-SDF `sdf_tessellate` jobs before the synchronous evaluation
(`collectSdfJobs` in modelGraphNurbsKernel.ts; `tessellateSdfGpuAware` consumes
primed scores). Async lives in the worker only; any failure falls back to the
CPU `sdf_tessellate`. A browser probe (`tools/browser-qualification/
sdf-gpu-probe.ts`, headless Chromium) reproduced the unit-sphere field exactly
(center -10.000, corner +10.785).

mainSolidWorker is now a persistent warm worker with latest-wins semantics:
job errors reject without terminating, cancellation still terminates
(mid-computation interrupt is impossible otherwise); each CAD operation no
longer pays the WASM inflate+compile. tests/mainSolidWorker.test.ts updated to
pin the new contract (reuse after a job-level error).

Verification: sdf-kernel 5/5 with the gpu feature (3 without), geometry-bridge
10/10 (prepare/finish roundtrip reproduces the reference indices exactly and
rejects a reused handle), full vitest 2240/2240 (engineManifest evidence hashes
updated to the rebuilt kernel: wasm cdc42ed0, cargo lock d5b5636), vue-tsc,
production build, verify-dist (total budget 3,110,000, documented).

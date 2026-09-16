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

## Lattice field on GPU (2026-09-10)

geometry-bridge gained an opt-in GPU evaluation of the spatial-lattice implicit
field (`lattice_accelerated`, feature `gpu`): the recursive BVH is flattened to
plain arrays (preorder nodes + leaf triangle windows), and each grid point runs
the shared `LATTICE_WGSL` shader — iterative nearest-distance walk (min is
order-free, so the result matches CPU pruning exactly), ray-parity with
sort/dedup in f32, and the capsule/skin/wall composition. Points whose hit walk
overflows are marked NaN and recomputed exactly by the CPU closure; snap,
marching and the final solid audit stay on the CPU reference.

Measured (M4 Max, Metal; 27-node/54-edge strut grid on a 40 mm cube, 64.7k
triangles): CPU 85-90 ms → GPU 30-33 ms (2.8x) with **identical triangle counts**
and volume delta 0.000009% (f32 noise), both organic and plain variants. The
remaining time is the CPU marching/inspection tail. A permanent test
(`gpu_lattice_matches_cpu_reference`, skips without an adapter) pins counts and
volume. Max-legal graphs (125 nodes/400 edges/64^3) amplify the win; the
browser lattice path (prepare/finish like SDF) is the documented next step.

The lattice benchmark example lives at crates/geometry-bridge/examples/
bench_lattice.rs. Evidence hash updated for the rebuilt kernel (a1300675).

## Phase 4: batched GPU sweep with in-shader selection + accurate preset (2026-09-17)

Goal: dense GPU stage ≥30% faster and ≥10% more accurate on the analytic
ground-truth scenes.

### Speed

Profiling the 256² kernel bench showed the GPU sweep at ~7 ms/view while the
CPU `select_from_scores` took ~20 ms/view plus a 16 MB score readback per view:
selection and transfer, not correlation, were the bottleneck. Changes:

- New `sweep_select` WGSL entry point does hypothesis selection on the GPU
  (best score, uniqueness margin, per-pixel bin range) and returns one `vec4`
  per pixel instead of `hypotheses` floats. `pick_depth` semantics are
  replicated in f32 (`s < x` in f64 ⟺ `s < ceil_f32(x)`); the browser's `sweep`
  entry (raw scores, bindings 0..=5) is unchanged and a permanent test compiles
  it against the browser's six-binding layout.
- The gray atlas is uploaded once per densify call; every view's job is pushed
  into one `SweepBatch` and submitted once, with a single readback.
- Depth is rebuilt on the CPU in f64 from the selected bin + offset, so the
  geometry pipeline downstream is untouched.

Measured (M4 Max, Metal, interleaved before/after binaries, medians):

| Case | Before GPU, ms | After GPU, ms | Change | Geometry |
| --- | ---: | ---: | ---: | --- |
| Analytic 5 scenes, densify total | ~110 | ~48-56 | −49..−56% | identical F1 / errors |
| 256² synthetic kernel, densify | ~156 | ~42 | −73% (depth stage 112 → 14) | identical |
| shell6, densify | 83-127 | 31-40 | ≈ −62% | 10,499 / 6,133 = |
| monstree6, densify | 90-124 | 33-43 | ≈ −64% | 19,423 / 18,210 = |
| shell12, densify | 167-246 | 55-68 | ≈ −67% | 17,147 / 10,759 = |

Remaining GPU-mode cost is consistency + fusion on the CPU (256²: fusion
~18 ms, consistency ~6 ms vs depth 14 ms).

### Accuracy

Probes on the analytic scenes (defaults: mean surface error 0.012463, mean F1
0.892229): 128 hypotheses collapse F1 to 0.64 (the ±3-bin uniqueness window is
grid-coupled); sub-bin peak refinement gains only ~0.3% (the error is bias, not
discretization); radius-2 alone 0.0108 / F1 0.859; sparse prior alone
0.013 / F1 0.927. The qualified bundle is exposed as `DenseOptions::accurate()`
= `patch_radius 2 + dual_scale + sparse_depth_prior`:

| Profile | Mean surface error | Mean F1 | GPU, ms | CPU, ms |
| --- | ---: | ---: | ---: | ---: |
| default | 0.012463 | 0.892229 | ~48 | ~725 |
| accurate | 0.009068 (−27%) | 0.921327 (+3.3%) | ~105-119 | ~2,410 |

The accurate preset on the batched GPU path costs about what the default
preset cost before this phase, i.e. the speed win is what makes the accuracy
bundle affordable (native CPU is 20-30× slower on the same bundle). Real sets
with `PHOTO_ACCURACY=on PHOTO_ACCELERATION=gpu`: shell6 69-75 ms, monstree6
94-97 ms, shell12 133-162 ms (CPU: 1.2-3.4 s); the meshes are denser (e.g.
shell6 6,133 → 10,281 faces) — no ground truth exists for those sets, so the
accuracy claim rests on the analytic scenes only.

### Verification

- New `tests/dense_accuracy.rs`: `accurate()` must beat the defaults by ≥15%
  mean surface error and ≥0.02 F1 on the analytic scenes; with an adapter the
  GPU and CPU accurate surfaces must agree within 2% of vertices.
- `browser_sweep_entry_compiles_against_six_binding_layout` (negative-checked:
  binding `sweep_select` against the same layout fails validation).
- 117 kernel tests with `--features gpu`, 113 without, 16 WASM adapter tests,
  photogrammetry vitest subset (46), vue-tsc, regenerated embedded WASM.
- Bench harness: `cargo run --release -p photogrammetry-core --features gpu
  --example gpu_dense_bench` (`DENSE_ACCELERATION`, `DENSE_ACCURATE`,
  `DENSE_KERNEL_SIDE`, `DENSE_REPEAT`, … documented in the file header).

# osv-math

Published name for the workspace `math-core` leaf: small dense `f64` vector
helpers (`V2` / `V3`) and a shared typed `Error` / `Result`.

Rust crate name remains `math_core` (`use math_core::…`). Builds on **stable** Rust.

```toml
[dependencies]
math-core = { package = "osv-math", version = "0.1" }
```

## Crate layout

The crate root is a thin facade: downstream users still import from
`math_core::{...}`, while implementation is split by domain:

```text
src/
├─ lib.rs              # public facade: module wiring + pub use
├─ types.rs            # V2, V3, M3, ID
├─ error.rs            # Error, Result, ensure()
├─ acceleration.rs     # Cpu/Auto/Gpu/Cuda placement and size heuristics
├─ linalg.rs           # dense vector/matrix helpers, eigen/SVD/solve
├─ bounds.rs           # point-cloud bounds reduction
├─ nearest_neighbor.rs # CPU reference plus GPU/CUDA dispatch
├─ chamfer.rs          # point-cloud Chamfer distance over nearest-neighbor
├─ registration.rs     # rigid transform fitting and ICP
├─ gpu.rs              # portable wgpu backend: Metal/Vulkan/DX12/WebGPU
└─ cuda.rs             # CUDA driver/PTX backend
```

## `transform_points` — plain CPU math, no GPU/CUDA

`transform_points(points, m, t)` batches `q = M*p + t` over a point set, in
exact f64. There is **no** `transform_points_accelerated`: an earlier version
of this crate had one, but `examples/bench_gpu.rs` measured GPU and CUDA
placements slower than this CPU reference at every size from 1K to 5M points
— even with device buffers reused across calls — because 3 fused
multiply-adds per point is too little work to amortize kernel-launch and
host/device synchronization latency. This crate only ships GPU/CUDA
acceleration for operations that measurably win; see below.

## GPU / CUDA nearest-neighbor search

`nearest_neighbor_accelerated(queries, targets, acceleration)` finds, for
every query point, the index of its closest point in `targets` and the
squared distance to it. `Acceleration::Cpu` (default) is the exact f64
reference (`nearest_neighbor`), brute force in O(queries × targets);
`Acceleration::Auto` resolves from `queries.len() * targets.len()` using the
measured crossover below; `Acceleration::Gpu` (feature `gpu`) runs a portable
wgpu compute shader in f32, one thread per query scanning every target;
`Acceleration::Cuda` (feature `cuda`, needs the `gpu` feature too) runs a CUDA
driver-API PTX port of the same kernel, falling back to the wgpu shader and
then the CPU reference when a device or kernel is unavailable. Both optional features
require nightly Rust (through `gpu-compute`); regenerate the checked-in PTX
with `npm run build:cuda-kernels`. Device buffers are cached per query/target
capacity (grow-only), so repeated calls at a stable size reuse allocations
instead of recreating them.

Unlike the trivial affine transform above, nearest-neighbor search does
`target_count` work per query, giving the GPU/CUDA placements enough
arithmetic intensity to win. **Measured, not assumed:** `cargo run --release
-p osv-math --features cuda --example bench_gpu`, on an NVIDIA RTX 5090
(CUDA 13.4):

| queries × targets | cpu | gpu | cuda |
| --- | --- | --- | --- |
| 1,000 × 100 | 0.078 ms | 0.260 ms | 0.137 ms |
| 3,000 × 100 | 0.237 ms | 0.238 ms | 0.138 ms |
| 1,000 × 1,000 | 0.750 ms | 0.309 ms | 0.175 ms |
| 3,000 × 1,000 | 2.260 ms | 0.366 ms | 0.185 ms |
| 10,000 × 1,000 | 7.782 ms | 0.493 ms | 0.265 ms |
| 100,000 × 1,000 | 76.676 ms | 2.243 ms | 0.902 ms |
| 100,000 × 5,000 | 384.680 ms | 2.670 ms | 1.177 ms |
| 1,000,000 × 2,000 | 1592.680 ms | 19.780 ms | 8.181 ms |

At the smallest size CPU still wins (fixed per-call overhead dominates); CUDA
crosses over first, already winning by 3,000 × 100 (work = 300K), while wgpu
breaks even a bit later and pulls ahead from around 1M work upward — up to
~200x faster than CPU at 1M queries × 2K targets. `Acceleration::Cpu` remains
the right choice for small batches; pick `Gpu`/`Cuda` once your workload sits
in this regime, and re-run the benchmark for your own sizes/hardware before
relying on the numbers above.

Don't want to hand-tune that threshold yourself?
`Acceleration::recommended_for_nearest_neighbor(query_count, target_count)`
encodes the crossover above as a `queries * targets` "work" threshold (CUDA
from ~200K, wgpu from ~700K, falling back to `Cpu` below that and to whichever
placement is fastest if the multiplication would overflow `usize`).
Passing `Acceleration::Auto` to `nearest_neighbor_accelerated` applies that
recommendation directly. It's a starting point tuned to the RTX 5090 numbers
above, not a guarantee for every device — treat it as a reasonable default,
not a substitute for benchmarking workloads where the choice actually matters.

`nearest_two_accelerated(queries, targets, acceleration)` uses the same
placement model for exact top-2 nearest-neighbor search. It returns the best
and second-best target for each query, which is useful for ratio tests,
correspondence filtering and robust registration pipelines:

```rust
use math_core::{Acceleration, nearest_two_accelerated};

let pairs = nearest_two_accelerated(&queries, &targets, Acceleration::Auto);
let best = pairs[0][0];
let second = pairs[0][1];
```

Top-2 has the same O(queries × targets) arithmetic intensity, so it gets
dedicated WGSL/CUDA kernels (`nearest_two.wgsl`, `nearest_two.cu`) instead of
being built from two CPU passes. Device buffers are cached grow-only, matching
the top-1 nearest-neighbor path for repeated stable-size registration and
filtering workloads. Measured with `cargo run --release -p osv-math --features
cuda --example bench_nearest_two` on the RTX 5090:

| queries × targets | work | cpu | auto | gpu | cuda |
| --- | --- | --- | --- | --- | --- |
| 256 × 512 | 131K | 0.132 ms | 0.134 ms | 0.220 ms | 0.081 ms |
| 1,024 × 1,024 | 1.0M | 1.106 ms | 0.128 ms | 0.262 ms | 0.204 ms |
| 4,096 × 4,096 | 16.8M | 17.723 ms | 0.319 ms | 0.825 ms | 0.318 ms |
| 16,384 × 8,192 | 134M | 143.374 ms | 0.807 ms | 1.417 ms | 0.788 ms |

For runtime diagnostics, `cargo run -p osv-math --features cuda --example
backend_report` prints the placement labels, portable wgpu backend report
(`metal`, `vulkan`, `dx12`, or `webgpu`) and native CUDA device report when
available.

## Chamfer distance

`directed_chamfer_distance(queries, targets, acceleration)` computes the mean
nearest-neighbor squared distance from one point cloud into another.
`chamfer_distance(a, b, acceleration)` runs both directions and returns the
directed summaries plus symmetric mean/RMS and Hausdorff worst-case values.
`directed_hausdorff_distance` / `hausdorff_distance` expose the worst-case
metric directly:

```rust
use math_core::{Acceleration, chamfer_distance};

let score = chamfer_distance(&cloud_a, &cloud_b, Acceleration::Auto)?;
println!("symmetric RMS = {}", score.symmetric_rms_distance);
println!("Hausdorff = {}", score.hausdorff_distance);
```

This is a higher-level geometry metric over nearest-neighbor work. Device
placements use fused directed-Chamfer reduction kernels, so they read back one
sum/max pair per workgroup instead of one nearest-neighbor result per query.
The wgpu shader uses the shared backend tuning (`128`-wide workgroups on Metal,
`256` elsewhere).
Measured with `cargo run --release -p osv-math --features cuda --example
bench_chamfer` on the RTX 5090:

| cloud sizes | cpu | auto | gpu | cuda |
| --- | --- | --- | --- | --- |
| 1,000 × 1,000 | 1.486 ms | 0.274 ms | 0.537 ms | 0.280 ms |
| 5,000 × 2,000 | 15.087 ms | 0.632 ms | 1.307 ms | 0.592 ms |
| 20,000 × 5,000 | 150.253 ms | 1.719 ms | 3.960 ms | 1.679 ms |
| 100,000 × 10,000 | 1526.633 ms | 7.169 ms | 14.744 ms | 6.848 ms |

## Pairwise squared distances

`squared_distance_pairs(a, b)` computes one-to-one squared Euclidean distances
for equal-length point arrays. `squared_distance_pairs_accelerated(a, b,
acceleration)` adds explicit `Gpu`/`Cuda` offload through the same portable
wgpu and CUDA-driver layers as nearest-neighbor. Device buffers are cached
grow-only per thread for stable-size repeated calls.
`squared_distance_pair_sum_accelerated` uses a device-side partial reduction
for scalar RMSE/loss-style consumers, reading only partial sums back; that
reduction path also reuses grow-only device buffers.

This kernel is intentionally conservative in `Auto`: unlike nearest-neighbor,
it is only O(pair_count) with a few FLOPs per pair, so on discrete GPUs the
copy/readback cost dominates. Measured with `cargo run --release -p osv-math
--features cuda --example bench_distance_pairs` on the RTX 5090, explicit
device execution is correct but slower; `Auto` therefore stays on CPU to avoid
regressing callers:

| pairs | cpu | auto | gpu | cuda |
| --- | --- | --- | --- | --- |
| 10,000 | 0.009 ms | 0.008 ms | 0.322 ms | 0.098 ms |
| 100,000 | 0.130 ms | 0.068 ms | 3.224 ms | 0.859 ms |
| 250,000 | 0.322 ms | 0.347 ms | 8.167 ms | 2.035 ms |
| 1,000,000 | 1.933 ms | 2.126 ms | 28.189 ms | 8.822 ms |
| 3,000,000 | 6.184 ms | 6.143 ms | 85.045 ms | 24.921 ms |

Scalar sum reduction avoids reading all distances but still has to upload both
input point arrays, so `Auto` remains CPU there too:

| pairs | cpu sum | auto sum | gpu sum | cuda sum |
| --- | --- | --- | --- | --- |
| 10,000 | 0.014 ms | 0.013 ms | 0.321 ms | 0.101 ms |
| 100,000 | 0.140 ms | 0.130 ms | 3.236 ms | 0.765 ms |
| 250,000 | 0.499 ms | 0.500 ms | 7.807 ms | 2.198 ms |
| 1,000,000 | 3.809 ms | 2.809 ms | 29.933 ms | 7.079 ms |
| 3,000,000 | 8.414 ms | 8.885 ms | 79.100 ms | 21.326 ms |

## Point-cloud bounds

`point_bounds(points)` computes exact CPU axis-aligned bounds, center and
extent for a point cloud. `point_bounds_accelerated(points, acceleration)`
adds fused wgpu/CUDA reductions with grow-only buffers and the same
Metal/default workgroup tuning as Chamfer. Because this is O(points) with very
little arithmetic per point, `Auto` intentionally keeps the CPU reference;
explicit `Gpu`/`Cuda` remain available for integrated-GPU/low-copy
experiments or for validating backend behavior.

Measured with `cargo run --release -p osv-math --features cuda --example
bench_point_bounds` on the RTX 5090:

| points | cpu | auto | gpu | cuda |
| --- | --- | --- | --- | --- |
| 10,000 | 0.053 ms | 0.041 ms | 0.356 ms | 0.216 ms |
| 100,000 | 0.422 ms | 0.401 ms | 1.856 ms | 0.670 ms |
| 1,000,000 | 4.188 ms | 4.155 ms | 15.795 ms | 5.097 ms |
| 5,000,000 | 21.135 ms | 21.265 ms | 77.053 ms | 25.854 ms |

## ICP / rigid point-cloud registration

`rigid_transform(source, target)` computes the least-squares rigid transform
for known correspondences (Kabsch/SVD). `icp_register(source, target, options)`
adds iterative closest-point registration on top: each iteration transforms
the source, finds closest target correspondences with
`nearest_neighbor_accelerated`, rejects optional distance outliers, fits the
next rigid delta, and composes the final `RigidTransform`.

This is where the nearest-neighbor kernel becomes a higher-level primitive:
`IcpOptions::default()` uses `Acceleration::Auto`, so large ICP batches select
CUDA/wgpu automatically when available and otherwise stay on the CPU reference.
Measured with `cargo run --release -p osv-math --features cuda --example
icp_registration` on the same RTX 5090:

| points | work/iteration | cpu | auto/cuda | iterations |
| --- | --- | --- | --- | --- |
| 1,000 | 1,000,000 | 2.297 ms | 0.421 ms | 3 |
| 5,000 | 25,000,000 | 56.703 ms | 1.212 ms | 3 |
| 20,000 | 400,000,000 | 911.654 ms | 5.109 ms | 3 |

## License

MIT — see repository root `LICENSE`.

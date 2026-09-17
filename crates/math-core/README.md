# osv-math

Published name for the workspace `math-core` leaf: small dense `f64` vector
helpers (`V2` / `V3`) and a shared typed `Error` / `Result`.

Rust crate name remains `math_core` (`use math_core::…`). Builds on **stable** Rust.

```toml
[dependencies]
math-core = { package = "osv-math", version = "0.1" }
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
`Acceleration::Gpu` (feature `gpu`) runs a portable wgpu compute shader in
f32, one thread per query scanning every target; `Acceleration::Cuda`
(feature `cuda`, needs the `gpu` feature too) runs a CUDA driver-API PTX port
of the same kernel, falling back to the wgpu shader and then the CPU
reference when a device or kernel is unavailable. Both optional features
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
placement is fastest if the multiplication would overflow `usize`). It's a
starting point tuned to the RTX 5090 numbers above, not a guarantee for every
device — treat it as a reasonable default, not a substitute for benchmarking
workloads where the choice actually matters.

## License

MIT — see repository root `LICENSE`.

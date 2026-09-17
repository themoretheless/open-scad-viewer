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
| 1,000 × 100 | 0.081 ms | 0.235 ms | 0.134 ms |
| 10,000 × 1,000 | 7.988 ms | 0.535 ms | 0.277 ms |
| 100,000 × 1,000 | 79.548 ms | 2.170 ms | 0.874 ms |
| 100,000 × 5,000 | 385.195 ms | 2.638 ms | 1.184 ms |
| 1,000,000 × 2,000 | 1630.644 ms | 19.476 ms | 7.885 ms |

At the smallest size CPU still wins (fixed per-call overhead dominates), but
past roughly 10K queries × 1K targets both GPU placements pull ahead, up to
~200x faster than CPU at 1M queries × 2K targets. `Acceleration::Cpu` remains
the right choice for small batches; pick `Gpu`/`Cuda` once your workload sits
in this regime, and re-run the benchmark for your own sizes/hardware before
relying on the numbers above.

## License

MIT — see repository root `LICENSE`.

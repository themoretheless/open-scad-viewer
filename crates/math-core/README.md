# osv-math

Published name for the workspace `math-core` leaf: small dense `f64` vector
helpers (`V2` / `V3`) and a shared typed `Error` / `Result`.

Rust crate name remains `math_core` (`use math_core::…`). Builds on **stable** Rust.

```toml
[dependencies]
math-core = { package = "osv-math", version = "0.1" }
```

## GPU / CUDA batch transform

`transform_points_accelerated(points, m, t, acceleration)` batches `q = M*p + t`
over a point set. `Acceleration::Cpu` (default) is the exact f64 reference
(`transform_points`); `Acceleration::Gpu` (feature `gpu`) runs a portable wgpu
compute shader in f32; `Acceleration::Cuda` (feature `cuda`, needs the `gpu`
feature too) runs a CUDA driver-API PTX port, falling back to the wgpu shader
and then the CPU reference when a device or kernel is unavailable. Both
optional features require nightly Rust (through `gpu-compute`); regenerate the
checked-in PTX with `npm run build:cuda-kernels`. Device buffers are cached
per point-set size (grow-only), so repeated calls at a stable size reuse
allocations instead of recreating them.

**Measured, not assumed:** `cargo run --release -p osv-math --features cuda
--example bench_gpu` compares placements on this affine transform. On an
NVIDIA RTX 5090 (CUDA 13.4), the CPU reference is faster than both GPU
placements at every size tried, from 1K to 5M points — the arithmetic per
point (3 fused multiply-adds) is too cheap to amortize kernel-launch and
host/device synchronization latency, even with buffers reused across calls.
GPU/CUDA offload pays off in this workspace for kernels with much higher
arithmetic intensity per point, e.g. `sdf-core`'s mesh-distance sampling
(brute-force nearest triangle per grid node), where the same bench pattern
shows a large win. Treat `transform_points_accelerated` as a correctness
reference/building block, not a general performance win, unless benchmarked
for your specific workload.

## License

MIT — see repository root `LICENSE`.

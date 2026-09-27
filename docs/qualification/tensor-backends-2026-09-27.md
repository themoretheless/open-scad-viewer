# Native tensor backend qualification

Date: 2026-09-27. Host: Apple silicon macOS, Metal GPU. Scope: new
`tensor-core`, resident WGSL tensors, `compute-cuda` and `compute-mlx`.

This records the initial f32 checkpoint. See the subsequent
[typed indexing qualification](tensor-indexing-2026-09-27.md) for current f32/u32
coverage, CUDA source compilation and WASM compile checks.

## Result

The shared f32 tensor contract executes on both WGSL/Metal and MLX/Metal. CUDA
Rust code and host contracts pass, but NVIDIA execution is not qualified on this
host. The strict all-backend report deliberately has `passed: false`: requesting
CUDA on a machine without a CUDA driver/device fails instead of silently passing.

| Check | Result |
| --- | --- |
| Shape/layout contracts | 7 CPU tests pass, including empty strided endpoint slices |
| WGSL tensor operations and shared conformance | 12 actual GPU tests pass |
| MLX | 6 actual GPU integration tests and 2 loader/error tests pass |
| CUDA precision/layout/loader contracts | 5 CPU tests pass |
| Required CUDA execution | Fails explicitly: no CUDA driver/device on this host |
| Existing platform/compute/math/raster regression | 222 tests pass with GPU, timestamp and subgroup requirements enabled |
| CPU-only math | 68 tests pass |
| All-feature math/SDF/geometry/photogrammetry consumers | Compile successfully |
| New tensor crates + compute-core | Strict Clippy for all targets and strict rustdoc pass |
| Dependency architecture | CPU-only contracts, backend isolation and dependency directions pass |

The shared fixture covers logical row-major reshape after permutation,
broadcasting, all nine unary and six binary operations, multi-axis sum,
keep-dims, scalar/empty shapes, batched strided matmul and zero contractions.
Backend-specific tests cover ownership, aliases, dispatch limits, native errors,
buffer tails and repeated programs. The full regression includes the existing
scan, compaction, fusion, matrix, domain math, profiling and raster paths.
Independent review also found and fixed a CUDA initialization error: a loadable
older NVRTC/cuBLAS library with missing symbols now returns `MissingSymbol`
before cudarc can panic. The archived CUDA result was refreshed after this fix.

Existing unrelated warnings remain: the `wgsl_export` binary name and a doc
comment on a macro in `geometry-bridge`. CI configuration now includes the
architecture checker and strict Clippy for the new adapters; remote CI has not
been run for this working-tree change.

## Evidence

- [Machine-readable report](tensor-backends-2026-09-27/report.json).
- [Shape/layout tests](tensor-backends-2026-09-27/contracts.txt).
- [WGSL tests](tensor-backends-2026-09-27/wgsl.txt).
- [MLX tests](tensor-backends-2026-09-27/mlx.txt).
- [CUDA host tests and required-device failure](tensor-backends-2026-09-27/cuda.txt).
- [Full native regression](tensor-backends-2026-09-27/gpu-regression.txt).
- [MLX versions, ABI and operation limits](../../crates/compute-mlx/README.md).
- [CUDA requirements and precision policies](../../crates/compute-cuda/README.md).

## Measured WGSL reduction improvement

The tensor reduction now distributes large strided contractions across multiple
workgroups and folds bounded resident partials. A transpose of 1,049,600 f32
values reduced to a scalar changes median host latency from **2.788 ms to
0.172 ms**. The separately measured GPU interval changes from **2.609 ms to
0.0383 ms**. Small reductions show no meaningful host improvement.

The comparison uses 31 rotated samples against the frozen initial tensor
reduction shader. Preparation/upload are outside the samples; host timing
includes final readback. Every sample is checked against an f64 reference.
This is a result for that workload on Metal, not a CUDA/MLX comparison or an
application speedup. [Full method, raw samples and source hashes](../../crates/compute-core/benchmarks/tensor-research.md).

## Scope still open

CUDA kernel compilation, numerical execution and actual Tensor Core selection
need NVIDIA hardware. Full typed operation parity, additional reductions,
device-index operations, native f16/bf16 storage, graph replay and domain
integration remain tracked in the [architecture and coverage plan](../design/tensor-backends-2026-09-27.md).

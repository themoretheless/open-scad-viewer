# Tensor statistics and normalization qualification

Date: 2026-09-27. This extends the
[low-precision phase](tensor-low-precision-2026-09-27.md) with f32 softmax,
log-softmax, logsumexp, population moments and layer normalization across
WGSL, CUDA and MLX. The broader compute objective remains in progress.

## Implemented behavior

`TensorStatsBackend` accepts arbitrary unique axes, including unsorted axes,
strided views, storage offsets and broadcasts. Softmax, log-softmax and layer
norm preserve shape. Logsumexp and moments support `keep_dims`. Moments return
separate resident mean and variance tensors. Gamma/beta affine transforms
compose with the existing binary operations.

All tensor intermediates remain on the device. WGSL supplies recorded methods
and `_into` variants; ownership, layout and alias checks complete before any
stages are appended to the caller's program. Repeated execution overwrites
scratch and responds to changed input buffers.

| Backend | Stable calculation | Execution evidence |
| --- | --- | --- |
| WGSL | Max-shifted exponentials; midpoint anchor and scaled centered moments; one workgroup for 2–256 values, bounded summaries for larger groups | Actual Metal execution |
| CUDA | Max-shifted exponentials; anchored and centered f64 device reductions, f32 public results | Host checks and NVRTC compilation; NVIDIA execution pending |
| MLX | Native precise softmax and logsumexp; shifted log-softmax; scaled centered moments in the native graph | Actual Metal execution |

Inputs are finite f32 values. Layer normalization remains usable when the
original variance exceeds f32 range. Mathematically out-of-range variance and
log probabilities may be infinite. Epsilon must be finite and strictly
positive, including positive subnormal parameters. Constant groups normalize
to zero. Empty axes form singleton groups; empty normalized outputs retain
their shape. Reduced empty contractions fail only when the output is nonempty.

### Numerical evidence

The shared fixture uses independent f64 anchored references. It exercises
large common offsets, constant 1e30 groups, opposite f32 extrema, tiny epsilon,
arbitrary axes, transposes, broadcasts, scalar/empty cases and a 131,077-element
contraction. Probability checks remain relative for small normal values.
An all-zero long-row softmax cannot pass through a broad absolute tolerance.

WGSL tests also replay strided programs with input/output offsets and sentinels
at group lengths 1, 255, 256, 257, 4096, 4097 and 131077. Independent groups
numbering 65,537 cover dispatch grid limits and changed-input reuse. Variance
overflow boundaries, rejected aliases and program integrity after errors are
checked. MLX adds explicit native dtype and ownership errors. The CUDA fixture
includes a resident moments → normalization → softmax → sum chain, but awaits
NVIDIA execution.

The initial WGSL private test incorrectly demanded relative accuracy for a
subnormal normalized output. Metal returned signed zero for `[-1e-20, 1e-20]`
with epsilon `f32::MAX`, where f64 gives approximately ±5.421e-40. The test now
uses the documented `f32::MIN_POSITIVE` underflow floor. This corrected the
assertion, with no production math change for that finding.
[Original captured failure](../../crates/compute-core/benchmarks/tensor-normalization-initial-failure.txt),
[exact diagnostic values](../../crates/compute-core/benchmarks/tensor-normalization-underflow-diagnostic.txt),
[full numerical contract](../../crates/compute-core/benchmarks/tensor-normalization-contracts.md).

## Verification

| Check | Result |
| --- | --- |
| gpu-compute / compute-core / raster-core / osv-math regression with required GPU, timestamps and subgroups | **249 passed, 0 failed, 0 ignored**, 38 suites |
| Focused WGSL statistics suite | **5 passed** on Metal |
| Shipped WGSL sources validated by Naga | **42 passed** |
| Complete MLX suite with required native runtime | **35 passed** on Metal; MLX-C 0.6.0, MLX 0.32.1 |
| Shared CPU contracts | **20 passed** |
| CUDA host tests / doctest | **10 / 1 passed** |
| CUDA NVRTC | **35 kernels × 4 architectures**, entry parameter ABI checked |
| Strict Clippy and rustdoc, four tensor crates | Pass |
| wasm32 compilation: tensor-core, compute-core, compute-mlx | Pass |
| Dependency architecture checker | Pass |
| NVIDIA numerical execution and Tensor Core instruction profiling | Pending hardware |

The full regression uses:

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math \
  --features osv-math/gpu -- --test-threads=1
```

[GPU regression](tensor-statistics-2026-09-27/gpu-regression.txt),
[focused WGSL](../../crates/compute-core/benchmarks/tensor-normalization-small-metal-tests.txt),
[MLX](../../crates/compute-mlx/qualification/statistics-metal.txt),
[CPU contracts](tensor-statistics-2026-09-27/contracts.txt),
[CUDA host](tensor-statistics-2026-09-27/cuda-host.txt),
[CUDA doctest](tensor-statistics-2026-09-27/cuda-doctest.txt),
[Clippy](tensor-statistics-2026-09-27/clippy.txt),
[rustdoc](tensor-statistics-2026-09-27/rustdoc.txt),
[WASM](tensor-statistics-2026-09-27/wasm.txt).
Existing manifest warnings concern `wgsl_export` naming and duplicate `bench`
example filenames when testing several crates together.

### CUDA compiler boundary

NVRTC 12.8.93 compiled the runtime's exact six-part source for compute_70,
compute_80, compute_90 and compute_120, using its precise-math options.
The 27,896-byte source SHA256 is
`e9ccb56638013e9d7e60d0266206e6d225498578abf802143e72c2ca7bbe5cb3`.
Source-part, combined-source and four retained PTX hashes were verified against
the [report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-statistics/report.json).
The five new entrypoints are `stats_partial`, `stats_merge`, `stats_emit`,
`stats_lse` and `stats_moments`. Compiler success does not validate driver
launches, numerical output, cuBLAS behavior or Tensor Core use. Internal f64
arithmetic may be costly on some NVIDIA devices; no CUDA timing claim is made.

## Measured WGSL performance

The [softmax report](../../crates/compute-core/benchmarks/tensor-normalization.md)
compares the new operation with max → subtract → exp → sum → divide on resident
buffers. Final median GPU time decreased 61.8% on 32 × 33 and 15.7–42.2% on
three approximately million-element workloads. Every execution passed an
independent f64 reference check. Host timings include final readback.

An initial short-group regression led to the one-workgroup specialization.
Both raw runs, the initial frozen sources and final source fingerprints are
retained. These shape-specific Metal results do not establish performance for
other statistics operations or other backends.

## Remaining scope

General tensor parity, broader numerical operations, low-precision operations
beyond matmul, CUDA/MLX graph reuse, other devices/browser targets and hardware
CI qualification remain in the
[full compute plan](../design/tensor-backends-2026-09-27.md). NVIDIA execution
and Tensor Core profiling require an available NVIDIA machine.

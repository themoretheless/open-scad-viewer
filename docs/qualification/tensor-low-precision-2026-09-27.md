# f16/bf16 tensor qualification

Date: 2026-09-27. This extends the
[scatter qualification](tensor-scatter-2026-09-27.md) with low-precision storage,
GPU casts and matrix products. It does not establish complete tensor coverage
or NVIDIA runtime execution.

## Implemented behavior

`TensorLowBackend` uses a separate associated low tensor type and an explicit
`LowDtype`. Raw u16 upload/read, materialization and views preserve all bits.
Casts round to nearest with ties to even, preserve signed zero and representable
subnormals, and overflow to infinity. Cast NaN payloads are unspecified.

| Backend | Input storage | Accumulation | Low result | Direct f32 result |
| --- | --- | --- | --- | --- |
| WGSL | Two 16-bit elements per u32, odd-lane padding | f32 tiles | One final device cast | Supported; operands decoded during tile loads |
| CUDA | Native two-byte allocation | cuBLAS COMPUTE_32F | One final device cast | Supported; native low input pointers passed to cuBLAS |
| MLX | Native two-byte allocation | Native matmul f32 accumulation | Native low result | Explicit unsupported error |

Whole operands are not expanded to f32 for these products. CUDA validates
cuBLAS/device support per dtype; BF16 GEMM requires SM80 or later. The WGSL
storage mode is explicitly `Packed16x2` and does not require `SHADER_F16`.
No Tensor Core instruction execution is claimed without an NVIDIA device run.

## Numerical and integration evidence

The same shared fixture runs through WGSL and MLX on the local Metal device:

- Every one of the 65,536 raw patterns per format survives upload, transpose,
  materialization and readback, including signed zeros and NaN payloads.
- All finite patterns widen to the expected f32 bits and re-encode exactly.
  NaNs retain their classification.
- Every positive rounding midpoint and both neighboring f32 values, together
  with their negative mirrors, obey ties-to-even. This includes subnormal and
  overflow boundaries.
- Products cover rank-one promotion, scalar dot products, broadcast batches,
  strided matrices, partial 17 × 19 × 21 tiles, empty outputs and K=0.
- Cancellation checks f32 accumulation. A midpoint-valued result distinguishes
  direct f32 output from a low result that was widened after rounding.
- Dtype mismatches, invalid shapes, foreign runtimes and invalid layouts fail.

WGSL-specific tests preserve adjacent halfwords when writing at odd offsets,
check physical allocation bytes, reject cross-type aliases and replay a resident
cast → strided matmul → low output → f32 → sum program with changed inputs.
MLX adds independent randomized conversion checks using `half` as a test-only
oracle and an odd-sized native GEMM reference.

### MLX defect found and fixed

MLX 0.32.1's native f32-to-bf16 cast flushed f32 subnormals. For example,
f32 bits `0x00010000` became bf16 `0x0000` instead of `0x0001`.
The adapter now reinterprets bits as u32, performs unsigned ties-to-even
rounding in the device graph and views the resulting u16 words as bf16.
No input readback or CPU conversion is used.

The [initial failure](../../crates/compute-mlx/qualification/low-bf16-initial-failure.txt)
and [corrected complete MLX run](../../crates/compute-mlx/qualification/low-metal.txt)
are retained. The final run has **29 passed, 0 failed** with
`COMPUTE_REQUIRE_MLX=1`, MLX-C 0.6.0 and MLX 0.32.1.

## Verification boundary

| Check | Result |
| --- | --- |
| Full gpu-compute / compute-core / raster-core / osv-math GPU regression | **244 passed, 0 failed, 0 ignored**, 37 suites; required GPU/timestamps/subgroups |
| New WGSL low suite plus shared f32 matmul regression | **4 + 10 passed** on Metal |
| MLX complete adapter suite | **29 passed** on Metal |
| Shared CPU contracts | **18 passed**, including mathematical low reference |
| CUDA host unit tests | **9 passed**; doctest passes |
| CUDA NVRTC | **30 kernels × 4 architectures**, exact PTX entry parameter ABI checked |
| Strict Clippy, all targets, four tensor crates | Pass |
| Strict rustdoc, four tensor crates | Pass |
| wasm32 compile: tensor-core, compute-core, compute-mlx | Pass; native adapters remain appropriately gated |
| Dependency architecture checker | Pass |
| NVIDIA numerical execution / Tensor Core profiling | **Pending hardware** |

The full GPU command and [raw output](tensor-low-precision-2026-09-27/gpu-regression.txt):

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math \
  --features osv-math/gpu -- --test-threads=1
```

[Clippy](tensor-low-precision-2026-09-27/clippy.txt),
[rustdoc](tensor-low-precision-2026-09-27/rustdoc.txt),
[WASM](tensor-low-precision-2026-09-27/wasm.txt),
[focused WGSL run](../../crates/compute-core/benchmarks/tensor-low-metal-tests.txt).
Existing unrelated warnings remain: the `wgsl_export` executable name and
duplicate `bench` example output names when testing several crates together.

### CUDA compiler evidence

The [NVRTC report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-precision/report.json)
records successful compilation for compute_70, compute_80, compute_90 and
compute_120 using NVIDIA NVRTC 12.8.93 in an ephemeral Linux ARM64 container.
The report includes compiler package/library hashes, source parts, options,
PTX hashes, kernel names and parameter widths. The current 22,393-byte
concatenated source hash is
`27905752227d9807469d6435d7f76a2226092c8af4ea73c8d906d343ab900e0b`.
Source and all four retained PTX hashes were verified against the report.
PTX contains native u16 loads/stores and f16 conversion instructions.

This does not validate driver launches, cuBLAS numerical results or Tensor Core
use. The shared low fixture and CUDA-specific byte-size, offset, million-element
tail and ownership tests compile, but await execution on NVIDIA hardware.
Required-CUDA mode still fails explicitly on this host's absent driver/device.

## Measured storage and timing

The [benchmark report](../../crates/compute-core/benchmarks/tensor-low.md) retains
raw samples and source fingerprints for three matrix shapes. Input storage is
approximately halved. On the two larger shapes, packed f16 GPU medians were
11.7–13.8% slower than f32, and packed bf16 was 3.6–7.6% slower. Every measured
output matched an independent f64 product exactly. These are storage and timing
observations for this portable WGSL implementation, not a general speedup claim.

## Remaining scope

Low storage/casts/matmul are implemented; low elementwise/reduction coverage,
native WGSL f16 execution and direct MLX low-input/f32-output matmul remain.
NVIDIA hardware qualification, graph reuse across CUDA/MLX, wider numerical
coverage, other GPU/browser targets and CI deployment proof remain part of the
[full compute objective](../design/tensor-backends-2026-09-27.md).

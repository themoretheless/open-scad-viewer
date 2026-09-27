# Native CUDA binary64 and resident geometry — 2026-09-27

The public f64 path is implemented and compiler-qualified. NVIDIA numerical
execution remains unqualified: this host is an Apple M4 Max with no CUDA device.
Required-device failures below are evidence of that missing gate, not passes.

## Implementation

- `TensorF64Backend` defines distinct binary64 storage/arithmetic, views, explicit
  read/evaluation and matrix products. `Float64Support` reports native, software
  binary64 or unsupported arithmetic. WGSL/MLX currently report unsupported.
- CUDA implements f64 upload/read/write, materialization, reshape/permutation,
  broadcasting/narrowing, all existing unary/binary operators, axis/full
  sum/product/min/max, mean and vector/batched matrix products. No f32 data or
  scalar conversion is used in this path. Min/max preserve signed-zero ordering.
- CUDA reuses typed elementwise descriptors and the existing reduction templates
  and launch plans. Reduction storage types are independent of the indexing
  catalog, avoiding an accidental f64 index into its f32/u32 kernel arrays.
- Shared GEMM launch now couples output storage, alpha/beta and compute mode.
  Binary64 uses `CUDA_R_64F` and `CUBLAS_COMPUTE_64F_PEDANTIC`, following the
  [cuBLAS scale-type table](https://docs.nvidia.com/cuda/archive/12.8.0/cublas/index.html#cublasgemmex).
  F64 output extent checks use eight-byte elements, including K=0 plans.
- `TensorMathF64` preserves original coordinates for resident transforms, bounds,
  pair distances and centered population covariance. Centering uses differences
  relative to the first resident point; it never subtracts raw second moments.

Finite intermediates and valid unary domains are required. Sum-based mean can
overflow, and covariance is not an arbitrary-range algorithm. Reduction order
and transcendental rounding may differ from CPU. No performance claim is made.

## Verification

Commands, exit codes and logs are in [checks.json](tensor-float64-2026-09-27/checks.json).
Log trailing whitespace is normalized. The native regression runner records its exact backend selection and required
flags in [native/report.json](tensor-float64-2026-09-27/native/report.json).

| Check | Result | Scope |
| --- | --- | --- |
| Pinned NVRTC 12.8.93 | PASS: 59 entry points on compute_70/80/90/120 | Production CUDA source and exact entry-point parameter ABI; includes double unary scale/bias and fill value |
| CUDA host unit tests | PASS: 155 | Includes f64 byte limits and cuBLAS storage/compute-mode validation; no GPU execution |
| CUDA integration tests without required flag | 12 unavailable skips | These test-harness successes do not qualify native CUDA |
| Required CUDA f64 integration | FAIL: 2 tests, device unavailable | Shared binary64 conformance and owner/write/scalar ABI fixtures did not run on a GPU |
| Required f64 geometry | FAIL: 1 test, device unavailable | Geometry fixture compiled, numerical assertions not executed |
| Shared tensor contracts | PASS | Existing shape/layout contracts |
| Required WGSL/MLX regressions | PASS on Apple GPU | Existing f32/u32/low tensor and resident geometry paths; not f64 support |
| WGSL unit tests | PASS: 14 | Required Metal device |
| Strict Clippy, all targets | PASS | tensor-core, compute-core, compute-cuda, compute-mlx, gpu-compute, osv-math with tensor-cuda |
| Strict rustdoc | PASS | Same packages |
| WASM check | PASS | compute-core compile only, no browser execution |
| Architecture check | PASS | Repository GPU dependency policy |

Cargo also emits the existing `raster-core` manifest warning about the
`wgsl_export` binary name. The new Rust code passes `-D warnings`.

The [NVRTC report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-float64/report.json)
contains compiler options, pinned wheel/image identifiers, source and PTX hashes,
and all parameter types. Raw generated PTX remains local and gitignored; the
report and compilation logs are tracked. Reproduce it with:

```sh
bash crates/compute-cuda/qualification/qualify-nvrtc.sh crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-float64 70 80 90 120
COMPUTE_REQUIRE_CUDA=1 cargo test --locked --manifest-path crates/Cargo.toml -p compute-cuda --test float64 -- --nocapture
COMPUTE_REQUIRE_CUDA=1 cargo test --locked --manifest-path crates/Cargo.toml -p osv-math --features tensor-cuda --test tensor_f64 -- --nocapture
```

`qualify-tensor-backends.py --backend cuda` includes the new tensor and geometry
fixtures. Tests cover coordinates differing below f32 resolution, `1e200` and
`1e-200`, bit-preserving strided views/subnormals, all unary/binary operations,
multi-pass reductions, empty identities, signed zeros, f64 GEMM accumulation,
nonzero matrix offsets, vector/batch promotion, owner checks and write aliases.

## Remaining scope

NVIDIA execution and performance are still required. F64 indexing, scan,
scatter, stable statistics, attention, convolution and prepared/CUDA Graph
execution remain open. WGSL/MLX need explicitly identified software binary64
arithmetic. Existing f32 domain adapters and application callers are not
implicitly migrated by adding `TensorMathF64`. These are outstanding parts of
the full shader/CUDA/tensor/MLX objective.

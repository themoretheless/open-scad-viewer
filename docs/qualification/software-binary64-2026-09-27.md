# Software binary64 and native geometry migration — 2026-09-27

## Implemented

`ComputeRuntime` now implements `TensorF64Backend` on native hosts with integer
WGSL shaders. `GpuF64Tensor` stores low/high u32 words without numerical
conversion. Software arithmetic has the binary64 exponent range and 53-bit
significand; it does not use double-single or any f32 arithmetic. Execution
uses the selected GPU and has no CPU arithmetic fallback.

Basic add/subtract/multiply/divide/sqrt round to nearest, ties to even, with
gradual underflow, signed zeros, infinities and quieted NaNs. NaN payload
selection, exception flags and dynamic rounding modes are not promised.
The implementation has been tested against CPU binary64, not formally proved
or exhaustively tested over all possible inputs.

The tensor implementation supports views, broadcast, materialization, all
current unary/binary operations, axis reductions, mean and batched matmul.
`exp/log/sin/cos` use approximate binary64 polynomials. Sin/cos argument
reduction uses a generated 1280-bit 2/pi constant. These functions are not
guaranteed correctly rounded. Parallel sums and matmul can differ from other
binary64 implementations because of operation ordering and fusion.

## Existing callers migrated

On native hosts, these `MathGpuSession` methods now use the binary64 backend:

- `try_squared_distance_pairs`
- `try_squared_distance_pair_sum`
- `try_transformed_squared_distance_pair_sum`
- `try_point_bounds`
- `try_transformed_point_bounds`
- `try_point_moments`
- `try_point_cloud_stats`
- `try_point_cloud_stats_stable`

Their convenience wrappers and free GPU adapters use the same implementations.
They report `GpuArithmetic::SoftwareBinary64`. `session.tensor()` now returns
`TensorMathF64`; existing explicit `TensorMath` f32 recipes remain available.
Shared f64 moments/statistics recipes compose with native CUDA as well as the
software WGSL backend. Only final values are read back by session wrappers.
Bounds center/extent are derived from the final min/max on the host as before.
Numerical products and sums must fit f64; arbitrary-range robust statistics
are outside this contract.

## Evidence

Native execution reports **Metal**, on the available Apple GPU. Required-device
flags make the tests fail when an adapter is absent rather than count a skip.

| Check | Result |
| --- | --- |
| Binary64 shader validation and native arithmetic/tensor tests | 3 passed |
| Arithmetic corpus | 41,444 pairs per operation; special boundaries, random bit patterns and cancellation |
| Add/subtract/multiply/divide/sqrt | Exact CPU bits, except NaN payload comparison |
| Comparisons | Exact equality/order results |
| Exp/log | Relative tolerance 8e-15 plus four minimum subnormals |
| Sin/cos | Absolute tolerance 4e-15 |
| Full `osv-math --features gpu` | 108 passed |
| Required WGSL tensor regression suite | 74 passed, including binary64 tests |
| Shared tensor contracts | 47 passed |
| Native f64 geometry | Passed on Metal; CUDA execution not tested on this host |
| Strict Clippy: compute-core + math/gpu, all targets | Passed |
| Strict Clippy: math/cuda,tensor-cuda, all targets | Passed (compile check, not CUDA execution) |
| Rustdoc with warnings denied | Passed |
| compute-core + math/gpu wasm32 build | Passed (compile check, not browser execution) |
| GPU architecture checker | Passed |
| Generated constants | Regeneration byte-identical |

Precision witnesses retain differences of 0.25 at an origin of 2^40, support
1e100 coordinates, preserve zero transformed residuals and accept f64::MAX
for bounds. The formerly quantized `[16_777_216, 16_777_217]` cloud now has
covariance 0.25 instead of zero. Tests also cover strided views, empty
reductions, signed zero, range beyond f32 and binary64 matmul accumulation.

Raw logs: [arithmetic](software-binary64-2026-09-27/binary64.txt),
[math](software-binary64-2026-09-27/math.txt),
[native suite](software-binary64-2026-09-27/native/report.json),
[checks](software-binary64-2026-09-27/report.json).

Reproduce the native tensor suite:

```sh
python3 scripts/qualify-tensor-backends.py --offline --backend wgsl --output /tmp/binary64-qualification
COMPUTE_REQUIRE_GPU=1 cargo test --offline --locked --manifest-path crates/Cargo.toml -p osv-math --features gpu
```

## Remaining migration

This is not completion of the repository-wide f64 migration.

- Nearest-neighbor/Chamfer session methods and `MathGpuProgram` recorded plans
  still use their explicit legacy f32 arithmetic.
- Browser session methods retain their previous f32 path; native synchronous
  `TensorF64Backend` is not yet an async browser API.
- WGSL f64 indexing/scatter extension traits, prepared recording and neural
  operations are not implemented yet.
- Legacy CUDA geometry adapters still use f32; the separate CUDA tensor f64
  backend is available but requires NVIDIA hardware qualification.
- MLX does not yet implement software binary64.
- No performance advantage or cross-vendor GPU qualification is claimed.

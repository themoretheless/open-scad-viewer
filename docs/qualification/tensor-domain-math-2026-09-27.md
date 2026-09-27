# Resident geometry on shared tensor backends

Date: 2026-09-27. The new `math-core::tensor` API runs the same domain recipes
through WGSL, CUDA and MLX contracts. Native WGSL/Metal and MLX/Metal execution
passed on this Apple host. CUDA execution remains unverified.

## Implemented behavior

[TensorMath](../../crates/math-core/src/tensor/mod.rs) provides resident affine
transforms, paired squared distances, bounds, centroid, raw second moments,
centered covariance, nearest neighbors and directed Chamfer summaries. Inputs
and outputs are native tensors; uploads and reads are explicit. Geometry uses
f32 coordinates of shape `[N,3]`. The f64 upload helper checks conversion; it
cannot recover differences lost to rounding.

Algorithms live in math-core, contracts in tensor-core, and native execution in
each adapter. `MathGpuSession::tensor()` shares its existing device and queue.
Default CPU math stays independent of GPU dependencies, and the MLX feature does
not pull in WGSL or CUDA. Existing specialized kernels and placement thresholds
remain available. These shared recipes make no performance or fusion promise.

[TensorEvalBackend](../../crates/tensor-core/src/evaluation.rs) completes
selected f32/u32 results without reading values on the host. All input owners
and dtypes are validated first, including empty tensors and late arguments.
WGSL flushes uploads and waits for its exact submission; CUDA synchronizes its
stream; MLX evaluates selected arrays and synchronizes. The blocking WGSL
implementation is native-only.

Nearest neighbors use direct differences, exact u32 tie-breaking by original
target index, tiled searches and resident output assembly. Missing results use
`(u32::MAX, f32::MAX)`. Evaluation fences bound the retained recipe dependency
chains. In MLX 0.32.1, evaluated non-tracer arrays detach their graph edges:
[evaluation](https://github.com/ml-explore/mlx/blob/v0.32.1/mlx/transforms.cpp#L279-L289),
[detach implementation](https://github.com/ml-explore/mlx/blob/v0.32.1/mlx/array.cpp#L108-L150).
This source reasoning does not measure peak device memory; shared views,
allocator caches and separately retained programs may still own storage.

Neighbor limits check dimensions, overflow, output bytes and conservative
recipe-level workspace/work estimates before evaluation. They exclude caller
inputs and their lazy producers, backend primitive temporaries/materializations,
metadata, caches and library workspaces. View-only operations count zero logical
work. Repeated rank searches cost O(Q T k²), so this portable implementation is
not a replacement for measured specialized kernels.

## Numerical correction and coverage

Covariance centers and scales each coordinate before a normalized Gram product,
subtracts the residual mean outer product, symmetrizes and restores units. Raw
moments use a separate normalized product. Centroid adds the measured residual
mean to its initial center.

That final correction fixes a real failure: a 513-point cloud with one `1e20`
coordinate produced initial centroid relative errors around 3.77e-6 on WGSL
and 8.00e-6 on MLX. Historical failed logs and the explicitly CPU-only rounding
diagnostic are retained under
[diagnostics](tensor-domain-math-2026-09-27/diagnostics/centroid-rounding.txt).
Both final native runs pass the original 3e-6 sparse relative tolerance.

For mixed-sign clouds the centroid comparison now scales by the mean absolute
coordinate, because cancellation can make the signed mean arbitrarily small.
This is a change to the tolerance model, not an unconditional relative-error
guarantee. Covariance and cross-moment checks use their corresponding diagonal
scales. Finite f32 intermediates are required; underflow and reduction order
remain backend-dependent.

The independent f64 oracle covers shifted coordinates, sparse `1e20` values,
anisotropic scales, symmetry and approximate positive semidefiniteness. Domain
fixtures also cover strides, broadcast views, empty inputs, owner errors,
cross-tile ties, k greater than the target count, exact budgets and resident
gather composition. No host point/distance fold implements the GPU recipes.

## Verification

[checks.json](tensor-domain-math-2026-09-27/checks.json) records exact commands,
environment variables, exits and elapsed times. Native runs required their
hardware, with no optional-runtime skips. Counts below are test entries and
include CPU reference and validation tests; they are not counts of GPU kernels.

| Check | Result | Evidence |
| --- | --- | --- |
| Shared tensor contracts | 40 passed | [Contracts](tensor-domain-math-2026-09-27/native/contracts.txt) |
| WGSL tensor suite | 67 passed, Metal backend | [WGSL](tensor-domain-math-2026-09-27/native/wgsl.txt) |
| MLX suite | 140 passed, GPU runtime 0.32.1 | [MLX](tensor-domain-math-2026-09-27/native/mlx.txt) |
| New domain fixture | Each backend: one CPU oracle and one native fixture passed | [WGSL](tensor-domain-math-2026-09-27/native/math-wgsl.txt), [MLX](tensor-domain-math-2026-09-27/native/math-mlx.txt) |
| Existing math | CPU 68; tensor-only 70; GPU-feature 89 unit plus 15 integration entries passed | [CPU](tensor-domain-math-2026-09-27/math-cpu.txt), [Contracts](tensor-domain-math-2026-09-27/math-contracts.txt), [GPU](tensor-domain-math-2026-09-27/math-legacy-gpu.txt) |
| Resident example | WGSL and MLX produced centroid `[2,-1,3]`, covariance diagonal `[1,1,0]` and identical neighbor IDs | [WGSL](tensor-domain-math-2026-09-27/example-wgsl.txt), [MLX](tensor-domain-math-2026-09-27/example-mlx.txt) |
| CUDA host suite | 151 unit entries, two CPU reference entries and six compiled doctests passed; seven native entries reported unavailable/SKIP | [Host](tensor-domain-math-2026-09-27/cuda-host.txt) |
| Required CUDA evaluation and domain fixtures | Both exit 101: driver/device unavailable; zero CUDA execution passes | [Evaluation](tensor-domain-math-2026-09-27/cuda-evaluation-required.txt), [Domain](tensor-domain-math-2026-09-27/math-cuda-required.txt) |
| Strict Clippy and rustdoc | Passed, including all three math adapter features and CPU/tensor-only Clippy | [Clippy](tensor-domain-math-2026-09-27/clippy.txt), [Rustdoc](tensor-domain-math-2026-09-27/rustdoc.txt) |
| WASM library build | Passed; compile-only, no browser execution | [WASM](tensor-domain-math-2026-09-27/wasm-check.txt) |
| Architecture, formatting and whitespace | Passed | [Audit](tensor-domain-math-2026-09-27/artifact-audit.json) |

The pre-existing stable-statistics helper used a native blocking readback and
prevented the GPU-feature library from compiling for WASM. It and the new
session tensor accessor are now explicitly native-only. Async browser domain
execution remains work to do. Cargo still reports the existing unrelated
`wgsl_export` manifest naming warning.

## Evidence scope and remaining work

The [source manifest](tensor-domain-math-2026-09-27/source-fingerprints.json)
identifies checked sources. The [artifact audit](tensor-domain-math-2026-09-27/artifact-audit.json)
records hashes, entry counts and local links. No GPU shader source changed in
this phase. The [reuse audit](tensor-domain-math-2026-09-27/kernel-source-reuse.json)
verifies existing CUDA source parts and retained PTX hashes; it is not a new
NVRTC compilation or NVIDIA execution result.

Remaining work includes NVIDIA numerical/graph qualification, Compute Sanitizer
and actual Tensor Core instruction profiling; domain recording and performance
measurements; async browser execution; broader operators and hardware CI.
This phase does not establish complete support for all computation.

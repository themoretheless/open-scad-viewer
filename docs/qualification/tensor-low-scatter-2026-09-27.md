# Low-storage scatter qualification

Date: 2026-09-27. This extends [low indexing and scans](tensor-low-indexing-2026-09-27.md)
with f16/BF16 scatter on WGSL, CUDA and MLX. The full compute goal remains in progress.

## Contract and architecture

`TensorLowScatterBackend` adds `scatter_low` and `scatter_low_f32`. Both share the
existing scatter axis/index expansion, update broadcasting and invalid-count
rules. The base and updates must have matching dtypes. Inputs remain unchanged;
the result keeps the base shape. Invalid indices are ignored and counted once
per logical index, including when other axes make destination slices empty.

- Replace selects the greatest row-major logical index on duplicate targets.
  Its low result preserves every raw bit, including signaling NaNs and zero
  signs. F32 output widens finite values, zeros and infinities exactly; NaN
  classification is preserved, without a payload guarantee.
- Min/Max select finite values exactly, including subnormals. Min chooses -0
  and Max chooses +0 on zero ties.
- Add/Multiply accumulate the base and all valid updates in f32. Intermediate
  values must remain finite. Parallel order and f32 underflow follow backend
  limits. Low output rounds each completed destination once with nearest-even;
  final low overflow is allowed. F32 output retains the unrounded accumulator.

Kernels load low updates directly. No full f32 update tensor is created.
Result-shaped f32 accumulators and integer indexing metadata are allowed.
The shared crate owns shapes, contracts and independent conformance; each
backend owns its storage and execution details.

## WGSL

Two generated storage specializations reuse the existing scatter traversal and
shared host metadata/Replace owner preparation. Raw Replace/Min/Max first copy
the base, then update one halfword using u32 compare-exchange, preserving its
neighbor. Add/Multiply decode the base into a result-shaped f32 accumulator,
read packed updates inside the fold and cast once after all updates complete.
Every operation also supports a direct f32 destination.

Recorded `_into` forms support output/count offsets, changing inputs and replay.
Owners and counts reset on every execution. Runtime ownership, shapes, dtype,
layout and data/count aliases are validated before a prepared program is
appended. Cross-type low/f32/u32 aliases are rejected.

[Contracts](../../crates/compute-core/benchmarks/tensor-low-scatter-contracts.md),
[five new and four existing scatter tests](../../crates/compute-core/benchmarks/tensor-low-scatter-metal-tests.txt).

## CUDA

The shared scatter traversal accepts a writer policy. Two native u16 entrypoints
implement raw halfword CAS and f32 accumulation. A raw result's physical
allocation is padded to an even u16 count, ensuring that CAS on an odd logical
tail stays inside a complete 32-bit word. Shape and readback remain logical.
Owner election, invalid counts and index traversal are shared with f32/u32.

NVRTC 12.8.93 compiled **47 entrypoints for compute_70/80/90/120** with the same
precise-math options as the runtime. All ten source parts, concatenation order,
entry parameters and four retained PTX files match the frozen source. Combined
source: 43,196 bytes, SHA256
`2bfe38d408e0208dba2ddc484b60a5011b543b541724b9aa8b7b17afae9544da`.

[Compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-scatter/report.json),
[source/PTX audit](tensor-low-scatter-2026-09-27/cuda-source-audit.txt).

This host has no NVIDIA driver/device. The optional native test explicitly
skips; required CUDA mode fails with exit 101. The native fixture includes
the shared cases, 65,539 duplicate updates, odd padded results, raw payloads,
strided inputs, exact tiny values and ownership/dtype guards. Compilation does
not establish numerical execution, launch correctness, performance or Tensor
Core use.

[Host tests and explicit skip](../../crates/compute-cuda/qualification/low-scatter-host-tests.txt),
[required CUDA failure](../../crates/compute-cuda/qualification/low-scatter-cuda-required.txt).

## MLX

Replace reuses deterministic owner selection and the verified native take/raw
selection path. For the other modes, a custom Metal producer builds linked
lists in a fresh zero-initialized atomic u32 allocation. It contains one head
per destination-axis coordinate and one next pointer per logical index. Zero
terminates a list; valid indices contribute unique one-based tokens. Invalid
indices do not enter a list. The dependent consumer starts after construction
finishes, so an early published head cannot expose an unfinished next pointer.

Each output element loads its low base, follows only its destination's list,
and loads low updates directly using logical shapes/strides. Extrema use raw
integer ordering; arithmetic folds in f32 and rounds once for low output.
This visits O(base elements + logical indices + valid expanded updates), with
an integer list allocation of `4 * (axis extent + index count)` bytes plus at
most 1,020 padding bytes. Native planner scratch and readback are separate.
A highly contended destination is traversed sequentially by its output thread;
no MLX performance claim is made. Replace retains the native planner's
`index_count <= i32::MAX` limit; list folds use one-based u32 index tokens.

The custom Metal cache includes the atomic-output mode in its key. The optional
C ABI group now includes zero initialization. A native MLX 0.32.1 probe built
lists for 65,539 indices and 17 destinations: all 59,580 valid nodes were visited
exactly once, with zero errors. It also checked lazy evaluation after freeing
temporary configuration/kernel wrappers. New Rust tests and the full MLX suite
passed on the first run.

[Native probe source](../../crates/compute-mlx/qualification/low-scatter-native-probe.c),
[native probe output](../../crates/compute-mlx/qualification/low-scatter-native-probe.txt),
[full MLX output](../../crates/compute-mlx/qualification/low-scatter-metal.txt).

## Verification and measurements

The common fixture routes all 65,536 raw encodings through strided Replace and
checks every finite encoding against its opposite sign for Min/Max. It covers
signed-zero ties, tiny extrema, 65,539 duplicate updates, deterministic repeated
Replace, all axes, scalar indices, broadcast updates, empty/all-invalid cases,
unchanged inputs and a resident scatter -> gather -> scan chain.

Arithmetic references use mathematical low decoding and independent rounding.
Exact dyadic fixtures remain representable for every permitted fold order.
Cancellation, low half-ULP additions, multiplication midpoints and final low
overflow distinguish f32 accumulation from repeated low rounding. Backend
fixtures add recording rollback, alias/ownership checks and storage sentinels.

| Check | Result |
| --- | --- |
| Required GPU regression: gpu-compute / compute-core / raster-core / osv-math | **274 passed**, 0 failed, 0 ignored; 42 suites |
| New / existing WGSL scatter | **5 / 4 passed** on Metal |
| Generated WGSL specializations | Both pass Naga validation |
| Required MLX suite | **56 passed**: 3 unit + 53 integration |
| Shared CPU contracts/references | **32 passed** |
| CUDA host / compile doctest | **13 / 1 passed**; native fixture explicitly skipped |
| NVRTC compiler/ABI | **47 kernels x 4 architectures** |
| Strict all-target Clippy / strict rustdoc, four tensor crates | Pass |
| wasm32: tensor-core, compute-core, compute-mlx | Pass |
| Architecture / changed Rust formatting / diff whitespace | Pass |
| NVIDIA execution and Tensor Core instruction profiling | Pending hardware |

[GPU regression](tensor-low-scatter-2026-09-27/gpu-regression.txt),
[contracts](tensor-low-scatter-2026-09-27/contracts.txt),
[Clippy](tensor-low-scatter-2026-09-27/clippy.txt),
[rustdoc](tensor-low-scatter-2026-09-27/rustdoc.txt),
[WASM](tensor-low-scatter-2026-09-27/wasm.txt),
[architecture](tensor-low-scatter-2026-09-27/architecture.txt),
[formatting](tensor-low-scatter-2026-09-27/rustfmt.txt),
[test summary](tensor-low-scatter-2026-09-27/test-summary.json),
[environment](tensor-low-scatter-2026-09-27/environment.json),
[source fingerprints](tensor-low-scatter-2026-09-27/source-fingerprints.json),
[MLX source/log manifest](../../crates/compute-mlx/qualification/low-scatter-source-manifest.json).
Existing workspace warnings concern `wgsl_export` naming and duplicate `bench`
example names. No optional CUDA skip is counted as a successful hardware test.

## Measured effect

The [matched WGSL benchmark](../../crates/compute-core/benchmarks/tensor-low-scatter.md)
compares resident low inputs and outputs against explicit f32 conversion,
scatter and one final low cast. It checks all outputs/counts on every run,
including warmup. Five geometries cover small, unique, spread, hot and strided
updates; both dtypes and all five operations produce 50 cases. Programs are
reused, with 200 ms warmup and 31 rotated samples. GPU shared-pass timestamps
and separate unprofiled host/readback timings are retained.

Final GPU median ratios (cast baseline / direct low):

| Operation | Range across measured cases |
| --- | ---: |
| Replace | 1.33-2.32x |
| Add | 1.02-1.64x |
| Multiply | 1.13-1.63x |
| Min | 1.35-2.61x |
| Max | 1.33-2.58x |

The first run exposed a 5-6% hot Max regression. Packed Min/Max now return when
an update cannot improve the observed extremum, avoiding a redundant CAS. The
observation is a valid linearization point because subsequent same-op extrema
are monotonic. Improving updates still retry CAS and preserve neighboring
halfwords. Independent review, focused tests and the repeated full 274-test
GPU regression passed on this final shader. Initial source, hashes and raw
timings are preserved alongside the final run.

All 50 final direct-path medians were lower than their matched baseline, but
small differences are not statistically established. Hot Add still takes
4.84-4.85 ms versus 4.96-4.98 ms: destination contention dominates. Hot Max is
about 0.026 ms versus 0.049 ms. No CUDA/MLX speedup or application-level gain
is established by this benchmark.

For N base/result elements and M stored updates, direct Add/Multiply remove
4N+4M bytes of visible f32 conversion buffers; raw Replace/Min/Max remove 8N+4M.
Shared low inputs/outputs, indexing metadata and readback remain. These are
logical allocation differences, not peak RSS. Raw samples, p90, exact shapes,
memory accounting and unmeasured cases are in the benchmark report.

## Remaining scope

Low-storage statistics and attention remain open. Native WGSL f16 arithmetic,
direct low-input/f32-output MLX matrix products, NVIDIA execution/Tensor Core
profiling and deployment/CI qualification also remain unverified. A highly
contended scatter Add still needs a different aggregation strategy for speed.
This phase establishes resident low scatter and its documented workload limits.

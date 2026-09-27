# Low-storage indexing and scans qualification

Date: 2026-09-27. This extends [low arithmetic](tensor-low-ops-2026-09-27.md)
with direct f16/BF16 comparison, selection, gather, stable compaction and prefix
sums across WGSL, CUDA and MLX. The full compute goal remains in progress.

## Common API

`TensorLowIndexBackend` extends the existing storage and typed-indexing traits:

- `compare_low`: all six IEEE comparisons, exact u32 zero/one output. Matching
  dtypes are required; shapes broadcast. Both zeros compare equal, infinities
  are ordered, and a NaN makes only NotEqual true. Subnormals compare exactly.
- `select_low`: every nonzero u32 mask selects the true operand. All three
  shapes broadcast jointly; low dtypes must match even for empty results.
- `gather_low`: inserts the index shape at the chosen axis; scalar indices
  remove that axis. Invalid indices produce positive zero and increment a
  resident u32 count once per logical index, including for empty output slices.
- `compact_low`: preserves logical row-major order, returns capacity
  `[input.numel()]`, writes a GPU count and zeros the entire unused tail.
- `scan_low_f32` / `scan_low`: prefix sums along any axis, with inclusive/
  exclusive and forward/reverse traversal. The complete prefix accumulates in
  f32; a low output rounds once with nearest-even. Input/intermediate sums must
  stay finite; final low overflow is permitted. Backend f32 order/underflow
  limits apply. Scalar scan axes are invalid; empty tensors retain empty shape.

Selection, gather and compaction copy every selected raw bit, including NaN
payloads and zero signs. These operations do not convert payloads through f32.
Scans decode low input directly. F32 prefix outputs and hierarchy partials are
allowed, without a full f32 input conversion. All intermediates stay on GPU.

## WGSL execution

Packed kernels reuse the existing logical Select/Gather traversal and common
host metadata builders. One invocation owns an output u32 word and preserves
halfwords outside odd-offset or odd-length output views. Comparison uses the
integer low codec and ordered IEEE keys with explicit NaN/zero handling.

Compaction reuses the u32 mask scan. Independent logical writers can share one
physical word, so a compare-exchange loop updates just the selected halfword.
Selected positions and zero-tail positions are disjoint; replay replaces both
and resets the count. The first scan pass receives a packed-loader hook; f32
totals, recursive scans and carries reuse the existing hierarchy. Non-last
axes may need an axis-last f32 result followed by an output layout copy.

Every operation has a recorded `_into` form. Ownership, output shape/layout,
dtype, data/count aliases and low/f32/u32 reinterpretation aliases are checked.
Multi-stage recording appends a prepared program only after successful setup.

[Contracts](../../crates/compute-core/benchmarks/tensor-low-index-contracts.md),
[five new and five existing indexing tests](../../crates/compute-core/benchmarks/tensor-low-index-metal-tests.txt).

## CUDA execution and compilation

Native u16 Select/Gather/Compact entrypoints specialize the existing templated
traversal. A raw comparison kernel implements IEEE ordering without arithmetic
on NaNs/subnormals. The generic scan first-load policy decodes u16 into f32;
existing f32 totals, recursive scan and carry kernels complete the result.
Low output uses one final cast. Storage types do not add u16 to the arithmetic
scalar abstraction.

NVRTC 12.8.93 compiled **45 entrypoints for compute_70/80/90/120**. The precise
math options match runtime compilation. All nine source parts, runtime concat,
emitted parameter widths and four retained PTX files were checked. Source:
39,613 bytes, SHA256
`b0f42e12a9e1649900ab1a9a80be5e3fa55e38f8e60b214097e7c6a274abeff8`.

[Compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-indexing/report.json),
[source/PTX audit](tensor-low-indexing-2026-09-27/cuda-source-audit.txt).

There is no NVIDIA device on this host. The optional native fixture explicitly
skips, while required CUDA mode fails with absent driver/device. Shared and
native low-index checks are wired for that hardware gate. Compilation does not
prove numerical execution, launch correctness, speed or Tensor Core use.

[Host output with skip](tensor-low-indexing-2026-09-27/cuda-host.txt),
[required CUDA failure](../../crates/compute-cuda/qualification/low-indexing-cuda-required.txt).

## MLX native behavior and corrections

An exhaustive C probe on MLX 0.32.1 checked all 65,536 patterns for each dtype.
Native `take` and unique-destination `put_along_axis` preserved every pattern.
Native BF16 `where` changed 507 payloads: subnormals became zero and some NaNs
were canonicalized. A custom raw selector now serves the existing dtype-generic
selection/gather/compaction planners, preserving their shape/count handling and
the verified native data movement.

Comparison uses raw IEEE classification. Scan moves the axis by a view, reads
low values into f32 chunk totals, scans those small totals, and reads low values
again to write final f32 prefixes with their carries. Restoring axis order is a
view. With R rows, K axis length and P=ceil(K/256), visible storage is 4*R*K bytes
for the f32 result and, if P>1, 8*R*P bytes for totals/carries. Native scratch and
readback allocations are separate; a final low output additionally casts the
f32 result.

The first short-scan test exposed a custom ABI detail: a scalar dummy carry is
generated as a value, but a syntactically indexed dead branch still needs a
pointer. Its shape is now `[1]`, retaining a four-byte allocation. The initial
compiler failure and corrected full run are preserved. Source review and runtime
checks both informed this implementation; source review alone missed that issue.

[C probe](../../crates/compute-mlx/qualification/low-index-native-probe.c),
[raw native behavior](../../crates/compute-mlx/qualification/low-index-native-probe.txt),
[initial short-scan failure](../../crates/compute-mlx/qualification/low-index-initial-failure.txt),
[corrected full suite](../../crates/compute-mlx/qualification/low-index-metal.txt),
[source manifest](../../crates/compute-mlx/qualification/low-index-source-manifest.json).

## Verification

The common fixture compares all payloads with themselves, opposite signs and
unrelated payloads, plus a broadcast cross-product of numerical edge cases.
The reference uses mathematical low decoding and host IEEE comparisons instead
of the device's integer ordering. Routing checks every raw pattern through
strided select/gather/compact, sparse 131,077-element compaction, broadcast masks,
invalid counts, scalar and empty cases, and unchanged inputs.

An independent f64 coordinate oracle checks all axes/modes of 3D scans, a
513-element interior axis, all four modes at length 131,077, cancellation that
would fail low accumulation, broadcast strides and final low rounding. A
resident compare -> compact -> scan -> gather -> select chain reads only final
results/counts. Backend tests add offsets, halfword sentinels, grid-stride rows,
foreign ownership, dynamic dtype errors and failed-recording integrity.

| Check | Result |
| --- | --- |
| Required gpu-compute / compute-core / raster-core / osv-math regression | **268 passed**, 0 failed, 0 ignored; 41 suites |
| Focused new / existing WGSL indexing | **5 / 5 passed** on Metal |
| Naga validation | 45 assembled shipped sources plus five new generated index specializations |
| Required MLX suite | **52 passed**: 2 unit + 50 integration |
| Shared CPU contracts/references | **29 passed** |
| CUDA host / compile doctest | **12 / 1 passed**; native fixture skipped |
| NVRTC compiler/ABI | **45 kernels x 4 architectures** |
| All-target strict Clippy / strict rustdoc, four tensor crates | Pass |
| wasm32 check: tensor-core, compute-core, compute-mlx | Pass |
| Architecture / changed Rust formatting / diff whitespace | Pass |
| NVIDIA execution and Tensor Core instruction profiling | Pending hardware |

[GPU regression](tensor-low-indexing-2026-09-27/gpu-regression.txt),
[contracts](tensor-low-indexing-2026-09-27/contracts.txt),
[Clippy](tensor-low-indexing-2026-09-27/clippy.txt),
[rustdoc](tensor-low-indexing-2026-09-27/rustdoc.txt),
[WASM](tensor-low-indexing-2026-09-27/wasm.txt),
[architecture](tensor-low-indexing-2026-09-27/architecture.txt),
[formatting](tensor-low-indexing-2026-09-27/rustfmt.txt),
[test summary](tensor-low-indexing-2026-09-27/test-summary.json),
[environment](tensor-low-indexing-2026-09-27/environment.json),
[source fingerprints](tensor-low-indexing-2026-09-27/source-fingerprints.json).
Existing workspace warnings concern `wgsl_export` naming and duplicate `bench`
example output names. No NVIDIA skip is counted as a successful hardware test.

## Measured effect and remaining scope

The [matched WGSL benchmark](../../crates/compute-core/benchmarks/tensor-low-index.md)
checks every result and count after timing. Across 24 finite-input cases,
direct packed execution improved GPU medians by 2.29-4.27x for Select,
1.51-2.87x for Gather, 1.46-1.72x for Compact and 1.11-1.29x for Scan.
The comparison includes conversion costs in the f32 baseline; no generic
end-to-end or CUDA/MLX speedup is established. Compare, other mask densities
and non-last scan axes were not benchmarked.

Low scatter, low statistics/attention, direct low-input/f32-output MLX matmul,
broader numerical primitives, CUDA/MLX replay and provisioned hardware CI remain
in the [full compute plan](../design/tensor-backends-2026-09-27.md).

# Low-precision arithmetic and reductions qualification

Date: 2026-09-27. This extends the [attention phase](tensor-attention-2026-09-27.md)
with resident f16/BF16 arithmetic and reductions across WGSL, CUDA and MLX.
The broader compute objective and NVIDIA runtime qualification remain open.

## Implemented behavior

`TensorLowOpsBackend` extends the low-storage contract with all nine unary
operations and six binary operations, plus sum, product, min, max and mean
over arbitrary axes. Binary inputs require matching dtypes and follow trailing
broadcasting. Views retain positive/zero strides and offsets.

Ordinary arithmetic evaluates in f32 and rounds once to the input low dtype,
using round-to-nearest, ties-to-even. Final low output may overflow even when
f32 evaluation remains finite. Reductions accumulate in f32; callers choose
an f32 result or a final low conversion. A true f32 result retains values that
would be lost by first rounding to low storage. Mean divides the accumulated
f32 sum by the contraction size once; the sum must remain finite.

Negate and Abs preserve exact finite bits. Min/Max select finite input values
exactly, including BF16 subnormals; Min chooses -0 and Max +0 for a zero tie.
The same rule applies through every reduction partial. Other operations follow
f32 tolerances and underflow limits. Function domains must be valid; NaN/Inf
inputs have no common arithmetic policy.

Empty axes preserve values, empty outputs stay empty, and empty contractions
use sum=0/product=1. Min, Max and mean reject an empty contraction with a
nonempty result. No intermediate computation or result readback occurs on CPU.

## Backend implementations

### WGSL

Packed elementwise kernels read two low values per u32. Each invocation owns
an entire output word, preserving untouched neighboring halfwords for odd
offsets and lengths. Ordinary operations decode on load; exact sign/extrema
operations use integer bits. Recorded `_into` methods validate ownership,
shape, dtype, contiguity and aliases before appending work.

The reduction specializes the existing generic traversal, using u32 carriers
for f32 bits. Sum/product perform f32 arithmetic; extrema compare ordered IEEE
keys without floating arithmetic. Large contractions use at most 256 parts
per output and 4096 f32 partials overall (16 KiB). The reduced f32 result is
the only conversion temporary for low-result reductions. Multi-stage programs
are prepared transactionally and can be reused after input writes.

[WGSL contract and focused tests](../../crates/compute-core/benchmarks/tensor-low-ops-contracts.md).

### CUDA

Four native kernels add low unary/binary operations and arbitrary/all-axis
reductions. An input-loader abstraction reuses the existing reduction traversal,
block fold and f32 hierarchy. The first pass reads native u16 values directly;
later passes consume f32 partials. Integer ordering in the common f32 extrema
combiner preserves subnormals and signed zero through the hierarchy. Existing
f32/u32 kernel argument layouts are unchanged.

NVRTC 12.8.93 compiled all **40 kernels** for compute_70, compute_80, compute_90
and compute_120 using the runtime's precise-math options. The eight-part source
is 37,023 bytes, SHA256
`aa61a982e27ab58ea06c35e1ded7a0c8c172ac8b8926dabace003f4d8c943f2c`.
All source parts, exact runtime concatenation and four retained PTX hashes
match the [compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-ops/report.json).
The new kernel ABIs match Rust launch arguments and emitted PTX parameter widths.

This Apple host has no NVIDIA device. The optional native fixture returns early;
its harness success is excluded from numerical qualification. Required mode
fails explicitly with absent driver/device. CUDA numerics, performance and
actual Tensor Core instructions remain unverified.

[Host run with explicit skip](../../crates/compute-cuda/qualification/low-ops-host-tests.txt),
[required hardware failure](../../crates/compute-cuda/qualification/low-ops-cuda-required.txt),
[source/PTX audit](tensor-low-ops-2026-09-27/cuda-source-audit.txt).

### MLX

An installed-runtime C probe found that MLX 0.32.1 native BF16 Negate/Abs and
extrema flush tiny values, while native low reductions return a rounded low
result. Custom Metal kernels now read native low storage directly, evaluate
ordinary arithmetic in f32 and preserve exact bit operations. Reductions use
f32 registers and partials without expanding the entire input.

For R output groups and K contracted values, P=min(ceil(K/4096),1024); when
P>1 the partial array is 4*R*P bytes. The optional final low cast uses only the
reduced result shape. Native shape/stride metadata addresses transposed and
broadcast inputs without a contiguous copy. Scalar inputs are promoted to
one-element views because MLX's custom ABI omits rank-zero metadata; their
logical output remains scalar.

The context caches complete source/header/input-name keys. Globally unique
kernel names avoid collisions in MLX's separate name-based cache. Optional
custom Metal symbols are checked before use. Independent review matched the
installed C headers and confirmed ownership of temporary vectors/configuration
and lazy array handles. MLX performance was not measured.

[Native probe source](../../crates/compute-mlx/qualification/low-ops-native-probe.c),
[probe output](../../crates/compute-mlx/qualification/low-ops-native-probe.txt),
[initial scalar failure](../../crates/compute-mlx/qualification/low-ops-initial-failure.txt),
[corrected complete run](../../crates/compute-mlx/qualification/low-ops-metal.txt),
[MLX source manifest](../../crates/compute-mlx/qualification/low-ops-source-manifest.json).

## Numerical and execution checks

The shared reference decodes low values mathematically and searches the
representable values to round by distance and parity. It does not reuse device
bit codecs. CPU tests verify every finite representable round trip, ties,
subnormal boundaries and overflow. Arithmetic expectations use f64 followed by
f32 evaluation; transcendental outputs allow one low ULP. Exact sign/extrema
checks compare raw bits, so absolute tolerances cannot hide flushed subnormals.

Shared fixtures exercise all finite low payloads for Negate/Abs and paired
Min/Max; all operations; transposed/broadcast inputs; every subset of three
reduction axes in both keep-dimension modes; exact f32 accumulation; final low
rounding; scalar/empty/error contracts; and 131,077-element partial reductions.
WGSL additionally checks odd halfword offsets, sentinels, cross-type aliases,
failed-recording integrity and changed-input program reuse. MLX covers lazy
lifetimes and more than 65,535 reduction rows. CUDA shares the fixture and adds
native checks behind its required-device gate.

| Check | Result |
| --- | --- |
| Required GPU regression: gpu-compute, compute-core, raster-core, osv-math | **262 passed**, 0 failed, 0 ignored; 40 suites |
| Focused WGSL low operations | **5 passed** on Metal |
| WGSL validation | 45 static assembled sources plus generated low reducer |
| Required MLX suite | **48 passed** on Metal, MLX-C 0.6.0 / MLX 0.32.1 |
| Shared CPU contracts/references | **27 passed** |
| CUDA host tests / compile doctest | **12 / 1 passed**; native fixture skipped |
| NVRTC source/ABI | **40 kernels x 4 architectures** |
| Strict Clippy, all targets of four tensor crates | Pass |
| Strict rustdoc, four tensor crates | Pass |
| wasm32 compilation: tensor-core, compute-core, compute-mlx | Pass |
| Dependency architecture / changed Rust formatting / diff whitespace | Pass |
| NVIDIA numerical execution and Tensor Core profiling | Pending hardware |

[GPU regression](tensor-low-ops-2026-09-27/gpu-regression.txt),
[focused WGSL](../../crates/compute-core/benchmarks/tensor-low-ops-metal-tests.txt),
[contracts](tensor-low-ops-2026-09-27/contracts.txt),
[Clippy](tensor-low-ops-2026-09-27/clippy.txt),
[rustdoc](tensor-low-ops-2026-09-27/rustdoc.txt),
[WASM](tensor-low-ops-2026-09-27/wasm.txt),
[architecture](tensor-low-ops-2026-09-27/architecture.txt),
[formatting](tensor-low-ops-2026-09-27/rustfmt.txt),
[environment](tensor-low-ops-2026-09-27/environment.json),
[source fingerprints](tensor-low-ops-2026-09-27/source-fingerprints.json).
Existing manifest warnings concern `wgsl_export` naming and duplicate `bench`
example names in combined workspace commands.

## Measured effect

On Apple M4 Max, direct packed square was **1.61-3.02x faster** and direct low
sum **1.07-2.28x faster** in 16 matched GPU cases compared with explicit f32
expansion. Every timed execution passed exact reference checks. Large square
host timings improve less because readback dominates. At 1,048,581 values,
direct square removes 8,388,648 bytes of f32 temporary arrays; direct sum removes
4,194,324 bytes of expanded input but still needs bounded partial storage.
These byte counts exclude other allocations and are not a peak-memory trace.

[Benchmark methodology, full table and limitations](../../crates/compute-core/benchmarks/tensor-low-ops.md).
There is no speed claim for the other operations or for CUDA/MLX.

## Remaining scope

Low indexing, statistics and attention; broader numerical primitives; CUDA/MLX
reusable execution; native WGSL half arithmetic; direct low-input/f32-output MLX
matmul; NVIDIA hardware qualification and provisioned hardware CI remain in the
[full compute plan](../design/tensor-backends-2026-09-27.md).

# Tensor computation across WGSL, CUDA and MLX

Date: 2026-09-27. This document tracks the implementation toward the requested
full compute library. “Tensor” includes both multidimensional arrays and NVIDIA
Tensor Core acceleration. The common APIs cover resident f32 arithmetic and
typed f32/u32 indexing, scatter and reductions.
`TensorLowBackend` adds explicit f16/bf16 storage and matrix products.
`TensorLowOpsBackend` adds f16/bf16 arithmetic and direct low-input reductions
with f32 accumulation and either f32 or low output.
`TensorLowIndexBackend` adds exact low comparisons, raw selection/gather/
compaction and prefix sums with f32 accumulation.
`TensorLowScatterBackend` adds direct low-update scatter with low/f32 results.
`TensorStatsBackend` adds stable distributions, moments and layer normalization.
`TensorLowStatsBackend` provides those operations from f16/BF16 inputs with
f32 results or one final low rounding.
`TensorAttentionBackend` adds masked scaled dot-product attention with grouped
query heads and explicit causal alignment. `TensorLowAttentionBackend` accepts
direct f16/BF16 Q/K/V with f32 output or one final low rounding.
The remaining coverage below is part of the same objective.

## Structure

| Crate | Responsibility |
| --- | --- |
| `tensor-core` | Shapes, positive/zero strides, offsets, broadcasting, axis validation, canonical operations, precision policy and backend contract |
| `compute-core` | WGSL tensor kernels and reusable `ComputeProgram` recording, alongside existing array/fusion/scan/compaction APIs |
| `compute-cuda` | CUDA resident storage and kernels, cuBLAS matmul, explicit reduced-precision eligibility |
| `compute-mlx` | MLX-C loading, GPU streams and lazy resident arrays, native ownership and error handling |
| `gpu-compute` | Platform device/queue, capabilities, transport, profiling and optional CUDA driver access |
| `math-core` | Domain algorithms, f64 references, numerical tolerances and placement policy |
| `raster-core` | Rendering and its buffer contracts |

`tensor-core` has no dependencies. The three executors depend on it; it does not
know their allocation types. WGSL and CUDA share platform infrastructure where
appropriate. MLX loads its own native runtime. Raster remains independent.
`scripts/check-gpu-architecture.py` enforces these dependency directions and the
CPU-only math/tensor contracts.

The existing `compute_core::{UnaryOp,BinaryOp,CompareOp}` paths re-export the
canonical definitions from `tensor-core`. Their names and shader discriminants
remain unchanged. Backend execution code stays separate because native APIs,
resource lifetimes and scheduling differ.

## Common contract

`TensorBackend` supplies upload/read, materialize, reshape, permute, broadcast,
unary/binary operations, arbitrary-axis sum and batched matmul. Operations return
resident tensors. Readback is explicit; intermediate values do not travel through
the CPU. Unsupported operations or unavailable runtimes return errors.

- A rank-zero shape is one scalar. Any zero-length axis makes a tensor empty.
- Shape products, strides, storage bounds, axes and permutations are checked.
- Broadcasting aligns trailing axes; a dimension of 1 can expand to 0.
- Layouts support arbitrary rank and nonnegative element strides. Zero strides
  describe broadcasts. Views retain storage offsets and validate their bounds.
- Common reshape preserves logical row-major order, materializing a strided view
  on the device when necessary. Metadata-only `Layout::reshape` requires
  contiguous input.
- Empty reduction axes mean identity. Duplicate axes are rejected. Sum and
  product of an empty contraction write zero and one respectively. Min, max
  and mean reject empty contractions that would produce nonempty outputs.
  An empty output is valid for every reduction.
- Matmul supports vectors, matrices and leading batch dimensions. A left vector
  is promoted to `[1,K]`, a right vector to `[K,1]`; the inserted dimensions are
  removed from the result. Two vectors produce a scalar. Batch axes broadcast;
  contraction axes must match. A zero inner dimension writes zeros.
- Tensors belong to one runtime instance. Independent runtime instances reject
  each other's tensors even on the same physical GPU.
- Floating-point reduction order differs by backend. Conformance uses finite
  inputs and operation-specific reference comparisons, without promising bitwise
  equality or uniform NaN payloads/transcendental rounding across devices.

The common interface is synchronous at host readback and its WGSL implementation
is compiled for native targets. Existing WGSL tickets and recorded programs remain available to
callers that need deferred reads or a single submission. MLX records a lazy graph;
CUDA queues operations on a retained stream. Runtime capability discovery and
placement are separate from mathematical shape validation.

### Typed masks and indexing

`TensorIndexBackend` extends the arithmetic contract with u32 storage/views,
f32/u32 comparisons, selection, axis scans, device-index gather and stable
compaction. Comparisons return exact zero/one masks. Selection treats any nonzero
u32 value as true and broadcasts all three operands. Integer values remain u32;
values above 2^24 do not pass through f32 conversions.

Scans support inclusive/exclusive and forward/reverse traversal on every checked
axis. Results keep the original shape and coordinate order. Integer accumulation
wraps modulo 2^32; floating accumulation uses each backend's parallel order.

Gather replaces the selected input axis with the entire index tensor shape.
Scalar indices remove that axis. Each invalid index produces zero values and
increments a scalar u32 `invalid_count` on the GPU. The count measures index
elements once, including when the output is empty because another input axis is
empty. It does not count each replicated output value. Kernels check an index
before accessing source storage; MLX masks unsafe indices before native `take`.

Compaction accepts a mask that broadcasts to the input shape. It preserves
logical row-major order, returns capacity `[input.numel()]`, writes the selected
count to a GPU scalar, and zeroes the unused tail. It composes without reading a
dynamic length back to allocate output. The u32 count limits gather index count
and compaction input count to u32::MAX; each backend also enforces its own storage
and dimension limits.

The shared `check_index_backend` fixture exercises unsigned extremes, strided
views, all scan modes, recursive block carries, empty dimensions, invalid device
indices, broadcast masks and a compare → compact → scan → gather → select chain.
Typed methods also validate dtype on runtimes such as MLX that use one dynamic
array handle type for both associated tensor types.

### Low-storage indexing

The low-storage counterpart, `TensorLowIndexBackend`, returns the same u32
masks/counts while preserving every selected raw low payload. Comparisons
define IEEE behavior for all 65,536 patterns, including infinities, NaNs and
signed zeros. Select/Gather/Compact never require a float conversion. Low
prefix sums decode directly into f32 accumulation, returning f32 or one final
low rounding. The generic f32/u32 and packed paths share traversal and scan
hierarchy where their native storage permits it.

### Typed reductions

`TensorReduceBackend` extends the indexing contract with `reduce_f32`,
`reduce_u32` and `mean_axes`. `ReduceOp` selects sum, product, min or max over
any unique set of axes, with optional dimension retention. The existing
`TensorBackend::sum_axes` uses the same implementation. Integer sums and
products wrap modulo 2^32. Mean accepts f32 input and divides by the logical
contraction length, including broadcast dimensions.

The portable numerical domain is finite f32 input with arithmetic that stays
within range. Parallel reduction order can change rounding; no cross-backend
NaN or infinity behavior is promised. Min and max use internal padding
identities only after shared validation has ruled out an undefined empty
contraction. Unsupported integer mean is absent from the typed interface.

`MatmulPlan` supplies promoted operand shapes, matrix output and logical output
to executors. Shape validation stays independent of cuBLAS, MLX and WGSL, while
each backend keeps its storage and scheduling rules.

### Scatter with duplicate indices

`TensorScatterBackend` accepts a base tensor, resident u32 indices, updates and
one destination axis. Updates broadcast to the shape produced by gathering the
same indices along that axis. The result retains the base shape; source tensors
remain unchanged. Invalid indices are ignored and counted once per logical
index, including when other dimensions make the result empty.

`ScatterOp::Replace` selects the last row-major logical index when indices
repeat. The result is deterministic even for strided index and update views.
Add, multiply, min and max combine the base value with every valid update.
Integer addition and multiplication wrap modulo 2^32. Floating operations allow
parallel accumulation order and require finite input and intermediate values.

WGSL and CUDA elect the last Replace owner for each destination-axis index,
then only winning updates write. Reduction modes use native unsigned atomics
where available and compare-and-swap for the remaining operations. CUDA f32
addition also uses compare-and-swap to keep the custom kernel's denormal policy.
MLX uses native scatter reductions; Replace elects owners and gathers the
winning slices. Work scales with the base, indices and expanded updates.
These implementations do not scan every index separately for every output.

### Low-storage scatter

`TensorLowScatterBackend` shares the typed scatter shape, broadcast, invalid-index
and deterministic Replace contracts. Base and updates must have the same dtype.
Replace low output preserves all raw payloads; its f32 output widens selected
values directly and preserves NaN classification. Min/Max select finite values
exactly, including BF16 subnormals and -0/+0 ties. Add/Multiply accumulate in f32,
then round each completed destination once for low output. F32 output retains
that accumulator without passing through low storage.

WGSL specializes the shared scatter traversal with direct packed update loads.
Raw Replace/Min/Max use halfword compare-exchange; Add/Multiply use a result-shaped
f32 accumulator and a final cast. No full expanded f32 update tensor is created.
Output halfword neighbors, count resets and deterministic owners are preserved
on recorded replay. See the [WGSL contracts and focused evidence](../../crates/compute-core/benchmarks/tensor-low-scatter-contracts.md).

CUDA reuses the shared traversal with native u16 writer policies. Raw results
pad an odd logical tail to a complete 32-bit CAS word. MLX reuses owner/take
selection for Replace; other modes build GPU head/next lists and let each
output traverse its valid updates directly. That avoids scanning every index
for every output or expanding low updates to f32. Contended MLX output folds
are sequential; WGSL/CUDA folds contend on destination atomics. See the
[qualification and measured limits](../qualification/tensor-low-scatter-2026-09-27.md).

### Low-precision storage and products

`TensorLowBackend` provides a distinct low tensor type with `LowDtype::F16` or
`LowDtype::Bf16`. Upload and readback accept raw u16 words. Views and
materialization preserve all bits, including signed zero and NaN payloads.
GPU casts round to nearest with ties to even, retain representable subnormals,
and overflow to signed infinity. Casts preserve NaN classification; their
payload and sign are unspecified. This storage API is separate from the
`MatmulPrecision` policy for f32 allocations.

The storage representation is explicit. CUDA and MLX allocate native two-byte
elements. The portable WGSL path stores two elements per u32 and reports
`Packed16x2`; an odd allocation has one padding lane. This path works without
enabling `SHADER_F16`. It does not claim native f16 arithmetic or Tensor Core
execution.

Both products require matching input dtypes and follow `MatmulPlan` rules:

- `matmul_low` accumulates in f32 and rounds the completed result once into the
  input dtype. CUDA/WGSL may use an f32 output temporary before the final cast.
- `matmul_low_f32` retains the f32 result. CUDA passes native low input buffers
  to cuBLAS. WGSL decodes packed elements while loading matrix tiles. MLX uses
  custom Metal kernels over native low arrays and their view strides. These
  paths do not expand whole operand tensors to f32.
- Native MLX matmul still supplies the low-output operation. Its direct f32
  capability requires the dtype accessor and the custom Metal ABI. Missing
  support returns an explicit error. The custom route preserves normal BF16
  products formed from tiny inputs and large partners; this stronger numerical
  property is qualified privately for MLX. See the
  [implementation and measurements](../qualification/tensor-low-matmul-2026-09-27.md).

`low_precision_support(dtype)` reports storage and the two product capabilities
for the current backend. CUDA also checks cuBLAS availability and hardware
support for the chosen format. Support means the execution path is available;
instruction profiling is still needed to prove Tensor Core use.

The shared low fixture checks every 16-bit pattern through storage and strided
materialization, every representable finite value through conversion, and every
rounding midpoint plus its two neighboring f32 values for both signs. Matrix
cases cover partial tiles, broadcast batches, vectors, strided inputs, zero
contraction strides, zero contractions and empty outputs. Independent f64
references check exact dyadic products and rounded sums of normal inputs.
Cancellation and midpoint cases distinguish f32 accumulation and output from
premature low rounding.

### Distributions, moments and layer normalization

`TensorStatsBackend` adds arbitrary-axis `softmax`, `log_softmax`, `logsumexp`,
`moments` and `layer_norm` for f32 tensors. `Moments` contains mean and population
variance (division by N). Logsumexp and moments support `keep_dims`; the other
operations preserve input shape. Affine gamma/beta transforms compose with the
existing resident binary operations.

Softmax subtracts the group maximum before exponentiation. Log-softmax retains
that shift through its final subtraction: subtracting an already rounded
`logsumexp(x)` directly from a large `x` can erase the log normalizer. Moments
avoid `E[x²] - E[x]²`, which loses small variance at large common offsets.

WGSL and MLX form an anchor from `min/2 + max/2`, then scale deviations into a
bounded range. They reduce centered values in those scaled coordinates. Layer
normalization combines the scale with `sqrt(epsilon)` through ratios bounded by
one, avoiding an overflowing original variance or an underflowed epsilon
parameter. CUDA instead accumulates anchored deviations and centered squares in
f64 on the device. This wider internal precision does not expose general f64
tensor storage or imply a performance benefit on devices with slow f64 units.

Inputs are finite f32 values. Out-of-range reported variances and log
probabilities may become positive or negative infinity. Layer normalization
remains defined when the original variance exceeds f32 range. Epsilon must be
finite and strictly positive, including positive subnormal parameters. Constant
groups normalize to zero. Floating reduction order and underflow follow backend
limits; numerical equality uses tolerances.

An empty axis list defines singleton groups: softmax one; log-softmax, variance
and layer norm zero; logsumexp and mean preserve input. Elementwise outputs
preserve empty shapes. Logsumexp/moments reject an empty contraction with a
nonempty output using `EmptyReduction`, while an empty output is valid.

The shared fixture uses an independent f64 reference with an input anchor.
It checks large offsets, constant 1e30 rows, opposite finite f32 extrema, tiny
epsilon, unsorted axes, transposes, broadcasts, singleton/empty cases and a
131,077-element contraction. Probability checks remain relative for small
values, so a long row of erroneous zeros cannot pass an absolute-only tolerance.

### Statistics from low storage

`TensorLowStatsBackend` preserves the f32 shape, axis, epsilon and empty rules.
Its `_low_f32` methods return the evaluated result before storage rounding;
the `_low` variants round once into the input dtype. Singleton mean/logsumexp
preserve every finite input value, including subnormals and signed zero.

All backends load low inputs directly. Actual f32 outputs and row/partial
statistics are permitted; full f32 input, exponentiated or centered-input
temporaries are avoided. WGSL specializes the existing statistics templates
and shared recorded planner. CUDA specializes the original anchored f64 passes
with a native u16 loader. MLX reuses the low reducer's row geometry, partial
hierarchy and tree, transforming values in registers.

WGSL and MLX use integer extrema and exact power-of-two coordinate scaling for
tiny groups. This protects BF16 subnormals before floating arithmetic and keeps
normal layer-norm results from being replaced by zero. Reported statistics
and final low conversion retain their documented underflow/overflow limits.
See [qualification and workload limits](../qualification/tensor-low-statistics-2026-09-27.md).

### Scaled dot-product attention

`TensorAttentionBackend::attention` computes `softmax(scale * Q * Kᵀ + bias) * V`
with resident inputs and masks. Shapes are `[..., heads, sequence, depth]`, or
rank-two matrices with one implicit head. Leading batch axes broadcast. Query
heads are divided into consecutive groups sharing each key/value head; Q heads
must be a positive multiple of the matching K/V head count. The output has the
broadcast batches, Q heads, Q sequence length and V depth. All-rank-two inputs
return a rank-two output.

`AttentionMask::Keep` accepts exact u32 masks, with every nonzero value true.
`Additive` accepts finite f32 biases and negative infinity for excluded keys.
Masks broadcast to the logical score shape. `AttentionOptions::causal` is an
optional signed offset: key `j` is allowed when `j <= query_i + offset`.
Zero aligns the upper-left triangle; `Lk-Lq` aligns the lower-right triangle
for decoding. Causal and explicit masks combine. Fully excluded rows and empty
key sequences return zeros; empty query/batch/value dimensions remain empty.
Q/K depth and head counts must stay positive, even for empty outputs.

Scale defaults to `1/sqrt(D)`; finite zero and negative overrides are valid.
Inputs and allowed dot/scaled/biased logits stay within finite f32 range.
Finite normalized weighted outputs, including a constant `f32::MAX` value,
must not overflow an internal unnormalized numerator. Backend rounding and
underflow limits still apply. This forward operation has no dropout or
automatic differentiation contract.

WGSL streams 32-key tiles in one workgroup per query and 64-channel value tile.
Online maxima and denominators eliminate score/probability buffers; wide V
recomputes QK for each value tile. Few-query long-key cases split across up to
64 workgroups per row and merge normalized partials, with a bounded workspace.
CUDA computes each tile's QK once per query
and keeps an f64 output-sized accumulator. Its wider arithmetic is a numerical
choice with unmeasured hardware cost, not a Tensor Core implementation. MLX
uses its native fast attention and device graphs for unsupported cases, with
adapter guards for fully masked rows and explicit causal alignment. Native MLX
kernel selection is not a cross-backend performance guarantee.

### Attention from low storage

`TensorLowAttentionBackend` shares the attention geometry and masking rules.
Q/K/V must share a dtype, even for empty outputs; additive masks remain f32.
`attention_low_f32` returns the evaluated f32 result, while `attention_low`
rounds it once to Q's dtype. No backend expands complete low operands or
materializes complete score/probability matrices.

WGSL specializes packed loads in the existing streaming/split-key pipeline,
including the shared f32 partial merge. F16 additionally caches queries up to
depth 256 in 1 KiB of workgroup memory; BF16 keeps direct loads. CUDA templates its existing
u16/f32 loader and f64 online body, preserving the original f32 launch ABI.
MLX uses a custom streaming Metal kernel because widening a native low result
would lose the required f32 output accuracy. Its workgroup retains 32 scores
and weights, plus per-channel registers, without global attention scratch.
WGSL/MLX repeat QK for each 64-channel value tile; that cost remains visible.

Integer exponent transforms protect normal products formed from BF16
subnormal operands and large finite partners. Ordinary f32 underflow limits
still apply to true subnormal intermediates/results. Final low rounding,
GQA, masks, offsets and resident composition use the common independent
fixture. See [qualification and matched measurements](../qualification/tensor-low-attention-2026-09-27.md).

## Coverage and proof

| Capability | WGSL | CUDA | MLX |
| --- | --- | --- | --- |
| Shared f32 tensor contract | Actual Metal execution | Implementation plus host checks; NVIDIA execution pending | Actual Metal execution |
| Strided/permuted/broadcast views | Checked layouts and device materialization | Checked layouts and device materialization | Native views and contiguous readback |
| All 9 unary / 6 binary operations | Shared conformance passes | Same conformance fixture wired for NVIDIA | Shared conformance passes |
| Typed sum/product/min/max, f32 mean | GPU execution verified | Device kernels; hardware validation pending | GPU execution verified |
| Vector, batched and broadcast matmul | Portable tiled WGSL | cuBLAS with row-major adaptation | Native MLX matmul |
| Reduced matmul precision | Explicit unsupported error | Opt-in TF32, FP16 or BF16 compute modes | Explicit unsupported error |
| Typed compare / select / scan / gather / compaction | Recorded WGSL tensor operations | Device kernels; hardware validation pending | Lazy native GPU graph |
| Typed scatter, including deterministic last-index Replace | Actual Metal execution, reusable programs | Device kernels; hardware validation pending | Actual Metal execution, native scatter/gather graph |
| f16/bf16 storage, views and exact casts | Packed16x2; Metal exhaustive conversion checks pass | Native16; NVRTC-qualified kernels, numerical hardware checks pending | Native16; Metal exhaustive conversion checks pass |
| Direct low-input matmul, f32 accumulation | Low or f32 result; Metal execution | Low or f32 result; native cuBLAS inputs, hardware execution pending | Native low result; custom Metal f32 result, native execution |
| Low unary/binary operations and f32-accumulating reductions | Direct packed loads, recorded programs; Metal execution | Native u16 loads; NVRTC compilation, hardware execution pending | Custom Metal kernels with direct low loads; Metal qualification |
| Low compare/select/gather/compact and f32-accumulating scans | Packed loads, exact payloads, shared traversal/hierarchy; Metal execution | Native u16 loads, templated traversal; hardware execution pending | Custom compare/select/scan, native take/unique put; Metal execution |
| Low scatter, low/f32 output and final rounding | Packed updates, shared traversal, atomic halfword/f32 outputs; Metal execution | Direct u16 loads, templated traversal; NVIDIA execution pending | Custom Metal updates and resident graph; Metal execution |
| Softmax, log-softmax, logsumexp, moments, layer norm | Recorded WGSL; Metal conformance and replay checks | Device kernels; NVRTC compilation, hardware execution pending | Native GPU graph; Metal conformance |
| Low statistics, f32/low outputs | Direct packed loads; Metal execution | Native u16 loads; numerical hardware qualification pending | Direct low Metal kernels; Metal execution |
| Low attention, f32/low outputs | Streaming/split-key packed loads; Metal execution | Native u16 shared online body; numerical hardware qualification pending | Custom streaming Metal kernel; Metal execution |
| Masked attention, grouped query heads, signed causal alignment | Streaming WGSL tiles; no score buffers | Online f64 device kernel; numerical hardware qualification pending | Native fast attention and resident GPU graph |
| Recorded reuse | `ComputeProgram`, device-resident intermediates | Prepared f32/u32/f16/BF16 schedules including indexing/count composition, statistics and attention; NVIDIA execution pending | Fixed-shape f32/u32/f16/BF16 programs; typed replay qualification below |

`tensor-core` tests check shape/layout contracts independently of any GPU. The
optional `conformance` module runs identical deterministic scenarios against each
backend: transposed 3D values, reshape order, scalar/empty broadcasting, all
canonical arithmetic operations, multi-axis sums, batched strided matmul and zero
contractions. Backend-specific tests add ownership, limits and native error paths.

MLX evidence and exact runtime versions are recorded in
[`compute-mlx/README.md`](../../crates/compute-mlx/README.md) and its
[`qualification/native-metal.txt`](../../crates/compute-mlx/qualification/native-metal.txt).
This host has an Apple GPU and no NVIDIA CUDA device. Compilation and precision
policy tests cannot establish that a CUDA kernel executed or that Tensor Cores
were selected.

CUDA source compilation is now checked separately with NVRTC 12.8.93 in a local
Linux aarch64 container: all 52 current kernels compile for compute_70/80/90/120 using
the runtime's precise-math options. See the [current compiler snapshot](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-typed-programs/report.json)
and [compiler evidence](../../crates/compute-cuda/qualification/README.md).

The [typed indexing qualification](../qualification/tensor-indexing-2026-09-27.md)
records the subsequent common masks/scans/gather/compaction checks and full
regression, including explicit required-CUDA failure on this host.
The subsequent [reduction/vector qualification](../qualification/tensor-reductions-2026-09-27.md)
records expanded operations, native MLX empty-batch handling and the updated
23-kernel CUDA compiler snapshot. The [scatter qualification](../qualification/tensor-scatter-2026-09-27.md)
adds deterministic indexed updates, current regression evidence and the
27-kernel CUDA compiler snapshot.
The [low-precision qualification](../qualification/tensor-low-precision-2026-09-27.md)
adds exhaustive f16/bf16 storage/casts, direct low-input products, the MLX
subnormal conversion fix, measured packed-storage timing and the 30-kernel CUDA
compiler snapshot.
The [statistics qualification](../qualification/tensor-statistics-2026-09-27.md)
adds distributions, stable moments and normalization, 249-test GPU regression,
35-test native MLX evidence, measured softmax timing and the 35-kernel CUDA
compiler snapshot.
The [attention qualification](../qualification/tensor-attention-2026-09-27.md)
adds masks, GQA and signed causal alignment, overflow corrections, 256-test
GPU regression, 42-test MLX evidence and the 36-kernel CUDA compiler snapshot.
Its matched WGSL decode case is 2.68× faster than composition; the other three
measured GPU cases are slower, so prefill/wide-value throughput remains open.
The [low-arithmetic qualification](../qualification/tensor-low-ops-2026-09-27.md)
adds explicit low rounding, exact signed-zero/subnormal extrema and direct
f32-accumulating reductions. MLX uses custom Metal kernels because its native
low operations flush BF16 subnormals and return low-precision reductions.
The [low-indexing qualification](../qualification/tensor-low-indexing-2026-09-27.md)
adds exact raw routing, IEEE comparisons and f32 prefix scans. MLX's native BF16
`where` altered payloads, so a custom raw selector is shared by its indexing
planners. Direct WGSL paths are compared with matched resident f32 expansion.

Run installed backends as required tests, preserving a JSON report and raw logs:

```sh
python3 scripts/qualify-tensor-backends.py --offline \
  --backend wgsl --backend mlx --output crates/target/tensor-qualification
# On an NVIDIA host with the CUDA driver, NVRTC and cuBLAS installed:
python3 scripts/qualify-tensor-backends.py --offline \
  --backend cuda --output crates/target/tensor-cuda-qualification
```

The runner executes backends serially. A missing required runtime fails the run;
ordinary workspace tests may skip optional runtimes. The normal CI workflow
checks architecture, tensor contracts and strict Clippy for all three adapters.
Those checks do not establish hardware execution on the CI machines.

## Tensor Core policy

`MatmulPrecision::F32` is the default. CUDA maps it to pedantic f32 cuBLAS
computation. `AllowTf32`, `AllowF16` and `AllowBf16` explicitly permit lower
precision input computation with f32 storage/output. They are not equivalent to
native half-precision storage.

CUDA checks device capability and the TF32 environment override before accepting
a requested mode. The API reports that Tensor Cores are permitted; cuBLAS selects
the actual kernel. Demonstrating Tensor Core execution requires an NVIDIA run
with a profiler or instruction metrics in addition to numerical and performance
checks. Unsupported modes must not silently become a different arithmetic policy.

## Remaining work toward full support

1. Run strict CUDA conformance, invalid-input tests and precision comparisons on
   NVIDIA hardware. Preserve device/version/kernel evidence and establish actual
   Tensor Core usage. Qualify WGSL beyond Metal and MLX beyond this installed ABI.
2. Expand numerical operation coverage beyond the current arithmetic, reductions,
   scans, indexing, normalization and forward attention. Preserve empty,
   overflow and numerical contracts across
   all runtimes before promising general tensor parity.
3. Qualify native WGSL f16 arithmetic. Broaden direct low-input/f32-output MLX
   matrix measurements beyond the current Apple GPU and shapes. Preserve
   explicit rounding and numerical tolerances per operation and precision.
4. Qualify the prepared CUDA operation set and implemented CUDA Graph mode on NVIDIA. The [CUDA execution design](cuda-reusable-execution-2026-09-27.md)
   describes prepared buffers, private graph slots and explicit stream bridges. Qualify allocation reuse,
   fusion, launch overhead and memory traffic before selecting defaults.
5. Route domain math through the shared resident primitives where appropriate,
   retaining specialized nearest-neighbor and point-cloud kernels. Existing CUDA
   domain support and CPU fallback wrappers are separate from common tensor parity.
6. Run the strict backend qualification on provisioned hardware in CI and qualify
   deployment packages. Native local success is not browser, packaging or CI
   execution proof.

New APIs should extend narrow contracts or backend-specific facilities as needed.
Do not put rendering, placement heuristics, point-cloud semantics and every
future numerical algorithm into one universal backend trait.

### Reusable MLX programs

`MlxProgramBuilder` records fixed-shape f32/u32/f16/BF16 graphs. Typed inputs
and results preserve the low tensor wrapper; `run_typed` accepts current
resident handles and produces fresh lazy outputs. The original `input`/`run`
transport remains f32-only. Views preserve dtype, u32 operations retain integer
semantics, and low casts/arithmetic/reductions/direct matmul share their eager
lowering recipes. Low arithmetic retains each node's final rounding.

Native tracing runs raw C operations inside the existing native-call lock.
Local guards release temporary handles within that lock. The program prepares
kernel and constant owners beforehand and retains them until its native closure
is freed. Typed input signatures are checked before tracing; callback errors
and panics remain inside the C boundary. Compiler symbols form an optional
group, and the adapter preserves global compilation settings.

The [original f32 qualification](../qualification/tensor-mlx-programs-2026-09-27.md)
remains a separate historical result. The
[typed qualification](../qualification/tensor-mlx-typed-programs-2026-09-27.md)
records the expanded contracts, tests and matched eager/compiled measurements.
Compiled programs reuse tracing; fixed GPU allocations and general numerical
equivalence under fusion require separate qualification. Indexing, scans,
compaction and scatter now share eager and compiled recipes for all four dtypes;
statistics/normalization and attention share lowering for f32 and native low inputs.
The [compiled indexing qualification](../qualification/tensor-mlx-index-programs-2026-09-27.md)
covers changed masks/indices, resident count composition and fresh scatter state
in both native compile modes.

### Prepared CUDA programs

`CudaProgramBuilder` records fixed-layout f32/u32/f16/BF16 arithmetic, casts,
comparisons/selection, views, reductions, mean, matmul and scan/gather/compact/
scatter, statistics/normalization and attention. Scalar GPU counts compose with later nodes. Native16 intermediates
remain two bytes per element; low reductions and GEMM can produce f32 directly.
Low arithmetic nodes round individually, and low reduction/matmul outputs add
one final cast. `run_typed_into` validates all bindings and current precision
policies before reusing prepared scratch/metadata with caller-owned outputs.
Legacy `run`/`run_into` require f32 input and output signatures. Shared launch
helpers serve eager and prepared execution. Every replay resets invalid counters,
compaction tails/counts and Replace owners; raw low scatter scratch includes
safe full-word padding. `stats().memset_calls` counts asynchronous clears
separately from kernel launches. Stable statistics and attention retain private
f64 state, included in preparation budgets. Singleton/zero-key identities are
rewritten on every replay; eager and prepared calls use the same launch helpers.

The [prepared indexing qualification](../qualification/tensor-cuda-index-programs-2026-09-27.md)
records the expanded planner, reset/budget checks and resident usage example.
The [typed host/NVRTC qualification](../qualification/tensor-cuda-typed-programs-2026-09-27.md)
retains CPU contracts and all 52 unchanged kernel entries for four virtual
architectures. Required native tests report unavailable hardware on this host.
The [earlier f32 qualification](../qualification/tensor-cuda-programs-2026-09-27.md)
is retained. CUDA numerical execution, performance, native graph capture/replay and Tensor
Core instruction selection remain unverified. The [graph implementation and host checks](../qualification/tensor-cuda-graphs-2026-09-27.md)
cover owned dense slots, logical view rebasing, explicit stream bridges and resource cleanup.


The [statistics/attention qualification](../qualification/tensor-prepared-statistics-2026-09-27.md)
records the current CUDA host checks and MLX native enabled/disabled results.

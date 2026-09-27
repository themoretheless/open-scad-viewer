# Scaled dot-product attention qualification

Date: 2026-09-27. This extends the
[statistics phase](tensor-statistics-2026-09-27.md) with resident forward
attention across WGSL, CUDA and MLX. NVIDIA runtime qualification and the
broader compute objective remain in progress.

## Common behavior

`TensorAttentionBackend` computes `softmax(scale * Q * Kᵀ + bias) * V`.
`AttentionPlan` validates shapes independently of the runtimes:

- Rank-two matrices or `[..., heads, sequence, depth]` tensors, with leading
  batch broadcasting and arbitrary positive/zero strides and storage offsets.
- Grouped query attention: consecutive groups of Q heads share K/V heads.
  Q heads are a positive multiple of the matching K/V head count.
- Resident u32 Keep masks, where every nonzero bit pattern means true, or
  additive finite f32 biases with negative infinity for excluded positions.
- Explicit signed causal offsets: key `j <= query_i + offset`. Offset zero
  gives upper-left alignment; `Lk-Lq` gives lower-right decoding alignment.
  Causal and explicit masks combine.
- Fully masked rows and empty key sequences return zeros. Empty batch, query
  or V-depth dimensions retain empty outputs. Heads and Q/K depth stay positive.
- Default scale `1/sqrt(D)` or any finite override, including zero/negative
  values. No dropout or automatic differentiation is implemented by this API.

All intermediates stay on the GPU. Finite Q/K/V, finite dot products and finite
allowed scaled/biased logits are the shared numerical domain. Constant finite
V values, including `f32::MAX`, must remain finite despite an overflowing
unnormalized numerator. Rounding, reduction order and intermediate underflow
follow each backend's f32 limits; this is not a bitwise parity promise.

## Backend execution

WGSL uses 32-key tiles, shared maxima/weights and a per-lane normalized V
accumulator. A workgroup produces one query's 64-channel value tile. Wide V
recomputes QK per value tile; operands retain their checked strided layouts.
Recorded `_into` operations validate aliases, ownership, offsets and output
shape before appending stages. Repeated submissions overwrite the output and
temporary state after input or mask changes.
For at most 64 query rows and at least 512 keys, up to 64 key ranges execute
independently and merge on the GPU. Partial values are capped at 4 MiB, with
at most 32 KiB of summaries. There is still no score matrix allocation.

CUDA computes each key's QK once per query using warp reductions. A 32-key
tile shares weights across V channels. Dot products, online maxima/sums and
the output-sized accumulator use f64 internally, with a final f32 output.
Scratch requires eight bytes per output element; there is no score matrix.
This custom kernel does not claim Tensor Core use or CUDA speedup.

MLX normalizes batch/head layouts for native rank-four fast SDPA. The adapter
converts Keep masks to additive zero/negative infinity, tracks valid rows on
the GPU, and selects zero for fully closed rows. Native lower-right causal
alignment is used only when it matches the requested offset. Other offsets
construct a device mask. Scales with magnitude above one use a device graph
that applies scale after QK, preventing native pre-scaling from overflowing Q.
Unsupported native head shapes can also use MLX's own device graph. Explicit
masks and graph paths can allocate score-sized arrays: MLX does not share the
WGSL/CUDA memory guarantee.

## Defects found and fixed

### WGSL large-value scaling

The initial shader returned zero for constant `V = f32::MAX`. Protective
floating power-of-two factors did not prevent the observed reciprocal/scale
failure on Metal. The final helper adjusts exponent bits explicitly, retaining
the scaling through shader optimization. When an earlier tile's weight becomes
zero, its old V scale and normalized state are discarded without evaluating a
ratio against the new smaller scale. Tests include transitions from MAX to 1,
1e-30 and zero, and subnormal inputs under the documented underflow floor.

[Original failure](../../crates/compute-core/benchmarks/tensor-attention-initial-failure.txt),
[targeted retest](../../crates/compute-core/benchmarks/tensor-attention-extreme-retest.txt),
[numerical and recorded-program contract](../../crates/compute-core/benchmarks/tensor-attention-contracts.md).

### MLX mask and numerator differences

MLX 0.32.1 returned different fully masked results depending on native kernel
selection: a generic bool-masked row became a uniform average; an additive
negative-infinity row could become NaN. A device validity reduction and final
zero selection make these cases consistent.

The native path also returned Inf for constant MAX values. The adapter now
scales V down by `2^ceil(log2(Lk))`, then restores the output by the same exact
power of two with a finite bound. The factor depends only on sequence length;
a large masked V does not set the scale. Very small intermediate values can
still underflow, as documented. The same protection applies to the explicit
MLX device graph.

[Native C probe](../../crates/compute-mlx/qualification/attention-native-probe.c),
[probe output](../../crates/compute-mlx/qualification/attention-native-probe.txt),
[original MAX failure](../../crates/compute-mlx/qualification/attention-extreme-initial-failure.txt),
[complete corrected run](../../crates/compute-mlx/qualification/attention-metal.txt).

## Shared and backend-specific checks

The shared dense f64 oracle independently derives batch/head addresses, QK,
the max-shifted softmax and PV. It does not call `AttentionPlan`, backend matmul
or the shared statistics reference. Hand-computed uniform GQA and masked causal
examples test the oracle itself. Every backend fixture checks source inputs
remain unchanged.

Cases include rank-two and rank-five tensors, broadcast GQA, permuted Q/K/V
and masks, partial tiles, key length 4097, value depth 67, negative/zero/large
scales, logits around ±10000, both mask types, signed causal extrema, empty
inputs and invalid geometry. Extreme V fixtures cover constant 1e30/MAX,
discarded large-value tiles and masked large values with ordinary live values.

WGSL adds 131,077 keys, 65,537 query rows, reused offsets and sentinel buffers,
changed masks, a downstream resident sum, failure integrity and conceptual
score shapes larger than u32 without allocating a score tensor. CUDA's gated
fixture includes depth 35/value-depth 263 tails, changed V, row-grid reuse and
foreign runtime checks. MLX checks native vector/full/two-pass eligible shapes,
unsupported head shapes, lazy input lifetimes and wrong dtype/ownership errors.

## CUDA compilation evidence

Actual NVIDIA NVRTC 12.8.93 compiled all **36 entrypoints** for compute_70,
compute_80, compute_90 and compute_120 with the runtime's precise-math options.
The 33,640-byte seven-part source has SHA256
`0068632b243d7d60f54f94f4bf21652a0637b4fd4415e2064e9896f6f8a592d7`.
All source parts, the runtime's exact concatenation and four retained PTX
hashes match the [report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-attention/report.json).
Entry parameter widths were checked, including `attention_f32`.

Compilation does not validate device launches, numerical CUDA output or Tensor
Core instructions. Required CUDA mode fails on this Apple-only host, rather
than silently passing through a missing-device skip. NVIDIA performance and
numerical conformance remain pending hardware.

## Final verification

| Check | Result |
| --- | --- |
| gpu-compute / compute-core / raster-core / osv-math regression with required GPU, timestamps and subgroups | **256 passed, 0 failed, 0 ignored**, 39 suites |
| Focused WGSL attention suite after split-key change | **6 passed** on Metal |
| Shipped assembled WGSL sources validated by Naga | **44 passed** |
| Complete MLX suite with required runtime | **42 passed** on Metal; MLX-C 0.6.0, MLX 0.32.1 |
| Shared CPU contracts, including independent reference tests | **25 passed** |
| CUDA host tests / doctest | **12 / 1 passed** |
| CUDA NVRTC source and parameter ABI | **36 kernels × 4 architectures** |
| Strict Clippy and rustdoc, four tensor crates | Pass |
| wasm32 compilation: tensor-core, compute-core, compute-mlx | Pass |
| Dependency architecture checker | Pass |
| NVIDIA numerical execution and Tensor Core profiling | Pending hardware |

Full regression command:

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math \
  --features osv-math/gpu -- --test-threads=1
```

[GPU regression](tensor-attention-2026-09-27/gpu-regression.txt),
[focused WGSL](../../crates/compute-core/benchmarks/tensor-attention-split-metal-tests.txt),
[MLX](../../crates/compute-mlx/qualification/attention-metal.txt),
[contracts](tensor-attention-2026-09-27/contracts.txt),
[CUDA host](tensor-attention-2026-09-27/cuda-host.txt),
[CUDA doctest](tensor-attention-2026-09-27/cuda-doctest.txt),
[required CUDA failure](tensor-attention-2026-09-27/cuda-required.txt),
[Clippy](tensor-attention-2026-09-27/clippy.txt),
[rustdoc](tensor-attention-2026-09-27/rustdoc.txt),
[WASM](tensor-attention-2026-09-27/wasm.txt),
[device and compiler versions](tensor-attention-2026-09-27/environment.json),
[qualified source fingerprints](tensor-attention-2026-09-27/source-fingerprints.json).
Existing manifest warnings concern `wgsl_export` naming and duplicate `bench`
example output names when several crates are tested together.

## Measured performance

The [benchmark report](../../crates/compute-core/benchmarks/tensor-attention.md)
compares streaming attention with resident public matmul → scale → softmax →
matmul on Apple M4 Max. The initial unsplit decode was 6.1 times slower.
After splitting keys, its GPU median was **0.236292 ms versus 0.633917 ms**
for composition, a 2.68× improvement in the matched final run. Partial values
and summaries used 17,952 bytes, compared with 196,656 bytes for composition's
three score-shaped buffers. These counts exclude other allocations.

The other three measured GPU cases remain **57–162% slower** than composition,
while using no score-shaped arrays. Prefill/wide-value throughput needs further
work; a general speedup is not established. Every timed execution passed the
independent f64 reference. Both raw runs and exact initial source snapshots
are retained, with final source fingerprints.

## Remaining scope

Attention backward/dropout, native low-precision attention, Tensor Core
instruction qualification, broader numerical operations, CUDA/MLX graph reuse,
other GPU/browser targets and provisioned hardware CI remain in the
[full compute plan](../design/tensor-backends-2026-09-27.md).

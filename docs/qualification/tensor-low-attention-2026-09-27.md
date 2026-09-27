# Attention from low storage: qualification

Date: 2026-09-27. This extends [low statistics](tensor-low-statistics-2026-09-27.md)
with direct f16/BF16 Q/K/V attention on WGSL, CUDA and MLX. The full compute
goal remains active; NVIDIA execution and Tensor Core profiling remain open.

## Contract and reference

`TensorLowAttentionBackend` adds `attention_low_f32` and `attention_low`.
The first returns the evaluated f32 result; the second performs one final
nearest-even cast into Q's dtype. All three operands must have matching dtypes,
including empty cases. Masks remain f32 additive biases or exact u32 keep
values. `low_attention_plan` checks dtype equality before the shared geometry.

Rank-two inputs and leading batch broadcasts, GQA, signed causal offsets,
empty keys/queries/value channels and fully masked rows follow the f32
attention contract. Inputs are finite; allowed dot products/logits and
normalized outputs stay within f32 range. Normal products formed from BF16
subnormal operands and large finite partners must survive initial input
flushing. True subnormal arithmetic/results otherwise follow backend f32
limits. Finite constant V must not overflow an unnormalized numerator.

No executor expands complete Q/K/V operands to f32 or materializes complete
score/probability matrices. Bounded tiles/partials and actual result-shaped
accumulators are allowed. All intermediate data stays on device.

The shared fixture decodes raw storage independently and reuses the dense f64
attention oracle. The oracle does not use the executor's online recurrence or
`AttentionPlan`. Cases cover transposed and zero-stride inputs/masks, broadcast
batches, grouped heads, 32/64-channel tails, 4097 keys, signed causal extremes,
fully masked rows and validation before empty shortcuts. Additive biases
65536/65537/65538 verify that masks retain f32 precision. Uniform thirds verify
that f32 output was never rounded through low storage. Every low output is
checked against an independent exact nearest-even cast of its actual f32 result.

Strict-relative cases include constant extreme V, discarded early large-value
tiles, tiny normal outputs and BF16 subnormal Q/K products in both operand
orders. For example, minimum BF16 subnormal times maximum BF16 is the normal
f32 value 255/8192. A chain of two attention operations reads only its final
result. Separate CPU tests check dtype validation and the tiny-product oracle.

## WGSL

The low path reuses the f32 attention planner, 32-key streaming tiles,
64-channel value tiles and split-key merge. Q/K/V loads specialize the storage
format. Few-query long-key work may use up to 64 parts, subject to the existing
four-MiB partial-value budget and device limits. Additional value tiles repeat
QK work; no matrix multiplication instruction claim is made.

Pipelines specialize by storage dtype. BF16 only takes the protected multiply
path for nonzero subnormal operands. F16 normals use `unpack2x16float` after
extracting the selected halfword; zeros, subnormals and special encodings keep
the exact integer decoder. Extracting first also isolates the unused neighbor.
This guard matters because WGSL permits unpacking builtins to flush subnormal
intermediates. [WGSL floating-point rules](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#floating-point-evaluation),
[unpack definition](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#unpack2x16float-builtin).

The production loader passed 63,488 finite half encodings × nine neighboring
payloads × two physical halfword positions: **1,142,784 exact f32 bit checks**,
including signed zero. A separate retained probe checks all 65,536 encodings,
infinities and NaN classification with three replays. Raw unpack also passed
on this Metal device; the portable guard remains necessary under the WGSL
contract. These checks measure correctness, not speed.
[Probe source, logs and reproduction](tensor-low-attention-2026-09-27/decoder-probe/README.md).

F16 caches one decoded query with depth at most 256 in 1 KiB of workgroup
storage. Larger depths load directly. Cache contents are refreshed for every
query/part/value tile, including workgroup grid reuse. BF16 has no query cache
or added barrier. This is bounded local scratch; global partial allocations
and the shared f32 path remain unchanged.

Exact power-of-two helpers are shared with statistics. They protect tiny Q/K
operands before multiplication and keep value ratios and score differences in
safe arithmetic ranges. The existing centered statistics formulas are unchanged
by extracting these helpers. Normalized partial values protect extreme V and
drop historical scaling when an earlier tile loses all weight.

Both output forms support recorded `_into` execution. Output ownership, shape,
contiguity, dtype and whole-buffer aliases, including cross-type aliases, are
validated before appending work. Odd output halfword offsets preserve adjacent
storage. Focused tests rewrite Q/K/V and masks between replays, cover fully
disabled masks, check sentinels, and compose a downstream resident reduction.

[WGSL contracts](../../crates/compute-core/benchmarks/tensor-low-attention-contracts.md),
[final focused Metal checks](../../crates/compute-core/benchmarks/tensor-low-attention-f16-cache-metal-tests.txt).

## CUDA

A native u16 loader feeds the existing shared 32-key online attention body.
QK, weights and the output-sized accumulator use f64 internally. Mask handling,
offsets, GQA and stream execution share the original f32 planner; its exported
ABI remains unchanged. Wider arithmetic is a stability choice with unmeasured
NVIDIA hardware cost. It does not expose general f64 tensors or establish
Tensor Core use.

NVRTC 12.8.93 compiled **51 entries for compute_70/80/90/120**. The 12-part
source is 48,291 bytes, SHA256
`d8a6cb52807f369f03f23bc5f01f226027bbbed1e6cee38bb4859b5d21699327`.
All ordered parameter ABIs, source parts and retained PTX hashes were checked.
The native fixture is compiled and explicitly skipped without NVIDIA; required
mode fails rather than counting absence as a successful numerical test.

[Compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-attention/report.json),
[source/ABI audit](../../crates/compute-cuda/qualification/low-attention-source-audit.txt),
[host tests](../../crates/compute-cuda/qualification/low-attention-host-tests.txt),
[required-runtime failure](../../crates/compute-cuda/qualification/low-attention-cuda-required.txt).
There is no NVIDIA numerical, launch, performance or Tensor Core proof yet.

## MLX

A custom Metal kernel streams 32 keys for one query and 64 output channels.
It consumes native low arrays through shape/stride metadata, including
broadcast batches and masks. It returns f32 directly; widening MLX's native
low attention result would not satisfy that precision contract. Tiny-product
and value-range protection reuse the shared low integer scaling helpers.
The low result uses the existing final exact cast.

For `attention_low_f32`, core tensor allocations are the actual 4N-byte f32
output (N output elements) and 52-byte parameter buffer, plus a four-byte
absent-mask sentinel when needed. Each workgroup uses 384 bytes of score/weight
scratch and per-thread registers. `attention_low` additionally creates the 2N-byte
low output and, for BF16, output-sized integer cast intermediates.
There are no global partial, score or operand-conversion arrays. Native driver,
cache, scheduling, compiler spill and readback staging allocations are outside
this count. Wide V repeats QK
for each 64-channel tile, and long-key rows remain serial across key tiles.
No MLX performance measurement is claimed.

The first four focused tests passed, followed by the full 64-test suite.
Private cases include 263-channel strided GQA, an excluded first tile, 65,537
rows beyond the bounded grid, lazy lifetimes and validation. Extracting shared
power-of-two helpers also passed the existing low-statistics suite.

[First native checks](../../crates/compute-mlx/qualification/low-attention-initial.txt),
[full native log](../../crates/compute-mlx/qualification/low-attention-metal.txt),
[source/log manifest](../../crates/compute-mlx/qualification/low-attention-source-manifest.json).

## Verification

| Check | Result |
| --- | --- |
| Required compute-core / gpu-compute / raster-core / osv-math GPU regression | **285 passed**, 0 failed, 0 ignored; 44 suites |
| Final focused WGSL library / low-attention integration | **13 / 4 passed** |
| Production F16 loader | **1,142,784 exact finite comparisons** |
| Required full MLX suite | **64 passed**: 3 unit + 61 integration |
| Shared CPU contracts and references | **36 passed** |
| CUDA host tests / compile doctest | **13 / 1 passed**; native fixture explicitly skipped |
| NVRTC compilation and ABI | **51 entries × 4 architectures** |
| Strict all-target Clippy and strict rustdoc, four tensor crates | Pass |
| wasm32 check: tensor-core, compute-core, compute-mlx | Pass |
| Architecture, changed Rust formatting, diff whitespace | Pass |
| NVIDIA numerical execution / Tensor Core profiling | Pending hardware |

[GPU regression](tensor-low-attention-2026-09-27/gpu-regression.txt),
[CPU contracts](tensor-low-attention-2026-09-27/contracts.txt),
[test summary](tensor-low-attention-2026-09-27/test-summary.json),
[Clippy](tensor-low-attention-2026-09-27/clippy.txt),
[rustdoc](tensor-low-attention-2026-09-27/rustdoc.txt),
[WASM](tensor-low-attention-2026-09-27/wasm.txt),
[architecture](tensor-low-attention-2026-09-27/architecture.txt),
[formatting](tensor-low-attention-2026-09-27/rustfmt.txt),
[diff check](tensor-low-attention-2026-09-27/diff-check.txt),
[environment](tensor-low-attention-2026-09-27/environment.json),
[final source fingerprints](tensor-low-attention-2026-09-27/source-fingerprints.json),
[artifact audit](tensor-low-attention-2026-09-27/final-artifact-audit.txt).
Existing Cargo warnings concern `wgsl_export` naming and duplicate `bench`
example names. No skipped CUDA fixture counts as a hardware test.

## Measured speed and memory

The [matched Metal benchmark](../../crates/compute-core/benchmarks/tensor-low-attention.md)
compares three full Q/K/V casts plus f32 attention with the direct low path.
Both return f32, use the same planner and validate against dense f64 attention.
Five geometries cover small attention, 4097-key decode, prefill, wide-V GQA and
strided/masked inputs. Both dtypes use 200 ms warmup and 31 rotated samples per
timing mode. GPU timestamps and unprofiled host/readback timings are separate.

Two unchanged final runs show lower direct GPU medians in **3/10 cases**, all
BF16, and higher medians in **7/10**. The cast/direct ratio ranges from
**0.690× to 1.112×**; the largest slowdown is **45.0%**. The small BF16 win is
about 1%, so it does not establish a broad speed advantage. Direct loads remove
14,156–4,196,352 bytes of f32 operand buffers in these shapes. Those are logical
allocation counts, not measured peak device memory. Final F16 query caching
adds 1 KiB of local workgroup storage while leaving global partials unchanged.

The initial generic decoder, per-dtype specialization, cache for both dtypes,
guarded conversion without cache, and retained F16-only cache all have preserved
source snapshots and two raw runs. Large timing shifts in some historical
samples affected both paths; their cause was not profiled. No samples were
removed. The report retains regressions, host timings, p90 and all samples.
The [audit](tensor-low-attention-2026-09-27/audit_benchmarks.py) recomputes each
stage's 80 median/p90 pairs from 2,480 samples.

## Remaining scope

Native WGSL f16 arithmetic, direct low-input/f32-output MLX matrix products,
additional tensor operations, reusable CUDA/MLX execution, domain integration,
NVIDIA hardware evidence and deployment/CI qualification remain part of the
full goal. Low-storage attention adds a shared forward operation; dropout and
automatic differentiation are not implemented by this API.

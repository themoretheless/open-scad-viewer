# Direct low-input MLX matrix products: qualification

Date: 2026-09-27. This extends [low attention](tensor-low-attention-2026-09-27.md)
by filling the existing MLX `matmul_low_f32` capability gap. The retained route
is qualified on Apple M4 Max with MLX 0.32.1 and MLX-C 0.6.0. It avoids full
operand expansion and has lower measured medians in four of twelve cases.
The full compute goal remains active.

## Contract and planning

The existing `TensorLowBackend` API accepts matching f16/BF16 inputs and returns
a genuine f32 result from f32 accumulation. It must not widen an already rounded
low result or expand complete inputs to f32. Matrix, vector and broadcast batch
shapes follow `MatmulPlan`; two vectors return a scalar. Empty contractions
produce zeros and empty result shapes return empty native arrays. Runtime owner,
dtype and shape checks precede shortcuts. The older native low-output
`matmul_low` path remains available independently.

The MLX implementation presents promoted and broadcast operands as native views.
A right vector becomes a column using singleton broadcast and transpose, without
reshaping a strided allocation into a copy. Kernel indexing uses the supplied
native shape/stride metadata and 64-bit logical offsets. The grid is capped at
65,535 workgroups and loops over remaining output tiles. Inputs remain on device
and graph nodes retain them after Rust input handles are dropped.

`matmul_f32` requires the native dtype accessor and the optional custom Metal
symbol group. Missing support returns an explicit error. Reporting support does
not establish use of NVIDIA Tensor Cores or any particular machine instruction.

## Scalar tiled baseline

The initial kernel computes 16×16 output tiles with 256 threads and two shared
16×16 f32 operand tiles. Each output has its own f32 accumulator. Input decoding
happens during tiled loads, so shared operand storage is bounded at 2 KiB.
The actual output is f32 and the parameter array has seven u32 words (28 bytes).
There are no global operand conversions or partial-result arrays. These are
source-level allocation sizes; allocator, driver, graph and compiler spills
are outside the count.

The initial five focused matmul tests and five existing low-precision tests
passed on the first Metal run. The full MLX suite then passed 69 tests. Private
checks cover precise f32 output, cancellation, both transposed operands,
broadcast batches, partial tiles, lazy lifetimes, zero strides and 65,537 output
tiles. Tiny BF16 inputs multiplied by large finite partners retain their normal
f32 products in both operand orders through the shared protected multiply.
This property is qualified specifically for this custom MLX path.

Both initial timing runs found the scalar tiled route slower in all 12 cases.
The [benchmark report](../../crates/compute-mlx/benchmarks/low-matmul.md) retains
all samples, workload definitions and source snapshots. Timings construct fresh
MLX graphs for each replay; repeated evaluation of a cached result is excluded.
They include host graph construction, evaluation and full result readback,
without claiming GPU-only timing.

## SIMD-group numerical probe

The installed MLX 0.32.1 GEMM source uses float SIMD-group matrix fragments for
low-input accumulation. A standalone custom-kernel probe checked this approach
before adapting it to the new API. Ordinary values and minimum positive f16
times maximum finite f16 produced exact results in all 64 matrix entries for
three replays. BF16 subnormal inputs instead produced zero although the expected
matrix products were normal f32 values. Minimum BF16 subnormal was checked in
both operand orders; maximum subnormal was checked on the left.

The probe does not isolate which load/multiply/compiler step drops the tiny
input and establishes no performance result. It motivates a guarded candidate
that retains protected scalar products for affected BF16 tiles.
[Probe source, results and reproduction](tensor-low-matmul-2026-09-27/simdgroup-probe/README.md).

## SIMD-group candidate

Four SIMD groups (128 threads) share a 16×16 output tile. Each group keeps an
8×8 float matrix accumulator and consumes the two 8-wide halves of each
16-wide contraction tile. Direct low loads feed bounded shared f32 tiles;
ordinary tiles use `simdgroup_multiply_accumulate`. Final shared stores allow
scalar bounds checks for output tails. Every grid-loop iteration resets the
accumulator and all lanes participate in the shared-memory barriers.

BF16 loads inspect raw f32 bits for nonzero subnormal operands. Each SIMD group
publishes a flag, and their combined flag gives one uniform branch for the
whole workgroup. An affected tile stores its current accumulators, evaluates
protected scalar products, then reloads the float matrix accumulators. This
preserves contributions across ordinary/tiny/ordinary tile sequences. F16
values are normal after widening to f32 and do not need this branch.

The source declares three 256-float tiles and four u32 flags: at most 3,088
bytes of shared storage per workgroup, excluding compiler spills and driver
allocations. Global output and parameter sizes are unchanged. This is a
bounded tile conversion, with no complete f32 operand expansion.

The first focused 15 tests passed, followed by the full 70-test MLX suite.
Added cases place a tiny BF16 input in the first, middle and last K tiles,
in both operand orders, with 17×19 output tails. These check that the protected
branch retains previous results and correctly resumes the ordinary path.
The scalar baseline and its logs remain preserved for matched measurements.

Both SIMD timing runs also found the direct route slower in all 12 cases.
For example, the 128×257 by 257×192 f16 case took 0.247/0.251 ms for the
direct route versus 0.138/0.144 ms for cast→f32 in the two runs. These wall
measurements include graph construction and readback; they do not identify a
device bottleneck. SIMD matrix instructions alone did not establish a benefit
for this implementation. The [paired results and archived sources](../../crates/compute-mlx/benchmarks/low-matmul.md)
remain available for comparison with the next candidate.

## Retained matrix and vector routes

The third candidate keeps the SIMD matrix algorithm and its BF16 protection.
One lane now decodes the native batch base for each operand and shares those
two addresses with the workgroup. Fixed row, column and contraction strides
then address every tiled load. Generic rank traversal and integer division
move outside the contraction loop. Transposed and broadcast views retain
their native strides, including zero strides.

Canonical matrix shapes with M=1 or N=1 use a separate vector kernel. Each
256-thread workgroup owns one output, partitions K across its lanes, then
reduces f32 partial sums through a shared tree. BF16 multiplication uses the
same protected helper. Workgroups loop over outputs beyond the grid bound;
a final barrier protects shared bases and partials before reuse. This also
handles vector dots and preserves their scalar result shape.

The local planner clones a native view when its shape already matches the
required promoted/broadcast shape. Validation still precedes this shortcut.
The three changes were measured together as one candidate; no individual
timing contribution is inferred.

Source declarations reserve at most 3,104 bytes of shared storage for GEMM
and 1,040 bytes for the vector route. Both upload seven u32 parameter words
and return f32 output without global expanded operand buffers or partial-result
arrays. These counts exclude compiler spills, registers, allocator and driver
storage. They are not a measured peak-memory reduction.

All 13 focused tests and the full **72-test native MLX suite** passed on their
first runs. Added vector tests cover inexact normal products at K=65/257/1025,
transposed inputs, zero contraction strides and tiny BF16 contributions in
middle/tail positions in both operand orders. Both matrix and vector routes
separately exercise 65,537 output workgroups. Strict MLX Clippy and rustdoc
also passed. The [four-file overlay](../../crates/compute-mlx/qualification/low-matmul-routed-source-manifest.json)
records the exact source and [full test log](../../crates/compute-mlx/qualification/low-matmul-routed-metal.txt).

## Final matched measurements

The retained candidate was measured twice with the original benchmark, resident
input handles, alternating path order and fresh output graphs. All 1,488 measured
output buffers matched the independent f64 oracle exactly. The 85 captured
source files and benchmark binary were unchanged through both runs.

Four of twelve cases had lower direct-path medians in both runs: vector dot
and vector×matrix for f16 and BF16. Their baseline/direct ratios range from
1.021 to 1.122. The other eight cases had higher direct-path medians, ranging
from near parity to 10.1% more wall time. There is no general speedup claim.
In particular, the 128×257 by 257×192 matrix case remains slower for both
dtypes. These measurements include host planning, allocation, evaluation and
full readback, so they do not establish a GPU-only speedup or isolate the
contribution of any of the three combined edits.

The [complete report](../../crates/compute-mlx/benchmarks/low-matmul.md) retains
all twelve rows for all three implementation stages. Its six raw runs contain
4,464 samples and 144 median/p90 pairs, independently recomputed from the logs.
The initial scalar and SIMD-only candidates were slower in every measured case;
their sources and unsuccessful performance results remain preserved.

## Independent shared numerical checks

The strengthened shared fixture computes host permutations, broadcast
coordinates, vector promotion and f64 dots independently of production
`Layout` and `MatmulPlan`. Exact dyadic cases cover K=65/257, both strided
operands, five-dimensional batch broadcasts, true zero contraction strides,
vector forms, cancellation and validation before empty shortcuts. Low outputs
are checked against exact final nearest-even rounding; f32 outputs must retain
bits a low output would lose. Input views are checked unchanged.

Additional normal inputs use every low mantissa bit and varied exponents.
Their products are exact and normal in f32, but their sums exercise rounded
accumulation. The test compares with an independent f64 dot and absolute-product
sum using a forward-error interval. It uses a conservative f32 rounding unit
of 2^-23 to allow accumulation order and directed rounding, includes f64 oracle
error, and rounds interval endpoints outward before checking low outputs.
CPU checks verify that forward, reverse and pairwise f32 sums fit the bound
while widened low results are rejected. Existing exact-output checks keep
their original tolerances. There are 39 passing CPU contract/reference tests.

These new portable cases contain no subnormal inputs. NVIDIA's documented
matrix-operation rounding and subnormal behavior do not provide the stronger
tiny-input guarantee tested privately on the custom MLX path.
[PTX 12.8 matrix semantics](https://docs.nvidia.com/cuda/archive/12.8.0/parallel-thread-execution/index.html#warp-level-matrix-instructions-mma).

## Verification

| Check | Result | Evidence |
| --- | --- | --- |
| Native MLX, runtime required | 72 passed, including the strengthened shared fixture | [Raw log](../../crates/compute-mlx/qualification/low-matmul-routed-metal.txt) |
| Full WGSL/math/raster/platform regression, GPU/timestamps/subgroups required | 285 passed | [Raw log](tensor-low-matmul-2026-09-27/wgsl-metal.txt) |
| Independent CPU contracts/references | 39 passed | [Raw log](tensor-low-matmul-2026-09-27/contracts.txt) |
| CUDA host checks | 13 CPU tests and one doctest passed; native test explicitly skipped | [Raw log](tensor-low-matmul-2026-09-27/cuda-host.txt) |
| Strict all-target Clippy and rustdoc | Passed for all four tensor/backend crates | [Clippy](tensor-low-matmul-2026-09-27/clippy.txt), [rustdoc](tensor-low-matmul-2026-09-27/rustdoc.txt) |
| WASM compile check and architecture boundaries | Passed | [WASM](tensor-low-matmul-2026-09-27/wasm.txt), [architecture](tensor-low-matmul-2026-09-27/architecture.txt) |
| Formatting and whitespace | 155 changed/current Rust files passed; historical archives and five pre-existing unformatted files excluded | [Formatting](tensor-low-matmul-2026-09-27/format.txt), [whitespace](tensor-low-matmul-2026-09-27/whitespace.txt) |

The existing Cargo manifest warning about `raster-core`'s `wgsl_export` binary
name is still printed. Strict Rust Clippy/rustdoc completed successfully.
The [final commands and exit statuses](tensor-low-matmul-2026-09-27/final-checks.json)
record the GPU regression and final build checks.

CUDA kernel sources did not change in this phase. An
[ordered-source hash audit](tensor-low-matmul-2026-09-27/cuda-source-audit.json)
matches all twelve files and the 48,291-byte combined source to the previous
51-entry NVRTC report for compute_70/80/90/120. This reuses compiler evidence;
it does not run NVIDIA hardware or establish Tensor Core selection. The new
shared numerical fixture compiles into the CUDA native tests, which still need
an actual device.

The [source fingerprints](tensor-low-matmul-2026-09-27/source-fingerprints.json)
bind the final implementation and shared fixtures. The
[artifact audit](tensor-low-matmul-2026-09-27/artifact-audit.json) records test
counts, source/log validation and independent benchmark recomputation.

## Remaining goal scope

All twelve current common tensor traits have implementations in WGSL, CUDA and
MLX. Completing a shared interface is separate from qualifying every runtime.
The concrete remaining work includes NVIDIA numerical and Tensor Core evidence,
native WGSL f16 arithmetic, reusable CUDA execution and allocations, an explicit
MLX compiled-function/replay interface, domain integration, and hardware CI and
deployment qualification. This phase implements the existing forward matrix API.
Further mathematical operations need their own explicit contracts and evidence.

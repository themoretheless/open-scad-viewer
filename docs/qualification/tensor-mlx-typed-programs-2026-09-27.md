# Typed MLX program qualification

Date: 2026-09-27. This phase extends the
[fixed-shape f32 programs](tensor-mlx-programs-2026-09-27.md) with typed u32 and
low-storage execution. Native correctness qualification passed on Apple M4 Max,
MLX 0.32.1 and MLX-C 0.6.0_4. The full shader/CUDA/tensor/MLX objective remains
active; compiled indexing/statistics/attention and NVIDIA execution remain open.

## Shared recording

The implementation uses one internal lowering interface for eager execution
and recorded programs. Shape and dtype metadata, native operations, constant
arrays and custom Metal kernels form its narrow boundary. Low unary/binary
arithmetic, casts, f32 reductions and direct low-input matrix products must
reuse the same recipes and kernel sources through both paths.

Program inputs and outputs keep the existing low tensor wrapper. Returning a
low allocation as an unrestricted `MlxTensor` would expose native low
accumulation through ordinary reduction calls. Typed program transport must
preserve that boundary and reject mixed dtypes before native tracing.

Custom kernels and constant arrays are prepared outside the native-call lock.
The program retains their Rust owners; its callback payload contains borrowed
raw handles and plain metadata. The callback must not acquire `Api::call`
again or drop wrappers whose destructors acquire that lock. Preparation errors,
native errors and callback panics must release the appropriate owners.

The pinned compiler keeps primitives outside a fused region and reconstructs
them with current input arrays on replay. This suggests that custom Metal
operations can retain their rounding boundaries while participating in the
compiled graph; native execution must verify that inference.
[MLX 0.32.1 compiler](https://github.com/ml-explore/mlx/blob/v0.32.1/mlx/compile.cpp).

## Numerical requirements

- Low views preserve all raw payloads, including signed zeros and NaNs.
- Casts retain round-to-nearest-even, BF16 subnormals and special-value rules.
  A cast to low and back must retain its intermediate rounding inside a graph.
- Each low arithmetic node rounds its own result. Two half-ULP additions to
  one must not become one higher-precision expression with only final rounding.
- Low reductions produce f32 accumulations. Empty identities, hierarchy
  boundaries, exact extrema and mean divisors retain the existing contracts.
- Direct low-input matmul produces f32 output without full operand expansion
  or widening an already rounded low result.
- u32 comparisons, selection and reductions preserve unsigned values above
  `2^24` and wrapping arithmetic, with no floating-point promotion.
- Changed values and input strides reuse tracing without capturing stale data.
  Old lazy results remain valid after later runs and resource-handle drops.

## Qualified API

The original `input` and `run` retain f32-only signatures. `input_u32` and
`input_low` declare the added dtypes, and `run_typed` accepts borrowed
`MlxProgramInput::{Tensor, Low}` values. Results are
`MlxProgramOutput::{Tensor, Low}` with checked accessors. Low storage remains
inside `MlxLowTensor`; passing a typed graph through legacy `run` is rejected
before tracing if any declared input or output is non-f32.

| Operation group | Recorded behavior |
| --- | --- |
| Views | Reshape, permutation and broadcast preserve dtype and logical order |
| u32 arithmetic | Wrapping add/subtract/multiply, unsigned min/max; division rejected |
| u32 branching | All six comparisons, nonzero-mask selection and broadcast rules |
| f32/u32 reductions | Sum, product, min and max; f32 mean |
| Low casts | f32 to f16/BF16 and back through the shared cast recipe |
| Low arithmetic | Existing unary/binary Metal kernels and per-node low rounding |
| Low reductions | Shared direct f32 reductions/mean; explicit optional final low cast |
| Low matmul | Shared direct f32 route; explicit optional final low cast |

Compiled `matmul_low` composes the direct f32 product with one final cast. The
ordinary eager `matmul_low` keeps its separate native MLX low-output route.
Matched matmul measurements compare the direct f32 operation on both paths.
Low comparison/selection/indexing, statistics and attention are not yet recorded
by this API; their existing standalone implementations remain available.

## Evidence

| Check | Result | Raw evidence |
| --- | --- | --- |
| Required native MLX suite | 104 top-level entries: 6 unit, 98 integration; zero failures | [Metal run](../../crates/compute-mlx/qualification/compiled-typed-metal.txt) |
| Focused native programs | 6 unit, 10 original compiled, 19 typed entries passed | [Focused run](../../crates/compute-mlx/qualification/compiled-typed-focused.txt) |
| Externally disabled compilation | All 19 typed entries passed in a separate process | [Disabled run](../../crates/compute-mlx/qualification/compiled-typed-disabled.txt) |
| Required WGSL/Metal stack | 286 passed, GPU/timestamps/subgroups required | [WGSL run](tensor-mlx-typed-programs-2026-09-27/wgsl-metal.txt) |
| CPU tensor contracts | 40 passed | [CPU run](tensor-mlx-typed-programs-2026-09-27/cpu-contracts.txt) |
| Strict Clippy and rustdoc | tensor-core, compute-core, compute-cuda, compute-mlx passed | [Clippy](tensor-mlx-typed-programs-2026-09-27/clippy.txt), [rustdoc](tensor-mlx-typed-programs-2026-09-27/rustdoc.txt) |
| WASM build | tensor-core, compute-core, compute-mlx passed | [Build](tensor-mlx-typed-programs-2026-09-27/wasm.txt) |
| Architecture, format, whitespace | Passed; 176 changed/new Rust source files checked | [Commands and exclusions](tensor-mlx-typed-programs-2026-09-27/final-checks.json) |

The original compiled suite has one marker-only entry; its parent separately
executes the child for three native runs with compilation disabled. Typed replay
checks one trace in enabled mode and a fresh trace per invocation in disabled
mode. Exhaustive raw storage/cast checks, per-node rounding, wrapping unsigned
arithmetic, hierarchy boundaries, direct f32 products, changed strides and
resource lifetimes are covered. These tests do not measure leaks or constitute
sanitizer qualification. Cargo still reports the pre-existing `raster-core`
non-kebab-case binary-name manifest warning.

The [native source manifest](../../crates/compute-mlx/qualification/compiled-typed-source-manifest.json)
retains 106 exact source files and hashes for every native log. The
[combined source manifest](tensor-mlx-typed-programs-2026-09-27/source-fingerprints.json)
records 333 current source files across the backend stack. Previous snapshots
are retained as historical evidence. The
[final audit](tensor-mlx-typed-programs-2026-09-27/artifact-audit.json) verifies
source/archive hashes, local evidence links, test counts and timing statistics.

The first native attempt stopped on two faulty test expectations: the direct
matmul dataset lacked a value demonstrating loss at final low rounding, and
the f64 empty-sum reference produced negative zero. The corrected fixture adds
an explicit half-ULP witness and uses the specified positive-zero identity.
Bit-exact comparisons are unchanged. The
[initial log](../../crates/compute-mlx/qualification/compiled-typed-initial-failure.txt)
and original fixture are retained in the native manifest.

CUDA reusable execution is a separate stream/ownership design problem; source
research alone cannot qualify NVIDIA execution or Tensor Core use. The
[CUDA execution design](../design/cuda-reusable-execution-2026-09-27.md) records
the inspected runtime/binding behavior and the prepared-execution/capture gates.

## BF16 multiplication correction

A separate probe found a pre-existing error shared by eager and compiled low
Multiply. Decoding BF16 `0x0001` and `0x7f7f` gives an exact product of
`0.0311279296875`; replacing the first operand with `0x007f` gives
`3.9532470703125`. Both products are normal f32 values. Native multiplication
flushed the tiny input and returned zero in either operand order.

The MLX binary kernel now uses the same protected product as its direct low
matrix kernels: move an exact power of two between the operands before
multiplication. The WGSL arithmetic path receives the same correction. The
shared contract requires preservation of finite normal f32 products even with
a BF16 subnormal input; genuinely subnormal arithmetic results retain the
existing f32 underflow limits. The retained
[original diagnostic](../../crates/compute-mlx/qualification/compiled-typed-tiny-multiply.txt)
and [corrected diagnostic](../../crates/compute-mlx/qualification/compiled-typed-tiny-multiply-corrected.txt)
record eager/compiled low bits. The shared suite adds 32 exact cases and an
independent host oracle. The WGSL replay test covers all 127 positive BF16
subnormal mantissas, both signs and operand orders, a changing scalar, odd
packed offsets and untouched neighboring halfwords. Other arithmetic kernels
and the CUDA source are unchanged by this correction.

## Matched typed replay measurements

The [benchmark and raw samples](../../crates/compute-mlx/benchmarks/compiled-typed.md)
compare identical eager and compiled operations over changing resident values
and strides. Both paths construct fresh outputs and include evaluation, two
complete readbacks and synchronization in host wall time. Setup, first tracing,
reference checks and final output destruction are outside timing.

| Workload | Eager/compiled median, run 1 | Run 2 |
| --- | ---: | ---: |
| F16 arithmetic and sum | 1.054x | 1.068x |
| BF16 arithmetic and sum | 1.046x | 1.067x |
| F16 direct f32 matmul and sum | 1.036x | 1.015x |
| BF16 direct f32 matmul and sum | 1.039x | 1.024x |
| Small u32 compare/select/sum | 1.114x | 1.021x |
| Large u32 compare/select/sum | 1.125x | 1.073x |

All 744 measured invocations and 1,488 result buffers passed exact references;
all 12 programs retained one trace. The six compiled medians are lower in both
runs, but the first run's small-u32 p90 is worse (0.287667 vs 0.261750 ms),
and BF16 arithmetic p90 is slightly worse (0.905792 vs 0.905292 ms). BF16
arithmetic has substantial within-run and between-run timing variation in both
paths. These measurements establish modest workload-specific host-time changes;
they do not identify GPU kernel time, a fusion mechanism or broad throughput
improvement. The same binary and 106 source files are retained for both runs.
An [independent raw-log audit](tensor-mlx-typed-programs-2026-09-27/independent-benchmark-audit.json)
recomputed all 24 median/p90 pairs, ratios, completion counts and trace checks.

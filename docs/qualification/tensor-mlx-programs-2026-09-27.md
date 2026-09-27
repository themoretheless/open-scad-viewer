# Reusable resident MLX programs

Date: 2026-09-27. This phase follows the
[direct low-input matrix work](tensor-low-matmul-2026-09-27.md) and adds reusable
execution to the MLX adapter. The complete native MLX suite passes 84 top-level
test entries on Apple M4 Max with MLX 0.32.1 / MLX-C 0.6.0_4. The full
WGSL/CUDA/tensor/MLX goal remains active.

## Execution contract

A program declares fixed f32 input shapes and composes existing operations:
unary/binary arithmetic, axis sums, reshape, permutation, broadcast and f32
matrix products. The builder checks shapes through `tensor-core`, including
scalar, empty and batched/vector matrix rules. Values belong to one builder;
using a value from another builder is rejected.

Compilation selects ordered output values. Each run supplies new resident
input handles and returns new lazy result handles. Input count, shape, dtype
and runtime owner are checked before native tracing. Programs retain their
runtime and stream. Previous results remain immutable after subsequent runs,
and results retain the inputs needed for later evaluation.

This interface reuses native graph compilation. It does not expose mutable
input buffers, promise the reuse of particular GPU allocations or add CUDA
graph replay. Low-storage operations and the other extended tensor traits
remain available through their existing APIs; recording them in compiled
programs remains further work.

The [public API example](../../crates/compute-mlx/README.md#compiled-resident-programs)
uses `MlxBackend::program`, `MlxProgramBuilder::compile`, and
`MlxCompiledProgram::run`. Canonical operation dispatch and native shape/dtype
validation are shared with the eager path. The compiled layer remains an MLX
facility; it adds no required methods to the portable tensor traits.

Native fusion can change f32 rounding or accumulation order. Exact agreement
on the dyadic qualification workload is not a promise of bitwise agreement
with eager evaluation for arbitrary f32 inputs. The existing operation domains
and numerical tolerances still apply.

## Native boundary

The optional compile symbol group is separate from ordinary MLX tensor support.
Missing compiler symbols must leave the existing backend available and make
program compilation fail explicitly. The adapter uses public `mlx_compile`
with fixed shapes and preserves the process's compile mode and default device.

`Api::call` serializes native calls through a non-reentrant mutex. The tracing
callback constructs its graph with raw C operations while that guard is held.
It must not call ordinary backend methods or drop an `Array`/`Context` wrapper
inside the callback. Temporary native arrays require local cleanup on success,
native error and caught Rust panic. Payload destruction must also avoid native
wrapper drops that acquire the same mutex.

MLX-C owns the payload after closure construction. Its callback input/result
vectors belong to the native wrapper; setting the result vector copies array
references. The [pinned ABI research](tensor-mlx-programs-2026-09-27/abi-research.json)
records inspected headers and source hashes.
[MLX-C closure ownership](https://github.com/ml-explore/mlx-c/blob/v0.6.0/mlx/c/closure.cpp).

The [initial native failure](../../crates/compute-mlx/qualification/compiled-initial-failure.txt)
caught an incorrect constructor check: `array_new` and `closure_new` return
empty destinations, whose handles are populated by subsequent setters. The
fix validates the populated result. That first attempt stopped in unit tests,
before integration tests ran. Final callback tests cover both caught Rust
panic/local cleanup and an actual C callback error followed by a valid call.

## Proving replay

Native compilation is lazy. A trace counter records actual callback invocations;
successful `mlx_compile` alone does not establish reuse. MLX can disable
compilation through process configuration. Tests must distinguish that state
from the ordinary cached path.

The installed version's compiler substitutes current input arrays into the
cached graph and creates fresh outputs. Its cache compares input shape/dtype
and the default stream/device; input strides are not part of that signature.
Qualification therefore needs changed values and same-shape contiguous,
transposed and broadcast inputs. See the
[pinned compiler implementation](https://github.com/ml-explore/mlx/blob/v0.32.1/mlx/compile.cpp).

The benchmark evaluates `y = square(x * weights + bias)` and `sum(y)`.
Both eager and compiled paths receive the same alternating resident input sets,
construct fresh output handles and read both outputs. Re-evaluating an already
completed result is excluded. Host timing includes graph application,
evaluation, full readback and synchronization; first-use compilation and host
validation are outside the timer.

## Native regression

The [complete native log](../../crates/compute-mlx/qualification/compiled-metal.txt)
records **5 unit tests and 79 integration entries**, with no failures. Ten of
the integration entries belong to the new compiled API. One is a marker-only
entry in the parent process; the disabled-mode parent test launches it in a
separate process for three actual executions and checks three traces. It must
not be counted twice when summing the main suite.

The [focused log](../../crates/compute-mlx/qualification/compiled-focused.txt)
also prints that child execution. Enabled replay checks one trace despite
changed values and strides. Coverage includes:

- GPU-produced replacement inputs, duplicate/input outputs, and unchanged old
  results when later runs are evaluated first.
- Dense, transposed and zero-stride broadcast inputs with identical shapes;
  reshape preserving logical order after permutation.
- Vector/batched matmul, zero contractions, empty batches, scalar/empty sums,
  empty output lists and zero-input programs.
- Foreign graph values and runtime owners, input count/dtype/shape mismatch,
  invalid axes/broadcast/matmul/reshape, and oversized inputs before insertion.
- Results readable after their program/input handles are dropped, and a
  program callable after its constructor backend handle is dropped.

The [native source manifest](../../crates/compute-mlx/qualification/compiled-source-manifest.json)
records all 89 files in its immutable source archive, final raw-log hashes,
commands and exact test counts. [Strict all-target Clippy](../../crates/compute-mlx/qualification/compiled-clippy.txt)
and [rustdoc](../../crates/compute-mlx/qualification/compiled-rustdoc.txt) pass.
Cargo still prints the pre-existing `raster-core` manifest warning about
`wgsl_export` naming; these logs are not warning-free build output.

## Matched measurements

The [benchmark method and raw runs](../../crates/compute-mlx/benchmarks/compiled.md)
cover small, large and changing-layout graphs. Every invocation creates fresh
results and verifies both outputs against independently addressed f64 values.
The selected inputs and all partial sums are exactly representable in f32.
Each run records 186 timed invocations, each with two fully checked outputs.
Both completed runs therefore cover **372 timed invocations and 744 output
buffers**, with zero validation failures. Each of the six programs starts with
zero traces and finishes with one, including the changing-layout case.

Host wall medians in milliseconds; the ratio is eager time / compiled time:

| Case | Eager, run 1 | Compiled, run 1 | Ratio | Eager, run 2 | Compiled, run 2 | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Small `[257]` | 0.182667 | 0.179291 | 1.019× | 0.206292 | 0.203666 | 1.013× |
| Large `[512,512]` | 0.279541 | 0.259208 | 1.078× | 0.248917 | 0.231666 | 1.074× |
| Changing layout `[513,257]` | 0.213792 | 0.210250 | 1.017× | 0.227291 | 0.210125 | 1.082× |

Compiled medians are lower for all three cases in both runs. The observed
difference is small, particularly for the small case; its second-run compiled
p90 is worse, 0.218042 ms versus 0.209750 ms eager. These two runs establish
correct replay and a bounded timing observation, without a general speedup
claim. GPU timestamps, allocator peaks and individual fusion decisions were
not measured. Timed eager graph construction includes dropping its temporary
intermediate handles; final result-handle destruction is outside both timers.

The [raw-sample audit](../../crates/compute-mlx/benchmarks/compiled-audit.json)
recomputes all 12 median/p90 pairs from 372 samples. The
[benchmark fingerprints](../../crates/compute-mlx/benchmarks/compiled-source-fingerprints.json)
bind both runs to 89 unchanged source files and the same release binary.

## Final checks and unchanged backends

[Final checks](tensor-mlx-programs-2026-09-27/final-checks.json) pass for the
MLX WASM boundary, architecture rules, 159 changed/new Rust files under
`rustfmt --check`, and tracked whitespace. Five pre-existing compute-core
formatting exclusions are listed explicitly; archived qualification sources
are excluded from formatting. MLX Clippy and rustdoc evidence is linked above.

The previous [low-matmul qualification](tensor-low-matmul-2026-09-27.md) records
285 WGSL stack tests, 39 CPU contract tests and CUDA host/compiler checks.
Current source hashes confirm that tensor-core, WGSL and CUDA sources are
unchanged in this phase, so those GPU suites were not repeated. The native
MLX suite was repeated in full because the optional FFI group, dispatch
mapping and result validation changed. The
[source fingerprints](tensor-mlx-programs-2026-09-27/source-fingerprints.json)
cover 316 current files; the
[artifact audit](tensor-mlx-programs-2026-09-27/artifact-audit.json)
verifies the 89-file native archive, logs, benchmark binary, raw samples and
derived summaries. The only code changes since the previous snapshot are two
existing MLX files and four new MLX implementation/test/benchmark files.

## Scope still open

This phase qualifies fixed-shape f32 program replay on the installed Apple
runtime. Low/u32 compiled graphs, the other extended operations, explicit
allocation reuse, dynamic shapes, CUDA graph replay and domain integration
remain open. NVIDIA execution and profiler evidence of actual Tensor Core use
are still required. Existing CUDA compiler evidence establishes source
compilation only. Hardware CI and deployment packaging also remain unqualified.

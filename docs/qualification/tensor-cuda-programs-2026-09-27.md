# Prepared CUDA programs: host qualification

Date: 2026-09-27. Prepared f32 execution is implemented and passes host tests.
**NVIDIA execution remains unverified.** The required-device test fails on this
Apple host because the CUDA driver/device is unavailable. No CUDA timing,
CUDA Graph replay or Tensor Core instruction use is claimed.

## Implemented behavior

`CudaRuntime::program()` creates a fixed-layout builder. It records all nine
unary and six binary f32 operations, affine, materialize, reshape, permutation,
broadcast, reductions, mean and vector/batched matmul with explicit precision.
`prepare(outputs, CudaPrepareOptions)` computes the complete schedule and checks
scratch/metadata budgets before creating device resources. Programs borrow the
runtime and retain scratch, uploaded metadata and prepared GEMM descriptors.

`run_into` binds current input allocations and writes caller-owned output
allocations. Input shape, strides and offset must match the prepared signature;
outputs must be dense, offset-zero, uniquely owned and independent of all input
and output allocations. All bindings and requested precision policies are
validated before the first enqueue. Read-only input aliases are allowed.
`run` checks inputs and policies, allocates fresh outputs, then uses the same
executor. Duplicate requested outputs and terminal input/view nodes produce
independent copies, so later executions preserve previously returned results.

Replay requests no adapter device tensor allocation, metadata upload, NVRTC
compilation or cuBLAS initialization. It still submits individual kernel and
cuBLAS calls and creates host argument bookkeeping. cuBLAS internal allocations
are outside this guarantee. `stats()` reports planned kernel launches, GEMM
calls, scratch bytes and metadata bytes, including empty allocation sentinels;
it does not measure peak memory or cuBLAS's internal kernels.

Empty sum/product and K=0 matrix results write their identities every replay.
Validation failures permit a corrected retry. An enqueue error or an error
observed through `program.synchronize()` permanently poisons that program;
partially written results are invalid and GPU work is not rolled back.

The [README example](../../crates/compute-cuda/README.md#prepared-resident-programs)
is compiled as a `no_run` doctest. The
[execution design](../design/cuda-reusable-execution-2026-09-27.md) distinguishes
this implemented path from the proposed fixed-address CUDA Graph mode.

## Shared implementation

- [Planner](../../crates/compute-cuda/src/program/builder.rs): shared tensor
  shape/layout rules, checked builder identities, aliases and transaction rollback.
- [Preparation](../../crates/compute-cuda/src/program/preparation.rs): full
  reduction expansion, output copies, precision checks and exact adapter budgets.
- [Elementwise dispatch](../../crates/compute-cuda/src/dispatch.rs) and
  [reduction passes](../../crates/compute-cuda/src/reduction/dispatch.rs): one
  checked parameter representation and launch ABI for eager and prepared paths.
- [GEMM geometry](../../crates/compute-cuda/src/matmul/plan.rs) and
  [shared launch](../../crates/compute-cuda/src/matmul.rs): row-major mapping,
  vector promotion, storage spans and broadcast batches. Batch geometry uses
  O(rank) host storage, without allocating one descriptor per batch. The native
  low-u16 eager route uses the same cuBLAS launch helper and retains its dtype.
- [Execution gate](../../crates/compute-cuda/src/program/gate.rs) and
  [executor](../../crates/compute-cuda/src/program/execution.rs): complete
  preflight before enqueue, retained resources and permanent poison on failure.

Scratch destinations temporarily move out of their pool so typed source and
output borrows remain disjoint. They are restored before propagating a launch
error. Launches retain cudarc's normal read/write event guards, including around
each cuBLAS call; no raw-pointer shortcut bypasses those dependencies. Source
review supports this ownership design; device lifetime/race qualification still
requires NVIDIA execution and Compute Sanitizer.

Review found that a frozen GEMM mode alone could bypass a later
`NVIDIA_TF32_OVERRIDE=0`. Prepared state now retains every requested matmul mode,
including empty/K=0 nodes, and revalidates the current policy before allocating
`run` outputs or entering the `run_into` schedule. CPU tests exercise the same
validation/execute gate as production.

## Evidence and limits

| Gate | Current result | Evidence |
| --- | --- | --- |
| CUDA host unit tests | 47 passed | [Host log](tensor-cuda-programs-2026-09-27/cuda-host.txt) |
| Public usage examples | 2 `no_run` doctests compiled | Same host log |
| Optional native entries | Both existing tensor and new prepared entries explicitly SKIP | Same host log; these are not GPU passes |
| Required native prepared execution | Failed: CUDA driver/device unavailable | [Required-device log](tensor-cuda-programs-2026-09-27/cuda-required.txt) |
| Strict Clippy, all targets | tensor-core, compute-core, compute-cuda, compute-mlx passed | [Clippy](tensor-cuda-programs-2026-09-27/clippy.txt) |
| Strict rustdoc | Same four crates passed | [Rustdoc](tensor-cuda-programs-2026-09-27/rustdoc.txt) |
| Architecture, format and whitespace | Passed; 188 changed/new Rust source files checked | [Commands and exclusions](tensor-cuda-programs-2026-09-27/final-checks.json) |

CPU tests cover all recorded operation variants, stride/offset/broadcast
propagation, dense materialization, late-error rollback, foreign builder values,
byte and BLAS-dimension overflow, hierarchy boundaries, storage spans and GEMM
batch offsets. Budget tests include scalar metadata sentinels, duplicate
terminal copies, extra reduction partials and mean division. The execution-gate
mock verifies that a later bad output or rejected precision performs zero
enqueues, validation errors remain reusable, and enqueue failure prevents later
execution. These mocks do not establish CUDA arithmetic or runtime safety.

The [native fixture](../../crates/compute-cuda/tests/prepared.rs) is ready for
mandatory-device execution. It checks every unary/binary operation, transformed
views, changing values and allocations, repeated `run_into`, previous output
lifetime, partial/full reductions, a 131,077-element hierarchy, empty identities,
K=0 writes, batched strided and offset-vector GEMM, every permitted precision,
foreign owners, aliases, output sentinels and retry after validation errors.

The [source manifest](tensor-cuda-programs-2026-09-27/source-fingerprints.json)
records 346 backend source files; the
[exact archive](tensor-cuda-programs-2026-09-27/source-archive.json) retains 91
CUDA, shared-contract and platform/dependency files. The
[artifact audit](tensor-cuda-programs-2026-09-27/artifact-audit.json) verifies
current/archive hashes, log counts and local evidence links.

All 12 CUDA kernel source parts are byte-identical to the existing NVRTC
12.8.93 snapshot: 48,291 bytes, combined SHA256
`d8a6cb52807f369f03f23bc5f01f226027bbbed1e6cee38bb4859b5d21699327`.
The [reuse check](tensor-cuda-programs-2026-09-27/kernel-source-reuse.json)
links its compiler evidence for 51 entry points on compute_70/80/90/120.
Source compilation was not repeated for unchanged kernels; new host launch
behavior requires native qualification. WGSL, MLX and shared tensor sources are
unchanged from the [previous phase](tensor-mlx-typed-programs-2026-09-27.md),
so their 286 WGSL, 104 MLX and 40 CPU results remain historical evidence rather
than newly executed tests. Cargo still prints the pre-existing `raster-core`
non-kebab-case binary-name manifest warning.

## Remaining gates

1. Run both CUDA native fixtures with `COMPUTE_REQUIRE_CUDA=1` on NVIDIA;
   preserve device/driver/cuBLAS/NVRTC versions and run Compute Sanitizer.
2. Measure eager versus prepared replay with changing resident inputs, equal
   readback/synchronization boundaries, raw wall/event samples and independent
   references. There is no prepared CUDA speedup claim yet.
3. Implement and qualify explicit-stream CUDA Graph capture, resource/error
   cleanup and fixed-address input/output slots. Prepared execution is the
   shared launch layer for that work.
4. Extend prepared signatures to u32/f16/BF16 and add indexing, statistics and
   attention. Their existing eager operations remain available.
5. Profile actual Tensor Core instructions for each permitted matmul mode.
   Capability, policy, compilation and cuBLAS invocation alone do not prove use.

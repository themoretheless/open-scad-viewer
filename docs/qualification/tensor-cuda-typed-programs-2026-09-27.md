# Typed prepared CUDA programs: host and source qualification

Date: 2026-09-27. Prepared CUDA programs now support **f32, u32, f16 and BF16**.
Host checks and actual NVRTC compilation passed. **NVIDIA execution remains
unverified:** this Apple host has no CUDA driver/device. There is no CUDA
performance, CUDA Graph replay or Tensor Core instruction-use claim.

## Implemented behavior

`CudaRuntime::program()` records fixed dtype/layout signatures. New typed
inputs, arithmetic, comparisons, selection, reductions, low casts and low GEMM
use resident typed storage throughout the schedule. `run_typed` returns owned
typed outputs; `run_typed_into` reuses caller-owned outputs. The existing f32
`run`/`run_into` API also accepts programs with internal typed nodes when their
complete input/output signature is f32.

| Dtype/path | Behavior |
| --- | --- |
| u32 arithmetic | Wrapping add/subtract/multiply and unsigned min/max; division is rejected before execution |
| u32 reductions | Sum/product wrap modulo 2³²; min/max retain unsigned comparison at every hierarchy level |
| f16/BF16 arithmetic | Native u16 inputs and intermediates; each unary/binary node rounds its own result once |
| Low reductions and mean | Decode directly in the first reduction pass; all partials and final result are f32; optional final low cast |
| Low matrix products | Native16 cuBLAS inputs and f32 accumulation/output; optional final low cast; vector and broadcast batch support |
| Views and selection | Preserve dtype and raw payloads, including NaNs and signed zeros; strided materialization uses the original storage format |
| Comparisons and casts | All six comparisons produce u32 masks; explicit f32 ↔ f16/BF16 conversion nodes |

Preparation accounts for actual two/four-byte scratch, hierarchical partials,
mean intermediates and metadata, including empty/scalar allocation sentinels.
It validates budgets and every matmul request before allocating device resources.
Replay checks every owner, dtype, layout, span, output uniqueness and alias,
plus current precision policy, before the first enqueue. Empty and K=0 matmul
nodes retain these policy checks. Validation failures allow retry; enqueue or
observed synchronization failures poison the program.

Replay requests no adapter device tensor allocation or metadata upload. It
continues to submit ordinary kernel/cuBLAS calls; cuBLAS internal allocations
are outside that guarantee. Scratch pooling, operation fusion and CUDA Graph
capture are not implemented. Scan, gather, compaction, scatter, statistics and
attention remain available through the eager API and are not recorded by this
prepared builder yet.

## Shared implementation

- [Typed planner](../../crates/compute-cuda/src/program/builder_typed.rs) uses
  shared tensor shape rules and transactional recording for compound nodes.
- [Typed transport](../../crates/compute-cuda/src/program/typed.rs),
  [storage](../../crates/compute-cuda/src/program/storage.rs) and
  [validation](../../crates/compute-cuda/src/program/validation.rs) preserve
  concrete f32/u32/u16 ownership with an explicit low dtype.
- [Expansion](../../crates/compute-cuda/src/program/expansion.rs),
  [reduction lowering](../../crates/compute-cuda/src/program/reduction_plan.rs)
  and [executor](../../crates/compute-cuda/src/program/launch.rs) retain checked
  launch descriptions and cudarc's normal read/write event guards.
- [Typed movement/comparison](../../crates/compute-cuda/src/indexing/dispatch.rs),
  [elementwise dispatch](../../crates/compute-cuda/src/dispatch.rs),
  [low casts/arithmetic](../../crates/compute-cuda/src/low_dispatch.rs) and
  [GEMM](../../crates/compute-cuda/src/matmul.rs) serve both eager and prepared
  calls. There is one launch implementation per operation family.

The [README](../../crates/compute-cuda/README.md#prepared-resident-programs)
includes a mixed f32/u32/BF16 replay example. The
[execution design](../design/cuda-reusable-execution-2026-09-27.md) describes
resource ownership and the remaining CUDA Graph work.

## Verification

| Gate | Result | Evidence |
| --- | --- | --- |
| CUDA host unit tests | 77 passed | [Host log](tensor-cuda-typed-programs-2026-09-27/cuda-host.txt) |
| Usage examples | 3 `no_run` doctests compiled after the documentation update | [Doctests](tensor-cuda-typed-programs-2026-09-27/doctests.txt) |
| Optional native entries | Eager, f32 prepared and typed prepared entries explicitly SKIP | Host log; these are not GPU passes |
| Required native typed execution | Exit 101: CUDA driver/device unavailable | [Required-device log](tensor-cuda-typed-programs-2026-09-27/cuda-required.txt) |
| Strict Clippy, all targets | tensor-core, compute-core, compute-cuda and compute-mlx passed | [Clippy](tensor-cuda-typed-programs-2026-09-27/clippy.txt) |
| Strict rustdoc | All four crates passed; updated CUDA documentation rechecked | [Initial check](tensor-cuda-typed-programs-2026-09-27/rustdoc.txt), [updated docs](tensor-cuda-typed-programs-2026-09-27/rustdoc-updated.txt) |
| Format, architecture, whitespace | Passed; 205 changed/new Rust files checked | [Commands and exclusions](tensor-cuda-typed-programs-2026-09-27/final-checks.json) |
| NVRTC 12.8.93 | All 52 entries and ordered parameter widths passed for compute_70/80/90/120 | [Compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-typed-programs/report.json) |

Host tests cover typed byte limits, physical spans, scalar/empty sentinels,
low-to-f32 widening limits, transaction rollback, exact scratch budgets,
unsigned hierarchy, low-to-f32 partials, native16 strided GEMM buffers and
retention of empty/K=0 policy checks. The production execution gate is tested
with a late output dtype failure: no enqueue occurs, and a corrected retry
remains valid. Separate tests verify permanent poison after an enqueue error.

The [native typed fixture](../../crates/compute-cuda/tests/prepared_typed.rs)
is compiled and statically reviewed, but has not executed on NVIDIA. It covers:

- unsigned overflow, offset/transposed inputs, changed scalar values, repeated
  execution, duplicate independent outputs and 8,197-element reductions;
- all 65,536 raw patterns of each low format through views, casts, comparisons
  and selection, including signed zero and NaN payload copies;
- all low unary/binary operations against the eager route, halfway rounding at
  each node, direct f32 sums/means and repeatable empty identities;
- native low vector GEMM with a direct f32 result, optional final low rounding,
  K=0 writes and rejection on unsupported hardware;
- mixed bindings, foreign runtimes, late wrong-dtype/shared outputs, unchanged
  output sentinels on validation failure, retry and legacy f32 signatures.

These cases are pending numerical/runtime qualification. Bit comparisons
between eager and prepared arithmetic test agreement of the two paths;
independent expected values are also used for wrapping arithmetic, raw casts,
rounding boundaries, reductions and selected matrix results.

## Compiler and source snapshot

NVRTC compiled the twelve production CUDA parts in a disposable Linux aarch64
container from the existing local image. No GPU was requested. Compilation
used `ftz=false`, `fmad=false`, precise divide and square root. The new
`binary_u32` entry has the same layout argument ABI as f32 binary arithmetic;
all 51 previous entry signatures are preserved. External PTX modules now need
to export this new entry too.

Combined source: **49,262 bytes**, SHA256
`d60ab5d94f748e432c584b0ec88eb2d0cdb54ba9ca47a25aad572f06c9216eda`.
The [compiler summary](../../crates/compute-cuda/qualification/README.md#current-typed-program-snapshot-2026-09-27)
links complete logs and retained PTX fingerprints. This verifies CUDA syntax,
instantiation and emitted ABI, not pointer validity or numerical execution.

The [source manifest](tensor-cuda-typed-programs-2026-09-27/source-fingerprints.json)
records 366 files. The [archive](tensor-cuda-typed-programs-2026-09-27/source-archive.json)
retains 111 exact CUDA, contract, dependency and qualification files. The
[artifact audit](tensor-cuda-typed-programs-2026-09-27/artifact-audit.json) verifies
live/archive hashes, compiler source and PTX hashes, previous ABI compatibility,
log counts and local evidence links.

WGSL, MLX and shared tensor sources remain unchanged from the preceding
[CUDA f32 prepared phase](tensor-cuda-programs-2026-09-27.md). Their earlier GPU
results were not rerun for these CUDA-only changes. Cargo continues to print
the existing `raster-core` binary-name manifest warning.

## Remaining work

1. Run all three native CUDA fixtures with `COMPUTE_REQUIRE_CUDA=1` on NVIDIA,
   preserving device/driver/compiler/cuBLAS versions and Compute Sanitizer logs.
2. Extend prepared recording to scan/gather/compaction/scatter, statistics and
   attention while sharing the eager launch paths.
3. Implement explicit-stream CUDA Graph capture with owned fixed-address slots
   and qualify capture failure, replay, lifetime and cleanup behavior.
4. Measure resident eager/prepared/graph execution with equal synchronization
   boundaries; profile actual Tensor Core instructions for permitted GEMM modes.

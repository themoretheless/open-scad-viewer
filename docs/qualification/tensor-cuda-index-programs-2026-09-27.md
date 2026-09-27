# Prepared CUDA indexing: host qualification

Date: 2026-09-27. Prepared CUDA programs now record scan, gather, stable
compaction and scatter for **f32, u32, f16 and BF16**. Host checks passed;
**NVIDIA execution remains unverified** because this host has no CUDA device.
This report makes no CUDA performance or Tensor Core instruction-use claim.

## Implemented behavior

| Operation | Prepared behavior |
| --- | --- |
| Scan | Arbitrary axes, inclusive/exclusive and forward/reverse; retained multilevel totals and carries; u32 wraps; low inputs accumulate directly in f32 with optional final low cast |
| Gather | Replaces the selected axis with the index shape; invalid reads write zero; resident scalar invalid count |
| Compaction | Stable logical order, fixed capacity, zero tail and resident scalar selected count; raw low payloads preserved |
| Scatter | Replace/Add/Multiply/Min/Max, broadcast updates and resident invalid count; Replace uses the last logical index on duplicate destinations |
| Low scatter | Raw Replace/Min/Max; Add/Multiply load low updates directly into an f32 result and optionally round once at the end; all modes also offer f32 output |

Counts are ordinary `CudaValue` nodes and can feed subsequent operations
without readback. Invalid indices are counted once per logical index even when
another dimension makes the values result empty. Caller output validation,
precision checks, ownership rules and poisoning remain shared with the
[typed prepared API](tensor-cuda-typed-programs-2026-09-27.md).

Replay explicitly resets counters, compaction capacity/count and Replace owner
tables, and initializes each scatter destination from its current base. This
prevents old values from surviving when indices or masks change. Explicit
asynchronous clears use cudarc's normal write guards and are reported as
`stats().memset_calls`, separately from kernel and GEMM calls.

Raw low scatter reserves complete aligned u32 CAS words in private scratch,
including an odd final u16 element. Logical shape and terminal outputs remain
unchanged. The checked scratch budget includes this physical padding, every
scan level and every auxiliary count/owner allocation before device resources
are created.

Scan and compaction write two destinations in one kernel. The executor moves
both allocations out of the scratch pool for disjoint source borrows and
restores both before propagating a launch error. Source buffers remain borrowed
through the normal cudarc argument guards. No caller allocation is retained.

## Shared code and examples

Eager and prepared calls share the checked
[scan](../../crates/compute-cuda/src/indexing/scan_dispatch.rs),
[gather/count](../../crates/compute-cuda/src/indexing/gather_dispatch.rs),
[compact](../../crates/compute-cuda/src/indexing/compact_dispatch.rs) and
[scatter](../../crates/compute-cuda/src/indexing/scatter_dispatch.rs) launch
implementations. The [planner](../../crates/compute-cuda/src/program/builder_indexing.rs)
records paired data/count results transactionally. The
[expander](../../crates/compute-cuda/src/program/indexing_plan.rs) creates
resident passes and resets; the
[executor](../../crates/compute-cuda/src/program/indexing_launch.rs) selects
concrete typed kernel ABIs. The
[scratch helper](../../crates/compute-cuda/src/program/scratch.rs) handles both
destinations and the Result-error restoration path.

The [README pipeline](../../crates/compute-cuda/README.md#resident-indexing-and-count-composition)
records gather → scan → compact → count-indexed gather/scatter, changes its
mask and reuses outputs. It compiles as a `no_run` doctest; it has not executed
on this host.

## Evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| CUDA host unit tests | 116 passed | [Host log](tensor-cuda-index-programs-2026-09-27/cuda-host.txt) |
| Public examples | 4 `no_run` doctests compiled | Same host log |
| Native entries | All four explicitly SKIP without a CUDA device | Same host log; these are not GPU passes |
| Required prepared indexing execution | Exit 101: CUDA driver/device unavailable | [Required log](tensor-cuda-index-programs-2026-09-27/cuda-required.txt) |
| Strict Clippy, all targets | tensor-core, compute-core, compute-cuda, compute-mlx passed | [Clippy](tensor-cuda-index-programs-2026-09-27/clippy.txt) |
| Strict rustdoc | Same four crates passed | [Rustdoc](tensor-cuda-index-programs-2026-09-27/rustdoc.txt) |
| Architecture, format and whitespace | Passed | [Commands](tensor-cuda-index-programs-2026-09-27/final-checks.json) |

New CPU tests cover all four dtypes, count-node composition, stride and offset
metadata, scan hierarchy and carry addressing, zero target axes, empty results,
paired-result rollback and byte overflow. Expanded-schedule tests check exact
padding/partial-buffer budgets, required resets, native-low-to-f32 transitions
and empty-index behavior. Scratch tests verify that an enqueue error restores
both destinations and invalid destinations cannot call the launcher.

The [native fixture](../../crates/compute-cuda/tests/prepared_indexing.rs)
compiles and awaits NVIDIA execution. It covers changing indices/masks, reused
outputs, independent prior outputs, resident count consumers, zero tails,
unsigned overflow, multilevel/strided scans, low accumulation and rounding,
raw payload movement, duplicate-index scatter ownership, odd u16 CAS tails,
empty outputs and validation failures before writes. Exact CPU expectations
and eager parity checks have distinct roles; neither is a native pass here.

## Source snapshot and limits

No CUDA kernel source or ABI changed. The
[reuse audit](tensor-cuda-index-programs-2026-09-27/kernel-source-reuse.json)
verifies all twelve current source parts against the existing
[NVRTC report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-typed-programs/report.json):
52 entries compiled for compute_70/80/90/120, 49,262 combined bytes, SHA256
`d60ab5d94f748e432c584b0ec88eb2d0cdb54ba9ca47a25aad572f06c9216eda`.
The host launch refactor still needs NVIDIA numerical and memory-safety checks.

The [source manifest](tensor-cuda-index-programs-2026-09-27/source-fingerprints.json),
[exact archive](tensor-cuda-index-programs-2026-09-27/source-archive.json) and
[artifact audit](tensor-cuda-index-programs-2026-09-27/artifact-audit.json) preserve
and verify source hashes, command logs and local evidence links. WGSL, MLX and
shared tensor sources are unchanged from the previous phase, so their historical
GPU results were not rerun for this CUDA-only change.

Statistics and attention still use the eager CUDA API. Remaining work includes
prepared recording for those operations, explicit-stream CUDA Graph capture,
NVIDIA/Compute Sanitizer execution, resident replay benchmarks and actual
Tensor Core instruction profiling. The broader backend goal remains open.

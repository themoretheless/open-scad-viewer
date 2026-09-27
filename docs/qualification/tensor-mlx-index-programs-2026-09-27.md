# Compiled MLX indexing, scans and scatter

Date: 2026-09-27. MLX programs now record compare/select, gather, axis scan,
stable fixed-capacity compaction and all five scatter operations for f32, u32,
f16 and BF16. Values and scalar counts compose within the resident graph.

## Implementation

Eager calls and compiled programs share the same checked
[indexing](../../crates/compute-mlx/src/native/lowering/indexing.rs),
[scan](../../crates/compute-mlx/src/native/lowering/scan.rs),
[compaction](../../crates/compute-mlx/src/native/lowering/compaction.rs) and
[scatter](../../crates/compute-mlx/src/native/lowering/scatter.rs) recipes.
The old eager implementations and their obsolete helper methods were removed.
Native emission remains inside the existing call lock, with raw array guards
and constants/kernel owners retained by the compiled program.

Gather returns zero for invalid indices and counts each logical index once,
including when another output dimension is empty. Compaction fills selected
values in logical order and zeros the remaining capacity; its count stays on
the GPU. Unsigned prefixes wrap modulo 2^32. Low prefixes accumulate in f32 and
optionally round once to the source dtype. Low movement preserves raw payloads.

Replace elects the last valid logical index deterministically. Low arithmetic
scatter uses the existing custom Metal list/fold kernels; their atomic list
initialization is part of each replayed primitive. Changing valid indices to
invalid indices cannot reuse earlier owners, links or counts. Compound builder
operations roll back both returned tokens and all intermediate nodes on error.

The [public example](../../crates/compute-mlx/examples/compiled_indexing.rs)
records gather → scan → compact → scatter, merges invalid counts on the GPU,
and replays with different values, masks and indices.

## Qualification

The checks use the actual Apple M4 Max Metal device, MLX 0.32.1 and MLX-C 0.6 ABI.
Exact commands, environment overrides and exit codes are retained in
[checks.json](tensor-mlx-index-programs-2026-09-27/checks.json).

| Check | Result | Evidence |
| --- | --- | --- |
| Full MLX regression | 138 top-level tests passed with `COMPUTE_REQUIRE_MLX=1`; no unavailable skips | [Full log](tensor-mlx-index-programs-2026-09-27/mlx-metal.txt) |
| Compilation disabled | All 18 new indexing/scan/compaction/scatter tests passed in a separate process | [Disabled log](tensor-mlx-index-programs-2026-09-27/mlx-disabled.txt) |
| Public resident pipeline | Both runs matched values/counts; enabled tracing occurred once, disabled tracing twice | [Enabled](tensor-mlx-index-programs-2026-09-27/example.txt), [disabled](tensor-mlx-index-programs-2026-09-27/example-disabled.txt) |
| Strict all-target Clippy | tensor-core, compute-core, compute-cuda and compute-mlx passed | [Clippy](tensor-mlx-index-programs-2026-09-27/clippy.txt) |
| Strict rustdoc | compute-mlx passed | [Rustdoc](tensor-mlx-index-programs-2026-09-27/rustdoc.txt) |
| Architecture, changed-file formatting and whitespace | Passed | [Checks](tensor-mlx-index-programs-2026-09-27/checks.json) |

The full total consists of nine unit tests and 129 integration entries, including
one marker-only child entry. The disabled-mode child process is not counted a
second time. All 18 new integration tests execute GPU work. README examples are
explanatory; the standalone indexing example above was built and executed.
Cargo retains the existing unrelated `wgsl_export` manifest naming warning.

New coverage includes all 65,536 low encodings in comparisons and raw movement,
all scan modes around 256-element block boundaries and at 131,077 elements,
unsigned wrapping, arbitrary axes, changed strides and broadcast masks, scalar
and empty inputs, resident counts consumed by later gather/scatter, and all five
scatter modes. Replay changes selection from all to sparse to none and back;
65,539-index scatter contention checks one final low rounding. Tests retain lazy
outputs after dropping their program and source handles. Independent host
references check logical values, exact payloads and counts.

## Source and limits

The [source manifest](tensor-mlx-index-programs-2026-09-27/source-fingerprints.json)
and [artifact audit](tensor-mlx-index-programs-2026-09-27/artifact-audit.json)
record the qualified files and check results. Existing CUDA, WGSL and shared
tensor sources and all MLX Metal sources are unchanged; their historical checks
are not presented as new execution evidence here.

Compiled replay reuses native tracing and produces fresh lazy outputs. It does
not establish fixed GPU allocations, improved performance, broader MLX ABI
support or NVIDIA execution. CUDA Graph capture, NVIDIA conformance and Compute
Sanitizer runs, Tensor Core instruction profiling, domain routing, broader
operators, platform qualification and hardware CI remain separate work.

# Reusable CUDA and MLX statistics and attention

Date: 2026-09-27. Both builders now record softmax, log-softmax, logsumexp,
population moments, layer normalization and attention for f32/f16/BF16.
Low inputs can produce f32 results or one final low cast. MLX passed native
Apple GPU qualification. CUDA numerical execution still requires NVIDIA hardware.

## Implementation

CUDA uses common checked [statistics launch helpers](../../crates/compute-cuda/src/statistics_dispatch.rs)
and [attention launch helpers](../../crates/compute-cuda/src/attention_dispatch.rs)
for eager and prepared execution. Preparation retains f64 row summaries,
bounded partials and an output-sized attention accumulator. These buffers are
private kernel state and count at eight bytes per element in the scratch budget;
they do not add a public f64 tensor type. Replay borrows current inputs and
reuses this storage without uploading metadata or allocating adapter tensors.

Both moments outputs are disjoint destinations. Singleton statistics write their
identities every run; mean/logsumexp preserve source bits or decode low values
directly. Attention with zero keys clears its output every run. Active attention
kernels initialize each accumulator row before use. Existing output validation,
error poisoning, masks, GQA, signed causal alignment and ownership rules remain.

MLX eager and compiled calls share declarative
[statistics](../../crates/compute-mlx/src/native/lowering/statistics.rs),
[low statistics](../../crates/compute-mlx/src/native/lowering/statistics_low.rs)
and [attention](../../crates/compute-mlx/src/native/lowering/attention.rs) recipes.
The native callback emits raw C operations inside the existing lock. Prepared
constants and kernel owners remain alive until the closure is released. Recording
failures roll back the entire operation, including both moments results and casts.
Native compilation controls remain unchanged; disabled compilation is tested in
a separate process. Reusing a trace does not prove fixed GPU allocations or speedup.

## Qualification

Exact commands, environment overrides and exit codes are in
[checks.json](tensor-prepared-statistics-2026-09-27/checks.json).

| Check | Result | Evidence |
| --- | --- | --- |
| CUDA CPU checks | 134 unit tests and one independent low-rounding oracle passed | [Host log](tensor-prepared-statistics-2026-09-27/cuda-host.txt) |
| CUDA public examples | Five `no_run` doctests compiled | Same host log |
| CUDA device tests | Five optional native entries explicitly skipped | Same host log; these are not GPU passes |
| Required CUDA statistics/attention | Exit 101: CUDA driver/device unavailable | [Required log](tensor-prepared-statistics-2026-09-27/cuda-required.txt) |
| MLX full native regression | 118 top-level tests passed with `COMPUTE_REQUIRE_MLX=1`; no unavailable skips | [Full log](tensor-prepared-statistics-2026-09-27/mlx-metal.txt) |
| MLX compilation disabled | All seven new statistics and six attention tests passed | [Disabled log](tensor-prepared-statistics-2026-09-27/mlx-disabled.txt) |
| Strict all-target Clippy | tensor-core, compute-core, compute-cuda and compute-mlx passed | [Clippy](tensor-prepared-statistics-2026-09-27/clippy.txt) |
| Strict rustdoc | compute-cuda and compute-mlx passed | [Rustdoc](tensor-prepared-statistics-2026-09-27/rustdoc.txt) |
| Architecture, changed-file formatting and whitespace | Passed | [Checks](tensor-prepared-statistics-2026-09-27/checks.json) |

The MLX total includes seven unit tests and one marker-only child entry; an
additional child process is not double-counted. The 13 new integration tests
execute statistics/attention on the GPU. MLX README examples are explanatory;
MLX has no README doctest inclusion. CUDA examples are compiled, not GPU-executed.
Cargo emits an existing workspace manifest warning about `wgsl_export` naming;
strict Rust Clippy and rustdoc complete successfully.

Coverage includes changed resident inputs, strided views, unsorted reduction
axes, long partial reductions, singleton subnormals and signed zeros, overflowing
reported variance, subnormal epsilon, final low rounding, GQA, broadcast masks,
fully masked rows, signed causal limits, zero keys and deferred output lifetime.
Independent CPU references accompany native tests. The CUDA fixture compiles
and retains these checks for NVIDIA execution.

## Source and limits

The [source manifest](tensor-prepared-statistics-2026-09-27/source-fingerprints.json)
and [artifact audit](tensor-prepared-statistics-2026-09-27/artifact-audit.json)
record hashes and checked results. CUDA and Metal shader sources did not change.
The existing 52-entry NVRTC report is reused only after comparing all current
CUDA source parts; compiler acceptance does not establish runtime safety.
WGSL and shared tensor code are unchanged and their previous GPU results are
not presented as newly executed results.

Remaining work includes MLX compiled indexing, CUDA Graph capture, NVIDIA
numerical/Compute Sanitizer tests, actual Tensor Core instruction profiling,
matched execution benchmarks, domain routing and hardware CI. No CUDA execution,
Tensor Core use, fixed-allocation MLX replay or performance gain is claimed here.

# CUDA Graph programs: host qualification

Date: 2026-09-27. Prepared CUDA programs now expose `capture_owned`, with
private input/output slots and replay through one instantiated CUDA Graph.
**Host checks passed; NVIDIA execution remains unverified.** This Apple host
has no CUDA device. No performance or Tensor Core instruction-use claim is made.

## Implemented contract

The [graph API](../../crates/compute-cuda/src/program/graph.rs) supports the
prepared program's f32/u32/f16/BF16 operations, including resident indexing
counts, statistics, attention and cuBLAS GEMM. `run_typed_into` checks every
external owner, dtype, layout, storage span, output alias and current precision
policy before enqueueing. Legacy `run`/`run_into` require f32 input/output
signatures. Returned outputs are independent snapshots.

Capture preparation rebuilds the retained logical plan for dense private input
slots. The [remapper](../../crates/compute-cuda/src/program/graph_plan.rs)
preserves permutation, broadcasting, reshape and matrix promotion history.
It inserts a materialization when rebasing makes a reshape or active GEMM
operand noncontiguous. Repeated consumers reuse that copy. Ordinary prepared
execution retains its original plan. External sparse strides never determine
the size of graph input allocations.

Each graph owns a nonblocking stream under a fresh Rust wrapper for the same
primary CUDA context. Slice event tracking is disabled only on this private
wrapper before allocation. Two preallocated events bridge caller-stream input
copies, private graph execution and caller-stream output copies. Public tensor
access guards remain active. Graph slots and eventless private allocations are
never exposed. Custom nonprimary contexts are rejected.

`run_typed_into` performs no adapter device-buffer allocation, metadata upload
or event creation. It still performs input/output copy kernels and event
bridges outside the graph. `run_typed` allocates fresh public outputs. Host
argument bookkeeping and opaque driver/library allocations are outside this
guarantee; the additional copies must be included in future benchmarks.

Preparation budgets cover dense input/output slots, expanded scratch, all
adapter metadata, empty sentinels, low CAS padding and the aligned cuBLAS
workspace. `CudaGraphStats` reports these bytes and scheduled calls, excluding
opaque driver graph, module and library bookkeeping. These are accounting
values, not measured peak memory or native graph node counts.

The graph retains a private cuBLAS handle/workspace and all functions and
allocations referenced by capture. Workspace alignment and usable capacity are
checked before allocation. Stream, host pointer mode and workspace are set
before warmup/capture. The ordering follows NVIDIA's
[cuBLAS graph and workspace contract](https://docs.nvidia.com/cuda/archive/12.8.1/cublas/index.html#cuda-graphs-support).
The public [example](../../crates/compute-cuda/examples/graph.rs) replays square
and sum with changed input values and asserts both results; it compiled here
but could not execute without NVIDIA hardware.

## Error and lifetime handling

The [platform graph layer](../../crates/gpu-compute/src/cuda/graph.rs) preflights
the complete capture/instantiate/upload/launch/destruction ABI group. Additional
stream/event/primary-context and cuBLAS symbols are checked before their first
graph use. Low-level graph ownership and typed tensor execution remain in
their separate crates.

The capture guard ends abandoned or invalidated capture and retains partial
handles for cleanup. Graph handles retire before captured resources. From the
first warmup submission onward, the owning guard checks completion on both
streams before releasing resources. If completion cannot be established, it
retains those resources. Enqueue errors, enqueue panics and observed
synchronization errors poison future replay; input validation failures permit
a corrected retry.

The shared scratch helper now restores moved primary/auxiliary buffers during
unwinding, before the outer capture guard ends capture. The shared execution
gate arms poisoning before entering driver/library code and clears it only on
a normal successful return. These fixes also protect ordinary prepared replay.

## Evidence

Exact commands, environment overrides and exits are in
[checks.json](tensor-cuda-graphs-2026-09-27/checks.json).

| Check | Result | Evidence |
| --- | --- | --- |
| CUDA host unit tests | 151 passed | [Host log](tensor-cuda-graphs-2026-09-27/cuda-host.txt) |
| Platform CUDA/graph tests | 15 passed, including 12 graph lifecycle tests | [Platform log](tensor-cuda-graphs-2026-09-27/platform-host.txt) |
| CPU integration reference | The same independent low codec check passed in two executables | Host log |
| README examples | Six `no_run` doctests compiled | Host log |
| Native CUDA entries | All six explicitly reported unavailable/SKIP; zero GPU passes | Host log |
| Required native graph execution | Exit 101: CUDA driver/device unavailable | [Required log](tensor-cuda-graphs-2026-09-27/cuda-required.txt) |
| Strict all-target Clippy | tensor-core, compute-core, compute-cuda, compute-mlx and gpu-compute passed | [Clippy](tensor-cuda-graphs-2026-09-27/clippy.txt) |
| Strict rustdoc | compute-cuda and gpu-compute passed | [Rustdoc](tensor-cuda-graphs-2026-09-27/rustdoc.txt) |
| Architecture, changed-file formatting and whitespace | Passed | Checks and [audit](tensor-cuda-graphs-2026-09-27/artifact-audit.json) |

CPU coverage includes an independent address-label oracle for dense rebasing,
transposed reshape/GEMM materialization, broadcast and scalar masks, empty and
K=0 geometry, exact resource limits, workspace alignment and every missing
supplemental driver entry. Fault injection checks partial instantiation,
capture abandonment/invalidation, cleanup order, enqueue errors and unwinding.
These mocks do not prove native driver behavior. Cargo still emits the existing
unrelated `wgsl_export` manifest naming warning.

The [native fixture](../../crates/compute-cuda/tests/graph.rs) compiles and
requires NVIDIA for execution. It covers changed allocations/values, strided
and offset input views, reshaped and permuted GEMM operands, f32 precision
policies, native low GEMM, all four dtypes in raw indexing/count composition,
statistics, GQA attention masks, empty replay, exact budgets, validation before
writes and output survival after graph destruction. CPU expectations are
independent of the CUDA kernels.

To require hardware for the complete CUDA suite:

```sh
COMPUTE_REQUIRE_CUDA=1 cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p compute-cuda -- --test-threads=1 --nocapture
```

## Source and remaining work

The [source manifest](tensor-cuda-graphs-2026-09-27/source-fingerprints.json)
and [artifact audit](tensor-cuda-graphs-2026-09-27/artifact-audit.json) identify
the checked source files, logs and local evidence links. Shared tensor, WGSL
and MLX sources are unchanged from the previous phase; no new GPU execution
is attributed to their historical results.

All twelve CUDA source parts are unchanged. The
[reuse audit](tensor-cuda-graphs-2026-09-27/kernel-source-reuse.json) checks
their hashes and the retained PTX artifacts against the existing
[NVRTC 12.8.93 report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-typed-programs/report.json):
52 entries for compute_70/80/90/120, 49,262 combined source bytes and SHA256
`d60ab5d94f748e432c584b0ec88eb2d0cdb54ba9ca47a25aad572f06c9216eda`.
No new NVRTC compilation was needed or claimed.

Remaining work includes NVIDIA numerical and capture/lifetime qualification,
Compute Sanitizer, Tensor Core instruction profiling and graph-versus-prepared
benchmarks including transfer costs. Broader backend work still includes domain
routing, additional operators, platform qualification and hardware CI. The
implemented graph API does not close that larger goal.

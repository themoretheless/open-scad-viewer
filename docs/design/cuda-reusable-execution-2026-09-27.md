# Reusable CUDA tensor execution

Date: 2026-09-27. Prepared f32/u32/f16/bf16 execution is implemented; host and
indexing qualification is recorded [separately](../qualification/tensor-cuda-index-programs-2026-09-27.md),
alongside the [typed/NVRTC evidence](../qualification/tensor-cuda-typed-programs-2026-09-27.md).
NVIDIA execution and timing are pending. The CUDA Graph mode below remains a
proposal based on source inspection and NVIDIA documentation.

## Implemented prepared path

A **prepared resident program** validates a fixed computation, uploads layout
metadata and allocates intermediates once. Each execution binds the caller's
current resident inputs and writes caller-owned outputs. Dtype and full input
layout are fixed. The schedule supports typed views, arithmetic, comparisons,
selection, explicit low casts, reductions, mean, cuBLAS matmul and all four
indexing families: scan, gather, compaction and scatter. Statistics, normalization
and attention are also recorded for f32/f16/BF16. f16/bf16
intermediates use native two-byte storage; reductions and native low GEMM can
produce f32 results directly without full-input conversion buffers. Statistics
retain f64 row summaries and bounded partials; attention retains an output-sized
f64 accumulator. These private buffers are checked against the scratch budget
and allocated once. Eager and prepared routes share metadata and launch helpers.

A future increment can capture the same prepared launch sequence as an explicit **CUDA Graph with
owned input/output slots**. Keep that mode separate: fixed addresses, capture
lifetime, and changes to input values need an additional ownership contract.
NVIDIA execution is a release gate for that mode. A host build or successful PTX
compilation cannot qualify graph replay.

The prepared path removes repeated adapter tensor allocation and metadata transfers.
It still submits individual CUDA/cuBLAS calls. The graph increment can reduce
submission work; neither mechanism fuses arithmetic or establishes a speedup.

## Runtime constraints, verified from source

The table describes ordinary eager calls and the resource constraints shared
with prepared execution. Prepared plans now retain their launch descriptors,
uploaded metadata and scratch; those helpers also serve eager operations.

| Area | Current behavior | Consequence for reuse |
| --- | --- | --- |
| [Platform device](../../crates/gpu-compute/src/cuda.rs) | `CudaDevice::new` retains a context and calls `context.default_stream()` | This is cudarc's null/legacy stream; capture needs an explicit stream choice |
| [Runtime and storage](../../crates/compute-cuda/src/runtime.rs) | A tensor owns `Arc<CudaSlice<T>>`, a `Layout`, and a runtime owner token | Views share allocations; retaining a view affects mutable access |
| Unary/binary execution | Allocate a result, upload dimensions/strides, construct launch arguments, launch | Eager and prepared paths share launch descriptors |
| [Reductions](../../crates/compute-cuda/src/reduction.rs) | Full reductions allocate a partial tensor and metadata at each level; partial-axis reductions use checked strided metadata | Prepared execution retains the full hierarchy and metadata |
| [Matrix multiplication](../../crates/compute-cuda/src/matmul.rs) | Materialize strided operands, lazily create one cuBLAS handle, allocate output, issue one GEMM per broadcast batch | Prepared execution retains copy buffers, batch geometry and handle; values are copied each replay |
| Writes | `write_f32`/`write_u32` require unique contiguous storage; `Arc::get_mut` rejects live clones | A program must not clone inputs and then promise those inputs remain writable through the existing API |
| Reads | Explicit device-to-host read; strided reads first materialize | Readback stays outside preparation and capture |
| [Dynamic libraries](../../crates/compute-cuda/src/libraries.rs) | NVRTC/cuBLAS symbols are checked before invoking cudarc's dynamic loader | New graph/workspace symbols need the same explicit failure path |

The resolved binding is cudarc 0.19.9, with `driver`, `nvrtc`, `cublas`,
`dynamic-loading`, and the CUDA 12.8 API feature. Its safe graph wrapper provides
`begin_capture`, `end_capture`, `upload`, and `launch`. `CudaGraph` retains the
graph handles and stream; it does not retain the buffers, metadata, CUDA function
handles or cuBLAS resources referenced by captured work.

Cudarc's ordinary launch builder records buffer access events. Its graph
`launch` method calls `cuGraphLaunch` directly, so a tensor wrapper must establish
the buffer dependencies and completion tracking around each replay. Also audit
capture failure cleanup: the current `end_capture` wrapper creates the raw graph
before instantiation; a failed instantiation returns before the owning wrapper
is constructed.

Metadata upload is real host/device work today. Its blocking behavior should not
be inferred solely from the runtime comment: in resolved cudarc 0.19.9, ordinary
slice/Vec `HostSlice` guards carry `Sync(None)`. Preparation eliminates these
transfers from replay regardless of the driver's pageable-copy behavior.

## Prepared API

The builder is local to `compute-cuda` and reuses `tensor-core` operations,
`Shape`, `Layout`, reduction validation and `MatmulPlan`. Pure planning and
device preparation have separate boundaries; numerical contracts remain shared.

```rust,ignore
let mut builder = cuda.program();
let x = builder.input(x_layout)?;
let w = builder.input(w_layout)?;
let b = builder.input(b_layout)?;
let product = builder.binary(x, w, BinaryOp::Multiply)?;
let shifted = builder.binary(product, b, BinaryOp::Add)?;
let y = builder.unary(shifted, UnaryOp::Square)?;
let total = builder.reduce(y, ReduceOp::Sum, &[0, 1], false)?;
let mut prepared = builder.prepare(&[y, total], options)?;

prepared.run_into(&[&current_x, &current_w, &current_b],
                  &mut [&mut y_output, &mut sum_output])?;
```

Exported types and operations:

```rust,ignore
CudaRuntime::program(&self) -> CudaProgramBuilder<'_>
CudaProgramBuilder::input(&mut self, layout: Layout) -> Result<CudaValue, CudaError>
CudaProgramBuilder::input_u32(&mut self, layout: Layout) -> Result<CudaValue, CudaError>
CudaProgramBuilder::input_low(&mut self, dtype: LowDtype, layout: Layout)
    -> Result<CudaValue, CudaError>
// typed arithmetic, casts, reductions/mean, views and indexing with resident counts
CudaProgramBuilder::matmul(&mut self, left: CudaValue, right: CudaValue,
                          precision: MatmulPrecision) -> Result<CudaValue, CudaError>
CudaProgramBuilder::prepare(self, outputs: &[CudaValue], options: CudaPrepareOptions)
    -> Result<CudaPreparedProgram<'_>, CudaError>
CudaPreparedProgram::run_into(&mut self, inputs: &[&CudaTensor],
                             outputs: &mut [&mut CudaTensor]) -> Result<(), CudaError>
CudaPreparedProgram::run(&mut self, inputs: &[&CudaTensor])
    -> Result<Vec<CudaTensor>, CudaError>
CudaPreparedProgram::run_typed_into(&mut self, inputs: &[CudaProgramInput<'_>],
    outputs: &mut [CudaProgramOutputMut<'_>]) -> Result<(), CudaError>
CudaPreparedProgram::run_typed(&mut self, inputs: &[CudaProgramInput<'_>])
    -> Result<Vec<CudaProgramOutput>, CudaError>
CudaPreparedProgram::stats(&self) -> CudaProgramStats
```

`run_typed` allocates fresh independent outputs; `run_typed_into` reuses caller
outputs without adapter device allocation or metadata upload. The legacy
`run`/`run_into` require exclusively f32 input and output signatures, but permit
typed intermediates. `CudaProgramInput`/`CudaProgramOutputMut` borrow f32, u32 or
`CudaLowTensor` buffers; `CudaProgramOutput` owns the matching variant and offers
checked accessors. F16/BF16 retain distinct dtype tags through the low wrapper.
A prepared program borrows its runtime, owns metadata/intermediates, and retains
no caller input/output clones between executions. The [crate README](../../crates/compute-cuda/README.md#prepared-resident-programs)
contains compiled `no_run` examples for f32, mixed typed signatures and a
gather → scan → compact → scatter pipeline with resident count composition.

### Typed operation boundary

- f32: nine unary/six binary operations, affine, four reductions, mean and
  explicit-policy vector/batched matmul.
- u32: wrapping add/subtract/multiply, min/max, four reductions; division is
  rejected. u32 values never pass through f32 for integer operations.
- f16/bf16: nine unary/six binary operations, reductions, mean and native low
  matmul. Binary/select/matmul operands must have matching low dtypes.
- All formats: materialize, reshape, permute, broadcast, six comparisons to u32,
  selection, scan, gather, stable compaction and all five scatter modes. Low
  copies/select/gather/compact and Replace retain raw payloads.
- Cast nodes explicitly connect f32 and low storage. Each low arithmetic node
  evaluates in f32 then rounds to low once; subsequent nodes see that rounded
  value. Low reductions decode native u16 into an f32 hierarchy. `*_low_f32`
  reductions/mean/matmul retain the f32 result; low outputs add a single final
  cast. Strided copies keep the source dtype.

Statistics/normalization and attention are recorded through shared eager launch helpers. There is no
builder narrow node, graph capture, fusion or scratch pooling. A pre-existing
narrow view can be a fixed-layout input.

### Resident indexing

Scan retains the axis/mode contract and recursively prepared chunk carries;
low inputs decode directly into f32 accumulators, with one optional final cast.
Gather and scatter return a scalar u32 invalid count beside their values.
Compaction returns fixed-capacity values and a scalar u32 selected count.
Counts are ordinary recorded values usable by later nodes without a readback.
Invalid indices count once per logical index even for empty values outputs.
Scalar indices remove the gathered axis; masks and updates obey the shared
broadcast rules.

Before each replay's dependent passes, explicit asynchronous clears reset
invalid counters, compaction values/count and Replace owner tables. Scatter
copies or decodes the current base on every run. Replace is deterministic for
duplicate indices. Low Add/Multiply use an f32 destination and direct u16 update
loads; low output adds one final cast. Raw low Replace/Min/Max use even native16
scratch capacity for aligned full-word CAS; terminal copies expose unpadded
logical outputs. Scratch budgeting includes physical padding, scan totals,
flags/prefixes and owner/count buffers.

### Fixed signatures and validation

- Input signatures include dtype, shape, element strides and offset. New storage
  handles are allowed when these fields match exactly. A different layout needs
  another prepared plan; it must not silently upload new metadata each replay.
- Each `CudaValue` carries a builder identity and node index. Reject foreign,
  dangling or otherwise invalid values before modifying the plan.
- Inputs can share read-only storage. All outputs must be unique, dense,
  offset-zero tensors of the declared output dtype and shape. Reject output/output and
  output/input allocation aliases, including aliases through views. Broader
  offset output support can follow with its own checks.
- Terminal view or input nodes copy into the independent declared outputs.
  Duplicate requested values produce independent output buffers. An output
  returned by `run` stays unchanged when the program runs again.
- Validate the entire binding set, owners, bounds, dtypes, shapes, unique outputs
  and precision before enqueueing any operation. A validation error leaves outputs
  untouched. A driver failure after enqueueing is not a transactional rollback;
  permanently poison that prepared instance. Reuse requires a new program and
  appropriate runtime recovery.
- `&mut self` serializes access to scratch and mutable launch state. Replay uses
  the runtime stream; maintain cudarc's read/write event tracking and stream
  ordering before scratch reuse or destruction.

### Preparation and lowering

Lower operations into a small ordered schedule: kernel launches, resident
copies/fills and GEMM calls. Share internal checked launch helpers with the eager
API so kernel argument ABIs, dtype policy and row-major GEMM conversion have one
implementation.

Preparation determines every reduction level, contraction count, GEMM batch
offset, strided materialization and output shape. Upload immutable metadata once
and retain it. Allocate one buffer per necessary intermediate initially; avoid
introducing a global pool or an aggressive liveness allocator in this increment.
Check total scratch/metadata byte counts against an explicit preparation budget
before device allocation or cuBLAS initialization. Count actual storage widths:
f32/u32 use four bytes and f16/bf16 two, including one-element empty sentinels.
Low reduction partials and GEMM results use f32; optional final casts have
separate low output storage. Metadata counts include empty u64 sentinels.
Caller outputs, runtime resources and opaque cuBLAS allocations are excluded.

Zero sizes still need a concrete schedule: empty outputs have no element work;
K=0 matmul and empty sum/product contractions write their identities on every
replay. No stale scratch or previous output may supply an implicit result.

`CudaProgramStats` separates `kernel_launches`, `gemm_calls` and `memset_calls`.
The last field counts explicit asynchronous replay clears, excluding preparation;
it is not a kernel count. Scratch/metadata bytes describe retained adapter
storage, including raw low padding. These are schedule counts, not cuBLAS's
internal kernel count or measured peak device allocation.

## Graph mode: separate fixed-address contract

CUDA capture excludes the legacy null stream and disallows synchronization of
the active capture. Captured events also have capture-specific dependency rules.
Use an explicit nonblocking stream created before allocating program resources;
do not begin capture around the existing default-stream eager calls.
[NVIDIA CUDA 12.8 graph capture rules](https://docs.nvidia.com/cuda/archive/12.8.1/cuda-c-programming-guide/index.html#stream-capture)

Provide an opt-in runtime stream configuration, for example
`CudaRuntime::new_with_stream(CudaStreamMode::NonBlocking)`. Keep the existing
constructor's behavior compatible. First graph mode requires that explicit
runtime stream; it does not migrate existing allocations to a new stream or
silently change global synchronization policy.

Suggested graph API:

```rust,ignore
CudaPreparedProgram::capture_owned(self, options: CudaCaptureOptions)
    -> Result<CudaGraphProgram<'_>, CudaError>
CudaGraphProgram::run_into(&mut self, inputs: &[&CudaTensor],
                          outputs: &mut [&mut CudaTensor]) -> Result<(), CudaError>
CudaGraphProgram::stats(&self) -> CudaGraphStats
```

The first implementation owns stable input slots, scratch, immutable metadata
and output slots. `run_into` validates first, copies/materializes current
resident inputs into those slots, launches the captured graph, then copies slot
results into caller-owned outputs. Everything remains on the GPU. The extra
input/output copies are part of the API cost and must appear in its benchmark.
This mode is useful to qualify capture safely; its benefit over prepared direct
launches needs measurement.

Input slots are dense. Capture preparation lowers the retained logical plan
against those dense slot layouts and builds their immutable metadata; it must
not reuse metadata describing the external strided inputs. The external binding
signatures remain checked, and the pre-graph copy follows each external layout.
This avoids allocating the entire storage span of a sparse strided view just to
preserve its physical addresses.

Do not expose graph-owned slots as clonable `CudaTensor` values that replay later
overwrites. A future zero-copy output lease would need a non-owning tensor-view
type whose borrow prevents replay, or an explicit snapshot copy. Returning a
normal tensor reference is insufficient because callers can clone it.

Capture only the prepared launch sequence. Perform NVRTC/module setup, metadata
transfers, allocations, stream creation, cuBLAS initialization and any warmup
before capture. Retain all referenced resources through the last replay's
completion. Wrap capture in an error guard that ends invalidated capture and
destroys any partially created graph; finish or safely order outstanding work
before freeing graph-owned buffers.

Use the existing symbol-probe pattern for each driver entry actually used,
including capture begin/end, instantiate, upload, launch and graph destruction.
Graph preparation should report unsupported/missing API explicitly; it must not
silently fall back to per-node submission while advertising graph replay.

Direct rebinding without copies is a later option. Owned kernel nodes can have
their parameters updated, but cuBLAS capture has opaque internal topology; do
not discover and patch undocumented library kernel arguments. NVIDIA recommends
whole-graph update when captured library topology is not known, while individual
updates suit known nodes. Graph API access also requires external serialization.
[NVIDIA graph update rules](https://docs.nvidia.com/cuda/archive/12.8.1/cuda-c-programming-guide/index.html#updating-instantiated-graphs)

For known custom kernels, the driver exposes
`cuGraphExecKernelNodeSetParams`; updates apply to future launches and retain
context/function restrictions. Cudarc 0.19.9 exposes the raw entry but no safe
parameter-update abstraction in `driver/safe/graph.rs`.
[NVIDIA driver graph API](https://docs.nvidia.com/cuda/archive/12.8.1/cuda-driver-api/group__CUDA__GRAPH.html)

## cuBLAS and Tensor Core policy

Retain the current explicit mapping from
[policy.rs](../../crates/compute-cuda/src/policy.rs):

| Request | Compute type / current eligibility |
| --- | --- |
| `F32` | `CUBLAS_COMPUTE_32F_PEDANTIC` |
| `AllowTf32` | `CUBLAS_COMPUTE_32F_FAST_TF32`; CC >= 8; reject `NVIDIA_TF32_OVERRIDE=0` |
| `AllowF16` | `CUBLAS_COMPUTE_32F_FAST_16F`; CC >= 7 |
| `AllowBf16` | `CUBLAS_COMPUTE_32F_FAST_16BF`; CC >= 8 |

These four modes currently accept f32 storage. Separately, `matmul_low_f32`
supplies native u16 f16/BF16 inputs to GEMM with f32 accumulation and output in
both eager and prepared execution. `matmul_low` adds one final cast. Native F16
requires compute capability 5.0 and BF16 8.0, plus cuBLAS availability. The
prepared schedule retains every f32 precision request and native low dtype,
including empty/K=0 matmul, and rechecks eligibility before every replay.
Unsupported requests fail before enqueueing; a changed TF32 environment
restriction cannot silently reuse an old policy.

Prepare a handle per program stream and retain it with the graph. Fixed host
alpha/beta are captured by value; changing device coefficients are read at replay.
Use an owned, 256-byte-aligned workspace for controlled lifetime and report its
size. Configure stream before workspace because `cublasSetStream` resets the
workspace selection. cuBLAS can otherwise introduce allocation nodes during
capture. These are resource-management choices, not requirements to change the
current numerical policy.
[NVIDIA cuBLAS graph/workspace behavior](https://docs.nvidia.com/cuda/archive/12.8.1/cublas/index.html#cuda-graphs-support)

Tensor Core eligibility remains distinct from actual instruction use. cuBLAS
selects its implementation; graph replay does not prove that selection. Verify
the selected precision numerically, and verify Tensor Core execution with an
NVIDIA profiler on representative hardware before claiming it.
[NVIDIA Tensor Core selection](https://docs.nvidia.com/cuda/archive/12.8.1/cublas/index.html#tensor-core-usage)

## Qualification gates

### Can be checked on the current CPU host

- Pure planning tests: broadcast/view propagation, nonzero offsets, invalid node
  identities, reductions/scans, indexing shapes/counts, scalar/empty/K=0 shapes,
  GEMM batch offsets, raw low padding, overflow and preparation budgets.
- Mock execution boundary: all validation precedes enqueue; replay requests no
  device allocations, metadata uploads or recompilation; identities are written
  each replay; new input/output handles populate the launch arguments.
- Ownership/state tests where represented independently of real CUDA handles:
  foreign runtime tokens, alias detection, capture failure cleanup ordering and
  poisoned execution state. Mocks do not prove actual CUDA lifetime safety.
- Missing-symbol diagnostics, explicit precision eligibility and TF32 override
  behavior; host build, Clippy, rustdoc and architecture dependency checks.
- NVRTC/PTX compilation if an existing compiler is available. This proves source
  acceptance and entry ABIs only, not execution, capture or Tensor Core use.

### Requires an NVIDIA machine

- Required-device tests with `CUDA_REQUIRED=1` or
  `COMPUTE_REQUIRE_CUDA=1`: changed resident input handles and values, multiple
  outputs, strided/broadcast inputs, all empty rules, reductions and matrix modes.
- Prepared replay parity with eager execution and independent f64 references;
  retained prior outputs must stay unchanged. Validate failures before launch
  using output sentinels, and exercise repeated execution after allowed errors.
- Nonblocking-stream capture/instantiate/upload/replay, captured event ordering,
  external producer dependencies, cuBLAS workspace, destruction after in-flight
  work, and capture/instantiation failure recovery. Run Compute Sanitizer for
  memory/race checks where supported.
- Compare eager, prepared direct launch and graph-with-slot-copies separately.
  Warm setup/compilation first, change inputs each sample, validate all results,
  rotate at least 31 samples, retain raw evidence and source/binary fingerprints.
  Report wall time including the same final readbacks; measure CUDA event time
  separately. Include graph input/output copies in both relevant timers.
- Record GPU, compute capability, driver, cuBLAS/NVRTC versions, precision and
  workspace policy. Profile actual Tensor Core kernels separately from graph
  launch overhead. Host-only results cannot replace this gate.

## Scope and rollout

1. Implemented: shared CUDA launch preparation, typed builder/transport and
   prepared `run_typed_into`, including typed indexing, statistics/normalization, attention and compatible f32 methods.
   CPU planner, binding,
   budget and poison-state tests pass; NVRTC compiled 52 current entries for
   compute_70/80/90/120. Required native fixtures are present, but numerical
   execution remains unverified on the current Apple host.
2. Qualify it on NVIDIA and measure eager versus prepared execution with changing
   resident inputs. Preserve the existing eager public contracts.
3. Add explicit nonlegacy stream selection and graph-owned slots over that same
   lowered schedule; qualify capture, lifetime and copied-slot performance.
4. Extend prepared operation coverage using existing eager dispatch helpers as
   needed. Consider known-node graph rebinding or typed zero-copy leases only
   after ownership qualification and measurements justify them.

Keep mathematical planning in `tensor-core`, CUDA tensor execution in
`compute-cuda`, and platform stream/driver capability support in `gpu-compute`.
This increment needs no raster changes and no new generic graph framework in
the backend-neutral crate.

The [statistics/attention qualification](../qualification/tensor-prepared-statistics-2026-09-27.md)
records the current CUDA host checks and MLX native enabled/disabled results.

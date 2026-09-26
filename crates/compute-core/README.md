# compute-core

Architecture and crate boundaries: [GPU library design](../../docs/design/gpu-library-architecture.md).

Runtime library for WGSL compute kernels — the compute counterpart of
`raster-core`. Domain kernels (math-core, photogrammetry-core, geometry-bridge)
own their WGSL sources; what they share is the dispatch plumbing, and that
lives here.

## Typed array programs

`ComputeRuntime` compiles reusable kernels and owns typed `GpuArray<f32>` and
`GpuArray<u32>` storage. Arithmetic is currently f32; u32 arrays support storage,
range updates and readback. `ComputeProgram` prepares an ordered chain once and
can submit it repeatedly after inputs change.

```rust
use compute_core::{BinaryOp, ComputeRuntime, UnaryOp, gpu_compute::GpuContext};
use std::time::Duration;

let context = GpuContext::new().expect("GPU adapter");
let runtime = ComputeRuntime::new(&context)?;
let input = runtime.upload(&[3.0f32, -4.0])?;
let mut program = runtime.program();
let squared_norm = program.dot(&input, &input)?;
let norm = program.unary(UnaryOp::Sqrt, &squared_norm)?;
let normalized = program.binary(BinaryOp::Divide, &input, &norm)?;

// All kernels and the final copy/map are submitted together. Intermediate
// arrays, including the scalar norm, stay on the GPU.
let ticket = program.submit_read(&normalized)?;
// An event loop can call ticket.try_read()? until it returns Some(values).
let values = ticket.wait(Duration::from_secs(10))?;
// Approximately [0.6, -0.8].
```

Supported operations:

- affine map (`input * scale + offset`);
- add, subtract, multiply, divide, min and max, including a single-element GPU
  array broadcast on either side;
- negate, abs, square, sqrt, reciprocal, exp, log, sin and cos;
- sum and dot product with all reduction passes on the GPU.

### Memory and submission rules

- New arrays are zero-initialized by wgpu without uploading a host zero vector.
- Clones and `prefix(len)` views share storage. `write(array, offset, values)`
  updates contents without changing prepared bindings.
- `_into` variants reuse caller-allocated output arrays. Input/output aliasing,
  length mismatches, foreign runtime arrays, invalid ranges and allocations over
  device limits return `ComputeError` before recording GPU work.
- Binary operations accept equal lengths or one scalar; dot requires equal
  lengths. Empty sum/dot returns zero, empty elementwise output stays empty.
- Programs retain their intermediate allocations. Repeated submissions reuse
  kernels, parameters, bindings and storage; each readback ticket gets separate
  staging memory so overlapping submissions cannot overwrite earlier results.
- Queue writes apply before the next submission, not between program steps.
  Use separate arrays for distinct per-step input values.
- `submit_read` performs no GPU wait. `try_read` polls completion without waiting;
  `wait(timeout)` is a native convenience. A ticket is consumed once. Dropping
  one discards that result and does not block other submissions.
- Arithmetic uses WGSL f32 semantics, including its function domains and
  backend-dependent rounding. No portable NaN, infinity or division-by-zero
  contract is promised. Compare reductions with tolerances against CPU references.

Run the complete centering/normalization example and measured CPU comparison:

```sh
cargo run --release --offline -p compute-core --example array_pipeline
COMPUTE_REQUIRE_GPU=1 cargo test --offline -p compute-core
```

The example reports median host-observed time for GPU compute plus full readback,
upload plus compute plus full readback, and a CPU reference with f64 sum
accumulation and f32 outputs (GPU arithmetic is f32 throughout). Preparation is
outside timing, three warmups precede nine samples, and timing order rotates.
These are workload-specific measurements, not GPU timestamp timings. The strict
GPU environment flag prevents the runtime suite from passing via adapter skips.

## What it owns

- **`Kernel`**: a validated, cached compute pipeline with a declared binding
  layout. Construction is eager and fallible — a broken WGSL source is a
  `KernelError`, not a panic (device error scope around pipeline creation).
- **Buffer helpers**: typed storage/uniform uploads
  (`storage_f32`, `storage_f32_zeroed`, `storage_u32`, `uniform_f32`) and
  typed readbacks (`read_f32`, `read_u32`) through a MAP_READ staging copy
  (storage buffers cannot hold `MAP_READ` in wgpu).
- **Generic kernels** in `shaders/` (`include_str!`-embedded):
  - `scale_add` / `scale_add4` — elementwise affine map
    `output[i] = input[i] * scale + offset`, scalar and vec4 variants
    (the vec4 variant streams 16 bytes per thread and runs ~3x faster
    than scalar on Apple Silicon)
  - `zip_mul` / `zip_mul4` — elementwise product `output[i] = a[i] * b[i]`
  - `block_sum` — grid-strided sum reduction into per-group partials: a
    fixed grid of workgroups strides over arbitrary lengths, so one
    dispatch covers more than the 65535-workgroup limit; chain passes —
    or call the [`reduce_f32`] helper — to fold down to a scalar

## The `WG` anchor convention

Every tunable kernel source declares:

```wgsl
const WG: u32 = 256;
@compute @workgroup_size(WG)
```

The runtime substitutes a tuned power of two before compilation
(`Kernel::with_workgroup_size`, `Kernel::tuned`). Substitution is validated:
non-power-of-two sizes are rejected, and the result is clamped to the device's
per-workgroup limits so a tuner pick never fails pipeline validation on a
tighter backend.

Per-backend tuning follows `gpu_compute::tuned_workgroup_size`: smaller
groups on Metal (tile-based GPUs), the larger default elsewhere. Note the
tuning is a heuristic — for pure streaming elementwise kernels a 256-wide
group measured ~40% faster than 128 on Apple Silicon (M4 Max, see
`examples/bench.rs`), so domain kernels should benchmark their own shape.

## Usage

```rust
use compute_core::gpu_compute::GpuContext;
use compute_core::shaders::SCALE_ADD_WGSL;
use compute_core::{Binding, Kernel, read_f32, storage_f32, storage_f32_zeroed, uniform_f32};

let context = GpuContext::new().expect("GPU adapter");
let device = &context.device;
let queue = &context.queue;

let kernel = Kernel::tuned(
    &context,
    "scale_add",
    SCALE_ADD_WGSL,
    "main",
    &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
    128, // metal_size
    256, // default_size
)
.expect("kernel builds");

let input = storage_f32(device, queue, &[1.0f32, 2.0, 3.0, 4.0]);
let output = storage_f32_zeroed(device, queue, 4);
// Params: count(u32), scale(f32), offset(f32), pad — packed as 4 f32.
let mut params = vec![0.0f32; 4];
params[0] = f32::from_le_bytes(4u32.to_le_bytes());
params[1] = 2.0;
params[2] = 0.5;
let params = uniform_f32(device, queue, &params);

kernel.dispatch(device, queue, &[&params, &input, &output], 4);
let result = read_f32(device, queue, &output, 4);
assert_eq!(result, vec![2.5f32, 4.5, 6.5, 8.5]);
```

Binding order is sequential from `binding(0)` in declaration order
(uniforms and storage interleaved as the shader declares them).

A single non-strided dispatch covers at most `65535 * workgroup_size`
invocations (the wgpu per-dimension workgroup-count limit); larger workloads
are the caller's job to chunk (`Kernel::max_dispatch_invocations`).
Grid-strided kernels like `block_sum` sidestep this via
`Kernel::dispatch_groups`, and `reduce_f32` schedules the whole multi-pass
chain for arbitrary input lengths.

## Tests and bench

```sh
cargo test --offline -p compute-core
cargo run --release --offline --example bench -p compute-core
```

GPU-backed tests skip cleanly on machines without an adapter.

Measured on Apple M4 Max (Metal, release): scale_add ~47 GB/s at WG=128 and
~67 GB/s at WG=256; the vec4 variant scale_add4 ~113 GB/s (16 bytes streamed
per thread vs 4); the zip_mul + block_sum dot-product pair ~70–115 GB/s
effective. Vectorized access is the single biggest lever for streaming
kernels — domain kernels that need bandwidth should default to the vec4
shapes or add their own multi-element-per-thread variants.

## Reusable GPU chains

`ComputeBatch` retains prepared bindings and records all steps into one queue
submission. `Reduction` prepares all sum passes once, with separate uniforms
and intermediate buffers for each pass. Its scalar output stays on the GPU and
can feed another kernel. The synchronous `reduce_f32` convenience function uses
this plan internally, then reads back the scalar.

```rust,ignore
let reduction = Reduction::new(device, queue, &sum, &products, count);
let mut batch = ComputeBatch::new();
batch.push(&multiply, &multiply_bindings, multiply.workgroup_count(count));
batch.push_reduction(&reduction);
// Bind reduction.output() as input to a downstream kernel if needed.
// Update input buffer contents before each submission; reuse this batch.
batch.submit(device, queue);
let total = read_f32(device, queue, reduction.output(), 1)[0];
```

Plans have fixed buffer identities and element counts; rebuild them when those
change. Queue writes before submission apply to the entire batch, so use distinct
uniform buffers for steps with different parameters. Empty sums produce zero;
zero-length typed readbacks return an empty vector. Readback currently uses a
separate copy submission and blocking map. Automatic graph scheduling, buffer
lifetime pooling and kernel fusion are not implemented. The typed runtime above
provides nonblocking readback within the compute submission.

### Local measurement (2026-09-26)

One release run on the local Metal backend, using the example above:

| Elements | GPU resident + full readback | Upload + GPU + full readback | CPU with f64 sums |
| --- | ---: | ---: | ---: |
| 4,097 | 0.334 ms | 0.378 ms | 0.007 ms |
| 1,000,003 | 3.014 ms | 2.783 ms | 1.807 ms |

Maximum absolute difference from the reference was 1.863e-9 and 1.164e-10,
respectively. The small reversal between the two large-array GPU measurements
is run variation, not a benefit from uploading data. This full-readback workload
shows no speedup over the CPU reference. CPU sums use f64 to avoid accumulated
serial-f32 reference error; the GPU uses f32 throughout. Use these numbers as an
example of end-to-end cost, not a hardware-independent performance claim.

### Shared storage and scratch capacity

`GpuArray::view()` exposes a borrowed `gpu_compute::GpuBufferView` for domain
adapters. `GpuBuffer::new(&context, bytes, usage)` creates storage with trusted
context ownership; `ComputeRuntime::import_buffer` checks owner, usage and scalar
length. Context clones share identity; unrelated devices are rejected.

`ScratchPool::new(&runtime, budget_bytes)` retains named allocations. A repeated
`reserve::<f32>("input", len)` reuses sufficient capacity; growth replaces storage
and increments `ScratchArray::generation`. Rebuild prepared bindings on a new
generation. Old plans retain their original buffers. The budget covers capacity
retained by the pool, excluding replaced buffers held by callers or GPU work.

Readback delegates to `gpu_compute::ByteReadback`. Use `record_read` and mark the
ticket `submitted` after submitting a caller-owned encoder. Mapping failures,
cancellation and empty payloads are distinct. Legacy `read_f32`/`read_u32` panic on
transport failure; use their `try_` variants for recoverable errors.

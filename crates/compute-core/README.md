# compute-core

Runtime library for WGSL compute kernels — the compute counterpart of
`raster-core`. Domain kernels (math-core, photogrammetry-core, geometry-bridge)
own their WGSL sources; what they share is the dispatch plumbing, and that
lives here.

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

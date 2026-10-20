# Apple Silicon shared-buffer uploads

## Source audit

The native compute and raster backends use wgpu 30.0.1, which selects Metal
on this Mac. They do not directly construct `MTLBuffer`s. Inspection of the
resolved wgpu sources established two relevant rules:

- `wgpu-hal/src/metal/device.rs::create_buffer` selects Metal
  `StorageModeShared` for buffers with mapping usage, and `StorageModePrivate`
  otherwise.
- `wgpu-core/src/device/resource.rs` implements `mapped_at_creation` through
  staging unless the buffer also has `MAP_WRITE` usage. Setting
  `mapped_at_creation` alone is insufficient for direct shared-memory upload.

Before this change, `ComputeRuntime::upload` allocated private storage and
called `queue.write_buffer`. The compatibility f32/u32/uniform helpers did
the same. Raster geometry used mapped-at-creation buffers without `MAP_WRITE`
and additionally allocated packed byte vectors. Instance records allocated
both a float vector and a packed byte vector.

## Implementation

`GpuContext` enables `MAPPABLE_PRIMARY_BUFFERS` automatically only for
macOS/aarch64 Metal adapters advertising it. Timestamps and subgroups remain
explicit opt-ins. Other platforms keep their existing feature policy.

`GpuBuffer::new_initialized` and `create_buffer_initialized` initialize a
fresh allocation through a write-only mapping, then release the mapping and
unmap before returning the buffer. On supported Apple Silicon devices they
add `MAP_WRITE`, so the CPU writes the final Metal shared allocation. Other
devices use wgpu's portable mapped-at-creation path. Synchronous browser
compatibility helpers do not block on error-scope promises.

The new path covers:

- `ComputeRuntime::upload`, and `upload_generated` for direct generation;
- software-binary64 tensor upload, generating low/high u32 words directly
  without a temporary word vector;
- compatibility f32/u32 storage and f32 uniform initialization;
- raster vertices, indices, morph sources, object uniforms and instance pools.

Raster float/index uploads borrow little-endian bytes instead of allocating a
packed vector. Instance pools serialize one fixed-size record at a time into
the mapping, eliminating the full host float and byte vectors.

An existing CPU slice still requires one CPU copy into the shared allocation.
Direct generation avoids that intermediate source allocation. This is not
arbitrary `Vec<T>` import: no page alignment assumptions or unsafe external
allocation lifetime are introduced.

GPU-only zero/output allocations remain private. Updates to existing buffers
remain queue writes to preserve ordering with GPU work and aliases. Snapshot
readback retains staging because multiple pending results must survive reuse
of the source buffer. No per-upload GPU wait is introduced by initialization.

The browser WebGPU/TypeScript path cannot expose native Metal shared memory.
The optional MLX backend delegates allocation and upload to MLX-C; this change
does not bypass that ownership. CUDA and reusable-buffer update paths are
unchanged. This is not a claim that every transfer in the repository is zero-copy.

## Reproduction

```sh
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core --tests -- --test-threads=1
cargo check --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core --target wasm32-unknown-unknown
cargo run --manifest-path crates/Cargo.toml --release \
  -p compute-core --example bench_shared_upload
```

The benchmark compares the previous `zeros + write` implementation with
mapped upload on the same device. It alternates order for 12 pairs per size,
after three warm-up pairs, then executes the same affine GPU kernel and waits
for completion. Complete result verification occurs outside timing for every
sample. `upload_us` measures CPU allocation/fill; `total_us` also includes
recording/submission and GPU completion. It does not measure frame time.

## Validation results

- Native full run: **198 passed, 0 failed, 0 ignored**, across 39 test executables
  (`native-tests.log`). Three of the new shared-upload tests are included.
- Focused final run: **4 passed**, including the additional featureless-device
  fallback test (`shared-upload-tests.log`). This overlaps the full run.
- WebAssembly compilation of gpu-compute, compute-core and raster-core passed
  (`wasm-check.log`). Browser runtime was not exercised.
- Native tests cover bit-preserving f32/u32/binary64 transfer, odd lengths,
  zero padding, empty input, oversized allocation rejection, affine GPU work,
  queued input updates, independent pending readback snapshots, and existing
  raster rendering/instancing/morph behavior.
- M4 Max/Metal runtime selected shared uploads; the new allocation test verifies
  the final buffer has MAP_WRITE usage. The resolved backend source establishes
  that this selects Metal StorageModeShared without initialization staging.
  No Metal frame capture was taken.
- M1/M2/M3 hardware and other native backends were not physically tested.
  The featureless-device test validates fallback on the same M4 Max.

## Final paired measurements

Apple M4 Max, Metal, release profile. Median microseconds; complete GPU affine
results checked after each sample. No task-owned GPU tests ran concurrently
with the final measurement. Background system activity was not controlled.

| Payload | CPU upload: queue → shared | Upload + shader + completion: queue → shared | Shared total wins |
|---|---:|---:|---:|
| 4 KiB | 21.2 → 15.1 | 190.4 → 193.6 | 2/12 |
| 1 MiB | 95.8 → 93.1 | 344.9 → 315.5 | 12/12 |
| 16 MiB | 1156.8 → 1255.2 | 2856.7 → 2630.8 | 12/12 |

`paired-upload.csv` preserves all final pairs; `summary.json` also records median
paired total-time ratios. `paired-upload-concurrent-tests.csv` retains the
initial run made while native tests were active; it is not the final result.
For 1 and 16 MiB, median total time fell by about 8.5% and 7.9%, respectively,
and every final pair favored shared upload. The 4 KiB total did not improve
(about 1.7% slower); no small-buffer latency win is claimed. At 16 MiB CPU
allocation/fill itself was about 8.5% slower even though total time improved.
The benchmark measures this upload-and-affine workload, not rendering FPS or
all geometry workloads. Eliminating initialization staging does not eliminate
the CPU copy from an existing source slice or shared-memory bandwidth costs.

# Buffer updates and readback copy audit — 2026-10-20

Follow-up: the controlled repeats and actual-decoder build comparison in
`../metal-transfer-stability-2026-10-20/README.md` supersede the timing conclusions
below. These exploratory measurements remain preserved as evidence of noise.

This follows the initial shared-upload work in `../metal-shared-upload-2026-10-20/`.
The earlier initialization benchmark is preserved independently.

## Removed copies

| Path | Before | After |
|---|---|---|
| Typed f32/u32 readback and compatibility helpers | Snapshot GPU copy → mapped bytes → host byte Vec → typed Vec | Same snapshot copy → decode mapping directly into final typed Vec |
| Software binary64 readback | Snapshot → host byte Vec → u32 Vec → f64 Vec | Snapshot → decode mapping directly into f64 Vec |
| RGBA image readback | Mapped padded bytes → full host Vec → tightly packed RGBA Vec | Copy useful row bytes directly from mapping into final RGBA Vec |
| Nearest-neighbor results | Index and distance byte Vecs → result tuples | Read both independent mappings into final result tuples |
| Descriptor matching | Flattened f32 Vec → packed byte Vec → queue staging | Borrow existing contiguous descriptor bytes → queue staging |
| Descriptor matching results | Host row/column byte Vecs → typed records | Decode mapped staging directly into final records |
| Nearest/chamfer point updates | f64 xyz → f32 Vec → byte Vec → queue staging | Convert directly into queue staging |
| SDF payload updates | Packed full byte Vecs → queue staging | Borrow existing contiguous f32/u32 bytes → queue staging |
| Raster style/scene updates | Packed byte Vec → queue staging | Borrow the existing stack array bytes → queue staging |
| Rectification | RGB → u32 Vec → byte Vec → queue staging; readback via byte/u32 Vecs | RGB words generated directly in queue staging; mapped output converted directly into final RGB Vec |
| Sweep selection/score readback | Byte Vec per batch job → decoded Vec | Decode each job directly from mapped staging |

`ComputeRuntime::write_generated` provides the generated-update path with
ownership and range validation before touching queue staging. `write` from an
already available scalar slice already used borrowed bytes on LE platforms,
so it retains that path. No claim is made that an existing source slice can
be uploaded through this queue API without any CPU copy.

## Synchronization and ownership

The destination of a queued update is not mapped or modified directly by the
CPU. `write_buffer_words` encodes into `Queue::write_buffer_with` storage and
drops the view before returning. The same queue ordering as `write_buffer`
is preserved: updates execute before the next submission, after previously
submitted GPU work. Buffer identity and existing prepared bindings remain
unchanged. No new GPU wait or implicit submission is added by updates.

Snapshot GPU copies remain intentional. Pending result A must retain its old
values even when later GPU work overwrites the source for result B. Mapping
the source automatically would also exclude GPU use of it until CPU unmap.
Removing that copy requires an explicit exclusive-lifetime API, not silently
changing the current snapshot contract.

`ByteReadback::try_read_with`, `wait_with`, and `try_read_buffer_with` expose
mapped bytes only for the duration of a consumer callback. The callback's
result cannot borrow the mapping. A guard drops the mapping after its view,
including on consumer panic. Timeout keeps the ticket pending; cancellation,
consumption and mapping failures keep their prior error behavior. Existing
byte-Vec APIs remain available and now delegate to this implementation.

The browser has an additional JS/WASM boundary inside wgpu's mapped view;
compilation coverage does not establish native shared-memory behavior there.

## Validation

- Native gpu-compute + compute-core + raster-core: **201 passed**, zero failures
  across 39 executables (`native-tests.log`).
- GPU domain tests: **31 passed** across math, photogrammetry and SDF
  (`domains-tests.log`).
- Math integration tests for cooperative nearest neighbors, plans, recorded
  domains and stable statistics: **15 passed** (`math-tests.log`).
- Final photogrammetry rerun after sweep/rectification changes: **9 passed**
  (`photo-final-tests.log`); overlaps the domain run.
- Final raster rerun after borrowed style/scene updates: **25 passed**
  (`raster-final-tests.log`); overlaps the full native run.
- Final wasm32 check of all six affected crates with GPU features passed
  (`wasm-final.log`). Browser runtime and CUDA execution were not exercised.

New regressions cover partial generated updates, cloned array aliases,
independent pending snapshots, empty/invalid/foreign updates before generator
invocation, mapped-consumer cancellation and timeout retry, consumption exactly
once, and unmapping on consumer panic followed by successful staging reuse.
Existing raster tests exercise row padding (including 1×3 and 65×2), rendering,
instancing and morph behavior. Existing domain tests check CPU reference
agreement and reusable buffers. No CPU fallback is used by the benchmark.

## Paired measurements

Apple M4 Max / Metal, release profile. Two separate runs; each uses three
warm-up pairs and twelve measured pairs per case, alternating old/new order
inside the same process. The old path reconstructs the removed allocations;
the new path uses the same GPU buffers, shader or snapshot copy, submission
and wait policy. Complete results are verified outside timing on every sample.
No task-owned GPU tests ran concurrently; unrelated background system activity
was not controlled. All measurements, including regressions, are retained.

`update_generated` compares the previous point-conversion-style float Vec +
packed byte Vec + queued write against direct queue-staging generation.
`update_borrowed` compares packing a ready f32 slice with borrowing its bytes.
Their total time includes the same affine shader and GPU completion.

Read totals include staging allocation, snapshot copy, mapping, wait and final
host decoding. CPU time isolates the removed host copy/packing/decoding work.
The f64 control also reconstructs the former intermediate u32 Vec. RGBA uses
511-pixel rows with four padding bytes per row. This is a transport benchmark,
not a complete geometry or rendering performance qualification.

### 16 MiB results

Median times in milliseconds. Each row is an independent measured run; pairs
remain available in the CSV files and both absolute medians and paired ratios
are recorded in `summary.json`.

| Case | Run | CPU old → new | Total old → new | Total new wins |
|---|---|---:|---:|---:|
| update_generated | 1 | 4.729 → 4.702 | 9.343 → 8.928 | 8/12 |
| update_borrowed | 1 | 2.162 → 1.700 | 4.380 → 3.377 | 9/12 |
| read_f32 | 1 | 2.044 → 1.594 | 3.552 → 3.289 | 6/12 |
| read_f64 | 1 | 4.488 → 1.456 | 6.614 → 3.022 | 12/12 |
| read_rgba | 1 | 2.221 → 1.667 | 3.821 → 3.299 | 7/12 |
| update_generated | 2 | 7.623 → 7.179 | 12.371 → 10.861 | 7/12 |
| update_borrowed | 2 | 3.474 → 2.598 | 6.844 → 7.503 | 6/12 |
| read_f32 | 2 | 2.308 → 2.109 | 4.739 → 7.774 | 2/12 |
| read_f64 | 2 | 6.803 → 2.079 | 9.846 → 6.225 | 9/12 |
| read_rgba | 2 | 3.239 → 2.509 | 17.346 → 10.822 | 7/12 |

The consistent result is binary64 CPU decoding: all 24 large-buffer pairs
favor the new path, with median paired CPU speedups of about 3.0× and 3.3×.
Absolute median total time fell from 6.61 to 3.02 ms in run 1 and from 9.85 to
6.22 ms in run 2. These runs do not establish a universal 2× end-to-end gain.

Other cases remove full-size temporary allocations and memory copies, but
total timings vary and some cases regress in a run. In particular the 4 KiB
generated update changes direction across runs, and 16 MiB f32 total readback
is slower in run 2. No stable total-time speedup is claimed for those cases;
the retained benefit is the eliminated allocations/copies without a new wait
or changed snapshot semantics. No size threshold is inferred from noisy data.

## Reproduction

```sh
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core --tests -- --test-threads=1
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml \
  -p osv-math -p photogrammetry-core -p sdf-core \
  --features osv-math/gpu,photogrammetry-core/gpu,sdf-core/gpu --lib gpu -- --test-threads=1
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml \
  -p osv-math --features gpu --test gpu_nearest_cooperative --test gpu_plans \
  --test gpu_recorded_domains --test gpu_stable_stats -- --test-threads=1
cargo check --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math -p photogrammetry-core -p sdf-core \
  --features osv-math/gpu,photogrammetry-core/gpu,sdf-core/gpu --target wasm32-unknown-unknown
cargo run --manifest-path crates/Cargo.toml --release -p compute-core --example bench_transfer_copies
```

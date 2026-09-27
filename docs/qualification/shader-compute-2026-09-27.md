# Shader compute expansion: local qualification

Date: 2026-09-27. Native Metal backend, Rust `nightly-2026-09-16`, wgpu 30.
Implementation builds on migration commit `39590313` in the working tree.

## Delivered behavior

- `CompareOp` creates zero/one u32 masks from f32 arrays, including scalar
  broadcasting on either side.
- Exclusive u32 scan uses hierarchical workgroup passes and wrapping addition.
  Stable compaction keeps selected f32 values in order, writes its count on the
  GPU and clears the remaining capacity on every execution.
- Recorded math now includes nearest neighbors and cloud bounds, raw moments,
  centroid and population covariance. These can feed generic compute in the
  same encoder without intermediate host reads.
- Kernel reflection derives the actual workgroup size and used buffer contract
  from the selected WGSL entry point. Checked bindings validate context, usage,
  size, alignment and writable aliases. Structural layouts remain compatible
  across entry points using different subsets of a shared bind group.
- A bounded per-context LRU cache retains compiled pipelines. Live handles
  survive eviction; failed builds do not evict valid entries.
- Buffer allocation failures are recoverable, typed readback can retry after
  timeout, and photogrammetry no longer unmaps already-released staging buffers.
  Matching/sweep transfer errors now reach existing CPU fallback paths.

## Correctness and compatibility

| Check | Result |
| --- | --- |
| Four-crate suite: gpu-compute, compute-core, math, raster | 158 passed on native Metal |
| CPU-only math | 68 passed |
| Photogrammetry GPU module tests and two production sweep paths | 8 passed |
| Math/SDF/geometry-bridge/photogrammetry all-feature compilation | Passed |
| Photogrammetry CUDA-feature compilation after readback changes | Passed |
| gpu-compute and compute-core all-target Clippy with warnings denied | Passed |
| gpu-compute, compute-core and math rustdoc with warnings denied | Passed |
| Production dependency directions and CPU-only math dependency check | Passed |

Coverage includes a 16,777,233-element scan beyond one dispatch grid, multiple
scan levels, nonbinary masks, reused buffers, aliases across scalar types,
foreign contexts, empty inputs, timeout recovery, allocation errors, invalid
shader contracts, shared bind groups, and cache eviction with live pipelines.

A repeated `nearest_neighbors → compare → compact → sum` test changes the
threshold between submissions. Only the resulting count and sum are read back.
Statistics tests cover 65,537 points and multiple reduction levels. Native
photogrammetry sweep agrees with CPU on all 1,058 valid fixture pixels.

Cargo retains the existing `wgsl_export` binary-name warning. The all-feature
geometry-bridge check also reports its existing unused documentation comment.
NVIDIA execution, Vulkan/DX12 and browser Rust runtime behavior were not measured.

## Reproducible timing

```sh
cargo run --release --offline --manifest-path crates/Cargo.toml \
  -p compute-core --example compact_reduce
```

The example records `compare → compact → sum` once and reuses that plan. Each
run submits once and reads only a u32 count and f32 sum: eight bytes total.
Inputs are `(i % 251 - 125) * 0.125`, selected when greater than zero. It validates
count exactly and sum against a CPU f64 reference with relative tolerance 2e-6.

Three warmups precede nine samples per path, with timing order rotated. The
following ranges are the per-process medians from three final runs. Plan
preparation is outside execution timings. GPU timings include host submission,
staging allocation, waiting and scalar readback; these are not GPU timestamps.

| Elements | Selected | Plan preparation, ms | Resident GPU, ms | Upload + GPU, ms | CPU direct selected sum, ms |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,097 | 2,000 | 0.082–0.092 | 0.463–0.993 | 0.684–1.121 | 0.006 |
| 1,000,003 | 498,000 | 0.103–0.113 | 3.711–4.722 | 6.021–6.179 | 1.516–1.531 |

The CPU path directly computes the final count and sum; the GPU path also
materializes a compacted array for downstream use. For this simple workload,
the CPU path is faster. Earlier local runs varied substantially with host load,
so these timings are local observations, not throughput guarantees or evidence
of a general GPU speedup.

## Numerical and ownership limits

- GPU math uses f32. Coordinates and intermediate sums/products must remain
  finite within f32 range. Covariance uses `E[pp] - E[p]E[p]` and loses precision
  when offsets dominate the cloud's spread. Recorded plans cannot inspect data
  already on the device before executing.
- Nearest-neighbor search is exhaustive: O(queries × targets). Empty targets
  produce u32::MAX/f32::MAX sentinels; distance overflow also keeps the sentinel.
- Compaction returns full capacity plus GPU count. Its zero tail makes a full
  `sum` safe; operations that change zero must respect the logical count.
- Scan arithmetic wraps modulo 2^32. It does not detect arithmetic overflow.
- KernelCache bounds retained entries, and ScratchPool bounds retained capacity.
  Caller handles and outstanding GPU work may keep evicted/replaced resources alive.

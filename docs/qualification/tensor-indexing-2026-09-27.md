# Typed tensor indexing qualification

This report preserves the typed-indexing checkpoint. Later reductions, vector
matmul and current validation are recorded in [the subsequent report](tensor-reductions-2026-09-27.md).

Date: 2026-09-27. This extends the [initial tensor backend qualification](tensor-backends-2026-09-27.md).

## Implemented behavior

`TensorIndexBackend` now provides the same typed operations in WGSL, CUDA and
MLX: u32 storage/views, f32/u32 comparisons and selection, arbitrary-axis scans,
device-index gather, and stable compaction. Existing f32 tensor APIs remain
available. `GpuTensor<T=f32>` and `CudaTensor<T=f32>` distinguish storage types;
MLX checks dtype on its dynamic handles.

- Scan supports inclusive/exclusive and forward/reverse traversal. u32 arithmetic
  wraps; f32 uses parallel accumulation. WGSL and CUDA scan blocks and recursively
  scan their totals. WGSL reuses the existing flat scan where its contract fits.
- Gather inserts the index tensor's shape at the chosen input axis. Invalid
  indices produce zeros and increment a separate GPU scalar once per index.
  An empty output does not erase invalid indices. Source memory is read only
  after validating the selected index.
- Compaction preserves logical row-major order, accepts a broadcast mask, returns
  a GPU count, and zeroes unused capacity. WGSL reuses the existing local prefix
  scan; CUDA normalizes/scans/scatters; MLX builds unique destinations for both
  selected and rejected elements, then scatters zero values into the tail.
- Recorded WGSL operations prepare all fallible stages before appending them to
  a program. Repeated execution resets gather counts and rewrites compacted tails.
- Three-way broadcasting and batched matmul validate the final element count,
  allowing zero dimensions to produce empty output even when an intermediate
  batch-only count would overflow.

No intermediate masks, indices, prefix sums or dynamic counts are read back to
the CPU. The common fixture passes a resident compare → compact → scan → gather
→ select chain through each backend.

## Local results

| Check | Result |
| --- | --- |
| Common CPU shape/layout/indexing contracts | 11 tests pass |
| WGSL tensor tests | 17 actual Metal tests pass, including 5 indexing tests |
| MLX | 11 actual Metal tests plus 2 native loader/error tests pass |
| CUDA host contracts | 7 tests pass |
| CUDA source compilation | NVRTC 12.8.93 compiles all 19 entries for compute_70/80/90/120 |
| Required CUDA execution | Fails explicitly because this host has no CUDA driver/device |
| Full platform/compute/math/raster regression | 228 tests pass with GPU, timestamps and subgroups required |
| Shader validation | 31 shipped templates and 7 u32 specializations pass Naga |
| Rust checks | All four tensor/compute crates pass strict Clippy and rustdoc |
| Consumers | All-feature math, SDF, geometry and photogrammetry compile |
| WASM compilation | `tensor-core`, recorded `compute-core` APIs and MLX's native-module boundary compile for wasm32 |

The final shared fixtures include every comparison, unsigned values above 2^24
and u32::MAX, strided logical views, all scan modes, several rows and chunks,
recursive carry across 131,075 elements, scalar and multidimensional indices,
empty axes, empty outputs with invalid indices, all-true/all-false masks and
stable order. WGSL-specific cases add one-submission program reuse, nonzero
output offsets, sentinel tails, alias rejection and transactional errors.

MLX testing exposed a missing empty-u32-sum initializer in the installed Metal
runtime. The adapter now constructs the mathematical zero identity with native
`mlx_zeros`, including nonempty outputs from empty reductions. Final testing also
checks the stricter dtype guards on the f32 trait methods.

Independent source reviews found no remaining issues in the shared contracts,
reference fixtures or CUDA indexing ABI, bounds, scan carries and pointer
lifetimes. Source review does not establish CUDA execution correctness.

An isolated Linux aarch64 container compiled the exact production CUDA sources
using NVIDIA's NVRTC 12.8.93 shared library and the runtime's precise-math options.
All four virtual architectures compiled successfully with the 19 expected entry
points. The compiler package was pinned and its SHA256 verified. The container
had no CUDA device; this proves source compilation, while CUDA execution, cuBLAS
results and actual Tensor Core use remain unqualified.
[Reproduction, compiler logs and PTX fingerprints](../../crates/compute-cuda/qualification/README.md).

The WASM compile check found an unconditional native adapter calling blocking
readback methods absent on wasm32. The native `TensorBackend` implementation is
now gated to native targets, while tensor recording and nonblocking readback
remain compilable. CI now includes this target check. A compiler check does not
establish browser runtime support.

Existing unrelated warnings remain for the `wgsl_export` binary name and a
macro doc comment in `geometry-bridge`. Remote CI was not executed for this
working-tree change. No new performance claim is based on these correctness runs.

## Reproduction and evidence

```sh
python3 scripts/qualify-tensor-backends.py --offline \
  --backend wgsl --backend mlx --backend cuda \
  --output crates/target/tensor-indexing-qualification
```

The strict combined report remains unsuccessful on this Mac because CUDA was
required. WGSL and MLX pass separately within the same serialized run.

- [Structured report](tensor-indexing-2026-09-27/report.json).
- [CPU contracts](tensor-indexing-2026-09-27/contracts.txt).
- [WGSL](tensor-indexing-2026-09-27/wgsl.txt).
- [MLX](tensor-indexing-2026-09-27/mlx.txt).
- [CUDA contracts and explicit unavailable-device failure](tensor-indexing-2026-09-27/cuda.txt).
- [Full regression](tensor-indexing-2026-09-27/gpu-regression.txt).
- [MLX's initial native error](../../crates/compute-mlx/qualification/index-empty-u32-initial-failure.txt).

Remaining full-library coverage is tracked in the
[backend design](../design/tensor-backends-2026-09-27.md): scatter, more reductions,
rank-one matmul, native half storage, graph/allocator reuse, domain integration
and hardware qualification remain open.

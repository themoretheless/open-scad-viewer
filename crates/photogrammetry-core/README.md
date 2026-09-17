# photogrammetry-kernel

Dependency-free Rust photo reconstruction. Public entry points: `reconstruct`, `dense::densify`, `camera` and `features`. Input is RGB plus supplied focal length in pixel units. No image codec, graphics runtime, C++ binary, or external process is required by this crate.

Outputs are relative-scale, partial observations. Unregistered views are explicit. Dense output is a collection of observed surface patches, not a watertight object. See [application documentation](../../docs/photogrammetry.md) for validation and limitations.


## Calibration and reference evaluation

`calibration::rectify` accepts measured Brown–Conrady intrinsics of the EXIF-oriented original raster and returns corrected RGB plus applied-camera provenance. No automatic fitting is performed. Both sparse and dense consume that same corrected image. Details: [calibration guide](../../docs/photogrammetry-calibration.md).

`FeatureOptions::ROOT` and `GeometryOptions::CONSENSUS` are the tested defaults; `BASELINE` variants preserve algorithm comparisons. Relative-pose estimation requires an identity first-camera pose (finite elements, tolerance 1e-12). Experimental ROBUST/PHYSICAL variants have documented regressions and are not defaults.

`DenseOptions` retains 3×3 frontoparallel sweep by default. `DenseEstimator::SlantedPlane` with patch radius 2 is experimental; it improves some sloped scenes and can lose thin geometry. Source-sample counters quantify work separately from hypothesis count.

`evaluation::evaluate_clouds` computes exact bidirectional point-to-point
metrics in a pre-established common frame/scale, including directed summaries,
precision/recall/F1, symmetric mean distance and symmetric Hausdorff
worst-case distance. It also reports sampled model/reference bounds, centroid
and covariance through `math-core`'s fused point-cloud stats primitive. The
`evaluate` example reads ASCII XYZ/PLY and emits JSON; it performs no
registration or scale fitting. `evaluation_comparison` checks the spatial index
against exhaustive distances. See [comparative validation](../../docs/photogrammetry-improvements.md).

`EvaluationOptions::acceleration` accepts `Cpu` (exact KD-tree), explicit
`Gpu`/`Cuda`, or `Auto`. `Auto` keeps small sampled clouds on the KD-tree and
uses the brute-force batch GPU/CUDA kernel for larger clouds via this crate's
`gpu`/`cuda` feature (forwarded to `math-core`). Counterintuitively, this
wins: `examples/kdtree_vs_gpu.rs` measured it 3-11x faster than the KD-tree on
an NVIDIA RTX 5090, at every size from 2,000 to 1,000,000 points per cloud —
the KD-tree's better asymptotic complexity (O(log n) vs O(n) per query) does
not outweigh the GPU's parallelism at these scales. Without the feature
compiled in, `Auto`/`Gpu`/`Cuda` are a no-op and the KD-tree always runs, so
switching them on can only help, never silently regress to a slow CPU scan.

## Native usage and acceleration

The crate is a plain Rust library (`rlib`, std-only) and works unchanged outside
the browser. The `reconstruct` example doubles as the reference CLI:

```sh
cargo run --release --manifest-path crates/Cargo.toml -p photogrammetry-core \
  --example reconstruct -- out.ply FOCAL_PIXELS input1.ppm input2.ppm ...
PHOTO_DENSE=1           # also write out.ply.surface.ply
PHOTO_ACCURACY=on       # qualified accuracy bundle; with PHOTO_DENSE also
                        # DenseOptions::accurate() (5x5 patches, dual scale,
                        # sparse depth prior: -27% surface error on analytic scenes)
PHOTO_ACCELERATION=gpu   # requires building with --features gpu (wgpu)
PHOTO_ACCELERATION=cuda  # native descriptor PTX, wgpu fallback for other kernels
```

The optional `gpu` feature adds `wgpu` and accelerates descriptor matching
(3.2-47x on the synthetic descriptor benchmark) and the frontoparallel NCC
depth sweep (batched, selection on the GPU: ~10x of the dense stage, 31-68 ms
on the frozen sets) via Metal on macOS and Vulkan/DX12 on Linux/Windows.
The optional `cuda` feature adds native PTX descriptor matching on NVIDIA, with
the wgpu path as fallback for kernels that do not have a CUDA port. CPU
defaults stay bit-identical with or without the feature. Browser builds keep
the feature off; there the same WGSL sweep runs through WebGPU from the
viewer's worker.
Qualification and measured numbers: [gpu-matching-2026-09-09](../../docs/qualification/photogrammetry/gpu-matching-2026-09-09.md).

`FeatureOptions { acceleration: Acceleration::Auto, .. }` now resolves
descriptor matching by measured pair-work: below 10K candidate pairs it keeps
the CPU scan; from 10K upward it uses CUDA when the crate is built with
`--features cuda`, otherwise the portable GPU path. Check or tune the
recommendation with `features::recommended_for_descriptor_matching(a, b)` and
`cargo run --release -p photogrammetry-core --features cuda --example
bench_matching`. Descriptor buffers are cached grow-only per matcher, so
repeated stable-size image-pair matching avoids per-call device allocation.
`photogrammetry_core::gpu::backend_report()` reports the portable backend
(`metal`, `vulkan`, `dx12`, `webgpu`); with `--features cuda`,
`cuda_device_report()` reports the native CUDA device. On an RTX 5090:

| features A × B | work | recommended | CPU | Auto | wgpu/Vulkan | CUDA |
| --- | --- | --- | --- | --- | --- | --- |
| 64 × 64 | 4K | cpu | 0.295 ms | 0.287 ms | 0.206 ms | 0.122 ms |
| 128 × 128 | 16K | cuda | 1.186 ms | 0.089 ms | 0.289 ms | 0.131 ms |
| 256 × 512 | 131K | cuda | 8.585 ms | 0.226 ms | 0.750 ms | 0.229 ms |
| 1,024 × 1,024 | 1.0M | cuda | 66.665 ms | 1.181 ms | 2.158 ms | 1.272 ms |
| 2,048 × 2,048 | 4.2M | cuda | 258.194 ms | 3.964 ms | 5.887 ms | 3.991 ms |

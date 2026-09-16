# photogrammetry-kernel

Dependency-free Rust photo reconstruction. Public entry points: `reconstruct`, `dense::densify`, `camera` and `features`. Input is RGB plus supplied focal length in pixel units. No image codec, graphics runtime, C++ binary, or external process is required by this crate.

Outputs are relative-scale, partial observations. Unregistered views are explicit. Dense output is a collection of observed surface patches, not a watertight object. See [application documentation](../../docs/photogrammetry.md) for validation and limitations.


## Calibration and reference evaluation

`calibration::rectify` accepts measured Brown–Conrady intrinsics of the EXIF-oriented original raster and returns corrected RGB plus applied-camera provenance. No automatic fitting is performed. Both sparse and dense consume that same corrected image. Details: [calibration guide](../../docs/photogrammetry-calibration.md).

`FeatureOptions::ROOT` and `GeometryOptions::CONSENSUS` are the tested defaults; `BASELINE` variants preserve algorithm comparisons. Relative-pose estimation requires an identity first-camera pose (finite elements, tolerance 1e-12). Experimental ROBUST/PHYSICAL variants have documented regressions and are not defaults.

`DenseOptions` retains 3×3 frontoparallel sweep by default. `DenseEstimator::SlantedPlane` with patch radius 2 is experimental; it improves some sloped scenes and can lose thin geometry. Source-sample counters quantify work separately from hypothesis count.

`evaluation::evaluate_clouds` computes exact bidirectional point-to-point metrics in a pre-established common frame/scale. The `evaluate` example reads ASCII XYZ/PLY and emits JSON; it performs no registration or scale fitting. `evaluation_comparison` checks the spatial index against exhaustive distances. See [comparative validation](../../docs/photogrammetry-improvements.md).

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
PHOTO_ACCELERATION=cuda  # same kernels on NVIDIA via wgpu; see docs/design/native-gpu-cuda.md
```

The optional `gpu` feature adds `wgpu` and accelerates descriptor matching
(3.3-18.9x) and the frontoparallel NCC depth sweep (batched, selection on the
GPU: ~10x of the dense stage, 31-68 ms on the frozen sets) via
Metal on macOS and Vulkan/DX12 on Linux/Windows; `Acceleration::Gpu` is opt-in and
falls back to the CPU reference without an adapter. `Acceleration::Cuda` is
accepted too: these kernels are portable shaders, so it runs the same wgpu path
(dedicated PTX ports currently exist in `sdf-core`). CPU defaults stay
bit-identical with or without the feature. Browser builds keep the feature off;
there the same WGSL sweep runs through WebGPU from the viewer's worker.
Qualification and measured numbers: [gpu-matching-2026-09-09](../../docs/qualification/photogrammetry/gpu-matching-2026-09-09.md).

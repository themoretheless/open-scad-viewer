# photogrammetry-kernel

Dependency-free Rust photo reconstruction. Public entry points: `reconstruct`, `dense::densify`, `camera` and `features`. Input is RGB plus supplied focal length in pixel units. No image codec, graphics runtime, C++ binary, or external process is required by this crate.

Outputs are relative-scale, partial observations. Unregistered views are explicit. Dense output is a collection of observed surface patches, not a watertight object. See [application documentation](../../docs/photogrammetry.md) for validation and limitations.


## Calibration and reference evaluation

`calibration::rectify` accepts measured Brown–Conrady intrinsics of the EXIF-oriented original raster and returns corrected RGB plus applied-camera provenance. No automatic fitting is performed. Both sparse and dense consume that same corrected image. Details: [calibration guide](../../docs/photogrammetry-calibration.md).

`FeatureOptions::ROOT` and `GeometryOptions::CONSENSUS` are the tested defaults; `BASELINE` variants preserve algorithm comparisons. Relative-pose estimation requires an identity first-camera pose (finite elements, tolerance 1e-12). Experimental ROBUST/PHYSICAL variants have documented regressions and are not defaults.

`DenseOptions` retains 3×3 frontoparallel sweep by default. `DenseEstimator::SlantedPlane` with patch radius 2 is experimental; it improves some sloped scenes and can lose thin geometry. Source-sample counters quantify work separately from hypothesis count.

`evaluation::evaluate_clouds` computes exact bidirectional point-to-point metrics in a pre-established common frame/scale. The `evaluate` example reads ASCII XYZ/PLY and emits JSON; it performs no registration or scale fitting. `evaluation_comparison` checks the spatial index against exhaustive distances. See [comparative validation](../../docs/photogrammetry-improvements.md).

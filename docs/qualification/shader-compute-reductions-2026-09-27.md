# Shader compute: default reductions and numerical stability

Date: 2026-09-27. Native macOS, Apple M4 Max with 40 GPU cores, Metal.
This continues [the previous research round](shader-compute-research-2026-09-27.md).
GPU benchmarks run serially. Compare paths within the same experiment: clock,
thermal and host scheduling conditions differ across runs. Results establish
behavior on this device; other backends require their own qualification.

## Dot uses fusion by default

`ComputeProgram::dot` now uses the shared expression compiler and fused sum
executor. It evaluates products in registers, reduces them to workgroup
partials, then reduces those partials to a scalar. The full product array is
removed. `dot_into` accepts caller-owned scalar storage and resets it for empty
inputs on every execution. Input lengths must match, and owner, shape and alias
errors leave the recorded program unchanged. The compiled pipeline is retained
lazily per runtime; prepared plans retain their bindings and intermediates.

`bench_dot` compares three prepared programs with identical resident full input
arrays and a four-byte final readback:

1. Materialized products followed by the frozen previous sum shader/schedule.
2. Materialized products followed by the current production `sum`.
3. The current production `dot`.

Each mode has at least 200 ms warmup and 31 rotated samples. Compilation,
allocation and input upload are excluded from steady execution. First-use dot
preparation is reported separately. GPU timestamps cover the compute pass only;
ordinary host timing contains no timestamp writes or query resolution. All
results are checked against f64 accumulation over the uploaded f32 operands.
Summation order and backend FMA contraction can change the answer within f32
tolerances. A changed summation order is not a compensated-precision algorithm.

| Elements | Previous map+sum, GPU | Current map+sum, GPU | Default dot, GPU | Previous map+sum, host | Current map+sum, host | Default dot, host |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000,003 | 0.113208 ms | 0.063917 ms | 0.019416 ms | 0.254042 ms | 0.199792 ms | 0.158958 ms |
| 4,000,003 | 0.478792 ms | 0.272292 ms | 0.063208 ms | 0.596542 ms | 0.397375 ms | 0.194167 ms |

At four million elements, default dot is 7.57× faster on the GPU and 3.07× in
ordinary host latency than the previous implementation. Against materialized
products with the newly optimized sum, the same dot still improves GPU time
4.31× and host latency 2.05×. It removes 16,000,012 bytes of product storage and
uses 7,816 bytes of first-stage partials plus a small tail. One-element and 4K
host timings are roughly neutral (0.12–0.13 ms). First-use dot preparation in
this benchmark took 0.911 ms; subsequent sizes took 0.021–0.034 ms. Driver
caches were not cleared, so these are not system-cold compilation measurements.

[Raw samples](../../crates/compute-core/benchmarks/dot-production-metal.txt),
[reproducible example](../../crates/compute-core/examples/bench_dot.rs).

## Bounded sum scheduling

The retained change uses the existing scalar `BLOCK_SUM_WGSL`. For typed `sum`
on Metal above 65,536 values, the first grid is `ceil(N/4096)`, capped at 256
workgroups. The grid-strided kernel covers the complete input, then one final
dispatch reduces the partials. Raw `Reduction` constructors, smaller arrays
and other backends keep the previous schedule. No input padding or extra
device feature is required. `sum_into` reuses distinct scalar storage and
actively resets it for empty inputs.

The study tried 28 initial shared-tree/vector candidates, then compared
ABI-compatible variants and bounded scheduling. True vec4 storage needs padded
input, while scalar-storage vector arithmetic added complexity for inconsistent
additional gains. The existing kernel with bounded scheduling was retained.
The exact 65,536-element case showed a small GPU regression under the new
schedule; it remains on the original path. At 65,537, the old path needs an
additional reduction stage, and the retained policy wins.

Final measurements compare the actual production API with the frozen old
kernel and schedule: 14 sizes, at least 200 ms warmup, 31 rotated samples,
separate compute-pass timestamps and ordinary four-byte readback timings.

| Elements | Previous GPU | Current GPU | Previous host | Current host |
| --- | ---: | ---: | ---: | ---: |
| 1,000,003 | 0.063667 ms | 0.013417 ms | 0.192375 ms | 0.134750 ms |
| 4,000,003 | 0.228750 ms | 0.026000 ms | 0.354083 ms | 0.149458 ms |
| 16,000,003 | 0.990791 ms | 0.152750 ms | 1.156375 ms | 0.323708 ms |

The 16M case improves 6.49× on the GPU and 3.57× in host latency. First-stage
partial storage falls from 250,004 bytes to 1,024 bytes. f32 summation order
changes; precision checks include cancellation and dynamic ranges against a
norm-scaled f64 reference. This is not compensated summation.

`Reduction::record` now groups its existing dispatches into one compute pass;
`record_in_pass` permits composition and profiling in a supplied pass. Supplied
kernel parameters and raw reduction scheduling stay unchanged.

[Candidate decisions and limits](../../crates/compute-core/benchmarks/reduction-research.md),
[final production samples](../../crates/compute-core/benchmarks/reduction-production-metal-round4.txt).

## Explicit subgroup experiments

`GpuContext::with_features` makes optional capabilities explicit and returns an
error when the adapter lacks a requested feature. The default constructor still
requests no optional features. The study compared shared-memory trees,
`subgroupAdd` and manual shuffle across 576 size/schedule configurations.
At 4M, the best tree measured 26.125 µs GPU and 154.917 µs host; the best
`subgroupAdd` measured 27.292 µs and 164.542 µs. At 16M, `subgroupAdd` had a
modest median advantage but broad timing variation. It was not promoted to
production. Manual shuffle remains restricted experimental code: ballot masks
alone do not make a butterfly reduction correct for arbitrary sparse active
lanes. The Add path avoids assumptions about local-index-to-subgroup mapping.

The native source dialect is pinned to the current Naga/wgpu version. Its
builtins work with explicit device features; the WGSL `enable subgroups`
directive is not implemented in this Naga version. Browser, Vulkan, DX12 and
CUDA execution were not measured here.

[Full experiment, primary specification links and restrictions](../../crates/compute-core/benchmarks/subgroup-reduction-metal.md).

## Centered point-cloud covariance

`MathGpuProgram::point_cloud_stats_stable` and its `_into` variant add a second
pass over the points. After the original statistics pass computes an approximate
centroid, the second pass accumulates residuals `d = p - centroid` and their
products. Covariance is `E[dd] - E[d]E[d]`; the residual-mean term corrects error
from rounding the first centroid. Bounds, raw sums, raw products and centroid
retain the existing 24-f32 output layout and values.

The first centered shader used nine scalar fields with a dynamic inner loop
and 256 lanes. A paired follow-up replaced it with three `vec3` workgroup arrays,
three vector additions per reduction level and 128 lanes. The retained shader
is 2.27–2.65× faster than the first centered version for the million-point cases.
The previous implementation is frozen in the benchmark for reproducibility.

All three paths use the same input and read the full 24-f32 output. GPU and
ordinary host measurements are separate, with 200 ms warmup and 31 rotated
samples per case. Maximum covariance error is over all matrix entries.

| Points / coordinate offset | Raw-moment GPU | First centered GPU | Retained centered GPU | Raw-moment error | Retained error |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4,097 / 1,000,000 | 0.322791 ms | 0.798000 ms | 0.645083 ms | 186,057.30 | 3.0e-8 |
| 1,000,003 / 0 | 0.819666 ms | 3.538084 ms | 1.557459 ms | 1.23e-7 | 1.23e-7 |
| 1,000,003 / 10,000 | 1.388750 ms | 5.819250 ms | 2.315667 ms | 22.61 | 1.23e-7 |
| 1,000,003 / 1,000,000 | 1.585083 ms | 7.069167 ms | 2.663958 ms | 146,124.48 | 1.23e-7 |

For the final million-point offset case, ordinary resident latency increases
from 2.057 to 3.503 ms; including upload, from 6.166 to 7.874 ms. This is a
precision option with an extra traversal, not a blanket speed optimization.
At 4K, the centered f64 CPU reference takes about 0.12 ms, faster than the GPU
round trip. Absolute GPU and CPU timings drift during the experiment; use
matched comparisons within each row, not timings across offsets or runs.

The synchronous `MathGpuSession::try_point_cloud_stats_stable` uses the same
recorded implementation and reads only the final 24 scalars. Kernels are cached;
this convenience call allocates/uploads its inputs and plan each time. Reuse a
recorded plan for repeated resident inputs.

The explicit API leaves legacy placement and CPU/CUDA behavior unchanged.
Arithmetic remains f32: differences lost when uploading large coordinates
cannot be recovered, all raw and centered intermediates must remain finite,
and rounding can still yield tiny negative eigenvalues. Precision comparisons
use a centered f64 reference over the actual uploaded f32 coordinates.
For background on variance accuracy when spread is small relative to the data,
see [Chan, Golub and LeVeque, 1983](https://researchportal.hkust.edu.hk/en/publications/statistical-computing-algorithms-for-computing-the-sample-varianc/).

[Precision and cost report](../../crates/math-core/benchmarks/stable-stats-metal.md),
[final paired samples](../../crates/math-core/benchmarks/stable-stats-metal.csv).

## Validation

The final native run passed **210 tests across 27 suites**, with adapter,
timestamp and subgroup requirements enabled and test threads serialized.
This covers compute, platform, math and raster together. CPU-only math also
passed all 68 tests. Separate subgroup CPU validation covers 48 generated WGSL
variants; native parity fixtures cover 36 candidates and two small-workgroup
variants.

The initial combined run exposed an overly brittle timestamp regression test:
a single short pass was delayed enough to miss its 8× long/short separation
threshold. Eight unchanged repetitions did not reproduce it. The revised test
keeps that threshold, takes seven pairs per encoder order, and checks every
sample's freshness and enclosing wall interval. Eight revised repetitions
(112 same-encoder pairs) passed with normal parallel execution as well as
focused serial execution. The production profiler was not changed.
[Diagnosis and preserved failure](../../crates/gpu-compute/benchmarks/timestamps-regression-stability.md).

Additional gates passed:

- All-target Clippy with `-D warnings` for compute and platform.
- Strict rustdoc for compute, platform and GPU math.
- All-feature consumer builds for math, SDF, geometry and photogrammetry.
- The dependency-direction/CPU-only-math architecture check.
- Targeted math Clippy with the three pre-existing math lint allowances.

Cargo still reports the existing raster binary naming warning (`wgsl_export`).
The consumer build also retains the existing geometry-bridge macro doc-comment
warning. These are not new diagnostics from this change.

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p osv-math -p raster-core \
  --features osv-math/gpu --lib --tests -- --test-threads=1
```

[Final test output](../../crates/compute-core/benchmarks/reductions-validation-metal.txt),
[source fingerprints](../../crates/compute-core/benchmarks/reductions-source-manifest.json).
Browser, Vulkan, DX12 and NVIDIA execution remain unverified in this round.

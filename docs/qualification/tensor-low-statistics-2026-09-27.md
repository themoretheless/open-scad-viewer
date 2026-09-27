# Statistics from low storage: qualification

Date: 2026-09-27. This extends [low scatter](tensor-low-scatter-2026-09-27.md)
with resident f16/BF16 softmax, log-softmax, logsumexp, moments and layer norm
on WGSL, CUDA and MLX. The full compute goal remains active.

## Shared contract

`TensorLowStatsBackend` extends the narrow storage/statistics traits. Its five
`_low_f32` methods return evaluated f32 results. The corresponding `_low`
methods round each result once into the input dtype, using the exact nearest-
even storage cast contract. Finite inputs, arbitrary axes, strides/broadcasts,
empty shapes/contractions, population variance and positive finite epsilon
follow `TensorStatsBackend`. Reported variance or log probabilities can exceed
f32 range; layer norm remains usable without materializing that variance.

Empty axes or contraction size one define singleton groups: mean/logsumexp
preserve every finite value exactly, including subnormals and signed zero;
softmax is one; log-softmax, variance and layer norm are zero. General
statistical/transcendental comparisons use f32 tolerances and documented
underflow limits. Low output may underflow or overflow after final rounding.

Inputs and intermediates stay on device. Complete input, shifted-input,
exponentiated-input and centered-input f32 temporaries are avoided. Actual f32
results and bounded row/partial statistics are allowed. The low variants may
materialize their actual f32 result before its final cast.

## WGSL architecture and numerical corrections

Four generated storage specializations reuse the existing statistics templates
and shared recorded planner. Packed loads decode values directly. Groups of
2-256 elements use one workgroup and retain values through the shared stages.
Longer groups use bounded partial reductions and per-row summaries. The low
summary has eight f32 words, including a flag for exact coordinate scaling;
the original f32 summary ABI remains four words.

Integer ordering preserves low extrema. Moment groups below 2^-64 lift their
coordinates by 2^64 through integer exponent manipulation before arithmetic.
This preserves BF16 subnormal coordinates whose normalized f32 output is
normal. Mean/variance restore original units; layer norm uses epsilon in
consistent units. Distribution operations retain their original logit units.

The first real Metal run found two compiler-sensitive arithmetic failures:
tiny layer-norm results became zero, and near-maximum same-sign groups produced
NaNs. A diagnostic showed correct lifted coordinates/variance followed by a
zero ratio; the nominal `(scale / sqrt(epsilon)) * 2^-64` arithmetic had lost
the intended intermediate range. The same diagnostic showed an infinite
midpoint despite finite extrema, consistent with reassociating the expression
into an overflowing sum before halving. A native f32 probe reproduced the midpoint
failure, extending the fix to the shared f32 statistics path.

Exact integer power-of-two helpers preserve the intended intermediate ranges.
Near-positive and near-negative f32 maxima were added to shared f32 conformance.
Initial failures and diagnostic sources/results are retained. Existing `_into`
ownership, alias, offset, replay and transactional-recording rules apply to
all ten low/f32 output methods, including the two moments outputs.

[WGSL contracts](../../crates/compute-core/benchmarks/tensor-low-normalization-contracts.md),
[initial failures](../../crates/compute-core/benchmarks/tensor-low-normalization-initial-metal-tests.txt),
[measured shader intermediates](../../crates/compute-core/benchmarks/tensor-low-normalization-diagnostic-metal.txt),
[native f32 probe](../../crates/compute-core/benchmarks/tensor-low-normalization-midpoint-native-diagnostic-metal.txt),
[corrected focused checks](../../crates/compute-core/benchmarks/tensor-low-normalization-metal-tests.txt).
The arithmetic diagnosis is based on shader outputs and controlled source
changes; GPU machine instructions were not inspected.

A reproducible [CPU audit](tensor-low-statistics-2026-09-27/audit_stats_scaling.py)
checks the integer scaling formulas against independent exact arithmetic:
103,371 finite encodings for each downshift power (1/64/128), 25,630 bounded
upshifts and 34 explicit rounding/carry/signed-zero cases all pass.
Its [output](tensor-low-statistics-2026-09-27/bit-scaling-cpu-audit.txt) records
the inspected shader hash. This validates the emulated formulas; the native
tests above establish their tested WGSL behavior.

## CUDA

Three native u16 entrypoints specialize the existing statistics partial,
element-output and moments-output passes. They share row addressing, reduction
geometry and anchored f64 arithmetic with f32 input; merge and logsumexp output
remain common. Singleton mean/logsumexp casts write the actual result directly.
There is no full f32 input expansion. Internal f64 is a stability choice; no
performance or general f64 tensor-storage capability is inferred from it.

NVRTC 12.8.93 compiled **50 entrypoints for compute_70/80/90/120** with precise
math options matching the runtime. The current 46,317-byte concatenated source
has SHA256 `624b0c91e46398b9038dd23e6d4c92fd829a86cdd021b210855576c0fbb07fff`.
Source parts/order, every entry parameter list and retained PTX hashes were
checked against the runtime source.

[Compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-low-statistics/report.json),
[source/ABI audit](../../crates/compute-cuda/qualification/low-statistics-source-audit.txt),
[host tests and explicit skip](../../crates/compute-cuda/qualification/low-statistics-host-tests.txt),
[required CUDA failure](../../crates/compute-cuda/qualification/low-statistics-cuda-required.txt).

There is no NVIDIA device on this host. Required mode fails with absent
driver/device. Compilation does not establish native numerical execution,
launch correctness, Tensor Core instruction use or speed.

## MLX

A shared low-reduction plan permutes axes by metadata, records row/contracted
geometry and uses `P=min(ceil(K/4096),1024)` partials per row. Statistics inject
register transformations into the same reduction tree/hierarchy as ordinary
low reductions. Max-shifted exponentials and scaled centered moments therefore
avoid complete transformed-input arrays. Custom output kernels restore logical
axis order by a view. Raw extrema and exact integer power-of-two scaling keep
tiny BF16 moment coordinates meaningful; CUDA's wider arithmetic handles this
without a separate coordinate lift.

For R groups, visible distribution scratch contains 2R f32 row values and up
to 2RP f32 partials; moments use 4R row values, a 4R-word state and up to 4RP
partials. Actual outputs, native internal allocations, metadata and readback
are separate. These describe logical allocations, not peak RSS. No MLX
performance measurement is claimed.

The shared fixture passed on the first native run. One private long-strided
mean check failed near zero: -9.535049e-6 versus -9.53637938e-6, absolute error
1.33e-9. A solely result-relative bound was inappropriate for cancellation;
that check now uses four f32 epsilons times input-coordinate magnitude.
Probability/variance relative bounds and the common fixture were unchanged.
The initial failure is retained; production code did not change for that issue.

[Initial private-test failure](../../crates/compute-mlx/qualification/low-statistics-initial-failure.txt),
[full native output](../../crates/compute-mlx/qualification/low-statistics-metal.txt).

## Verification coverage

The common f64 oracle anchors input deviations independently of backend scaling.
The low fixture checks every finite encoding as a singleton, arbitrary/unsorted
axes, transposed/broadcast inputs, group lengths 255/256/257/513/131077, extreme
offsets, finite maxima, variance overflow and epsilon down to minimum positive
f32. Returned low values must exactly match independent rounding of the
corresponding f32 result. A long uniform softmax checks low subnormal output;
uniform thirds distinguish actual f32 output from widened low results.

Strict relative tiny-BF16 layer-norm checks cover mixed signs, subnormal/normal
boundaries, the lift threshold, mixed magnitudes, constant rows and hierarchical
partials. Both shape and output length are checked before numerical comparison.
A resident low softmax -> layer norm/moments chain reads only final outputs.
Backend tests cover more than 65,536 rows, offsets, lazy/replayed inputs,
ownership, aliases and validation before empty shortcuts.

| Check | Result |
| --- | --- |
| Required compute-core / gpu-compute / raster-core / osv-math GPU regression | **279 passed**, 0 failed, 0 ignored; 43 suites |
| New / strengthened existing WGSL statistics | **4 / 5 passed** |
| Naga | All four new generated variants and existing shipped sources pass |
| Required MLX suite | **60 passed**: 3 unit + 57 integration |
| Shared CPU contracts/references | **33 passed** |
| CUDA host tests / compile doctest | **13 / 1 passed**; native fixture explicitly skipped |
| NVRTC compilation and ABI | **50 kernels x 4 architectures** |
| Strict all-target Clippy and strict rustdoc, four tensor crates | Pass |
| wasm32 check: tensor-core, compute-core, compute-mlx | Pass |
| Architecture, changed Rust formatting, diff whitespace | Pass |
| NVIDIA execution / Tensor Core instruction profiling | Pending hardware |

[GPU regression](tensor-low-statistics-2026-09-27/gpu-regression.txt),
[contracts](tensor-low-statistics-2026-09-27/contracts.txt),
[Clippy](tensor-low-statistics-2026-09-27/clippy.txt),
[rustdoc](tensor-low-statistics-2026-09-27/rustdoc.txt),
[WASM](tensor-low-statistics-2026-09-27/wasm.txt),
[architecture](tensor-low-statistics-2026-09-27/architecture.txt),
[formatting](tensor-low-statistics-2026-09-27/rustfmt.txt),
[test summary](tensor-low-statistics-2026-09-27/test-summary.json),
[environment](tensor-low-statistics-2026-09-27/environment.json),
[source fingerprints](tensor-low-statistics-2026-09-27/source-fingerprints.json),
[final artifact audit](tensor-low-statistics-2026-09-27/final-artifact-audit.txt),
[MLX source/log manifest](../../crates/compute-mlx/qualification/low-statistics-source-manifest.json).
Existing Cargo warnings concern `wgsl_export` naming and duplicate `bench`
example names. No CUDA skip counts as a successful hardware test.

## Matched performance and memory

The [WGSL benchmark](../../crates/compute-core/benchmarks/tensor-low-statistics.md)
measures all five operations, both dtypes and five geometries, including
transposed input and a non-last axis. Both paths produce f32 results and are
checked against an independent f64 reference. Moments reads both outputs.
Programs are reused, with 200 ms warmup and 31 rotated samples; GPU timestamps
and unprofiled host/readback timings are separate.

Two unchanged runs each had 34/50 lower direct-path GPU medians. Across runs,
32 cases were faster in both, 14 slower in both, and four changed sides.
The largest measured improvement was 1.26x; the worst slowdown was 11.1%
for long F16 moments. An unusual strided BF16 softmax timing jump in both
paths did not repeat, so both raw runs are retained. Its cause is unprofiled.
The repeat has other absolute jumps affecting both paths; stable absolute
latency is not established by these runs.
No significance, application-level or CUDA/MLX speed claim is made.

The direct path removes a 4N-byte full f32 input conversion. Larger low row
summaries partly offset that saving: for K>256, the net visible f32 scratch
reduction is `4N-32R-16` bytes for distribution or `4N-48R-16` for moments/
layer norm. N is input size and R is group count. Small groups remove 4N bytes.
Actual outputs, metadata, staging and driver allocations are separate;
these are logical buffer differences, not peak RSS. The benchmark report
contains both timing tables and per-case memory accounting.

## Remaining scope

Low-storage attention remains open. Native WGSL f16 arithmetic, direct
low-input/f32-output MLX matrix products, NVIDIA numerical/profiling evidence,
broader tensor operations and deployment/CI qualification are still part of
the full goal. This phase closes statistics coverage and strengthens the
existing f32 numerical regression.

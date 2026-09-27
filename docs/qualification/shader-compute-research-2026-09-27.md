# Shader compute: reduction, scan and matrix research

Date: 2026-09-27. Native macOS, Apple M4 Max with 40 GPU cores, Metal, release
builds. This round compares with the implementation retained in
[the previous round](shader-compute-performance-2026-09-27.md). Different runs
had different clock/thermal conditions, so compare paired measurements within a
row, not absolute timings across rounds. GPU benchmarks ran serially.

## Fusing expression evaluation with reduction

`FusionGraph::compile_sum` emits a first pass that evaluates an expression in
registers and reduces to workgroup partials. `ComputeProgram::fused_sum` folds
those partials with the existing reduction. `fused_sum_into` reuses an existing
scalar output, including resetting it for empty input. Ordinary fused maps and
fused sums share graph analysis, WGSL expression generation, binding validation
and the bounded compiled-kernel cache.

Typed `Predicate` handles support six comparisons and `select` in both maps and
sums. For a conditional sum, values flow directly into the reduction without a
mask, scan or compacted array. Both select branches are evaluated and must obey
their arithmetic domains. Use stable compaction when the selected sequence is
needed by later work.

`bench_fused_sum` compares an already-fused map followed by the existing sum with
the new fused map/reduce. Three warmups, 21 rotated samples per path, resident
arrays, and a four-byte final readback. Compilation/preparation excluded; every
sample is checked against a CPU reference accumulating into f64. GPU arithmetic
is f32 and summation order changes; no compensated-summation claim is made.

| Workload / elements | Fused map then sum | Fused map/reduce | Including input upload, fused |
| --- | ---: | ---: | ---: |
| Dot / 1,000,003 | 0.640 ms | 0.271 ms | 0.914 ms |
| Squared distance / 1,000,003 | 0.622 ms | 0.249 ms | 1.012 ms |
| Squared distance / 4,000,003 | 1.981 ms | 0.361 ms | 3.196 ms |
| Nonlinear nine-operation expression / 4,000,003 | 1.781 ms | 0.431 ms | 3.425 ms |
| Conditional sum / 4,000,003 | 1.714 ms | 0.351 ms | 3.415 ms |

These workloads broadcast the second input scalar. The four-million conditional
case also compares the current optimized `compare → compact → sum` path:
3.219 ms. That path produces a stable selected sequence, while conditional sum
only produces its scalar aggregate; they meet different downstream needs.
At 4,000,003 elements, the new path removes the 16,000,012-byte mapped array and
uses 7,816 bytes of first-stage partials plus small reduction tails. At 4,097
elements the new path is roughly neutral (some workloads slightly slower) and
CPU remains faster. Upload cost can dominate after reduction is optimized.

[Full paired measurements and samples](../../crates/compute-core/benchmarks/fused-sum-metal.txt).

## Scan and stable compaction

The retained scan computes four adjacent values per lane in registers, then
scans lane totals with the existing work-efficient shared tree. A 256-lane group
covers 1,024 values. Compaction combines local element offsets with scanned block
offsets in its scatter; this removes the full-size offset-add pass. Scatter,
count publication and zero-tail writes share one dispatch because their output
write ranges are disjoint. All-zero/all-one masks avoid unnecessary per-element
scatter reads. Public old shader strings retain their original contracts.

Frozen old shaders/planning are benchmarked in the same shared-pass execution
and with identical readback. Fifteen paired samples after warmup, masks with
0%, 50% and 100% selection, 4K/1M/4M inputs, full output checked for every case.

| Workload / elements / selected | Prior implementation | Retained implementation |
| --- | ---: | ---: |
| Full compact readback / 1,048,576 / 50% | 0.983 ms | 0.770 ms |
| Compare → compact → sum / 1,048,576 / 50% | 0.943 ms | 0.628 ms |
| Full compact readback / 4,194,304 / 50% | 3.896 ms | 3.086 ms |
| Compare → compact → sum / 4,194,304 / 50% | 1.546 ms | 1.118 ms |

Across the measured large cases, full compact improves 1.25–1.43× and the scalar
pipeline improves 1.38–1.60×. 4K cases are effectively neutral, with a small
negative result preserved in the report. Tests cover u32 wrapping, nonzero-mask
normalization, stable order, tail reset, repeated execution, prefix views,
1,024-element boundaries and logical blocks above physical dispatch limits.

[Method and limitations](../../crates/compute-core/benchmarks/scan-compaction-metal.md),
[paired CSV](../../crates/compute-core/benchmarks/scan-compact-metal.csv).

## Matrix kernel selection

Six kernel approaches were explored across 23 square, odd, tall and narrow
shapes. Retained Metal substitutions use direct scalar products for bounded
small/narrow shapes, aligned vec4 tiles for suitable divisible-by-32 shapes,
and cooperative inner-axis reduction for at most 1,024 outputs with K≥256.
Other shapes and backends retain the general 32×32 tiled kernel. The policy is
encapsulated in the matrix module and leaves matrix validation/composition APIs
unchanged. Selection changes summation order; results are checked against f64.

Final measurements compare the actual production program against the previous
32×32 kernel, with the same full output readback. At least 250 ms of alternating
warmup, then 17 rotated samples. Compilation and allocation are excluded.

| M×K×N | Previous tiled32 | Production selection |
| --- | ---: | ---: |
| 127×259×193 | 0.222 ms | 0.196 ms |
| 512×512×512 | 0.427 ms | 0.356 ms |
| 1024×1024×1024 | 2.106 ms | 1.803 ms |
| 256×1024×256 | 0.516 ms | 0.358 ms |
| 4096×256×8 | 0.222 ms | 0.172 ms |
| 32×1024×32 | 0.507 ms | 0.121 ms |
| 8×2048×8 | 0.674 ms | 0.119 ms |

Blanket direct selection was rejected after regressions on 511×63×257 and
16×512×4096. Extra scalar-aligned/direct-vector variants were rejected for
inconsistent gains. The production policy leaves those regression shapes on
the original kernel. These measurements establish no universal Metal crossover
or vendor-BLAS comparison.

[Kernel selection and rejected candidates](../../crates/compute-core/benchmarks/matmul-research.md),
[final production samples](../../crates/compute-core/benchmarks/matmul-production-metal-round3.txt).

## GPU timestamp profiling and validation failure found during research

`GpuContext::with_timestamps` explicitly requests timestamp-query capability;
ordinary contexts keep their previous feature set. `ComputeProgram` and
`ComputeBatch` can record into a supplied compute pass. `GpuTimer` measures that
pass on the device and returns a separate asynchronous timing ticket.

Alternating real map/reduce programs exposed incorrect measurement when query
resolution shared the measured command buffer: a short pass could report the
preceding long pass's duration, even exceeding the enclosing host interval.
Allocating fresh queries alone produced zero/stale samples. A targeted
1-versus-128-dispatch regression reproduced the fault; a constant-workload
"nonnegative elapsed time" check had not detected it.

The retained timing path owns an independent query set per ticket and waits for
measured submission completion before encoding/submitting resolve+readback.
This adds profiling turnaround, outside the reported compute-pass interval.
It does not add submissions to ordinary unprofiled compute programs. Raw encoder
validation can be deferred until finish, so `GpuTimer::finish` captures that
error before submission. Unsupported features, invalid counter values, backwards
samples, unsubmitted/cancelled/consumed tickets and timeout remain explicit.

[Invalid shared-query evidence](../../crates/gpu-compute/benchmarks/timestamps-shared-query-invalid.txt),
[invalid same-submission resolution with fresh queries](../../crates/gpu-compute/benchmarks/timestamps-same-submission-invalid.txt).
The invalid measurements are retained for diagnosis and excluded from all
performance claims.

### Validated compute-pass timestamps

The corrected profiler was exercised with 1/128-dispatch workloads in both
orders, mixed-duration passes in the same encoder, reverse readback order,
cancellation, unsubmitted tickets and timeout recovery. Every measured interval
fit inside the enclosing host interval. Mixed-pass long/short ratios were 126×
and 281×; the old lagged sampling cannot pass these checks.

`profile_fused_sum` then compared squared differences of **two full resident
arrays**, with at least 200 ms of warmup and 31 paired samples. Both programs
produce the same sum checked against f64. These input shapes differ from the
broadcast-scalar benchmark above.

| Elements | Fused map + separate sum, GPU | Fused map/reduce, GPU | Speedup |
| ---: | ---: | ---: | ---: |
| 4,097 | 9.667 µs | 8.708 µs | 1.11× |
| 1,000,003 | 115.417 µs | 19.334 µs | 5.97× |
| 4,000,003 | 459.167 µs | 61.458 µs | 7.47× |

The four-million case's full **profiling** wall times were 0.753 and 0.346 ms.
Those include the extra query-resolution submission and cannot substitute for
unprofiled latency. GPU timestamps exclude host encoding, submission wait and
readback outside the measured pass. Metal reported a timestamp period of 1 ns.

[Corrected pass timings and samples](../../crates/compute-core/benchmarks/fused-sum-gpu-timestamps-metal.txt),
[strengthened profiling tests](../../crates/gpu-compute/benchmarks/timestamps-completion-resolve-tests-metal.txt).

## Reproduction

Run each benchmark separately from other GPU work:

```sh
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example bench_fused_sum
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example profile_fused_sum
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example bench_scan_compact
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example bench_matmul_candidates -- --extended
```

The timestamp example requires timestamp support and fails explicitly if the
adapter cannot provide it. All gains above are measured on this M4 Max; Vulkan,
DX12, browser WebGPU and NVIDIA execution are outside this qualification.

## Final integration checks

- **196 tests passed** across compute, platform, math and raster, requiring both
  an actual GPU and timestamp capability. This includes arithmetic/predicate
  parity, scan/compaction contracts, selected matrix paths, shared-pass ordering
  and the strengthened timestamp lifecycle/regression tests.
- All-target Clippy passed for `compute-core` and `gpu-compute` with `-D warnings`.
- Strict rustdoc passed for compute, platform and math. All-feature consumer
  checks passed for math, SDF, geometry and photogrammetry.
- GPU dependency directions and CPU-only math dependency rules passed the
  architecture checker. Scoped whitespace checks passed.
- Existing workspace warnings remain for the `wgsl_export` binary name and a
  geometry-bridge doc comment on a macro invocation.

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 cargo test --offline \
  --manifest-path crates/Cargo.toml -p gpu-compute -p compute-core \
  -p osv-math -p raster-core --features osv-math/gpu --lib --tests
```

[Retained source hashes](../../crates/compute-core/benchmarks/shader-research-source-manifest.json)
identify the implementation and benchmark drivers in this working tree. The
matrix study also preserves hashes for its individual experimental candidates.

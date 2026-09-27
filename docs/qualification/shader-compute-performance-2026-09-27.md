# Shader compute execution: measured improvements

Date: 2026-09-27. Native macOS, Apple M4 Max (40 GPU cores), Metal, release builds.
These are host wall times including command submission, completion and readback.
They are not GPU timestamps or measurements of the viewer's frame rate. GPU
benchmarks were scheduled serially. Existing unrelated desktop work can still
cause timing variation; medians describe these workloads on this machine.

## Fused arithmetic

`compute-core/examples/bench_fusion.rs` compares the existing nine-operation
`affine → sin → square → affine → cos → square → affine → sin → abs` chain with
one generated shader. The baseline already uses the new shared compute pass.
Both paths read every output element and reuse prepared buffers/bindings.
Compilation (46.090 ms in this run) and preparation are outside execution timing.
Three warmups and fifteen samples per path, rotated order. Every output matches
a single-loop f32 Rust reference within 3e-6 absolute error.

| Elements | Existing chain, resident | Fused, resident | Chain incl. upload | Fused incl. upload | CPU loop |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,097 | 0.230 ms | 0.154 ms | 0.251 ms | 0.193 ms | 0.017 ms |
| 1,000,003 | 2.436 ms | 0.930 ms | 3.131 ms | 1.666 ms | 3.832 ms |

At 1,000,003 elements the resident path is 2.62× faster and upload-inclusive
execution is 1.88× faster than the existing GPU chain. Dispatches fall from nine
to one. Eight intermediate arrays (32,000,096 bytes) disappear. Both retain a
4,000,012-byte final output. CPU is preferable for the small tested workload.
The compiler keeps operation order; GPU transcendental rounding and FMA
contraction preclude a general bitwise-equivalence promise.

Implemented contracts: common-expression reuse, dead-node elimination, bounded
pipeline caching, multiple outputs, scalar broadcasting in either operand,
constant-only graphs, empty output, distinct reusable output buffers, checked
owners/shapes/aliases and inspectable generated WGSL. Tests cover every supported
unary/binary operation, uniform padding with seven live inputs, repeated readonly
bindings, reduction consumers and dispatch-limit overflow via grid strides.

## Shared compute pass

`compute-core/examples/batch_passes.rs` uses identical kernels and buffers and
compares separate compute passes with one pass containing all dispatches. Three
warmups and 21 paired samples. No timed uploads; every run reads a four-byte final
value. Dependency tests include write/read/overwrite, reductions, comparison,
scan, compaction and queued readbacks awaited in reverse order.

| Elements / dispatches | Separate passes | Shared pass | Encode+finish, separate | Encode+finish, shared |
| --- | ---: | ---: | ---: | ---: |
| 4,096 / 4 | 195.08 µs | 163.75 µs | 18.58 µs | 7.29 µs |
| 4,096 / 32 | 827.17 µs | 350.29 µs | 163.83 µs | 17.67 µs |
| 4,096 / 128 | 2,748.67 µs | 966.21 µs | 858.33 µs | 57.67 µs |
| 1,000,003 / 32 | 1,991.67 µs | 1,767.67 µs | 237.12 µs | 23.38 µs |

## Dense matrix multiplication

`compute-core/examples/bench_matmul.rs`: fixed 32×32 shared tiles, 64 lanes, 4×4
register accumulators per lane. Compared with one-output-per-thread naive WGSL
and single-threaded, cache-blocked Rust CPU loops with contiguous inner access.
The CPU is not a vendor BLAS baseline. Both GPU paths include full output readback
from resident inputs. Preparation excluded; two warmups and seven interleaved
samples. All outputs checked against an independent f64 reference outside timing;
largest absolute GPU error was 3.160e-5.

| M×K×N | Tiled GPU | Naive GPU | CPU |
| --- | ---: | ---: | ---: |
| 64×64×64 | 0.293 ms | 0.259 ms | 0.073 ms |
| 127×259×193 | 0.669 ms | 0.446 ms | 1.836 ms |
| 512×512×512 | 1.179 ms | 2.365 ms | 37.736 ms |
| 1024×1024×1024 | 4.969 ms | 6.520 ms | 301.425 ms |

A 16×16/2×2-register candidate lost to naive WGSL on 1024³ and was discarded.
The retained 32×32 kernel improves both larger measured shapes, while the small
and rectangular cases expose tile overhead. No automatic CPU/GPU placement or
universal tiled-kernel speedup is claimed. The API composes matrix products with
array operations/reductions without intermediate host copies. Tests cover odd
shapes, zero-inner products resetting reused storage, exact shape and alias
validation, repeated execution, matrix chains, and >65,535 tiles.

## Reproduction

From the repository root:

```sh
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example bench_fusion
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example batch_passes
cargo run --release --offline --manifest-path crates/Cargo.toml -p compute-core --example bench_matmul
cargo run --release --offline --manifest-path crates/Cargo.toml -p osv-math --features gpu --example bench_recorded_nearest
```

Run performance measurements serially, separately from the GPU test suite.

## Nearest-neighbor cooperative reduction

The retained shader assigns one 64-lane workgroup to each query. Lanes scan
separate target subsequences and reduce `(distance, index)`, preserving exact
f32 first-index ties and sentinel behavior. Recorded and synchronous calls share
this dispatcher. The synchronous adapter also combines the kernel and both
output readbacks into one submission instead of three.

Apple M4 Max (40 GPU cores), Metal, release build: seven warmup rotations and
45 measured rotations, identical resident inputs and both full output arrays.
All paths pass exact CPU-output checks on binary-fraction inputs before timing;
adversarial tests separately cover ties, near ties, overflow and partial groups.
Measurements are host latency including completion/readback, not GPU timestamps.

| Queries × targets | Resident scalar → recorded, ms | Upload scalar → recorded, ms | Sync old transport → selected API, ms |
| --- | ---: | ---: | ---: |
| 256 × 512 | 0.411458 → 0.171166 | 0.279709 → 0.195333 | 0.337583 → 0.182291 |
| 4096 × 4096 | 0.592708 → 0.218625 | 0.639083 → 0.256042 | 0.766708 → 0.322959 |
| 16384 × 8192 | 1.440791 → 1.024625 | 1.530792 → 1.151083 | 1.974583 → 1.524500 |

Actual recorded speedup: **2.40× / 2.71× / 1.41×**. The synchronous comparison
reconstructs the old scalar three-submission transport with cached buffers and
full f64-to-f32 input conversion; omitted legacy uniform/error-scope overhead
favors that baseline. CPU f64 medians were 0.125 / 17.165 / 143.793 ms, so CPU
remains faster at the smallest shape.

Default cooperative selection is bounded to Metal, 512..8192 targets and
256..(2 * targets) queries. The original scalar shader remains elsewhere, and
CPU/CUDA placement is unchanged. This is one M4 Max measurement, not a universal
Metal crossover claim. Shared-target tiling and 256-lane cooperative candidates
were measured; only the selected 64-lane shader was retained alongside reference.

An initial 15-sample production run had noisy tails and a large-case raw/recorded
gap (1.130/1.478 ms); it did not prove that production case faster. The one longer
45-sample repeat resolved the gap (1.048/1.025 ms), with both below scalar
1.441 ms. The reported gains use actual production recording from that repeat.

[Final raw CSV with p90](../../crates/math-core/benchmarks/nearest-metal-production.csv),
[initial production CSV](../../crates/math-core/benchmarks/nearest-metal-first-production.csv),
[full method and candidate evidence](../../crates/math-core/benchmarks/nearest-neighbor-metal.md).
Reproduce with `NN_BENCH_REPEATS=45 NN_BENCH_WARMUPS=7 cargo run --offline --release --manifest-path crates/Cargo.toml -p osv-math --features gpu --example bench_recorded_nearest`.

## Integration validation

- **177 tests passed** across `gpu-compute`, `compute-core`, `osv-math` and
  `raster-core`, with `COMPUTE_REQUIRE_GPU=1` on native Metal.
- **68 CPU-only math tests passed**, without the GPU feature.
- **6 focused GPU photogrammetry tests passed**, covering shader entry/layout,
  matching parity, cached buffers, empty grids and failed readback handling.
- `compute-core` and `gpu-compute` all-target Clippy passed with `-D warnings`.
  Math's focused Clippy check passed with three pre-existing lint families
  allowed (`chunks_exact_to_as_chunks`, `needless_range_loop`, `needless_borrow`);
  those exceptions apply to older bounds/moments/distance/nearest-two code.
- Strict rustdoc passed for compute, platform and math; all-feature checks passed
  for math, SDF, geometry and photogrammetry. GPU dependency directions and the
  CPU-only dependency contract passed `scripts/check-gpu-architecture.py`.
- Existing workspace warnings remain: the `wgsl_export` binary naming warning
  and a geometry-bridge doc comment attached to a macro invocation.

No Vulkan, DX12, browser WebGPU or NVIDIA execution is established by these
native Metal tests. SIMD/BLAS CPU libraries were not benchmarked.

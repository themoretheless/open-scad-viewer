# Photogrammetry performance qualification — 2026-09-09

Integrated Rust optimizations preserve the existing reconstruction while reducing computation and temporary allocation. No dependencies, image/feature/depth/hypothesis budgets, quality thresholds or presets were added or reduced. The actual viewer was rebuilt and its generated WASM is byte-identical to the qualified module.

## Actual browser result

Chrome 155.0.8040.2; real JPEG import with EXIF, a fresh browser context per run, one Chrome process, one warmup plus three measured repetitions per version and dataset, alternating order. Both host snapshots are identical except the photogrammetry WASM. The timer covers the Reconstruct action through completion; file selection/import happens before it. No other qualification builds or benchmarks ran concurrently during this window.

| Input and preset | Before, s | After, s | Time reduction |
| --- | ---: | ---: | ---: |
| shell12 | 6.486 | 4.978 | 23.2% |
| monstree6-slanted | 6.357 | 4.348 | 31.6% |

All 16 browser runs produced identical PLY bytes within their dataset and identical geometric reports after excluding timing/work diagnostics. The standalone WASM comparisons below additionally compare the complete decoded diagnostics. Rotation through multiple full turns, fit view, point/surface display, PLY/JSON downloads, invalid-calibration rejection and cancellation passed. Cancelling a running dense Worker took 24.48 ms in one functional check: all six photos and sparse output remained, and no late surface was published.

The photo panel has no internal horizontal overflow at 390 px. Existing scene panels still overlap part of the mobile page; this is not a qualification of the entire application’s mobile layout.

## Controlled native and WASM comparisons

Environment: Apple M4 Max; macOS-26.7-arm64-arm-64bit-Mach-O; rustc 1.98.0-nightly (57d06900f 2026-05-27); Node v22.23.2. Each version/case uses one warmup and three measured fresh processes in rotating order. Timed comparison jobs ran sequentially after the browser window. This controls our own workload, not all OS background activity; medians from three repetitions are observations, not a statistical confidence interval.

Native wall time includes PPM input, sparse and dense computation, and unchanged PLY output. It is not the UI timer. WASM time uses the real TypeScript adapter and includes module initialization, input transfer, reconstruction and result decoding; it excludes PPM loading, TypeScript import, JPEG decoding and UI.

| Runtime / input / mode | Before, s | After, s | Time reduction |
| --- | ---: | ---: | ---: |
| Native / shell6 / default | 1.700 | 1.100 | 35.3% |
| Native / shell12 / default | 3.894 | 2.617 | 32.8% |
| Native / monstree6 / default | 2.875 | 2.015 | 29.9% |
| Native / shell6 / slanted | 3.628 | 2.233 | 38.4% |
| Native / shell12 / slanted | 7.730 | 5.033 | 34.9% |
| Native / monstree6 / slanted | 5.280 | 3.576 | 32.3% |
| Node WASM / shell6 / default | 2.047 | 1.198 | 41.5% |
| Node WASM / shell12 / default | 4.409 | 2.753 | 37.6% |
| Node WASM / monstree6 / default | 3.303 | 1.989 | 39.8% |
| Node WASM / shell6 / slanted | 4.192 | 2.342 | 44.1% |
| Node WASM / shell12 / slanted | 8.971 | 5.138 | 42.7% |
| Node WASM / monstree6 / slanted | 5.897 | 3.495 | 40.7% |

Every native sparse/surface PLY and all three dense work counters match exactly across each input/mode group. Every WASM sparse response, surface response including diagnostics, and complete report has the same decoded JSON SHA256. All 20 final analytic combinations (five scenes, two patch radii, both estimators) match every coordinate bit, color, triangle and diagnostic.

## Separate compiler profile contribution

At the time of these measurements the workspace used opt-level="s" and LTO,
with opt-level=3 overrides for the two photogrammetry packages. The following
comparison isolates that historical release profile: combined-size has all
accepted source changes but retains the old size profile. Current package
names/settings are authoritative in `crates/Cargo.toml`: the workspace still
uses `s`; `photogrammetry-core`, `photogrammetry-ffi` and `polygon-core` use `3`,
and the decompression bootstrap packages use `2`. A workspace-wide switch to
`z` was evaluated but not retained; see the
[ModelGraph profile control](design/modelgraph-size-profile-2026-09-20.md).

| Node WASM default / input | Original, s | Source changes only, s | Source changes + opt3, s |
| --- | ---: | ---: | ---: |
| shell6 | 2.047 | 1.894 | 1.198 |
| shell12 | 4.409 | 4.106 | 2.753 |
| monstree6 | 3.303 | 2.879 | 1.989 |

| WASM artifact | Before, bytes | After, bytes |
| --- | ---: | ---: |
| Raw module | 588,227 | 549,881 |
| Generated compressed/base64 TypeScript source | 221,292 | 245,984 |

The generated source size is an embedding/build artifact, not the final HTTP transfer size. No new WASM runtime imports are required.

## Allocation traffic and memory

Allocation profiling uses a separate instrumented executable. Its atomic counters affect time, so its elapsed times are not used for speed claims. Allocation calls and cumulative allocated bytes measure traffic; peak live Rust bytes and whole-process RSS are distinct. Fewer allocations do not imply an equivalent reduction in peak RAM.

| Sparse phase | Allocation calls before → after | Cumulative allocated MB before → after | Peak live MB before → after |
| --- | ---: | ---: | ---: |
| shell6 | 1,231,860 → 78,792 | 406.46 → 337.41 | 38.69 → 38.43 |
| shell12 | 1,918,957 → 76,819 | 787.21 → 672.22 | 52.19 → 51.67 |
| monstree6 | 1,921,336 → 29,584 | 517.74 → 390.20 | 44.04 → 43.52 |

| Default runtime / case | Peak process RSS MB before → after | WASM linear memory MB before → after |
| --- | ---: | ---: |
| Native / shell6 | 54.62 → 57.31 | n/a |
| Node WASM / shell6 | 136.69 → 139.62 | 41.81 → 44.30 |
| Native / shell12 | 80.76 → 78.54 | n/a |
| Node WASM / shell12 | 176.14 → 182.91 | 55.71 → 60.69 |
| Native / monstree6 | 59.95 → 63.06 | n/a |
| Node WASM / monstree6 | 156.47 → 160.37 | 46.79 → 49.61 |

RSS includes the process and harness; Node RSS also includes V8. Linear memory is the allocated WASM page extent after reconstruction, not live Rust heap or browser-wide memory.

## Optimization rationale

The changes reduce repeated arithmetic, temporary geometry objects and small heap allocations while retaining the existing reconstruction policy. They reuse the repository's Rust kernel, WASM adapter and MGV1 codec; no dependency or runtime import was added. Source-level comparisons used the then-production size profile (`opt-level="s"`, LTO). The historical integrated build additionally selected `opt-level=3` for the two photogrammetry packages. The controlled measurements above separate source improvements from this compiler change; they do not measure later package-profile changes.

### Accepted changes and architectural boundaries

Feature extraction selects retained corners across the Harris pyramid before computing descriptors. This avoids descriptors for corners subsequently suppressed or excluded by the feature limit. Pyramid samples and selected feature order remain unchanged. Gaussian orientation and descriptor weights are computed once per extraction, with the same values and accumulation order. Matching combines its two rejection thresholds into one maximum; it preserves the squared-distance reduction and rejection position.

Camera refinement and small linear systems use bounded stack arrays. Jacobians no longer allocate two vectors per observation and iteration, and normal-equation rows are accumulated without allocating each row. Existing pivoting, random sequences, solver stopping rules and numerical routines remain authoritative. This principally reduces allocator traffic; its effect on elapsed time is smaller.

Dense sampling computes NCC sum, squared energy and covariance while visiting each source pixel. Both estimators share one private accumulator instead of writing a 25-value scratch array and traversing it three more times. Each reduction retains its original floating-point order and signed-zero identity. Successful-sample counters still include pixels read before a patch is rejected.

The WASM response adapter borrows geometry and serializes it directly into MGV1. It first measures the complete response, including keys, envelope and diagnostics, then allocates the exact output capacity. Finite floating-point and integer triples are appended as 32-byte records; arbitrary metadata continues through the shared codec. Sparse results borrow point fields, and diagnostics are stored once and borrowed for subsequent responses.

These boundaries apply SOLID and DRY through existing responsibilities: the kernel owns reconstruction, the session owns persistent results, and the response module owns protocol adaptation. Numerical solvers and the NCC accumulator provide shared implementations. The adapter shares transport limits and delegates metadata encoding to the codec. Existing public options, JavaScript interfaces, ownership rules and error behavior remain intact. Review confirmed that session results are published before a transport-limit failure in both the original and optimized paths.

### Independent comparisons after each step

All figures here are component or isolated-candidate evidence, collected while other agents could be active. They are not the final integrated native or browser results.

- **Features and matching:** alternating reference/candidate runs compared exact feature and match fingerprints after each change. On shell12, deferred descriptors reduced extraction from 196.39 to 170.82 ms; the maximum cutoff reduced matching from 1194.74 to 1093.45 ms in its separate comparison. The Gaussian-weight addition had a small, noisy incremental benefit. All four feature profiles were checked. See [feature qualification](qualification/photogrammetry/performance-2026-09-09/sparse.md).
- **Camera workspaces:** baseline, feature-only and final candidates produced identical sparse and dense PLY bytes across 36 pipeline runs. Separate qualification covered 3,000 pivoted systems, singular cases, streamed normal equations, mixed focal lengths and all six geometry profiles. For the full sparse candidate on shell12, allocation calls fell from 1,918,957 to 76,819, whereas peak live Rust allocation changed only from 52.19 to 51.67 MB. Allocation traffic is not peak RAM.
- **Dense NCC:** 48 process runs covered three real datasets, two estimators, alternating order, one warmup and three measured repetitions. Every corresponding sparse/surface PLY and work counter matched. Twenty analytic scene/radius/estimator combinations also matched every coordinate bit, color, triangle and diagnostic value. The shell12 default candidate comparison was 3827.5 to 3662.3 ms for the full process. Subtracting sparse time provides only an approximation that still includes input/output overhead. See [dense qualification](qualification/photogrammetry/performance-2026-09-09/dense.md).
- **Transport:** borrowed encoding was tested first, followed by packed triple writes. Each stage matched the original wire bytes. In the latter comparison, shell12 serialization took 2.381 to 0.301 ms and allocated 17 objects instead of 45,154. The larger stress fixture is synthetic transport data, not a high-resolution scan. Unchanged JavaScript decoding matched all three fixtures. Tests cover nonfinite-as-null behavior, integer tags, missing cameras, Unicode, empty geometry and complete byte/item/depth limits. See [transport qualification](qualification/photogrammetry/performance-2026-09-09/transport.md).

### Rejected experiments and measurement limits

Blockwise matching rejection and explicit eight-lane unrolling preserved outputs but offered no repeatable gain; shell12 matching regressed by about 3%. Explicit SlantedPlane ray/offset caching and shared initialization depths also preserved outputs, but shell12 elapsed time changed by less than 0.2%, with no improvement in the post-sparse approximation. These variants were reverted; their comparison artifacts remain available.

Image resolution, feature/hypothesis budgets, thresholds, visibility, cancellation checkpoints and reconstruction modes were preserved. Equality on frozen inputs establishes regression protection, not improved absolute accuracy. The real datasets have no physical reference, and analytic scenes use arbitrary units. Canon R8+iPhone accuracy in millimeters remains unverified.

Serialization speedups apply only to serialization. Allocation instrumentation affects timings; cumulative allocated bytes, peak live Rust allocation, process RSS and browser memory describe different quantities. The controlled integrated measurements above identify native versus WASM execution and include the compiler-profile and artifact-size tradeoff.

## Actual viewer verification and limits

97 Rust tests passed in the actual workspace (74 kernel, 5 fixture/evaluator example tests, 15 WASM adapter, 3 shared codec). One explicit transport microbenchmark is intentionally ignored by ordinary tests. All 39 photogrammetry Vitest tests, Vue and MCP type checks, the production Vite build and distribution verification passed. The actual source hashes and raw/generated WASM hashes match the qualified candidate. Package and lockfile dependency contents were preserved.

Actual build/type/distribution checks used Node v26.8.1; the isolated WASM timings above used v22.23.2. Two additional functional browser runs served the actual viewer distribution on port 5203. The served HTML matched the actual dist/index.html SHA256, and both PLY files matched the qualified current version exactly. These two smoke runs add integration evidence, not additional comparative timing claims.

These changes preserve the existing quality; they do not close holes, remove every spurious fragment, add an automatic calibration solver or establish physical millimeter accuracy. The Canon R8+iPhone capture setup has not been qualified against a measured physical reference. SlantedPlane remains experimental; its known thin-detail tradeoffs are unchanged.

## Reproduction and evidence

The frozen source snapshots, input paths/hashes, comparison drivers and complete PLY/binary oracles remain in `/Users/themoretheless/Documents/Sources/scan/optimization/photogrammetry-2026-09-09`. Run `python3 run-final.py` there to repeat the sequential native/WASM/allocation/analytic suite. `run-native.py`, `run-wasm.py`, `browser-final.mjs` and the stage reports document narrower runs. The previous input manifest is `/Users/themoretheless/Documents/Sources/scan/implementation/photogrammetry-2026-09-09/inputs.json`.

Portable report records are in [performance-2026-09-09](qualification/photogrammetry/performance-2026-09-09/). They include all measured repetitions, output SHA256 values, exactness checks, compiler profiles, allocation measurements, browser manifests, cancellation evidence and actual-workspace check results. Prior quality qualification remains in [photogrammetry-improvements.md](photogrammetry-improvements.md).

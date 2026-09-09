# Dense photogrammetry optimization — 2026-09-09

Accepted change: accumulate source patch NCC statistics during sampling. Previously each hypothesis wrote a 25-value scratch buffer, then traversed it for sum, squared energy and covariance. Both dense modes now share one private Correlation accumulator. Each reduction keeps its original floating-point order, signed-zero identity and threshold. Successfully sampled pixels still increment the u64 counter immediately, including before an eventual partial-patch rejection.

Only `src/dense/estimation.rs` and `src/dense/plane.rs` change. No new dependencies, unsafe code, public API, quality preset, depth resolution, hypothesis count, visibility rule, cancellation checkpoint or allocation budget change. The source scratch buffer is removed; heap allocation policy is unchanged. Peak resident memory was not measured by this subtask.

## Comparative tests after accepted step

Production profile: opt-level="s", lto=true. Same frozen PPM pixels and focal values as prior qualification; all input SHA256 values verified. Each of 6 real input/mode combinations has one warmup and 3 measured runs for each binary, alternating order. **All 48 runs have identical sparse/surface PLY bytes and all 3 work counters within their input/mode group.** The geometric algorithm and sampled work are unchanged.

| Input | Dense mode | Process wall ms, median | Reduction | Post-sparse approximation ms |
| --- | --- | ---: | ---: | ---: |
| shell6 | baseline | 1717.2 → 1585.7 | 7.7% | 1185.0 → 1059.8 |
| shell6 | slanted | 3599.0 → 3452.7 | 4.1% | 3074.2 → 2928.4 |
| shell12 | baseline | 3827.5 → 3662.3 | 4.3% | 2185.2 → 1994.3 |
| shell12 | slanted | 7544.3 → 7416.2 | 1.7% | 5924.2 → 5755.7 |
| monstree6 | baseline | 2879.8 → 2748.9 | 4.5% | 1617.2 → 1497.6 |
| monstree6 | slanted | 5440.7 → 5328.5 | 2.1% | 4158.6 → 3957.8 |

Timing is indicative under parallel agent activity. Post-sparse is process wall minus the existing sparse-computation timer; it includes dense work and remaining input/output/process overhead, and is not an isolated dense timer. Root will run the final controlled sequence. No claim about WASM speed or physical accuracy follows from these native timings.

The analytic harness covers 5 scenes (0°, 15°, 30°, 60°, and 30° with thin ribbon/occluder), radii 1 and 2, and both dense estimators: **20 combinations match bit-for-bit**. The binary oracle includes every f64 coordinate bit, every color, triangle and every DenseDiagnostics value, with no decimal rounding. Thus those known-shape outputs, their accuracy/completeness and work counts are unchanged. Analytic units are arbitrary.

25 dense-related tests passed, including two new regressions: 2,000 randomized 9/25-sample NCC comparisons against the old independent reductions (including near-blank rejection), and partial-patch rejection / nonfinite projection / u64 counter crossing u32::MAX. Existing cancellation-after-initialization, all hypothesis budgets, memory planning, depth discontinuity and visibility checks pass.

## Evaluated and rejected second step

Explicitly cached SlantedPlane normalized reference rays and patch offsets, and reused identical per-patch depths during frontoparallel initialization. All 20 analytic outputs and shell12 outputs remained bitwise identical, but shell12 slanted median process time was 7514.9 → 7506.3 ms (<0.2%); the post-sparse approximation did not improve. This extra complexity is **reverted**. The `precomputed-target` benchmark binary and reports remain available; candidate source contains only the accepted NCC change.

## Reproduction and artifacts

- `candidate.json`: exact changed source hashes, compiler, frozen input verification and comparisons.
- `fused-ncc/results.json`: 48 raw process runs with binary/output SHA256 values and work counters; per-run logs/PLY are adjacent.
- `analytic-fused-ncc-results.json`: 20 binary geometry/diagnostic comparisons. Harness: `fixture-harness.rs`; output bytes: `analytic-{baseline,fused-ncc}`.
- `tests-fused.log`: accepted candidate dense tests; `tests-final.log`: final full kernel library tests.
- `precomputed-exploratory/results.json`, `analytic-precomputed-results.json`: rejected second candidate evidence.
- `compare-dense.py LABEL CASES MODES REPEATS`: repeat the native process comparison; `DENSE_BENCH_VARIANTS` can provide a JSON map of names to frozen binaries. Default inputs come from the previous qualification manifest.

No actual open-scad-viewer source was edited by this subtask. Root must merge these two files, rebuild the actual WASM and validate the integrated host.

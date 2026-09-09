# Sparse photogrammetry optimization qualification

Only four production source files are candidates for integration. See `changed-paths.json` for before/after SHA256 values. No dependency, quality threshold, image/feature/hypothesis budget, random sequence, solver stopping rule, or public camera/feature interface changed.

## Measured changes

1. **Deferred descriptors.** Corner score/location selection now finishes across all Harris levels before computing descriptors. The old code computed descriptors for corners later suppressed by a finer level or excluded by the final feature limit. The retained grayscale levels preserve precisely the original samples. Unused final downsampling was removed.
2. **One distance cutoff per match pair.** `distance > second_best_a && distance > best_b` becomes `distance > max(second_best_a,best_b)`. The scalar squared-distance accumulation and rejection position are identical; each dimension needs one comparison instead of two.
3. **Fixed Gaussian sample weights.** The same 169 orientation and 256 descriptor weights are computed once per extraction. Exact values and accumulation order are preserved. The isolated additional native timing benefit is small/noisy; this removes repeated computation without a persistent cache.
4. **Bounded camera workspaces.** Camera Jacobians use `[f64;6]` instead of allocating a `Vec` twice per observation and iteration. Pivoted 3/6/8 systems use stack arrays. Essential/DLT normal equations consume stack rows directly instead of allocating every row. The existing private numerical functions remain the single implementation; pivoting and arithmetic order are unchanged. Random subsets preallocate their known sample capacity.

Explicitly rejected: blockwise scalar early rejection, including explicit eight-lane unrolling. They preserved outputs but gave no repeatable speedup and regressed shell12 matching by about3%; their source snapshots/raw results remain under `step1*` for audit only.

## Per-step comparisons

Each feature microbenchmark includes the frozen and candidate implementations in the same production opt-s/LTO binary, alternates order, and uses one warmup plus three measured repetitions. All feature and accepted-match coordinates/descriptor values/distances/ratios are compared by exact IEEE-bit fingerprints. Times are milliseconds and indicative while other agents were running.

| Change | shell6 before→after | shell12 before→after | monstree6 before→after |
|---|---:|---:|---:|
| Deferred descriptors, extraction |97.69→84.28|196.39→170.82|250.01→156.52|
| Maximum cutoff, matching |258.72→239.01|1194.74→1093.45|854.32→794.34|
| All feature changes, extraction |98.21→86.02|193.32→167.35|240.79→150.65|
| All feature changes, matching |257.98→241.32|1171.72→1079.08|851.95→784.25|

See `feature-step-summary.json` and individual `step*-*.ndjson` records. The weights row is compared against the original reference, not an isolated weights-only effect.

## Full pipeline equivalence and allocation comparisons

`reports/compare_pipeline.py` ran baseline, feature-only checkpoint, and final candidate in rotating order: warmup plus three fresh-process repetitions for each real case. Every sparse PLY and dense surface PLY was **byte-identical** across all36 runs. Camera counts, point counts and RMSE were identical. These datasets lack a physical reference; equality proves preservation, not absolute accuracy.

| Case | Sparse time baseline→features→final, ms | Registered | Points | RMSE, px |
|---|---:|---:|---:|---:|
| shell6 |527.19→492.01→491.34|6/6|597|0.3457641145797722|
| shell12 |1638.78→1513.52→1511.43|12/12|977|0.40805940673713126|
| monstree6 |1308.16→1140.06→1108.73|6/6|570|0.2603874953623804|

The geometry allocation change has a much larger effect on allocation traffic than on native elapsed time. Counts below use the root's instrumented allocator, whose own atomics affect timings:

| Case | Sparse allocation calls before→after | Cumulative allocated bytes before→after | Peak live bytes before→after |
|---|---:|---:|---:|
| shell6 |1,231,860→78,792|406,455,403→337,406,651|38,689,669→38,431,741|
| shell12 |1,918,957→76,819|787,205,532→672,218,428|52,187,825→51,671,849|
| monstree6 |1,921,336→29,584|517,736,496→390,198,216|44,035,996→43,520,020|

Cumulative allocated bytes are allocation traffic over the run; they are **not** peak RAM/RSS. Allocation calls fall94–98.5%, while peak live memory changes by less than1MiB. Source data, solver iterations and reconstructed model quality remain the same.

## Validation and reproducibility

- Kernel release suite: **72 tests passed**, including exhaustive matching equivalence with strict nearest-neighbor ties and pyramid size boundaries.
- All four feature profiles (baseline, subpixel, refined, root): same descriptor and match bits on the first pair of each frozen real case. Raw `profiles-*.ndjson`.
- Separate frozen-reference geometry qualification: `sparse_geometry_exact` checks 3,000 pivoted systems including singular cases, streamed 9/12-dimensional normal equations, mixed-focal triangulation/refinement, and relative/PnP estimation under all six existing geometry profiles. Raw `geometry-exact.ndjson`; the source stays a qualification artifact because it intentionally imports the pinned reference.
- `pipeline/results.json` contains raw native timings, per-run process RSS, executable hashes and sparse/surface SHA256 values. `allocation-results.json` contains complete allocation profiles.
- `features`, `camera` and `math` are the only production code modules changed. BA/seed scheduling are unchanged; profiling put them well below matching/feature extraction, and changing their optimization policy would risk quality.

Use the project root's final controlled native/browser measurements for final speed claims. This subtask ran no physical Canon R8+iPhone capture and makes no claim about millimeter accuracy. WASM binary size, generated payload size, and browser behavior require the root's integration checks. All routines remain single-threaded, portable safe Rust without new runtime imports.

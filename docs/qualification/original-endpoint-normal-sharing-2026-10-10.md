# Original endpoint normal sharing

`OriginalChordWorkspace::inspect_pair` now supplies its immutable source-bound normal-hull cache to the mean-matrix endpoint exclusion as well as the later coordinate-path exclusion. Previously the first stage called `normal_bounds` directly, paying again even on a warm workspace. The shared helper retains all original source/domain/angular validation and the same interval exclusion formulas; zero-budget queries still refuse before cache access.

Focused regression: 45 passed, zero failed. The first run correctly exposed a changed work counter: a warm planar pair now consumes zero spans instead of one. Its assertion now explicitly requires that saving; exclusion result remains true. The rational arch normal-diameter controls, foreign-source controls, invalid rectangles and zero-budget refusals all pass. Log: `/tmp/original-endpoint-normal-sharing-20261010.log`.

A full NURBS suite is launched in `/tmp/original-endpoint-normal-sharing-nurbs-all-20261010.log`. No terminal broad result yet. This is later native source than the currently published WASM `499506fd...`; no WASM/worker/UI claim is made for this change. The already-running section-data-cache partial wall binary predates this edit and remains a separate generation. This change does not establish whole-wall thickness or general fillet qualification.

The full B-rep library regression is also launched (`/tmp/original-endpoint-normal-sharing-brep-all-20261010.log`). Its 1,811 Rust/Cargo inputs were pinned in `mapped-retained-carrier-2026-10-08/original-endpoint-normal-sharing-native-inputs-2026-10-10.json`; only `surface_self_chord.rs` differs from the preceding section-cache generation.

An additional cold-path regression explicitly compares the cached and fresh original-pair paths on the rational arch. Both retain the real normal diameter; the cold workspace consumes fewer spans by sharing hulls between its two stages. One test passed in 0.03 seconds (`/tmp/original-endpoint-normal-sharing-cold-regression-20261010.log`). These extra assertions were added after the full-suite builds started; production code was unchanged. Their source inventory is `original-endpoint-normal-sharing-cold-test-inputs-2026-10-10.json`.

## Full NURBS regression

The full suite terminated successfully: 850 passed, zero failed, two ignored, 60.69 seconds. The separately added cold-path assertion passed in its focused run; production code is identical. A full build/optimization/packaging was subsequently launched in `/tmp/original-endpoint-normal-sharing-wasm-build-20261010.log`. Until publication terminates, the worker/UI-qualified public artifact remains the preceding section-cache generation. Native B-rep and the older matched partial-wall run are still live.

## Matched complete-carrier measurement

A matched native run of `restricted_profile_seams_match_owned_neighbor_supports` was launched after verifying the current 1,811-input manifest. Budgets: 10,000 cells, 1,000,000 spans, 100,000 trim cells. Absolute report path avoids Cargo package cwd ambiguity. Request: `mapped-retained-carrier-2026-10-08/original-endpoint-normal-sharing-trimmed-wall-request-2026-10-10.json`. Baseline is the preceding section-data-cache complete-carrier report (22,933 spans, 4,081 unresolved products), not the different partial specimen. Log `/tmp/original-endpoint-normal-sharing-trimmed-wall-20261010.log`. No terminal result or specimen-scale work reduction yet.

## Fresh raw WASM ABI

Release compilation finished in 7m39s; optimizer process 53723 is live on its immutable `osv-wasm-opt-GZ9JBg/input.wasm` snapshot. The freshly compiled raw WASM is `c2ac7c670aa75a38aa063a4f40cb9bff5db1b763f0eeee519b91a915005cf0e5`, 14,772,804 bytes. Direct ABI parity passed for planar, rational proposed and genuine normal-diameter cases, then the checker was rerun with whole-coverage cases included. Zero-budget refusal, invalid-domain refusal and recovery pass. Report: `mapped-retained-carrier-2026-10-08/original-endpoint-normal-sharing-unpacked-abi-2026-10-10.json`. Raw ABI is not packaged-worker/UI qualification; public WASM remains the preceding qualified generation until successful packaging.

The six-case raw ABI run terminated successfully, including complete, stopped and curved whole-coverage parity.

## Next optimization inspection

The wall traversal currently performs same-face intrinsic exclusion before position enclosure/distance exclusion. A possible next change is an early distance exclusion using only already-complete cached positions keyed by immutable face and exact UV bits; it must not pre-charge new position work or reuse bounds from another source. No traversal change was made while the current matched baselines are still running. Any implementation needs specimen-level work/coverage measurement rather than assuming benefit from stage order alone.

Matched complete-carrier run terminated successfully: one passed in 403.15 seconds under concurrent workload. Exact coverage/work-reduction comparison passed: 22933 -> 19007 spans; all 4081 pending products, endpoint bits, reasons and exclusion counters unchanged. Report: `mapped-retained-carrier-2026-10-08/original-endpoint-normal-sharing-wall-regression-2026-10-10.json`. This measures span work, not uncontended elapsed speed or whole-wall completion.

Independent pending-containment comparison also passed with matched request sidecars (same test and all three budgets) and source-manifest hashes pinned: `mapped-retained-carrier-2026-10-08/original-endpoint-normal-sharing-wall-pending-2026-10-10.json`. The existing exact-report comparison and this separate domain/request gate both preserve the unresolved coverage; neither grants geometry admission.

## Terminal native and publication results

Full B-rep suite: 1035 passed, 0 failed, 3 ignored (2274.30 s). WASM optimizer completed: 14772804 → 13095936 bytes. Published SHA-256: `dded25909e53fca0021abbc78b00a89e28be5869ff51a8517f9d0b6a98031adf`. These results exclude the subsequently removed early cached-distance experiment. Exact-artifact worker checks are recorded separately after termination.

Exact published-artifact checks completed for SHA `dded25909e53fca0021abbc78b00a89e28be5869ff51a8517f9d0b6a98031adf`: all 35 real-worker boundary tests passed (15.31 s); selected independently partitioned root-carrier/STEP checks passed (2 passed, 28 skipped, 6.82 s). The profile bridge checker independently checked two worker artifacts: native/whole/rational parity, diameter preservation, error recovery, zero-budget refusal, cancel and retry passed. Reports: `../qualification/original-normal-sharing-public-worker-2026-10-10.json` and `mapped-retained-carrier-2026-10-08/original-normal-sharing-root-worker-2026-10-10.json`. Whole thickness and general fillets remain unqualified.

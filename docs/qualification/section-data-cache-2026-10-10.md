# Section-data cache — 2026-10-10

A one-second live sample of partial-wall process 81445 reached corrected bridge derivatives and source section mixed/second derivative bounds (`/tmp/intrinsic-wall-live-sample-20261010.txt`). This sparse sample does not establish hotspot percentages.

`ProfileSectionBridge::derivative_bounds_cached` reuses complete section data across changes of the across parameter. The key includes immutable bridge identity and exact free-interval bits. Validation precedes lookup; incomplete section data is never inserted; the cache is capped at 100,000 entries. It stores derivative enclosures, not qualification decisions. Existing public uncached entry points remain fresh computations. The private cache belongs to a workspace retaining its immutable bridge.

Focused profile-expression regression: 16 passed, zero failed, 3.34 seconds. Extended ownership regression: one passed, 0.05 seconds. It verifies identical full jets and lower work for across-only changes, and no reuse between different section owners. Existing controls retain zero-budget refusal and invalid-domain rejection.

The full NURBS regression has been launched (`/tmp/section-data-cache-nurbs-all-20261010.log`); no terminal result yet. No whole-wall work or wall-clock improvement is claimed. Existing intrinsic-cache and width-Gram partial-wall processes continue on their already-loaded code; their reports must not be attributed to this later cache generation. Public WASM is unchanged by this native-source edit.

Full NURBS regression terminated successfully: 850 passed, zero failed, two ignored, 47.55 seconds. This includes the original-chart width-Gram test and the extended section-cache ownership controls.

Matched complete-carrier wall run launched: `restricted_profile_seams_match_owned_neighbor_supports`, 10,000 cells, 1,000,000 spans, 100,000 trim cells. Log `/tmp/section-data-cache-trimmed-wall-20261010.log`; report and request sidecar use the `section-data-cache-trimmed-wall` prefix under `mapped-retained-carrier-2026-10-08`. All 1,811 Rust/Cargo inputs are pinned by `section-data-cache-native-inputs-2026-10-10.json`. Compare against the completed width-Gram complete-carrier report, not the different partial-carrier specimen.

Full WASM build/optimization/packaging launched for this generation: `/tmp/section-data-cache-wasm-build-20261010.log`. There is one publisher in this worktree. Another observed build process belongs to the separate `osv-nurbs-opt2-wasm-stage-20261010` staging checkout and is not modified. No published SHA or worker/UI qualification is claimed before this build terminates.

Existing `cache_hits` counts top-level derivative/position/direction lookups; internal section-data reuse is reflected in the consumed span counters. Do not interpret that field as the total number of internal cache lookups or compare wall-clock speed under simultaneous native jobs.

## Matched complete-carrier result

The test terminated successfully: one passed, 416.99 seconds under concurrent workload. The report was written relative to Cargo's package cwd (`crates/brep-core`); it and its native request sidecar were moved to the intended repository qualification directory without changing bytes.

Total span work decreased from 23,653 to 22,933: 720 fewer, approximately 3.044%. Exact-coverage comparison passed, preserving every domain endpoint, pending reason, exclusion counter and all 4,081 unresolved products. Report: `mapped-retained-carrier-2026-10-08/section-data-cache-wall-regression-2026-10-10.json`. Whole-wall qualification remains false. Concurrent test durations do not establish production wall-clock speed.

The full B-rep library regression is now launched for the section-data-cache generation (`/tmp/section-data-cache-brep-all-20261010.log`). The previous width-Gram full-suite result must not be used as this generation's regression result. WASM optimizer PID 12011 is live on its immutable input snapshot.

## Unoptimized WASM ABI

The freshly compiled WASM (`4615016ace27ff35f6fe67aa947f2c667b2351be8915ac06d0d8a34d923b94d6`, 14,770,587 bytes) passed six direct ABI cases: planar, rational proposed profile, genuine normal diameter, complete planar coverage, stopped coverage, curved coverage. Native report parity includes work counters and unresolved domains. Zero-budget refusal, invalid-domain refusal and recovery all passed. Evidence: `mapped-retained-carrier-2026-10-08/section-data-cache-unpacked-abi-2026-10-10.json`. This precedes optimized publication and cannot qualify packaged workers/UI by itself.

## Optimized publication and worker qualification

The build/optimizer/packaging process terminated successfully. Published WASM SHA-256 is `499506fdcb58168d991597288619e5cc90e11394cbc481d40f5f03f6b3b25da1`, 13,093,802 bytes. The optimized input was the raw ABI-qualified artifact above.

On this exact published SHA, the profile worker checker passed across two independently checked worker artifacts: native parity, whole-coverage parity, rational profile parity, preservation of a genuine normal diameter, native error recovery, zero-budget refusal, cancellation and Retry. Evidence: `section-data-cache-publication-worker-2026-10-10.json`. The report explicitly leaves whole-thickness and solid qualification false.

Two selected root-worker tests passed (28 skipped), 7.96 seconds of tests: independently partitioned knot-root restoration and mapped-root STEP preparation. Evidence: `mapped-retained-carrier-2026-10-08/section-data-cache-root-worker-2026-10-10.json`; log `/tmp/section-cache-public-root-worker-20261010.log`. This is bounded fixture evidence, not arbitrary fillet or three-part acceptance.

The full real-worker boundary suite also terminated successfully on that exact published SHA: 35 passed, zero failed, 30.16 seconds of tests (33.17 seconds total). It includes artifact-identity mismatch refusal and replacement recovery, cancellation/Retry, stale replies and document restoration. Log: `/tmp/section-cache-public-real-worker-20261010.log`. This does not close the complete command/UI matrix.

The full native B-rep suite remains live (process 19524 verified after the worker suite completed); no terminal native result is claimed here.

## Browser panel on the published artifact

The production profile panel and production worker were exercised on isolated origin 5297 using the published SHA above. Planar whole normal-chord coverage completed in 22 cells. A curved run was cancelled, then retried successfully: 2,564 cells and 15 unresolved products remained. Expanding diagnostics displayed the first pair's exact parameter ranges and guidance to increase work or change sections, explicitly distinguishing unresolved regions from confirmed defects. Console errors were empty. Evidence: `section-data-cache-ui-2026-10-10.json`; screenshot `/tmp/section-cache-ui-retry-20261010.png`. This fixture does not qualify modeler edge selection/preview or whole-wall thickness.

## Partial-carrier matched run

A matched partial rational multispan carrier run has started with 100,000 cells, 1,000,000 spans and 1,000,000 trim cells. All 1,811 recorded source hashes were verified immediately before launch. Request: `mapped-retained-carrier-2026-10-08/section-data-cache-partial-wall-max-request-2026-10-10.json`. Baseline: the completed width-Gram partial-carrier run with 788,859 spans and 46,287 unresolved products. The report path is absolute to avoid Cargo package-cwd relocation. Log: `/tmp/section-data-cache-partial-wall-max-20261010.log`. Process 41264 was confirmed running; no result or improvement is claimed before termination.

## Baseline unresolved-product triage

The completed width-Gram partial-carrier baseline has 46,287 unresolved products, all explicitly classified `budget`. Pair counts are recorded in `mapped-retained-carrier-2026-10-08/width-gram-partial-wall-triage-2026-10-10.json`. This identifies same-face interval exclusion as the next optimization target; it does not demonstrate geometric defects or justify accepting exhausted products. The full-seam endpoint refusal on the control body's pole remains intentional: restricted regular intervals can pass while an undefined endpoint normal cannot be admitted from factored diagnostic limits.

## Terminal native regression

The section-data-cache full B-rep suite terminated successfully: 1,035 passed, zero failed, three ignored, 880.17 seconds. Log: `/tmp/section-data-cache-brep-all-20261010.log`. This supersedes the live-process status above. The separate matched maximum-budget partial-carrier run remains live; regression success is not a whole-wall certificate.

## Completed maximum-budget partial-carrier run

Native integration: 1 passed, 0 failed (2963.44 s). Strict work-reduction comparison passed: 788859 → 694187 spans (94672 saved, 12.001%); all 46287 pending products, UV intervals, reasons and exclusion counters remain exactly unchanged. Evidence: `mapped-retained-carrier-2026-10-08/section-data-cache-partial-wall-regression-2026-10-10.json`. This is reduced proof work, not completion of whole-wall qualification; elapsed times under concurrent load are not performance benchmarks.

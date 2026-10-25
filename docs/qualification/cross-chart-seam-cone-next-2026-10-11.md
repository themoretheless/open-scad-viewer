# Next cross-chart seam proof

Status: the published shared-corner-cache WASM is qualified on the recorded
control cases. Current native source additionally contains an alternate owned
path proof and fragment-local fixed-point cache. Neither provides new whole-wall
coverage on the saved request. The fixed-point candidate has exact complete
response parity. Its full BRep gate passed (1035 passed, 3 ignored), and both
canonical raw and optimized WASM passed staged production-worker parity,
cancellation and fresh Retry. Publication awaits performance review.

## Current qualification by source generation

| Generation | Confirmed evidence | Remaining qualification |
| --- | --- | --- |
| Rail-position cache | Native and published worker controls passed | Superseded public package; historical evidence below |
| Same-sign transverse cone | Full NURBS/BRep and unchanged saved response | Included in the current published package; general curved seam coverage remains open |
| Exact rational-boundary products | Full NURBS/BRep and unchanged saved response | Included in the current published package; not a general rational surface proof |
| Shared-corner interval cache | Full NURBS/BRep; same pending coverage, spans 17759 → 11821; published SHA aec4f261…; complete native/worker parity, cancel/Retry, profile and STEP/root controls; three-part controlled UI fillet/history/reload and independent STEP acceptance | Initial worker timeout and one full-suite sphere timeout retained; serial retries passed. Stable latency, UI source construction and full keyboard parity remain open |
| Alternate owned path | Full NURBS, focused BRep seam consumer and unchanged complete saved response | No demonstrated saved-control coverage gain; not in published WASM |
| Fragment-local fixed-point cache | Bitwise interval/count parity, capacity/ownership and root consumers; strict complete saved response equality; full BRep 1035 passed/3 ignored; canonical raw/optimized WASM worker parity, cancel/Retry | Alternating artifact benchmark running; repeated-station microbenchmark improves, unique stations show overhead. No demonstrated coverage gain or publication decision |

Published artifact verified this turn: SHA256
`aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3`.
It excludes the alternate-path and fixed-point candidates. Qualification record:
`shared-corner-cache-publication-2026-10-11.json`.

The diagonal partition and early boundary-point return experiments were rejected
and rolled back. The latter reduced wall pending rectangles but lost previous
contact proof coverage at faces 6,28. Rollback complete-response equality passed.
The current retained control has 4081 budget-pending wall rectangles, which are
unverified regions rather than confirmed defects.

## Full objective and remaining acceptance

| Workstream | Evidence boundary | Still required |
| --- | --- | --- |
| Shared trimmed 3D edges | Bounded curved affine carriers, independent UV partitions and owned root ends have fixtures | General unrelated surface charts and full edge network qualification |
| Closed fillet bodies | Partial annular cases and equal-radius orthogonal valence-3 cuboid cases | General NURBS/cylinder–cylinder, mixed radii and higher-valence corners |
| Result diagnostics | Recorded interval, seam, root contact and thickness controls | Whole seam tangency, all-wall thickness, gaps/overlaps and complete general self-intersection qualification |
| WASM/worker/new UI | Published package parity; actual three-part UI edge selection, preview/cancel/apply, injected error/Retry and late actual worker-response rejection; candidate staged worker parity | Candidate publication, complete UI interaction matrix, browser diagnostic-panel qualification and WebGPU |
| Three-part acceptance | Native source construction; actual UI controlled fillets/translation, exact Undo/Redo/reload, six STEP-menu exports with independent validity/bounds/volume and 24 gauges; STEP bytes survive reload | Full UI source construction, full mouse/keyboard parity and general geometry; point-cache candidate not covered by three-part browser run |
| P0 | Bounded history/multitab tests, three-part worker fault scenarios and command inventory; fresh enclosure before/after-publication process-crash recovery passed | Complete per-command matrix, broader physical crash/multitab qualification and stable latency |
| P2/P3 | Existing bounded profile/surface/scene capabilities | General NURBS/offset/loft/sweep/G1–G2, large scene/material qualification and Blender/C4D/Sub-D integrations |

Earlier running-status entries below are historical. This summary supersedes
them; individual artifacts retain their own source-generation scope.

## Chronological evidence


Saved-request replay session 58878 terminated successfully: 306752.275791 ms elapsed. Its complete request and response exactly equal the previous rail-cache native replay, including all 4081 ordered pending regions, 17759 spans and 87 transition exclusions. Strict exact-report regression passed (`mapped-retained-carrier-2026-10-08/transverse-cone-dispatch-regression-2026-10-11.json`). The new proof provides no coverage improvement on this control. Elapsed times were collected under different host load and are not a controlled performance comparison.

A separate native geometric regression was added for rectangles touching both owned seam ends on both source axes, including width 1e-6. It explicitly requires a zero-containing direct endpoint-difference hull and tests whether the owned mean-path proof still excludes every nonzero normal chord. The first run failed before entering the proof because the fixture used rounded scale 0.2 instead of the exact section difference 0.6 - 0.4. After correcting that fixture, the regression passed: 1 passed, 0 failed, 0.22 seconds (`/tmp/transverse-cone-zero-seam-retry-20261011.log`). This proves the owned path for these planar seam-adjacent rectangles, not general curved seams.

Implementation update: the algebraic helper is now implemented in `surface_self_chord.rs`; its two focused tests passed, including a counterexample where opposite transverse coefficients cancel the tangent part and leave a normal chord. The helper requires same-sign coefficients, a uniform rank proof and the existing complete endpoint-normal angle proof. Undefined normals, singular interval matrices and invalid scales remain refused.

The helper has been wired into the owned profile-to-transition first path. Exact input endpoint ordering establishes signs; interval subtraction endpoints are clamped at zero only where those exact comparisons prove the sign. Complete path derivative bounds and endpoint normals are unchanged. The profile-radius focused suite is running in session 99868 (`/tmp/transverse-cone-profile-focused-20261011.log`); no carrier improvement or general admission is claimed yet.

This source generation is newer than the rail-cache WASM currently being optimized. That optimizer uses an immutable raw-file snapshot (`rail-cache-compiled-snapshot-2026-10-11.json`); the seam-cone helper and integration are absent from that artifact. They require their own native and WASM qualification.

The current retained-to-transition path encloses the exact chord as three terms: source transverse derivative times `cut - source_parameter`, source free derivative times `bridge_free - source_free`, and corrected bridge transverse derivative times `bridge_across - end`. Independent interval multiplication loses direction information when both endpoints approach the shared seam.

When the two transverse coefficients have the same sign throughout their domains, their weighted derivative sum can be represented by one column. For any fixed positive scale `w`, set `a = (cut - source_parameter) / w`, `b = bridge_across - end`, and `q = a + b`. If q is nonzero, the transverse sum equals q times a convex combination of `w * source_transverse_derivative` and `bridge_transverse_derivative`, since a/q and b/q are nonnegative and sum to one. If q is zero, both a and b are zero. Together with the free derivative, this yields a complete two-column mean-matrix enclosure for all nonzero chords, including arbitrarily short chords.

Candidate implementation conditions:

- Use the existing complete connecting-path derivative bounds and immutable owner. No sampled tangents or serialized proof.
- Admit only when both transverse coefficients are nonnegative or both are nonpositive over the complete domains. Otherwise preserve the existing fallback.
- Hull the two scaled transverse columns with outward interval arithmetic. Use the existing uniform rank and endpoint-normal angle proof on the resulting two-column matrix.
- Preserve continuity, invalid-domain, zero-budget and unresolved-normal refusals. A rank failure means unresolved, never singular geometry or collision.
- Explicitly retain the existing convention that only nonzero chords are candidates; shared identical seam points must not become positive-distance witnesses.

Required controls before use: both seam ends and source axes; arbitrarily short seam-adjacent rectangles; independently scaled columns; opposite-sign transverse coefficients with a cancellation counterexample; zero/undefined normals; reversed endpoint orientation; singular interval matrices; rational multi-span source profiles and non-C1 knots. Then compare full pending products and numerical tolerances on the carrier, rather than assuming a local proof closes the entire wall.

This derivation does not establish that the candidate excludes any of the current 4081 pending products. Instrumented native qualification must measure that effect after implementation.

The integrated profile-radius focused suite terminated with 10 passed, 0 failed, 6.28 seconds. The full NURBS suite has now started (`/tmp/transverse-cone-nurbs-all-20261011.log`).

Full NURBS session 94437 subsequently terminated successfully: 852 passed, 0 failed, 2 ignored, 42.02 seconds. The new source manifest (`mapped-retained-carrier-2026-10-08/transverse-cone-native-inputs-2026-10-11.json`) records exactly two changed native files relative to the rail-cache generation: `profile_expression_radius.rs` and `surface_self_chord.rs`. Saved-request native replay has started (`/tmp/transverse-cone-dispatch-native-20261011.log`); its whole-wall result remains unknown.

A two-second native sample captured 103 observations while replay was in `qualify_volume`, including corrected support positions, jets and side remainders (`/tmp/transverse-cone-native-sample-20261011.txt`). This locates the observed phase only; it does not establish the dominant cost of the complete run or a regression caused by the new chord proof. Replay session 58878 and the separate rail-cache optimizer session 38616 remain live.

The strict native work summary (`mapped-retained-carrier-2026-10-08/transverse-cone-work-summary-2026-10-11.json`) attributes all 17759 span queries exactly once. All 4081 pending rectangles cover 58 face pairs and carry the budget reason, with zero enclosure/resolution reasons. Shared-corner queries account for 6952 spans (39.15%); this is query accounting, not elapsed-time profiling and not a defect count. The full B-rep native suite is running in session 38620 (`/tmp/transverse-cone-brep-all-20261011.log`). A quadratic parabolic extrusion regression now exercises both source axes and both seam ends; its focused run is session 43655 (`/tmp/transverse-cone-curved-seam-20261011.log`).

The polynomial quadratic-extrusion seam regression passed: 1 passed, 0 failed, 1.06 seconds (`/tmp/transverse-cone-curved-seam-20261011.log`). It checks both source axes, both seam ends, zero-containing direct chord hulls, source normal bounds and zero-budget refusal. The same fixture now additionally uses positive nonconstant separable rational weights; that extended run is in progress (`/tmp/transverse-cone-rational-seam-20261011.log`). This is bounded profile qualification, not a general NURBS fillet claim.

The first rational extension failed before chord inspection: `constrain_original_rails` refused weights varying along both parameter axes because `preserved_boundary` requires its existing exact boundary-weight identity (or a constant boundary). The failed log is retained at `/tmp/transverse-cone-rational-seam-20261011.log`. The retry uses nonconstant interior U weights [1, 1.25, 1] and constant V weights so all four original boundary identities satisfy that contract. General rational boundary preservation remains an explicit unsupported case; the production guard was not relaxed. Retry session 16845 is running (`/tmp/transverse-cone-rational-seam-retry-20261011.log`).

The polynomial plus supported rational regression terminated successfully: 1 passed, 0 failed, 1.63 seconds (`/tmp/transverse-cone-rational-seam-retry-20261011.log`). Both source axes, both seam ends and both rectangle widths pass with nonconstant interior rational weights. This does not cover the refused doubly varying boundary weights above.

A newer native source change replaces the overly restrictive equal-weight boundary condition with exact dyadic product equality: row[fixed] * column[free] == sourceWeight * pivot (and the symmetric axis case). Positive rational weights then represent the identical original boundary, with no tolerance or samples. Binary64 significands are multiplied in u128 and normalized with integer exponents, avoiding floating product overflow/underflow. The existing clamping, constant-boundary, ownership and control-budget gates remain. Focused controls cover subnormal/large products and a one-ULP mismatch; the doubly varying rational seam fixture has been restored for qualification. This change is newer than the running B-rep binary and the immutable WASM optimizer snapshot; neither qualifies it.

Exact dyadic boundary controls passed (1 test, 0 failures, 0.00 seconds, `/tmp/exact-rational-boundary-20261011.log`). The restored doubly varying separable rational seam fixture also passed (1 test, 0 failures, 1.43 seconds, `/tmp/exact-rational-boundary-seam-20261011.log`). This supersedes the specific boundary-weight refusal above for exactly separable products, while arbitrary nonseparable weight profiles remain unqualified. A full NURBS suite is starting for this source generation (`/tmp/exact-rational-boundary-nurbs-all-20261011.log`).

The exact-boundary generation is recorded in `mapped-retained-carrier-2026-10-08/exact-rational-boundary-native-inputs-2026-10-11.json` (1812 native inputs, with only the profile regression and weight-profile source changed from the transverse-cone manifest). The unchanged saved-request replay is running in session 96802 (`/tmp/exact-rational-boundary-dispatch-native-20261011.log`). No carrier coverage change is claimed before its terminal result and strict comparison.

Pending-face classification now binds the complete replay SHA (`mapped-retained-carrier-2026-10-08/transverse-cone-pending-face-classes-2026-10-11.json`). Of 4081 pending rectangles, 2049 are same-face products (1203 original, 574 retained, 272 transition); only 784 are distinct retained-to-transition products. Improving only cross-chart seams cannot close this control. The classification is diagnostic accounting, with no collision or thickness admission.

Full NURBS qualification of the exact-boundary source generation terminated successfully: 855 passed, 0 failed, 2 ignored, 334.11 seconds (`/tmp/exact-rational-boundary-nurbs-all-20261011.log`). Rehashing all 1812 manifest inputs during the run found no changed paths. This supersedes the earlier in-progress NURBS entry; B-rep and WASM qualification remain separate.

Exact-boundary saved-request native replay terminated successfully in 511499.669 ms. Both the unchanged request and the entire response are exactly equal to the transverse-cone replay, including 4081 pending rectangles and 17759 span queries. The comparison is saved as `mapped-retained-carrier-2026-10-08/exact-rational-boundary-dispatch-comparison-2026-10-11.json`; the extracted complete wall report is `exact-rational-boundary-dispatch-wall-2026-10-11.json`. This change expands exactly separable rational boundary support demonstrated by the focused seam fixture, but provides no coverage gain on this carrier. Runtime measurements under differing host load are not a controlled performance comparison.

The B-rep consumer regression `source_support_shell::tests::exact_profile_owned_curved_seam_limits` is running on the exact-boundary source generation (session 15033, `/tmp/exact-rational-boundary-brep-seam-20261011.log`). It preserves the low-budget path refusal and separately checks the sufficiently refined seam-angle proof; it is not a whole-body wall qualification.

The exact-boundary B-rep consumer regression terminated successfully: `source_support_shell::tests::exact_profile_owned_curved_seam_limits`, 1 passed, 0 failed, 41.96 seconds (`/tmp/exact-rational-boundary-brep-seam-20261011.log`). It preserves the bounded-work refusal and proves the refined curved seam case. Full B-rep qualification for this newest source generation is still outstanding; the earlier full B-rep process covers the transverse-cone generation.

The immutable rail-cache WASM build terminated successfully and published SHA256 `853d0656a470c6b09e6f82a3eb9efb793269f7f7389342de5bd66df55ead80ad`, 13100699 bytes. Public bytes and generated identity match. Publication record: `rail-cache-publication-2026-10-11.json`. Saved-request complete-response worker parity is running in session 14389 (`/tmp/rail-cache-matched-worker-20261011.log`); this artifact excludes the later transverse-cone and exact rational-boundary changes.

Rail-cache WASM real-worker qualification terminated successfully: exact complete native response parity, refusal, dispatch cancellation and fresh-worker Retry all passed. Both started workers were joined. Timings: refusal 4796.371083 ms, cancellation 141.797333 ms, Retry 64675.59975 ms under the unchanged 120000 ms deadline. Report: `rail-cache-matched-worker-2026-10-11/expression-inspection-worker-report.json`. These are one-run observations, not a latency-margin guarantee. The exact-SHA real boundary suite is running in session 60687 (`/tmp/rail-cache-real-boundary-20261011.log`).

Exact-SHA real worker boundary suite terminated successfully: 35 passed, 0 failed, 54.78 seconds (`/tmp/rail-cache-real-boundary-20261011.log`), with `CAD_EXPECTED_WASM_SHA256=853d0656a470c6b09e6f82a3eb9efb793269f7f7389342de5bd66df55ead80ad`. This qualifies the published rail-cache artifact only. The older transverse-cone full B-rep run now has only the authored-radius control outstanding; no terminal suite result is claimed yet.

After the rail-cache worker gates passed, the latest exact-boundary source generation was rehashed (1812 inputs unchanged) and its full B-rep suite started in session 2207 (`/tmp/exact-rational-boundary-brep-all-20261011.log`). Its own WASM build is session 91404 (`/tmp/exact-rational-boundary-wasm-build-20261011.log`); the current published 853d0656 artifact remains the earlier generation until completion. A pinned profile-chord worker gate for that published artifact is running (`/tmp/rail-cache-profile-worker-20261011.log`).

The pinned rail-cache profile-chord real worker gate terminated successfully (session 67180, exit 0). Report: `rail-cache-profile-worker-2026-10-11.json`; this gate compares full native polynomial and rational reports and checks cancellation/Retry on SHA 853d0656a470c6b09e6f82a3eb9efb793269f7f7389342de5bd66df55ead80ad.

The expanded pinned profile worker gate with `--whole-coverage` terminated successfully: exact native whole/stopped/curved coverage parity, rational profile parity, native-error recovery, zero-budget refusal and cancel/Retry passed (`rail-cache-profile-whole-worker-2026-10-11.json`). Exact-SHA STEP/root worker controls also passed: 2 passed, 28 intentionally filtered, 46.52 seconds (`/tmp/rail-cache-root-worker-20261011.log`), covering original curved root carriers and mapped root carriers through STEP/history. Neither gate is fresh UI three-part acceptance or general whole-wall admission.

The latest WASM Rust compilation finished successfully in 16m51s. All 1812 source inputs still match the exact-boundary manifest. Raw artifact SHA256 is `68bc000728b3b668cbb59335b74e99e361ffce65fc5885a7f0210669beadd4a7`, 14780595 bytes (`exact-rational-boundary-compiled-snapshot-2026-10-11.json`). Optimizer PID 1746 is confirmed active on its immutable copy. Public artifact remains the already qualified earlier 853d0656 generation until optimization and packaging complete.

A newer queue optimization partitions identical same-face UV products D×D directly into L×L, L×R and R×R modulo exact endpoint exchange. Every nonzero candidate remains covered; the optimization skips intermediate half-product work and grants no exclusion. The full partition is checked on both axes, boundary/midpoint pairs and unequal-face/domain refusals. Focused partition test passed: 1 passed, 0 failed (`/tmp/diagonal-wall-partition-focused-20261011.log`). This source change is absent from the immutable latest WASM snapshot and both running full B-rep binaries; it has its own manifest `mapped-retained-carrier-2026-10-08/diagonal-wall-partition-native-inputs-2026-10-11.json`. No control coverage improvement is claimed yet.

The unchanged saved inspection request is now replaying against the diagonal partition source generation (session 66748, `/tmp/diagonal-wall-partition-dispatch-native-20261011.log`). Its output uses a fresh create-only path `mapped-retained-carrier-2026-10-08/diagonal-wall-partition-dispatch-native-2026-10-11.json`. Pending coverage and complete outcomes remain unqualified until that replay finishes and is compared.

Diagonal partition adjacent native checks passed: all four `product_tests`, 0 failures, 0.02 s (`/tmp/diagonal-wall-product-tests-20261011.log`). These cover exact same-face endpoint exchange, exact containment guards, partition coverage, and preservation of unresolved domains/reasons in the diagnostic wire format. This validates bookkeeping, not general wall thickness. The older full B-rep binary has 1034 passing tests and one running annular-radius test; the exact-boundary binary has 1031 passing tests and four running tests. Neither has a terminal suite result yet.

Current shared trimmed-edge audit: all 9 `source_mapped_edge::tests` passed in 0.14 s (`/tmp/mapped-edge-current-20261011.log`). The implemented scope already includes partial curved affine carriers, differently partitioned source UV curves, independent irrational roots with a proven unique common plane relation, exact root candidates, and restore of the owned partial edge. Wrong branch and displaced exact candidate controls refuse admission. Therefore the historical statement that only full original edges are proven is stale for these specific cases. This does not prove arbitrary unrelated chart mappings or complete mixed-radius fillet solids.

Terminal updates: transverse-cone full B-rep suite passed, 1035 passed / 0 failed / 3 ignored, 3639.13 s. Diagonal partition replay exited 0 but regressed bounded coverage: 4081 -> 6166 pending products and 17759 -> 28392 spans. Request and full response outside the wall report match exactly. Candidate integration, helper and candidate-specific test were removed; the wall file SHA matches the pre-candidate exact-boundary source manifest exactly. The rejected source and comparison are retained as `mapped-retained-carrier-2026-10-08/diagonal-wall-partition-rejected-source-2026-10-11.txt` and `diagonal-wall-partition-comparison-2026-10-11.json`. No speed or coverage improvement is claimed. The immutable WASM build was never based on this rejected candidate.

Newer unqualified source generation: `SharedCornerWorkspace` now reuses complete original rectangle/normal evidence on two immutable source references and one fixed angular policy. The exact common-corner and proposed direction checks still run per query; zero work remains refused before cache reuse. All nine existing original-corner tests passed (0.03 s). A newly added cache/direct parity and refusal regression is compiling in session 12594 (`/tmp/shared-corner-workspace-regression-20261011.log`). This change is not wired into B-rep wall inspection yet and is absent from the ongoing immutable WASM build and full B-rep binary. No performance or wall coverage gain is established.

Shared-corner cache qualification: all 10 original-corner tests passed, 0 failures, 0.03 s, including cache/direct decisions, a warmed zero-span reuse, zero-budget refusal, changed projections, invalid domains and an approximately shared corner refusal. B-rep integration now owns one workspace per ordered immutable original face pair and preserves queue order and the 64-span per-query cap. Test builds compare direct admission and span work. Saved unchanged request replay is compiling/running in session 98393 (`/tmp/shared-corner-cache-dispatch-native-20261011.log`). A separate 1812-input manifest binds this newer generation; no coverage or performance gain is claimed before replay.

Shared-corner cache full NURBS gate terminated successfully: 856 passed, 0 failed, 2 ignored, 133.35 s (`/tmp/shared-corner-cache-nurbs-all-20261011.log`). All 1812 generation input hashes still match the manifest. The native wall replay remains live; its full report is not yet qualified.

Shared-corner cache saved-request replay terminated successfully (417343.832375 ms under host load). All 4081 ordered pending UV products and the complete response outside the wall report are exactly unchanged. Strict coverage/nonincreasing-work comparator passed: 17759 -> 11821 spans (5938 saved, 33.44%). Shared-corner stage alone changes 6952 -> 1014; all other span stages are exact. Cache hits change 73903 -> 79841. All 1812 input hashes still match. This is verified work reduction, not elapsed-time acceleration or new whole-wall admission. New B-rep suite has 611 passing tests so far, including unequal-radius annular shared seams; its final result remains outstanding.

Shared-corner cache WASM raw compilation started separately in session 99900 (`/tmp/shared-corner-cache-wasm-compile-20261011.log`) using the same locked release/strip/remap recipe. On success it copies the raw Cargo artifact create-only into `shared-corner-cache-wasm-stage-2026-10-11/geometry-kernel.raw.wasm`. This job does not optimize or publish. The older exact-boundary optimizer owns an immutable input snapshot, so compilation cannot change its output generation. Public WASM and generated transport identity remain owned by the older build until that build terminates.

Exact rational-boundary full B-rep gate terminated: test result: ok. 1035 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 3430.56s (`/tmp/exact-rational-boundary-brep-all-20261011.log`). This binary excludes the later shared-corner cache.

Shared-corner cache raw WASM compilation terminated successfully in 17m16s. Its 14786487-byte immutable snapshot has SHA256 `8a653c0fc553561f58fee3e04d385ba0b9adb72d7279fffc5d52d5440913d00f`; all 1812 input hashes matched. The generated-only stage was moved from the qualification directory to `/Users/themoretheless/.codex/tmp/shared-corner-cache-wasm-stage-2026-10-11` to keep build payloads outside repository documentation. Snapshot provenance is recorded in `mapped-retained-carrier-2026-10-08/shared-corner-cache-compiled-snapshot-2026-10-11.json`. Optimizer session 27560 now writes only a create-only staged optimized file (`/tmp/shared-corner-cache-wasm-optimize-20261011.log`); it cannot publish over the older build.

Exact-boundary WASM build session 91404 terminated successfully and published SHA256 `5584980642278d1f2f414885899cfc22fed49e7aa701c38ed6cad0f1fc228232`, 13103192 bytes. Public/generated WASM bytes and generated identity all match. Publication evidence is `exact-rational-boundary-publication-2026-10-11.json`. Saved-request full native/worker response parity with cancel/Retry is running in session 93867 (`/tmp/exact-boundary-matched-worker-20261011.log`); exact-SHA real boundary suite is running in session 64095 (`/tmp/exact-boundary-real-boundary-20261011.log`). Shared-corner cache optimizer remains isolated in session 27560 and cannot change these public bytes.

Exact-boundary artifact worker results: exact-SHA real boundary suite passed all 35 tests, 127.08 s total (`/tmp/exact-boundary-real-boundary-20261011.log`). Full saved-request native/worker parity did NOT qualify: fresh-worker Retry hit the unchanged 120000 ms deadline, workersStarted=2/workersJoined=2, immutable WASM/request identities retained in `exact-boundary-matched-worker-2026-10-11/expression-worker-failure.json`. Host load was concurrent, but the cause is not established. No latency or full-response parity admission follows. Whole-profile worker and STEP/root-carrier controls are now running separately.

Exact-boundary whole-profile worker gate passed: native/rational/whole-coverage parity, error recovery, zero-budget refusal and cancel/Retry all true; two loaded artifact identities checked. STEP/root worker controls passed 2 tests (28 unrelated tests filtered), 39.90 s. These do not supersede the full expression inspection Retry timeout. The expression checker now explicitly binds each worker entry to the expected artifact identity and records request dispatch/entry/response and existing warmup/execute/prepare timing when available. An intentional 1 ms failure control passed: timeout remains a failure, dispatch observation is preserved and every worker is joined. No geometry request or deadline was changed.

Exact-boundary observed worker rerun passed with complete native response equality and both loaded artifact identities checked (`exact-boundary-observed-worker-2026-10-11/expression-inspection-worker-report.json`). Cancel to Retry passed; Retry took 69380.017583 ms. The earlier 120000 ms timeout remains evidence of unqualified latency stability. Fresh Chrome Dev reload at port 5195 rendered the new CAD UI with WebGPU; this is startup evidence only. Embedded Codex browser crash remains unresolved.

Worker stage timing diagnosis: production runtime requires transport `traceTiming: true`; prior checker observations omitted that flag, so their response timings do not separate warmup/execute/prepare. The checker now explicitly enables this transport instrumentation without changing the geometry request or deadline. Execute timing includes the entire execution path, not exclusively Rust.

Instrumented worker replay terminated successfully (`exact-boundary-timed-worker-2026-10-11/expression-inspection-worker-report.json`): full native response parity, two loaded identities, cancel/Retry and both response stage timing objects verified. Retry timing: {"warmupMs": 1049.4949579999993, "executeMs": 66475.30845899999, "prepareMs": 0.2869159999972908}; full client Retry 69074.653 ms. This remains a single run under concurrent load and does not establish latency stability.

Mandatory timing regression passed (`exact-boundary-required-timing-worker-2026-10-11/expression-inspection-worker-report.json`): both completed responses contain validated stage timing; native full-response equality, two worker identities and Cancel/Retry passed. Client timings ms: {"refusal": 4854.355375, "dispatchCancellation": 228.07066699999996, "retry": 64001.69125}. Earlier timeout remains retained.

Pending inventory recorded in `mapped-retained-carrier-2026-10-08/shared-corner-cache-pending-inventory-2026-10-11.json`, bound to replay SHA and unchanged request: 1058 same-face overlapping-domain rectangles, 991 same-face separated-domain rectangles, 2032 different-face rectangles. All 4081 are budget pending. Largest pair is 27–28 (448 rectangles); retained-to-transition pairs total 784. Counts are neither geometric area nor defects. This identifies owned retained-to-transition coverage as a next proof target; cache savings alone do not resolve it.

Alternate owned-path candidate: `profile_expression_radius.rs` now applies the existing same-sign transverse theorem to the second fully enclosed source path as well. Exact owned section, signed deltas, complete derivative columns and endpoint normals remain required; partial alternate paths cannot admit. Four focused tests passed (0.62 s). Full NURBS session 94587 and saved-request native replay session 45305 remain live; coverage gain is unproven. Candidate identity is in `mapped-retained-carrier-2026-10-08/alternate-owned-path-candidate-2026-10-11.json`. This source edit is newer than both public 558 WASM and the immutable shared-corner cache optimizer input.

Alternate owned-path full NURBS gate terminated: 856 passed, 0 failed, 2 ignored, 161.66 s (`/tmp/alternate-owned-path-nurbs-all-20261011.log`). Candidate source SHA still matches. Native saved-request replay remains live; B-rep consumer `exact_profile_owned_curved_seam_limits` is running separately. No coverage gain or WASM admission yet.

Alternate owned-path B-rep consumer passed (1 test, 19.66 s). Saved-request replay terminated (238140.835833 ms under concurrent load): exact same request and entire response, including all 4081 pending rectangles and 11821 spans. Comparison: `mapped-retained-carrier-2026-10-08/alternate-owned-path-dispatch-comparison-2026-10-11.json`. No coverage or work reduction demonstrated on this control; a fixture distinguishing the alternate cone path remains needed before claiming expanded support.

Independent path-cone algebra control passed: `independent_complete_path_hulls_can_resolve_an_inconclusive_cone` (1 passed, 0 failed). Broad source column hull refuses, narrower complete hull admits for both common signs, opposite signs still refuse. This is an algebraic fixture only; it does not establish an owner-bound CAD case benefiting from the new alternate path. Only test code changed in `surface_self_chord.rs` after the preceding full NURBS gate.

Shared-corner cache full B-rep suite terminated successfully: 1035 passed, 0 failed, 3 ignored, 3181.56 s (`/tmp/shared-corner-cache-brep-all-20261011.log`). This compiled generation excludes the later alternate-owned-path candidate. Shared-corner optimization is still live on its immutable raw snapshot; no public artifact was overwritten.

P0 timing transport: `MainSolidWorkerClient.run` now accepts optional `onTiming`, requests runtime stage instrumentation and emits accepted-response warmup/execute/prepare plus client elapsed time. Stale, malformed geometry/protocol, cancelled and timed-out replies cannot emit successful-response timing; invalid diagnostic values are ignored, observer exceptions cannot change command settlement. Client suite passed 30 tests, Vue and MCP type checks passed. The real expression checker now uses this public client option rather than injecting trace flags at its port; session 79972 is verifying full native parity and cancel/Retry on published 558 WASM. No pure-Rust timing or UI latency qualification is claimed.

Production client timing API passed real worker qualification (`exact-boundary-client-timing-worker-2026-10-11/expression-inspection-worker-report.json`): entire native response equality, two loaded artifact identities, Cancel/Retry and exactly two accepted client timing observations. Retry timing: {"warmupMs": 605.6314999999995, "executeMs": 34054.500667, "prepareMs": 0.11433300000499003, "clientElapsedMs": 35905.289416}. Client unit suite now passes 31 tests including typed failure timing and late callback after cancellation; unchanged production source already passed Vue/MCP type checks. This is real Node worker evidence, not full browser UI qualification.

Shared-corner WASM optimization and publication terminated successfully: raw 14786487 → optimized 13108282 bytes (11.3% smaller), SHA256 `aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3`. Public/generated bytes verified equal; previous published 558 package preserved outside the repository. Publication provenance: `shared-corner-cache-publication-2026-10-11.json`. New artifact excludes the later alternate-owned-path candidate. Full saved-request real worker session 97238, exact-SHA boundary session 40707 and whole-profile worker session 23022 are running. Worker/UI qualification is not yet established.

Shared-corner published aec4 artifact: whole-profile worker passed; STEP/root worker passed 2 tests (28 unrelated skipped), total 198.75 s. Chrome Dev startup, cube creation, keyboard Undo/Redo and final Undo back to empty scene passed. Full expression worker Retry hit unchanged 120000 ms timeout (failed report retained in `shared-corner-cache-worker-2026-10-11/expression-worker-failure.json`, both workers joined); no full-expression worker or latency admission. Boundary test session 40707 remains live. A serial repeat is planned after the other CAD gate ends; concurrency is not established as the cause.

Shared-corner exact-SHA boundary suite terminated: 34 passed, 1 failed, 276.82 s. Failure is the unchanged 30000 ms Vitest deadline for `converges oblique sphere distances across the real worker boundary`; no geometric assertion failure reported. Original log retained at `/tmp/shared-corner-cache-real-boundary-20261011.log`. Both own parallel CAD gates ended; serial full expression worker started in session 73374, same saved request and 120000 ms deadline. No stability admission.

Shared-corner serial full expression worker passed (`shared-corner-cache-serial-worker-2026-10-11/expression-inspection-worker-report.json`): complete native response equality, two loaded artifact identities, Cancel/Retry and stage timing. Retry: {"warmupMs": 372.65412500000093, "executeMs": 65173.974083, "prepareMs": 0.10033400000247639, "clientElapsedMs": 66997.410208}. Earlier full worker and sphere deadline failures remain retained; no stable latency admission or proof that host concurrency caused them. Oblique sphere distance is now repeating alone with its original 30000 ms test deadline.

Shared-corner sphere-distance serial repeat passed: 1 passed, 34 unrelated skipped, tests 5.05 s / total 9.91 s with unchanged 30000 ms deadline. This gives a passing observation for all 35 boundary cases across runs, not a green full-suite rerun or stable latency proof; original full-suite failure remains retained.

Coverage bottleneck audit: the unchanged saved request exhausts 10000 cell visits while consuming only 11821/100000 spans and all 100000 trim-domain cells. Further span-cache savings alone cannot expand this control coverage. `source_profile_shell_contacts.rs::support_domain` already restricts retained faces at the exact owned root parameters, and the transition uses its owned section range; do not reapply a second approximate trim. Existing canonical cylinder arcs use rounded sqrt(1/2) weights, so an ideal-circle minimum cannot be admitted merely from the primitive name. Any analytic shortcut needs a fresh owner-bound identity or conservative enclosure proof.

Contour budget candidate: source winding now stops unresolved if a complete interval enclosure of an owned fragment core station is wholly inside the padded rectangle. Such a boundary point prevents the existing whole-band exclusion proof. No winding or outside/inside admission is inferred, and triangle queries never use bounding-box containment. Focused rational contour test passed, including early crossing refusal and unchanged outside admission. Full B-rep and unchanged saved-request replay started; candidate source identities recorded in `mapped-retained-carrier-2026-10-08/source-boundary-point-band-candidate-2026-10-11.json`. Public aec4 WASM excludes this candidate.


## Boundary-point band audit: partial coverage improvement, contact regression

The native candidate returns unresolved early when a complete enclosure of an
actual source boundary station lies inside the padded rectangular query. It
never admits inside/outside from that condition and excludes triangle queries.
The saved request is unchanged. Wall domain work falls from 100000 to 64359;
wall pending regions fall from 4081 to 4035. Exact endpoint containment of every
new wall pending product in an old product passes (five changed face pairs).
This is bounded wall coverage evidence, not whole-wall qualification.

The effect is global: contact subdivision and its finite work allocation also
change. An exact endpoint union-partition audit preserves old admitted contact
coverage for 48 of 49 changed pairs, but pair index 174 (faces 6,28) loses one
previously admitted product and adds a pending product outside the previous
pending union. Therefore this candidate does not pass the contact coverage
nonregression gate and must not replace the published WASM as a qualified
improvement. The replay, recursive differences, pending containment result,
contact coverage report and reproducible audit are retained under
`mapped-retained-carrier-2026-10-08/source-boundary-point-band-*` and
`audit-source-boundary-contact-coverage-2026-10-11.py`.

UI hard-limit hints passed 65 tests across expression focus, radius panel and
inspection controller; vue-tsc exited zero. Explicit capture option objects
fix Node EventTarget listener cleanup compatibility. A browser listener leak
was not established. Native full BRep candidate suite remains live at this
entry; no terminal success is claimed.


### Candidate rollback after contact coverage audit

Removed the early boundary-point return from source_contour_winding.rs after
its full-response audit identified a contact coverage regression at faces
6,28. The rejected source is preserved create-only alongside its replay.
The rational boundary-crossing regression still requires unresolved location,
no winding admission and no rectangle proof; the outer rectangle remains
certified outside. The running full BRep binary predates this rollback and
must be labelled as the rejected candidate generation when it finishes.
The public WASM was not changed.

Rollback validation: the rational source contour test passed, 1 passed / 0
failed (1037 filtered), 0.03 s; cargo exited zero. Log:
`/tmp/source-boundary-rollback-focused-20261011.log`. This does not certify
full contact replay parity of the current sources. Next optimization must
retain coverage under shared resource limits, with both wall-pending and
contact-product coverage gates; fewer reported pending regions alone is
insufficient.


## Fragment-local source point interval cache candidate

FixedWorkspace owns one immutable Curve reference and caches only complete
successful point interval evaluations under exact binary64 parameter keys.
Capacity is 4096; misses after capacity call the original evaluator. Each
logical query, including hits, retains endpoint_queries accounting. Loop order,
cell/span budgets, source fragments and subdivision remain unchanged. Cache
lifetime is one fragment inside one classification. No early boundary return.

The rational contour fixture checks all interval endpoints bitwise against the
original evaluator across repeated parameters, logical query count parity and
five distinct cache entries. Focused native test passed 1/0, 0.03 s; cargo
exited zero. Full saved-request replay is running; no runtime benefit or full
response parity is yet claimed. Public WASM remains unchanged.


Fixed-point cache capacity/ownership extension passed 1/0 in 0.09 s, cargo
exit zero (`/tmp/source-fixed-point-cache-capacity-20261011.log`). The fixture
fills 4096 entries from real evaluated source points, verifies bounded miss
fallback bitwise, unchanged logical query increments, and independent shifted
source ownership. The contact audit now rejects non-finite or degenerate boxes
before union coverage comparison; its original pair 6,28 regression reproduces.


Strict full-response regression gate is now available in
`scripts/compare-expression-wall-reports.mjs --full-replay`. It checks replay
schema, source request hash/bytes/object and the complete response before the
existing exact wall report validation. Elapsed time is outside response and
is not treated as geometric evidence. Full replay mode rejects weaker work
reclassification options. Shared-corner versus alternate-owned-path saved
replays pass. A contact-only cells mutation with unchanged wall report fails
and writes no passing artifact. New running replay outputs are not yet present.


Rollback full saved-request replay completed with cargo exit zero. Strict
`--full-replay` comparison against alternate-owned-path baseline passed:
`mapped-retained-carrier-2026-10-08/boundary-point-rollback-full-regression-2026-10-11.json`.
Every response field matches, including contacts and wall report. This proves
rollback parity on that saved request; it does not complete general geometry
qualification. Fixed-point cache replay remains active.

Rollback replay elapsed 237114.7175 ms under concurrent host workload;
not a benchmark. Wall pending count restored to 4081. The winding module
filter contains only the same single rational contour test (1 passed, 1037
filtered); it is not broader independent coverage of arbitrary contours.


Fixed-point cache consumer gate: original interior contact removed only after
root-valued region clipping passed 1/0 in 1.07 s, cargo exit zero. This exercises
source contour classification through original-face contact clipping rather
than only standalone point evaluation. Log:
`/tmp/source-fixed-point-cache-root-contact-20261011.log`.
Full cache replay remains live and unpublished.


Fixed-point cache full replay completed, cargo exit zero. Strict complete
response equality passed against alternate-owned-path baseline, recorded in
`source-fixed-point-cache-full-regression-2026-10-11.json`. All contact,
volume, thickness and pending fields are unchanged on this request. No
throughput/latency improvement is claimed from concurrent elapsed timings.
The root proposal clipping consumer test also passed 1/0 in 0.66 s.

The obsolete rejected-boundary candidate full BRep run was intentionally
terminated (owned cargo 99524/test 99528, session 99127 exit 143) after its
coverage regression and rollback. Its partial log remains preserved; it is
not a passing full gate. A new full BRep --lib run for the current point-cache
candidate is active in `/tmp/source-fixed-point-cache-brep-all-20261011.log`.
Public WASM remains unchanged pending qualification.


Manual fixed-point repeated-station microbenchmark passed. Debug build,
20000 calls / 128 unique rational-arc parameters, five alternating-order
rounds: median direct 823.782 ms, cached 29.937125 ms. Endpoint checksum bits
and 20000 logical query counts match per round. Concurrent host workload
precludes application latency claims; this is a bounded hot-path measurement.
Raw rounds are recorded in
`source-fixed-point-cache-repeated-stations-benchmark-2026-10-11.json`.
The manual ignored fixture now also checks 20000 unique parameters to expose
miss-path overhead; that second run remains active. Full BRep gate is active.


Both manual station workloads passed bitwise checksum and logical query
accounting. Under concurrent load, debug median repeated 128 stations:
1223.016417 ms direct versus 89.045292 ms cached; unique 20000 stations:
898.159833 ms direct versus 1094.324042 ms cached. Large round variation and
opposite workload behavior forbid a universal speedup claim. Raw rounds are
retained in `source-fixed-point-cache-workloads-benchmark-2026-10-11.json`.
An isolated application-level measurement remains required before deciding
whether to publish this optimization. Staged geometry-wasm release compilation
is running from the new input manifest; public artifacts are untouched.


P0 inspection dispatch guards: startInspection now requires active panel,
source options and no calculation already pending. Both error Retry and
unqualified-result Retry buttons reflect those same guards. The renderer
fixture checks duplicate click suppression and inactive/missing-source Retry,
including direct invocation of the disabled button handler. Three UI files,
65 tests passed (42.80 s); log:
`/tmp/expression-inspection-retry-guards-20261011.log`. Type check is still
running at this entry. Native inputs remain unchanged during staged WASM build.


Inspection dispatch guard vue-tsc exited zero. Error Retry keyboard focus
is being added with current active/options/error guards. The first focused
renderer test failed because a sync watcher requested nextTick before the
conditional Retry button was rendered (64 passed, 1 failed). The watcher now
runs post-render; the repair suite is active. Failed run retained in
`/tmp/expression-inspection-error-focus-20261011.log`; repair log is
`/tmp/expression-inspection-error-focus-repair-20261011.log`.
The focus change vue-tsc exited zero; the subsequent scheduling-only repair
does not alter its types. No full keyboard acceptance claim is made yet.


Error Retry focus repair passed all three UI files: 65 passed, 0 failed,
23.22 s, exit zero. The active error focuses the rendered Retry button; a late
inactive error does not move focus. Full native BRep and staged WASM jobs are
still separate and active. This validates renderer focus handling, not the
complete real-browser keyboard command matrix.


Disposable production worker qualification staging is implemented in
`scripts/stage-expression-worker-qualification.mjs`. It copies runtime sources
and canonical package outputs into a new external directory, verifies packed
WASM bytes and unchanged live public hash, and rejects checkout destinations,
symlink aliases into the checkout, and existing destinations. The parent
directory must exist. Syntax and path guards passed.

Staged runtime smoke passed with the already published aec4f261 package:
budget refusal, cancellation and fresh-worker Retry; two workers started and
joined, loaded identities verified. Report retained as
`expression-worker-staging-runtime-2026-10-11.json`, summary
`expression-worker-staging-smoke-2026-10-11.json`. This qualifies the staging
mechanism on that bounded control, not the new point-cache WASM or full CAD UI.


Staging now records `runtime-inputs.json`: 854 copied runtime/script/config
files, exact SHA256 per file and manifest hash in staging.json. Copies own
source files (dereference symlinks); node_modules remains an explicit shared
dependency link outside the source inventory. A new staging smoke verified
all 854 hashes and the manifest hash after packaging, with unchanged live
WASM. Record: `expression-worker-runtime-manifest-smoke-2026-10-11.json`.
This is snapshot provenance, not an additional completed worker execution;
the preceding worker smoke covers the earlier staged runtime copy.


Initial fixed-point raw WASM compilation completed in 9m14s; 1812 inputs
match. Raw SHA 46612ed8682805f24945e6b834e5f15775e65e04857261e31f23624d973c4e72,
16390436 bytes. This first command omitted canonical strip-symbols and path
remapping flags. It is explicitly diagnostic, ineligible for publication or
size/performance comparison. Its optimizer was intentionally canceled
(owned child 33801; session 56224 exit 1), preserving the raw snapshot.

A corrected build now uses the exact cargo arguments/environment from
build-geometry-kernels.mjs and reproducibleCargo, with locked dependencies,
strip symbols, remapped paths and pinned crates/target. Recipe hashes and log
are retained in `source-fixed-point-cache-canonical-recipe-2026-10-11.json`.
No public/generated package was replaced.


Fresh controlled part acceptance passed on the published aec4f261 package:
three newly constructed parts, translation, radius-1 qualified fillets, two
commits, two Undo/Redo and exact JSON restore. Six before/after STEP files
passed independent OpenCascade validity, analytical bounds and volume checks.
Record: `cad-fresh-parts-shared-corner-normalized-2026-10-11/README.md`.
Explicit new .mts script type check and application vue-tsc both exited zero.
The first attempted run failed on comparing unnormalized input JSON; corrected
fixture preparation additionally asserts unchanged B-rep through normalization.
That failed log/output remains retained. No fresh browser reload or general
fillet qualification is inferred from this result.


Fresh STEP parts additionally passed 24 independent OpenCascade original-face
line-intersection gauges across six files: webs, walls/cavity/bottom/open top,
flange bore/off-axis bore/axial size, and radius-1 sections. Script:
`verify-cad-fresh-part-dimensions.py`; result in the fresh part directory's
`dimension-report.json`. This is explicit cross-section evidence, not all-wall
thickness or entire-seam tangency. Canonical WASM compilation completed 4m58s,
raw SHA 96a292df220b1daa4c33868b5ecb8c45fe2423e3cf1ab7b8e1c2d53fafccf08b,
14791396 bytes; 1812 input hashes still match. Its immutable optimizer is active;
public package remains unchanged.


Fresh modified STEP reimport: all topology identity sets and exact vertex/edge
bindings are preserved for all three parts. Strict raw loop/face/shell bindings
fail. Diagnostic inspection shows planar chart flips with face-use sense
changes and full-period flange supports replacing quarter-span supports;
face-to-loop identities remain unchanged. These observations are not a proof
of chart/orientation equivalence. Reports: fresh directory identity-report.json
and chart-differences.json. The verifier now labels native-fresh scope explicitly,
keeping browser mode as the legacy default. New import harness type check passed.

### Fresh STEP oriented incidence

Added `scripts/verify-cad-step-oriented-incidence.py` and create-only report for bracket, flange and enclosure. All three preserve exact vertex/edge geometry bindings, body-shell bindings and oriented face-loop-edge boundaries modulo cyclic start and shell orientation. Both wrong shell orientation and wrong coedge direction are rejected on each part. Surface interior and UV pullback equivalence remain unqualified; this does not replace the failed strict raw-representation report.

## Algebraic chart and incidence qualification

`chart-map-incidence-report.json` supersedes the earlier unqualified chart assessment for these three fixtures only. All 63 faces have exact coefficient agreement under identity, U reflection or owned quadratic Bezier-span restriction. Matching positive rational bases bound trim UV deviation at every parameter by at most 4.1633363423443376e-17 (tolerance 1e-14). Exact edge/vertex bindings and oriented incidence are required in the same gate. Surface and trim mutations are rejected separately on each part. Strict raw-representation equality remains false; general chart matching, full seam tangency, whole-wall checks and browser reload remain open.

### Fresh browser history/reload and native suite

`cad-fresh-reload-browser-2026-10-11/report.json`: all three fresh native edited/rounded parts imported through the actual running UI in an isolated browser context. One additional UI translation per part preserved topology IDs; Undo, Redo and browser reload returned exact exported JSON snapshots. This does not claim fixture construction/fillet through UI, keyboard parity, independent STEP measurement of the additional UI translation, or full command matrix. Published WASM aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3.

Point-cache native BRep suite completed: 1035 passed, 0 failed, 3 ignored, 1827.83s, terminal exit 0. Evidence: `mapped-retained-carrier-2026-10-08/source-fixed-point-cache-brep-suite-2026-10-11.json`. Canonical WASM optimization remains live and unpublished.

Visual review of enclosure reload screenshot shows CPU/SVG fallback with display/topology preparation hints still visible. This run qualifies document/history recovery, not settled visual refinement or WebGPU.

### Actual UI edit → STEP → history → reload → STEP

`cad-fresh-reload-step-browser-2026-10-11`: all three rounded native fixtures imported through UI, translated +1 mm X through the command panel, exact Undo/Redo and browser reload verified. Six actual STEP-menu downloads (edited and reloaded) independently pass 24 OpenCascade face/line gauges, maximum error 7.105427357601002e-15 mm. Expected placement [4,-2,1] is analytical fixture placement plus commanded translation; not inferred from measured geometry. Aggregate OCCT validity/bounds/volume job still running; native creation/fillet is not claimed as browser action.

The six UI STEP downloads now also pass independent OCCT solid validity, one-solid count, analytical bounds and volume. Maximum bounding error is 1.000000082740371e-7 mm; maximum volume error 1.6495405361638404e-6 mm³. Each edited/reloaded pair has identical STEP bytes. Scope remains the three controlled fixtures and one additional UI translation, not general modeling or full P0.

### Fresh UI fillet acceptance

`cad-fresh-ui-fillet-2026-10-11/report.json`: native source bracket/flange/enclosure imported, edges selected through actual SVG controls with Enter/Shift+Enter, radius 1 preview cancelled with Esc without document change, recomputed/applied, exact Undo/Redo, UI translation, STEP export, exact browser reload and second STEP export. All three pass. Independent 24 line/face gauges also pass. Full keyboard navigation and native-source construction through UI remain uncovered. Flange screenshot viewed: correct rounded exterior and bore visible in CPU/SVG fallback; WebGPU unqualified. Aggregate OCCT bounds/volume/validity still running at this entry.

Fresh UI fillet OCCT aggregate acceptance completed: all six STEP files valid, one solid each, analytical bounds and volume pass. Edited/reloaded STEP bytes identical for each of the three parts. This closes the controlled fillet UI/export/history/reload chain; general fillets, complete UI source construction, keyboard parity and WebGPU remain open.

### Three-part UI worker fault qualification

`cad-fresh-ui-fillet-faults-2026-10-11/report.json` passes all three parts: injected CAD_TRANSPORT failure preserves document, Retry followed by Esc preserves document, delayed actual bodyEdit success delivered after Esc is ignored. Then normal fillet, history, translation and reload pass. Six STEP files are byte-identical to the independently OCCT-measured no-fault run (`step-byte-parity.json`). This is fault-injected transport/stale-result coverage for these fillets, not OS crash or full P0 matrix.

### Pending contact diagnostic UI

Added selection of a single native pending contact product and its two UV rectangles, explicitly labeled as proposed-shell chart domains. Unverified contact pairs are not labeled confirmed self-intersections. Transition-face highlighting limitations are visible; source highlight is disabled while inactive/pending or unavailable. Region selection resets when the selected pair changes; invalid region hides intervals. 67 component/controller tests pass; vue-tsc passes. This improves reporting and does not resolve native pending contacts or certify full self-intersection coverage. Evidence: `expression-contact-regions-ui-2026-10-11.json`.

### Canonical pre-optimization WASM worker qualification

Canonical raw WASM SHA256 `96a292df220b1daa4c33868b5ecb8c45fe2423e3cf1ab7b8e1c2d53fafccf08b`, 14791396 bytes, staged outside the checkout with 860 runtime files hashed. Production Node worker/client test passes budget refusal, cancellation with terminated realm and fresh-worker Retry. Complete Retry response strictly equals native point-cache replay for the unchanged 504950-byte request SHA256 `bc675ce7e69c50feb54dd30fc6d5db63d9f2fc39e8a34da285971fd59356b80d`. Both workers joined. Retry observed 42850.927959 ms under current host load; not a stable performance comparison. Geometry coverage unchanged, including 4081 budget-pending aligned-chord regions. Optimized WASM remains under optimization; raw package is not published. Live published SHA aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3 unchanged. Report: `mapped-retained-carrier-2026-10-08/source-fixed-point-cache-canonical-raw-worker-2026-10-11.json`.

Maximum trim-domain budget hint fixed alongside existing cell/span limits. 67 component/controller tests and vue-tsc pass. No native budget increased or coverage reclassified. Evidence: `expression-trim-domain-budget-ui-2026-10-11.json`.

### Canonical optimized WASM worker qualification

Optimized artifact SHA256 `68b872ac8479a8d4af850bf3949b2f3b8bb5fc34fbe229a3c1b17d094cbb6a21`, 13112736 bytes, optimizer terminal exit 0. Disposable production Node worker/client passes full native response equality for unchanged saved request, budget refusal, dispatched cancellation, terminated realm and fresh-worker Retry; two workers joined. Evidence: `mapped-retained-carrier-2026-10-08/source-fixed-point-cache-canonical-optimized-worker-2026-10-11.json`. Live artifact remains aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3; serial alternating artifact benchmark launched before publication decision.

### Fresh enclosure physical process-crash recovery

`cad-fresh-enclosure-process-crash-2026-10-11/process-crash-browser.json` passes: dedicated Chromium process killed with SIGKILL while paused before IndexedDB head publication and after transaction completion; persistent profile reopened. Exact exported JSON recovers old document before publication and new document (same edited/rounded enclosure plus 60 large groups) after publication. Both native geometry and document identities survive through exact JSON equality. This is one fresh enclosure and two commit boundaries, not arbitrary OS/power failure or complete multitab qualification.

The alternating worker-artifact benchmark overlapped this owned-browser crash test. Its timings must be treated as preliminary host-load observations; do not present them as an isolated performance comparison.

### Artifact benchmark and next shared-edge boundary

First alternating artifact benchmark completed six full native-parity runs. Elapsed medians: published 36898.959625 ms, candidate 32847.40575 ms. Owned process-crash browser test overlapped this run, paired times varied substantially, and no performance improvement is qualified. A second serial artifact benchmark without overlapping owned browser/native test jobs is running in `source-fixed-point-cache-worker-serial-benchmark-2026-10-11`.

Shared-edge source review: `source_mapped_edge::qualify_with_candidates` freshly verifies both surface/pcurve compositions via `source_affine_use::qualify`, then owned root identity. Canonical parameter correspondence is still an authored affine range, not a general nonlinear rational parameter map. Existing partial/root and independently partitioned fixtures therefore do not close that general case. Extending correspondence needs exact composition and owned endpoint/root identity for the new map, not matching coordinate boxes.

## Exact projective parameter correspondence

Added a source-leaf exact predicate for C(m(t)) = S(P(t)), with two normalized rational endpoint fractions and positive rational-linear map weights. The homogeneous denominator is retained through polynomial cross products; original controls remain unchanged. Forward, reversed and partial parabola fixtures pass, and changed geometry/map weights are rejected. The first full cad-predicates run passed 87 tests.

The NURBS source adapter admits single Bezier carriers/pcurves and single Bezier surface charts; it explicitly returns unsupported for piecewise inputs. Chart membership, shared-root endpoint transport, B-rep sewing and general fillet qualification remain separate obligations. This candidate has not been built into the published WASM.

Follow-up validation: all seven curve_surface_affine tests passed, including the new projective original-source/domain/budget test. The focused exact projective predicate test also passed with zero and just-insufficient budget checks.

## Projective retained roots across charts

Added a separate immutable ProjectiveUse: fresh exact composition authority, unchanged original Fragment/root expressions, positive map weights, exact orientation and outward canonical endpoint bounds. It does not enter the existing affine SharedEdge serialization path.

The crossing-endpoint consumer proves both source roots lie on the supplied plane and that their entire mapped parameter enclosure union contains a unique canonical world-plane root. A two-chart transposed-UV parabola fixture passes; wrong plane and insufficient work are refused. Focused test passed (source_projective_use, one test with multiple checks; log /tmp/source-projective-use-20261011.log).

Remaining: general piecewise projective correspondence, fixed/mixed endpoint identity, portable restore/replay, complete shared-edge sewing and fillet admission. No published WASM change.

## Exact fixed and mixed projective ends

Added projective_parameter_identity: source-domain fractions, positive map weights and rational endpoint ranges are compared using unreduced exact expansion cross products. Exact 1/3 mapped by weights 1:2 equals 1/2; binary64-rounded 1/3 is correctly different. Reverse/partial ranges, invalid weights, out-of-domain parameters, zero and just-insufficient work are covered. Focused native predicate test passed.

ProjectiveUse::same_parameter_endpoint accepts fixed endpoints or root candidates freshly verified against the original root equations and selector. The mixed fixture uses weights 1:3, an exact source root at parameter 12 and a polynomial neighbouring use at parameter 14; both map to the same canonical midpoint. Changed candidate and exhausted work are refused. The expanded source_projective_use test passed (log /tmp/source-projective-mixed-20261011.log). Root-root plane correspondence remains covered by the same test.

This adds endpoint identity authority, not a serialized SharedEdge or a sewn fillet body. Portable replay, piecewise projective maps, sewing and published WASM remain outstanding.

Full cad-predicates follow-up passed: 43 unit, 23 construction3, 12 construction, 10 independent-oracle tests (88 total), zero failures.

## Portable projective use and paired edge

ProjectiveUse now saves only original world/fragment/range/weight definitions. Restore recomputes original root membership, exact composition and orientation. Focused restoration test passed, including refusal of changed weights/ranges, unsupported version, zero mapping allowance and insufficient exact work.

Added source_projective_edge::Edge: both incident uses are freshly qualified under a shared exact-work budget, must traverse oppositely, and must identify both retained endpoints through exact parameter/candidate proofs or unique common plane roots. Driver cells share one budget. Restore repeats every obligation from the two original use definitions; cached direction/bounds are not stored. The paired edge replay and changed-weight/changed-plane/zero-driver/insufficient-work refusal checks passed in the expanded source_projective_use test (/tmp/source-projective-edge-restore-20261011.log).

This type is not yet substituted into the existing affine SharedEdge consumers. Their inverse maps, seam sampling/tangency and Model sewing must be updated consistently before admitting projective edges there. Published WASM remains unchanged.

JSON strict decode and binary codec roundtrips preserve the paired-edge recipe exactly; JSON-decoded replay freshly qualifies both uses and both endpoints. Expanded test passed (/tmp/source-projective-edge-codecs-20261011.log).

## Projective inverse and whole-seam normal qualification

ProjectiveUse now supplies outward normalized forward mapping and an unclipped rational inverse. An inverse outside the source domain remains outside instead of hiding unsupported canonical tails. Forward/inverse enclosure checks pass.

The existing whole-interval source_seam_tangency qualifier now uses a sealed native Carrier abstraction, implemented only by previously qualified affine SharedEdge and projective Edge types. The interval-normal algorithm and budgets are shared; positional ownership still comes from freshly qualified edge construction. Existing default Seam/Report types retain SharedEdge compatibility.

A retained root-ended parabola on two transposed plane charts passes the angular envelope. The exact same positional parabola on z=v-u^2 is rejected as an oblique seam. The expanded projective test passed (/tmp/source-projective-seam-fixtures-20261011.log), including replay, mixed/fixed/root endpoints and inverse checks. This qualifies bounded G1 tangent planes for the fixtures, not G2, radius, whole-wall validity or Model sewing. No published WASM change.

All seven existing source_seam_tangency regression tests passed, including interior creases, singular endpoints, rotating planes, reversed/partial affine carriers and root-ended seams (/tmp/source-projective-seam-regression-20261011.log).

## Common projective boundary partition

Generalized the existing shared endpoint/interior/chain/mesh routines over the sealed native Carrier interface. SharedEdge retains its existing public methods and default report types; projective Edge uses the same algorithms. Root ends remain expressions, inverse enclosures must lie strictly inside both retained uses for interior samples, and refused requested stations remain explicit in the ordered chain.

The projective parabola fixture passed common endpoint/interior samples, analytical UV checks on both transposed charts (enclosure widths below 1e-10), refusal of boundary/outside stations, shared-budget refusal and a boundary mesh with all deviation bounds <=0.1 mm and no pending leaves. Existing shared-edge tests: 12 passed; existing mapped-edge tests: 9 passed. Record: projective-shared-boundary-partition-2026-10-11.json.

Closed-shell incidence, face clipping, vertex links and embedded/material body admission remain unqualified for projective edges. This partition is preparation for sewing, not a closed body. Published WASM unchanged.

## Projective closed incidence and vertex links

Refactored source_shell_incidence assembly into one private core for address validation, budgets, exact pairing, vertex ownership and face connectivity. Existing public assembly functions retain SharedEdge defaults. Added assemble_projective and Shell<projective Edge>; maps on each incident use are freshly qualified before incidence is admitted. Vertex-link inspection and chart injectivity work on either native shell type.

A tetrahedron with different positive rational weights on all twelve directed uses qualifies six shared edges and four manifold vertex-link cycles of length three. Altered weights, missing pairing and exhausted work are refused. All 11 source_shell_incidence tests passed, including irrational-root partitions, curved shell inverse/contact/volume replay, mapped partial chains, poles and disconnected/geometry-mismatched refusal (47.04 seconds). The focused projective cross-layer run also passed. Record: projective-shell-incidence-2026-10-11.json.

New projective shell qualification currently establishes incidence and vertex links, not embedded faces/material/volume. Its persistence and material geometry admission remain to connect. Root-ended curved projective ownership/partition/tangency are separately tested; the new closed tetrahedron control has fixed-ended line carriers. Published WASM unchanged.

## Projective original-region shell persistence

Added assemble_regions_projective to preserve qualified original material-region definitions in the new shell type. New source_projective_shell_restore saves original regions, addressed paired uses and original projective-edge recipes. JSON and binary restore re-audit regions/root definitions, require recorded fragments to equal the replayed region boundaries, require both canonical carriers to agree, and rerun the complete shared-edge/incidence assembly. No saved vertex IDs, bounds or admission flags are trusted.

The rationally reparameterized tetrahedron passes both codec replays with identical definitions, vertex owners and pair addresses. The restored material topology has Euler characteristic 2 and genus 0. Changed weights with an injected certificateQualified flag yield no shell; a missing pair is rejected. The final two projective integration tests pass (/tmp/source-projective-shell-final-20261011.log). Root candidate checks are now included in the new edge/shell diagnostic counter without changing exact-work accounting. Record: projective-material-shell-replay-2026-10-11.json.

Material-region ownership and manifold incidence do not establish embeddedness, extra-contact absence, orientation/volume or a valid fillet Model. Those projective shell consumers remain to connect. No published WASM change.

## Fresh projective-shell contact and embedding admission

Generalized the exact source_allowed_contact shared-line proof over sealed native edge authority. Existing SharedEdge certificates/reports retain default types; projective certificates retain the original projective edges. Added full-pair inspection using original rational hull separation, exact shared-line confinement and original UV-domain search fallback. Adjacent pairs remain in the traversal. Unsupported contacts are unresolved, never topology-pruned.

Geometry<projective Edge> can now be admitted only by freshly recomputed manifold material topology, whole-chart injectivity and complete face-pair qualification. The tetrahedron passes all six pair proofs. A one-pair allowance yields no Geometry and reports next_pair=[0,2]. Both projective integration tests, all 11 shell-incidence regression tests and the original clipped-interior-contact regression pass. Record: projective-shell-contact-admission-2026-10-11.json.

This establishes embedding for the bounded planar control, not general nonlinear fillets. Additional nonlinear contact consumers, orientation/volume and Model conversion remain to connect. Published WASM unchanged.


## Projective material volume and orientation

The generic source-volume gate now consumes privately admitted projective shell
geometry. Complete original carriers use the same Green-flux algorithm; unsupported
trims keep conservative source-domain integration. The tetrahedron fixture with
independent positive rational UV weights admits a signed interval excluding zero,
a positive volume interval containing the independently known 1/6, and width at
most 0.01. No serialized volume or certificate flag grants authority.

Validation: `/tmp/source-projective-volume-20261011-repair.log`, 2 focused tests
passed, 0 failed. This is a native planar control fixture, not qualification of
general curved fillet bodies, STEP export or WASM/UI deployment.

A projective-body replay adapter has been added to re-run source region/incidence,
embedding/all-pair contacts and volume in sequence. Its binary replay and forged
certificate/insufficient-pair-budget tests are running; not yet claimed passed.


### Projective-body replay result

`/tmp/source-projective-body-replay-20261011.log`: 2 passed, 0 failed.
Binary body replay preserves original definitions and vertex ownership, repeats
all geometry gates and independently encloses volume 1/6. Modified weights refuse
admission; injected certificate/volume fields do not bypass a one-pair contact
budget. Evidence snapshot: `projective-body-volume-replay-2026-10-11.json`.
Existing source-shell incidence regression is running separately.

Source-shell incidence regression: 11 passed, 0 failed, including irrational root partitions, mapped chains, curved shell inverse-contact/volume replay and pole contraction. Log: `/tmp/source-projective-volume-incidence-regression-20261011.log`.


## Projective nonlinear boundary contact ownership

Natural-plane fiber and paired-fiber certificates now retain the generic private
carrier type. Their proof still uses original material regions, exact fiber
preimages and all boundary/vertex ownership checks. It does not use rounded
endpoint proximity or sampled seam matches. The existing nonlinear wedge with
rational rim weights 0.5 assembles through projective edge qualification and
proves at least two directed natural-fiber contacts; a one-unit work budget
refuses certification. Focused result: 1 passed, 0 failed, log
`/tmp/source-projective-natural-fiber-20261011.log`.

Projective shell geometry now consumes these proofs with global exact/span/driver/
linear budgets, retaining conservative domain fallback for unsupported contacts.
Combined integration tests are running. General nonlinear interiors, complex
fillet corners and deployment remain unqualified.

Combined projective integration: 3 passed, 0 failed (`/tmp/source-projective-fiber-integration-20261011.log`). Existing capped canal body volume regression also passed: 1 test, 118.98 s (`/tmp/source-projective-volume-canal-regression-20261011.log`). Evidence snapshot: `projective-boundary-fiber-contacts-2026-10-11.json`.

Paired-fiber regression: 3 passed, 0 failed, including coincident-face refusal, both spherical seam orders, exhausted work and straight neighbor rail ownership (`/tmp/source-projective-paired-fiber-regression-20261011.log`).


## Projective interior fiber contact extension (validation running)

The existing source-interior contact proof now retains the admitted generic
carrier type. It still rechecks original chart injectivity, the complete plane
preimage, retained region support and every possible boundary/vertex contact.
The projective pair inspector tries both face orders under shared exact/span/
driver/linear limits and falls back conservatively if support is unproved.

The root-clipped curved wedge fixture now also assembles projective edge maps
from its original affine fractions and exact root candidates, requiring a
qualified interior contact with the newly authored cap. Validation log:
`/tmp/source-projective-interior-fiber-20261011.log`. No result claimed until
that process exits. This does not establish general curved interior contacts.

Interior extension validation completed: 1 passed, 0 failed, 33.26 s. The root-clipped wedge projective incidence and new cap interior contact passed alongside existing whole-body checks. Evidence: `projective-root-clipped-interior-contact-2026-10-11.json`.

Combined projective regression after interior-contact extension: 3 passed, 0 failed (`/tmp/source-projective-interior-regression-20261011.log`).


## Projective vertex contact and four-edge links

Exact endpoint fractions from privately qualified projective maps now support
original carrier corners. Only fractions exactly zero or one authorize an
endpoint control point; root-valued ends remain original expressions and are
not replaced by fixed points. Planar material hulls retain original positive
carrier control hulls and original full-fragment checks.

The weighted rational octahedron fixture proves six manifold vertex links with
four edges each, 12 vertex-only face-pair contacts in both orders, refusal under
exhausted work and fresh embedding qualification of all 28 face pairs.
Existing vertex regression: 2 passed. Combined projective integration: 4 passed,
0 failed (`/tmp/source-projective-vertex-integration-20261011.log`). Snapshot:
`projective-four-edge-vertex-contacts-2026-10-11.json`.

This qualifies contacts of the authored control shell; construction of a
four-edge fillet corner and general root-valued vertex-point proofs remain open.


## Generated end supports through projective incidence

Projective assembly now accepts original collapsed pole boundaries through the
same exact native collapse proof and union/link algorithm as the existing path.
Original material-region preparation is shared through a typed assembly callback;
no copied geometric recipe or input Body/closed flag becomes proof authority.
Projective shell replay records raw pole addresses/points and freshly proves
collapse. Older pole-free definitions remain readable.

The generated linear canal fixture (endpoint radii 0.5 and 1.25) and spherical
end supports produce a manifold genus-zero projective shell. Whole-chart proofs
include pole quotients; binary replay preserves definitions, vertex ownership
and poles; a 1e-12 pole perturbation refuses incidence. Default edge mapping also
now accounts for original fragment traversal separately from canonical traversal.
Combined tests: 5 passed, 0 failed (`/tmp/source-projective-generated-endcap-20261011.log`).
Evidence: `projective-generated-endcaps-2026-10-11.json`.

All-pair pole contact/volume qualification in the new projective path remains
open. This is not general curved-spine or mixed-radius corner fillet admission.


## Projective capped canal embedding and material volume

Natural pole-planar and paired-pole contact certificates retain the admitted
carrier type, original region and fiber proofs. Projective all-pair inspection
uses the same native proof algorithms under globally shared exact/driver work.
No pole topology or adjacency skips contact checks.

The generated straight canal (length 8, endpoint radii 0.5/1.25) now admits
embedded projective geometry with all 120 original face pairs qualified. Its
positive volume interval contains the independent axial frustum plus two
spherical-segment integral and has width <= 0.25. Signed orientation is negative
and the admitted body reports the required orientation reversal.

Combined integration: 5 passed, 0 failed, 74.26 s
(`/tmp/source-projective-endcap-volume-20261011.log`). Original paired-pole
regression: 1 passed, 0 failed (`/tmp/source-projective-pole-contact-regression-20261011.log`).
Evidence: `projective-capped-canal-volume-2026-10-11.json`.

Projective pole body replay, Model/STEP transfer and general curved-spine or
complex-corner fillet qualification remain open. Published WASM is unchanged.


## Projective UV parameter transport into Model/STEP

Full canonical projective bodies now share the native Model transfer algorithm.
Directed original Bezier pcurves are proposed under the inverse positive
projective map by homogeneous weight scaling. A fresh exact whole-interval
curve/surface composition proof must succeed before a proposed pcurve is used.
Rounding, overflow, unsupported multispan transport or partial/root restrictions
refuse transfer; no sampled coincidence or positional welding is authority.

The independently weighted tetrahedron passes Body -> Model -> STEP v6 ->
imported Model with full geometry/topology/entity identity checks. Focused result:
1 passed, 0 failed (`/tmp/source-projective-model-transfer-repair-20261011.log`).
Snapshot: `projective-uv-model-step-transport-2026-10-11.json`.

Generated capped-body Model/STEP and complete body binary replay are running
separately. Independent external STEP validation of these new fixtures is open.


### Capped projective body Model/STEP and replay

The generated capped body now passes native Model conversion, STEP v6 export/
import with full geometry/topology/entity identity checks, binary Body replay
through all fresh gates, identical volume interval and vertices, and identical
Model conversion after replay. Result: 1 passed, 0 failed, 98.53 s, log
`/tmp/source-projective-endcap-step-replay-20261011.log`.

Independent OCCT checks are being prepared from the exported artifact. The
existing capped-body oracle now optionally accepts separately supplied center/
radius fixture data while retaining the original default fixture. No external
STEP acceptance is claimed before that oracle runs.


### Independent capped STEP acceptance

OCCT reads the generated projective capped STEP as one valid solid. Independent
axial frustum/spherical-segment volume: 24.952242822677132 mm3; OCCT:
24.952242861024978 mm3; absolute error 3.8347845787711776e-8 mm3.
Maximum axis-bound error: 1.0000000294496658e-7 mm.
Input center/radius fixture is separately supplied; no native result is used as
the expected measurement. Artifact hash:
86982557f19e79356ce8345c5ca74337e208bac53dcbec2c71a20b94a18dd20d.
Artifact: `projective-capped-canal-2026-10-11.step`; OCCT report:
`projective-capped-canal-step-occt-2026-10-11.json`.
Model transfer regression also passed: 2 tests, 0 failed.
Evidence summary: `projective-capped-body-step-replay-2026-10-11.json`.

This is independent qualification of the specified straight canal control, not
completion of general fillet/edit workflows or bracket/flange/enclosure parity.


## Projective root restriction persistence

Native Body edge restrictions now retain the admitted carrier type through
original source definition storage, canonical endpoint interval mapping, XYZ
endpoint enclosures and bounded display segments. Projective restoration freshly
replays original root equations/selectors, both compositions and cross-face
endpoint identity. No interval midpoint becomes an authoritative endpoint.

Focused root replay: 1 passed; full incidence regression: 12 passed, 0 failed.
Logs: `/tmp/source-projective-root-restriction-20261011.log` and
`/tmp/source-projective-restriction-incidence-regression-20261011.log`.
Evidence: `projective-root-restriction-persistence-2026-10-11.json`.

This closes native restriction transport for the qualified projective edge;
root/partial Model and STEP transport remains open. Display enclosures neither
admit a solid nor certify chord error. Whole-seam tangent checks remain separate.

## Synchronized whole-boundary tangent planes

Original homogeneous Bezier derivative numerators now retain a common seam parameter through outward Bernstein polynomial products. Positive normal lower bounds are required; singular or unsupported charts remain unresolved. The freshly qualified shared edge supplies position and correspondence ownership.

The generated spherical-ended canal passes all eight cone/cap junctions at squared-sine tolerance 1e-10, with one complete parameter cell per seam and explicit polynomial work budget 100000. All seven previous seam tests pass. This is tangent-plane evidence for the stated charts, not general fillet or wall qualification. WASM publication is unchanged.

The native Body API now checks owned edge indices before synchronized tangency qualification. The nonunit projective root-ended fixture passes via the existing interval fallback and retains its exact definition; its oblique neighbor is refused. The capped-body chain passes fresh embedding/volume, native STEP identity, binary body restoration, and all eight tangent seams both before and after restoration (76.11 s). Direction reversal, translation, unsupported partial UV map, invalid budgets and singular normals also pass their focused regression. Git diff-check observation timed out; selected modified files have no trailing whitespace.

## Projective common-parameter normal transport

The synchronized normal calculation now substitutes the inverse positive Mobius map directly into homogeneous Bernstein normal polynomials. Each coefficient receives the corresponding outward positive map-weight powers; the common denominator cancels from the tangent-plane angle. Reversed source traversal swaps the inverse factors. Only complete canonical endpoint ranges are accepted by this adapter; original partial UV curves still use interval fallback. Work for every coefficient multiplication is counted. No rounded curve reconstruction or sampled normals are used as authority.

A native rational shared-boundary fixture now proves original exact composition, synchronized tangent planes, binary definition replay and forged map rejection. Extending its retained endpoint to source parameters 1/2 and 1/3 exposed an unresolved gate: candidate verification accepts binary64 parameter pairs, and existing plane binding requires the entire other source curve image to be planar. The second cutter only crosses the plane at the certified UV root. This fixture is deliberately refused with projective-shared-endpoints-unproven; it remains an explicit implementation gap, not qualified root support.

## Exact projective linear root identity

The preceding 1/2 versus 1/3 refusal is now resolved for original transverse rational linear UV crossings. A fresh exact predicate derives each normalized root fraction from unchanged line equations, applies positive Mobius weights and exact range fractions, then compares by cross multiplication. Private SourcePoint uniqueness binds this algebraic crossing to the retained root. Projective edge admission tries this bounded proof before the planar-cutter fallback. Native admission, binary replay and whole-boundary tangency pass on the previously refused control. All 44 current cad-predicates tests pass, including wrong-map, nonpositive-weight and exact work-budget controls. Nonlinear UV crossings remain outside this new proof.

## Nonlinear rational root candidates

A separate exact candidate gate now accepts numerator/positive-denominator pairs on original single Bezier UV curves (degree 1..32). The outward candidate enclosure must lie strictly inside the original unique-root selector, and homogeneous original-curve equality is freshly proved. The original SourcePoint definition remains unchanged. The exact-input common normalization requires an explicit unit to balance numerator and domain-times-denominator expressions; a new predicate regression covers this invariant. All 45 cad-predicates tests pass. A parabola crossing at boundary 1/3 and cutter 1/9 passes candidate verification and binary root replay, while rounded binary64 parameters, a wrong rational candidate inside the same selector, invalid denominator and insufficient work are refused. Shared-edge rational candidate transport is still a separate pending integration.

## Nonlinear rational candidates in shared 3D edges

Projective edge qualification now accepts optional original rational root candidates. Both candidates are freshly verified against their original unique UV root expressions, then a separate exact predicate compares their mapped canonical parameters while retaining authored domains. All work shares the edge admission budget. The optional definition field is omitted for legacy calls; restoration replays source uses, candidates and canonical identity.

The control parabola has source parameters 1/3 and 1/5 on adjacent incident uses. It passes exact admission without a plane/root driver, binary edge replay, retained tangent-plane qualification and restriction replay (bounds, endpoint boxes and display segments). Wrong candidates inside the selector, forged maps, removed candidates and one-less-than-required work are refused. Two native projective tests and all 46 current predicate tests pass. Automatic shell candidate generation and general fillet-body qualification remain open.

## Rational root proposals through material shell assembly

Addressed RootFractionCandidates now pass through projective incidence assembly and original material-region assembly. Legacy APIs pass an empty candidate list. Duplicate pair addresses, nonfinite fractions and nonpositive denominators are refused. Projective shell restoration reads the optional edge metadata and freshly replays original regions, root candidates and canonical correspondence with the shell exact-work allowance.

A two-face control with opposite UV charts, nonlinear retained boundaries and source roots 1/3 and reversed 4/5 passes four-edge closed incidence, four manifold vertex links, genus zero, binary original-region/shell replay and identical vertex ownership. Corrupting a candidate inside its selector prevents admission. This coincident-face fixture is only incidence evidence; it does not prove an embedded material body. The originally attempted parabola-plus-chord contour remains unproven by the existing simplicity gate and is retained as a refusal control, rather than silently qualified. Automatic rational candidate discovery and the general fillet/body goal remain open.

## Supporting-line proof resolves the original chord control

The preceding parabola-plus-chord simplicity refusal is resolved for positive original Bezier boundaries satisfying a supporting-line certificate. Original control orientations are proved with the exact native predicate. Nonnegative coefficients with at least one strictly positive coefficient place the entire open rational curve strictly on one side; the other curve stays in the opposite closed half-plane. A nonshared far endpoint must be strictly on that side or outside the other control hull. Two-curve loops may share both already-owned endpoints. Original injectivity is still required. One counted orientation cell permits at most 256 exact work units, within the existing query cell limit.

All seven simplicity tests pass, including two-curve and split-chord loops, reverse traversal, rotation/translation, coincident geometry, a nonshared endpoint intersection and budget refusal. The original chord contour was restored to the material-shell fixture: original-region qualification, rational root pairing, manifold incidence and binary material-shell replay now pass on that exact control. This remains UV simplicity and closed incidence evidence, not general embedded-solid or wall qualification.

## Bounded rational-root discovery in shared edges and material shells

Original certified root selectors now drive bounded numerator/denominator proposals. Denominators are limited to 4096, candidate lists to 64 per axis, and numerators to exactly representable integers of magnitude at most 2^40. A midpoint only proposes a numerator. Outward selector inclusion and fresh homogeneous original-curve equality remain mandatory; accepted fractions never replace the original SourcePoint. Search is incomplete and cannot qualify irrational roots by approximation.

New explicit edge and material-shell entry points charge discovery and subsequent original composition/endpoint re-verification to the same caller work limit. Accepted fractions use existing optional replay metadata. Native root discovery and projective-edge replay pass, including insufficient denominator/work refusal and identical manual/automatic edge definitions. The full nurbs-core regression completed with 860 passed, 2 ignored, and no failures in 295.93 seconds. The material-shell regression is tracked separately in bounded-rational-root-discovery-2026-10-11.json.

Final fraction-filtered regression: five tests pass in 0.94 seconds, including automatic material-shell definition/vertex equality and an irrational-root refusal. The first shell run exposed empty optional metadata on two non-root edges; discovery now omits that field when it finds no candidates. Both automatic edge and shell paths refuse one-less-than-required exact work. The complete shell regression is rerun on the final version.

## Projective retained-root STEP preparation

Endpoint, trim and STEP preparation now accepts both privately admitted shared-edge types. Projective refinement updates only original source fragments, retaining world carriers, ranges, weights and rational root proposals. Fresh replay repeats original composition and endpoint identity; mapping equality is checked before any numeric representative is chosen. The same original body definition binds endpoint and trim preparation. Original curves and surfaces are serialized unchanged. Source Model conversion still refuses partial/root restrictions.

A unit tetrahedron with one source-root split passes projective material incidence, all six face-pair checks, bounded signed volume, endpoint/trim preparation and STEP output. Its original definition remains identical after preparation; endpoint and trim work exhaustion refuse output. Independent OCCT validates one solid, four faces, seven edges and five vertices. Volume differs from 1/6 by 5.551115123125783e-17 mm^3; the root vertex differs from (0.75,0,0) by 2.220446049250313e-16 mm; optimal bounds differ by approximately 1e-7 mm. The STEP and independent report are saved under projective-root-clipped-tetrahedron*. All thirteen shell incidence regressions pass on the final generic implementation. This planar control uses unit projective weights and does not qualify general nonlinear fillet bodies.

## Capped rational source-carrier exchange and local wall diagnostics

The new bounded original-carrier STEP preparation passes the admitted conical canal with unequal endpoint radii, spherical caps and privately qualified pole uses. Original body replay and eight whole-seam tangent joins remain qualified. Independent OCCT reads one valid solid from the original-carrier exchange output; volume error is 3.834783157685706e-8 mm^3 and maximum extent error is 1.0000000116860974e-7 mm. This complete-boundary control does not qualify root-clipped general nonlinear caps.

Original material-segment audits, normal-chord certificates and automatic local thin-witness search now accept native projective bodies while retaining immutable Body ownership. A 10x20x30 mm projective cuboid control qualifies a 10 mm material chord, visits six proposal faces, reports a thin witness below 11 mm and none below 9 mm, preserves original definitions, and refuses exhausted boundary/normal work. Proposal exhaustion never proves a whole-wall minimum. General projective whole-wall lower coverage and curved/root-clipped thickness remain open. Material regressions are tracked in projective-material-chord-2026-10-11.json.

## Projective whole-wall and selected-wall bounds

Body-owned lower certificates, self-face coverage, retained-domain gaps, pair anchors and mixed pair proofs now retain the native carrier type. All complete-pair enumeration, work limits, stale fact and pointer ownership checks remain active. The optional shared affine chord proof uses equal projective weights only; nonaffine maps do not acquire an affine certificate. Other conservative native paths remain available.

The projective cuboid passes 21 face/self pairs at 9.99 mm and refuses 10.01 mm, 20-of-21 enumeration and insufficient normal work. Six original self facts can be reused only at their admitted threshold and angle. Selected-wall lower and normal material upper bounds contain 10 mm, and automatic refinement reaches a width at most 1e-4 mm on the control. Original Body definitions are unchanged. A complete brep-core library regression is running; this does not yet qualify general curved/root-clipped minimum thickness or publish WASM.

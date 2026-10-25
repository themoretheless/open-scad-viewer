# Complete section-jet position reuse

The position enclosure path can reuse the position/transverse entries of a previously completed derivative jet for the same immutable bridge and exact free interval. It does not populate new caches or reuse incomplete data. Boundary delegation is unchanged and zero budget refuses before reuse. The corrected-position workspace passes its existing invocation-local cache into this path.

Regression checks compare fresh and reused base/corrected enclosures exactly across multiple across intervals, enforce zero-budget refusal, and check a different bridge owner cannot reuse source data. Full NURBS suite: 850 passed, 0 failed, 2 ignored, 24.58 s; `/tmp/section-position-reuse-nurbs-all-20261010.log`. Source hashes: `section-position-reuse-2026-10-10.json`.

A matched complete-carrier run is in progress with 10000 cells, 1000000 spans and 100000 trim cells; baseline `original-endpoint-normal-sharing-trimmed-wall-2026-10-10.json`. Report output uses an absolute path because Cargo test runs from the crate directory. The initial invocation was stopped during compilation to correct the relative output path; it is not validation. Active log: `/tmp/section-position-reuse-trimmed-wall-20261010.log`.

Specimen benefit, full B-rep qualification, and publication remain unverified. Current public WASM SHA dded25909e53fca0021abbc78b00a89e28be5869ff51a8517f9d0b6a98031adf predates this change. This optimization does not qualify unresolved wall geometry.

Additional transport/UI gate completed: expression neighbor/support tangency, inspection controller, focus handling and radius panel tests passed. Log: `/tmp/section-position-reuse-tangency-ui-20261010.log`. This gate checks transport and displayed proof states; it does not prove general surface tangency. Full native B-rep and matched specimen runs remain active.

## Completed matched specimen

The complete-carrier integration test terminated successfully (1 passed, 0 failed). Exact work-reduction comparison passed: 19007 → 18911 spans (96 saved, approximately 0.505%). All 4081 unresolved products, their ordered UV domains, reasons and exclusion counts are unchanged. An independent endpoint-containment comparison also passed with matching cell/span/trim budgets and source manifest hashes. Reports: `mapped-retained-carrier-2026-10-08/section-position-reuse-wall-regression-2026-10-10.json` and `section-position-reuse-wall-pending-2026-10-10.json` in the same directory. This does not resolve pending geometry or establish elapsed-time improvement. Full B-rep suite remains live; public WASM still predates this change.

## Live full-suite observation

The full suite has 1031 completed `ok` test lines and no FAILED lines at this observation; this is not a terminal suite result. A one-second sparse sample of PID 11018 confirms four active geometry tests: annular corrected authored radius, partial rational multispan carrier, partial rational transition contact, and restricted profile seam neighbors. Sample `/tmp/section-position-reuse-brep-sample-20261010.txt` contains only two observations per active thread and supports no hotspot percentage or ETA. Original session remains active; no restart or budget reduction.

## Next geometry investigation

The completed 4081-product report contains 2049 same-face and 2032 cross-face pending products. The largest pair is retained face 27 / transition face 28 (448 products). Native traversal limits each retained-to-transition chord query to min(remaining spans, 256), unlike same-face query caps of 100000. Existing aggregate profile_transition_checked/excluded/spans counters do not distinguish an incomplete budget refusal from a complete inconclusive exclusion. This is a hypothesis for instrumentation, not proof that raising the cap will qualify geometry. Full pending triage: `mapped-retained-carrier-2026-10-08/section-position-reuse-pending-triage-2026-10-10.json`.

## Terminal full-suite result

The original section-position-reuse binary completed: 1035 passed, 0 failed, 3 ignored in 2044.73 seconds. Log `/tmp/section-position-reuse-brep-all-20261010.log`. This binary predates transition outcome counters and the subsequent chord query order change; it does not qualify those later generations.

# Reuse of source derivatives in corrected transitions

The chord workspace now retains complete fourth-derivative source hulls used
by the transverse Hermite remainder bounds. Each key includes immutable
profile identity, exact UV-domain bits and derivative order. Entries are
bounded to 100,000 and incomplete source queries are never retained.

In `side_remainders`, the source derivative domain spans the complete section
interval at one fixed side endpoint. Only the subsequent Hermite factors
depend on the current across interval. The cache reuses the source hulls;
it recomputes those factors and the final interval intersection unchanged.
The public non-workspace path uses a fresh local cache. The chord workspace
retains both original profiles for its entire lifetime.

The rational regression verifies exact equality with the uncached first jets
on three distinct across intervals, including a collapsed interval, with less
source work. It also verifies zero-budget refusal before and after warming,
invalid-domain refusal, bounded partial computation followed by Retry, and
separate computation for a distinct profile owner. One test passed:
`/tmp/side-remainder-cache-controls-20261010.log`.

The preceding section-bridge suite passed 13 tests before this new regression
was added (`/tmp/side-remainder-cache-native-20261010.log`). The current full
NURBS suite and matched assembled-wall comparison are running separately.
The latter uses 10,000 cells, 1,000,000 spans and 100,000 trim-domain cells,
matching `original-normal-cache-trimmed-wall-2026-10-10.json`.

This source change follows completion of Rust compilation in the still-running
original-normal-cache WASM build. That build does not include this change.
No WASM qualification, whole-wall completeness or runtime speedup is claimed
for the new remainder cache until the respective results are verified.

The first full NURBS run exposed two regressions (844 passed, two failed,
two ignored): an assertion compared cached work to uncached work, and an
added zero-budget shortcut changed the existing boundary-rail error contract.
The assertion now compares interval values exactly and bounds cached work;
the shortcut was removed to preserve the original boundary behavior.
Log: `/tmp/side-remainder-cache-nurbs-all-20261010.log`. The repaired full run
is recorded separately; the first run is not a passing qualification.

The repaired full NURBS library run completed: 846 passed, zero failed, two
ignored, 91.96 seconds. Log:
`/tmp/side-remainder-cache-nurbs-all-fixed-20261010.log`. The source manifest
`mapped-retained-carrier-2026-10-08/side-remainder-cache-native-inputs-2026-10-10.json`
records all 1,811 Rust/Cargo inputs; only the section-bridge and expression-radius
files differ from the preceding original-normal-cache source manifest.

The matched assembled-wall test completed successfully in 579.06 seconds
under concurrent work. Span work falls from 35,238 to 34,842 (396 saved,
1.124%). Pending domains/reasons are exactly identical: 4,081 regions, all
budget-limited; completeness remains false. Only derivative-related stage work
changes (intrinsic, corrected pairs, normals and profile-transition pairs).
This is work reduction, not a runtime speedup or full wall qualification.
Comparison: `mapped-retained-carrier-2026-10-08/side-remainder-cache-comparison-2026-10-10.json`.
The test binary predates the zero-budget contract repair; its positive-budget
calculation is unchanged by that repair. It also predates the later fixed-shear
bound extension, which requires separate qualification.

## Follow-up diagnostic accounting

The validated accounting script produced
`mapped-retained-carrier-2026-10-08/side-remainder-cache-work-summary-2026-10-10.json`.
All 34,842 spans are attributed exactly once. Pending regions: 4,081 across
58 face pairs, all classified budget-limited. Largest pending product: faces
27/28 (448 regions). Retained pairs consume 35.42% of span work; shared-corner
proofs consume 19.95%. This gives an explicit next optimization target and
does not certify minimum physical thickness.

Separately, an intermediate stage of the live fixed-shear run reported
128 unresolved contact products: 40 changed-original-face transfers,
43 chart proofs, 45 interior-fiber support or ownership proofs. These are
distinct from the wall-budget regions. Source inspection confirms the fiber
paths require whole-chart injectivity before exact plane preimage and shared
boundary ownership. `surface_injectivity::certify_contraction` explicitly
refuses collapsed chart boundaries. An interior-only Jordan certificate
cannot replace that guard without additional boundary/pole ownership proof.
The current aggregate log does not establish how many of the 43 chart
failures are collapsed boundaries; per-face evidence is still required.
No refusal guard was weakened.

Subsequent output from the same live run resolves those intermediate contact
products: `profile paired audit pending=1`, then `transition material XY plane
audit pending=0 work=15115426 spans=52824`. Inspection of the test confirms
that complete cross-face contact qualification is asserted before obtaining
the owned embedding and entering the wall checks. The following
`expression wall pending reasons budget=435` line is the intentional
zero-cell refusal control, not the final positive-budget wall result.
Thus the 128 intermediate products are not a current final defect count for
this control body. This does not qualify arbitrary B-rep self-intersections.
The full test and positive-budget wall calculation remain live at this point.

# Joint internal seam and cross-jet search — 2026-11-03

Status: **partial; curvature cap violated; no closed-body admission**.

The bounded Rust proposal now jointly varies the internal owned seam curves,
common first and second cross derivatives, and free interior control points.
Alternating individual-row and paired symmetric/antisymmetric trials preserve
external boundary two-jet rows. Search scores are approximate proposal fitness;
independent original-surface checks determine acceptance.

## Native fixture

Quartic profile, cyclic rim origin 1, handle 0.03125 mm, radii
0.1/0.2/0.35 mm, runout 0.1 mm, span 2 mm, stock 10 mm.
Six search sweeps, 1,000,000 operation and 95,000 evaluation allowances.

All four complete internal seams pass fresh original-surface G2 checks:
582, 816, 774 and 720 cells, no pending cells, regular surfaces.
Maximum reported normal bound: 1.771914435495233e-7;
maximum curvature-tensor difference bound: 9.996653330227227e-5 /mm.

Independent point certificates still reject the absolute curvature cap 10 /mm:
internal seam 3 has curvature at least **17.100346471778508 /mm** at
UV [0.6923077943857381, 0.125]. Largest checked interior witness is
16.842708840017153 /mm. These are lower bounds, not whole-field maxima.
Unresolved points and zero lower bounds provide no positive cap evidence.

The earlier fixed-seam fixture had a seam witness of 23.6537205733 /mm;
this reduction does not qualify the new network.

## Validation and remaining work

Region replacement regression filter: **61 passed, 0 failed**.
Includes changed common curve with preserved exterior rows and G2, common
second-jet editing, and exhausted-budget rejection without input mutation.

Full exterior G2, whole-field curvature, volume, embedding, thickness and global
self-intersections remain unqualified. Original-face trimming and closed-body
sewing of this five-patch network remain unfinished. The example's subsequent
legacy single-surface sewing refusal is not a sewing test of this network.
No WASM/UI or independent STEP acceptance is claimed.

Evidence: `native-stages.log` contains diagnostic stages only; successful process
exit means the diagnostic completed, not that a CAD operation was accepted.

## Handle sensitivity follow-up

Native run completed with exit 0 for handles 0.02 and 0.04 mm, same
profile, radii, stock, origin and six joint sweeps. Both retain all four
complete internal G2 seams, but neither meets the curvature cap:

| Handle mm | Interior witness lower /mm | Seam witness lower /mm |
|---|---:|---:|
| 0.02 | 19.452095812 | 19.290763601 |
| 0.04 | 20.948521175 | 21.014144423 |

Changing handle alone in these tested cases does not fix the excess.
The 0.03125 mm candidate remains the best of these three checked cases;
this is not an optimum or whole-field certificate. Next construction work
needs more independent interior shape freedom while preserving the original
exterior two-jets, followed by fresh original-surface and solid checks.

## Degree-nine construction in progress

Added a bounded positive Cartesian V elevation from degree seven to nine,
then endpoint-jet reconciliation and the same joint seam/interior search.
The original degree-seven API remains available. Four free V rows now supply
additional independent shape variables. No tolerance or curvature cap changed.
The diagnostic flag is `--degree9-joint`; it uses three sweeps within the
same 95,000 evaluation allowance.

Added a regression covering degree-elevation point agreement, immutable
external control rows during search, literal common-root ownership and a
fresh complete original-surface G2 seam check. Its result and actual fixture
curvature are pending. Do not treat this proposal as qualified or sewn.

Running handles at this checkpoint: test session 26651, native session 66762;
logs `/tmp/osv-degree9-tests.log` and `/tmp/osv-degree9-body.log`.
The native job is queued behind the test build in the isolated target.

### Degree-nine native result

Native diagnostic completed with exit 0. Five degree-nine patches were
constructed; all four complete internal G2 seams pass (1362, 720, 1116,
846 cells, no pending cells, regular). The largest checked interior curvature
lower bound is **15.595709250593478 /mm** and the largest checked seam lower
bound is **15.386088452354874 /mm**. The cap 10 /mm is still rejected.
The reduction relative to degree seven concerns checked witnesses only,
not a whole-field maximum or a globally optimal construction.
`degree9-native-stages.log` records the original-surface diagnostics.
The following legacy sewing refusal does not test this five-patch network.

Final library regression completed: **62 passed, 0 failed** in 8.83 s,
including `degree9_network_keeps_external_rows_and_owned_g2_seam`.
The earlier pending test status above is superseded by this result.
The initial broad test command also completed with exit 0; its filter only
ran the previous 61-test library snapshot and selected matching tests.

## Actual network exterior audit

Added a native six-boundary partition against actual patch U domains. Each
world curve and both original/source and central pcurves are trimmed on the
same original parameter interval before fresh position and G2 inspection.
A missing interval or duplicate overlapping patch is rejected before checks.
No source boundary is replaced by a fitted curve.

Region-replacement library regression: **62 passed, 0 failed**, including
missing and overlapping network rejection. Native degree-nine fixture
completed: ten exterior intervals, nine proven positional matches, two
proven G2 intervals (boundary 1 / patch 3; boundary 3 / patch 3). Other G2
intervals remain Unresolved, not mismatches proven absent. Boundary 4 /
patch 1 also has unproven positional agreement. The generic inspection
uses 100,000 cells and 1,000,000 exact operations partitioned among pieces.
See `network-exterior-stages.log`.

This identifies an additional acceptance gap beyond the curvature excess:
full exterior ownership/agreement and singular endpoint G2 need qualification
on the actual new network. No sewn body, trimming completion, global field
certificate, WASM integration or STEP acceptance is claimed.

## Partial original-cell coefficient restriction

The unresolved positional interval is boundary 4 / patch 1, target side.
The original support side proves WithinTolerance; exact Different there
means nonidentical coefficients, not a tolerance violation. The target side
remains Unresolved, with no mismatch witness.

Extended the boundary Bernstein comparison to intersecting partial cells.
Both original curve and surface nets are restricted with outward affine
preimages; original fully saturated knot requirements remain. Added a
partial-cell positive regression and shifted-end rejection. **50 agreement
tests passed**. Native rerun completed: the target interval remains
Unresolved, so this extension alone does not qualify it. In particular,
the present relative-weight bound does not cancel a nonconstant common
rational denominator factor. Next work is a bounded correlated rational
coefficient comparison, not an increased positional tolerance.
No closed-body or cap admission changed.

## Correlated rational boundary residual — confirmed

Added exact rational Bernstein cross products using the original binary64
coordinates, weights and affine clock ratios. Coefficients of
`Xa*Wb - Xb*Wa` are bounded against corresponding positive coefficients of
`Wa*Wb`; the sum of axis bounds bounds Euclidean distance on the entire
cell. A nonconstant common denominator factor cancels without fitting,
source pole changes or tolerance relaxation. The proof is sufficient only;
failure/exhaustion grants no mismatch or agreement certificate.

Preprocessing is bounded by min(8192 * cell allowance, 1,000,000) arithmetic
operations and 8192-bit numerators/denominators. Remaining work is shared
across all compared original cells. Source degree is limited to 16 each.

**51 curve/surface agreement tests passed**. Regression proves a curve
represented with a nonconstant common rational factor, rejects an actual
shift, and returns no certificate under an exhausted work allowance.

The original native degree-nine fixture now confirms positional agreement
for **all ten exterior intervals**, including boundary 4 / patch 1.
Exterior G2 remains proven on two intervals and Unresolved on eight.
All four internal G2 joins remain positive; the curvature cap remains
violated. No closed-body sewing, full exterior G2, volume or embedding
acceptance follows. See `rational-boundary-native-stages.log` and
`rational-boundary-tests.txt`. This supersedes the earlier nine-of-ten
positional result only; other acceptance gaps remain.

## Actual-patch support and pole audit

Added specialized extrusion/planar support checks for the same owned network
interval partition. Collapsed-end certificates borrow the actual generated
patch, not the old single surface. Every pole trial is bounded and must
prove its original quotient; its consumed cells are included in the returned
interval report and subtracted from the support-check allowance. Each
interval retains its own refusal or unresolved report. No missing interval
can be hidden by another interval's positive result.

The native degree-nine fixture completed: all ten positional intervals
remain proven. Specialized checks prove **three of ten exterior G2
intervals**: boundary 1 / patch 3 (6 cells), boundary 3 / patch 3 (4), and
boundary 5 / patch 0 (405, including its actual collapsed-pole proof).
The last interval was Unresolved under the generic audit. Seven remain
Unresolved under the shared 100,000-cell allowance.

No full exterior G2 or closed body is admitted. Remaining work includes
correlated regularity/orientation and profile curvature bounds on these
seven actual intervals, then whole-field curvature <=10 /mm and solid
assembly/validation. `network-support-native-stages.log` records the
original-surface checks.

Final source snapshot regression: **62 passed, 0 failed**, including
network support-result interval count and per-interval aggregate cell budget
assertions. `network-support-tests.txt` records the result.

## Exact planar boundary Jacobian — confirmed

Added bounded exact rational preprocessing for actual Cartesian-transverse
planar collars. With positive boundary denominator W, the Jacobian sign is
the sign of the Bernstein polynomial
`(X_a' W - X_a W') (Y_b - X_b) - (X_b' W - X_b W') (Y_a - X_a)`.
The common positive degree/span factors are omitted, not approximated.
End-V orientation is retained. Nonnegative coefficients with at least one
positive coefficient prove a strict interior sign; zero original endpoints
are not admitted by this sign receipt. Existing actual-patch pole coverage
remains a separate requirement. Mixed signs and exhausted preprocessing
fall back to interval analysis.

Preprocessing shares min(128 * cell allowance, 1,000,000) operations across
original spans; each exact number is capped at 8192 bits, U degree at 16.
**49 G2 tests passed**, including a collapsed endpoint that cannot be
admitted without coverage, reversed orientation, an actual fold, unequal
transverse weights and exhausted work.

Native fixture completed: **5/10 exterior G2 intervals now proven**, all ten
positional matches retained. Newly positive: boundary 3 / patch 2 (4 cells)
and boundary 3 / patch 4 (171 cells including actual pole inspection).
Boundary 5 / patch 0 now needs 169 cells. All five planar intervals pass.
The remaining five extrusion/profile intervals are Unresolved. Internal
G2 remains positive; the previously established curvature excess remains.
No closed-body, global field, volume or embedding acceptance follows.
See `planar-jacobian-native-stages.log` and `planar-jacobian-tests.txt`.

## Exact original product-profile preparation

Diagnostic counters isolate the remaining uncertainty to the projected
profile comparison: source support queries are not unresolved on the native
fixture. Added exact rational preparation from literal original product
profiles and actual candidate boundary coefficients. Only saturated original
Bezier cells with literal axial product structure are eligible. Affine
source/target clocks, original centered homogeneous coefficients, degree
elevation and residual cross products are computed before outward conversion.
No fitted profile or rounded extracted source receives source credit.

The exact interval conversion is checked against its rational value; resource
exhaustion or unsupported source structure falls back to the original path.
Preparation shares min(128 * cell allowance, 1,000,000) operations across
source/candidate segments; each exact number is capped at 8192 bits.
Original shape, position, support membership and pole checks remain required.

**50 G2 tests passed**, including nonconstant-factor cancellation, detection
of an actual bend and exhausted exact-preparation allowance. An initial
module-visibility compile error was fixed before this successful final run.

Native fixture completed: positional agreement remains 10/10 and exterior
G2 remains **5/10**. Boundary 0 / patch 1 now proves regularity but its shape
difference upper bound is too wide for G2 admission. Other curved profiles
retain unresolved field queries. Thus exact coefficient cancellation alone
does not close the five curved joins. Next work is conditioned local
parameterization/jet enclosures of the same matched profile intervals,
without tolerance relaxation or geometry replacement. No curvature-cap or
closed-body acceptance changed.

## Conditioned intervals and exact profile jets — 8/10 exterior G2

Added local interval restriction and a common positive Mobius clock for
both homogeneous profiles and their residual/denominator polynomials. The
matching physical points, orientation and covered set remain the same.
Both direct and local rigorous bounds are retained/intersected; a clock
proposal alone grants no acceptance. Point-witness clocks stay unchanged.

Exact original-source preparation now also computes first-derivative
numerators F = X'W - XW' and second-derivative numerators
G = F'W - 2FW', before outward conversion. These supply an additional
jet enclosure via F/W^2 and G/W^3. Ordinary jets remain available.
An initial native attempt found numeric overflow in overwide optional jet
enclosures; the fixed version discards that optional enclosure and retains
the ordinary proof. All exact preprocessing remains under its existing
shared operation/bit limits.

**53 G2 tests passed**, including real-bend detection, zero-speed refusal,
common Mobius transport and preservation of the ordinary proof when a
conservative optional jet enclosure overflows. The first repeat build also
hit an unrelated shared-worktree private-constant access in
`minkowski3d/curved/graph_order_boolean.rs`; it now follows the existing
string error-code comparison used by `trimmed_shell.rs`. The final native
repeat compiled and completed with exit 0.

Final actual-network result: **8/10 exterior G2 intervals**, all ten
positions and all ten regularity checks proven, all four internal G2 seams
positive. Newly admitted exterior intervals: boundary 0 / patches 0 and 2
(1451 and 4199 cells), boundary 2 / patch 4 (3059). No interval has a
numeric refusal. Boundary 0 / patch 1 and boundary 4 / patch 1 remain
Unresolved with 250 and 384 pending cells. Their regularity is proven,
but shape-difference upper bounds are too wide.

The unchanged interior curvature witness is still at least
15.595709250593478 /mm, exceeding 10 /mm. No new network trimming or
closed-body sewing is complete; the following legacy single-surface sewing
refusal is not validation of this network. Next exterior work is tighter
derivative bounds for the correlated profile difference on the two patch-1
contacts, followed by shape correction and actual solid assembly/audit.
See `conditioned-profile-native-stages.log`, `conditioned-profile-tests.txt`
and `conditioned-profile.json`.

## Rational jet hull — complete exterior G2 on this fixture

Added exact first/second derivative numerators for the correlated difference
E/D as well as for each individual profile. To avoid dividing a whole-cell
numerator maximum by an unrelated denominator minimum, derivative numerator
polynomials are elevated exactly to the degree of W^2 / W^3. Matching
positive Bernstein denominator coefficients then provide a rational convex
hull on the same restricted cell. The ordinary quotient enclosure remains
available and is intersected with the rational hull when both succeed.
Unsupported/exhausted or numerically failed optional paths grant no proof.

**54 G2 tests passed**. The added regression verifies correlation across
weights differing by 20 orders of magnitude. Existing real-bend, zero-speed,
common-factor, clock-orientation and optional-overflow regressions remain
positive. Exact preprocessing remains under the same shared operation and
8192-bit limits; no positional, angular or tensor tolerance was increased.

Final native degree-nine fixture completed with exit 0: **all ten exterior
G2 intervals**, all ten positional agreements and seam regularity checks,
and **all four internal G2 joins** pass. Exterior proof consumes **1268
cells**, with **zero pending cells**. Maximum exterior normal upper bound
1.4762307932023531e-6; maximum tensor upper bound
7.629861530811035e-5 /mm (limits 1e-5 and 1e-4 /mm). The two previously
unresolved patch-1 contacts now pass in 23 and 50 cells.

This is a complete seam result for the named fixture only. It does not
prove regularity/injectivity of whole patches, global self-intersections,
volume, thickness or a general arbitrary-B-rep fillet. The unchanged interior
curvature witness remains **at least 15.595709250593478 /mm**, above 10 /mm.
The new network is not yet sewn into a closed body. The example still calls
the legacy single-surface sewing constructor afterwards; its internal G2
refusal is not a rejection or validation of this five-patch network.
Next work: topology/trim assembly for this actual network and joint shape
correction under the curvature cap, followed by whole-body qualification.
See `rational-jet-hull-native-stages.log`, `rational-jet-hull-tests.txt` and
`rational-jet-hull.json`. This supersedes the earlier 8/10 seam result.

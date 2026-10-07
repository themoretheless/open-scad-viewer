# Completion audit: progressive sweep/miter

The original six requirements remain the completion contract. Selected passing
fixtures do not close an all-mode requirement. This audit points to evidence and
open obligations; it does not replace the contract in
`../../design/sweep-contract-matrix.md`.

| Original requirement | Available evidence | Open obligation |
| --- | --- | --- |
| Affine + authored frame + guides through native, WASM, Rush, viewport and Solid | Joint-mode fixtures and STEP cases; `sweep-periodic-domain.md` covers a separate periodic source route | Every applicable declared combination, including independent law domains and refusals, must be covered; selected combinations alone are insufficient |
| Full continuousBound: walls, frames, miter, decomposition, filled caps and correction | Exact periodic/unclamped selected bodies certify complete constructor bounds; native interval and retained-domain audits have separate premises | Arbitrary rounded source contour topology; general moving-frame transport and corrections; a complete bound for every declared mode |
| Regularity, intersections, holes, nesting and orientation in all modes | Selected boundary embedding and cavity-role fixtures, STEP topology checks | All-mode continuous regularity and global embedding; unsupported or exhausted predicates must retain explicit unresolved status |
| Applicable G1/G2, moving frames, multispan and closed seams; sharp miter C0 | Exact periodic fixture's eight profile seams and work-budget refusal; sharp corners retain C0 | General along-path and moving-frame seams, filled-cap joins and closed corrected seams; profile G2 is not whole-boundary G2 |
| Independent STEP matrix, geometry/topology/volume | `external-step-authored-cap-mode/manifest.json` and its `opencascade-sweep.json` contain 38 passing cases on the authored-cap WASM artifact | All applicable combinations and artifact-specific requalification after new WASM packaging; finite material probes remain sampled |
| Wide/narrow UI success/refusal, active build/Solid cancellation, source change, restore | Periodic success/refusal, active Solid cancellation in both layouts, narrow 32-segment retry, Undo and reload restoration in `sweep-periodic-domain.md` | Complete cross-mode viewport-build and source-change matrix; active Solid request cancellation does not prove cancellation inside a specific Rust instruction |

## Current in-flight change

Periodic single-curve wall-loop closure no longer trusts the periodic flag.
Native exact extraction tests passed 2/2 and sweep tests passed 153/153. The new
public adapter regression reproduces the missing refusal on the previous WASM.
The current build is recorded in `sweep-periodic-wall-closure-wasm.log`; all five
public suites subsequently passed 45/45 on artifact SHA256
`675feaabbe364be7c4dcf9e57934ef960999010d316af316877df9c85e3a8099`.
Fresh STEP export and independent OCCT verification subsequently passed 36/36
on this artifact in `external-step-periodic-wall-closure`, including 520
whole-domain cap coedge checks. This requalifies the selected matrix only.
The historical 36-case STEP
report must not be attributed to a newly built artifact without requalification.

The goal is incomplete. No row above supplies evidence of full completion.

## Joint periodic follow-up

The new periodic hollow + authored frame + orientation guide + affine scale and
center-offset Rush fixture has a complete within-budget boundary certificate and
passes real worker-to-Solid validation. Collapsed-axis, parallel-guide and zero
scale refusals are checked with native reasons and immutable source graphs.
The two focused public suites passed 23/23. The independent STEP matrix in
`external-step-periodic-joint` passed 37/37 with 536 cap coedge checks and the
new case's analytic 125/4 mm³ volume. This closes one previously absent joint
combination, not the general moving-frame or all-mode obligations. Actual wide/narrow UI success, zero-scale refusal and corrected rebuild now
pass for this joint fixture. Reopening preserves the exact source, rebuilding
keeps one group, and Undo plus a fresh browser load restores only the original
model and Body 1 B-rep with no selection. Evidence and screenshots are in
`sweep-periodic-domain.md`. Joint-mode active cancellation and viewport/source
change coverage remain open.

## Moving longitudinal axis with joint laws

A new native regression varies the authored longitudinal axis from Z to
[0,0.5,1] while an independent-domain guide determines the projected normal.
It combines nonzero varying twist, affine transverse scale and side/longitudinal
center offsets. The retained interpolation certificate is checked against an
independent closed-form trajectory at interior profile and path parameters.
Those evaluations are falsification checks, not the continuous proof itself.
The native interval certificate supplies the upper bound. Zero frame work
budget leaves the error unresolved; corrupting an interior retained control by
0.125 is included in the returned upper bound. Evidence: test
`moving_longitudinal_guide_twist_affine_bound_covers_retained_interpolation` and
`sweep-moving-axis-joint-native.log`. This is native retained-wall E evidence;
it does not qualify filled caps, global embedding, G1/G2, the general rational
path moving-frame modes, or this new combination through public WASM/UI/STEP.
The change adds a native test only and does not change packaged WASM geometry.

The same moving-axis joint trajectory now passes through the public WASM
adapter. The public test converts the authored 0.25-radian endpoint to degrees
(the adapter's contract), checks interior retained points against the independent
formula, rejects an error tolerance below the certified bound, refuses a zero
longitudinal axis with the native reason, and preserves all source inputs.
An initial oracle failure used radians directly as public degrees; that test
unit error was corrected. The first suite run passed the other 31 tests; the
corrected new test and tolerance refusal passed the focused rerun. Logs:
`sweep-moving-axis-joint-public.log` (initial run) and
`sweep-moving-axis-joint-public-focused.log` (corrected test). This remains
retained-wall error qualification, with `continuousBound:false` deliberately
asserted; Rush/Solid/STEP for this particular trajectory remain open.

## Moving-axis hollow Rush route: seam identity repair in flight

The new `miter-periodic-moving-axis-guide-affine-hollow.r` attempts the full
constructor with periodic hollow profiles and varying axis, guide, twist, scale
and center. Its public Solid-admission test currently fails on the packaged
WASM at exact active seam ownership (`sweep-moving-axis-joint-rush.log`).
Diagnosis: the ownership gate unnecessarily requires every extracted Bernstein
pole to be representable exactly in binary64. A coincident rational seam can
remain exact when its common endpoint mean is not representable.

Native ownership now also accepts a single-curve seam when both endpoint basis
stencils translate by the exact active-domain period and corresponding original
controls and weights match. Strict adjacent knots establish the same one-sided
endpoint convention. This proves seam identity only; it does not promote any
regularity, intersection or cap-domain predicate. A nonrepresentable mean
regression passes, while a 1-ULP exterior knot and mismatched seam weight refuse.
Evidence: `sweep-moving-axis-seam-native.log`. Packaging and the new positive
Rush admission remain in flight in `sweep-moving-axis-seam-wasm.log`; the failing
public test is retained as the target and must not be reported passed.

The nonrepresentable endpoint regression also exercises the actual native wall
ownership gate with zero downstream audit budgets; it passes. The public seam
regression now includes a matching-stencil nonrepresentable mean and a retained
weight mismatch, in addition to the existing one-ULP knot refusal. These public
assertions await the active packaging process; no passing public result is
claimed yet. Rust release compilation completed; Binaryen optimization is live.

The translated-stencil proof is recorded in `sweep-active-seam-identity.md`.
Native tests now additionally cover degrees 1–6, unequal positive rational
weights and individual pole/weight/knot mutations; all three exact-curve
segment tests passed (`sweep-moving-axis-seam-native.log`). The source changes
since release compilation add tests and documentation only. The same Binaryen
process remains active; public artifact requalification is still pending.

## Packaged moving-axis result

Packaging completed with SHA256
`e2817a686d9d7ced0a6a3ab3b8ee1e43b6a937814caa4714e84626244b190f0c`,
10,744,604 bytes. Public exact seam ownership now passes, including the
nonrepresentable mean. The new moving-axis Rush body now constructs instead
of failing that gate. It does not yet qualify for Solid: filled-cap error is
`filled-cap-bound-unproved`, retained cap decomposition reports
`cap-region-unproved`, retained cap identity reports
`unsupported-section-decomposition`, and boundary pairs remain unclassified.
Wall decomposition and ideal cap source domains are certified. Compact evidence
is in `sweep-moving-axis-rush-diagnostic.json`; the initial positive admission
assertion failed in `sweep-moving-axis-seam-public.log`.

The regression now asserts the correct unresolved result and actual Solid
refusal. Successful filled-cap qualification/admission remains an explicit
`it.todo`, not a completed requirement. The new artifact also requires fresh
STEP requalification; historical 37-case results belong to the old artifact.

The final two public suites passed 41 tests with 1 explicit TODO on this
artifact (`sweep-moving-axis-seam-public-final.log`). The TODO is the successful
moving-axis periodic filled-cap/Solid qualification, not an ignored completion.

## Periodic endpoint correction route

The explicit correction variant
`miter-periodic-moving-axis-guide-affine-hollow-corrected.r` initially refused
at a TS blanket periodic-basis exclusion. Native `curve_project_section`
already supports positive periodic rational bases and preserves knots, weights
and the periodic flag. The adapter now requires matching periodic flags between
stations instead of rejecting every periodic curve. Public projection regression
checks bounded correction, exact planar controls, unchanged basis and source,
and refusal of mixed periodic flags. Geometry projection stays in Rust.

The corrected moving-axis example now advances past filled-cap checks but
refuses retained wall regularity (`sweep-moving-axis-cap-correction.log`).
The 1 mm correction tolerance is explicit in that separate fixture; it does
not change the 0.01 mm uncorrected target or its open obligation. Projection
onto path-normal caps can move this tilted endpoint substantially; no regular
wall or Solid qualification is claimed. A public regression preserves this
specific refusal and immutable source graph. The general intended route still
needs a cap construction/correction compatible with the moving authored plane
and all continuous error, regularity and global predicates.

## Authored-plane native correction primitive

Rust `section_projection::project_authored_axis` now constructs a candidate
plane normal from an independently parameterized authored axis, places its
offset near a retained pole on the requested grid, and delegates to the existing
exact-plane and whole-rational-curve displacement proof. Candidate arithmetic
is not accepted as a certificate. Degenerate axis, missing work and excessive
displacement remain refusal paths. The operation preserves the periodic flag,
knots, weights and repeated seam controls.

On a tilted periodic section with a non-dyadic translated plane origin, native
correction is below 1e-9; projecting the same section onto the path's flat end
plane exceeds that tolerance. Both section-projection native tests, including
transport dispatch, passed in `sweep-authored-cap-plane-native.log`.
`vue-tsc --noEmit` passed (`sweep-authored-cap-plane-types.log`). The public
adapter `projectAuthoredNurbsSection` and a public projection regression are
prepared; packaging is running in `sweep-authored-cap-plane-wasm.log`.

This primitive does not yet change Rush cap-correction selection or qualify the
full moving-axis hollow body. Default path-plane correction semantics remain
unchanged. Native exact retained cap regions, complete error composition, wall
regularity and global boundary predicates must still qualify the integrated
authored-plane route before Solid admission or STEP success can be claimed.

## Explicit authored-plane mode qualified through Rush/Solid

`cap_correction_authored_frame:true` is now an optional progressive-miter Rush
field, exported schema field, runtime boolean and body-constructor option. It
selects the native authored-axis section projection at each independent law
endpoint; default path-plane correction is preserved. An authored axis and an
explicit correction tolerance are required. Both endpoint corrections share
work and retain the existing correction/error/regularity/material gates.
The exported schema was regenerated with the repository generator; runtime
scalar-expression processing excludes this boolean field. Native lower tests
passed 3/3, runtime compatibility tests passed 56/56 and type checking passed.

`miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r` retains the
original changing axis, guide, twist, scale and side/axial center laws. Its
correction tolerance is 1e-9 mm and total boundary tolerance remains 0.01 mm.
The initial 1000-cell retained-wall audit budget was insufficient. Explicit
10000-cell work now proves the retained charts without changing geometry or
tolerance. The complete constructor boundary certificate is within budget
(`continuousBound:true`) and actual Solid admission passes.

Negative tests retain rejection for 1000 retained-chart cells, zero correction
work, missing correction tolerance and the default path-plane correction on
this tilted cap. All input graphs remain unchanged. Three final public suites
passed 34 tests with one explicit TODO in `sweep-authored-cap-mode-public-final.log`.
The TODO is still the uncorrected moving-axis body, not this explicitly
corrected route. Evidence also includes `sweep-authored-cap-mode-body.log`,
`sweep-authored-cap-mode-rush-native.log`, `sweep-authored-cap-mode-runtime-native.log`
and `sweep-authored-cap-mode-types.log`.

Current geometry artifact SHA256 is
`bb255c89ec35d3249128347ae883f56e5f48f359f15ee81efb58dfacc5e99de8`,
10,747,592 bytes (`sweep-authored-cap-plane-wasm.log`). Language WASM was rebuilt
with the new boolean/runtime/schema route.

Receiving-document validation now accepts this 130-face body and rejects zero
correction work without publishing a body. `sweep-authored-cap-mode-worker.log`
records 17/17 passing tests. These execute real Solid construction and receiving
validation through a fake worker transport; browser Worker execution remains a
separate UI obligation.

The fresh `external-step-authored-cap-mode/opencascade-sweep.json` records 38/38
passing independent STEP cases against the current manifest and geometry WASM,
including 552 full-domain cap coedge uses. The new moving-axis case has 130
faces, one solid, one closed shell and two cap-hole faces. Its independent exact
polynomial boundary integral gives 22.92781437975149 mm³; OCCT gives
22.92781437975148 mm³ (relative error 3.0990426038498033e-16). The reference uses
exact rational arithmetic on polynomial wall and planar cap controls without
kernel, STEP or sampling calls. Its self-test covers a cube, a hollow prism,
translation, and rejection of open/nonplanar cap contours. Oracle metadata now
reverses both coedge directions and order for reversed shell-face wires;
export, reference calculation and OCCT were repeated after that correction.
Material-side probes remain sampled and do not establish universal containment.

Reproduce in order: export with `scripts/export-sweep-step-oracle.mts`, calculate
the manifest reference using `scripts/reference-sweep-generator-volume.py`, then
run `scripts/verify-sweep-step-occt.py` with the OCCT interpreter. The reference
runner also supports `--self-test`; OCCT refuses a missing exact volume reference.
Logs are `sweep-authored-cap-mode-step-export.log`,
`sweep-authored-cap-mode-volume-reference-tests.log`,
`sweep-authored-cap-mode-volume-reference.log` and
`sweep-authored-cap-mode-step-occt.log`.

Wide/narrow live UI for this particular corrected moving-axis route remains to
be qualified. Full all-mode E/R/I/J, applicable G1/G2 and the six original
completion obligations remain open.

## Authored moving-axis live UI: wide window

The real localhost UI successfully built the authored-cap fixture as a new
B-rep group in Solid. Reopening its source confirmed byte-for-byte equality
with the fixture. Setting correction work to zero displayed
`Miter authored cap correction unproved: work-limit`, retained the existing
body and kept the dialog editable. Restoring the original source successfully
rebuilt the group; the scene remained at two bodies (original plus test),
without an extra body. `sweep-authored-cap-ui-wide.png` records the restored
Solid. Two Undo actions removed the rebuild and test creation, returning to
the original one-body scene. Narrow-window coverage and active cancellation
for this moving-axis route remain pending. This does not qualify all modes.

## Authored moving-axis live UI: narrow Solid cancellation/retry

At an explicit 720×900 viewport, the File menu opened the code-group dialog.
The authored fixture was submitted; the Build button visibly became disabled
with pending indicator before Cancel was clicked. The dialog closed, Undo
remained disabled, and the original scene remained. A second submission of the
same fixture completed and selected the new group in Solid.
`sweep-authored-cap-ui-narrow.png` records that narrow result. Resetting the
viewport revealed exactly two B-rep bodies/groups, original plus test, with no
late publication from the cancelled request. One Undo restored the one-body
original scene. The temporary viewport was reset and the test tab closed.
This proves cancellation of an active UI request, not interruption at a
specific Rust instruction. Narrow refusal/source-restoration and viewport-build
cancellation for this route, plus wide active Solid cancellation, remain open.

## Current-artifact smooth-station regression

On the current packaged WASM, `miterStationSmoothness.test.ts` and
`miterSmoothStationWalls.test.ts` passed 12/12 tests; evidence is
`sweep-authored-cap-smoothness-regression.log`. These cover straight and unequal
spans, distinction between G1 and unresolved G2, exact-work exhaustion, malformed
station pcurves, reconstructed moving-frame hollow Solid, shared quintic jets,
constructor-owned corrected sections and preservation of sharp original
stations as C0. The moving-frame reconstruction fixture requires its declared
2 mm boundary tolerance; its 0.1 mm refusal remains tested. This does not prove
G1/G2 for arbitrary authored laws or arbitrary rational moving paths.

## Hidden moving-frame singularities: native regression

The native authored-frame certificate rejects a quadratic longitudinal axis
whose magnitude is `(t-3/8)^2`: all quarter-grid preview stations are nonzero,
but the exact interior zero prevents a continuous nonzero-axis proof. A second
case keeps the transverse vector nonzero while making it parallel to the
longitudinal axis at that same interior parameter. Independent authored domains
are retained. Both return unresolved with no partial normalized frame. The new
native test passed 1/1 in `sweep-hidden-frame-singularity-native.log`; this
changes tests only and requires no WASM rebuild.

The general rational-path implementation in `progressive_sweep.rs` still
accepts refinement based on sampled control deviation and explicitly documents
that this is not a continuous RMF/injectivity/rounding-inclusive certificate.
Its sampled acceptance must not be promoted to a full continuous guarantee.
Connecting original-law interval certificates to that retained construction
remains a required production change.

## Whole-traversal authored-frame regularity: native implementation

`authored_frame_certificate::certify_regularity` now adaptively partitions the
entire normalized traversal, proving every accepted closed interval with
original-law interval jets. All restrictions charge one shared cell budget;
exhaustion and unrepresentable subdivision remain unresolved. A rotating
nonzero axis whose full-domain box includes zero now certifies after subdivision;
the hidden interior parallel direction still refuses. The certificate-module
tests passed 7/7 in `sweep-whole-frame-regularity-native.log`.

`Sweep::authored_frame_regularity` exposes this premise on the general native
sweep without modifying sampled preview acceptance or claiming a complete
continuous bound. No report is supplied for non-authored transport modes.
WASM/transport integration, retained-wall error composition, caps and global
embedding remain pending; the existing packaged WASM does not contain this
new native method.

The general-sweep integration test and existing authored regressions subsequently
passed 33/33 (`sweep-whole-frame-regularity-integration.log`). Native transport
now exposes `sweep_authored_frame_regularity`, returning `regularityCertified`,
shared work, certified interval count and unresolved reason. Its response
explicitly retains `continuousBound:false`; it cannot substitute for a complete
retained-boundary certificate. The transport regression passed 1/1 in
`sweep-whole-frame-regularity-transport.log`.

A thin public adapter, `nurbsAuthoredFrameRegularity.ts`, forwards original laws
and the cell budget to Rust. Public regressions cover subdivision, budget
exhaustion, immutable inputs and an interior zero missed by quarter-grid
stations. Type checking passed (`sweep-whole-frame-regularity-types.log`).
Geometry WASM packaging is running (`sweep-whole-frame-regularity-wasm.log`);
public regressions and artifact-specific STEP requalification remain pending
until the new artifact is packaged. No new public proof is claimed yet.

The public general authored-sweep construction, multi-profile construction and
preview adapters now attach the separate `authoredFrameRegularity` Rust report
to their reports/levels. `frameRegularityMaxCells` controls its work budget;
`continuousBound` and sampled acceptance remain unchanged. Report-integration
tests were added, and type checking passed. Public execution awaits packaging.
The current release compile finished in 1m13s and its Binaryen optimizer remains
active. A second observed build belongs to the separate loft-publication
worktree, not this checkout; it was left untouched.

The existing Rush authored-progressive-sweep regression now also asserts that
the construction report retains `authoredFrameRegularity.regularityCertified`
while both the nested and whole-sweep `continuousBound` remain false. This
assertion is pending public execution on the new packaged artifact. Binaryen
is confirmed live and CPU-active; the existing build was not restarted.

Pending public regressions additionally cover invalid cell budgets and streaming
preview ownership: mutating a yielded regularity report must not change final
construction evidence. The general-sweep stream's return type now identifies
its actual general-sweep report rather than the general/miter union. Type
checking passed before that annotation refinement; public execution still waits
for the confirmed live Binaryen process.

Type checking also passed after the streaming return-type refinement. A public
rational multispan/independent-domain regularity regression was added. Its
execution, like the other new public assertions, remains pending packaging.
The same Binaryen process is verified CPU-active after more than six minutes;
this is a live build wait, not an inferred stalled build.

## Whole-frame regularity: packaged public qualification

Geometry WASM packaging completed successfully. The new artifact SHA256 is
`423ada9414dac68f3169a21015ca3212b29fb759f2b9b2afb80f0120b3a73d93`,
10,749,933 bytes (`sweep-whole-frame-regularity-wasm.log`). Public regularity
and general progressive-sweep suites passed 31/31 on this artifact
(`sweep-whole-frame-regularity-public.log`), including adaptive subdivision,
rational multispan laws, independent domains, invalid and exhausted budgets,
hidden interior singularity, construction/preview/stream report propagation,
preview-copy immutability and Rush construction-report propagation. All continue
to retain `continuousBound:false` for general progressive sweep.

Receiving-Solid and retained-decomposition regressions passed 32 tests with
one explicit TODO (`sweep-whole-frame-regularity-solid-regression.log`).
Fresh export to `external-step-whole-frame-regularity` is running. The previous 38/38 STEP
report remains evidence for its original bb255c89 artifact, not this new one.
Independent reference-volume and OCCT verification of the new export remain
pending. Full error composition/global embedding/UI obligations stay open.

## New artifact STEP requalification completed

Fresh export, independent exact reference-volume calculation and OCCT
verification completed successfully for artifact 423ada9414. The report in
`external-step-whole-frame-regularity/opencascade-sweep.json` passes 38/38,
including 552 full-domain cap coedge uses. Its manifest SHA256 was checked
against the current manifest bytes, and its artifact provenance records the
new geometry SHA256, 10,749,933 bytes and verified public/packed identity.
The moving-axis authored-cap body retains the same independent volume
22.92781437975149 mm³ (OCCT 22.92781437975148 mm³). Logs are
`sweep-whole-frame-regularity-step-export.log`,
`sweep-whole-frame-regularity-volume-reference.log` and
`sweep-whole-frame-regularity-step-occt.log`. This requalifies the selected
38-case matrix; it does not close the original all-mode completion contract.

## Native complete authored-frame jet cover

The adaptive whole-frame proof now retains its complete ordered interval cover
with longitudinal, transverse and binormal value/first/second-derivative
enclosures. Certified status publishes this cover only after every traversal
interval succeeds. Budget exhaustion publishes no partial cover. The focused
native test proves that returned intervals cover exactly [0,1] with matching
adjacent endpoints and verifies absence of partial jets on exhausted work; it
passed 1/1 in `sweep-whole-frame-jet-cover-native.log`.

This native premise prepares retained-wall interpolation-error composition:
path/scale/twist/affine terms and initial profile-coordinate rounding must be
combined on the same restricted traversal before a wall error can be claimed.
Those compositions are still pending. The new internal cover is not included
in the previously packaged artifact or attributed to its public/STEP results.

## Native twist composed into the complete frame cover

`certify_twisted_cover` now shares the adaptive-cover implementation and cell
budget across authored axis, normal and twist laws. Each accepted interval
retains normalized-traversal frame value, first and second derivatives.
`Sweep::authored_frame_jet_cover` supplies this premise to the general native
sweep. A fixed-axis rotation with independent law domains is checked against
analytic value/first/second jets; exhausted budgets retain no cover.
The authored-focused native suite passed 34/34 in
`sweep-twisted-frame-jet-cover-native.log`. These additions are native-only and
not attributed to the previously packaged public artifact.

Remaining wall-bound composition must combine path translation, scalar scale,
affine scale/center and these twisted-frame jets, then account for initial
profile coordinates and actual retained endpoint rounding. Caps, arc-length
correspondence, non-authored moving frames and global embedding remain separate
original completion obligations.

## Native original-law control trajectory composition

`authored_frame_certificate::certify_control_trajectory` now composes original
path translation, uniform scale, optional affine axis scale/center and twisted
authored-frame jets on one restricted normalized traversal. It returns value,
first and second derivatives with a shared work budget and no partial jet on
unresolved work. Local ideal-profile coordinate intervals remain an explicit
premise. The independent quadratic/linear analytic trajectory regression passed
1/1 (`sweep-control-trajectory-native.log`), including independent domains and
zero/insufficient work refusal. The authored-focused regression passed 35/35 in
`sweep-control-trajectory-regression-native.log`. The new file was formatted.

This is not yet a retained-wall error certificate: constructor-owned initial
coordinate enclosures, actual retained endpoint displacement and interpolation
remainder still need composition. Arc-length correspondence, non-authored
frames, caps/global embedding and public integration remain required.
The implementation is native-only and not attributed to artifact 423ada9414.

## Native owned initial-coordinate and trajectory premises

`Sweep::authored_initial_coordinates` now encloses ideal initial coordinates
from the original profile poles, original path start and un-twisted authored
frame. Outward subtraction/dot products retain rounding uncertainty; twist and
affine transformation occur after this local basis as in construction. A
translated path and oblique authored normal with independent domains certify
the expected local coordinates; insufficient work publishes no coordinates.
The focused test passed 1/1 (`sweep-initial-coordinate-enclosure-native.log`).

`Sweep::authored_control_trajectory` now combines those owned coordinate
enclosures with the original-law trajectory jets under one shared budget.
The analytic translated/rotated control trajectory, derivatives, exhausted
work and invalid profile index are checked. Arc-length correspondence and
closed-frame correction explicitly remain unresolved. The authored-focused
native regression passed 36/36 (`sweep-owned-control-trajectory-native.log`).
The new module is formatted. These native additions are not attributed to
packaged WASM 423ada9414. Retained endpoint displacement, interpolation
remainder, non-authored transport and the complete boundary/global obligations
remain required.

## Native point-safe control values and retained displacement

`certify_control_value` composes the original trajectory at point restrictions
without derivative division on subnormal widths. Tests cover exact endpoints
and an interior traversal with independent domains, including all affine
terms and the shared budget. The focused regression passed 1/1
(`sweep-control-endpoint-value-native.log`).

`ControlValueReport::retained_displacement_upper` now supplies an outward
Euclidean bound from a corresponding retained binary64 control to the ideal
control enclosure. A deliberately displaced pole by 0.125 is covered, while
unresolved reports supply no bound. The authored-focused native suite passed
36/36 (`sweep-control-endpoint-displacement-native.log`). Source formatting and
scoped diff checks passed. Native-only additions remain separate from the
packaged artifact's evidence. Constructor-owned station correspondence,
interpolation remainder and the full wall/boundary composition remain pending.

## Native linear control interpolation remainder

Trajectory reports now bind their jets to the original normalized traversal
interval. `linear_remainder_upper` uses outward arithmetic for
`h² sup ||x″|| / 8`, only when the original-law restrictions certify a single
smooth span. `linear_error_upper` adds the maximum of the two endpoint
displacement bounds by convex interpolation. It rejects negative/nonfinite
endpoint premises and supplies no remainder for unresolved jets or unproved
knot transitions. The quadratic analytic case has remainder 0.5; adding a
0.125 endpoint displacement gives an upper bound at least 0.625.

The authored regression passed 36/36
(`sweep-control-linear-remainder-native.log`). A subsequent actual piecewise
linear path with an internal derivative jump retains no global remainder,
even though each separate path piece has zero second derivative. That focused
regression passed 1/1 (`sweep-control-knot-remainder-refusal-native.log`).
Both touched helper files were formatted. Actual constructor-owned retained
station correspondence, profile decomposition and whole-wall aggregation still
need to consume these premises. All-mode/full-boundary obligations remain open.

## Native constructor-owned section interpolation bound

`Sweep::authored_section_interpolation_bound` now consumes actual native
sections. It verifies unchanged positive rational profile basis, proves each
retained control's station displacement against the original ideal trajectory,
and combines interval remainders and endpoint errors across every control and
station interval with one shared budget. No partial error bound is published
on unresolved work. The profile-wise contraction uses positive rational basis
weights; retained profile decomposition is still a separate obligation.

The owned rational-profile/rounding/budget regression passed 1/1 in
`sweep-owned-section-interpolation-native.log`. The varying joint-law case
(frame normal, twist, scale, affine scale and center) also passed 1/1 in
`sweep-joint-section-interpolation-native.log`, comparing against an independent
trajectory and verifying that the certified bound decreases from 3 to 9
sections. Those evaluations are falsification checks; interval jets and the
linear remainder supply the continuous proof. The general-sweep regression passed 33/33 in
`sweep-owned-section-regression-native.log`. These changes remain
native-only; full retained-patch/profile-decomposition composition, caps,
arc-length/closed/non-authored modes and global qualification remain open.

## Native retained patch error includes profile decomposition

`Sweep::authored_patch_error_bound` now combines the owned section
interpolation bound with the original-span/retained-Bezier decomposition
certificate. The common `profile_parts` constructor supplies both retained
construction and qualification. Matching degree/knots/weights/periodicity
across all decomposed stations proves that ruled V interpolation preserves
the maximum section-decomposition displacement. Final patches are returned
with the bound only after complete qualification. Product work is separately
bounded and charged across every original section/span.

The general-sweep regression passed 33/33
(`sweep-owned-patch-error-native.log`). A subsequent 33-control rational dense
profile with alternating weights verifies all 32 retained patches against an
independent rotated/translated trajectory, checks nonzero decomposition error
and 384 charged products, and refuses the 383-product budget without patches
or a partial error bound. It passed 1/1
(`sweep-owned-dense-patch-error-native.log`). Evaluations are falsification
checks; the interval/decomposition certificates provide continuous bounds.

This closes the selected native open, parameter-spaced authored patch-error
composition. It is not all-mode proof or a complete capped body guarantee.
Native level-report/WASM/Rush integration, caps, smooth knot transitions,
arc-length, closed/non-authored moving frames and global embedding remain
open under the original six requirements. Public artifact 423ada9414 does
not yet contain this new patch-error path.

## Native level and transport integration of authored patch error

General-sweep level reports now carry continuous retained-patch error, interval
cell/product work, unresolved reason and explicit retained-patch scope.
`continuousBound`/`roundingCertified` become true only for the complete native
patch certificate. A known certified error exceeding the tolerance forces
continued refinement instead of sampled acceptance. Unsupported/unresolved
modes retain sampled preview semantics and false continuous-bound evidence;
this is not capped-body or global material admission.

Multi-profile levels now share 10000 interval cells and 1000000 products across
the entire level. Aggregate continuous error is published only if every
profile certifies. The first native run had one obsolete false-bound metadata
expectation; after updating the transport assertion, 34/34 tests passed in
`sweep-native-level-error-integration-final.log`. A subsequent explicit shared
budget test passed 1/1 (`sweep-native-level-shared-budget.log`). Type checking
passed (`sweep-native-level-error-types.log`). Public types and assertions are
prepared for the new metadata; new WASM packaging/execution is still pending.
Artifact 423ada9414 and its 38/38 STEP report remain historical evidence for
the previous implementation. Caps, other declared modes and global proofs
remain open.

### Retained-patch scope versus complete viewport boundary evidence

Inspected `readSweepViewportEvidence` and boundary-union composition: complete viewport certification requires the separate `retained-sweep-boundary-union` certificate with matching `boundary-set-hausdorff` scope and numeric evidence. Added a regression using the new authored retained-patch report (`continuousBound: true`) as capped-body evidence; it remains uncertified for both complete boundary and Solid geometry. `tests/sweepViewportEvidence.test.ts`: 7/7 passed; scoped diff check passed.

Updated geometry WASM build started with log `/tmp/sweep-authored-patch-error-wasm.log`; process session 50810 was verified live. Public authored-sweep tests and new-artifact STEP qualification remain pending build completion. This is not complete all-mode qualification.

### Public authored patch-error regression preparation

Added `tests/nurbsAuthoredPatchError.test.ts` for independently parameterized authored-frame, twist, uniform scale, affine scale and center laws with a rational profile. It checks certificate transport, refinement reduction and input immutability. A separate path-knot case requires an explicit `section-knot-transition-unproved` result rather than a false global remainder. TypeScript check passed (`/tmp/sweep-authored-patch-public-types.log`); execution remains pending the updated WASM. Rust release build finished in 1m09s; Binaryen optimization was verified live at 100% CPU (PID 16302, build session 50810).

### Authored preview worker admission transport

Found and fixed a worker integration gap: ordinary authored sweep previews now include a non-null `continuousErrorUpper` in the existing bounded worker event contract. Without this field, native rejection by continuous error could disagree with the protocol's sampled-only admission reconstruction. Unsupported/unresolved authored bounds are omitted; the separate miter route keeps its existing metadata. Added protocol regression for sampled error inside budget with certified patch error outside budget, accepted inside-budget evidence, and rejection of lost evidence. Protocol tests 21/21 passed; TypeScript and scoped diff check passed. Logs: `/tmp/sweep-authored-preview-protocol.log`, `/tmp/sweep-authored-preview-types.log`. Actual new-WASM worker execution remains pending optimization.

### Continuous-error refusal diagnostics and remaining aggregation case

Rush construction refusal now identifies continuous retained-patch error when its certified upper bound exceeds budget; sampled-only refusals retain their existing diagnostic. TypeScript and scoped diff checks passed (`/tmp/sweep-authored-refusal-types.log`); actual new-artifact execution remains pending.

Inspection also identified a remaining multi-profile transport case to address: aggregate admission ANDs individual admissions, while aggregate `continuousErrorUpper` becomes null if any profile certificate is unresolved. A known continuous-error rejection in one profile can therefore be lost as numeric evidence if a later profile exhausts shared work. The worker needs explicit scoped partial-bound evidence, without promoting this to a complete aggregate certificate. This case remains open; the goal is not complete. Existing build session 50810 remains live in Binaryen optimization (PID 16302).

### Rust partial-profile admission evidence

Added native `known_profile_error_upper` / serialized `knownProfileErrorUpper`: maximum among individually certified profile bounds, retained even if another profile exhausts the shared certificate budget. Complete `continuousErrorUpper` and `continuousBound` still require every profile. Worker preview admission and Rush refusal diagnostics now consume the partial bound for conservative budget refusal only. Regression uses 32 profiles, zero sampled error, strict positive budget, shared work exhaustion, null complete bound and retained known error above budget: native 1/1 passed (`/tmp/sweep-partial-profile-error-native.log`). TypeScript and scoped diff checks passed. Matching public regression prepared in `tests/nurbsAuthoredPatchError.test.ts`.

The active WASM optimization began before this native field was added; its output will be an intermediate artifact, not the complete current-source artifact. Let the live build finish, verify the intermediate source scope, then rebuild with this fix before final public/STEP claims. Session 50810 / PID 16302 was confirmed live; no build restart occurred.

### Separate worker field for partial profile bounds

Refined worker transport: `knownProfileErrorUpper` is an explicit optional partial-profile field, separate from complete `continuousErrorUpper`. The bounded event validator checks finite nonnegative values and includes both applicable bounds in conservative admission. Regression verifies partial-bound rejection without inventing a complete bound, successful in-budget partial evidence and invalid numeric refusal. Native progressive sweep suite 35/35 passed; worker protocol 22/22 passed; TypeScript and scoped diff checks passed. Logs `/tmp/sweep-partial-profile-native-all.log`, `/tmp/sweep-partial-profile-protocol.log`, `/tmp/sweep-partial-profile-protocol-types.log`.

### Intermediate authored-patch WASM verified; current aggregation build started

Build session 50810 completed successfully. Intermediate packaged identity SHA256 `7b78a806dae992f5b3d581bc916fd1654c5db3b621ce670e4e6b5d1602601bec`, 10,771,014 bytes. Public general sweep + authored-frame suites 31/31 passed (`/tmp/sweep-authored-patch-intermediate-public.log`). New combined-law patch refinement and path-knot refusal tests 2/2 passed; the new partial-profile aggregation test was explicitly skipped because this artifact predates that field (`/tmp/sweep-authored-patch-intermediate-error.log`).

Started current-source build session 67909 (`/tmp/sweep-partial-profile-current-wasm.log`) only after prior build confirmed completion. Next: full three-suite execution including partial-profile evidence and current-artifact Solid/STEP regression. No all-mode or complete-body guarantee is claimed.

### Packaged authored dense-profile decomposition proof regression

Added `tests/nurbsAuthoredPatchDecomposition.test.ts`: 33 positive rational controls, 32 original spans, independently parameterized path/scale/frame laws, constant twist. Packaged Rust preview reports 32 retained patches, full retained-patch error below 1e-9 and 384 charged decomposition products. Independent analytic rotation/translation samples over every span supplement (do not replace) the native interval proof; input immutability checked. Test 1/1 passed on intermediate SHA `7b78a806dae992f5b3d581bc916fd1654c5db3b621ce670e4e6b5d1602601bec`; TypeScript and scoped diff passed. Log `/tmp/sweep-authored-decomposition-public.log`.

Existing miter wall audit + retained decomposition suites: 20 passed, 1 existing TODO (`/tmp/sweep-authored-patch-intermediate-wall-regression.log`). Current aggregation WASM build session 67909: Rust release completed in 1m14s, Binaryen PID 25900 verified active. Full current-artifact execution and STEP requalification remain pending.

### Intermediate Solid boundary regression and contract status refresh

`tests/exactSolidWorker.test.ts` + `tests/rushFrontend.test.ts`: 32/32 passed on intermediate patch-error WASM (`/tmp/sweep-authored-patch-intermediate-solid.log`). Includes real authored-cap Solid construction, validation, exhausted correction refusal and selected periodic/frame-guide-affine hole cases. This is selected regression coverage, not all-mode or full UI qualification. Updated current E and STEP status cells in `docs/design/sweep-contract-matrix.md` to distinguish retained-patch proof, selected complete miter boundary examples, prior-artifact 38-case STEP evidence and pending current-artifact qualification.

### Worker full/partial bound consistency

The worker contract now rejects conflicting full and partial profile bounds when both are present: native complete certification aggregates the same maximum as the known certified-profile subset, so the two values must agree. Added positive equal-bound and negative conflicting-bound assertions; protocol 22/22 passed (`/tmp/sweep-partial-profile-consistency.log`), TypeScript and scoped diff passed. The new original-law public regression also checks exact equality of `knownProfileErrorUpper` and `continuousErrorUpper` whenever every profile is certified; execution awaits the current WASM. Geometry proof remains native Rust; this is transport validation only.

### Rush continuous-admission end-to-end regression prepared

Added `tests/nurbsAuthoredSweepAdmission.test.ts`: real authored Rush fixture with zero twist, three sections and strict positive tolerance; both synchronous and asynchronous builds must refuse with continuous retained-patch diagnostics despite zero sampled error. Async preview must carry the native upper bound and matching known-profile bound, with exactly one preview before refusal. TypeScript and scoped diff passed (`/tmp/sweep-authored-rush-admission-types.log`). Execution awaits current WASM: optimization process PID 25900, session 67909 confirmed live at 100% CPU after 5m30s. No current-artifact STEP or UI completion claim.

### Current partial-profile WASM public qualification

Build session 67909 completed. Packaged SHA256 `40b16140b71e019ed3e8131d56a7f44854e6faab474d19feef8b0d0326cc70de`, 10,771,244 bytes. Five public suites passed 36/36, including full/partial aggregation, dense rational decomposition, combined original laws, knot-transition refusal and real sync/async Rush continuous-error refusal (`/tmp/sweep-partial-profile-current-public.log`). Fresh STEP export started to `external-step-authored-patch-error` with log `/tmp/sweep-authored-patch-current-step-export.log`. Reference-volume and OCCT verification still pending.

Separately added native-only `ControlValueReport::retained_segment_displacement_upper`: value-box/retained-segment enclosure can conservatively cover knot transitions without a smoothness premise. Native regression 1/1 passed (`/tmp/sweep-knot-value-segment-native.log`). This helper is not yet integrated into sweep interpolation or the packaged artifact; existing knot-transition refusal remains intentional until owned correspondence and shared work are wired.

### Fresh current-artifact STEP qualification and owned knot fallback

Fresh export, exact retained-generator volume reference and OCCT verification completed successfully: 38 cases, report `external-step-authored-patch-error/opencascade-sweep.json` passed true. Manifest provenance verifies public/packed SHA `40b16140b71e019ed3e8131d56a7f44854e6faab474d19feef8b0d0326cc70de`, 10,771,244 bytes. Logs `/tmp/sweep-authored-patch-current-step-export.log`, `/tmp/sweep-authored-patch-current-volume-reference.log`, `/tmp/sweep-authored-patch-current-occt.log`. This selected matrix remains distinct from all-mode surface/seam/containment qualification.

Native section interpolation now falls back, when the smooth single-span remainder is unavailable, to original-law value enclosure versus the actual retained control segment on the same station interval. Shared certificate budget includes this extra request. Existing progressive sweep tests 35/35 passed after the implementation change (`/tmp/sweep-knot-fallback-native.log`). New owned-knot regression initially failed because the test used a zero-length line primitive to represent constant laws; corrected fixture uses constant control curves. Final targeted run is confirmed live in session 49488 (`/tmp/sweep-owned-knot-native-final.log`), result pending. This native change is newer than the qualified WASM and is not yet published through WASM.

Final owned-knot targeted test completed successfully: 1/1 passed. It checks a genuine parameter-derivative kink, sampled falsification against the original path, and shared-work exhaustion with no partial global upper. WASM integration/public expectation updates remain next.

### Knot-value bound WASM integration in progress

Updated public knot regression to a genuine derivative kink (path control z=3 at normalized knot about 0.4), requiring certified original-value/retained-segment coverage, refinement reduction and analytic joint-law falsification samples around the knot. Former blanket knot refusal is superseded in native for intervals with certified jets but without the single-span smoothness premise. Unresolved value requests and shared-work exhaustion still yield no global upper. Native full progressive sweep suite now 36/36 passed (`/tmp/sweep-knot-value-native-all.log`); public TypeScript and scoped diff passed (`/tmp/sweep-knot-value-public-types.log`).

New WASM build session 19520 started (`/tmp/sweep-knot-value-current-wasm.log`). Public test execution awaits this artifact. Prior 38/38 STEP evidence remains tied to SHA40b16140 and is not transferred to the new build. Remaining arc-length, closed correction, guides/all-mode bounds, filled caps/global geometry and full UI obligations remain open.

### Native rational multispan transport-law matrix

Added and passed `knot_fallback_covers_every_authored_transport_law`: six separate rational multispan cases for uniform scale, twist, authored axis, authored normal, affine axis-scale and center offset, with independently chosen knot domains and weights 1/0.5/2. Every case certifies the owned retained section interpolation bound, supplementary source-section samples stay enclosed, and zero shared work supplies no error upper. Targeted test 1/1 passed covering all six cases (`/tmp/sweep-knot-law-matrix-native.log`); scoped diff passed. This is native selected-law coverage, not a global geometry or all-combination guarantee. The test-only change does not alter production WASM compiled earlier in active session 19520; optimization PID 36211 remains live. Public knot regression and fresh-artifact verification remain pending.

### Simultaneous rational multispan transport composition

Extended native knot-law matrix to a seventh case composing all six rational multispan laws simultaneously (scale, twist, authored axis/normal, affine axes and center) with independent original domains. Targeted test passed all seven cases (`/tmp/sweep-knot-joint-law-native.log`); source-section falsification and zero-work refusal remain checked. Added matching public joint-law regression requiring full retained-patch evidence, equal known/full upper and refinement reduction. TypeScript and scoped diff passed (`/tmp/sweep-knot-joint-public-types.log`). Public execution awaits active WASM session 19520; Binaryen PID 36211 confirmed live at 5m28s. Test-only native changes do not change production compiled bytes. This does not close arc-length, corrected closed seams, full cap/global geometry, all mode combinations or UI matrix requirements.

### Packaged knot-transition and joint-law proof verified

WASM session 19520 completed: packaged SHA256 `2512f01f91ef9590b21c291762692fdc86ed1106d3fc3ea08fa014dd3a3c7924`, 10,773,357 bytes. All five authored/general sweep suites passed 37/37 (`/tmp/sweep-knot-value-current-public.log`), including actual path derivative kink, full simultaneous rational multispan laws, refinement reduction, decomposition, partial-profile admission and sync/async Rush refusal. Viewport/protocol separation tests 29/29 passed (`/tmp/sweep-knot-pending-evidence-regression.log`).

Clarified the contract: known certified-profile maxima can support refusal, but never certify the unresolved union or caps. Fresh STEP export started to `external-step-knot-value-bound` (`/tmp/sweep-knot-value-step-export.log`); actual process handle is in tool output, export/reference/OCCT completion pending. Broader arc-length/closed/guided/all-mode error, global geometry, smoothness and UI obligations remain open.

### Fresh knot-value WASM STEP qualification

Export, independent exact retained-generator volume reference and OCCT verification all completed for the new knot-value packaged artifact. `external-step-knot-value-bound/opencascade-sweep.json` passes 38 cases; manifest verifies public/packed SHA256 `2512f01f91ef9590b21c291762692fdc86ed1106d3fc3ea08fa014dd3a3c7924`, 10,773,357 bytes. Logs `/tmp/sweep-knot-value-step-export.log`, `/tmp/sweep-knot-value-volume-reference.log`, `/tmp/sweep-knot-value-occt.log`. Selected fixture import/topology/volume qualification remains separate from all-mode geometry/smoothness/UI proof.

Inspected general guided sweep source: authored and orientation-guide modes are distinct; guide normal is the original rail-to-path offset projected onto the original path tangent normal plane, with an additional shared contact-width law for anchored mode. A continuous general guided certificate must own these original-path tangent/projected-rail/contact premises; current authored control-trajectory proof cannot be relabeled as guided proof. This remains an open implementation requirement. RAG tools are available for archival refresh after the next consolidated evidence update; no archive refresh claimed in this turn.

### Original-path guided frame values in Rust

Added `authored_frame_certificate/guided_path_values.rs` and `certify_path_guide_values`. It obtains original rational path position/derivative interval jets, derives tangent normalization from that original derivative (positive original domain width preserves direction), projects original guide-minus-path offset and includes original twist using the existing outward guide-frame value certificate. Path/guide/twist requests share the same charged work budget; incomplete frame output is discarded. It does not infer contact fitting, retained error, arc-length correspondence, closed correction, or global geometry.

Native tests 2/2 passed: independently parameterized straight path/guide/twist with analytic rotated normal, shared-budget exhaustion and parallel-rail refusal; curved quadratic path with independently parameterized offset guide and analytic variable tangent/projected normal. Logs `/tmp/sweep-original-path-guide-values.log`, `/tmp/sweep-original-curved-path-guide-values.log`; scoped diff check passed. This prerequisite is not yet integrated into general guided control trajectory/section error or exposed through WASM; existing packaged SHA2512f01f and its STEP evidence remain the latest qualified artifact.

### Guided original-law control image composition

Added native `certify_path_guide_control_value`, composing original guided frame values with original path position, uniform scale, affine axis-scale and local center laws. Local-coordinate intervals remain an explicit initial-frame premise. Refactored the authored value route to share the same native law composition (`control_value_from_frame`); no production geometry moved to TS. All requests share the frame-inclusive cell budget, and unresolved requests return no partial control image.

Frame-certificate regression suite 14/14 passed (`/tmp/sweep-guided-control-shared-native.log`); guided module 3/3 passed (`/tmp/sweep-guided-control-values-native.log`) including analytic composed XYZ law, independently parameterized guide/twist/scale/axes/center and one-cell-short refusal; progressive sweep regression 37/37 passed (`/tmp/sweep-guided-composition-sweep-regression.log`). Scoped diff passed. This control-image premise is not yet constructor-owned initial coordinates, contact fitting, retained wall error, arc-length/closed correction or WASM/Rush guided certification. Latest qualified packaged artifact remains SHA2512f01f.

### Original-span endpoint tangent prerequisite

Inspection confirmed existing derivative restriction computes jets by dividing by the restricted interval width, making exact-start point requests numerically inconclusive. Added scalar `endpoint_first`: one-sided endpoint rational derivative from interval homogeneous controls of the whole original active span, dividing only by original positive span width. It does not use sampled endpoints, rounded trims, or a vanishing subinterval. Start/end rational derivative regression with independent domain [7,9] and weights [1,2] passed 1/1 (`/tmp/sweep-original-endpoint-first-native.log`); scalar certificate regression passed (`/tmp/sweep-original-endpoint-scalar-regression.log`), scoped diff passed. Caller must charge one scalar span; zero budget returns no derivative.

Next integration: XYZ endpoint derivative with shared component budget, guided initial-frame values, constructor-owned local coordinates, then full guided section/patch error. This native prerequisite is newer than the packaged artifact and is not yet exposed via WASM or used for guided constructor admission. All original completion requirements remain active.

### Guided frame endpoint ownership prerequisite

Added native XYZ `certify_endpoint_first`, charging one original scalar span per component and discarding partial derivatives when budget is insufficient. General `certify_path_guide_values` now uses this original-span endpoint derivative plus point-safe original path position for [0,0]/[1,1], preserving the shared path/rail/twist budget. Endpoint tangent no longer depends on an ill-conditioned vanishing jet restriction.

Guided native suite 4/4 passed (`/tmp/sweep-guided-endpoints-native-final.log`), including start/end rational path derivatives with domain [2,5], independent guide/twist domains, exact analytic frame enclosure, two-component partial-work refusal and one-cell-short full-frame refusal. Initial compilation caught a test literal typo (`0._f64`); corrected to `0.0_f64`, then verified successful. Vector certificate regression passed (`/tmp/sweep-guided-endpoint-vector-regression.log`); scoped diff passed. Constructor-owned guided q coordinates, contact fitting and full section/patch error remain next. Latest qualified WASM/STEP artifact unchanged; these endpoint additions are native-only so far.

### Constructor-owned guided initial coordinates

Added `Sweep::guided_initial_coordinates`: original path-start position and point-safe original guided initial frame define all local profile poles before twist and affine transformation, sharing the total cell budget. Extracted shared `coordinates_in_basis` interval arithmetic for authored/guided routes while preserving authored-only API behavior. This certifies the q premise only; contact fitting, retained interpolation, caps and global geometry remain separate.

Progressive sweep regression after refactor 37/37 passed (`/tmp/sweep-guided-initial-refactor-native.log`). New translated rational-profile fixture with independent path/guide domains, nonzero twist and affine scale/center certifies analytic q=[1,-0.5,4] and [1,-1.5,4], tight enclosure, one-cell-short refusal with no partial coordinates: 1/1 passed (`/tmp/sweep-guided-initial-owned-native-final.log`). Initial compile used an incorrect test enum abbreviation Rmf; corrected to RotationMinimizing. Scoped diff passed. Native only; WASM/STEP qualification remains tied to the previous packaged artifact. Next: constructor-owned guided control image and complete retained section/patch error, including contact-width fitting.

### Constructor-owned guided control image

Added `Sweep::guided_control_value`: the source profile pole's local coordinates are derived by the owning request, then original guided frame/path/scale/affine law images are composed with one shared work budget. Explicit guards leave contact-width fitting, arc-length correspondence and closed-frame correction unresolved; no retained-wall or complete-boundary certificate is inferred.

Owned-control analytic test 1/1 passed (`/tmp/sweep-guided-owned-control-native.log`), including no partial output with one cell less and invalid pole index refusal. Added separate contact-guide and arc-length refusal assertions (null image, explicit reasons and zero work). Full progressive sweep native suite 40/40 passed (`/tmp/sweep-guided-owned-control-all-native.log`); scoped diff passed. This native control-image premise still needs complete retained interpolation/decomposition error and contact fitting, then WASM/Rush/viewport/Solid qualification. Original objective remains uncompleted.

### Guided retained section interpolation value bound

Added `Sweep::guided_section_interpolation_bound`: constructor-owned initial q and original guided control value enclosure cover each actual retained control segment on its stored station interval. Unchanged positive rational profile basis extends the maximum control bound over the entire profile. Work is shared across all controls/intervals; incomplete work yields no global error upper. Contact-width, arc-length and closed-frame correction remain explicitly unresolved. Endpoint displacement is included by bounding the actual stored segment, without inventing a separate endpoint certificate.

Native targeted test passed (`/tmp/sweep-guided-section-bound-native.log`): whole request certifies, bound reduces between 3 and 9 sections, one-cell-short budget refuses with null error. This value-box bound is conservative (about5 for a 10-unit straight path at3 sections despite exact interpolation), so tighter derivative/dependency bounds remain required for useful tight tolerances. It is an intermediate proof path, not completion of general guided qualification. Decomposition, level/transport integration, contact fitting and WASM/public qualification remain next. Scoped diff passed.

### Guided original-profile to retained-patch composition

Added `Sweep::guided_patch_error_bound`. Authored/guided routes now share `patch_error_from_section`, retaining the same production profile decomposition, original-span correspondence, interval coefficient decomposition audit and positive rational U/V convex contraction. Guided total upper includes the entire original-law section bound plus retained decomposition displacement. Decomposition work is bounded separately from the shared law/frame cell budget. Incomplete decomposition supplies neither complete error upper nor patches.

Dense guided rational profile regression passed 1/1: 33 controls, 32 retained patches, 384 products, positive decomposition correction, complete composed upper within the selected generous tolerance; 383 products refuses with no global upper (`/tmp/sweep-guided-patch-decomposition-native.log`). Progressive sweep regression 42/42 passed (`/tmp/sweep-guided-patch-composition-regression.log`); scoped diff passed. Tight guided jet bounds still required before integrating this conservative value bound into ordinary small-tolerance level admission. Contact, arc-length/closed correction, public WASM/Rush/viewport/Solid guided certificate and original broader completion requirements remain open.

### Original rational third derivative for general guided jets

Added scalar `certify_third_traversal` / `ThirdReport` from original interval homogeneous restrictions, composing q''' = (N''' -3q''W'-3q'W''-qW''')/W. The original relative-origin representation avoids subtracting huge absolute origins. Lower jets and third-derivative work share a budget; incomplete third work discards all reported jets, preserves explicit reason and consumes no more than the requested cells. The single-original-span premise remains separate from work count.

Native analytic cubic/rational third-derivative regression passed 1/1 (`/tmp/sweep-path-third-native.log`); scalar regression 8/8 passed (`/tmp/sweep-path-third-scalar-regression.log`); scoped diff passed. Rational nonzero third derivative is tested even for degree1 with nonconstant weights, so it is not approximated as a polynomial derivative. Next: shared XYZ third jets and differentiation of normalized original path tangent/projected rail, then tighter guided retained interpolation. Not yet used for constructor admission or exposed through WASM. Full goal requirements remain open.

### Original-path third XYZ jets and general guided second frame jets

Added shared-component XYZ `certify_third_traversal`, preserving original-parameter value/first/second/third jets and a separate single-span premise; incomplete work returns no component jets. Analytic rational XYZ regression passed 1/1 (`/tmp/sweep-path-xyz-third-native.log`), checking all four orders, independent [2,5] domain and 6-versus5-cell refusal.

Added `certify_path_guide` for general guided second frame jets: original path third derivatives form normalized-traversal tangent jets; original guide/path value/velocity/acceleration differences form projected-rail jets; twist is composed with the existing outward derivative rules under one shared budget. This removes the constant-polyline-tangent assumption for this prerequisite. Curved quadratic path test checks analytic normalized tangent derivatives through second order and shared-work refusal, 1/1 passed (`/tmp/sweep-guided-curved-path-jets-native.log`). Authored/guided frame regression passed (`/tmp/sweep-guided-third-frame-regression.log`); scoped diff passed.

Next required work: compose these jets into original guided control trajectories and tighter retained interpolation, contact-width fitting and full native-to-WASM/Rush/viewport/Solid integration. These prerequisites alone do not complete a guided wall or body error certificate. Latest packaged/STEP-qualified artifact remains unchanged.

### Guided second control jets and interpolation remainder

Added native `certify_path_guide_control_trajectory`, composing original guided frame jets with source path/uniform scale/affine axis scale/center and explicit local q intervals. Authored/guided routes now share `control_trajectory_from_frame`, retaining original-law normalized derivative rules, single-span premise, and one shared work budget. Neither route invents derivatives from retained/sampled stations.

Analytic guided joint-law test passed 1/1 (`/tmp/sweep-guided-control-jets-native.log`): values, first and second jets contain independent polynomial XYZ; h² sup||x''||/8 encloses the expected0.03125 within1e-9; one-cell-short work refuses with no jet. Frame regression18/18 passed (`/tmp/sweep-guided-control-jet-frame-regression.log`), progressive sweep42/42 passed (`/tmp/sweep-guided-control-jet-sweep-regression.log`), scoped diff passed. Next: constructor-owned guided jet and point-safe stored-station displacement, then tighten retained section/patch admission. Contact fitting, arc-length/closed correction, public integration and all original global/STEP/UI obligations remain open.

### Point-safe original derivatives at guided interior stations

Added scalar/XYZ `certify_first_point`: interval homogeneous controls of the full original active span are differentiated using its positive width, then rational derivative polynomials are evaluated at the outward normalized point parameter via interval de Casteljau. Multiple touching spans share work and their derivative union; this does not assert smoothness across knots. Partial component/span work returns no derivative.

Guided frame values now use this point-safe path derivative for interior point restrictions, preserving original-span one-sided endpoint derivatives and original position/rail/twist budget accounting. Analytic rational station regression passed1/1 (`/tmp/sweep-point-first-native.log`); constructor-owned guided control point regression passed1/1 (`/tmp/sweep-guided-point-stations-native.log`), with independent-domain rational path, signed twist, analytic XYZ containment, enclosure width below1e-9 and one-cell-short refusal. Scalar/sweep regression logs `/tmp/sweep-point-first-scalar-regression.log`, `/tmp/sweep-guided-point-sweep-regression.log`; scoped diff passed.

Next: store these endpoint displacements per generated station and compose them with guided second-jet remainders, retaining the value-bound fallback for knot transitions. Contact fitting, arc-length/closed correction, WASM/public and original complete-body/global/smoothness/UI requirements remain open. Native changes are not yet included in the latest STEP-qualified WASM.

### Tight guided interpolation and native refinement admission

Guided retained-section proof now records point-safe original-law displacement at every stored station/control and composes it with the original guided second-jet interpolation remainder on each single original span. Knot transitions retain the conservative original-value fallback. The positive rational profile convex contraction and original decomposition audit remain shared with authored transport.

Native progressive sweep suite passed44/44 (`/tmp/sweep-guided-tight-level-native.log`). Straight guided transport now has certified error below1e-9. Joint guide/twist/uniform scale/affine-axis/center/rational-profile case refines to a certified0.01 tolerance, including multiple retained V patches. Focused admission regression passed1/1 (`/tmp/sweep-guided-level-admission-native.log`): coarse refusal, fine acceptance, propagated bound/work fields, zero-cell unresolved result, and iterator refinement through certified levels. Independent analytic XYZ samples falsify incorrect bounds; sampling is not the certificate.

Prepared public WASM regression `tests/nurbsGuidedPatchError.test.ts`; not yet executed against the new artifact. Geometry WASM build started with log `/tmp/sweep-guided-tight-current-wasm.log`. No packaged/public/STEP qualification claim is made until build and corresponding tests complete. Contact fitting, arc-length/closed correction, complete boundary including caps, general miter combinations, global guarantees, smoothness and full STEP/UI matrices remain open.

### Packaged guided retained-patch qualification

Geometry WASM build completed (`/tmp/sweep-guided-tight-current-wasm.log`), SHA256 `ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0`,10,799,552 bytes. Six public suites passed40/40 (`/tmp/sweep-guided-tight-current-public.log`), including guided joint-law original-to-retained enclosure, refinement admission, unsupported arc-length/contact scope, and Rush sync refusal/async preview despite zero sampled deviation. Five authored/progressive regression suites passed in the same run. Viewport/protocol regression passed29/29 (`/tmp/sweep-guided-tight-viewport-protocol.log`); this is protocol validation, not full live UI qualification.

Fresh STEP export started to `external-step-guided-tight` (`/tmp/sweep-guided-tight-step-export.log`). STEP/exact-volume/OCCT evidence is pending for this new SHA and cannot be inherited from the prior artifact. Guided E scope remains retained patches; caps/contact fitting/arc-length/closed correction and all original global/smoothness/full-matrix completion requirements remain open.

### Fresh STEP qualification on guided-certificate artifact

`external-step-guided-tight/manifest.json` identifies38 selected fixtures, SHA256 `ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0`,10,799,552 bytes, publicAndPackedVerified true. Export completed; independent exact retained-generator volume reference completed before OCCT (`/tmp/sweep-guided-tight-volume-reference.log`). OCCT verification passed38/38 and exited0 (`external-step-guided-tight/opencascade-sweep.json`, `/tmp/sweep-guided-tight-occt.log`). Scope: selected import/topology/analytic-volume/full-domain cap-coedge cases, not all-mode global geometry, smoothness or live UI completeness. This refresh proves the new artifact preserves the selected miter frame/guide/affine fixtures; it does not expand the scenario count or prove a general guided body boundary certificate.

### Original guide-width jets for contact fitting

Added native `certify_guide_width` / `GuideWidthReport` to authored-frame certificate module. Original path/rail rational vector jets are normalized using each independent source domain; interval rail-minus-path norm and its first/second derivatives share one work budget. Width must be separated from zero. Incomplete work publishes no width or derivative jets; single-span smoothness premise remains separate from overall work count.

Analytic sqrt(1+t²) width/first/second derivative enclosure with independent [2,5]/[17,19] domains, one-cell-short refusal and zero-width refusal passed1/1 (`/tmp/sweep-contact-width-jets-native.log`). Full frame-certificate regression passed (`/tmp/sweep-contact-width-frame-regression.log`); scoped diff passed. This prerequisite does not prove normal-plane contact, original anchor ownership, transformed-anchor positivity, fit quotient jets, retained contact interpolation or body E. Latest packaged/STEP-qualified SHA remains ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0; new width jets are native-only and not yet wired into admission.

### Original-law contact-fit quotient jets

Added native `certify_contact_fit`, computing width / (uniform_scale × transverse_axis_scale × anchor + transverse_center). The supplied positive anchor interval remains an explicit premise. Original independent-domain scalar/vector jets and rail-width proof share one cell budget; quotient derivatives use outward rational differentiation. Denominator must be separated from zero. Partial work/nonpositive denominator supplies no fit jets. Original single-span premises are propagated from source certificates without deriving smoothness from sampled stations.

Analytic joint-law fit 2/[2(1+t)²+0.5t], first/second derivatives, independent domains, one-cell-short refusal and negative denominator refusal passed within frame regression20/20 (`/tmp/sweep-contact-fit-frame-native.log`). Initial test fixture incorrectly used a zero-length geometric line for a constant law; replaced with explicit constant law coefficients. Scoped diff passed. This remains native-only, with anchor ownership, continuous normal-plane/zero-twist compatibility, point-safe fit values and retained contact control/section/patch composition pending. It does not promote contact admission or full body continuousBound. Latest packaged/STEP-qualified artifact remains SHA ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0.

### Constructor-owned contact anchor provenance

`Sweep` now retains `contact_source: Option<(&Curve, parameter)>` in addition to its existing evaluated contact point. Single-profile contact records the original reference curve and its original-domain parameter. Multi-profile contact propagates the selected source reference and parameter to every contour, preserving one common fitting anchor rather than pretending each contour owns a separate anchor. Switching to ordinary orientation guide clears both evaluated point and source provenance. Existing geometry evaluation remains unchanged.

Native sweep regression45/45 passed (`/tmp/sweep-contact-anchor-ownership-native.log`), including reference pointer/parameter ownership on two contours and reset. This establishes source provenance, not a continuous interval anchor value or normal-plane/contact identity certificate. Next: original profile point enclosure and initial guided-frame projection under shared budget, then contact coefficient/control/retained-patch composition. Native-only changes since the last packaged SHA remain unqualified through WASM until a subsequent complete build/test cycle.

### Original rational anchor enclosure in constructor-owned initial frame

Added `Sweep::contact_anchor_bound` / `ContactAnchorReport`. The original selected source curve and original-domain parameter supply a point enclosure via outward inverse traversal mapping and original rational value certificate. Original path start and untwisted guided frame share the same work budget; projection gives all three local anchor coordinates and requires a positive transverse lower bound. No evaluated/stored contact point is used as the mathematical source premise. Partial work publishes neither point nor coordinates. This enclosure does not assert exact anchor/rail coincidence or normal-plane contact.

Native sweep regression46/46 passed (`/tmp/sweep-contact-anchor-bound-native.log`). New test checks interior rational source point at original domain[7,9], translated path/profile, exact ideal transverse coordinate5/3, tight enclosure below1e-10, one-cell-short refusal and orientation-guide reset. Scoped diff passed. Next: constructor-owned fit jets/control transport and point-safe station fit values, then continuous contact compatibility and retained-patch composition. This is native-only work after SHA ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0; full goal remains open.

### Constructor-owned contact-fit jet composition

Added `Sweep::contact_fit_jet` / `ContactFitReport`. Original-source anchor enclosure supplies the transverse anchor interval to original-law width/scale/affine quotient jets under one total cell budget. Incomplete anchor or quotient proof returns no fit; arc-length correspondence remains explicitly unresolved. Output fit work includes anchor work, preventing accidental independent budget resets. This proves the original quotient premise, not contact identity or retained interpolation.

Native sweep46/46 passed (`/tmp/sweep-owned-contact-fit-native.log`), with original rational interior anchor, normalized constant fit value/derivatives and one-cell-short total-budget refusal. Extended multi-profile regression passed1/1 (`/tmp/sweep-owned-contact-fit-multi-native.log`): both contours enclose the same fit from the selected reference profile, regardless of their own widths. Scoped diff passed. Contact control composition, point-safe station fit values, compatibility, decomposition/admission integration, WASM/public qualification and the original full body/global/smoothness/STEP/UI goal remain open.

### Contact-fit original control trajectory jets

Added native `certify_contact_control_trajectory` with explicit q/anchor interval premises. Shared trajectory composition now accepts optional transverse fit jets; the full uniform×axis×q+center transverse coefficient is multiplied by fit through second order, while side/longitudinal coefficients remain unchanged. Original width/quotient/frame/control-law work shares one budget. Single-original-span premises are combined independently; nonzero twist is rejected. Incomplete proof publishes no trajectory jet.

Frame regression21/21 passed (`/tmp/sweep-contact-control-frame-native.log`), including independent rational x=2[(1+t)²+0.5t]/[2(1+t)²+0.5t], polynomial Y/Z, analytic first/second derivatives and one-cell-short refusal. Progressive sweep regression46/46 passed (`/tmp/sweep-contact-control-sweep-native.log`); scoped diff passed. Constructor-owned q/anchor binding, point-safe contact control values, continuous contact compatibility and retained section/patch/admission remain next. Native-only prerequisites do not change the last packaged/STEP-qualified SHA or close full body/global/smoothness/STEP/UI requirements.

### Constructor-owned contact control trajectory

Added `Sweep::contact_control_trajectory`. Original reference-profile anchor enclosure and original current-profile initial guided coordinates supply the explicit premises to contact control jets under one total cell budget. It validates normalized traversal/control index; unsupported arc-length and closed correction return explicit unresolved reports. Partial coordinate/anchor/fit/frame/law work publishes no control jet. No sampled contact point supplies a proof premise.

Progressive sweep regression46/46 passed (`/tmp/sweep-owned-contact-control-native.log`), including translated rational interior-anchor control value/first/second analytic containment, one-cell-short combined budget and invalid control index. Scoped diff passed. Native-only original trajectory proof remains separate from continuous rail compatibility, point-safe stored-station values, retained section/decomposition/cap composition and admission. These next obligations and every original full body/global/smoothness/STEP/UI requirement remain open; packaged/STEP-qualified SHA is unchanged.

### Point-safe original contact fit/control values

Added native `certify_contact_fit_value` / `ContactFitValueReport`, composing original rational path/rail values, positive rail width, uniform scale, affine transverse scale/center and explicit anchor intervals. Point restrictions use value-only source enclosures, avoiding derivative divisions at vanishing interval widths. Shared budgets, width/denominator positivity and no partial output remain required.

Added `certify_contact_control_value`; shared original control-value composition now accepts optional transverse fit. Fit/frame/control laws share one budget, and nonzero twist is rejected. Independent joint-law fit and XYZ station regression checks endpoints and interior fractions0.13/0.375/0.5/0.87, independent source domains, enclosures below1e-9 and one-cell-short refusal. Frame22/22 passed (`/tmp/sweep-contact-control-point-native.log`); sweep46/46 passed (`/tmp/sweep-contact-point-sweep-regression.log`); scoped diff passed. Constructor-owned value wrapper and retained endpoint/remainder/decomposition integration remain next. These native prerequisites do not prove rail contact compatibility, full boundary E, global/smoothness/UI completeness or change packaged artifact qualification.

### Constructor-owned point-safe contact control values

Added `Sweep::contact_control_value`, binding original current-profile guided coordinates and the original selected rational anchor to value-only contact fit/frame/control-law composition. One total budget includes both constructor premises. Invalid index/traversal is rejected; arc-length and closed correction remain unresolved; incomplete work publishes no value.

Sweep regression46/46 passed (`/tmp/sweep-owned-contact-point-native.log`). Translated rational interior-anchor test now checks all endpoint/interior stations, independent analytic XYZ containment with width below1e-9 and one-cell-short total-work refusal. Scoped diff passed. Next: retained station displacement plus second-jet remainder/value fallback and original decomposition composition. Rail contact compatibility, full body boundary/caps and all global/smoothness/STEP/UI requirements remain open; native-only changes still await packaged qualification.

### Native retained contact-section interpolation bound

Added `Sweep::contact_section_interpolation_bound`. Guided/contact section proofs share actual source/retained basis ownership and station/interval composition; the contact route additionally owns the selected original anchor once, then composes contact point values and second control jets with the original initial q coordinates. Each stored control displacement is included at interval endpoints. Single-span interpolation uses the certified second remainder; knot transitions/unresolved jets retain original-value versus retained-segment fallback. All work shares one budget; partial traversal publishes neither global error nor endpoint upper. This is E relative to original fitted profile transport, not proof that the rail remains exactly in the normal plane or that the ideal transported anchor coincides with the rail.

Sweep regression46/46 passed after production integration (`/tmp/sweep-contact-section-native.log`). New joint scale/affine/contact regression passed1/1 (`/tmp/sweep-contact-section-joint-native.log`): rational profile, actual retained controls, independent ideal formula, coarse/fine reduction, fine bound below0.01, endpoint displacement below1e-9 and one-cell-short refusal. Scoped diff passed. Contact decomposition/patch composition and admission, WASM/public qualification, compatibility and full original body/global/smoothness/STEP/UI requirements remain next. Existing guided admission keeps contact unresolved until explicit patch integration.

### Contact retained-patch decomposition and refinement admission

Added `Sweep::contact_patch_error_bound` using shared original-profile/actual-retained decomposition proof. Native level selection now chooses this route when constructor contact provenance exists. Complete original fitted transport E (station rounding + interpolation + decomposition) controls admission; existing partial/unresolved report semantics remain explicit. This E scope does not assert continuous exact rail contact, global geometry or caps.

Native sweep47/47 passed (`/tmp/sweep-contact-patch-admission-native.log`). Dense contact rational profile checks32 actual patches,384 decomposition products, positive correction,383-product refusal without patches/global bound. Joint affine contact regression checks coarse refusal and fine acceptance with propagated complete bound. A multi-profile test previously expected immediate sampled acceptance; it now checks certificate-based coarse refusal and refinement to a level whose geometry preserves shared hole widths, rather than weakening the new bound.

Public contact original-law regression prepared in `tests/nurbsGuidedPatchError.test.ts`; TypeScript and scoped diff passed. New geometry WASM build started (`/tmp/sweep-contact-current-wasm.log`); public/STEP qualification is pending for the new artifact. Latest qualified SHA remains ecd6c13ccaeff7bf9a2d8d1df44abd4c1ff9242cf3246adf3c2d74103de97ac0. Full original all-mode body/global/smoothness/STEP/UI requirements remain open.

### Prepared Rush contact refusal/preview qualification

Extended public Rush regression to both orientation and contact guide modes with explicit contact_profile/contact_parameter and zero twist. Both must reject an impossible tiny tolerance using the original-to-retained continuous upper despite zero sampled deviation, and propagate the certificate through async preview. TypeScript passed (`/tmp/sweep-contact-rush-typecheck.log`); scoped diff passed. Runtime execution awaits the active contact WASM build (`/tmp/sweep-contact-current-wasm.log`), which has not yet completed.

Contract matrix now distinguishes native contact fitted-transport E/admission from still-unproved exact continuous rail identity and pending contact WASM qualification. Existing qualified orientation-guide evidence remains bound to the previous SHA; no new public/STEP/UI claim is made before the corresponding checks.

### Contact aggregate-budget refusal regression

Added native/public32-profile contact case at9 stations with constant original geometry and tolerance1e-30. Native targeted regression passed1/1 (`/tmp/sweep-contact-union-native.log`): sampled deviation0, request rejected by knownProfileErrorUpper, complete continuousErrorUpper null and continuousBound false when shared10,000-cell work runs out. This proves partial profile evidence cannot promote the unproved union. TypeScript passed (`/tmp/sweep-contact-union-typecheck.log`); public runtime still awaits the contact WASM build. Test-only native changes after build launch do not change its production sources. Scoped diff passed.

### Packaged contact E/admission public qualification

Geometry WASM build completed (`/tmp/sweep-contact-current-wasm.log`): SHA256 `42739c3fbeedb1ff65f80c687d93a9b3123aa513b67912d639ccec4ce6a1bd1c`,10,815,162 bytes. Eight public suites passed72/72 (`/tmp/sweep-contact-current-public.log`): contact original-anchor/affine refinement enclosure and admission; orientation/contact Rush sync refusal and async preview despite zero sampled error; partial32-profile contact work exhaustion; progressive/authored regression; viewport/protocol evidence gates. This includes protocol checks, not a full live UI matrix.

Fresh selected STEP export started to `external-step-contact-current` (`/tmp/sweep-contact-current-step-export.log`). Independent exact retained-volume reference and OCCT qualification are pending for this SHA. Contact E scope is original fitted transport to retained patches, with original decomposition/stored-station error; exact rail identity, caps/full body boundary, all-mode/global/smoothness and complete STEP/UI requirements remain open.

### Fresh selected STEP qualification on contact artifact

Fresh export, independent exact retained-generator volume reference and OCCT completed successfully for38/38 selected cases (`external-step-contact-current/manifest.json`, `opencascade-sweep.json`; logs `/tmp/sweep-contact-current-step-export.log`, `/tmp/sweep-contact-current-volume-reference.log`, `/tmp/sweep-contact-current-occt.log`). Manifest binds SHA42739c3fbeedb1ff65f80c687d93a9b3123aa513b67912d639ccec4ce6a1bd1c,10815162bytes and publicAndPackedVerified true. This requalifies the selected miter frame/guide/affine geometry, topology and volume under shared composition changes; it does not add a general contact-body fixture or prove the full scenario space.

Production Vite build started (`/tmp/sweep-contact-current-vite.log`) to prepare the existing isolated headless browser matrix runner. This is pending preparation, not UI evidence. Full body/global/smoothness/all-mode and full original live UI requirements remain open.

### Current artifact browser matrix started

Production Vite build passed (`/tmp/sweep-contact-current-vite.log`). Existing `check-sweep-miter-matrix-browser.mjs` now runs against this dist and SHA42739c3f to `ui-contact-current`; isolated browser/server preserve the user's current browser scene. Initial three wide conic reconstruction cases passed13 checks each. Their Solid screenshot was inspected: actual curved body visible, scene retains a B-rep Body1, CPU fallback scope shown. Full matrix remains running (`/tmp/sweep-contact-current-ui.log`, `ui-contact-current/matrix.json`); partial results do not complete the original wide/narrow lifecycle requirement. Held-dispatch cancellation is lifecycle evidence, not mid-kernel interruption latency; headless CPU rendering does not qualify hardware GPU behavior.

### Completed current retained-miter browser lifecycle matrix

Existing isolated browser runner completed with exit0: `ui-contact-current/matrix.json` has passed=true,74/74 cases,852 assertions,zero page errors;37 source modes at1440×1000 and600×1000. Artifact SHA42739c3fbeedb1ff65f80c687d93a9b3123aa513b67912d639ccec4ce6a1bd1c matches the current packaged WASM. Scope includes corrected/uncorrected hollow cases, affine/frame/guide combinations, moving frames, closed corrections, reconstruction/conic cases, native smoothness presentation, Build/Solid success or expected refusal, held-dispatch cancellation, source supersession and restoration. Selected wide/narrow screenshots were visually inspected. Full source hashes and per-case downloaded B-rep JSON are retained.

This completes the current runner's finite retained-miter lifecycle matrix, not all original progressive surface/contact modes. Headless CPU rendering and held dispatch do not prove hardware GPU behavior or interruption inside synchronous kernels. Current guided/contact surface live-UI extension, arc-length/closed error qualification, full body continuousBound/global/smoothness and complete all-applicable STEP scope remain open. No publication or unrelated-branch CI evidence is transferred to this checkout.

### Progressive surface lifecycle and visible viewport qualification

Extended the existing browser runner with `--surface` and `SWEEP_UI_HEADED=1`, preserving native geometry ownership. Four sources (RMF, authored, guided, contact) at1440×1000 and600×1000 passed8/8 scenarios and80 assertions in `ui-progressive-surfaces-fitted-current/matrix.json`, zero page errors, current WASM SHA42739c3fbeedb1ff65f80c687d93a9b3123aa513b67912d639ccec4ce6a1bd1c. Includes build, renderer-ready, UI Fit, held-dispatch build/Solid cancellation, source replacement, restoration and expected native-B-rep refusal for open surface patches. Log `/tmp/sweep-surfaces-fitted-current-ui.log` completed exit0. Wide and narrow contact preview screenshots inspected: actual yellow surface geometry visible after Fit. Renderer-ready and Fit assertions do not independently measure rendered geometric accuracy.

Earlier headless surface run8/64 passed lifecycle assertions but screenshot exposed WebGPU requestDevice failure; it is not rendering proof. Headed8/72 renderer run proved readiness but lacked Fit. Preserve these as weaker intermediate evidence. Current fitted screenshots still show the material controls overlay obscuring the upper model region, a remaining viewport usability issue. This finite surface matrix does not close general GPU hardware qualification, actual mid-kernel cancellation, all-mode geometric guarantees or the complete six-item goal.

### Viewport material controls collapse and rerun

`src/App.vue` now wraps viewport material settings in a native collapsed details menu with localized Material summary; existing material component and Rust geometry remain unchanged. Existing surface runner asserts initial collapse, opens actual shading controls, closes them, and uses UI Fit. On narrow windows it closes the existing dock first through its accessible button (initial attempt correctly timed out because the dock intercepted clicks; no forced clicks or DOM hiding used).

Fresh production dist: vue-tsc --noEmit passed (empty `/tmp/sweep-material-typecheck.log`), Vite build passed (`/tmp/sweep-material-vite.log`). Headed surface matrix `ui-progressive-surfaces-material-current/matrix.json` completed8/8,88 assertions,0 page errors, unchanged WASM SHA42739c3f. Log `/tmp/sweep-surfaces-material-current-ui.log` exit0. Wide/narrow contact screenshots inspected: material controls collapsed and surface visible. The normal toolbar still occupies upper viewport space, especially narrow; this is not universal obstruction-free proof. Full six-requirement geometry objective remains open.

### Closed guided/contact retained error: native implementation, WASM pending

Removed blanket closed-path refusal only from guided/contact original control values/trajectories and guided/contact retained section composition. `Sweep::sections` applies zero closure correction whenever an orientation guide is present; every normal comes from the guide. Its copied final section is enclosed by the existing station endpoint-displacement term. Authored/RMF closure correction and arc-length remain unproved; no cap/body/embedding/smoothness promotion.

A circle/translated-circle guide regression initially exposed tiny-span derivative failure at an outward-mapped knot. Scalar `span_jets` now differentiates full original homogeneous Bernstein spans before interval evaluation for tiny restrictions, retaining the adjacent outward overlap rather than dropping it. Ordinary restrictions retain their existing implementation. Reuses point-safe interval de Casteljau helper. Native progressive regression131/131 passed (`/tmp/sweep-closed-guide-native-regression.log`), then dense intermediate trajectory/seam/work-exhaustion regression passed at129 and33 sections (`/tmp/sweep-closed-guide-bound-dense.log`, `/tmp/sweep-closed-guide-bound-33.log`). Existing full regression preceded the additional dense assertions; production Rust was unchanged afterward.

Public WASM closed guided/contact tests added to `tests/nurbsGuidedPatchError.test.ts` but pending execution against rebuilt artifact. `node scripts/build-geometry-kernels.mjs` active (`/tmp/sweep-closed-guide-wasm.log`, exec session56516); build had reached geometry-wasm compile at latest verified poll. Old SHA42739 remains historical qualified artifact until new recipe finishes and public tests pass. This native improvement does not close the original six-requirement goal.

### Tiny-span independent analytic regression and final native rerun

Added `tiny_original_span_restriction_retains_rational_jets`: one-ulp subinterval of the original rational line with value(1+3t)/(1+t), first2/(1+t)^2, second-4/(1+t)^3. All outward bounds contain analytic values at both endpoints and have width<1e-10; zero-cell budget discards all jets. Focused1/1 and final progressive regression132/132 passed (`/tmp/sweep-tiny-span-native.log`, `/tmp/sweep-closed-guide-native-final.log`). `vue-tsc --noEmit` passed (`/tmp/sweep-closed-guide-typecheck.log`) after updating stale Rush help to describe scoped retained-patch certificates and explicitly unresolved modes.

WASM session56516 verified live again: cargo release finished; Binaryen child23196 running at100%CPU on the new snapshot, parent22559. No restart; public closed tests still pending the resulting artifact. RAG previous compile loop completed (wiki revision6,index rebuilt,logseq4373).

### Closed Rush/UI fixtures prepared during verified optimization wait

Added `closed-guided-progressive-sweep.r` and `closed-contact-progressive-sweep.r`: original circle path, translated circle guide, open line profile,33 retained stations, explicit100mm preview budget. These are wall surface examples, not native bodies; Solid must refuse. Public tests assert closed retained-patch certificates directly and through Rush async preview callbacks. Existing browser runner surface matrix now includes these two sources and creates a zero-radius source error for circle paths using normal file loading. Typecheck and script syntax/diff checks passed. These new public/UI cases are prepared but not executed yet; no passing claim before new WASM finishes. Session56516/Binaryen23196 still confirmed active near7minutes optimizer CPU time. No restart or changed build recipe.

### New packaged closed-guide artifact and public refusal evidence

WASM build session56516 completed exit0. New packaged identity SHA256 b94ebdc0da0a0b96963833a00c914d206017abdcb2c9ce1bafeed39a8fafb28c,10,817,632bytes. Binaryen12,166,379→10,817,632bytes; log `/tmp/sweep-closed-guide-wasm.log`. Vite build passed (`/tmp/sweep-closed-guide-vite.log`). Public eight-suite regression76/76 passed (`/tmp/sweep-closed-guide-public.log`): direct closed guided/contact retained error certificates plus sync/async Rush refusal propagation.

Initial Rush success assertion failed for the new closed fixtures: complete error upper284.27777158894105mm guided and576.9410912818211mm contact against100mm budget, sampled error0.026629999634062727mm. Rust/Rush correctly refuse; do not raise the budget or advertise accepted geometry. Fixtures now explicitly document expected continuous-error refusal and public tests require it. They were removed from the successful surface lifecycle source list until tighter bounds permit useful admission. Prepared circle-source-invalid mutation remains in the runner.

Fresh headed existing open-surface matrix `ui-progressive-surfaces-closed-bound-current/matrix.json` on SHA b94ebdc passed8/8,88 assertions,zero page errors. Narrow contact screenshot inspected: visible yellow surface, materials collapsed. This covers four open sources, not successful closed guided/contact viewport construction. Independent selected STEP export started (`/tmp/sweep-closed-bound-step-export.log`, session89942) but remains running; OCCT validation has not yet run on this artifact. Full goal remains open; tightening closed guided/contact error is a concrete next geometry obligation.

### b94ebdc independent STEP complete; tighter Rust image bound pending packaging

Selected STEP export session89942 and independent exact retained-generator volume/OCCT session61087 completed exit0. `external-step-closed-bound-current/opencascade-sweep.json` has passed=true,38cases, provenance SHA b94ebdc0da0a0b96963833a00c914d206017abdcb2c9ce1bafeed39a8fafb28c,length10817632,publicAndPackedVerified=true. Logs `/tmp/sweep-closed-bound-step-export.log`, `/tmp/sweep-closed-bound-step-volume.log`, `/tmp/sweep-closed-bound-step-occt.log`. These selected miter fixtures do not prove the all-applicable STEP requirement or closed contact bodies.

Added `TrajectoryReport::retained_segment_displacement_upper`, reusing its existing original control value enclosure. Guided/contact section composition now takes the minimum of this pointwise original-image/retained-segment bound and the independent smooth h² remainder plus station rounding when both exist, or uses whichever is available. Both certify the same error; no extra work, discarded knots or larger tolerance. Existing value-only fallback remains for unresolved jets. Native closed guide/contact regression now requires admission at the unchanged100mm fixture budget, preserves dense intermediate/control and copied seam checks, and explicit work exhaustion. Final native132/132 passed (`/tmp/sweep-closed-guide-image-bound-native.log`). This improves bound conservatism; useful small-tolerance qualification remains separate.

Restored closed Rush fixtures and prepared public async success plus strict-budget refusal tests and successful surface UI source entries. They await new production WASM. Build session52797 started (`/tmp/sweep-closed-guide-image-wasm.log`); b94ebdc public76/STEP38/UI8/88 evidence remains scoped to its previous production code. New production Rust is native qualified only until packaging/public rerun. Full six-requirement goal remains active.

### Closed image/remainder convergence and shared-work refusal measured

Native closed circle/translated-guide regression now compares33 and129 retained sections. Guided error upper10.838117648072132→1.1569119978329256mm (cells3804→14364); contact21.856605077917436→1.3949568114739457mm (cells6017→22721). Existing sampled intermediate trajectory/control and copied endpoint seam checks remain. Both fine native certificates are complete under100000cells. Under the actual public10000cell budget both fine certificates explicitly become Unresolved with no full upper; this is now asserted. No budget increase in production or misleading complete claim. Logs `/tmp/sweep-closed-guide-image-convergence.log`; after removing diagnostic prints full progressive regression132/132 passed (`/tmp/sweep-closed-guide-image-convergence-regression.log`). These were test-only additions after the current WASM build began.

WASM session52797 verified live, release compile finished and Binaryen child37970 CPU~100% on snapshot; no restart. Public success/strict refusal and12-case UI remain pending its final identity. This establishes native convergence and resource refusal, not useful small-tolerance public qualification or all original guarantees.

### Narrow opened material popup containment

While new WASM optimization remains live, fixed `viewport-material-menu` popup alignment from left:0 to right:0: its280px content box could otherwise extend beyond600px viewport when the Material button sits near the right edge. Existing surface UI runner now opens actual controls and asserts the popup bounding box is inside the viewport before closing and fitting the model. vue-tsc passed (`/tmp/sweep-material-popup-bounds-typecheck.log`), script syntax/diff checks passed. Runtime UI check is pending a fresh dist/new artifact; do not claim containment verified before the run. No Rust production edits during this optimization wait.

### Narrow material popup containment runtime verified on b94ebdc

Fresh Vite build passed (`/tmp/sweep-material-popup-current-vite.log`) while Rust optimization remains pending. Existing isolated headed runner filtered to contact-progressive-sweep.r at600×1000 completed exit0,1/1 scenario11 assertions0 page errors (`ui-material-popup-b94-current/matrix.json`, `/tmp/sweep-material-popup-b94-ui.log`). Actual opened material popup bounding box asserted within viewport, then controls closed and UI Fit invoked; cancellation/refusal/source restore checks still passed. Report SHA b94ebdc matches unchanged packaged identity before/after run. This verifies popup alignment at this narrow fixture only; new image-bound Rust and closed12-case matrix remain unqualified until session52797 finishes. No transferred new-Rust claim from this old artifact.

### Tight image/remainder WASM and closed surface UI qualified

WASM session52797 completed exit0: SHA256 da10339d24420c5b217f13090fc67c9656c9c21ea938d76d7f177ffa109ad4b9,10,818,436bytes; Binaryen12,167,264→10,818,436bytes (`/tmp/sweep-closed-guide-image-wasm.log`). Public8 suites76/76 passed (`/tmp/sweep-closed-guide-image-public.log`), including closed guided/contact async Rush success at unchanged100mm budget and sync/async strict-budget continuous-error refusal. Native final132/132 evidence remains scoped to this production code; later edits were tests only.

Fresh Vite build passed. Headed six-source surface matrix at1440/600 completed12/12,132 assertions,zero page errors, exact da10339 artifact (`ui-progressive-surfaces-image-current/matrix.json`, `/tmp/sweep-surfaces-image-current-ui.log`). Includes RMF/authored/guided/contact plus closed guided/contact, renderer readiness/UI Fit, material popup bounding containment, Build/Solid held-dispatch cancellation, source replacement/restoration, invalid path refusal and native-B-rep Solid refusal for surfaces. Wide and narrow closed-contact screenshots inspected: wall geometry visible. Tessellation segments4 produces visibly faceted display; this is not an independently certified mesh error or a body proof. Complete retained-patch E remains the native scope.

Independent selected STEP export active session70427 (`/tmp/sweep-image-step-export.log`); volume reference and OCCT for da10339 not yet run. Full native/miter/BRep/global/smoothness/all-mode/STEP requirements remain open; small-tolerance closed public certificates still limited by10k shared work. Viewport's completed BRep evidence panel currently excludes patch-only geometry, so native patch certificate presentation is another integration obligation (do not use body whole-boundary labels for patches).

### da10339 independent STEP and scoped patch viewport evidence complete

Independent exact retained-generator reference plus OCCT session34141 completed exit0 (`/tmp/sweep-image-step-volume.log`, `/tmp/sweep-image-step-occt.log`). `external-step-image-current/opencascade-sweep.json`:passed=true,38cases, provenance da10339d24420c5b217f13090fc67c9656c9c21ea938d76d7f177ffa109ad4b9,length10818436,publicAndPackedVerified=true. These remain selected miter fixtures, not complete all-applicable STEP or arbitrary contact-body proof.

TS adapters now attach `sweepPatchEvidence` only to the exact native patches source node with progressive-fourfold construction report. Separate presentation reader/panel validates accepted status, scope, rounding/full flags, finite matching full/known upper, tolerance and shared work/product bounds before displaying a positive flag. It never enters the BRep/body evidence reader. Native Rust still owns geometry/error construction; TS only transports and presents. Unresolved/partial/corrupt reports cannot show a complete patch upper. Public tests verify invalid scope/rounding/known bound/work/over-budget/partial reports plus real closed Rush display snapshots; source cancellation hides final patch evidence.

Typecheck/Vite passed;8 public suites77/77 (`/tmp/sweep-patch-viewport-all-public.log`). Headed surface matrix `ui-progressive-surfaces-evidence-current/matrix.json`:12/12,144 assertions,0 page errors on da10339,including scoped evidence display and cancellation clearing. Narrow closed-contact screenshot inspected:21.8566/100mm displayed with original-profile-transport and retained-patch scope, actual wall visible. It is not a mesh/body/embedding certificate.

Positive combined frame+guide+affine hollow Solid regression at600 passed1/11,0 page errors (`ui-body-after-patch-evidence-current`, `/tmp/sweep-body-after-patch-evidence-ui.log`). Downloaded native body retains10faces/1shell; WebGPU body visible in inspected screenshot. Initial attempt failed an old CPU-only SVG-polygon assertion although WebGPU was active; runner now validates visible GPU canvas or CPU SVG and records backend, without forced rendering or geometry fallback. Wider hardware GPU and all-mode UI qualification remain separate.

Full original six requirements remain open, including small-tolerance/shared-budget closed proofs, arc-length/actual closure corrections, complete global/body/smoothness and all-applicable STEP scope. No goal completion inferred from these finite checks.

### Shared original control-value work: native only, packaging intentionally pending

Added Rust `ControlValuesReport` and shared original-value batch construction for guided/contact controls. Path,scale,affine,frame and contact-fit source values are computed once per requested traversal interval, then the same original transform is applied outwardly to each source coordinate. Single-control APIs wrap this batch and retain their former results. Guided/contact retained station endpoint composition charges the batch source work once; every original source proof remains within the shared budget and any source exhaustion discards the complete values vector. No source spans or certificate premises are dropped; production work ceiling unchanged.

Measured33/129 sections: guided cells2442/9066 (previous3804/14364); contact3830/14198 (previous6017/22721). Fine error uppers remain exactly1.1569119978329256 and1.3949568114739457mm. Guided129 now fits public10000 cell ceiling; contact129 remains unresolved at that ceiling. Native regression asserts the guided budget improvement and complete-vs-unresolved behavior with no partial upper. Independent analytic two-control batch test checks each original enclosure, equality to single-control APIs, shared cell accounting and one-cell-short complete refusal. Initial fixture attempted primitive zero-length line for a constant twist and was corrected to a valid constant scalar Curve; no production workaround.

Full progressive native133/133 passed (`/tmp/sweep-batch-values-final-native.log`); previous measurement log `/tmp/sweep-batch-values-native.log`. No WASM build started for this batch change yet: next work is shared trajectory jets so contact can fit budget too, then one combined packaging/public/UI qualification. Existing da10339 public77/STEP38/UI12-144 remains valid only for previous production Rust. Full original six-item objective remains active.

### Shared original trajectory jets: native134, contact work gap remains

Added Rust `TrajectoriesReport` and batch original-law control jets for guided/contact modes. Frame, path, scale, affine and fit restrictions are shared across profile controls, with identical outward control composition and single-span metadata. Single-control APIs retain the same enclosures and source work. Section interpolation charges the batch once per interval; value-only fallback is also batched when jets are unresolved. Any source exhaustion exposes no partial jets/vector, and production10k ceiling remains unchanged.

Measured closed33/129: guided1922/7202cells (from2442/9066 after value batching), contact3050/11402cells (from3830/14198). Fine error upper unchanged1.1569119978329256/1.3949568114739457mm; both fit2mm at sufficient native work. Contact129 still exceeds public10k and must remain Unresolved there. An initial speculative assertion that both would fit10k failed, exposing11402; it was replaced by the measured guided-under10k invariant and the preserved conditional complete/unresolved budget check, without changing the production limit.

Analytic two-control shared-jet test verifies value/first/second against original independent formulas, exact equality to single-control reports, once-only source accounting and whole-batch refusal one cell short. Full progressive native134/134 passed (`/tmp/sweep-batch-jets-final-native.log`); additional fine2mm assertion passed focused test (`/tmp/sweep-batch-jets-fine-two-mm-native.log`). New source remains uncompiled to WASM; next optimize duplicated original restrictions between fit/frame/transport, then combined packaging. da10339 previous public77/STEP38/UI12-144 is historical evidence for its old production recipe only. Full original six requirements remain open.


### Shared contact fit path values: native tight closed acceptance

The contact value batch now reuses the original path value restriction already proved and charged by its owning fit certificate, for the same path and traversal. No work limit or enclosure arithmetic was relaxed. At 129 sections, guided uses 7202 cells with error upper 1.1569119978329256 mm; contact uses 9854 cells with error upper 1.3949568114739457 mm. Both fit the existing 10000-cell budget.

Native regression additionally checks refusal at one cell below actual fine certificate work, and accepted continuous retained-patch bounds at 129 sections with a 2 mm budget while 33 sections remain unaccepted. Full progressive native regression: 134/134, `/tmp/sweep-cache-tight-native.log`. Closed Rush fixtures now request refinement from 33 to 129 sections at 2 mm; public tests require the final accepted preview and retain strict-budget refusal.

Combined WASM rebuild is in progress (`/tmp/sweep-shared-source-cache-wasm.log`, exec session 21628). Public tests and UI for these changes are pending. The previously qualified da10339 artifact remains the published evidence baseline until rebuild and verification complete. This does not prove caps, global embedding, full-body continuousBound, or all-mode smoothness.


Additional cache qualification: native 135/135 (`/tmp/sweep-cache-affine-native.log`). New regression compares cached and uncached original enclosures exactly across affine axes, moving center, two translated source paths, three traversal restrictions including a point, and two profile controls. Saved work equals the original path restriction work; one-cell-short budget leaves the whole control union unresolved. TypeScript check passed (`/tmp/sweep-shared-source-cache-typecheck.log`). WASM session 21628 remains live in optimization; no public artifact qualification claimed yet.


Public refinement assertions now require the 33-to-129 preview acceptance sequence `[false,true]` at the exact 2 mm budget. The surface UI runner additionally parses the displayed retained upper/budget for closed cases and requires budget 2 mm and upper within budget, on its existing wide/narrow matrix. Script syntax and scoped diff checks passed. These checks await the new artifact and have not yet been counted as successful UI/public evidence.

Broader native `nurbs-core --lib` suite launched, session 26281, `/tmp/sweep-shared-source-full-native.log`; still live on last poll. WASM session 21628 also remains live; optimizer Node was observed consuming CPU. Neither process has been restarted.


Broader native result: all `nurbs-core --lib` tests passed, 955/955, 59.96 seconds, `/tmp/sweep-shared-source-full-native.log`, session 26281 exited 0. Worker and viewport presentation suites passed 30/30 (`/tmp/sweep-shared-source-presentation.log`); these presentation checks do not qualify new WASM numeric results. WASM session 21628 is still live on the latest poll, with release compilation complete and optimization pending.


Correction to pending preview assertions: actual native and public iterator refinement doubles station intervals, yielding 33 → 65 → 129. The earlier two-preview `[false,true]` expectation was incorrect and has been replaced by three previews `[false,false,true]`. Native focused regression confirms 65 sections has a continuous retained bound but remains unaccepted at 2 mm; 129 is accepted. `/tmp/sweep-cache-refinement-native.log` passed 1/1. This is a test correction; production refinement behavior was unchanged. WASM numeric/public/UI qualification remains pending session 21628.


### Native JSON transport exposes identity-affine budget overhead

New native transport regression initially failed for closed contact at 129 sections: the transport attaches generated identity affine laws, causing full error work to reach 10000 with `contact-fit-value-unresolved` even though the direct no-affine case used 9854. Sampled acceptance remained true but continuousBound false. This contradicts a public qualification claim; public tests have not been counted as passed.

Rust `Sweep::with_affine_laws` now validates both curves and axis positivity, then recognizes exact identity only when all axis poles are [1,1,1] and all center poles [0,0,0]. Equal rational poles define the identity over the full domain with validated positive weights; the absent transform consumes no extra affine certificate work. Nonidentity laws retain their original source certificates. Progressive native regression passed 136/136 (`/tmp/sweep-cache-identity-native.log`), including JSON transport guided/contact levels 33/65/129 at 2 mm and the shared 10000-cell limit.

Important artifact boundary: this Rust production correction occurred AFTER session21628 completed release compilation and entered optimization. Its output will not include identity normalization. Allow that live build to finish, then rebuild latest Rust before public/Rush/UI/STEP qualification. Do not restart the existing optimizer merely because elapsed time is long.


Identity normalization qualification: progressive native 137/137 (`/tmp/sweep-identity-rational-native.log`). Exact identity rational affine laws with nonuniform positive weights, multiple spans and independently shifted knot domains preserve retained section poles and weights exactly relative to absent affine laws. One-ulp axis perturbation and minimum-positive center perturbation retain affine provenance; zero/negative/NaN weights and unsorted knots are rejected before normalization. Full native suite for latest identity production change started in `/tmp/sweep-identity-full-native.log`; result pending. Initial WASM session21628 is still live and predates this production change.


Latest complete native result including identity normalization and transport: 957/957 passed, `/tmp/sweep-identity-full-native.log`, 65.39 seconds, session96107 exit0. Existing initial optimizer session21628 remains live; observed Node CPU 100% at 11:40 elapsed. Because optimizeSnapshot uses a private immutable input copy, compilation-only latest reproducible geometry-wasm build was started separately (`/tmp/sweep-identity-wasm-compile.log`) without restarting or modifying that optimization. Latest source still requires its own optimization/package and public qualification after the initial build terminates.


Initial batch/cache WASM build session21628 completed exit0. Packaged SHA256 6e574b30ae00185dafc1eb0d525adbd1e0e10f44740def53575583fa01a218d0, 10820741 bytes, optimized from12169913. It predates exact identity affine normalization and is NOT the final qualification target. Latest compile session7037 is still active; canonical latest build launched in `/tmp/sweep-identity-current-wasm.log`, allowing Cargo to serialize through the existing target lock. No source changes planned during this latest build. Primary RAG wiki updated revision13 and index rebuilt.


UI matrix extended while latest optimizer is running: closed guided/contact sources now exercise an actual strict continuous retained-patch error-budget build refusal, assert final patch evidence is cleared, then restore the original 2 mm source and require successful scoped proof presentation again. Both existing window widths apply. Runner syntax and scoped diff checks passed; execution is pending latest WASM packaging and Vite rebuild, so this is not counted as passed UI evidence. Latest build session86956 remains live on poll.


Core-only Rust feature check initially failed because two existing geometric tests contained unconditional JSON transport subchecks in authored_frame_certificate.rs and audit/cap_wall.rs. Only the JSON subchecks now have cfg(feature="transport"); native geometric assertions remain active without codec. This changes cfg(test) code only, not the in-flight production WASM snapshot. Core-only progressive recheck compiled and is running as session29759 (`/tmp/sweep-identity-core-only-native.log`); latest production WASM optimizer session86956 remains live. Do not count this recheck as passed until terminal results are observed.


Core-only progressive native result passed 134/134 without default features, `/tmp/sweep-identity-core-only-native.log`, session29759 exit0. JSON transport subchecks remain enabled with transport feature; all geometric checks remain enabled for core-only Rust. Native BRep integration target `rational_section_loft` started in `/tmp/sweep-identity-brep-native.log` to cover rational body construction, authored/contact laws, holes and STEP roundtrips under latest nurbs-core. Result pending. Latest WASM optimizer session86956 remains live.


Latest native BRep integration `rational_section_loft` passed 16/16 (`/tmp/sweep-identity-brep-native.log`, 13.90 seconds, session68437 exit0), including affine/authored transport, common contact anchor with outer and hole profiles, closed periodic inner shell ownership, and native STEP roundtrip checks. These finite fixtures do not prove arbitrary global geometry or independent complete STEP coverage. Latest WASM session86956 remains live; no public numeric/UI result has been claimed for it.


Current-source next-gap inspection: `Sweep::level_with_error_budget` selects full retained patch error only for authored frame_laws or orientation_guide; ordinary fixed/RMF/Frenet modes have no certificate and report transport-mode-error-unproved. `sections` uses constant initial tangent and normal in Fixed mode, but original source initial-frame enclosure, twist and rounding still must be composed to certify it; merely converting sampled directions into authored laws would not establish original-source provenance. General RMF also applies sampled chord-length closure correction on closed paths, still unproved. These requirements remain part of full goal after latest artifact qualification. Session86956 was verified live with optimizer CPU ~100%; not restarted.


UI artifact provenance strengthened: runner now refuses unless source packed WASM, public WASM and dist public WASM hashes match, generated identity contains that fingerprint, and built geometry worker assets embed it. Report records byte length and verified source/public/dist provenance. Syntax and scoped diff checks passed. Negative qualification with currently stale dist correctly exited1 before browser launch: dist was da10339 while current intermediate packaged kernel was 6e574b30. `/tmp/sweep-stale-dist-provenance-check.log`. This is an intentional stale-artifact refusal, not a geometry/UI case failure. Latest optimizer session86956 remains live; latest Vite build and successful UI execution still pending.


### Latest shared-source and identity-normalized WASM public qualification

Build session86956 completed exit0: latest SHA256 434557816d8080761f4cd3acf1a33d2951e1b0aca009ecce33128bc1d8e47f46, 10820931 bytes, optimized from12170123. Public eight suites passed77/77 (`/tmp/sweep-identity-current-public.log`), including closed guided/contact 129 sections at2mm and three-level [false,false,true] Rush acceptance, strict continuous-error refusal and native patch evidence adapter. Vite build passed (`/tmp/sweep-identity-current-vite.log`). Latest surface headed UI12-case matrix and selected STEP export started in shared-source-current roots; results pending. This does not close remaining all-mode/body/cap/global/smoothness requirements.


### Latest artifact UI, BRep public and selected independent STEP results

On SHA434557816d8080761f4cd3acf1a33d2951e1b0aca009ecce33128bc1d8e47f46: surface UI12/152 passed with source/public/dist/worker fingerprint validation. Closed guided/contact refine to2mm, strict error budget refuses and clears evidence, original source restores; narrow contact screenshot shows <=1.39496 /2mm. Root ui-progressive-surfaces-shared-source-current. Joint frame/guide/affine hollow Solid UI2/22 passed at1440/600; narrow solid.png visually inspected, rendered body visible. Root ui-solid-shared-source-current. These are finite UI fixtures and lifecycle dispatch cancellation, not full all-mode or mid-kernel interruption proof.

Additional body/cap/smoothness/embedding public nine suites passed36/36 (`/tmp/sweep-identity-current-body-public.log`). Initial run exposed two stale expectations: curved reconstruction's full boundary upper is0.9000000000000291mm, so1mm passes; regression now refuses0.8mm and requires full upper in(0.8,1]. Altered retained pole is rejected earlier as retained-wall-mismatch with no wall bound, rather than expected retained-wall-domain-unproved. Native behavior was measured before changing expectations; admission and mutation refusal remain tested.

Independent STEP oracle export, exact retained-generator Fraction volume and OCCT verification all exited0:38 selected cases passed; opencascade-sweep.json records exact latest SHA/10820931 bytes and publicAndPackedVerified:true. Root external-step-shared-source-current. This refreshes selected matrix evidence, not complete arbitrary-mode/cap/global geometry coverage. Ordinary fixed/RMF/Frenet certificates, full body E, arbitrary nesting/intersections and applicable general moving-frame/closed G1/G2 remain open.


### Fixed original frame value foundation — native only

New Rust authored_frame_certificate/fixed_path_values.rs derives constant longitudinal/transverse/binormal directions from an outward certificate of the original path's initial derivative and authored normal. Original twist value intervals rotate that constant basis; batched original path/scale/affine transport reuses the existing interval implementation. No sampled direction is promoted into an original authored law. Work exhaustion leaves the entire union unresolved; parallel normal is unresolved.

Native progressive138/138 passed (`/tmp/sweep-fixed-original-values-native.log`). Independent analytic straight path with scale1+t and twist pi*t/2 is enclosed at endpoint and interior restrictions; one-cell-short refusal tested. This is a value-certificate foundation only: ordinary fixed retained patch report is still unchanged/unproved. Next work: original fixed frame jets, constructor-owned initial coordinate intervals, retained interpolation/rounding/decomposition, then public path. Current packaged434557 artifact predates this foundation; its earlier77+36/UI/STEP results remain evidence for that artifact only. Full six-point goal remains active.


Fixed original trajectory jets foundation: native progressive139/139 passed (`/tmp/sweep-fixed-original-jets-native.log`). The fixed basis is derived from an outward original initial tangent certificate and treated as constant in normalized traversal; original twist contributes all frame derivatives through the existing apply_twist product rules. Existing batched path/scale/affine trajectory transport is reused. Independent analytic scale1+t, angle(pi/2)t checks value/first/second derivatives for two controls over two interior restrictions; one-cell-short returns no jets for the complete batch. No sampled basis is re-authored as source geometry. Constructor-owned initial-coordinate, retained interpolation/rounding and decomposition integration is still pending; fixed continuousBound remains unproved. No new WASM build started for this foundation;434557 remains last qualified artifact.


### Fixed retained-patch certificate connected in native Rust

Constructor-owned fixed initial coordinate intervals now use original path start enclosure and original endpoint derivative frame certificate. Fixed values/jets feed the same retained endpoint rounding, interpolation and profile decomposition union used by guided/contact modes. Level reports select this certificate for ordinary Fixed mode; arc-length correspondence remains explicitly unproved. No global/body/cap or smoothness flag is inferred.

Initial integration exposed excessive image bounds on a cornered piecewise linear path. The exact affine-interval proof now permits constant original scale/twist/affine laws and degree1 equal-weight path, domain[0,1], dyadic station grid, and no source knot strictly inside the station interval. On each such interval transport is affine despite a C0 join at its endpoint, so the original-to-retained error is bounded by endpoint displacement. It still requires the source batch proof and preserves work exhaustion. Other paths/laws use the general interval jet/image bounds.

Native progressive140/140 passed (`/tmp/sweep-fixed-retained-native.log`). New independent analytic scale1+t and twist(pi/2)t retained test checks coarse refusal, 33-section acceptance at0.05mm, dense retained error containment and one-cell-short whole-union refusal. Existing cornered Fixed test also passes unchanged. Latest qualified WASM434557 predates this integration; native closed/affine/decomposition and public/Rush/UI checks for Fixed remain to be completed before packaging a new artifact.


Fixed closed and affine-piece premise qualification: native progressive142/142 passed (`/tmp/sweep-fixed-closed-guards-native.log`). Closed original rational circle Fixed transport refines 33→129, full retained bound <=2mm within10000 cells, covers copied last section, and refuses one-cell-short. Dense original controls are enclosed. A rational degree1 path with unequal weights and a degree1 path with an interior off-grid corner do not qualify for exact affine-piece interpolation; their actual interpolation error exceeds0.5mm and is enclosed by the general bound, so1e-12mm budget refuses. Both regressions preserve continuous assessment rather than masking real displacement with zero remainder. Additional affine/decomposition and native JSON/public/UI qualification remain pending before a Fixed WASM build; last qualified artifact remains434557.


Fixed joint affine/decomposition native progressive143/143 passed (`/tmp/sweep-fixed-affine-decomposition-native.log`). A33-pole rational profile, varying scalar/axis scales and center with constant twist composes384 decomposition products and encloses independently computed original rational profile transport at interior surface samples. One-product-short refuses complete error and patches. Native full suite launched session40067 (`/tmp/sweep-fixed-full-native.log`), pending.

Added open and closed Fixed Rush fixtures and tests/nurbsFixedPatchError.test.ts for full original retained certificate, preview acceptance/refusal, strict budget rejection and patch-vs-body evidence separation. Surface UI source list now includes these fixtures (8sources×2widths when run). Script syntax and diff checks passed; TypeScript check session15557 pending. Public/ UI tests await a newly built Fixed WASM;434557 remains the last qualified artifact and does not include these Fixed changes.


Fixed full native qualification: all nurbs-core963/963 passed (`/tmp/sweep-fixed-full-native.log`,75.04seconds,session40067 exit0). Separate focused Fixed JSON transport subcheck passed (`/tmp/sweep-fixed-json-native.log`,session70772 exit0): original retained error equals the constructor's native bound, continuousBound/accepted true and correct retained-patch scope. Latest source includes Fixed initial coordinates, values/jets, interpolation/corner premise and decomposition. Canonical WASM build launched `/tmp/sweep-fixed-current-wasm.log`; public Fixed/Rush/UI verification awaits packaging. No geometry moved to TypeScript. Last qualified434557 evidence remains historical until latest artifact verification.


Fixed mode boundary checks: focused native test passed1/1 (`/tmp/sweep-fixed-mode-refusals-native.log`). Fixed arc-length retains arc-length-correspondence-unproved with no global error upper; RMF/fixed_normal explicitly refuse Fixed certificate and retain unproved continuous level reports. Added matching three public tests, now5 Fixed tests total, awaiting latest artifact. TypeScript passed (`/tmp/sweep-fixed-refusals-typecheck.log`), scoped diff passed. This adds cfg(test) checks only; production WASM snapshot is unchanged. Build session2692 release compilation completed1m59s and remains live in optimization. Full native963 result predates this additional test; no full964 result claimed.


Pending Fixed UI matrix strengthened: open Fixed fixture must display a proved retained upper within0.05mm, then strict continuous budget must refuse, clear final evidence, and restore successfully after loading the original source. Closed fixtures retain2mm checks. Matrix now targets8sources×2widths with expected208 checks if all pass. Syntax and scoped diff checks passed; none of these added Fixed UI checks have run yet. Latest WASM session2692 remains live in optimization and was not restarted. Existing434557 qualification remains historical.


Latest source BRep regressions passed16/16 (`/tmp/sweep-fixed-brep-native.log`,session46128 exit0). Added separate Fixed hollow-body test passed1/1 (`/tmp/sweep-fixed-owned-brep-native.log`,session98018 exit0): shared surface continuous bound reaches accepted rational outer/hole body, stays within10000 cells and0.01mm budget, native topology is closed, STEP roundtrip preserves closed edges and cap holes, and1e-30mm budget refuses construction. This tests retained patch admission and body topology, not global embedding or complete body boundary E. Only cfg(test) files changed; active production WASM snapshot is unaffected. Build2692 remains live in optimization; latest Fixed public/Rush/UI results pending.


Fixed core-only progressive141/141 passed without default transport features (`/tmp/sweep-fixed-core-only-native.log`,3.41seconds,session3972 exit0). Primary scoped RAG wiki refreshed revision14 with qualified434557 evidence and pending Fixed-native/latest artifact boundary; index rebuilt, append log4401. WASM2692 verified live in optimization; no restart or fixed public qualification claimed.


Independent STEP fixture added for native Fixed linear-scale hollow frustum: outer radius3, inner radius1, length10, scale1→2, zero twist, two original stations. Original retained-patch continuous certificate and constructor globalEmbeddingCertified:false are required before export. Independent analytic volume is560*pi/3; canonical retained-generator Fraction integration can independently replace it before OCCT. Selected matrix target now39 cases. TypeScript/diff checks passed (`/tmp/sweep-fixed-step-typecheck.log`); new fixture export/reference/OCCT has NOT run, awaiting Fixed WASM2692. Added reference data and orchestration only; geometry stays Rust.


Fixed WASM packaging completed successfully (session2692 exit0). Current packaged artifact SHA256 a259e6608e5d987813151bd63b84e84263924d579e84535a53505ab912e082f6,10828488 bytes. Public9 suites82/82 passed (`/tmp/sweep-fixed-public-qualified.log`); Vite build exited0 (`/tmp/sweep-fixed-vite.log`). Initial public run81/82 uncovered a test assumption: closed Fixed already satisfies2mm at the initial33 stations. Removed mandatory refinement for that case, preserving certificate, successful admission and strict-budget refusal assertions; open Fixed still requires initial refusal. UI16 cases and independent STEP39-case export are actively running; neither is claimed complete. Full goal remains incomplete: additional transport modes, full body/caps bounds, global guarantees and all applicable smoothness remain open.


Current a259e660 surface UI runner exited0: all16 cases across1440/600 widths passed,208 checks (`/tmp/sweep-fixed-ui.log`, artifacts `ui-progressive-surfaces-fixed-current`). Includes original Fixed proof display, strict-budget refusal and restoration. Body public9 suites35/36 passed; one closed authored-frame guide affine cavity test exceeded30000ms while independent STEP export/UI ran concurrently (`/tmp/sweep-fixed-body-public.log`). No body suite success claimed; isolated rerun remains required. STEP exporter session44715 remains live after all five unsegmented modes; independent reference/OCCT remains pending.


Current Fixed artifact a259e660 qualified STEP matrix39/39 passed: export session44715 exit0 (`/tmp/sweep-fixed-step-export.log`); independent canonical-generator Fraction volume integration exit0 (`/tmp/sweep-fixed-step-volume.log`), new hollow linear-scale Fixed reference586.4306286700947 (560*pi/3); OCCT session13165 exit0 (`/tmp/sweep-fixed-step-occt.log`). `external-step-fixed-current/opencascade-sweep.json` passed:true,39cases, exact SHA/10828488 publicAndPackedVerified:true. Scope remains fixture import/topology/analytic volume, not universal geometry/smoothness/containment. Isolated embedding file6/6 passed in26.86s (`/tmp/sweep-fixed-embedding-isolated.log`,session34789 exit0); prior concurrent run35/36 timed out the closed frame/guide/affine cavity test, so report separate successful rerun, not a clean full36 run. Current public82/82, UI16/208 remain valid. Next production gap: original FixedNormal moving frame, with original rational path derivatives, singularity/work-budget refusal and full retained-error integration; RMF/Frenet/corrections/body caps/global/smoothness remain open.


New Rust FixedNormal frame implementation in authored_frame_certificate/fixed_normal_path.rs derives normalized tangent jets from original rational path first/second/third derivatives and projects constant seed with differentiated cross/normalization; original twist applied with existing shared budget. No generated guide/path samples used. Frame certificate only, not yet integrated into retained E or packaged WASM. Analytic C(t)=(t,t²,0) test with independent domain[2,5] checks tangent values/d1/d2 and projected normal, one-cell-short refusal and parallel seed refusal. First compilation caught missing test Curve.periodic field, corrected; compile then succeeded, focused native test session37713 currently live, result not yet claimed. Current qualified a259e660 remains earlier Fixed artifact.


Focused FixedNormal test initially refused its invalid zero-length line fixture before certificate execution; changed to native constant_vector_law zero twist, matching existing law construction. Rerun session7987 active; no passing test result claimed yet.


FixedNormal focused analytic frame test passed1/1 after valid constant law fixture correction (`/tmp/sweep-fixed-normal-frame-native.log`,session7987 exit0). This verifies original curved-path tangent jets, projected normal and conservative singularity/work refusal; retained transport E integration/public packaging remains pending.


FixedNormal native source now includes original-path value frame restrictions, authored endpoint first-derivative handling and batch control values/jets with shared source work. Focused expanded analytic test passed1/1 (`/tmp/sweep-fixed-normal-values-native.log`,session47473 exit0), including endpoints0/1 and interior0.375, original domain[2,5], frame derivative analytic bounds and one-cell-short frame/batch refusal. No FixedNormal retained E integration or new WASM qualification claimed. Broader authored-frame regression started separately; original goal scope retained.


FixedNormal retained-patch integration added in Rust: own mode-gated error API, original initial coordinates (same initial seed frame as Fixed), shared original control values/jets, station rounding/interpolation and original profile decomposition. Fixed affine corner shortcut stays restricted to Fixed; FixedNormal uses general moving-frame bounds. Native dense independent C(t)=(t,t²,0), profile with transverse/binormal coordinates and linear scale test passed1/1 (`/tmp/sweep-fixed-normal-retained-native.log`): accepted33 sections at0.1mm, analytic intermediate values enclosed, one-cell-short full-bound refusal and arc-length explicit unproved. Initial progressive run144/145 exposed stale negative FixedNormal expectation now certifiable; Fixed API still mode-refuses and RMF remains unproved. Updated mode test; full rerun session48860 active. New Rust not yet packaged; a259e660 public/UI/STEP remains previous artifact evidence only.


Latest integrated native progressive146/146 passed (`/tmp/sweep-fixed-normal-integrated-native.log`,session48860 exit0). Full native core and fresh reproducible FixedNormal WASM build started separately; current public evidence still a259e660 until latest artifact completes and qualifies.


Latest FixedNormal integrated full nurbs-core966/966 passed in92.68s (`/tmp/sweep-fixed-normal-full-native.log`,session24827 exit0). Progressive146/146 previously passed. New Rush fixed-normal-progressive-sweep.r prepared with original quadratic path, moving tangent/constant projected normal, linear scale and0.1mm budget. Public Fixed suite now includes this successful source and strict refusal; FixedNormal removed from obsolete unproved-transport expectation, RMF remains negative; both Fixed/FixedNormal arc-length explicit unproved tested. Surface UI now9sources×2widths pending latest WASM, numeric0.1mm/strict refusal/restoration included. vue-tsc exit0 (`/tmp/sweep-fixed-normal-typecheck.log`), runner syntax/scoped diff passed. WASM93607 remains live compiling geometry-wasm; no public FixedNormal or UI claims yet. Current qualified a259e660 predates these Rust changes.


FixedNormal native JSON focused test passed1/1 (`/tmp/sweep-fixed-normal-json-native.log`,session77987 exit0): public operation surface_progressive_sweep_level/ orientation fixed_normal carries exact original retained upper/scope and accepted:true at0.1mm, then accepted:false at1e-30 while continuousBound remains truthful. Only cfg(test) source changed during active packaging. Latest vue-tsc passed (`/tmp/sweep-fixed-normal-qualified-typecheck.log`). WASM93607 release finished2m34s, optimization live; no restart. Added FixedNormal hollow BRep/native STEP roundtrip regression (linear original path, linear scale, nonzero twist) now running65968. Independent STEP target expanded40 cases via same canonical annular-frustum generator with fixed_normal transport, analytic560*pi/3 and native original retained certificate/global false required; exporter/typecheck passed (`/tmp/sweep-fixed-normal-step-typecheck.log`), new fixture not exported/OCCT-qualified yet. Full goal remains incomplete.


FixedNormal hollow BRep focused test passed1/1 (`/tmp/sweep-fixed-normal-brep-native.log`,session65968 exit0): native retained bound reaches rational outer/hole body within0.01mm and10000 cells, closed native edges/cap holes and native STEP roundtrip preserved, strict1e-30 construction refused. Does not certify complete body Hausdorff bound/global embedding. WASM93607 remains live in optimization.


Expanded FixedNormal native qualification: original rational generator C(t)=(t,t²,0)/(1+t), independent source domain[7,11], linear uniform/axis scale, center offsets and moving twist has certified retained bound within0.1mm at33 sections. Dense intermediate original equation checked; one-cell-short full-bound refusal preserved. Focused test1/1 (`/tmp/sweep-fixed-normal-rational-affine-native.log`,session78458 exit0). Interior singularity negative test C(t)=(t²-t,0,t) with constant Z seed proves both endpoints regular but refuses entire interval and midpoint with no partial frame, jets reason fixed-normal-projection-unresolved. Combined FixedNormal4/4 passed (`/tmp/sweep-fixed-normal-negative-native.log`,session77047 exit0). Only cfg(test) edits during active WASM93607 optimization, production snapshot unchanged; full966 evidence predates these two added tests, no full968 run claimed. Public/Rush/UI/STEP still pending latest packaged artifact.


Closed FixedNormal original rational-circle radial transport and copied seam focused test passed1/1 (`/tmp/sweep-fixed-normal-closed-native.log`,session41542 exit0). Profile has radial side offset1 plus Z heights1/2, so it exercises moving-frame transport rather than only constant Z. Fine129 retained certificate accepted within2mm/10000 cells, dense original circle radial equation enclosed, station endpoints copied identically and one-cell-short whole bound refused. Added closed-fixed-normal-progressive-sweep.r to public source tests and wide/narrow matrix now10sources×2; syntax/scoped diff passed, typecheck running1912. No public/UI closed-FixedNormal pass claimed. New additions are cfg(test)/qualification inputs only, active production WASM93607 snapshot unchanged; optimizer live, last full966 predates three addedtests, no full969 result claimed.


FixedNormal dense rational profile decomposition focused test passed1/1 (`/tmp/sweep-fixed-normal-decomposition-native.log`,session6800 exit0):33 rational poles/32degree1 spans, affine axes and center plus uniform scale/constant twist,384products; dense rational-profile values against independent original equation enclosed;383product budget refuses patches and aggregate error. Separate core-only progressive run57592 active. Production snapshot unchanged (cfg(test) edit only). WASM93607 verified live, packaged identity still historical a259e660; public/Rush/UI/STEP latest FixedNormal remain pending. Full default966 evidence predates four addedtests; no full970 result claimed.


FixedNormal union work regression passed1/1 (`/tmp/sweep-fixed-normal-union-native.log`,session12092 exit0):2 profiles33sections certified,64profiles share10000-cell ceiling, partial union retains known-profile error but discards full continuous upper, strict1e-30 refuses even zero sampled deviation. Public matching test prepared, pending latest artifact. Process inspection confirmed WASM build27960 and optimizer child29933 live9m08s at98.4%CPU, so no restart or terminal claim. Core-only latest147/147 passed (`/tmp/sweep-fixed-normal-core-only-native.log`); this predates union test. Full966 predates five addedtests, no full971 result claimed.


FixedNormal latest WASM93607 packaging completed exit0. Artifact SHA256 a413130aa8c93c4a03539bb190b7872338dea987346b82b4508ca8eee4c6286c,10835548bytes (12185751 raw before optimization). Started latest public9 suites session16173 (`/tmp/sweep-fixed-normal-public.log`), Vite24954 (`/tmp/sweep-fixed-normal-vite.log`) and40-case STEP exporter82062 (`/tmp/sweep-fixed-normal-step-export.log`,root external-step-fixed-normal-current). No new public/STEP pass claimed until terminalresults; UI waits new dist build. Contract matrix updated with artifact-scoped previous Fixed qualification/native FixedNormal boundaries.


Current a413130 public9 suites85/85 passed (`/tmp/sweep-fixed-normal-public.log`,session16173 exit0), Vite built1.75s/session24954 exit0. Started headed20-case surface UI root ui-progressive-surfaces-fixed-normal-current, log /tmp/sweep-fixed-normal-ui.log. Independent STEP exporter82062 still running; reference/OCCT pending.


Current a413130 surface UI20/20 cases264 assertions passed (`/tmp/sweep-fixed-normal-ui.log`,session59282 exit0), actual matrix artifact provenance SHA/10835548/source-public-dist-worker verified. Narrow open FixedNormal screenshot visually inspected: rendered yellow curved surface and proved retained-patch upper0.0196400/0.1mm, separate original-transport scope. Expected open-surface Solid refusal/strict budget/cancellation/source restoration covered. STEP export82062 exited0; independent exact retained-generator volume exited0 (`/tmp/sweep-fixed-normal-step-volume.log`), Fixed and FixedNormal frustum volumes586.4306286700947 each. OCCT started, result pending. No universal/full body or complete all-mode matrix claim.


Current a413130 independent OCCT40/40 passed (`/tmp/sweep-fixed-normal-step-occt.log`,session24812 exit0), exact public/packed SHA10835548 verified by opencascade-sweep.json. Scope fixture import/topology/independent volume, not universal surfaces/seams/containment. Additional body/cap/smoothness9-suite public rerun95056 started with explicit node project/maxWorkers1, result pending. Full six-requirement goal remains active.


Current a413130 additional body/cap/smoothness public9 suites36/36 passed (`/tmp/sweep-fixed-normal-body-public.log`,session95056 exit0,39.35s). This is a clean full rerun on latest artifact, replacing earlier concurrent timeout limitation only for this artifact. Public85/85/UI20-264/STEP40 and extra36 are finite scoped evidence; original six obligations remain open.


New Rust original FixedNormal adaptive frame cover added after qualified a413130 artifact: complete ordered traversal intervals only after all cells certify nonzero tangent/nonparallel seed, one shared budget; circle subdivisions cover0→1 contiguously, radial binormal values enclosed, one-cell-short/zero budget and interior singularity refuse with no partial intervals. Native focused cover and JSON transport test passed1/1 (`/tmp/sweep-fixed-normal-cover-json-native.log`,session49144 exit0). New operation sweep_fixed_normal_frame_regularity explicitly returns continuousBound/surfaceRegularityCertified/globalEmbeddingCertified:false, proving frame premise only. TS adapter contains request/report typing only; new operation not yet in packaged a413130. Broader frame regressions started, typecheck pending78084. No new public/WASM cover claim; full original goal remains active.


Original Frenet frame value source added in Rust after a413130: rational path first/second outward derivative restrictions, normalized tangent and curvature cross basis, original twist interval rotation; batch controls share original source/law values. No sampled/authored proxy frame. Analytical quadratic path independent domain[2,5] with twist endpoints/interior and one-cell-short refusal passed; straight curvature refuses. Cubic C(t)=(t,(t-1/2)^3,0) has regular endpoint curvature but full interval/midpoint frame refuse without partial values. Native2/2 passed (`/tmp/sweep-frenet-values-negative-native.log`,session52300 exit0), scoped diff passed. Frenet retained interpolation, initial-coordinate wiring, fourth derivatives/second frame jets, corrected Frenet/RMF remain pending. FixedNormal cover/regressions30/30 and TS adapter typecheck passed; both new original-frame APIs are not yet packaged/public-qualified. a413130 remains previous qualified snapshot.


Frenet retained-patch integration now uses original Frenet initial coordinates, shared original control values, retained endpoint rounding/image fallback, and original profile decomposition. Replaced boolean transport selector with enum OriginalTransport to prevent mixed modes. General Frenet frame values give conservative original interval image bound; full spatial second frame jets remain pending. Initial native153/154 exposed old quarter-circle0.01mm admission failure because value-only bound was too loose. Preserved that tolerance and implemented exact original coordinate-plane premise: all original control poles share one coordinate, strict curvature-cross sign proves constant binormal, original C3 tangent jets give second frame jets, twist and affine/scale chain rules compose. No sampled/reauthored path substituted. Revised native155/155 passed (`/tmp/sweep-frenet-integrated-native.log`,session62067 exit0). Dense analytic curved scale/twist retained test257sections≤0.1mm, work exhaustion and arc-length refusal passed. Independent analytic control value/d1/d2 test with original domain[2,5], one-cell-short batch refusal and nonplanar one-MIN_POSITIVE pole perturbation passed1/1 (`/tmp/sweep-frenet-planar-jets-native.log`,session5044 exit0). Latest full native run started, no current full976 pass claimed. New production not yet packaged; qualified a413130 remains historical public/UI/STEP. CorrectedFrenet/RMF/arc-length/full-body/global/smoothness remain pending.


Original rational fourth scalar/XYZ derivative certificates added, sharing lower jets and all components under one budget. Full-span homogeneous differentiation plus exact Leibniz recurrence preserves rational denominator terms; narrow restrictions do not divide tiny rounded pole differences. Independent rational2t/(1+t) D4 on domain[2,5] and XYZ amplitudes tests2/2 passed (`/tmp/sweep-fourth-jets-native.log`,session22022 exit0), one-cell-short removes all usable lower/higher jets. Previous source full976/976 passed62.26s (`/tmp/sweep-frenet-full-native.log`,session21650 exit0), predating fourth source.

Spatial original Frenet frame now uses C1-C4 outward jets, normalized velocity, normalized velocity×acceleration and differentiated frame/twist chain rules, integrated into shared control trajectory retained E. Exact coordinate-plane paths retain cheaper proven constant binormal branch; nonplanar uses full fourth source. Native progressive158/158 passed (`/tmp/sweep-frenet-spatial-integrated-native.log`,session79422 exit0). Independent spatial C(t)=(t,t²,t³) original domain[2,5] tangent/binormal value/d1/d2 analytical test and one-cell-short complete-frame refusal passed1/1 (`/tmp/sweep-frenet-spatial-axes-native.log`,session99929 exit0). New current full native and WASM build started separately, logs /tmp/sweep-frenet-spatial-full-native.log and /tmp/sweep-frenet-current-wasm.log. No latest public/WASM Frenet result claimed; qualified a413130 remains prior artifact. Full goal remains active.


Current full native source979/979 passed88.15s (`/tmp/sweep-frenet-spatial-full-native.log`,session57989 exit0), predating subsequent one cfg(test) addition. Spatial Frenet retained test passed1/1 (`/tmp/sweep-frenet-spatial-retained-native.log`,session77407 exit0): original C(t)=(t,t²,t³) domain[2,5], varying scale/twist,129sections within0.1mm and10000cells; dense independent T/B/N transport equation enclosed, coarse refusal and one-cell-short complete-error refusal. No full980 result claimed. Added planar/spatial Frenet Rush sources to async public proof/strict refusal, added arc-length Frenet explicit unproved check, and new FixedNormal frame-cover public tests. Wide/narrow surface matrix now12sources×2 (24cases), numeric0.1mm/strict refusal/restoration for both Frenet sources. TypeScript/syntax/scoped diff passed. Public execution awaits new artifact. WASM14593 release finished2m28s, optimizer live; production snapshot unaffected by later cfg(test)/TS/fixture edits. Last qualified a413130 does not contain new Frenet/Fourth/FixedNormal cover code.


Closed Frenet native test initially refused nominal C0/C1 rational-circle knots because Curve::evaluate deliberately hides unavailable shared second derivative. Exact original coordinate-plane/sign value certificate now fixes binormal, and constructor may use a normalized midpoint only when original one-sided tangent and principal-normal enclosures have diameter≤1e-10; no sampled neighbor or reauthored frame. Generic evaluator continuity semantics unchanged. Closed circle radial transport/copied seam129sections≤2mm/10000 cells now passed1/1 (`/tmp/sweep-frenet-closed-native.log`,session75095 exit0). Genuine C1 join with Y principal normal on left and Z on right refuses sections and retained proof (`/tmp/sweep-frenet-knot-refusal-native.log`,1/1,session67941 exit0). Source changed after WASM14593 immutable release snapshot: that pending package will NOT qualify new knot constructor fix. Started compile-only latest Rust using existing reproducibleCargo settings, no package writes; optimizer reads private immutable input snapshot, so existing14593 optimization is preserved. New progressive rerun started. Latest source will require own optimized package after pending publication completes.


Frenet curved hollow native BRep/STEP focused test passed1/1 (`/tmp/sweep-frenet-brep-native.log`,session72434 exit0): original quadratic path, rational outer0.1/inner0.05 circles in initial tangent-normal plane, original retained E within0.01mm/10000 cells, closed body edges/two cap holes, native STEP roundtrip preserves topology, strict1e-30 construction refuses. This does not prove full cap/body Hausdorff/global embedding. Independent selected STEP target41 now includes frenet-quadratic-hollow.step, original nominal tube volume and canonical retained-generator Fraction reference before OCCT; new fixture not exported/reference/OCCT-qualified yet. TypeScript passed (`/tmp/sweep-frenet-step-typecheck.log`,session58901 exit0). Closed Frenet Rush added, public source tests and13-source×2 UI target26 await latest knot-fix artifact.

Compile-only latest knot-fix Rust completed2m51s/session79736 exit0; no package writes. Older14593 optimized12207254→10855261 bytes and remains packaging, predating knot-fix production. Must let it publish terminal before packaging latest source to avoid competing writes. Current qualified a413130 identity still visible; no new artifact hash/results claimed yet.


Older Frenet WASM14593 packaging exited0; latest reproducible knot-fix build started only after terminal publication, log /tmp/sweep-frenet-closed-current-wasm.log. No competing package writes or canceled optimizer. Intermediate Frenet package predates closed-knot source and is not the artifact used to qualify full prepared matrix; latest package required.


Current knot-fix source complete nurbs-core982/982 passed113.83s (`/tmp/sweep-frenet-closed-full-native.log`,session59922 exit0), including closed Frenet/rational join/source disagreement regression. Entire native rational-section-loft BRep19/19 passed52.23s (`/tmp/sweep-frenet-full-brep-native.log`,session71624 exit0), including new Frenet hollow body and strict refusal. Current latest knot-fix WASM46860 release used compiled snapshot and completed0.12s, verified live optimizing. Intermediate30b4e7d9be7e3c269e6820fadc97bd3476a082a1c68b9bf912521f2b8b2a84c6 (10855261bytes) predates knot fix; it is not qualified as latest source. Public26-case UI/41 STEP qualification still pending latest optimized artifact. No full body/cap/global/smoothness/all-mode completion claim; original six requirements remain active.


### Frenet shared-profile budget follow-up

The public `tests/nurbsFixedPatchError.test.ts` shared-profile regressions passed for both FixedNormal and Frenet (2 passed, 12 skipped; `/tmp/sweep-frenet-union-intermediate-public.log`). This is intermediate-package evidence only, not qualification of the pending closed-knot source package. The native Frenet budget-accounting fixture now explicitly uses the matching public 1 mm admission budget; the production tolerance was not changed.

Native recompilation is temporarily prevented by concurrent unrelated analysis additions: missing `curvature_spectrum`, `higher_derivative_map`, and `reference_deviation` modules, plus usize dereference errors in `span_continuity_map.rs` (`/tmp/sweep-frenet-union-native-recheck.log`, exit 101). The earlier watermark compilation errors have already been repaired in current shared files. No unrelated source was reverted. The latest WASM publication handle 46860 remained live when polled; no replacement pack writer was started. Full source/public/UI/STEP qualification remains pending.


### 2026-10-05 current Frenet package qualification

The previously pending optimizer completed and published geometry WASM SHA256 `92b438f8f0f2931b10ad4f508e94fe3b2b0855641a3649dc2fa97cccd46d8b60`, 10,857,060 bytes. Ten public suites passed 93/93 (`/tmp/sweep-frenet-public-20261005.log`), including closed Frenet, joint spatial affine/center/twist, shared profile budgets, and FixedNormal adaptive frame cover. Current native shared-profile Frenet test passed 1/1 (`/tmp/sweep-frenet-union-native-20261005.log`); the previous unrelated analysis compilation blocker is resolved in current files. Vite build passed (`/tmp/sweep-frenet-vite-20261005.log`). Full native library, 28-case headed surface UI matrix, and 41-case STEP export/oracle remain running or pending; no completion claim for those gates or the full library scope.

Current native full library completed: **1019/1019**, 65.91 s, log `/tmp/sweep-frenet-full-native-20261005.log`. This covers the current shared library tests; it does not prove unimplemented all-mode global/cap/smoothness guarantees.

STEP export completed (exit 0). Independent volume reference refused the new Frenet fixture with `Noncanonical retained poles` (`/tmp/sweep-frenet-step-volume-20261005.log`, exit 1). The canonical polynomial-generator exactness assumption does not hold for these rounded transported poles. This is an unresolved oracle applicability gate; no tolerance/assertion was weakened and no 41/41 qualification is claimed. Next work must provide an independent reference valid for the actual rational retained geometry.


### 2026-10-05 rational Frenet oracle and UI completion

Added independent `scripts/reference-rational-boundary-volume.py`: divergence-theorem rational Bezier wall flux plus outward planar cap-contour flux, Gauss orders 16/32/64 with strict convergence checks. This is numerical reference evidence, explicitly not interval certification. Rational annular cylinder and translated-cylinder analytic regressions passed (`3*pi/4`); nonplanar caps refused. Exporter supplies actual unique shell face orientations and outward cap contours. The old canonical exact-reference assertions remain unchanged for applicable cases. OCCT verifier requires convergence metadata before accepting this reference. Typecheck and scoped diff check passed.

Frenet retained-body volume: 0.03473701422615421 mm^3; order results [0.03473701422615422, 0.03473701422615423, 0.03473701422615421]. Export, reference preparation and independent OCCT all completed successfully; see `external-step-frenet-current/occt-report.json` and `/tmp/sweep-frenet-step-occt-20261005.log`.

Headed surface UI completed **28 cases / 376 checks**, wide1440/narrow600, `ui-progressive-surfaces-frenet-current/matrix.json`, passed=true. This includes explicit refusal of Solid for these unproved surface fixtures; successful Solid coverage remains a separate matrix. Current package SHA256 92b438f8f0f2931b10ad4f508e94fe3b2b0855641a3649dc2fa97cccd46d8b60. Full all-mode cap/global/smoothness and cancellation guarantees remain incomplete.


### 2026-10-05 successful-Solid and cap/smoothness qualification follow-up

Nine current public cap/boundary/profile/station smoothness suites passed35/35,34.91s (`/tmp/sweep-caps-smooth-current-20261005.log`). Full headed non-surface matrix started with existing runner, output `ui-miter-solid-frenet-current`, process handle61433, log `/tmp/sweep-miter-solid-ui-20261005.log`. Completed cases include successful Solid publication, held-dispatch build/Solid cancellation, source replacement, restoration and bounded refusal preserving the retained body. Whole matrix remains pending; this evidence does not certify actual mid-kernel cancellation or close general cap/global/smoothness obligations. Contract matrix updated with current Frenet evidence and remaining scope.


### Native actual-retained regularity (2026-10-05)

Added Rust `Level::certify_retained_regularity` and `MultiLevel::certify_retained_regularity`, returning independent `RetainedRegularityReport`. All actual patches share one bounded surface-Jacobian certificate budget; invalid surfaces are validated even at zero work; exhaustion/numerical failure never certifies a partial level. No E admission, global embedding, nesting, caps or seam status is promoted. Two focused native tests passed: interior bilinear fold and one-cell-short shared budget refused; retained patches for Fixed/FixedNormal/Frenet/CorrectedFrenet/RMF are regular on an independently refused strict-E preview; coincident multi-profile walls explicitly do not turn R into I. Logs `/tmp/sweep-retained-regularity-native-20261005.log` and `/tmp/sweep-retained-frame-regularity-native-20261005.log`. Public/WASM exposure of this new method is pending; existing package92b438f8 qualifications predate it. Full progressive regression is running.

Focused progressive-sweep module regression completed **69/69**,5.07s (`/tmp/sweep-retained-regularity-progressive-20261005.log`); this is the exact selected module count, not a full-library rerun.


### Shared retained-R JSON/WASM boundary (2026-10-05)

Added Rust JSON operation `sweep_retained_patch_regularity` plus thin TS adapter `nurbsRetainedPatchRegularity.ts`. It returns actual surface Jacobian regularity, cells and unresolved patch indices while explicitly retaining `continuousBound=false`, `globalEmbeddingCertified=false`, `solidCertified=false`. Native transport/regularity tests passed3/3 (`/tmp/sweep-retained-regularity-transport-final-20261005.log`). Public regressions prepared for shared/zero/one-cell-short budgets, an interior bilinear fold, empty inputs and invalid weights at zero work. They require the next package and have not yet run. Geometry WASM build76912 is live (`/tmp/sweep-retained-regularity-wasm-20261005.log`); no all-source qualification is claimed before its publication. Successful-Solid matrix61433 remains running against preceding package92b438f8.


### Solid UI negative-fixture repair (2026-10-05)

Full Solid UI run61433 terminated with one stale negative expectation after33 completed wide cases/376 assertions. `progressive-miter-reconstructed-stations.r` has a current complete boundary error about0.9mm: the harness changed the reconstruction budget2mm to1mm and incorrectly expected refusal. Corrected negative fixture to0.1mm and asserted the source mutation is nonempty. Targeted headed run passed both1440/600,2 cases/28 assertions (`ui-miter-reconstruction-refusal-current/matrix.json`, log `/tmp/sweep-miter-reconstruction-refusal-ui-20261005.log`). Full rerun94870 started in separate artifact root `ui-miter-solid-frenet-fixed-refusal-current`. Prior failed full report is preserved; no whole-matrix pass claimed. New regularity WASM76912 completed release compilation1m44s and remains live during postprocessing/publication; installed package still92b438f8.


### Actual-mode retained-R public regression preparation

Current full native core run passed1022/1022,74.81s (`/tmp/sweep-retained-regularity-full-native-20261005.log`). Public retained-R tests now include actual multi-profile previews produced through the existing native sweep adapter for Fixed/FixedNormal/Frenet/CorrectedFrenet/RMF. Each preview is deliberately refused at E tolerance1e-30, independently checks R, and includes coincident profiles so R cannot be mistaken for I/global admission. Typecheck passed (`/tmp/sweep-retained-regularity-actual-preview-typecheck.log`). Seven public tests remain pending the live WASM76912 publication. Full Solid UI94870 remains live; completed cases only qualify preceding package92b438f8.


### Retained-R report propagation to Rush and viewport

The existing TS sweep construction adapter now attaches separate native retained-R evidence to accepted published patch sets (shared10000-cell budget); it does not change native E admission. Rejected previews can call the independent API. The report travels through existing Rush native geometry artifact metadata. Viewport displays separate retained-surface regularity, and accepts only correct method, bounded integer work, empty unresolved patch list and explicitly false E/global/Solid flags. Typecheck passed; nine viewport reader tests passed (`/tmp/sweep-retained-regularity-viewport-reader-test.log`), including invalid/partial proofs and R independent of E. Public five-mode tests additionally assert accepted previews carry the native report; execution still awaits live package76912. No new WASM pack writer was started.


### Pending package and separate-R browser assertion

The surface UI runner now requires an explicit separate retained-surface regularity line; it does not infer regularity from continuous E or silently promote an unresolved result. Syntax/scoped diff checks passed. This updated surface run must use the forthcoming package and rebuilt viewport; it has not executed yet. Solid full rerun94870 is still live against frozen preceding dist/package92b438f8,43 completed cases/501 assertions at last report inspection, with moving-frame/guide/affine narrow case running. WASM76912 remains live and installed public SHA is unchanged; no restart or competing publisher was launched.


### Original whole-path Frenet frame regularity

Added Rust `certify_frenet_cover`: adaptive complete normalized traversal cover using original rational path jets through C4, twist and one shared cell budget. It certifies nonzero tangent/curvature frame prerequisites; not knot/seam continuity, surface Jacobians, intersections or body E. Focused native and JSON transport checks passed; Frenet regressions16/16 passed,5.84s (`/tmp/sweep-frenet-cover-regression-20261005.log`). Tests include independent path domain2..5, contiguous whole traversal, one-cell-short/zero budget discarding all usable intervals and an interior cubic inflection refused despite regular endpoints. Operation `sweep_frenet_frame_regularity` and thin TS adapter/public tests prepared; typecheck/scoped diff passed.

This new cover operation postdates the immutable input of live WASM76912; it requires a subsequent package. Started compile-only reproducible geometry build to prepare it, without public/source package writes; the optimizer uses its existing private snapshot. Current installed package remains92b438f8. Public qualification for retained-R awaits76912, and whole-frame cover awaits the following package. No competing pack writer started.


### Retained-R current public package and full Solid UI completion

WASM76912 completed exit0 and published SHA2567381f2248c5af5e0530ddfbf32473237cad3c4c88d790c9553184793ab95d7d3 (10,858,858bytes). Eleven public suites passed101/101,5.18s (`/tmp/sweep-retained-regularity-public-20261005.log`), including seven new retained-R tests on actual Fixed/FixedNormal/Frenet/CorrectedFrenet/RMF previews and report propagation; Vite build passed. The later whole Frenet cover operation is not included in this package; compile-only20699 remains live for it.

Full corrected-refusal successful-Solid UI matrix94870 completed exit0,passed=true,**74 cases/852 checks** at1440/600; `ui-miter-solid-frenet-fixed-refusal-current/matrix.json`. That frozen browser run qualifies preceding package92b438f8 and preceding frontend, not the new regularity presentation. Held-dispatch cancellation is lifecycle evidence, not actual mid-kernel interruption. The previous failed matrix remains preserved. New headed surface UI run started against7381f224 with separate regularity presentation assertion in `ui-progressive-surfaces-retained-regularity-current`; pending.


Compile-only20699 completed exit0,release1m35s. After previous publisher76912 completed, started next package89037 (`/tmp/sweep-frenet-cover-current-wasm-20261005.log`) for whole Frenet cover; no concurrent package publisher. Current retained-R surface UI80612 continues against frozen7381f224 dist. Inspected1440 spatial-Frenet-affine screenshot: rendered surface and separate regularity-certified presentation visible, retained E0.0185835/0.1mm; no solid/volume claim for the open surface. Full surface matrix pending.


### Retained-R browser completion and rational closed Frenet cover

Headed surface run80612 completed exit0,passed=true,**28 cases/404 checks** at1440/600 against package7381f224 (`ui-progressive-surfaces-retained-regularity-current/matrix.json`). Separate native retained-surface regularity presentation survives refusal/cancellation/source change/restoration scenarios. Existing full Solid74/852 remains preceding package92b438f8 evidence.

Added cfg(test)-only rational closed-circle Frenet whole-jet-cover test: complete contiguous0..1 cover including all three internal knot sides and one-cell-short full-cover refusal. Native1/1 passed (`/tmp/sweep-frenet-closed-cover-native-20261005.log`); this does not certify closed seam G1/G2. Public corresponding third test prepared. Production source unchanged by this additional test, so live package89037 need not restart. Public whole-cover operation remains pending this package.


### Original source-frame report plumbing

Existing TS adapters now request Rust FixedNormal/Frenet full original frame cover for accepted unguided patch results, and preserve this independent report in Rush artifact metadata. Guided/contact and other frame modes do not receive a substituted Frenet path report. Shared degrees-to-radians request conversion is unchanged and factored once; no geometric certificate algorithm added to TS. Viewport separately displays original-frame regularity, retained-surface regularity and continuous retained E. Whole-frame proof reader requires correct method, bounded integer work, positive complete interval count and explicit false E/surface/global flags; no elevation of Solid. Typecheck and ten viewport-reader tests passed (`/tmp/sweep-source-frame-viewport-reader-test.log`). Public actual-mode tests now assert source-frame report for FixedNormal/Frenet, absence for other modes and authored guides. Full public/native-to-viewport qualification still awaits live package89037 containing the cover RPC; package7381f224101/101 evidence predates this extra plumbing.


### Whole Frenet cover package qualification and RMF straight-source work

Package89037 completed exit0, SHA25608bebcc710f1be9203936f09fa234891f4a028a6eef1bee25a940f140c20afba,10,907,946bytes. Twelve public suites passed106/106 (`/tmp/sweep-frenet-cover-public-20261005.log`), including original full Frenet frame cover on spatial/closed paths, inflection/partial budget refusal, source-frame report propagation and guide-frame non-substitution. Vite/typecheck passed; updated source-frame surface UI70631 is live in `ui-progressive-surfaces-frame-cover-current`.

Later native RMF work derives constant ideal orientation only from an exact original axis-aligned degree-one positive-weight rational line. Scalar/affine/center/twist laws, stored station rounding and profile decomposition reuse native interval transport composition under this new proven premise; general curved/oblique RMF stays explicitly unresolved. No eligibility inferred from station samples. First integration run showed68 passed and two outdated negative assertions on straight RMF, updated to the new separate source proof. Added independent dense rational profile/path affine/twist equation and one-cell-short/min-positive off-axis rejection. Test parameter domains corrected to each retained patch active V interval. Latest recompilation has been intermittently prevented by concurrent unrelated missing helper/import edits (`interval_newton.rs` and `fillet_surfaces.rs`); those were repaired externally in current files, and recheck is running. RMF work is not included in08bebcc7 and has no final qualification claim yet.


### Original-frame UI completion and straight RMF native verification

Whole-frame surface UI70631 completed exit0, passed=true, 28 cases/416 checks at1440/600 against package08bebcc7; `ui-progressive-surfaces-frame-cover-current/matrix.json`. This adds explicit original-frame presentation checks to retained-R/E and lifecycle scenarios.

Focused Rust progressive sweep module passed71/71,7.62s (`/tmp/sweep-rmf-straight-native-recheck-20261005.log`), including independent rational straight RMF affine/center/twist equations over each actual patch domain, shared budget refusal and a min-positive non-axis source refusal. Updated public tests await an RMF-containing package; they must not run against08bebcc7. Typecheck71348 completed exit0. Full native92848 and WASM44787 failed on concurrent unrelated isosurface marching signature mismatch (seven call arguments versus six definition arguments); current definition now has the matching config argument. Fresh full native48937 and single WASM publisher26815 started after both failed handles were terminal. Full RMF/package qualification remains pending.

Full native48937 and WASM26815 are now terminal exit101. Current compile diagnostics are concurrent non-sweep changes: RANSAC Result/Option propagation (`ransac_primitives.rs:482`), mutable budget closure (`surface_offset.rs:266`), and four fitting test unwrap_err Debug bounds. No new package published;08bebcc7 remains last qualified package. These failures do not invalidate the preceding focused71/71 snapshot but prevent current full-source/package qualification.


### Build repair and current RMF qualification processes

RANSAC Result propagation was already repaired in current source. Applied minimal compile repairs: mutable captured-budget closure in surface_offset; four fitting negative tests use err().expect rather than unwrap_err requiring unrelated success-type Debug implementations. Scoped diff check passed. Full native26542 is live and has reached test execution (`/tmp/sweep-rmf-straight-full-native-repaired-20261005.log`); sole WASM publisher62356 is live and passed nurbs-core compilation (`/tmp/sweep-rmf-straight-wasm-repaired-20261005.log`). Neither has terminal success evidence yet; do not restart these live processes or run new RMF public assertions on old08bebcc7.


### Full Rust success and axis-direction RMF coverage

Full native26542 completed exit0:1064/1064,46.73s. WASM62356 release compilation completed1m33s; packaging process still live, no final artifact success yet. Added test-only independent straight RMF oracle over six axis directions, translated source, unequal rational path weights and actual retained patch domains. First axis test compilations30952/12195 failed on concurrent OBB budget signature changes; repaired recursive calls to forward the same guard, preserving depth-budget ownership. Focused recheck62909 is live (`/tmp/sweep-rmf-axis-matrix-native-fixed-20261005.log`). The1064 pass precedes this test-only addition and later OBB changes.

Axis recheck62909 terminated101 on a new concurrent knot_removal negative-test Debug bound. Applied the same narrow err().expect test repair; final focused recheck started in `/tmp/sweep-rmf-axis-matrix-native-final-20261005.log`. No six-direction success claim yet. WASM62356 remains live.

Axis15735 failed on concurrent function_surface test local request shadowing helper; fixed first local name. Axis86915 reached execution and exposed test-fixture zero-length line construction for constant laws, replaced with explicit constant NURBS poles. Axis93823 then failed on duplicated watermark MAX_PAYLOAD_BYTES constant; removed identical adjacent duplicate. Current axis73404 is live (`/tmp/sweep-rmf-axis-matrix-native-deduplicated-20261005.log`); success still unproven. WASM62356 remains live; process inspection confirms optimizer using its immutable private input, no competing publisher.


### Six-direction straight RMF oracle passed

Axis native73404 completed exit0,1/1,0.27s: six translated rational axis directions (including negative traversal) checked against independent closed-form source positions at actual retained patch parameters, within native certified E. `/tmp/sweep-rmf-axis-matrix-native-deduplicated-20261005.log`. This is source-premise coverage, not general oblique/curved RMF proof. Added three public axis cases each exercising both directions; public execution awaits live WASM62356. Typecheck96138 is live. Source/public package remains preceding08bebcc7 until publisher completion; no concurrent publisher or timeout restart.

Added existing progressive-sweep.r and affine-progressive-sweep.r to actual Rush preview/viewport retained-E integration and strict tolerance refusal matrix. Native proof remains Rust; TS additions exercise transport only. Prior axis-public typecheck96138 completed exit0. New Rush typecheck35874 running; new public matrix still awaits live publisher62356. Optimizer process inspection shows active CPU work on private immutable input; no restart.


### Explicit RMF viewport matrix preparation

Surface UI harness now includes affine-progressive-sweep.r and requires original-source retained E presentation for both RMF fixtures, correct0.005/0.01mm source budgets, strict tolerance refusal and restoration at both widths. Matrix now has15sources (30width cases) but is not executed/qualified yet. Syntax and scoped diff checks passed. This avoids counting mere renderer success as new RMF proof propagation. Publisher62356 remains live and prevents stale package execution; no restart.


### RMF WASM published and public integration recheck

Publisher62356 completed exit0. WASM12245642→10890355bytes, SHA25611478a07952f59265dd481a6046fd4fe488f54cd396c6365fe209ad894c7eca4; source/public hashes match. Eight public suites initially108passed/3failed111total: two stale sampled-refinement expectations now correctly refuse by continuous retained-patch E (0.6979184386829022/0.005mm at5sections); third arc-length129station fixture exhausted inverse-work budget before testing correspondence refusal. Updated expected continuous refusal; correspondence-only arc fixture uses3stations while retaining explicit unproved E. Eight-suite recheck36403 live `/tmp/sweep-rmf-current-public-recheck-20261005.log`; Vite68522 live `/tmp/sweep-rmf-current-vite-20261005.log`. Public/package and30case browser qualification still pending; no success claimed from first failed matrix.

Eight public suites36403 completed exit0:111/111,49.23s on11478a07, including rational RMF scalar/affine/center/twist, six axis directions, curved/non-axis/arc refusal, actual Rush preview/viewport E and strict admission refusal. Vite68522 completed exit0. Headed30case surface UI50101 live (`ui-progressive-surfaces-rmf-current`, `/tmp/sweep-rmf-current-surface-ui-20261005.log`); both1440 RMF fixtures passed15checks each, no full-matrix claim yet. Open surface Solid refusal is expected and not evidence of filled-body completion.

Started independent STEP export26701 against11478a07 into `external-step-rmf-current`; existing41fixture matrix is regression scope, not yet direct new RMF external evidence. Export still live `/tmp/sweep-rmf-current-step-export-20261005.log`. Rational/analytic volume references and OCCT must follow export completion sequentially. UI50101 still live, both1440/600 RMF fixtures passed15checks each; no full30case claim. Planned direct RMF hollow linear-scale analytic-volume fixture after current export snapshot completes.

UI50101 completed exit0: passed=true,30cases/450checks at1440/600, SHA11478a07. `ui-progressive-surfaces-rmf-current/matrix.json`. Finite surface success/refusal, held-dispatch cancellation/source change/restoration evidence only; no actual mid-kernel cancellation or filled-body/global certificate claim.


### Current-package STEP regression and direct RMF expansion

Export26701, rational/canonical volume references and OCCT43596 completed exit0 on11478a07. OCCT passed41/41, authoritative `external-step-rmf-current/opencascade-sweep.json` with identical archived occt-report.json copy. Frenet retained volume0.03473701422615421mm3. This is preceding fixture regression scope; no direct new RMF case in these41. Extended linear-scale hollow fixture with rmf mode, original retained-E and separate global admission false, independent analytic volume560*pi/3mm3. Expanded42case export11465 is live in separate external-step-rmf-expanded-current, log `/tmp/sweep-rmf-expanded-step-export-20261005.log`. Direct RMF external result remains pending.


### Direct RMF STEP matrix qualified

Expanded export11465, rational/canonical volume references and OCCT21049 completed exit0. `external-step-rmf-expanded-current/opencascade-sweep.json` (identical archived occt-report.json) passed42/42 on11478a07. Direct rmf-straight-linear-scale-hollow.step independently agrees with analytic volume560*pi/3=586.4306286700947mm3, valid import and scoped full-domain wall/cap-coedge tolerance checks. Native separate volume inspection is solidGeometryCertified=true for this fixture, while constructor globalEmbeddingCertified remainsfalse; do not promote retained E to global admission. General curved RMF/arc/corrected closure and all-mode filled-body guarantees remain open. Contract matrix updated from pending to public111/UI30-450/STEP42 evidence.


### Original oblique rational RMF premise

Rust source criterion now covers every nonperiodic two-pole degree-one positive-weight rational line with distinct poles, not only coordinate axes: P(t)=P0+rho(t)(P1-P0), rho derivative positive, hence constant ideal tangent direction; ideal double reflections preserve perpendicular seed. Existing native interval endpoint displacement still charges actual floating frame/station arithmetic. Nonlinear degree-two near-line with min-positive transverse pole remains explicitly unresolved; no sample-derived straightness classification. Native independent oblique rational equation test passed1/1,0.17s, in both directions with translated path and one-cell-short refusal (`/tmp/sweep-rmf-oblique-native-recheck-20261005.log`). Initial compile failure came from concurrent distance field rename and intersection negative-test Debug bounds, repaired narrowly.

Focused module51906 live `/tmp/sweep-rmf-oblique-module-20261005.log`; sole next publisher41794 live `/tmp/sweep-rmf-oblique-wasm-20261005.log`; typecheck15535 live. Public oblique/degree-two refusal tests prepared, must await this package. Installed11478a07 public111/UI30-450/STEP42 qualifies preceding axis-only production, not this oblique extension. General curved RMF, holonomy/correction, arc correspondence and complete filled-body guarantees remain unresolved.

Focused oblique module51906 completed exit0:73/73,2.09s; typecheck15535 completed exit0. Added actual oblique-rmf-progressive-sweep.r with translated rational two-pole path, varying scale/twist and retained0.01mm budget. Included in public Rush preview/viewport/strict-refusal loop and headed surface matrix (now16sources/32width cases, not run yet). Extended independent native oblique oracle to variable scale and twist;19248 passed1/1,0.18s in both path directions and one-cell-short rejection (`/tmp/sweep-rmf-oblique-laws-native-20261005.log`). This later cfg(test)-only oracle update does not change live publisher input. Rush typecheck57036 and harness syntax/scoped diff checks passed. Publisher41794 release compilation finished1m16s; optimizer/pack process remains live. New oblique public/UI/STEP qualifications must await its final package.


### Oblique RMF independent STEP fixture prepared

Prepared43rd external fixture rmf-oblique-linear-scale-hollow.step with path[3,4,0], hollow rational circles and linear scale1→2. Requires native original retained-E and preserves constructor global embedding false. Rational-boundary Gauss reference uses uniquely oriented wall uses/outward cap contours, retaining canonical reference assertions for existing cases. Nominal analytic volume pi*(0.1²−0.05²)*5*7/3; rounded retained volume must be independently converged before OCCT. No export on old11478a07: this fixture requires live publisher41794 oblique branch. Typecheck13114/scoped diff checked; publisher remains live. Existing42case qualification unchanged and archived separately.


### Filled-cap provenance gap inspected

Current filled_cap_error::Premises is conditional composition (caller booleans/source bounds); JSON RPC explicitly preserves continuousBound=false. Miter already owns original source material proof in profile_domain_certificate::certify_local_profile_domain: exact plane predicates, nonsingular source-to-local projection and shared-budget contour simplicity/holes/nesting. certify_ideal_cap_domains adds original endpoint normals and positive affine law premise. Progressive Sweep currently lacks this constructor-owned source-region/endpoint-domain chain; retained wall E or STEP validity cannot replace it. Next cap work must reuse the original-region proof with actual progressive initial frame and both endpoint frames, then link retained cap identity/correction/decomposition and only finally compose boundary E. General moving-frame/arc modes still require independent correspondence proofs. Publisher41794 remains live; no competing package writer.


### Shared original material-domain proof connected to progressive constructor

Extracted original-profile exact-plane/projection/contour proof into crate-private shared certify_original_profile_projection_domain, with miter continuing to supply its own original certified initial axis and frame work. Progressive MultiSweep::certify_local_profile_domain now obtains actual original initial frame from authored/path-guide/Frenet/fixed seed Rust certificates and owns all source profiles. CorrectedFrenet remains explicit initial-frame-unproved because its initial normal can depend on later sampled curvature. Single shared cell/pair/exact budgets retained; no endpoint/cap/E/global promotion.

Native source-domain regression22464 passed4/4, including holes/one-cell-short/zero work/invalid loop layout/touching-hole refusal and both existing miter source/endpoint domain tests. Added constructor affine/Frenet/authored/guide and singular edge-on projection checks;97383 is running (`/tmp/sweep-progressive-source-domain-modes-20261005.log`). This production addition postdates immutable oblique publisher41794 and needs a later package/RPC path; no concurrent publisher launched. Full boundary E still requires both original endpoint domains and actual retained cap/correction/decomposition correspondence.

Expanded source-domain97383 executed4passed/1failed: test authored frame was created with Frenet orientation, correctly rejected by existing with_frame_laws constructor requiring Fixed. Corrected test configuration to Fixed for authored normal/edge-on variants;51366 is live in `/tmp/sweep-progressive-source-domain-modes-recheck-20261005.log`. Production proof unchanged by test fix. No expanded5/5 claim yet.

Oblique publisher41794 completed exit0:WASM12063508→10720668bytes, SHA256db4719371ae7af9bb5d4677abdcef9ade5da534dbf124d5569f98dde1cd3a2bc. Includes general two-pole oblique rational RMF criterion; does not include later shared progressive source-domain production/RPC. Eight-suite public47282 live `/tmp/sweep-rmf-oblique-public-20261005.log`, Vite3506 live `/tmp/sweep-rmf-oblique-vite-20261005.log`; expanded source-domain native51366 still live. New32case UI and43case STEP must follow package/public/build qualification.


### Oblique public regression: unavailable WASM clock

Expanded source-domain51366 passed5/5,0.02s, including affine/Frenet/authored/guide frames and singular source-to-local projection refusal. Oblique public47282 failed32/113: all32miter tests trap unreachable; remaining81tests across7suites passed (including new oblique RMF source/Rush). Package db471937 is not qualified for miter or full UI/STEP. Vite3506 passed but is not behavior evidence.

Current shared guards.rs unconditionally Instant::now during Budget::guard was identified as probable WASM trap: wasm32-unknown-unknown has no monotonic host clock. Rust repair creates no clock for iteration/depth-only budgets; unavailable finite wall-clock limits return explicit resource error via check/tick/enter_depth rather than silently disabling the promise. Native clockless tests45319 passed2/2; full core check started in `/tmp/sweep-clockless-full-native-20261005.log`. Previous native miter compile75608 failed8new concurrent offset/fillet negative-test Debug bounds; fixed those exact tests with err().expect. Sole next WASM publisher55729 live `/tmp/sweep-clockless-oblique-wasm-20261005.log`, includes clock repair and later source-domain native production. Do not rerun miter/UI/43STEP qualification against known failing db471937 while awaiting fixed package. Cause requires final WASM reproduction to be confirmed.

Clock repair full native71469 completed exit0:1142/1142,38.79s. Minimal direct ABI probe of the same original miter request reproduced unreachable on published db471937 (`/tmp/sweep-miter-clock-old-probe-20261005.log`) and succeeded on freshly compiled raw Rust/WASM (`/tmp/sweep-miter-clock-fixed-raw-probe-20261005.log`, accepted=true,2sections), confirming clock repair on actual WASM execution before optimization. Diagnostic script `/tmp/sweep-miter-wasm-clock-probe.mts` uses existing binary codec/ABI and performs no geometry in TS. Publisher55729 release completed1m16s and remains live for optimized packaging; direct raw1request is not full public/package qualification. Public113/UI32/STEP43 still pending the repaired packaged artifact.


### Progressive ideal endpoint material domains implemented (native pending)

Added MultiSweep::certify_ideal_endpoint_domains linking constructor-owned shared source-region proof with both original endpoint frame certificates and positive scale/affine law validation. Original 3D affine maps preserve holes even when source profile plane is oblique to initial frame. Report intentionally labels endpoint_frame_axes, not cap normals; no source-plane/frame-axis conflation. Closed paths, contact-width fitting, general curved RMF and CorrectedFrenet remain explicitly unproved here; partial endpoint frame arrays discarded.

Native test33430 live `/tmp/sweep-progressive-endpoint-domain-native-20261005.log`: oblique planar hollow source with affine axes/center, exact shared-work one-cell-short refusal, curved RMF refusal and singular end FixedNormal refusal. Scoped diff passed. This production addition postdates live clock-repair publisher55729 and requires a later package; no competing publisher. Still missing actual retained cap material correspondence/projection normal/correction/decomposition before full boundary E can be composed.

Endpoint domain33430 completed exit0:6/6,0.05s. Added Rust JSON operation surface_progressive_sweep_cap_domains reusing original constructor payload/configuration, with separate ideal/local domain scope, endpoint frame axes and explicit false continuous/retained-cap/global/Solid flags. Native transport79635 remains live `/tmp/sweep-progressive-endpoint-domain-transport-native-20261005.log`; source/cap/refusal tests now7expected, not qualified until terminal result. Prepared thin inspectProgressiveSweepIdealCapDomains adapter and3public scope/partial-budget/malformed-loops/touching-holes/curved-RMF tests; shared existing scale/twist/guide/frame request conversion factored once without geometry in TS. Adapter typecheck16911 passed, scoped diff passed. New RPC/post-publisher production must await a subsequent package after55729; do not run its public tests on missing RPC. Current repaired oblique packaging remains live.

Clock-repair publisher55729 completed exit0:12300886→10939070bytes, SHA2566f152072d061fd397c4291d263aa80aef3409a10ad91c8efa39c77a056783e2c. New113test public recheck17043 live `/tmp/sweep-clockless-oblique-public-20261005.log`; Vite64489 live `/tmp/sweep-clockless-oblique-vite-20261005.log`. This package predates endpoint-domain RPC and cannot run its3new tests. Source-domain transport79635 remains live; no replacement process launched.


### Repaired oblique RMF package qualified

Package6f152072 public17043 passed113/113,50.27s; Vite64489 passed. Headed surface UI31362 completed exit0,passed32cases/480checks at1440/600, including oblique rational RMF strict error refusal/restoration (`ui-progressive-surfaces-oblique-rmf-clockless-current/matrix.json`). Expanded STEP export62462, rational/canonical references and OCCT72500 completed exit0;43/43 passed in external-step-oblique-rmf-clockless-current/opencascade-sweep.json (archived identical occt-report.json). New oblique retained volume0.27488935718910684mm3 converged16/32/64; OCCT relative volume difference1.5058805649842001e-10. These finite checks do not prove general curved RMF/closure/arc/full body.

Next publisher52780 remains live for endpoint-domain RPC (native7/7); public3new RPC tests not yet run. Later source default-budget improvement uses no implicit clock limit on WASM, preserving explicit finite-time budget rejection, while native default stays60s. It postdates the likely compiled input of52780 and needs subsequent build verification; do not infer packaging inclusion. No competing publisher.


### Endpoint plane normals and raw endpoint RPC verification

Compile-only54850 completed exit0,release1m13s, including clockless WASM default guard changes; native budget96679 passed2/2. Direct raw ABI endpoint-domain request passed (`/tmp/sweep-endpoint-domains-raw-probe-20261005.log`): ideal/local domaintrue,15cells,3388exactWork; continuous/retained-cap/global/Solidfalse. Not a substitute for3public tests on optimized next package.

Added native certify_ideal_endpoint_planes: source exact normal retained by shared profile proof, transformed by R_end D^-1 R_initial^T with original zero-twist initial basis, positive anisotropic axes and original endpoint twist/frame enclosures. Positive uniform scale cancels in normalized direction; translation has no normal effect. Nonzero norm required, shared cell budget charged for every source/endpoint/axis evaluation, both normals discarded on partial proof. Output normals are actual source-plane normal directions, never substituted frame axes. Refactored original-frame selection for source/endpoint consistency. Native8case source-domain/plane regression77128 is live `/tmp/sweep-progressive-endpoint-plane-native-20261005.log`; new independent oblique plane z=x with axes2/3/4,twist0→0.25rad checks analytic unit normals and one-cell-short refusal. This later production change postdates live publisher52780; no concurrent package publisher. Full retained-cap/correction/decomposition/body E remains open.


### Endpoint domains public qualification and retained cap projection prerequisite

Endpoint-domain publisher52780 completed: packaged SHA256 aabd73ad6b9e2c9bb66b41db674c1b4fe47b2b1a8adcf959573e439470ad1a2c,10944634bytes. Public81387 passed9suites/116tests,50.29s (`/tmp/sweep-endpoint-domains-public-20261005.log`); Vite76224 passed. This package does not qualify later endpoint-plane/projection additions. UI32/480 and STEP43 remain tied to earlier6f152072 artifact.

Native inverse-transpose endpoint planes77128 passed8/8. Added Rust MultiSweep::certify_endpoint_cap_projection: original endpoint normals plus exact retained-cap plane and nonzero normal-dot/orientation certificates, one shared cell/exact-work budget; incomplete pairs discard both projections. Native62992 passed9/9; projection JSON transport75198 passed10/10 (`/tmp/sweep-progressive-cap-projection-transport-native-20261005.log`). JSON and TS adapter retain explicit false continuousBound, retainedCapRegionsCertified, globalEmbeddingCertified and solidCertified. Projection does not prove region ownership, endpoint pairing, contour error, correction or complete boundary E.

Added tests/nurbsProgressiveSweepCapProjection.test.ts: different retained/source regions with reflection, cell/exact-work one-short failures, singular/nonplanar refusal and malformed cap cardinality. Public execution pending new packaged RPC. Typecheck52643 passed. Sole publisher48148 remains live after release compilation (`/tmp/sweep-endpoint-planes-projection-wasm-20261005.log`); transport RPC was added during compilation, so its inclusion must be checked after publication rather than assumed. No competing publisher launched.

Public projection test typecheck58346 passed. Current compiled raw wasm inspected: new projection RPC absent. Live optimizer6916 remains running on its private immutable input, CPU6m10s at observation; publisher48148 was not restarted. A subsequent compile-only build started in /tmp/sweep-cap-projection-compile-only-20261005.log, with no packaging or public mutation. Prepared direct ABI projection probe for fresh raw output.

Fresh projection compile-only29920 completed exit0. Direct raw ABI projection probe completed exit0 (`/tmp/sweep-cap-projection-raw-probe-20261005.log`): five checks passed, including reflected orientation, shared cell/exact-work exhaustion, and nonplanar refusal; all continuous/material/global/Solid flags remain false. This verifies new Rust RPC in raw WASM, not the optimized public package. Publisher48148 still live on preceding immutable input; next package and public119-test matrix remain pending. Existing contour and trim-region audit code was inspected for subsequent retained material-ownership integration; it does not yet establish cap/source endpoint correspondence.


### Original endpoint-section displacement exposed for cap composition

Previous endpoint-plane publisher48148 completed exit0: geometry-kernel.wasm SHA25616946dfdbf1c4c1e31c894ca5efa8c9d861bb1824fc0aa2a2ef1598b8751ab31,10944563bytes. It lacks later projection RPC and has not been public/UI/STEP qualified. Sole next publisher39801 started /tmp/sweep-cap-projection-packaged-wasm-20261005.log using fresh projection raw input; remains live.

Rust PatchErrorReport and LevelReport now preserve existing certified original-section control displacement as original_section_endpoint_error_upper, serialized originalSectionEndpointErrorUpper. It bounds both end contours before decomposition/correction/filled-region construction, using the maximum original stored station control error. Multi-profile union requires all profile endpoint bounds; partial shared budget discards the aggregate. TS adds only an optional numeric metadata field. Native progressive recheck passed81/81,2.05s (/tmp/sweep-original-endpoint-report-native-recheck-20261005.log), including independent analytic endpoint scalar-scale/twist oracle, JSON propagation and partial-union refusal. This addition postdates39801 input and needs subsequent WASM qualification. No full filled-cap/body guarantee is promoted.


### Retained end-contour error composition implemented

Rust patch_error_from_section now derives retained_endpoint_error_upper by outward interval addition of certified original-section endpoint displacement and complete profile decomposition error. It is assigned only after decomposition coverage and matching retained station rational bases succeed. Level/MultiLevel serialize endpointContourErrorUpper for both endpoints; aggregate is null if any profile is unresolved. No correction, filled cap material ownership or body guarantee is claimed. TS only declares optional metadata.

Added native independent dense rational-profile endpoint geometry assertions, one-short decomposition product refusal, aggregate partial-budget refusal and JSON propagation checks. Native15083 remains live after successful compilation in /tmp/sweep-retained-endpoint-report-native-20261005.log; result not yet known. Typecheck51235 passed; scoped diff check passed. Sole projection publisher39801 still live; these later endpoint metadata/composition additions postdate its immutable input and require a subsequent package.

Retained endpoint native15083 completed exit0:81/81,2.04s. Typecheck51235 passed. Body construction inspected in brep-core/src/analytic/rational_loft.rs and geometry-bridge dispatch: geometry and cap construction already native; returned MultiApproximation carries the new endpoint metadata automatically after serialization. No filled-cap/source ownership audit is yet attached to returned model. Fresh compile-only build started /tmp/sweep-retained-endpoint-compile-only-20261005.log, without publishing over live39801.


### Native actual-cap ownership audit and projection package

Fresh endpoint compile-only53394 completed exit0,release1m15s; raw endpoint probe passed2checks, positive original/retained endpoint bounds and explicit nulls for curved RMF (/tmp/sweep-retained-endpoint-raw-probe-20261005.log).

Projection publisher39801 completed exit0: SHA25617abdf5616d1849ecc09e9cb33eb2d63d86602c6c46bf439fdd944c8aac7ebba,10953110bytes. Focused public cap projection/domain69350 passed6/6 (/tmp/sweep-cap-projection-public-focused-20261005.log). Full119/UI/STEP not rerun; later endpoint metadata is absent from this packaged input.

Added brep-core::sweep_retained_caps::inspect, a single native audit of both actual planar cap regions against segmented endpoint contours, exact native edge identity, native chart/trim audit, shared exact-work and edge limits. Promoted existing retained_wall_coefficients from transport-private to public nurbs-core module without duplicating implementation. Native actual-cap test passed1/1 (/tmp/sweep-retained-caps-native-qualified-20261005.log), proving both endpoint matches and refusal for mutated endpoint, one-unit exact budget and seven-edge limit. Initial compile failed private matcher visibility and a test index, repaired; initial one-short work expectation was invalid because the underlying adaptive audit can certify with fewer operations, replaced with deterministic inadequate budget assertion. This audit is not yet wired into source-owned progressive body reports or JSON transport; no full boundary/global/Solid claim.


### Native retained-cap transport and adapter migration

Added geometry-bridge operation brep_sweep_retained_caps_audit with actual Model, two endpoint ring sets and bounded native cap-audit budgets. Geometry-bridge native20172 passed1/1 (/tmp/sweep-retained-caps-bridge-native-20261005.log), including malformed endpoint cardinality, partial exact-work rejection and explicit false continuous/global/Solid scope. inspectSweepRetainedCaps TS implementation now only passes data to this native operation, preserving its existing result interface; removed duplicated TS loop/work proof composition. Typecheck52200 passed. New native hollow-region test verifies both caps, hole retention and rejection of omitted/swapped ring ownership; combined retained-caps/cap-contacts suite68206 passed (/tmp/sweep-retained-caps-hollow-native-20261005.log). Scoped diff passed.

Sole publisher61082 started /tmp/sweep-retained-caps-owner-wasm-20261005.log, includes retained endpoint error metadata and native cap ownership operation. Public retained-cap/miter consumers must be requalified against this next artifact; previous17abdf lacks native ownership RPC and is not compatible with the migrated adapter. No second publisher. Source-owned body boundary composition remains to be integrated; this transport-only audit does not certify original ideal domain correspondence or full boundary error.


### Constructor-owned actual caps and filled-cap error composition

Added native brep-core::analytic::progressive_profile_body_with_evidence. It constructs transported source sections/model and recomputes actual retained-cap ownership inside the same factory; old typed tuple APIs delegate to it. Geometry-bridge progressive body request now returns retainedCaps metadata bound to constructor-owned endpoint regions. Native25366 passed2/2,0.13s covering plain RMF, affine, authored+affine and guide+affine and forbidden authored+guide rejection. Initial test constants used zero-length primitive lines; corrected to the existing native constant-vector-law constructor. Typecheck27968 passed.

Further native factory addition reconstructs the same original MultiSweep law/frame/guide source, proves original ideal material domains and actual endpoint cap projection, and composes retained endpoint contour error with exact retained regions using filled_cap_error. No corrections occur in this builder; endpoint contour error already includes profile decomposition, and exact retained cap identity contributes zero further decomposition. Conservative nonparallel sqrt2 cap bound is used for both ends. JSON includes capProjection, filledCapErrorUpper and explicit boundaryContinuousBound:false/globalEmbeddingCertified:false; no complete retained wall union proof is claimed. Native91889 passed2/2,0.13s with filled-cap bounds in all four modes; typecheck76866 passed. Curved RMF null/refusal regression newly started in /tmp/sweep-source-owned-filled-caps-refusal-native-20261005.log. These changes postdate live publisher61082 and need a later WASM package and full public regression.


### Constructor-owned complete retained wall/cap boundary composition

Curved RMF filled-cap refusal regression93047 passed2/2,0.15s. Extracted existing rectangle/world-edge exact proof into pure typed nurbs-core::retained_wall_domain_certificate; transport delegates instead of duplicating geometry. Native51605 passed2/2,0.01s for complete trims and rational world identity. Added brep-core::sweep_retained_walls auditing actual coefficient family, full rectangular domains with shared exact work, and complete face ownership in body shells. Native97338 passed3/3 across retained caps (including holes) and walls; modified UV domains, omitted shell face, changed section and zero budget refuse certification.

ProgressiveBodyEvidence now links actual retained-wall union to the existing original-source continuous error and both filled-cap bounds. Returns boundaryErrorUpper, boundaryContinuousBound, and scope constructor-owned-retained-wall-and-cap-union; globalEmbeddingCertified remains false. Native76166 passed2/2,0.18s across plain, affine, authored+affine, guide+affine plus curved RMF null refusal. Typecheck8700 and public test typecheck3123 passed. Added5 public source-owned body boundary tests, not executed until their new WASM input is published. Separate boundaryErrorWithinBudget added later; native budget regression in /tmp/sweep-source-owned-boundary-budget-native-20261005.log. These local original-source/actual-body bounds do not prove general curved RMF, correction, all global embedding/orientation, all smoothness or all-mode/UI/STEP completion.

Owner-audit publisher61082 completed: SHA25682aef4c45635acca626bda1196476cc6c36162b57eaf32c6456613f2666d4c3f,10960035bytes. Six public retained-cap/decomposition/sweep/Rush regression suites39442 passed75 with1existingTODO,26.91s (/tmp/sweep-retained-caps-owner-public-regression-20261005.log). This package predates source-owned full body boundary API. Sole next publisher44883 live /tmp/sweep-source-owned-boundary-wasm-20261005.log; later within-budget field inclusion must be checked rather than inferred. No parallel publisher.

Boundary within-budget native72187 passed2/2,0.18s; typecheck42719 passed. Direct fresh raw WASM body probe passed (/tmp/sweep-source-owned-body-raw-probe-20261005.log): constructor-owned complete boundaryErrorUpper1.7852386235972537e-13, boundaryContinuousBoundtrue, globalEmbeddingCertifiedfalse. Raw input lacks latest boundaryErrorWithinBudget field; current44883 immutable packaging therefore does not qualify that field and requires a subsequent package. New public5 tests intentionally require both proven boundary and within-budget metadata and await that next artifact. No full all-mode completion claim.

Constructor-owned body boundary hollow regression23169 passed2/2,0.64s (/tmp/sweep-body-boundary-hollow-native-20261005.log). Added square-hole fixture requiring actual16 cap edges, all retained walls, complete boundary and within-budget result. Public body boundary matrix expanded from5 to6 cases; execution still waits latest packaged budget metadata. Fresh compile-only32384 is live /tmp/sweep-body-boundary-budget-compile-only-20261005.log, compatible with continuing optimizer44883 on immutable preceding input; no parallel publisher.


### Rush and viewport body-boundary propagation prepared

Fresh latest-budget compile-only32384 completed exit0,release1m11s. Native hollow full-boundary23169 passed2/2,0.64s. Progressive body construction metadata now forwarded by rushGraphNurbsKernel as sweepBodyBoundaryEvidence only for its exact source B-rep node. Added presentation-only readSweepBodyBoundaryViewportEvidence and current-source-gated viewport status; derived/source-edited nodes do not inherit proof metadata, and this reader does not admit Solid. Native upper bound, wall/cap prerequisites, scope and budget flags are only checked/presented in TS; all geometry composition remains Rust.

Added examples/rush/progressive-hollow-boundary.r and public actual-Rush/viewport/strict-budget refusal test. Public body matrix now7 cases including hollow, four mode combinations and curved-RMF refusal. Typecheck97110 passed; final public test typecheck63025 is pending at this save. New UI render/wide-narrow matrix and7 public tests not yet executed against packaged latest-budget input. Publisher44883 remains live for preceding full-boundary/no-budget artifact; latest raw API verification log /tmp/sweep-source-owned-body-budget-raw-probe-20261005.log. No competing publisher.

Latest raw WASM body-budget probe completed exit0: boundaryContinuousBoundtrue, boundaryErrorWithinBudgettrue, upper1.7852386235972537e-13, globalEmbeddingCertifiedfalse. Final public/Rush test typecheck63025 passed. Public7-case execution and visual UI qualification remain pending latest optimized package.

Publisher44883 completed exit0: optimized10967150bytes from12333072; contains complete native body bound but lacks latest within-budget field. Sole next publisher3529 live /tmp/sweep-body-boundary-budget-packaged-wasm-20261005.log. Extended existing isolated headed UI harness with --body-boundary using progressive-hollow-boundary.r at1440/600, full-boundary presentation, strict refusal/restore, held build/Solid cancellation, source replacement, successful native Solid/export/restore, invalid-path and strict-budget rejection preserving the previously admitted body. Syntax and diff checks passed. No browser execution yet; successful Solid remains an asserted requirement to test, not evidence.

Prepared independent STEP matrix expansion43→47: four dyadic hollow rectangular progressive body fixtures (plain/affine/authored+affine/guide+affine), require complete native wall/cap boundary and within-budget flags, and independent native Solid audit. Independently authored analytic volumes30mm3/180mm3; subsequent OCCT checks geometry/topology/cap holes/volume. Existing canonical and rational oracle gates unchanged. Execution pending latest-budget package; this is prepared coverage, not a47-case pass claim.

Application vue-tsc92951 passed. Important scope: tsconfig includes src TS/Vue, not test/export script typechecking; new public test and STEP exporter separately passed TypeScript transpile syntax diagnostics, not full semantic typechecking. UI harness node syntax passed. Source worktree remains heavily dirty with pre-existing untracked qualification files; no commit/push performed.

Actual Rust-backed Rush parser accepted progressive-hollow-boundary.r:11nodes,1progressive body (/tmp/sweep-body-boundary-rush-parse-20261005.log). This verifies source syntax/graph lowering only, not its pending latest-budget body/viewport/UI execution.

Independent new prism volume reference now derives exact Fraction area-minus-hole × affine area scale × path height from authored dimensions, and checks supplied expectedVolume before issuing numerator/denominator evidence. OCCT verifier requires matching positive exact reference metadata. Isolated independent oracle test passed30/180mm3 and rejected deliberately wrong31mm3; both Python verifier scripts passed bytecode syntax. New47-case STEP execution remains pending actual package; no STEP import was performed for these four cases. Publisher3529 re-polled live; no restart or extra publisher.


### Latest-budget body package and independent global prerequisites

Latest publisher3529 completed exit0: optimized10967369bytes, SHA2565bcf270c45e0b2e67478a7cb3553a2e7aaeb610f178e319064dba7f9de9d69c7. Full native4937 passed1149/1149,37.35s. B-rep regression86874 passed4/4 across contacts/caps/walls. Direct raw global audit passed all four new hollow prism modes (/tmp/sweep-body-boundary-global-raw-probe-20261005.log): original/actual boundary within budget, independently solidGeometryCertified/boundaryEmbeddingCertified/allFacesInjective/allPairsClassifiedtrue, nextPairnull. This is finite-fixture global evidence, not all-mode closure.

Public14-suite body/progressive/miter/viewport regression61614 started /tmp/sweep-body-boundary-public-matrix-20261005.log; Vite89218 live /tmp/sweep-body-boundary-vite-20261005.log. STEP47 export started /tmp/sweep-body-boundary-step-export-20261005.log into external-step-body-boundary-current; no OCCT pass yet. Do not mutate published kernel during these qualification runs. UI --body-boundary at1440/600 awaits finished matching dist.


### Completed constructor-owned body qualification, 2026-10-05

Verified current report artifacts: package SHA256 `5bcf270c45e0b2e67478a7cb3553a2e7aaeb610f178e319064dba7f9de9d69c7`, 10,967,369 bytes. Full nurbs-core 1149/1149; focused B-rep contacts/caps/walls 4/4; public 14 suites 152 passed and one existing TODO. Vite display build passed. These supersede the pending-run statements above for this package only.

Independent STEP report `external-step-body-boundary-current/opencascade-sweep.json` passed every one of 47 cases, with matching package provenance and manifest SHA256 `ddc09361ba8464a35d8269b4f7f4c32303b4bcc449429e0395a028a9674aca33`. Four added constructor-owned hollow prism modes (plain, affine, authored+affine, guide+affine) have one closed solid/shell, 18 faces, two cap-hole faces, preserved full-domain wall and cap coedges, opposite manifold edge uses and correct shell orientation. Independent exact authored prism volumes are 30/180 mm3; this finite matrix does not prove general embedding or all combinations.

Headed UI `ui-progressive-body-boundary-display-current/matrix.json` passed two cases at 1440/600 pixels, 28 assertions, matching source/public/dist/worker provenance. Includes successful Solid export, refusal preserving prior body, build/Solid cancellation with held dispatch, source replacement and restoration. Positive tiny boundary bounds now display scientific notation (approximately 1.785e-13 mm) instead of misleading zero. Held dispatch is lifecycle evidence, not mid-kernel interruption.

Remaining full-goal obligations: general curved/closure-corrected RMF and CorrectedFrenet original-frame correspondence, arc-length correspondence, corrections and general decomposed body/cap ownership, all-mode global intersections/nesting/orientation, applicable G1/G2/multispan/closed seams, and all-applicable STEP/UI coverage. Boundary E remains separate from global I and Solid admission. Geometry/proof algorithms remain Rust; TS carries ABI, metadata, presentation and fixtures. No commit, push, deployment or full CI claimed.


### Shared cap preparation budget correction, 2026-10-05

Rust `sweep_retained_caps::inspect` now charges copied segmented control rows against the same `max_exact_work` used for contour identity and cap audits, across both endpoints and all rings. Previously preparation independently allowed one million rows per source curve. A two-unit regression now proves only the first linear segment can be prepared before an explicit work-limit refusal; no partial exact cap certificate is returned. Native contacts/caps/walls regression passed4/4, exit0 (`/tmp/sweep-cap-shared-preparation-budget-20261005.log`); scoped diff whitespace check passed. This latest source correction is not yet in the previously qualified WASM package; public/UI/STEP47 evidence above belongs to its recorded SHA, not this unpublished source change. General unsegmented decomposition ownership remains open.


### Complete decomposition partition premise, 2026-10-05

New native `curve_decomposition_certificate::inspect_partition` checks cardinality and every ordered active source span with one shared interval-product budget; only a complete partition exposes the maximum parameterwise error. Unclamped rational regression covers three spans, exact27-product completion,26-product partial refusal, omitted span refusal and interior retained-control displacement. Focused certificate tests3/3 passed (`/tmp/sweep-complete-decomposition-partition-20261005.log`), scoped whitespace check passed. This is a curve correspondence premise only: it is not yet integrated into whole-body cap/wall ownership or packaged WASM; do not infer filled-region/global/Solid certification from it.


### Progressive partition certificate integration, 2026-10-05

Progressive authored/fixed/FixedNormal/Frenet/straight-RMF patch composition now uses the shared complete-partition certificate for every decomposed station instead of its local per-span aggregation. Existing section and endpoint error composition retains the same shared product budget, rejects partial coverage and mismatched station rational bases, and exposes no partial retained bound. Native progressive regression81/81 passed, command exit0 (`/tmp/sweep-partition-progressive-integration-20261005.log`); scoped diff whitespace check passed. Latest source remains unpublished in WASM.

Source inspection established a separate body obligation: surface `profile_parts` retains profiles with<=32 controls undecomposed, whereas B-rep `retained_bezier_pieces` extracts all Bezier spans. General body correspondence must add the latter extraction error and bind its actual coefficient family; a surface decomposition bound cannot silently substitute for this different extraction path. Existing unsupported body ownership remains refused.


### Constructor extraction error linked to actual body, 2026-10-05

Rust body construction now prepares the same Bezier sections as its B-rep constructor, certifies complete ordered original-to-retained partitions across every station with one shared million-product allowance, and requires constant retained rational bases for convex wall interpolation. Already segmented coefficient-identical curves contribute zero extraction error. Actual cap ownership and wall-family audits use these constructor-owned retained sections. Body extraction error is outward-added to retained wall E and composed with endpoint contour E in filled caps; possible preview decomposition double counting is conservative. Partial products or variable station bases leave body error unproved. No embedding/Solid promotion was added.

Native new small unsegmented degree2 rational-profile body passes actual wall/cap ownership and complete within-budget boundary E; preparation regression covers54 products for three stations,36 for two,35-product partial refusal, variable weights refusal and changed source poles. Initial test fixture used an invalid zero-length line for a constant law; fixed by constructing the law directly, no production behavior weakened. Focused new tests2/2, rational-loft regressions11/11, geometry-bridge transport2/2 passed terminal0. Application vue-tsc passed. JSON exports bodyDecompositionErrorUpper/bodyDecompositionProducts; TS only propagates metadata. Four public mode scenarios prepared (plain/affine/authored/guide), awaiting package. Publisher81703 launched once, current log `/tmp/sweep-body-decomposition-packaged-wasm-20261005.log`; no public/WASM pass claimed yet. General curved frames, corrections, nesting/intersections, smoothness and all-applicable UI/STEP remain open.


Publication concurrency observation: sweep publisher PID54357/session81703 is live; another externally started offset publisher PID55045 writes `/tmp/offset-contact-band-wasm-build.log`. `lsof` identified separate logs; current optimizer uses an immutable private input. Neither process was restarted or terminated. Public qualification must wait for both publishers to become terminal, then revalidate the actually published SHA/provenance. Do not attribute current shared package to this source change from build intent alone.


Publisher isolation correction: `lsof -d cwd` verifies offset publisher55045 runs in `/private/tmp/open-scad-viewer-push-2026-10-04`, whereas sweep publisher54357 runs in this repository. The earlier inference of shared package directories was incorrect; the two optimization/publication paths are isolated. Qualification only needs this sweep session81703 to finish and its own actual package SHA checked. No process was restarted or terminated.


### Full native extraction regression and expanded STEP preparation

Full nurbs-core1149/1149 passed terminal0,38.01s (`/tmp/sweep-body-decomposition-full-native-20261005.log`). Four new independent STEP scenarios now require native full boundary E and54 extraction products for small unsegmented rational profiles in plain/affine/authored/guide modes. The volume reference is separately converged rational-boundary divergence quadrature, not a claimed exact analytic prism volume; native Solid is audited but not required by these new fixtures. Expanded matrix target51 is prepared, not passed. Export script and public test passed TypeScript transpile syntax checks, not semantic test/script typechecking. Publisher81703 remains live in private optimization; no restart. Public/UI/STEP qualification still pending its actual package.


### Raw WASM unsegmented hollow body global audit, 2026-10-05

Direct ABI probe of compiled raw WASM SHA256 `676f6a6f124ef576409d252751ee2c032e7eca07170f7b8723147f8eb5e48797` passed four modes plain/affine/authored/guide with the small unsegmented rational outer edge plus a retained hole. Each reports54 decomposition products, positive extraction error (~9.50e-15 plain/~1.97e-14 others), complete body boundary E within budget, and independently solidGeometryCertified/boundaryEmbeddingCertified/allFacesInjective/allPairsClassifiedtrue,nextPairnull. Evidence `/tmp/sweep-body-decomposition-global-raw-probe-20261005.log`, process58115 exit0. Raw ABI evidence is separate from optimized-package qualification and finite-fixture global certificates are not all-mode proof.

Added authored Rush example `progressive-unsegmented-hollow-boundary.r`; public regression now also tests its snapshot-local viewport metadata and strict refusal. Headed body UI mode now prepares both sources at1440/600 (four cases), including successful Solid and lifecycle/refusal/restoration scenarios. Expanded STEP51 now requires native Solid for the four new unsegmented scenarios after raw ABI global evidence; actual optimized export/OCCT remains pending. UI harness node syntax passed. Publisher81703 remains live in private optimization; no restart.


### Packaged extraction kernel and qualification dispatch

Publisher81703 completed exit0, optimized12,337,814→10,970,819 bytes. Actual source/public/packed provenance verifies SHA256 `81e22eb732a9dad8bb76d6b729c33c4dd9db350203fa806e61143840f22b7ec0`. Vite build76369 passed exit0. Native partition regression18567 passed3/3, including an exchanged-span partition whose finite error bound is checked against independent source evaluation across every span; equal cardinality does not silently yield a near-zero correspondence error. New Rush source parser also passed11nodes/one body.

On this package: public eight-suite regression8260 live (`/tmp/sweep-body-decomposition-public-20261005.log`); expanded STEP51 export33389 live into `external-step-body-decomposition-current`; headed body UI96609 live with two sources at1440/600 into `ui-progressive-body-decomposition-current`. Only terminal results and actual reports count; independent volume/OCCT must follow the completed export sequentially. No kernel publisher remains active in this checkout.


### Packaged public extraction qualification and STEP export

Package81e22eb732a9dad8bb76d6b729c33c4dd9db350203fa806e61143840f22b7ec0 public8suites104passed+1existingTODO,50.57s, process8260 terminal0. This includes four small unsegmented modes and two actual Rust-backed Rush source/viewport/strict-refusal scenarios. Expanded STEP51 export33389 terminal0; manifest51 cases and new four nativeVolume.solidGeometryCertifiedtrue verified. Independent rational quadrature and authored generator/prism volume references terminal0. OCCT73151 live (`/tmp/sweep-body-decomposition-step-occt-20261005.log`); not yet a51/51 import pass.

UI first run96609 failed terminal1: CLI --headed was not interpreted; reportheadedfalse and requestDevice external Instance failure/no-gpu overlay blocked dock interaction. This is failed headless evidence, not a UI pass. Harness now recognizes --headed as well as existing SWEEP_UI_HEADED=1. Replacement79298 actually headed, output `ui-progressive-body-decomposition-headed-current`, currently live. No application behavior was changed to bypass the failure. Prior two-case28-assertion UI remains evidence of its own older package.


### Completed optimized extraction STEP and headed UI qualification

Actual optimized package SHA81e22eb732a9dad8bb76d6b729c33c4dd9db350203fa806e61143840f22b7ec0: independently verified OCCT report `external-step-body-decomposition-current/opencascade-sweep.json` passed51/51, process73151 terminal0. All four new small unsegmented rational plain/affine/authored/guide cases import as12faces/one solid, pass native Solid, retained surface/coedge/topology/orientation checks, with independent converged rational quadrature relative volume discrepancies about3.956e-11. This is numerical oracle agreement, not an interval volume certificate or all-mode theorem. Existing47 cases also pass.

Actual headed report `ui-progressive-body-decomposition-headed-current/matrix.json` passed4/4 cases and56 assertions, process79298 terminal0, headedtrue, package/source/public/dist/worker provenance matching. Both segmented hollow and unsegmented rational hollow sources run at1440/600: build, nonzero bound display, strict refusal/restore, material/fit, held build/Solid cancellation, source replacement, successful Solid export, project restoration and preserving previous body on invalid/resource refusal. Visually inspected600unsegmented preview: rendered profile and scientific bound1.920e-13mm visible; source editor horizontally scrolls at narrow width. Held dispatch remains lifecycle evidence, not actual mid-kernel interruption. Failed earlier headless run retained separately. No active sweep build/test/oracle/UI handles remain.

Combined current qualification: full native1149, rational-loft11, native bridge2, certificate3, public104+1TODO, STEP51, headed UI4/56. Complete original goal remains unproved: general curved/closed-corrected RMF/Frenet correspondence, arc-length, corrections, all-mode geometry/global/smoothness and all-applicable matrix coverage. No commit/push/CI/deployment claim.


### Original planar RMF analytic premise, 2026-10-05

Native Rust now recognizes an original nonperiodic rational path whose controls have an exactly constant coordinate and a finite nonzero seed normal parallel to that coordinate axis. Positive weights preserve that source plane; whenever the rational tangent is regular, the normal is orthogonal to T and has N'=0, hence is the ideal Bishop/RMF normal. This is original coefficient identity, not sampled planarity. Existing interval fixed-normal trajectory values/jets, original RMF initial coordinates, stored station displacement and decomposition bounds compose retained-patch E. Endpoint-domain/plane code uses the same premise for moving tangent/normal orientation. General spatial RMF, non-axial planar seeds and arc-length correspondence remain unresolved; no global or smoothness implication.

Native new tests3/3 passed terminal0 (`/tmp/sweep-planar-rmf-native-20261005.log`): independent analytic quadratic-path/scalar-scale transport; three translated coordinate planes with negative axial normal and nonuniform rational weights; even one subnormal out-of-plane control refuses the premise; non-axial normal and singular tangent refuse; original endpoint material domains/normals match curved tangent and partial-budget normals are discarded. Scoped whitespace check passed. Progressive-wide regression currently running; latest source is not packaged. Earlier optimized104/STEP51/UI56 evidence belongs to package81e22eb7 and cannot qualify this new RMF premise.


Planar RMF scope repair: first progressive regression83passed/1failed because the new premise also reached closed circle+full-turn twist and changed its acceptance under the existing section limit. Closed RMF adds seam/holonomy correction, which this correspondence does not yet qualify. The premise now explicitly requires an open path via native path_is_closed; general closed/corrected work remains open, preserving existing closed behavior. Re-run progressive84/84 passed terminal0,2.01s (`/tmp/sweep-planar-rmf-progressive-regression-20261005.log`). No tolerance/budget was relaxed and no proof inferred from stations. Open planar native E and ideal endpoint planes are now tested; packaged qualification remains pending.


### Planar RMF full native regression and public scenario preparation

Full native nurbs-core1152/1152 passed terminal0,38.03s (`/tmp/sweep-planar-rmf-full-native-20261005.log`). New public `nurbsPlanarRmfSweep.test.ts` prepares packaged-WASM acceptance, original subnormal off-plane refusal, actual Rust-backed Rush→viewport metadata and strict-budget refusal. Authored `planar-rmf-progressive-sweep.r` is added to headed surface UI matrix, now17sources/34wide+narrow cases. Independent STEP export adds a planar RMF quadratic hollow body with converged rational-boundary volume reference, target52cases; body global/Solid is independently audited rather than assumed from frame E. The fixture requires original retained-patch E, not a previously unproved complete cap/global claim. Scripts/tests passed transpile syntax, harness node syntax and scoped whitespace checks; semantic/runtime public/UI/STEP qualification pending package. Publisher84021 is live with private optimizer; no duplicate/restart in this checkout.


### Raw WASM planar RMF preview and filled body evidence

Fresh compiled raw WASM SHA25671ec71737d3721f813405fe763609b14d62068c3053e86cdd4ea9b74417daa77 direct ABI probe terminal0 (`/tmp/sweep-planar-rmf-raw-probe-20261005.log`): planar quadratic path with scale1→2 retained surface bound0.002446716932798053mm in0.1mm budget,623cells,33sections. A square YZ profile on the same curved path with constant scale has complete body boundary E0.0017547088952822756mm within0.01mm,64actual wall faces certified, both filled cap regions exact (8edges), original cap domains and nonzero projection dots certified. Native whole-body error remains separate from global embedding/Solid, not probed here. Added this curved full-body case to public planar-RMF tests. Actual Rust-backed Rush parser accepted planar source4nodes/orientationrmf. Optimized publisher84021 remains live; raw result does not replace public package qualification.


### Planar RMF curved body global prerequisite and expanded scenarios

Separate fresh raw ABI global audit for the quadratic-path square profile passed terminal0 (`/tmp/sweep-planar-rmf-global-raw-probe-20261005.log`): solidGeometryCertified/boundaryEmbeddingCertified/allFacesInjective/allPairsClassifiedtrue,nextPairnull. This strengthens one finite curved-body case, not general spatial/closed RMF global coverage. Authored `planar-rmf-body-boundary.r` parser passed7nodes/orientationrmf. Public Rush body metadata/strict refusal matrix adds this third source; headed body UI now targets6wide/narrow cases and includes Bezier-path degeneration refusals. Independent STEP adds quadratic rectangular profile requiring complete within-budget boundary E and native Solid, with independent rational-boundary volume oracle. STEP target53; surface UI17sources/34cases and body UI3sources/6cases prepared. Transpile syntax of exporter/newpublic tests, harness node syntax and scoped diff whitespace passed. Actual optimized runs still pending publisher84021; no prepared-count pass claim.


### Joint planar RMF law regression

Independent rational-generator joint-law native regression now executes both FixedNormal and planar RMF with simultaneous scalar scale, twist, affine axes and center displacement. Analytic point equation is compared against retained station interpolation over the whole path; partial-cell refusal remains checked. Focused regression1/1 passed terminal0,0.10s (`/tmp/sweep-planar-rmf-joint-laws-20261005.log`), scoped whitespace check passed. Only test coverage changed, so active production WASM input remains current; publisher84021 continues without restart. Added analogous public joint-law scenario, pending package. Full1152 native run above predates this test extension and is not re-labelled as a post-extension full run.


### Optimized planar RMF public, STEP and body UI qualification

Publisher84021 terminal0, package10,971,862bytes SHA2563442e7bf38d40144ae273e0c8a42f108dcb051b2c31e2edde017a19ecf49d3b4, source/public/packed verified. Vite82214 passed. First public9-suite run9141 had108passed/2failed/1TODO: new tests incorrectly expected a nonexistent surface global flag and omitted display request needed for native viewport artifacts. Tests corrected to actual API; five planar scenarios75302 passed5/5. Eight other suites passed105+1TODO on the same package. Combined coverage110passed+1TODO is not a single all-green rerun. No production contract relaxed.

Body UI first65384 had two passed wide cases then failed on locale decimal comma parsing; harness now accepts comma/point numeric display without changing application behavior. Corrected headed27868 report `ui-planar-rmf-body-locale-current/matrix.json` passed6cases/84assertions, matching package/source/public/dist/worker SHA, three sources at1440/600 including curved planar RMF successful Solid and full lifecycle/refusal/restoration. Visually inspected600curved Solid screenshot. Held dispatch cancellation does not prove mid-kernel interruption.

Expanded STEP53 export37143 terminal0. Initial OCCT was dispatched before all reference writers were guaranteed terminal and correctly refused missing rational metadata; no geometry pass inferred. Reference sequencing repaired and persisted metadata inspected before final verification. Final OCCT76739 terminal0; actual `external-step-planar-rmf-current/opencascade-sweep.json` passed every one of53cases with matching package provenance. Native Solid required on new quadratic rectangular body; quadratic hollow RMF import/topology/volume verified without assuming native global success. Independent numerical rational quadrature is not interval-certified volume. Complete original goal remains open. Surface UI42471 currently live, target34cases at1440/600; only completed cases count.


### Completed planar RMF surface UI qualification

Headed surface UI42471 terminal0. Actual `ui-planar-rmf-surfaces-current/matrix.json` passed34/34cases and506assertions at1440/600, SHA3442e7bf38d40144ae273e0c8a42f108dcb051b2c31e2edde017a19ecf49d3b4 matching source/public/dist/worker provenance. Includes new planar RMF open surface and existing straight/oblique RMF, authored/guided/contact/FixedNormal/Frenet and applicable closed surface cases. Surface-to-Solid refusals remain explicit; this is not a claim of all-mode successful Solid. Together with body6/84 and STEP53, this package completes its finite qualification. No live sweep process remains.

Next closed RMF diagnostic: on current packaged API, closed circle radius5, profile[5,0,-1]→[5,0,1], scale1 and twist0→360deg, existing FixedNormal original transport returns certified but too-loose retained bounds:9sections584.57895,33sections3.52290,129sections0.560891,257sections0.264491mm against0.01mm requested. This does not certify closed RMF correction and does not establish actual error exceeding budget. It establishes that current interval bounds need tightening before extending existing closed acceptance with this proof path. Adaptive/one-sided original knot enclosures and holonomy correspondence remain future work; do not relax budgets or infer a bound from samples.


### Native first-derivative defect bound at original knots

Closed circle knot diagnostic passed1/1: an interval beginning at an original knot has certified jets but single_span=false (13cells), while a strictly interior interval has single_span=true (7cells). Existing second-derivative remainder correctly remains unavailable at the knot. Added Rust conditional first-derivative defect bound: for retained chord L, integrate Q-prime minus L-prime from the nearer endpoint, giving max endpoint displacement plus half traversal width times the outward derivative-defect norm. The caller requires exact normalized original source domains and no strictly interior knot in path, scale, twist, affine, authored-frame or guide laws; existing original endpoint value certificates cover one-sided endpoint displacement. No inferred endpoint C2, budget relaxation or sampled proof.

Focused progressive sweep regression passed84/84 terminal0,2.01s (`/tmp/sweep-first-defect-progressive-corrected-20261005.log`). Two old tests assumed65sections must refuse the closed guided/contact circle at2mm; replaced this refinement-count assumption with actual certified-error admission and independent rational-circle comparisons, including the seam interval. Added independent polynomial/chord test with displaced endpoints and unresolved/invalid endpoint refusal. Full native process24123 currently running; packaged SHA3442 remains the earlier implementation and does not yet contain this production change. Closed RMF correction correspondence remains unproved.

Full native process24123 terminal0:1153/1153 passed,39.33s (`/tmp/sweep-first-defect-full-native-20261005.log`), including the independent polynomial/chord regression. Scoped production diff whitespace check passed. Optimized WASM/public/STEP/UI requalification of the new bound remains pending; previous package evidence is not relabelled.

Follow-up package process69361 compiled release successfully1m16s and remains live in optimization; other publisher89332 belongs to separate /private/tmp checkout, verified by lsof. Added closed FixedNormal/full-turn twist refinement regression33/65/129/257 with no closed RMF claim. Focused process67618 remains live; actual executable92512 is waiting in macOS dyld before test harness output (sample `/tmp/sweep-first-defect-native-process-sample.txt`, footprint112KiB, `_dyld_start`). No timeout restart or pass claim. Earlier1153full-native result predates this added diagnostic test.

Closed FixedNormal diagnostic67618 terminal0:1/1 passed0.33s after loader wait. Certified upper at33/65/129/257:3.52290294656165/0.5367793520710178/0.11986289248656923/0.04039630178235795mm, cells992/1888/3680/7264. New raw ABI probe `/tmp/sweep-first-defect-closed-raw-20261005.log` exactly matches native values and refuses0.01mm at all four levels. Bound tightened but tolerance remains unmet; no new closed RMF admission. Fresh raw planar surface/body probe terminal0 (`/tmp/sweep-first-defect-planar-raw-20261005.log`). Browser package69361 remains live with optimizer92842; no restart. Public closed guided/contact stream expectations updated for earlier65section acceptance established by native tests; added public FixedNormal full-turn regression requiring0.01mm refusal and upper<0.05mm, pending optimized package. Scoped whitespace check passed.

Additional first-defect premise regression: admissible continuous piecewise-linear scale with an interior slope knot has nonzero original/chord error despite zero piecewise second derivative. Native focused59773 passed1/1 (`/tmp/sweep-first-defect-kink-native-20261005.log`), checks original interior-knot gate, independent point error>0.3mm, certified coverage and strict0.01mm refusal. An initially disconnected-law fixture was correctly rejected by existing curve input multiplicity validation, not a production proof failure. Added analogous public kink regression pending package. Only tests changed after publisher69361 began; production input remains unchanged.

Fresh raw guided/contact matrix terminal0 (`/tmp/sweep-first-defect-guided-raw-20261005.log`): guided33/65/129 upper10.838117648/1.761843117/0.274988660mm,1922/3682/7202cells; contact33/65/129 upper21.856605078/2.552856421 (refused at2mm)/fine129accepted. Corrected pending public stream expectations separately: guided[false,true], contact[false,false,true]. Earlier blanket65section expectation for contact was incorrect and caught before optimized testing. Full native process76048 terminal0, see `/tmp/sweep-first-defect-full-native-final-20261005.log`; no updated optimized qualification claim.

Final native1155/1155 passed37.34s. Contact129raw upper0.31883662435687754mm,cells9854. Design matrix historical public-cell limitation updated against actual raw result; optimized package remains pending.

First-defect publisher69361 terminal0: optimized12,341,862→10,974,352bytes; public SHA2564b68b971002cfdebb488604bd03826414b11a2e4b81420342b6f8672e9b8a369. Vite59583 terminal0,968ms. Public9-suite22981 and STEP53export96685 now live, logs `/tmp/sweep-first-defect-public-20261005.log` and `/tmp/sweep-first-defect-step-export-20261005.log`. Headed body UI launched for six existing1440/600cases; no pending-run pass claims.

Optimized4b68 first-defect public22981 terminal0:9/9suites,100/100tests,52s (`/tmp/sweep-first-defect-public-20261005.log`). Includes new closed FixedNormal/kink and mode-specific guided/contact stream regressions. Headed body75993 terminal0:6cases/84assertions, source/public/dist/worker SHA4b68 matched (`ui-first-defect-body-current/matrix.json`),1440/600 successful Solid/refusal/cancel/source/restoration finite scenarios. Surface UI11725 live target34cases; STEP export96685 live. All-mode/mid-kernel cancel claims remain unproved.

First-defect STEP export96685 terminal0:53cases, manifest artifact SHA4b68/publicAndPackedVerifiedtrue. Rational reference35389 terminal0, all8references persisted and inspected. Generator reference terminal0. Started OCCT10697 only after both manifest writers completed; verification pending. Surface11725 continues34case1440/600 matrix. Numerical reference convergence remains distinct from interval-certified volume.

OCCT10697 terminal0, actual `external-step-first-defect-current/opencascade-sweep.json` passed53/53 with matching4b68 provenance and persisted reference manifest. This qualifies finite fixture import/topology/volume; does not prove all-mode embedding, seam/smoothness/containment or mid-kernel cancellation. Surface UI11725 remains live; current log includes narrow spatial-Frenet-affine case.

First-defect surface UI11725 terminal0:actual `ui-first-defect-surface-current/matrix.json` passed34cases/506assertions, matching4b68 source/public/dist/worker provenance. Together with native1155/public100/body6/84/STEP53, finite first-defect qualification is complete. All original broader scope remains open.

Next production step: conditional second-derivative h²/8 bound on an open original rational span. A bounded Q-second derivative and finite one-sided endpoint displacement yield the vector Green-kernel bound on the closed interval by endpoint limits; matching endpoint derivatives are unnecessary. Caller uses exact normalized source-domain/no-interior-knot premise and certified regular frame jets. Universal single_span remains unchanged; interior source knots retain prior image/first/second refusal behavior. Added native polynomial regression with single_span=false but independently C2 open polynomial. Progressive process27921 currently live; packaged4b68 does not contain this new second-bound change. No runtime/public/STEP/UI claim for it yet.

Open-span second-bound progressive27921 terminal0:86/86passed2.14s (`/tmp/sweep-open-span-second-progressive-20261005.log`). Closed diagnostic terminal0 (`/tmp/sweep-open-span-second-closed-native-20261005.log`). Full native48391 now live; prior4b68finite qualification remains attributed only to its first-defect implementation.

Open-span second full native48391 terminal0:1155/1155passed44.97s, predates diagnostic test expansion. Local diagnostic64869 terminal0:actual second-error enclosures near original circle knots are1e12–1e13 while smooth interior estimates are0.0003–0.0006mm; first-defect remains valid and governs the global0.0403963mm result.

Native original coordinate-plane identity added to FixedNormal jets: exactly axial seed and identical source coordinate across every original rational control imply constant projected normal, zero normal derivatives and tangent normal-axis derivatives. Positive rational source plane ownership, regular tangent certification and original twist application remain required; non-planar/non-axial seeds retain the general calculation. No sampled-plane inference or RMF closure claim. Five fixed-normal tests13333 passed5/5, including translated rational sources in all three planes with positive/negative seeds and singularity/budget refusal. Closed diagnostic37451 passed1/1:33/65/129/257 bounds0.4156179730/0.09789973308/0.03097763905/0.01195039524mm with unchanged992/1888/3680/7264cells. Still exceeds requested0.01mm; next needed original one-sided knot jets, not tolerance relaxation. Full native82453 currently live (`/tmp/sweep-planar-normal-identity-full-native-20261005.log`). Browser package4b68 predates second/open-span and constant-normal changes.

Constant-normal full native82453 terminal0:1156/1156passed36.19s, before thin-third change. Identified precise knot inflation: scalar third-derivative code differentiates ULP-restricted control polygons while scalar first/second code already differentiates full original spans for tiny overlaps. Added same original full-span-before-restriction strategy to third derivative, using interval de Casteljau to evaluate full-span derivative polynomials on the tiny local interval. Both one-sided knot limits stay in union; single_span remains false. Work counters/budgets unchanged.

Thin-third closed diagnostic19656 terminal0:33/65/129/257 upper0.13655419116224654/0.015075944907867967/0.002856971064310196/0.0006280154389876577mm, cells992/1888/3680/7264. At knot station64 second-bound0.0003874396928247875 and station1280.0005269565571456777 replace artificial1e12–1e13 inflation. Added independent piecewise-rational scalar third-limit regression (left6/right12, partial-budget refusal); process69631 live. Expanded closed diagnostic to independently evaluate original circle tangent/full-turn transport, all256intervals and copied seam, plus65refusal/129admission. Progressive32843 and full46753 live; no post-expansion pass claim yet. Prepared public closedFixedNormal acceptance/refusal expectations now match improved native diagnostic; old browser4b68still lacks these changes. Scope remains FixedNormal, not closed RMF correction/global proof.

Thin-third scalar69631 terminal0:1/1passed0.02s. Expanded progressive32843 terminal0:86/86passed3.68s, including independent full-turn geometry checks and coarse refusal/fine admission. Full46753 remains live. Scoped whitespace checks passed.

Thin-third full native46753 terminal0:1157/1157passed37.81s. New sole publisher21687 (`/tmp/sweep-thin-third-packaged-wasm-20261005.log`) compiled release1m12s, now optimization live. Added explicit Rush `closed-fixed-normal-full-turn-progressive-sweep.r` with constant scale/full360deg twist and0.01mm; prepared public129section success/65section refusal and UI source at1440/600. Surface matrix now18sources/36cases target (not pass claim). Numeric presentation parser accepts comma/point locale consistent with already qualified body harness; source budget checks retained. Script syntax/scoped whitespace passed. These JS/TS changes are fixture/adapters only; all geometry/interval bounds remain Rust.

Closed planar RMF correspondence diagnostic94784 terminal0:1/1passed0.08s,12sourcefamilies (threecoordinateplanes×0/17translation×negative/positiveaxialseed), each33/65/129sections. Actual RMF section control arrays agree exactly with FixedNormal (maximumdifference0), including copied seam/full-turn twist. Original closed RMF admission gate stays false; diagnostic sampling does not replace the source/correction proof.

Next integration premise: source coordinate-plane/axial-seed identity supplies constant Bishop normal and zero ideal holonomy. Actual constructor must expose/check its sampled tangent/chord plane identity, all transported normals equal initial and correction exactlyzero; otherwise return unresolved. Reuse constructor calculations, not a parallel reconstructed normal path. Only after this actual-ownership predicate can original FixedNormal trajectory certificates apply to closed planar RMF, with endpoint displacement/decomposition/body composition unchanged. General spatial/nonaxial/correctedRMF remains separate. Current publisher21687 still live, optimizer11310 activelyCPU-bound; new changes since launch are tests/examples/harness only, production input unchanged.

Constructor-owned closed planar RMF identity implemented in Rust after publisher21687 captured its immutable optimization snapshot. Existing sections API delegates to one internal sections_with_frame_identity; no parallel frame reconstruction. The real constructor checks original coordinate-plane/axial-seed coefficients, every sampled tangent has zero normal-axis component, positions stay in plane, all pre-twist transported normals exactlyequal signedunitseed, and computed correction==0. Source plane gives constant Bishop normal/zero ideal holonomy. Only matching actual closed sections may use original FixedNormal trajectory certificates; mismatch refuses `rmf-closed-correction-correspondence-unproved`. General source/arc-length/frame gates preserved; no automatic global/Solid admission.

Expanded12family closed RMF native diagnostic now requires real constructor identity and accepted129section continuous report at0.01mm. Progressive27639 live (`/tmp/sweep-closed-planar-rmf-identity-progressive-20261005.log`); compilation passed, tests pending. Publisher21687 still optimizes the preceding thin-third/fixednormal snapshot and does not include this closed-RMF production change. Its upcoming qualification must remain attributed to that snapshot, followed by a fresh publisher for current source. Scoped whitespace passed.

Closed planar RMF progressive27639 terminal0:87/87passed2.24s, including all12sourcefamilies exact section equality, constructor-owned correction identity and accepted0.01mm original-law bound. Full native launched (`/tmp/sweep-closed-planar-rmf-identity-full-native-20261005.log`), pending. Optimized/public proof still belongs to earlier snapshots until rebuilt.

Closed RMF full native78170 terminal0:1158/1158passed39.76s, predates joint-law test addition. Joint rational scale/affine/center/full-turn native12189 terminal0:1/1passed0.27s; independent original transport equation covers all128intervals/both controls, one-short work drops complete bound, nonaxialclosedseed refuses constructoridentity andoriginalerror. Prepared two dedicated closed-RMF public tests/Rush fixture; do not run them against prior snapshot.

Publisher21687 terminal0: thin-third pre-closed-RMF snapshot optimized12,344,993→10,977,278bytes, SHAe5c3331154b044f73f1f3cc2a7ec239b2a82df35f8c9bb8247a69a37e9df476a. Public9suite16394, STEP53export65122 and Vite49085 launched for this snapshot. New closed-RMF constructor proof remains Rust-source-only until next publisher. Explicitly preserve artifact/source distinction; do not attribute current Rust continuous RMF guarantees to e5c333.

Vite49085 terminal0; headed e5c333 surface UI38744 launched36case target (`/tmp/sweep-thin-third-surface-ui-20261005.log`). Native closed-RMF/Rush fixtures are prepared separately; current matrix adds only the e5c333-capable closedFixedNormalfullturn fixture. Public16394 and STEP65122 remain live.

Initial e5c333 public16394 terminal1:100passed/1failed across9suites. Single failure is obsolete closed-contact stream count expectation3 insteadactual2, caught by directpublishedWASM raw probe `/tmp/sweep-thin-third-guided-raw-20261005.log`: guided65upper0.8557239602699225/contact65upper1.2472117622682461mm (bothaccept2mm),33bothrefuse; fine129upper0.028811582162653903/0.034801720080770635. Updated contact expectation to actual[false,true] without budget relaxation. Focused corrected public19058 live (`/tmp/sweep-thin-third-guided-public-corrected-20261005.log`); no single all-green suite claim yet. STEP65122/UI38744 remain live.

Thin-third e5c333 qualification complete: corrected guided public19058 terminal0:10/10passed1.66s; original9suite100passed/1obsoleteexpectationfailure, then corrected focused10pass provides combined101casecoverage but not a single allgreen9suite rerun. STEP65122 export53terminal0, rational87536terminal0/all8persisted, generatorterminal0, OCCT51407terminal0 actualreport53/53matching e5c333 provenance. Headed surface38744 terminal0:36cases/538assertions including newclosedFixedNormalfullturn0.01mm at1440/600; body71920 terminal0:6cases/84assertions. All scope finite; cancellation remains held dispatch.

Added closed-RMF full-turn square periodic body native test covering actual retained walls, no caps, extraction E and completeboundary0.01mm. First test compilation correctly exposed wrong MultiApproximation.report test access (actual lastlevel owns report); corrected test-only access,19461live (`/tmp/sweep-closed-planar-rmf-body-native-corrected-20261005.log`). No runtime pass claim yet. Sole next publisher launched for current constructor-owned closed RMF identity (`/tmp/sweep-closed-planar-rmf-packaged-wasm-20261005.log`); old e5c333 qualifications not relabelled as closedRMFproof.

Body19461 terminal1 before construction: test used geometric line primitive for constant scale, correctly rejected zero-length geometry. Changed fixture to existing constant_vector_law (test only);90950live (`/tmp/sweep-closed-planar-rmf-body-native-law-corrected-20261005.log`). Publisher34073 current closed-RMF production snapshot live; no body runtime pass claim.

Closed periodic body90950 terminal0:1/1passed0.80s; complete owned-wall boundary0.01mm without caps/extraction E, on original test profile with tangential offsets. Replaced acceptance fixture with a genuinely transverse XZrectangle at circle start, to support subsequent external solid/topology/volume verification; rerun83899live (`/tmp/sweep-closed-planar-rmf-transverse-body-native-20261005.log`). Test-only geometric change, no production changes after publisher34073 snapshot.

Prepared independent STEP54target by adding `rmf-closed-planar-full-turn-rectangular.step`: source-owned continuous/withinbudget fullboundary required, all actual walls have unique shell ownership, capFaces/outwardCapContours empty, independent rational-boundary flux volume, native global/Solid remains separate (requireNativeSolidfalse). Do not export against prior e5c333 snapshot; pending closedRMFpublisher34073. Existing53 qualification remains historical, not54pass.

Transverse closed-RMF periodic body83899 terminal0:1/1passed0.82s, actual current XZrectangle fixture verifies complete accepted boundaryE≤0.01mm, all actual periodic walls certified, extraction0, no retained caps/projection/filledcaps, exact facecount4*(sections−1). This does not establish global embedding/native Solid. NewSTEP fixture remains prepared until currentpackage ready.

Current closedRMF raw ABI body probe terminal0:complete boundary0.008111513990519416mm,65sections/256walls, certified retainedwalls (`/tmp/sweep-closed-planar-rmf-body-raw-20261005.log`). Default global audit stops at10000pair budget,nextPair[42,194],allFacesInjectivetrue. Explicit maxPairs40000 audit66505 terminal0 returns nextPairnull (compact report omits visited count), but allPairsClassifiedfalse/boundaryEmbeddingfalse/Solidfalse; remaining obstruction is unresolved contacts, not just budget. All other default work/tolerances preserved. Prepared embedding pair diagnostic to locate concrete unresolved contacts. Do not advertise native Solid or change production default budgets.

Prepared public closedperiodic body regression requiring actualowned wall/fullE/no caps, surface UI now19sources/38case target includes closedRMFfullturn0.01mm. These await current publisher34073 (immutable current source snapshot). Solid-body UI for this source remains unqualified because global audit refuses.

Generic actual contact diagnostic84757 terminal0 (`/tmp/sweep-closed-planar-rmf-body-contacts-raw-20261005.log`):9638/32640visited,100000cells/311429domaincells;9459disjoint,88shared-boundary,91unresolved,0certifiedcontact witnesses. Unresolved pairs include[0,5],[0,7],[0,253],[0,255] and analogous diagonal corner neighbors/seam; inspect actual corner topology and shared-vertex exclusion proof next. Compact volume nextPairnull alone did not establish all-pair visitation/classification; preceding broad visitation statement corrected. Standalone sweepembedding cap[] refuses1..16caps; genericcad_face_contacts is the correct existing no-cap diagnostic path, not fabricated cap selection. Current closedRMF publisher34073 still live.

Closed RMF publisher34073 terminal0:12,345,977→10,978,190bytes,SHA3466eb5076f9d13f05be446e87e33b74a812eca93f4328be58f4eedfc249eca3. Public41753 terminal0:10/10suites,104/104tests,49.99s; includes closedjointlaws/Rushviewport/bodyfullE plusFixedNormalnewfixture. Vite14412 terminal0. STEP54export78795 live; surface UI84871 live38case target. No STEP54/UI38 pass claim yet.

Boundary-stage diagnostic17510 terminal0:closedbody agreementall_equal/all_joins_exacttrue,trimall_validtrue,256positive winding faces. Existing exacthull certificates already prove representative cornerpairs[0,5],[0,7],[0,253],[0,255]. Genericcad_face_contacts had no injected hull certificates, so its unresolved list is not the actual full-boundary refusal list. Added direct native boundary_embedding diagnostic with actual joint prerequisites/40000pairs to identify true unresolvedpairs;37020live (`/tmp/sweep-closed-planar-rmf-joint-boundary-diagnostic-20261005.log`). Do not change valid vertex certificate or infer globalSolid from genericdiagnostic.

Actual full boundary diagnostic37020 terminal0:all32640pairsvisited,520exacthullcontacts,36368cells,onlyfourunresolved[125,130],[126,129],[190,195],[191,194],nextPairNone. Distinguish these true remaining pairs from genericdiagnostic's91unresolved prefix. Added originalpolenet/perpair exactcertificate diagnostic72421live (`/tmp/sweep-closed-planar-rmf-four-pair-diagnostic-20261005.log`); do not infer sharedvertex certificate insufficiency until actual poles/stage results read.

ClosedRMF3466 surface UI84871 terminal0:actual matrix38cases/568assertions at1440/600, matching3466source/public/dist/worker provenance, includes closedRMFfullturn0.01mm success/strictrefusal/restoration with surfaceSolidrefused. STEP54export78795terminal0, rational68429terminal0/all9referencespersisted and inspected; generatorreference terminal0. Closedbody independent rationalvolume converges1.2525925942061618 across16/32/64rules. OCCT34944live; no54pass claim yet.

OCCT34944 terminal1:53/54passed. New closedRMF STEP imports as valid256face/1solid/1closed-shell model with manifoldoppositeedgeuses and positivevolume1.2525925942061518 (reference1.2525925942061618,relative7.98e−15), but full-domain sourceUV/control-net/surfacepreservation gates fail. Keep failure; inspect parameterization/export correspondence next instead of relaxing oracle.

Fourpair72421 diagnostic terminal0:actual poles at half/threequarter path knots show contact restricted to exact coordinate supporting plane. Existing projected2D vertex branch tries only four world-axis candidates; actual rotated corners need a different projected separator. Added existing boundedpropose_vertex_plane anchors to this2D branch for≤16poles perface, followed by unchanged exact projected orientation of every actual supportpole/endpointtopology. No tolerance change or blanket adjacent-face exemption. Native fullperiodicbody test now requires joint boundary proven,35030live; boundaryhull regression73141live. Production change is after3466snapshot; current packagedAPI does not include it.


Projected vertex follow-up: native35030 and94940 terminal101, existing hull contact regression73141/41722 terminal0 (4/4). Direct diagnostic58829 terminal101 visits all32640pairs,522hull certificates,34368cells, only[190,195]/[191,194] unresolved; the projected2D candidate closes[125,130]/[126,129]. Remaining two select a collapsed projection when only one AABB coordinate stays free. Source change now tries both possible complementary projection axes, each with unchanged exact orientation/owned-endpoint verification. Native59962 live (`/tmp/sweep-closed-planar-rmf-both-projections-native-20261005.log`); no fullboundary or Solid pass claim yet. This production edit remains absent from packaged3466.

Independent STEP failure diagnosed: existing Rust `canonical_face_senses` reverses U/control rows/knots/pcurve U for reversed shell face uses. New closed RMF has reversed wall uses; original oracle compared identical UV rather than that retained chart correspondence. Python oracle now maps U to u0+u1-U, reverses derivative U and control-row indexing, compares transformed original knots and exact weights, and maps original wall pcurve UV by the same source-owned orientation. TS exporter records unique shell wall-use orientation for every fixture (metadata only); no geometric algorithm moved to TS. Current existing54fixture manifest already carries the closed-case wall orientations. OCCT46211 terminal0:54/54 pass, source package3466; `/tmp/sweep-closed-planar-rmf-occt-chart-map-20261005.log`. Negative independent case with deliberately false wall orientation29288 terminal1, geometry/control/UV gates refuse (`/tmp/sweep-step-wrong-orientation-negative-20261005.log`). Tolerances and coefficients/weights/topology/volume gates unchanged. Exporter metadata addition still needs fresh export pipeline qualification; neither STEP evidence nor valid OCCT Solid grants native global Solid/all-mode completion.


Both-projection native59962 terminal0:1/1passed8.55s. Closed periodic joint boundary proven=true,524exacthull certificates, all32640pairsvisited,nextNone,32368cells,unresolved[]. Both[190,195]/[191,194] now close by exact projected source-pole signs and actual shared endpoint topology. This qualifies this256wall fixture with explicit40000pair work; default10000pair/UI admission and general all-mode guarantees remain separate. Expanded same native fixture to require `volume_validity::inspect_sweep` no-cap boundary/nesting/outward orientation, using100000orientation cells/1000000domains; new56272live (`/tmp/sweep-closed-planar-rmf-native-volume-20261005.log`). Final production hull regression17622live, full nurbs-core+brep-core lib13617live (`/tmp/sweep-projected-corners-full-native-20261005.log`). Sole new current-source WASM publisher96170live (`/tmp/sweep-projected-corners-packaged-wasm-20261005.log`); do not relabel3466 API or public/UI matrix as containing projected-corner production changes.


Closed-periodic native volume56272 terminal0:1/1passed14.86s, volumeproven=true,824orientationcells,outward=[Some(true)], jointboundary=true/524hullcontacts/all32640pairs/32368cells/unresolved[]. Actual no-cap body nesting/material orientation are now certified for this explicit40000pair finite fixture. Final hull regression17622 terminal0:4/4passed0.77s. Full13617 brep-core leg697passed/2ignored/0failed78.86s; nurbs-core leg still running, do not claim terminal combined pass yet. WASM96170 release compile finished1m09s, optimizer/publisher still live. Added public closedbody assertions: default10000pair audit must refuse with nextPair; explicit40000pairs must certify boundary/nesting/outward Solid. Prepared public regression is not yet run against new package. Native proof does not silently increase product defaults or qualify all-mode/UI/global promises.


Current production full native13617 terminal0: brep-core697passed/2ignored/0failed78.86s; nurbs-core1159passed/0failed37.63s (1856passed total; ignored not promoted). Added actual four newly proven corner ownership negatives to closedbody native fixture: one-ULP vertex displacement and geometry-identical independent edge/vertex topology must each refuse every projected corner. Focused29343 terminal0:1/1passed9.70s (`/tmp/sweep-closed-planar-rmf-corner-ownership-negative-20261005.log`); test-only additions after full-run snapshot, production unchanged. Publisher96170 remains live; authoritative wasm-opt PID46285 executing, release compilation terminal previously. No timeout-based restart.

Prepared new Rush example `examples/rush/closed-planar-rmf-body-boundary.r`: same originalcircle/XZrectangle/fullturn/.01mm body, constructor-owned completeE, default10000pair Solid audit intentionally remains refusal. Added it to public originalbody Rush→snapshot-localviewport parameterized cases and headed body-boundary UI matrix (now4sources/8cases target). Closedbody UI tests require build boundary presentation, strictE refusal/restoration, lifecycle cancel/source replacement, bounded-global-Solid refusal with preserved empty project, and invalid circle-radius refusal. Success at explicit40000pairs belongs to separate public native volume audit test. No prepared public/UI case claimed passing before new package is terminal. Existing UI cancellation remains held-dispatch; real mid-kernel interruption and all-mode completion still unqualified.


Publisher96170 terminal0: newWASM12,348,018→10,980,071bytes,SHA c5ed9131ddf1d98615df1b2fd53be3d87aabddbc376cb73c04309af9964db830, source/public identical. Initial current-package public98259 terminal0:12suites84/84passed19.24s; Vite94508terminal0. Fresh STEP24976terminal0:54cases, explicit closedcase nativeAuditPairs40000/requireNativeSolidtrue; every fixture now records nativeAuditBudgets and actual unique shell wall-use orientation. Rational74938terminal0/all9referencespersisted, generatorterminal0, OCCT38877terminal0:54/54passed; closedcase native_volume_certified=true/control_net=true/exact_shared_basis=true/full_domain_wall_pcurve_uv=true. Preserve distinction from earlier3466 STEP qualification where nativeclosedvolume was false. Actual evidence root `external-step-projected-corners-current` and `/tmp/sweep-projected-corners-occt-20261005.log`.

Initial current-package headed body92027 terminal1 after three wide successes: newclosedcase falsely expected refusal, but UI published Body1 B-rep. Source audit identified a real admission omission: `inspectProgressiveSweepSolidAdmission` guarded miter operations but skipped ordinary `brep_progressive_sweep`. Thus earlier ordinary progressive body UI successfulSolid results alone did NOT prove native global/material admission. Fixed thin TS adapter to invoke existing Rust volume audit for ordinary progressive sweep. Constructor closedPath selects only native cap scope; every actual selected cap/wall, shell role and orientation is freshly recomputed, no positive snapshot certificate admitted. Missing closure scope uses generic all-face audit. Initial generic-all-face test96244 terminal1:19pass/1failure (open unsegmented hollow needs native cap support proof). Correct cap-scope adapter80348 terminal0:20/20passed2.95s; actualthree open cases remain native-certified, closed256walls default10000pair refuses, forgedpositive flags and wrongcap closure cannot bypass native proof. No geometry algorithm implemented in TS.

Final changed-adapter public84085terminal0:13suites89/89passed21.02s, includes one-ULP/independent-topology Rust negative checks separately, actual closed native40000pair positive, default10000negative, source→viewport body cases, forgedsnapshot-positive/falsecap selection refusal, and B-rep Solid bridge tests. Vite40058terminal0 after actual admission fix. Headed updated body74635terminal0:8cases110assertions at1440/600, current c5ed source/public/dist/worker provenance verified; all three open sources publish native-audited Solid, closedsource refusesdefaultwork and retains/restores emptyproject; strictE/invalid/cancel/source-change/restoration covered. Prior failed92027 artifacts retained rather than relabelled. Surface75472terminal0:38cases568assertions on c5ed (dist before ordinary-adapter fix; surface sources contain no B-rep and that change is outside their admission path). Full updated miter UI49159 live (`/tmp/sweep-ordinary-admission-miter-ui-20261005.log`), evidence root `ui-ordinary-admission-miter-current`; no fullmiterpass claim yet. UI cancellation still held dispatch, not real mid-kernel latency. All-mode E/R/I/J, general spatial RMF holonomy, smoothness/moving frames and complete combinations remain open.


Final miter UI49159 terminal0: actual `ui-ordinary-admission-miter-current/matrix.json` passed=true,74cases/852assertions,1440/600, c5ed9131ddf1d98615df1b2fd53be3d87aabddbc376cb73c04309af9964db830. Existing per-source cancellation remains held worker dispatch. Additional real executing-WASM lifecycle probe61273 terminal0: `ui-native-cancel-warm-gated-current/matrix.json` passed=true,2cases/38assertions, same geometry SHA, six verified geometry ABI stacks (function index87) across native build cancellation, Solid cancellation and Solid source replacement at both widths, with worker destruction and restoration. QA helper `scripts/sweep-native-cancel-probe.mjs` verifies full geometry bytecode SHA and actual ABI activation before cancellation; debugger suspension proves destruction/no stale publication while Rust is active, not unconstrained cancellation latency. Selected real-activation source is closed-planar-rmf-body-boundary.r, not all modes. Earlier samplers76634/41375 terminal1 are retained: cold decoder/bootstrap sampling missed geometry activation; corrected probe waits unpaused for previously SHA-qualified geometry module before sampling, without restarting a live handle. Coordinator/body targeted regression32714 terminal0:2suites57/57; no new production cancellation code required. Logs `/tmp/sweep-native-cancel-warm-gated-20261005.log`, `/tmp/sweep-native-cancel-coordinator-regression-20261005.log`. Full all-mode continuousBound, general spatial/nonaxial RMF, source arc-length correspondence, holonomy/correction and applicable complete smoothness guarantees remain unresolved; this evidence does not complete the goal.


New Rust source progress after c5ed package: guided_section_interpolation_mode now supports normalized arc-length reference for Fixed and straight RMF on positive two-pole rational lines, without guides/authored frames. Exact length field is original P0+s(P1-P0); original inverse-length sections remain unchanged and actual endpoint displacement is certified against that field. Nonlinear twist/scale/affine source laws remain original; parameter-specific affine shortcut is disabled for arc-length. General curved/guided/authored arc-length remains unresolved. Added independent exact geometric image regression with unequal path weights, Fixed/RMF, variable scale/twist and work-budget refusal. Cargo handle9901/PID5669/testPID5798 compiled successfully but not terminal at record time; sample `/tmp/sweep-arc-line-test-startup-20261005.sample.txt` shows live `_dyld_start`, not executing test body. Do not restart due observation delay; no test pass or WASM/Rush/UI promotion claimed.

Arc-length test9901 terminal101: actual original inverse-length constructor refused WorkLimit10000 at tolerance1e-7,2/33stations, before new certificate. Retain this failure; not a proof pass. Expanded original test source with affine axes/center and independent expected twist/scale/offset geometry, plus curved Fixed/RMF unresolved negatives. New launch86070 uses existing production regression inverse-length budget tolerance1e-3/max_cells100000; certificate geometric budget0.1 unchanged. Production source unchanged since previous edit; no published WASM promotion.

Arc-length expanded native86070 terminal0:1/1passed30.33s, Fixed/RMF original rational path weights1/4, nonlinear joint affine axes/center/scale/twist, independent normalized-length geometry sampled over actual retained patches, budget exhaustion unresolved and curved source refusals. Startup sample confirmed `_dyld_start` before actual test then original handle progressed without restart. Source implementation not yet published in WASM; full progressive_sweep module regression newly launched, `/tmp/sweep-arc-line-module-regression-20261005.log`; no broad pass claimed. General spatial RMF and curved/guided/authored arc-length remain unresolved.

Full progressive_sweep module95958 terminal101:88passed/1failed43.58s. Only failure was old fixed_patch_certificate_keeps_arc_length_and_other_modes_unproved expecting no certificate on an original straight line. Updated test to assert new native positive line level, retain explicit curved arc-length unresolved and wrong-entry-point mode refusal. Full rerun26050 live, `/tmp/sweep-arc-line-module-regression-final-20261005.log`. Prepared public existing Rush arc-length test requires continuousBound=true; added asynchronous native→Rush→viewport Fixed/RMF evidence and strict continuous error refusal. Public tests not run until new WASM publication; current c5ed remains previous snapshot.

Final native progressive_sweep module26050 terminal0:89/89passed44.09s, including updated straight arc-length positive/curved negative/mode scope, existing parameter-mode regressions, new joint affine length-reference regression and original arc-length station test. Sole geometry WASM publisher newly launched with existing native wasm-opt driver, `/tmp/sweep-arc-line-packaged-wasm-20261005.log`; no package/public pass yet. Prepared native→Rush→viewport public tests await its terminal publication.

Sole new package publisher19891 confirmed live (nodePID11662/cargoPID11681), still release geometry-WASM build at last observation; do not restart on observation delay. Added Rush fixture `examples/rush/arc-length-affine-progressive-sweep.r` matching qualified native weight1/4/affine/center/twist setup. Prepared public async test now covers both original simple arc-length and joint fixture, Fixed/RMF, viewport bound/budget and strict error refusal. These are unrun preparation until publisher terminal; current package claims unchanged.

Arc-length package19891 terminal0:raw12348424→optimized10980444bytes,public WASM SHA cb7026267c1ec9877964ae5da90acd7a2efbe483dd7c77d8b348fb08463998dc. Native body19364 terminal101:fixture wrongly constructed constant laws as zero-length primitives; corrected to constant_vector_law, new31781 live (`/tmp/sweep-arc-line-body-native-corrected-20261005.log`), no nativebody pass yet. Public52591 terminal1:45/46passed, new viewport test lacked required display policy; corrected7214 terminal1:45/46passed, strict joint33-section case hit inverse-length WorkLimit51/65 before continuous refusal. Final public15273 terminal0:46/46passed,2suites,5.12s (`/tmp/sweep-arc-line-public-final-20261005.log`): simple/joint original arc-length Fixed/RMF native→WASM→Rush→snapshot-local viewport, strict3-section continuous error refusal, WASM actual-wall/filled-cap boundary positive for Fixed/RMF and existing regressions. Strict fixture now maxSections3 only to isolate continuous-error refusal; positive fixture remains33. Earlier failed artifacts retained. Actual new WASM source/package public proven; dist/browser UI/STEP not yet requalified for this SHA. General curved/guided/authored arc-length and spatial RMF/correction remain open.

Corrected arc-length native body31781 terminal0:1/1passed14.10s, Fixed/RMF actual retained walls/exact retained caps/filled-cap bound/complete boundary<=0.01mm. Vite24152 terminal0, newcb702 source/public/dist/worker package. Headed focused affine arc-length UI10445 terminal0:2cases30assertions at1440/600, `ui-arc-length-affine-current/matrix.json` passed=true, matchingcb702provenance, independent scoped finalerror/budget, strict3-section refusal/restoration, lifecycle held cancellations/sourcechange, surfaceSolidrefusal and invalidpath refusal. Full new surface UI64075 live, `/tmp/sweep-arc-line-surface-ui-20261005.log`,21sources/42cases target, includes simple and joint arc-length. Expanded current public77971 terminal0:10suites74/74passed5.24s (`/tmp/sweep-arc-line-full-public-20261005.log`). Scope remains finite qualification; current STEP/global-all-mode/spatialRMF/curvedarc-length not promoted.

New source-only Rust guided arc-length extension after cb702 package: original path and guide must each be positive two-pole rational lines; each reference field uses its own exact normalized-length P0+s(P1-P0), matching constructor's separate inverse-length tables. Retained original sections remain unchanged, endpoint displacement charges both residuals/frame projection. Contact and curved guides still unresolved. Added independent rotating normal expected[(1+s),s,0]/norm with pathweights1/2 vsguideweights1/4, actualpatch image, resource refusal and curvedguide negative. Native6129 live, `/tmp/sweep-guided-arc-line-native-20261005.log`; no pass/package/UI promotion. Existing UI64075 continues against immutable cb702 package, not these unpublished guided edits.

Full cb702 surface UI64075 terminal0:actual `ui-arc-length-surface-current/matrix.json` passed=true,42cases628assertions at1440/600, matching source/public/dist/worker fingerprint. New guided arc-length native6129 terminal0:1/1passed5.33s, independent rotating-normal expectedgeometry and actual retained patches plus budget/curved-guide negatives. Guided production edit remains absent from cb702 WASM; module regression90720 newly live (`/tmp/sweep-guided-arc-line-module-20261005.log`), no allmodule or publication claim. Broader curved arc-length/spatialRMF/holonomy/all-mode guarantees unresolved.

Guided arc-length fullmodule90720 terminal0:90/90passed53.33s. Sole current-source WASM publisher76738 live, `/tmp/sweep-guided-arc-line-packaged-wasm-20261005.log`; cb702 previous package remains only published result until terminal. Prepared `arc-length-guided-progressive-sweep.r` with independent path1/2 andguide1/4 weights and rotating frame, plus public native→Rush→viewport continuous budget0.05/strict3section refusal. Prepared UI source addition for guided arc-length (44cases target) and STEP fixture matrix expansion54→57: arc-rmf/arc-fixed/arc-guide hollow rectangular bodies with affine scaling, independent analytic volume180, original separate inverse-length paths and guides, mandatory native Solid. These prepared new public/UI/STEP cases not run before new publisher terminal; no completion claim.

Publisher76738 still live:node19702/native optimizer20623 CPU executing after release compilation1m14s; no restart. Existing independent OCCT interpreter `/tmp/sweep-structure-occt-venv/bin/python` verified importOCP; no new dependency install required. Full nurbs-core library newly launched (`/tmp/sweep-guided-arc-line-full-native-20261005.log`), separate from previously terminal90module. Updated design matrix with qualified cb702 arc-length scope and source-only guided scope; no prepared57STEP qualification claimed yet.

Guided arc-length publisher76738 terminal0:raw12348775→optimized10980780bytes,newpublic SHA a3ccea2dd684c58a2e9c51213bb80040d074cba601ed7ff6a38820036108cf88. Current public38426 terminal0:10suites75/75passed8.42s, includes original independently inverse-length guided Rush→viewport nativebound0.05 and strict3section refusal. Fullnative71879 live; new57STEP export58812 live (`/tmp/sweep-guided-arc-line-step-export-20261005.log`), no independent57pass yet. Vite newly launched `/tmp/sweep-guided-arc-line-vite-20261005.log`; UI remains previouscb702 until requalified. No all-mode/global completion claim.

New57STEP export58812 terminal0:manifest57cases on a3ccea source package. All three new arc-rmf/arc-fixed/arc-guide rectangular hollow affine fixtures nativeVolume.solidGeometryCertified=true, independent analytic expectedVolume180. Rational35618 terminal0/all9converged references persisted; generator reference terminal0. Independent OCCT newly live (`/tmp/sweep-guided-arc-line-occt-20261005.log`), root `external-step-guided-arc-length-current`; no57OCCT pass yet. Vite54714 terminal0; full44case surface UI53625 live on a3ccea source/public/dist/worker; native full71879 remains live/progressing. Preserve handle continuity; no timeout restart or broad completion claim.

Independent OCCT70163 terminal0:actual `external-step-guided-arc-length-current/opencascade-sweep.json` passed=true,57/57cases. All three new arc-rmf/arc-fixed/arc-guide fixtures native_volume_certified=true, relativeVolumeError0 against independent analytic180mm³; existing unchanged coefficient/weights/basis/UV/edge topology/material/volume gates remain active. Current a3ccea package STEP57 qualification established; not a continuous all-mode theorem. Fullnative71879 and headed UI53625 still live; no terminal pass claims.

Full a3ccea native71879 terminal0:1161/1161passed244.67s, serial library run. UI53625 remains live. New source-only authored arc-length patch reference extension: nonperiodic positive two-pole original path→exact uniform normalized-length image, original authored frame/scale/twist/affine laws retained; actual inverse-length sections charged by certified endpoint displacement and interpolation/decomposition. Added independent moving-normal N=(1,s,0)/sqrt(1+s²) geometry with weight1/4path, variable scale/axes/center and resource refusal. Focused47048 live (`/tmp/sweep-authored-arc-line-native-20261005.log`); no pass/publication claim, current UI/STEP a3ccea snapshot excludes this edit. General curved and spatialRMF/correction remain unresolved.

Full headed a3ccea surface UI53625 terminal0:actual matrix passed=true,44cases/658assertions at1440/600, source/public/dist/worker verified, including independent guided arc-length rotating frame/budget/strict refusal/restoration. Root `ui-guided-arc-length-surface-current`. Held-dispatch lifecycle scope remains distinct from previous actual native activation probes. Current package qualification remains finite, not all-mode completion.

Authored arc-length focused47048 terminal101:fixture violated with_frame_laws requirement (Fixed or new_authored), before geometric proof. Corrected test configuration to Fixed, focused54466 live; original handle not restarted while live. Source additionally extends authored_control_trajectory original line arc-length via same exact normalized-length reference; old API line-negative changed to certified positive, curved authored arc-length remains explicitly unresolved. New test source includes budget/curved negatives and API certificate. These additions postdate running54466 compiled test; full final module rerun required after terminal. Current a3ccea WASM unchanged, no authored arc-length publication claim.

Corrected authored arc-length focused54466 terminal0:1/1passed7.59s, independent moving-frame/affine geometry and actual patches. Final source includes authored control trajectory API line-positive and curved-path/resource negatives; module84279 newly live, `/tmp/sweep-authored-arc-line-module-20261005.log`. Prepared original Rush example arc-length-authored-progressive-sweep.r and native→Rush→viewport/strict3section public check. New authored source/API/public tests not yet published in WASM; a3ccea/STEP57/UI44 scope unchanged.

Final authored arc-length module84279 terminal0:91/91passed59.49s, includes direct authored_control_trajectory original-line positive, moving-frame/affine patchimage, curved arc-length refusal and budget exhaustion. Sole new WASM publisher35333 live, `/tmp/sweep-authored-arc-line-packaged-wasm-20261005.log`; no new package claim. Prepared STEP matrix57→58 adds arc-authored hollow affine prism/nativeSolid/independent180volume; prepared headed surface44→46 adds moving-frame authored Rush fixture/budget0.1/strict3station refusal. New package/public/UI/STEP tests pending.

Next general authored arc-length investigation: existing original curve_measure::DivisionReport carries total length bounds and per-station prefix/target residual bounds, not inverse parameter brackets. A normalized arc-length source has position Lipschitz constant totalLengthUpper, so residual bounds enclose positional endpoint discrepancy without assuming a parameter-to-length identity or speed lower bound. Candidate general proof should combine original authored relative pose law bounds with original length/position bounds and actual retained endpoint displacement; smooth curvature can later sharpen chord interpolation. This is a design direction only, not an implemented/qualified certificate. Preserve actual original curve and sections; never present a substituted curve as original continuous geometry.

Authored package35333 release compilation terminal1m11s; publisher still live in optimization, same handle observed. Full authored-source nurbs-core newly launched (`/tmp/sweep-authored-arc-line-full-native-20261005.log`) for changed direct trajectory API, not yet passed. General authored arc-length proof investigation confirms original DivisionReport per-point point_bounds/prefix_bounds/target_bounds/residual_upper and total.bounds. Candidate continuous position chord defect <=totalLengthUpper*h/2 plus actual endpoint errors is a source arc-length Lipschitz bound; relative authored-pose interpolation can compose separately. Requires outward interval arithmetic, original curve continuity, measured path budgets and actual station parameter identity before implementation/certification. No general curved claim yet.

Authored publisher35333 terminal0:raw12349129→optimized10981098bytes,public SHA33e4a30fce95a94bb10da26a9d696cef90a1f89c683db4b5ba59479a068709f1. Current public1157/export58STEP36030/Vite22505 newly live with `/tmp/sweep-authored-arc-line-public-20261005.log`, `/tmp/sweep-authored-arc-line-step-export-20261005.log`, `/tmp/sweep-authored-arc-line-vite-20261005.log`. Fullnative28196 live (`/tmp/sweep-authored-arc-line-full-native-20261005.log`). No new public/STEP/UI/fullnative pass claim yet; previous a3ccea qualification retains its snapshot scope.

Authored33e4 public1157 terminal0:10suites76/76passed10.32s. Vite22505 terminal0; headed46case UI47231 live. STEP58export36030 terminal0; rational80056 launched (`/tmp/sweep-authored-arc-line-rational-reference-20261005.log`), generator/OCCT subsequent. Fullnative28196 still qualifies compiled33e4 source snapshot, not new general edits.

New source-only general authored arc-length section certificate implemented after33e4: exact original knot multiplicity continuity prerequisite (reject periodic/closed/discontinuous paths), original length division point/target residuals and exact constructor parameter identity, actual retained basis/sections, original initial local coordinates and original authored relative pose interval jets. Position chord defect bounded outward by totalLengthUpper*h/2; actual endpoint originalpoint+relativepose displacement plus length residual added, and pose remainder composed before retained decomposition. Measurement work charged to shared certificate budget; partial paths/frames do not certify. Focused62698 live (`/tmp/sweep-authored-general-arc-native-20261005.log`), independent C0 right-angle length path, actual patches and resource-negative. No new general certificate pass/publication claim yet; current33e4 UI/STEP excludes general edits.

Published authored33e4 fullnative28196 terminal0:1162/1162passed259.62s, compiled before new general authored arc-length source; do not promote general edits by this run. Rational80056 terminal0/all9references persisted; generator terminal0, OCCT62556 live (`/tmp/sweep-authored-arc-line-occt-20261005.log`). Native general62698 still live; UI47231 still live.

Independent authored33e4 OCCT62556 terminal0:actual opencascade-sweep.json passed=true,58/58cases; new arc-authored native_volume_certified=true and independent180mm³ volume relativeerror0. Existing full geometry/topology/UV/basis gates unchanged. General-source62698 and headedUI47231 remain unqualified until terminal.

Published33e4 full headed surface UI47231 terminal0:actual matrix passed=true,46cases/688assertions at1440/600, source/public/dist/worker verified, authored moving-frame arc-length source included. Independent STEP58/native1162/public76 remain same33e4 snapshot, not new general edits. New general C0 source62698 terminal0:1/1passed0.03s, actualpatch independent right-angle arclength geometry and shared proof-budget exhaustion. Added smooth exact polynomial C(u)=(0,u²,u), independent analytic length primitive/inversion, original moving frame/twist/affine/center, focused46311 live (`/tmp/sweep-authored-curved-arc-native-20261005.log`); no smoothgeneral pass yet.

Smooth authored general arc-length focused46311 terminal0:1/1passed1.96s, independent original polynomial length primitive/inverse + moving-frame/twist/affine/center and partial-work refusal. Tightened source regression to production certificate budget10000 and explicit preview_at continuousBound/accepted assertion; full progressive_sweep module newly live (`/tmp/sweep-authored-general-arc-module-20261005.log`), these strengthened assertions unqualified until terminal. General source remains unpublished in33e4 WASM.

Prepared `examples/rush/arc-length-curved-authored-progressive-sweep.r` from independent polynomial/moving-frame/affine/twist source and native→Rush→viewport budget0.2/strict3station public regression. General source/public test awaits current module74490 and new WASM publication; source33e4 qualified public76/STEP58/UI46 cannot cover this unpublished mode. No general spatialRMF/holonomy/closed curved arc-length/global/smoothness completion claim.

General authored module74490 terminal101:92passed/1failed60.74s. Strengthened smooth source production-budget10000 +preview continuous/accepted positive passed. Only failure: discontinuous negative fixture was rejected at Curve constructor (interior multiplicity>degree), not by section report. Corrected assertion to constructor rejection and added valid closed-polyline source certificate unresolved. Rerun79568 live, `/tmp/sweep-authored-general-arc-module-corrected-20261005.log`; no correctedmodule pass yet. Prepared UI surface matrix includes original curved authored Rush (48cases target), budget0.2; current33e4 snapshot remains46qualified, newmode unrun. Production general source unchanged by test correction.

Corrected general authored module79568 terminal0:93/93passed64.18s, includes independent C0 and smooth length-inverse source geometry, production10000cell budget/preview positive, constructor discontinuity and validclosed-path certificate refusals. Sole new general-source WASM publisher newly live, `/tmp/sweep-authored-general-arc-packaged-wasm-20261005.log`; current published33e4 still excludes generalsource until terminal. Prepared Rush/viewport/UI48/newgeneral body/STEP qualification remain to be completed; full original goal unchanged.

General authored publisher77475 terminal0:raw12354129→optimized10985715bytes,publicSHA a66c2218a603967b495ee0c446640d323a1055e2cfe5e97cb0b2e214ce67cc81. Current public20186 terminal0:10suites77/77passed11.05s, includes original smooth curved authored arclength native→Rush→viewport budget0.2 and strict3station refusal. Vite45926 terminal0; new48case headed UI newly live (`/tmp/sweep-authored-general-arc-surface-ui-20261005.log`).

Full curved hollow body19392/diagnostic79401 terminal101 (15.83/14.95s). Exact cause: initial3sections full aggregate certificate6104cells/error0.37053772365765697 proves but exceedsbudget0.2; refined5sections reaches10000cells across8profiles, continuous_bound=false/errorNone/reasonarc-length-correspondence-unproved though sampledacceptedtrue. This is conservative refusal of completebody proof, not a nativeSolid positive. Common original path division is recomputed/charged perprofile; next repair must reuse one source-owned measurement proof in MultiSweep while charging it once and retaining aggregate10000 cells, without increasing defaults or promoting sampled acceptance. Prepared59STEP matrix new curvedbody must not be qualified before completebody proof closes. Previous33e4 STEP58/native1162/UI46 remain their own snapshot.

Rust MultiSweep shared original length proof implemented after a66c package: one local DivisionReport for identical original path pointer/spacing, charged once to aggregate10000 cells; profile pose/initial coordinates/decomposition still individually charged. Shared reference stays invocation-local and parameters must exactly match actual constructor; incomplete measurement/frame/union remains unproved. No budget increase or retained sampling promotion. Native curved hollow body33377 terminal0:1/1passed14.96s, original curved path+authored+affine, exact retained walls/caps/filledcap complete boundary<=0.2 and bothcap holes preserved. Added 64profile resource-negative to prevent partial union promotion; full progressive module newly live (`/tmp/sweep-shared-authored-general-arc-module-20261005.log`), no full pass/publication claim. Previous UI16780 continues against immutable a66c without shared-proof change. Prepared59STEP newcurvedbody awaits publication/nativeSolid/independent oracle rather than assuming nativeboundary implies global.

A66c headed surface UI16780 terminal0:actual matrix passed=true,48cases/718assertions at1440/600, source/public/dist/worker matched, smoothgeneral authored arclength included. Snapshot excludes new MultiSweep sharing. Prepared arc-length-curved-authored-body-boundary.r with two original rings/affine/static authored frame/curved path and public original-source completebound0.2 plus actual nativeSolid audit; not yet run on new shared-proof WASM.

### Shared authored arc-length regression and publication in progress

- Native progressive sweep module run completed: 94 passed, 0 failed, 179.68 s. Log: `/tmp/sweep-shared-authored-general-arc-module-20261005.log`. Includes the 64-profile partial-work refusal; aggregate exhaustion does not promote a continuous bound.
- Sole WASM publisher session 24511 is live; log `/tmp/sweep-shared-authored-general-arc-packaged-wasm-20261005.log`. No new package or public/STEP/UI qualification is claimed yet.
- Body UI matrix now includes the original curved authored arc-length hollow-body source, uses its authored max_deviation budget, and makes the entire quadratic path degenerate for the negative case. Script syntax checked; actual browser run pending publication.
- Curved STEP fixture omits straight-prism metadata and keeps independent rational-boundary volume reference plus mandatory actual native Solid admission. STEP59 remains pending.

### Shared authored arc-length WASM published

- Publisher 24511 terminated successfully. WASM raw 12,355,184 bytes -> optimized 10,986,698 bytes; public SHA256 `cb054835740ac01e393e6a9a2528dd8bbde9eceef1164c1e08c4065fc2118f44`.
- Public session 2506 passed 10 suites / 78 tests, 14.20 s. This includes the new original Rush curved authored arc-length hollow body, snapshot-local full boundary evidence at 0.2 mm, and actual native positive Solid geometry admission. Log `/tmp/sweep-shared-authored-general-arc-public-20261005.log`.
- Vite build 4389 terminated successfully. Full native run 12627 remains live, log `/tmp/sweep-shared-authored-general-arc-full-native-20261005.log`.
- STEP59 export session 52262 remains live; new root `external-step-shared-authored-arc-length-current`. No independent OCCT result yet.
- Focused body UI session 69187 remains live; wide 1440 px case completed 14 checks, narrow case pending. Root `ui-shared-authored-arc-body-current`. Partial log is not a completed matrix claim.

- Focused curved-body UI session 69187 subsequently terminated with exit 0: 2 cases, 28 assertions, 1440 and 600 px. Matrix confirms source/public/dist/worker SHA provenance for cb054835. Cancellation scope remains held-dispatch lifecycle, not unconstrained mid-kernel latency.

### Independent STEP59 qualification for shared authored arc-length package

- Rational-boundary reference session 13531 terminated exit 0; generator reference exit 0; OCCT session 13769 terminated exit 0. New package cb054835 matrix: 59/59 passed, public/packed provenance verified.
- New curved authored arc-length hollow-body case has actual native volume certification and native/external material agreement; 34 faces, one closed outward shell, 72 manifold edges with opposite uses, two cap-hole faces. Shared basis, control net, full-domain wall/pcurve/edge/cap coedge checks passed.
- OCCT volume 18 mm^3 vs independent converged rational-boundary flux 17.99999999999995 mm^3, relative difference 2.763221750178175e-15. Numerical convergence is an independent finite oracle, not a universal interval theorem.
- Evidence: `external-step-shared-authored-arc-length-current/opencascade-sweep.json`; logs `/tmp/sweep-shared-authored-arc-{rational-reference,generator-reference,occt}-20261005.log`.
- Full body UI matrix session 41064 live (10 cases intended); focused new body matrix is already terminal 2 cases / 28 assertions. Full native session 12627 remains live. These pending runs are not recorded as completed.

### Full native and body UI terminal evidence

- Full native session 12627 terminated exit 0: 1165 passed, 0 failed, 371.19 s. This run includes the current shared original path proof and 64-profile exhaustion negative.
- Body UI session 41064 terminated exit 0: 10 cases / 138 assertions across 1440 and 600 px; four positive Solid bodies and one closed-body default-budget refusal at each width. Artifact `ui-shared-authored-arc-body-matrix-current/matrix.json` covers constructor-owned full boundary presentation, strict refusal, lifecycle cancellation, source change and restoration on package cb054835.
- Surface UI requalification for this package is started separately at `ui-shared-authored-arc-surface-current`; no completion claimed yet. All-mode source frame/closed correction/global embedding/smoothness gaps listed above remain open.

### Curved Fixed arc-length proof under native qualification

- Surface UI for cb054835 session 18363 terminated exit 0: 48 cases / 718 assertions at 1440/600 px; immutable source/public/dist/worker provenance verified. This is the published authored/shared-length package, before the following Rust edits.
- New Rust relative Fixed pose certificate derives its constant original frame from the original initial tangent, including interval uncertainty; only translation is delegated to the original path length/position enclosure. The common arc-length section remainder now supports authored and plain Fixed for continuous open original paths. No RMF/guide/Frenet extension implied.
- Fixed original curved path test uses independent analytic inverse of C(u)=(0,u^2,u), joint scalar/affine/center/twist laws, and explicit budget exhaustion. Focused corrected native session 9981: 8/8 passed, 4.92 s. Initial session 89506: 6 passed / 1 outdated curve-refusal expectation failed; that negative now tests an unsupported closed original path. Failed log retained.
- Shared path measurement is extended to eligible plain Fixed multi-profile invocation, retaining per-profile work charging and source identity checks. Added original C0-corner Fixed test. Module session 17532 is live; BRep complete hollow-body test session 46181 is live and checks both Fixed and authored.
- These Rust edits are NOT in cb054835 WASM. No new WASM publication or public/STEP/UI qualification claimed for them yet.

### Curved Fixed native body and public fixtures prepared

- BRep session 46181 terminated exit 0: complete curved hollow-body proof passed for both plain Fixed and authored, 30.14 s. This proves original path/domain, aggregate wall plus filled caps boundary budget (0.2 mm) and retained cap holes; actual native global Solid admission still needs public qualification for the new Fixed package.
- Added original Rush Fixed surface/body sources; public tests check continuous source evidence and strict refusal, and body test requires actual Rust Solid geometry admission. Browser source matrices and independent STEP exporter now include this Fixed combination (STEP60 planned). These new checks are pending WASM publication.
- Native module session 17532 still running the specific 64-profile aggregate-exhaustion negative. Earlier Fixed C0 and independent smooth analytic inverse tests are passed in its log; no terminal module claim yet.

- Curved Fixed module session 17532 terminated exit 0: 96/96, 186.37 s. Includes C0 original path, independent smooth inverse with joint laws, and aggregate exhaustion. Sole new WASM publisher 72332 live, `/tmp/sweep-fixed-curved-arc-packaged-wasm-20261005.log`; full native regression 71988 live, `/tmp/sweep-fixed-curved-arc-full-native-20261005.log`. No new published SHA claimed yet.

### Fixed curved arc-length package published, public 80/80

- Publisher 72332 terminated exit 0: raw 12,358,312 -> optimized 10,989,404 bytes. Public SHA256 `7447cfdb19cb1247ac638e59a72f1e70a812ed131a41bc8ddb65be08e1fc2b22`.
- Public session 41632 terminated exit 0: 10 suites / 80 tests, 23.10 s. New Fixed original-source surface/body evidence and strict-budget refusal passed; body requires separate actual Rust Solid admission.
- Vite session 87025 exit 0. STEP60 export session 10036 live, new root `external-step-fixed-curved-arc-current`. Focused Fixed body UI session 42522 live, root `ui-fixed-curved-arc-body-current`. Full native session 71988 still live. These runs are not completed evidence yet.
- Contract matrix updated with package-scoped authored/Fixed curved position theorem and retained open requirements for curved normalized-length jets, moving frames/guides/contact, closed corrections, global embedding and smoothness.

- Focused Fixed body UI session 42522 terminated exit 0: 2 cases / 28 assertions, 1440 and 600 px. Source/public/dist/worker SHA7447cfdb verified. Covers actual Solid publication, strict/invalid refusal, source change, restoration and held-dispatch lifecycle cancellation. No unconstrained mid-kernel latency claim.
- Full surface UI requalification session 54050 live (50 cases intended) at `ui-fixed-curved-arc-surface-current`; no result yet. STEP60 export10036 and full native71988 remain live.

### Fixed curved arc-length full native and STEP60 terminal

- Full native session 71988 exit 0: 1167/1167, 381.75 s. Log `/tmp/sweep-fixed-curved-arc-full-native-20261005.log`.
- STEP export10036, rational reference19197, generator reference, and OCCT95368 all exit 0. Independent STEP60 matrix passed 60/60, package7447cfdb public/packed provenance verified. Evidence `external-step-fixed-curved-arc-current/opencascade-sweep.json`.
- Fixed curved hollow fixture: actual native volume certified and external material agreement, 34 faces, 72 manifold oppositely used edges, one outward closed shell, two cap-hole faces. Shared basis, control net, full-domain wall/edge/pcurve/cap-coedge gates passed. OCCT18 mm^3 vs converged independent boundary flux17.99999999999995 mm^3, relative2.763221750178175e-15. Same STEP geometry as static authored fixture is expected; separate original Fixed proof/source admission is tested natively and publicly. This numerical agreement is not an all-mode theorem.
- Surface UI54050 remains live, wide cases completed and narrow cases underway. Focused Fixed body UI2/28 remains terminal. Broad UI completion is not claimed yet.

### Fixed package surface UI complete; next source inverse enclosure

- Surface UI session54050 terminated exit0: 50 cases / 748 assertions,1440/600 px. Source/public/dist/worker identity7447cfdb verified in `ui-fixed-curved-arc-surface-current/matrix.json`. This completes the current finite surface matrix for the Fixed package; all-mode mathematical requirements remain open.
- New native-only source measurement field `LengthPoint.inverse_parameter_bounds` preserves the interval bisection bracket. A side is discarded only by disjoint certified prefix/target length intervals; work/precision/target uncertainty retains its enclosing bracket, without asserting existence/uniqueness. Normalized fraction0/1 endpoints select original domain endpoints exactly. No constructor station or accepted flag changed.
- Independent integration test adds rational-line exact inverse and curved analytic inverse on nonunit parameter domain, plus an exhausted-work negative. Session96777 live, `/tmp/sweep-arc-inverse-brackets-native-20261005.log`. These changes are not in the qualified7447cfdb package and do not yet certify FixedNormal/Frenet arc-length frames.

- Source inverse-bracket integration session96777 terminated exit0: 10/10,1.44 s, including nonunit-domain independent curved inverse and exhausted-work bracket. This is a native source-parameter foundation; no moving-frame arc-length certificate or new WASM qualification is implied.
- Full body UI matrix session76751 is now live (12 cases intended) at `ui-fixed-curved-arc-body-matrix-current`, log `/tmp/sweep-fixed-curved-arc-body-matrix-20261005.log`; it targets existing qualified7447cfdb package, independently of new native source-measurement edits.

### FixedNormal independent path/station value domains under native test

- Rust FixedNormal value certificate now has an internal relative-pose entrypoint with separate normalized original path parameter interval and normalized station-law interval. Tangent/projection follows original source parameter; scale/twist/affine follows original station laws. Translation is excluded explicitly and remains owned by original length bounds. Existing parameter-mode wrapper supplies the same interval to both domains, preserving its contract.
- Added independent analytic C(u)=(0,u^2,u) frame/scale/twist oracle with disjoint path [0.2,0.3] and station [0.6,0.7] intervals and one-cell-short refusal. Native session58949 live, log `/tmp/sweep-fixed-normal-separated-source-values-20261005.log`.
- This is not yet connected to complete FixedNormal arc-length section remainder/decomposition/caps, and is not in qualified7447cfdb WASM. Full original objective remains active.

### Fixed package broad body UI terminal; FixedNormal ArcLength remainder native candidate

- Body UI76751 terminated exit0:12cases/166assertions,1440/600 px, package7447cfdb source/public/dist/worker provenance verified in `ui-fixed-curved-arc-body-matrix-current/matrix.json`. Together with surface50/748, STEP60/60, public80/80 and fullnative1167, this qualifies the finite current Fixed package; original all-mode obligations stay open.
- Separated FixedNormal value-domain test58949 exit0:1/1. Original path parameter and station laws are independently enclosed.
- Native FixedNormal ArcLength retained-section candidate now uses original inverse-parameter brackets for frame values, independent station-law intervals for scalar/affine/center/twist, original actual sections/basis/rounding and original-length positional chord remainder. Relative interval image vs actual relative endpoint convex hull bounds the whole traversal without claiming inverse-length derivative jets. Closed/periodic original paths remain refused.
- Independent analytic curved frame/scale/affine/center/twist test97765 live, log `/tmp/sweep-fixed-normal-arc-retained-native-20261005.log`. Unqualified candidate; caps, shared multi-profile work, WASM/public/STEP/UI remain pending for FixedNormal ArcLength. Existing zero-budget refusal expectation retained; old categorical open-curved refusal no longer represents intended mode support.

### FixedNormal ArcLength original frame remainder positive native oracle

- Analytic retained proof session97765 terminated exit0:1/1,9.47s. The independent inverse of C(u)=(0,u^2,u) and original FixedNormal frame oracle encloses actual retained patches with scale/affine/center/twist, accepts the declared2mm budget at17sections, and refuses one-cell-short proof work. This is an interval position/pose image certificate, not normalized-length derivative or smoothness proof.
- Shared original length division extended to eligible open FixedNormal multi-profile invocation; original source identity/spacing and aggregate10000-cell budget remain required. Added64-profile incomplete-work negative for this mode.
- Native BRep test50577 live for Fixed/authored/FixedNormal complete curved hollow bodies, including walls, filled caps, source domain and holes; native module91009 live including new aggregate negative. Logs `/tmp/sweep-fixed-normal-arc-{body-native,module}-20261005.log`.
- Native candidate remains unpublished. Caps/global/WASM/public/STEP/UI for FixedNormal ArcLength are pending; original all-mode goal stays active.

- Prepared original Rush FixedNormal ArcLength surface and hollow-body sources. Small-profile body is intended for separate actual native Solid admission; large-profile `arc-length-curved-fixed-normal-folded-body-boundary.r` is retained as a future global-refusal fixture. Turning-frame large cross sections can fold even with a proven approximation error, so no Solid result is assumed for either source before native audit.
- Body50577 and module91009 handles re-polled live. Native body validation remains boundary-error/cap/domain evidence, not a global Solid claim. No new publisher was started while these regressions are pending.

### FixedNormal full-body refinement refusal and interval correlation repair

- Initial body50577 terminated exit101 after104.37s: `BREP_RATIONAL_SWEEP_REFUSED: Progressive body misses sampled refinement budget`. This occurred before caps qualification; no complete-body success claimed. Failed log retained. Native module91009 remains live on that earlier source snapshot.
- Declared2mm/max17 budget is unchanged. Rust FixedNormal values now use an exact coefficient premise when an original rational coordinate is constant and the seed is axial/perpendicular: N is the exact signed unit seed, B=T×N already unit. The source-tangent enclosure remains certified and source-derived. Otherwise the generic projection/normalization remains. Twisted B uses the exact orthonormal identity B*cos(theta)-N*sin(theta) instead of an unnecessarily widened T×rotated-N box. No actual constructor geometry or tolerance was changed.
- Added independent positive/negative seed and nonplanar-source fallback analytic oracle. Tight value regression90427 live, `/tmp/sweep-fixed-normal-tight-values-native-20261005.log`; unchanged-budget body rerun20005 live, `/tmp/sweep-fixed-normal-arc-body-tight-native-20261005.log`.
- No new WASM publication; all new full-body/global qualifications remain pending.

### FixedNormal exact caps and actual proof-work failures isolated

- Tight value90427 exit0:7/7,0.07s including sign and nonplanar fallback. Tight body20005 exit101:115.83s, now passes approximation budget but fails exact retained-cap assertion. Small native volume84272 exit101:6.45s at the same cap gate, before actual volume admission; no positive Solid claim.
- Earlier module91009 exit101:97pass/1fail,363.16s. Failure was the negative fixture assumption:64 simple FixedNormal profiles fit its proof budget. New negative keeps identical zero-twist geometry but authors16 twist spans, each charged under the existing aggregate budget, so required source-law work can genuinely exhaust it. Test68726 live, `/tmp/sweep-fixed-normal-multi-span-work-native-20261005.log`.
- Added explicit Rust `EndpointCapCorrection` and `progressive_profile_body_with_evidence_and_correction`, preserving existing API's uncorrected behavior. Uses existing Rust section_projection exact dyadic plane predicate/displacement certificate, a shared work budget across both caps, actual corrected endpoint sections/walls, and adds correction displacement to wall plus filled-cap error composition. No tolerance or budget increase, and no inferred global admission.
- Small corrected-body actual native volume test8393 live, `/tmp/sweep-fixed-normal-small-body-corrected-volume-native-20261005.log`. New correction API and all FixedNormal ArcLength changes remain native/unpublished; WASM/Rush/public/STEP/UI integration still pending.

### Corrected FixedNormal complete body and actual native Solid positive

- Small corrected body8393 exit0:1/1,6.40s. Asserts original continuous bound, exact corrected retained caps with holes, filled-cap/full-boundary within2mm and separate actual native volume/boundary/nesting/orientation admission. This is a finite native small-profile fixture, not all-mode Solid proof.
- Genuine16-span zero-twist aggregate-work negative68726 exit0:1/1,131.65s. Shared partial work does not promote a positive continuous bound. Earlier64-profile simple fixture fit its budget and was not a production defect.
- Native geometry bridge supports explicit cap_correction_tolerance/quantum/max_work with missing-tolerance refusal and serializes capCorrectionErrorUpper. Rust bridge44785 cargo check exit0. TS body types/schema/Rush adapter only carry parameters/metadata; vue-tsc21275 exit0. Geometry/correction/error composition remain Rust.
- Rush source preflight with old language package refused new named correction fields; added explicit Rust rush-frontend lowering and dimensional source regression. Native Rush parser test is running; language WASM must be republished alongside geometry before public qualification. No success claimed from old package.
- Corrected large-body modes77655 and current native sweep module5445 remain live; new tightened assertion on nonzero bounded correction needs the next native run. None of these changes are in qualified7447cfdb package.

### Corrected FixedNormal native regressions pass; publishers active

- Corrected large body modes77655 exit0:1/1,107.13s, Fixed/authored/FixedNormal walls plus filled caps within unchanged budgets. Rust parser25903 exit0:1/1, explicit dimensioned correction parameters lower correctly.
- Current sweep module5445 exit0:98/98,320.42s. Native correction refusal82795 exit0:1/1,9.12s; exhausted correction work and excess displacement return an error, not geometry with a positive proof.
- Language publisher89247 exit0:raw2463836 ->2279628bytes. New geometry publisher18989 live at `/tmp/sweep-fixed-normal-corrected-arc-geometry-wasm-20261005.log`. Full native43894 live at `/tmp/sweep-fixed-normal-corrected-arc-full-native-20261005.log`.
- Public tests prepared for corrected small body, independent original-source surface and folded large body global-refusal separation. No geometry/public/STEP/UI success claimed until new geometry publisher and actual tests terminate.

### Fused Rush strict-schema gap found during publication

- First new language publisher89247 produced SHA407c78a4d11218dcc912e25bd241af706d4697d40b57a74f246595673f22c1d6. Public source preflight now reaches strict Rust runtime validation but rejects `cap_correction_max_work`: lowering alone was insufficient. This language artifact is NOT qualified for the new body parameters.
- Regenerated checked-in `docs/languages/rush-nurbs-1.schema.json` from updated declaration schema using repository export script. Rust runtime includes this exact file. Added fused language-bridge test spanning source parse/lowering plus strict runtime validation for both corrected sources; native12704 live, `/tmp/sweep-corrected-body-rush-fused-native-20261005.log`. Language must be republished after it passes.
- Geometry publisher18989 remains live (release compile complete, optimizer active), fullnative43894 live. No new public/STEP/UI success claim until both final language and geometry packages pass actual source requests.

### Final corrected FixedNormal packages and qualification dispatch

- Fused Rust language test12704 exit0:1/1,0.01s; both original corrected Rush sources pass text lowering and strict runtime schema. Final language publisher92538 exit0, SHA fdf9132ab95cdbee3467b7f3e2f16a95d97ad4de626b878b0669307268ffccab. Actual TS entrypoint `compileRushFrontend` accepts both sources and preserves fixed_normal, tolerance1e-9, quantum2^-40, maxWork1000000.
- Geometry publisher18989 exit0, raw12371252 ->11000905bytes, SHA941ee54fe7f4e5bcb8d4071568969f5356fc55def5f5275872e4a806321d4338. Full current native nurbs43894 exit0:1171/1171,512.56s. Vite80084 completed successfully (build log891ms); no new public/STEP/UI success inferred from these results.
- Public10-suite dispatch37058 live at `/tmp/sweep-fixed-normal-corrected-public-20261005.log`. STEP export40510 live at `/tmp/sweep-fixed-normal-corrected-step-export-20261005.log`, root `external-step-fixed-normal-corrected-arc-current`; new positive small curved FixedNormal hollow case includes explicit native correction metadata and requires actual native Solid plus independent rational-boundary volume oracle.
- UI matrix now includes original FixedNormal surface/body sources, unchanged E2 budget for this mode and genuinely degenerate-source refusal. Focused headed body30464 live at `/tmp/sweep-fixed-normal-corrected-ui-body-20261005.log`, root `ui-fixed-normal-corrected-body-current`. Held-dispatch cancellation remains limited lifecycle evidence. Folded large negative source remains a public global-admission test, not an assumed positive STEP fixture.
- `node --check` UI script and focused `git diff --check` passed. Full six-part goal remains active; finite matrices do not prove universal geometry or smoothness guarantees.

### Actual public/UI failure: corrected body viewport triangulation

- Public37058 exit1:82 passed,1 failed/83 across10 suites. Original corrected small FixedNormal body source fails at `brep_tessellate(4)` with `Profile cannot be triangulated without crossing its boundary`; body-bound metadata test does not reach assertion. Do not claim this source fully integrated. Other tests, including folded large global Solid refusal, passed.
- Focused UI30464 exit1 at initial build assertion for the same original small source. No successful UI lifecycle qualification on this source yet.
- STEP exporter40510 exit0 (native body retained geometry construction succeeds without viewport triangulation). Independent rational reference69276 exit0, generator reference exit0. OCCT18245 live at `/tmp/sweep-fixed-normal-corrected-step-occt-20261005.log`; no external acceptance claimed before terminal/result inspection.
- Added Rust geometry-bridge regression `corrected_fixed_normal_hollow_body_tessellates_original_boundary`: reproduces native corrected small constructor then demands closed tessellation at details1/2/4/8. Initial test compilation failed due wrong Curve namespace and nonexistent evidence field; corrected to core::curve::Curve and boundary_error_within_budget. Current82397 live at `/tmp/sweep-fixed-normal-native-tessellation-diagnosis-20261005.log`. Test-only triangulation boundary diagnostics added to identify failing UV rings. No production triangulation fix yet; unchanged geometry budgets.

### Independent STEP61 passed; native viewport reproduction fails

- OCCT18245 exit0, `external-step-fixed-normal-corrected-arc-current/opencascade-sweep.json`:61cases,passedtrue. New corrected FixedNormal hollow case:18faces/40edges,1closed outward shell,2hole caps,native volume certified/material agreement true, original full-domain wall distance/pcurve/control basis and16cap coedge uses preserved. OCCT volume0.05523271584915873 vs independent Gaussian0.0552327158491587, relative5.025205657355283e-16. Finite qualification only, not universal proof.
- Native reproduction82397 exit101,1failed in4.88s. Details1/2 succeed then detail4 fails in planar ear clipping. Captured cap UV rectangle x[0.7316718427000524,1],y[0,0.1999999999998181] with sampled hollow rectangle. Corresponding sample x coordinates differ by~2.27e-13 (e.g.0.9329179606750131 vs0.9329179606747857), while clip_ears uses fixed1e-12 epsilon for ear and point containment. This is a concrete candidate cause, not yet a verified fix. Need preserve authored boundary coordinates and replace unreliable geometric sign classification with bounded robust predicate; no tolerance/budget relaxation.

### Rust ear clipping correction in progress

- `planar-geometry` now depends on existing Rust cad-predicates. Ear convexity, point containment and final triangle sign use bounded exact orientation over current binary64 display UV boundary bits. This proves mesh decisions only, not original source geometry; coordinates remain unchanged. Predicate work is charged to original8M profile budget; indeterminate signs refuse.
- Added captured actual cap UV regression `corrected_cap_samples_keep_near_aligned_hole_boundary`, checking authored boundary conformity and material area. Added native geometry-bridge corrected small-body tessellation regression details1/2/4/8 and closed mesh. Test-only failure diagnostics remain.
- Initial20977/33580 failed because predicate limit8M exceeds predicate hard limit1M. Fixed by per-operation context capped at min(remaining profile work, predicateMAX), cumulative actualwork remains charged. Obsolete captured88291 also compiled this initial wrong-limit snapshot (failure, not current evidence).
- Corrected bounded native73946 exit0:1/1,4.97s, details1/2/4/8 all closed. Bounded triangulation89938 exited101:9passed/1failed (10x10holes/32samples exhaust8M), including captured cap pass. No publication yet: existing workload compatibility still needs closure.
- Added exact equal-coordinate/duplicate source-bit collinearity fast path, charged before return; it avoids expansion work for sampled straight edges without epsilon or displacement. Latest full profile89280 live at `/tmp/sweep-exact-ear-axis-filter-regressions-20261005.log`. Pending result determines further optimization. Native73946 compiled before this added exact zero fast path; new snapshot must receive native/package/public/UI checks after full profile passes.

### Preserve large-hole workload under exact triangulation signs

- Axis-only profile89280 exit101:9/10passed,10x10holes/32samples still exhaust original8Mbudget. Exact collinearity alone is insufficient for prior workload compatibility. No WASM publication or budget increase.
- Added binary64 orientation forward-error filter over normal differences/products: accept only |det|>8*EPSILON*(|left|+|right|), deliberately loose relative to subtraction/product/sum rounding. Require normal determinant/error and finite magnitude; nonnormal differences/products, underflow/overflow/cancellation go to existing bounded exact predicate. Filter actualwork16 is charged plus admission1. Equal-axis and duplicate-coordinate zero shortcut retained. Coordinates unchanged; mesh-only source-bit semantics unchanged.
- Latest profile3525 live at `/tmp/sweep-exact-ear-normal-filter-regressions-20261005.log`; latest original corrected-body native61724 live at `/tmp/sweep-fixed-normal-normal-filter-native-20261005.log`. Both handles re-polled live, no terminal assertion yet. Earlier axis-only native80679 and module-unit45526 remain live at respective `/tmp/sweep-fixed-normal-axis-filter-native-20261005.log` and `/tmp/sweep-exact-ear-axis-filter-unit-20261005.log`. Do not restart delayed startup handles.
- rustfmt latest triangulation source and focused gitdiffcheck passed. Next: inspect latest profile/native results, repair any remaining true work or geometry regression before geometry publisher, then public83 and UI wide/narrow reruns; STEP61 historical geometry941 evidence remains finite qualified old package.

### Exact ear clipping workload and corrected body native checks pass

- Latest normal-filter profile3525 exit0:10/10,0.46s. Includes actual corrected cap boundary conformity/area,100aligned holes×32samples, reflected/reordered/scaled holes, collinear edges, crossing/nonfinite/budget refusals. Original8M budget unchanged.
- Latest original corrected-body native61724 exit0:1/1,4.88s; details1/2/4/8 tessellate closed. Earlier axis-only native80679 exit0 and module45526 exit0:2/2. Latest normal-filter module-unit37112 live at `/tmp/sweep-exact-ear-normal-filter-unit-20261005.log`; it checks work exhaustion plus concave holes/orientation.
- Sole geometry publisher39194 live at `/tmp/sweep-exact-ear-filter-geometry-wasm-20261005.log`, release compile active. Do not run public/UI against old941 package and relabel it a filter qualification. Final language fdf9132a remains unchanged and source preflight previously passed.
- Updated design contract to distinguish new open curved FixedNormal/corrected-cap finite evidence, current display-triangulation correction and remaining general Frenet/guide/contact correspondence, spatial/closed-corrected RMF, all-mode global and smoothness obligations. Full six-part objective unchanged.

### Exact-filter package verified wait and wider B-rep dispatch

- Latest normal-filter module37112 exit0:2/2,0.00s, concave cap/hole area across orientations and work-budget refusal. Latest profile10/10 and corrected-body details1/2/4/8 remain the current native evidence.
- Sole geometry publisher39194 re-polled live; release compilation finished1m07s and wasm-opt process confirmed active on its own input/optimized paths. Public geometry remains old941ee54f until terminal publication; no premature public/UI dispatch.
- Added relevant wider native B-rep registry dispatch10507, log `/tmp/sweep-exact-ear-bridge-registry-20261005.log`, to check shared tessellator compatibility across other bodies/seams. Compile complete, test handle not terminal. Next actual package checks remain public83, rebuilt Vite, corrected body wide/narrow lifecycle UI, expanded surface/body UI and independent new-package STEP provenance.

### Exact-filter WASM published; actual public and UI dispatched

- Geometry39194 exit0:raw12374527 ->11004258bytes, SHA075c58cdaf082edb2d4a5c7c6c22232a6da5e032ade531f4cec47db8762f9150. Final languagefdf9132a unchanged. Vite17243 exit0 at `/tmp/sweep-exact-ear-filter-vite-20261005.log`.
- Public10-suite94893 live at `/tmp/sweep-exact-ear-filter-public-20261005.log`; expected83tests, no success inferred before terminal. Focused headed original corrected FixedNormal body UI dispatched at `/tmp/sweep-fixed-normal-exact-ear-ui-body-20261005.log`, root `ui-fixed-normal-exact-ear-body-current`; covers wide/narrow successful Solid and lifecycle refusal/cancellation/restoration. Native wider bridge10507 remains live.

### New-package public83 and corrected-body wide/narrow UI pass

- Public94893 exit0:10suites83/83,57.31s. Original corrected small FixedNormal Rush body now passes viewport metadata and actual separate Solid admission, preserving strict-budget refusal; folded large body global-refusal test passes too.
- Native wider bridge10507 exit0:10/10,7.52s, including original corrected body tessellation, periodic STEP sphere/torus seams, curved trim holes and shared edge registry across bodies. New Rust filter has no identified regression in this relevant suite.
- Focused UI40387 exit0, `ui-fixed-normal-exact-ear-body-current/matrix.json`:2cases28checks,1440/600,headed WebGPU,zero pageErrors,passedtrue. Actual Solid18faces1shell; strict body E refusal, invalid path and bounded work refusal, build/Solid cancellation, source-change/restoration all passed. Cancellation fault scope remains held worker dispatch, not unconstrained native interrupt latency.
- UI provenance verifies source/public/dist and geometry worker identity SHA075c58cdaf082edb2d4a5c7c6c22232a6da5e032ade531f4cec47db8762f9150. Full body UI85807 live at `/tmp/sweep-exact-ear-ui-body-matrix-20261005.log`, root `ui-exact-ear-body-matrix-current`; new surface matrix must follow after body run.
- New-package STEP exporter93631 live at `/tmp/sweep-exact-ear-step-export-20261005.log`, root `external-step-exact-ear-current`; independent rational/generator/OCCT checks must run sequentially after exporter exits. Prior STEP61 remains old941 evidence, not current075 evidence.
- Full six-part completion remains unproven. Newly fixed display integration is a qualified finite mode, not a substitute for remaining general correspondence/global/smoothness requirements.

### Current075 package broad qualification and next Frenet premise

- New-package STEP exporter93631 exit0. Independent rational reference27188 exit0 and generator reference exit0 for `external-step-exact-ear-current`. New-package OCCT dispatched at `/tmp/sweep-exact-ear-step-occt-20261005.log`; result must be inspected before claiming current075 STEP61 passed.
- Full body UI85807 remains live:1440width7source cases completed, six successful Solid14checks each plus closed-planar13checks with expected separate Solid refusal. Narrow cases still pending; no full matrix success yet.
- Started next original-domain gap in Rust only: `frenet_values_at` now separates original path traversal from station-law traversal, preserving existing same-parameter wrapper behavior. New crate-private `certify_frenet_relative_values` builds original curvature frame at path interval and twist/scale/affine at station interval, zeroing translation solely for relative pose. It is not yet wired to arc-length acceptance and proves no new continuousBound by itself.
- Added independent analytic quadratic-path/moving Frenet frame test over Cartesian path[.2,.3]×station[.6,.7], nonunit source domain[2,5], plus one-cell-short unresolved refusal. Native69267 live at `/tmp/sweep-frenet-separated-source-values-native-20261005.log`. This later source premise is absent from current075 WASM; do not conflate qualification snapshots. Focused diffcheck passed.

### Current075 STEP61/bodyUI194 pass; Frenet retained arc-length proof under test

- OCCT4341 exit0:current `external-step-exact-ear-current/opencascade-sweep.json`61/61passed, provenance SHA075c58cdaf082edb2d4a5c7c6c22232a6da5e032ade531f4cec47db8762f9150,publicAndPackedVerifiedtrue. Numerical independent flux references remain finite oracle evidence.
- Body UI85807 exit0, `ui-exact-ear-body-matrix-current/matrix.json`:14cases194checks,1440/600,passedtrue; source/public/dist/worker identity075verified. Expected closed-planar separate global-Solid refusal preserved. Current surfaceUI85617 live at `/tmp/sweep-exact-ear-ui-surface-matrix-20261005.log`, root `ui-exact-ear-surface-current`.
- Frenet separate source/station value native69267 exit0:1/1,.00s. New original frame arc-length retained-section helper shares the FixedNormal interval image theorem, dispatching original Frenet relative values and original initial Frenet coordinates. Original inverse brackets, source-owned point bounds, Lipschitz position chord, station laws, retained rational basis and decomposition/correction remain mandatory. Open nonperiodic only; no closed seam/holonomy or derivative-jet claim.
- Added original C(u)=(u,u²,0) independent analytical length inverse/Frenet oracle with joint scale/axes/center/twist and actual retained surfaces, plus short proof budget refusal. Native38765 live at `/tmp/sweep-frenet-arc-retained-native-20261005.log`; native progressive module32042 live at `/tmp/sweep-frenet-arc-module-native-20261005.log`. These later source changes are not published in current075 WASM. No new Frenet public or body/full-bound qualification claimed yet. Shared multi-profile Frenet length measurement still needs integration before final publishing.
- Full six-part goal remains active, including all-mode global and smoothness gaps beyond qualified finite matrices.

### Original Frenet arc-length analytical proof passes; shared/native body checks dispatched

- Native38765 exit0:1/1,10.64s. Original quadratic C(u)=(u,u²,0), independent analytic length inverse and Frenet rotation with joint scale/axes/center/twist lie within retained-patch continuous error<unchanged2budget; actual preview accepted. One-cell-short certificate refuses. This later source is not current075 WASM.
- Added Frenet `frenet_patch_error_with_length`, shared MultiSweep original length measurement once per identical path/spacing, original source/spacing identity checks and aggregate per-profile pose/decomposition work. Shared64profile/16zero-twistspan negative39445 live at `/tmp/sweep-frenet-arc-shared-work-native-20261005.log`; required outcome is no positive E after partial work.
- Added native small corrected Frenet hollow-body test in original YZ profile with original XY quadratic path, explicit correction displacement and full body E, separate actual native volume/global admission. Native7581 live at `/tmp/sweep-frenet-corrected-arc-body-native-20261005.log`. Not yet proven positive; do not qualify body from surface proof.
- Earlier native progressive32042 remains live at `/tmp/sweep-frenet-arc-module-native-20261005.log`; compiled before later shared-length changes. Current surface UI85617 still live on old-qualified075 package at `/tmp/sweep-exact-ear-ui-surface-matrix-20261005.log`; wide scenarios progressing. Further latest-source module/fullcore and package/public/body/UI/STEP checks needed after focused tests.

### Original Frenet corrected hollow body native Solid passes; Rush fixtures prepared

- Native7581 exit0:1/1,5.41s. Small original YZ hollow profile along XY quadratic Frenet arc-length path: retained walls/exact corrected caps, filled full-body boundary E<=2, correction>0<=1e-9, separate actual native volume/global admission proven. Original maxSections17/E2 unchanged; explicit native cap correction, no source remapping shortcut.
- Shared-budget negative39445 still live at `/tmp/sweep-frenet-arc-shared-work-native-20261005.log`; old progressive module32042 still live (pre-shared integration snapshot). Latest full current core6517 live at `/tmp/sweep-frenet-arc-full-native-20261005.log`, including shared integration and new tests. Do not restart delayed test handles.
- Added original `examples/rush/arc-length-curved-frenet-progressive-sweep.r` and `arc-length-curved-frenet-body-boundary.r`. Actual source compile/strict schema via current languagefdf succeeds, preserving frenet/E2 and body correctionτ1e-9. Prepared public surface/body matrix additions, expected85tests; actual new geometry WASM/public evaluation not run until native gates and publisher.
- Current075 surface UI85617 remains live, narrow scenarios progressing. Its UI script/source list not edited while running, to preserve recorded provenance. Add Frenet UI modes after current075 run terminates; add independent STEP mode and publish complete source snapshot after focused native budget gates.

### Current075 surfaceUI780 pass and new Frenet publisher dispatched

- Surface85617 exit0, `ui-exact-ear-surface-current/matrix.json`:52cases780checks,1440/600,headed,passedtrue,source/public/dist/worker SHA075verified. Current075 public83/bodyUI194/STEP61 and native exact-triangulation checks are now terminal finite evidence.
- Shared Frenet39445 exit0:1/1,131.58s. 64profiles×16authored zero-twist spans exhaust aggregate proof budget without positive continuousE. No budget increase.
- After current UI terminated, added Frenet original surface/body sources to future UI modes, E2 and fully degenerate path refusal retained. Independent STEP factory now includes original small YZ hollow/Frenet XY quadratic case with explicit native correction and native Solid requirement, expected62cases; original rational-boundary Gaussian oracle and all retained full-domain topology checks remain required.
- vue-tsc81637 exit0, node UI syntax and focused diffcheck passed. Latest full native6517 remains live, earlier pre-shared module32042 live. Sole new geometry publisher39715 live at `/tmp/sweep-frenet-arc-geometry-wasm-20261005.log`, complete current Frenet source snapshot. No public85/STEP62/UI new-Frenet success yet. Final languagefdf unchanged; actual two new Rush sources previously passed strict compile.

### Frenet publication verified wait; wider native body regression dispatched

- Earlier progressive module32042 exit0:99/99,338.54s; compiled before shared Frenet integration, so it is not the latest full-source gate. Latest full current core6517 remains live and has reached progressive theorem tests, log `/tmp/sweep-frenet-arc-full-native-20261005.log`.
- Sole Frenet geometry39715 re-polled live; release compilation finished1m14s, optimizer/publication not yet terminal. Current public WASM remains075 until final SHA verification. New public85/UI54surface16body/STEP62 must run only after terminal publisher.
- Started full current native B-rep43405 at `/tmp/sweep-frenet-corrected-full-brep-native-20261005.log`, currently live, progressing through constructor/corrected-cap tests. Scope checks latest explicit endpoint-correction wrapper compatibility and global volume/topology beyond focused corrected Frenet body.
- Current qualified075 evidence remains public83,bodyUI14/194,surfaceUI52/780,STEP61. No all-mode or latest-Frenet publication completion inferred from verified waits. General curved guide/contact normalized-length correspondence and spatial/closed-corrected frame/smoothness/global gaps remain.

### Frenet arc-length optimized package published; actual end-to-end dispatch

- Sole geometry39715 exit0:raw12375814 ->11005404bytes, SHA4ead47d1a35d182fdfe30afd8e1c6f91e8c64dfafd889bd791360c1a935595f3. Final languagefdf unchanged. Vite15011 exit0,1.14s.
- Public29298 live at `/tmp/sweep-frenet-arc-public-20261005.log`, expected85tests/10suites. STEP exporter61257 live at `/tmp/sweep-frenet-arc-step-export-20261005.log`, root `external-step-frenet-arc-current`, expected62cases; independent references/OCCT must follow sequentially after terminal exporter.
- Focused headed Frenet original body UI dispatched at `/tmp/sweep-frenet-arc-ui-body-20261005.log`, root `ui-frenet-arc-body-current`, width1440/600. Its actual viewport tessellation and successful Solid are not inferred from native focused body pass.
- Latest full core6517 and full B-rep43405 still live on exact source snapshot, respectively `/tmp/sweep-frenet-arc-full-native-20261005.log` and `/tmp/sweep-frenet-corrected-full-brep-native-20261005.log`. No new-package comprehensive success claimed before terminal evidence. All original general/global/smoothness goal gaps preserved.

### Frenet public85 and wide/narrow bodyUI28 pass; guide triple-domain premise started

- New-package public29298 exit0:10suites85/85,69.77s. Includes original Frenet curved arc-length surface→viewport proof and corrected hollow body→actual separate Solid admission/strict-refusal, plus earlier regressions.
- Focused Frenet body UI10509 exit0, `ui-frenet-arc-body-current/matrix.json`:2cases28checks at1440/600,passedtrue,source/public/dist/worker SHA4ead47d1a35d182fdfe30afd8e1c6f91e8c64dfafd889bd791360c1a935595f3verified. Original body Solid, strict/invalid/budget refusals, held-dispatch cancellation, source-change/restoration passed. No arbitrary native interrupt-latency claim.
- STEP exporter61257 exit0,root `external-step-frenet-arc-current`. Independent rational reference29214 live at `/tmp/sweep-frenet-arc-step-rational-20261005.log`; generator and OCCT must follow sequentially. Focused surface UI87688 live at `/tmp/sweep-frenet-arc-ui-surface-20261005.log`,root `ui-frenet-arc-surface-current`. Full core6517 and B-rep43405 remain live on exact published Frenet source snapshot.
- Later guide source-only work: preserved old same-parameter public wrappers; `certify_guide_values_at`/`path_guide_values_at` separate original path, original guide and station-law intervals; new relative-values helper zeroes translation only for pose composition. Original path velocity/position and guide rail are each enclosed on their own interval; twist/scale/affine use station interval. No contact or arc-length acceptance dispatch yet.
- New independent analytic path C(u)=(0,u²,u), guideG(v)=(1,v²,v), distinct source domains[2,5]/[-3,7], Cartesianu/v/stationgrid and one-cell-short refusal native4748 live at `/tmp/sweep-guided-separated-source-values-native-20261005.log`. This later premise is absent from current4ead WASM and fullnative6517 snapshot. Focused diffcheck passed; no guided arc-length completion inferred.

### Frenet STEP62/surfaceUI32 pass; original curved guide arc-length source theorem dispatched

- Guide triple-domain native4748 exit0:1/1,.00s, independent Cartesian path/rail/station analytic frame and one-cell-short unresolved refusal. Later source-only premise, absent from current4ead.
- Focused surface87688 exit0, `ui-frenet-arc-surface-current/matrix.json`:2cases32checks,1440/600,passedtrue,source/public/dist/worker4eadverified. Surface-only Solid refusal remains expected. Current full body matrix90558 live at `/tmp/sweep-frenet-arc-ui-body-matrix-20261005.log`; full surface matrix must follow.
- Independent rational29214 exit0,generator exit0,OCCT23649 exit0. Current `external-step-frenet-arc-current/opencascade-sweep.json`:62/62passed,publicAndPacked4eadverified. Frenet corrected hollow18faces40edges,native Solid true,full-domain wall andcapcoedge agreement true; OCCTvolume0.0574701756807908 vs independent0.057470175680790804,relative1.2073904110626423e-16. Finite evidence, not universal theorem.
- Later source-only original-frame helper now supports open nonperiodic curved guide/path arc-length without contact. It independently measures both original curves within one charged budget, checks actual constructor path AND guide station parameters, encloses two inverse brackets and station laws, requires original retained section basis before relative-image/chord composition. Closed/periodic guide refuses. No contact fit or new derivative-jet/smoothness claim.
- Added independent analytic quadratic translated curved guide and path (different source domains), joint laws, original retained patches and short-budget refusal. Native94928 live at `/tmp/sweep-guided-curved-arc-retained-native-20261005.log`. No guided complete/public/body/Solid claim before test and further shared-budget checks. Full core6517 remains live on published Frenet snapshot, fullBRep43405 now exit0; final count/log still to inspect.

### Full published Frenet native core passes; curved guide retained proof refuses

- Full current published-Frenet core6517 exit0:1174/1174,669.40s, including shared Frenet aggregate work and prior FixedNormal/source inverse predicates. This compile predates later guide triple-domain/retained source edits; do not call latest guide source fully regression-qualified.
- Full published-Frenet B-rep43405 exit0:702passed/2ignored,461.36s. No hidden ignored-test completion claim; ignore scope remains previous repository behavior.
- Curved guide retained independent test94928 exit101:1failed,1.61s, expectedCertified gotUnresolved at main bound assertion. No actual guided bound/public success. Added Debug certificate to assert and diagnostic12378 live at `/tmp/sweep-guided-curved-arc-diagnostic-native-20261005.log` to distinguish two-source inverse measurement/budget refusal from pose enclosure. Original maxcells10000 and E2 unchanged. Do not weaken refusal or relabel source as qualified.
- Current4ead full body UI90558 remains live, narrow cases progressing at `/tmp/sweep-frenet-arc-ui-body-matrix-20261005.log`. Fullsurface must follow after terminalbody, no parallel UI matrix. Next actual guide action depends on diagnostic result; current positive guide triple-domain premise alone is insufficient for continuousE.

### General original length prefix cache; guided bound certified but initially above budget

- Added per-division prefix enclosure cache to native original `curve_measure`: only completedwithinTolerance prefix reports reused across station targets for the same exact curve/tolerance/total; cellconstructionchargedonce. No global/cross-source cache, incomplete reports not reused, inverse sign/brackets and residuals unchanged. Single point_at_length keeps isolated cache.
- Native53144 exit0:existing10length/inverse tests,1.27s. New two-source/nonunitdomain independent analytic inverse65019 exit0:1/1,.75s; both16segment divisions within original combined10000cellbudget, residual<=.001 and originalinversebrackets containanalyticparameters. Later sources only, not current4ead.
- Guided cached83323 exit101: main proof nowCertified,cells7615,error2.0175976993051488>unchangedE2; no unresolved length budget anymore. Test refuses at errorbudget, not coefficient/regularity. Original failed94928/diagnostic12378 remain truthful uncached worklimit failures. No guided public success.
- Tightened guide values binormal using cross(originalvelocity,originaloffset), then normalize: positive originalspeed factor cancels exactly, avoiding repeated interval tangent quotient; no constructor/source changes. Native80903 live at `/tmp/sweep-guided-curved-arc-raw-velocity-native-20261005.log`. Latest profilemodule6031 live at `/tmp/sweep-guided-curved-arc-cached-module-native-20261005.log`, compiled before this tightening and contains known2.017budget failure; do not conflate snapshots.
- Current4ead bodyUI90558 exit0:16cases222checks,1440/600,source/public/dist/workeridentityverified. Fullsurface97378 live at `/tmp/sweep-frenet-arc-ui-surface-matrix-20261005.log`. Current4ead qualification remains public85/STEP62/native1174/BRep702+2ignored and focused/body matrices; later guided/prefix source requires new native/package checks.
- Added explicit zero remaining guide-length-work unresolved guard before second division instead of invalid0cell request. Focuseddiffcheck passed before velocity tightening; recheck latest before publication. Full goal unchanged.

### Guided curved original-domain retained E passes; aggregate two-length integration dispatched

- Raw-velocity guided80903 exit0:1/1,11.55s, independent original curved path/guide normalized-length images with joint affine/center/scale/twist fit unchangedE2 at17sections. Retained full proof/certificate and actual previewaccepted, short budget refuses. Earlier2.017cached snapshot remains failure evidence, not current result. No guided WASM/public qualification yet.
- Added MultiSweep sharedpath AND guide length reports, identical original curve/spacing/guide ownership checks, measuredonce each within original10000aggregatecells. Per-profile pose/decomposition consumes remainingbudget. Exact original constructor stations/bases still checked; no cross-source numerical cache. Guided-only eligibility excludes authoredframes/contact/closedguide.
- Latest check54862 exit0 (before adding explicit frame_laws.is_none eligibility), focused diffcheckpassed after latest edits. Native shared2profilepositive/64profile16zero-twistspannegative87947 live at `/tmp/sweep-guided-two-lengths-shared-native-20261005.log`. Must prove positive two-profile certificate and refuse partial64 without false E. No result inferred from compilation.
- Added original small guided hollow body native actualSolid test: two curved independent-domain sources, affineaxes2/3,explicit nativecapcorrection,actualretainedcaps/walls/filledE and independent globalvolumeadmission. Native43888 live at `/tmp/sweep-guided-curved-arc-body-native-20261005.log`. Current bodyglobal positivity is unverified until terminal evidence.
- Earlier cachedmodule6031 remains live, compiledbefore rawvelocity/sharedguide changes and includes known2.017failure; latestsource broadregression required later. Current4ead fullsurface97378 remains live on narrow cases. Full objective/allmode/smoothness scope preserved.

### Guided native corrected body passes; original Rush/UI/STEP fixtures prepared

- Native guidedbody43888 exit0:1/1,23.94s. Small original hollow XY profile, original curved path/guide, separate normalized-length images, sharedtwo-sourcemeasurement, affineaxes2/3,explicitcapcorrection/fullfilledE and actualglobalvolumeSolid admission proven. No publicguidedbody success until newpublication.
- Shared87947 remains live, log `/tmp/sweep-guided-two-lengths-shared-native-20261005.log`. Required2profilepositive/64profilepartialnegative not inferred midtest. Latest module46369 live at `/tmp/sweep-guided-two-lengths-latest-module-native-20261005.log`, latestfullcore94582 live at `/tmp/sweep-guided-two-lengths-full-native-20261005.log`.
- Older cachedmodule6031 exit101:100passed/1failed,313.44s, exactly knownpre-tightening2.017guidebudgetfailure. Preserve failure as oldsnapshot evidence, not latestregression.
- Current4ead fullsurface97378 exit0, `ui-frenet-arc-surface-matrix-current/matrix.json`:54cases812checks,1440/600,passedtrue,source/public/dist/worker4eadverified. Current4ead public85/bodyUI16/222/STEP62/native1174/BRep702+2ignored now terminalfinite qualification.
- Added original curvedguided Rushsurface/body examples, actualstrict compile accepts guide reference/rmf/E2 andbodycorrection1e-9. Preparedpublic87, future UI56surface/18body and independentSTEP63 with smallhollow/explicitcorrection/nativeSolid requirement. Updated UI only after prior matrixterminal to preserve prior provenance. Nodecheck/diffcheckpassed, vue-tsc11336 live at `/tmp/sweep-guided-curved-arc-types-20261005.log`.
- No new guidedgeometrypublisher yet. Current4ead package is Frenetqualification, laterprefixcache/guide/shared changes require own package/public/UI/STEP. Originalall-mode/contact/RMF/closed/smoothness objectives remain active.

### Guided shared aggregate gate passes; geometry publication dispatched

- Shared87947 exit0:1/1,216.98s. Two profiles accepted with one path and one guide length certificate;64profiles with16authoredzero-twistspans refuse partial aggregate work and leave continuousE absent. Original10000proofcells/E2 preserved.
- Latestguidedmodule46369 andfullcore94582 remain live on exact source snapshot. Solegeometrypublisher3562 live at `/tmp/sweep-guided-curved-arc-geometry-wasm-20261005.log`, releasecompileactive. Do not reuse4eadartifactasnewguidedqualification. Final languagefdf unchanged, actualtwooriginalguidedsourcespassedstrictcompile.
- Typecheck11336 exit0. Started full latestnativeBRep at `/tmp/sweep-guided-curved-arc-full-brep-native-20261005.log`, to verify sourceinverseprefixcache and sharedguidedconstructor changes beyond focusedbody. Nativefocusedguidedbody and independentquadguideoracle already passed, but generalregression packagegates remain pending.
- Inspected contact_anchor: retained contact values/jets currently refuse nonparameter spacing. General contact fitted normalizedlength and original support/anchor correspondence remain explicit nextgaps; no completion claim from guidedwithoutcontact proof.

### Guided optimized original-domain package published; actual public/STEP/UI dispatch

- Guidedgeometry3562 exit0:raw12384678 ->11013360bytes, SHA7514d7be29640eb2e69d11440489c7b53dc7ccc4aafa630e8171bc564edf943a. Includes generalper-divisioncompletedprefixcache,guide/path/stationvalue intervals, rawvelocitybinormal and sharedtwo-source aggregate lengths. Final languagefdf unchanged. Vite78158 exit0.
- Public18708 live at `/tmp/sweep-guided-curved-arc-public-20261005.log`, expected87tests10suites. STEP exporter48480 live at `/tmp/sweep-guided-curved-arc-step-export-20261005.log`,root `external-step-guided-curved-arc-current`,expected63; independent rational/generator/OCCT must follow sequentially.
- Focused headed originalguidedbody UI dispatched at `/tmp/sweep-guided-curved-arc-ui-body-20261005.log`,root `ui-guided-curved-arc-body-current`,1440/600. No successfuldisplay/Solid inferred before actualterminalartifact. Later native module46369/fullcore94582/fullBRep26781 still live on exact publishedsource snapshot; no broadnativecompletion yet.
- All six original goal parts remain active beyond this finite guided qualification: contactfitnormalizedlength,spatial/closedcorrected RMF,closedframe/seam/smoothness,all-mode globalembedding and fullcombinationmatrix not closed. No newcontactacceptance or claim from this guidedwithoutcontact package.


### Guided package gates completed; distinct guide-curvature oracle dispatched

- Published geometry SHA7514d7be29640eb2e69d11440489c7b53dc7ccc4aafa630e8171bc564edf943a: public18708 exit0,10suites87/87 (69.28s); native sweep module46369 exit0,102/102 (536.61s); full core94582 exit0,1177/1177 (735.03s); full BRep26781 exit0,703passed/2existing manual timing ignored (444.15s). These full native gates predate only the following test-only distinct-curvature addition.
- STEP exporter48480, independent rational-boundary71793, generator-reference and OCCT76667 all exit0. Root external-step-guided-curved-arc-current contains63cases; OCCT overallpassed true. Independent Gaussian convergence/volume is a numerical oracle, not a new interval theorem.
- Focused headed body92597 exit0:2cases28checks,1440/600,successfulSolid. Focused surface25446 exit0:2cases30checks, source-onlySolidcorrectlyrefused. Both actual matrix provenance verifies source/public/dist/worker geometry7514. Held dispatch lifecycle checks do not prove arbitrary mid-kernel cancellation latency.
- Full body UI90327 live: /tmp/sweep-guided-curved-arc-ui-body-matrix-20261005.log,root ui-guided-curved-arc-body-matrix-current. Full surface matrix on new7514 remains pending; earlier4ead matrices remain historical evidence.
- Added Rust-only guided_distinct_curvature_arc_lengths_enclose_independent_frame_oracle: original C=(0,u²,u),G=(1,2v²,v),guide domain[-3,7], independent analytic inverse primitives with slopes2/4, direct float cross/normalization, affine/center/twist, retainedpatch samples, actual preview and one-cell-short refusal. Native27805 remains live at /tmp/sweep-guided-distinct-curvature-native-20261005.log; not yet a passed claim. No runtime geometry/source changes after published7514.
- Full original objective remains active: contactfit normalizedlength, spatial/closed RMF and correction, applicable movingframe/closed/multispan smoothness,all-mode global certificates and comprehensive combination qualification remain open.


- Distinct-curvature27805 terminalexit101:17stations exhaust10000proofcells with arc-length-correspondence-unproved (1.42s), not a geometric contradiction or falsepositive. Preserve actual denial as explicit regression; new77670 tests17stationrefusal plus9station independent analytic geometry at same10000/E2/tolerance. Log /tmp/sweep-guided-distinct-curvature-budget-native-20261005.log, stillpending. General17stationdistinctcurvature acceptance remains unproved; no limitincrease or claim.
- Inspected actual new guided STEP case:34faces/72edges,1closed outwardShell,2holecaps,nativevolume true, retainedexactbasis/controlnet/UV/full-domainwall andcapcoedge agreements true; OCCTvolume0.058055170563036364 equals independent rational-boundary estimate at printedprecision. Numericalvolumeagreement remains finitefixtureevidence.


### Original contact-fit normalized-length implementation in Rust; native gates

- Distinct guide-curvature regression77670 exit0,1/1,11.72s:9stations independent geometry enclosed;17stations correct10000cellrefusal preserved. No runtime source change in this test-only gate.
- Full headed guided body90327 exit0:18cases250checks,1440/600; actual artifact provenance7514 source/public/dist/worker verified. Full surface1362 live at /tmp/sweep-guided-curved-arc-ui-surface-matrix-20261005.log,root ui-guided-curved-arc-surface-matrix-current,expected56cases. Preserve package7514 until UI terminal; no newpublisher yet.
- Rust contact_fit_value_at now separates original path parameter restriction,original guide restriction and authored station laws. Uses actual constructor norm(G-P) width, positive original anchor and denominator; any exhausted stage yields no fit. Old same-parameter public wrappers unchanged. New relative control composition uses zero translation source, original guided frame, scale/axes/center and charged fit work. No derivative jet/contact identity is inferred.
- General original inverse-length retained section theorem now accepts open nonperiodic Contact sources: both original divisions with exact actualconstructorparameter agreement; original anchor ownership/value certificate; complete fit/frame image; actual retained basis/control correspondence; endpoint and Lipschitz path chord residual; mandatory decomposition. MultiSweep shares original path+guide length work once and charges each original anchor/fit/frame/decomposition against aggregate10000.
- Native86363 exit0,1/1,11.19s: curved C=(0,u²,u),original G=(1,u²,u) domain[-3,7],contact profile endpointx1, joint scalar/affine/center laws; independent analytic inverse/direct fit formula coversretainedpatches, actual preview accepted, onecellshort unresolved.
- Nativecontactvalues32756 exit101 only due test fixture primitives::line rejecting constant zero-length law. Fixed fixture uses existing constant_vector_law;68607exit0,1/1,.00s. Independent Cartesianu=.25/v=.5/s=.75 oracle coversrelativefit/frame; incompletebudget and negativeoriginaldenominator yieldnoimage.
- Existing contact regressions53598 exit0,27/27,14.14s (before latest sharedcontact routing); cargo check75196exit0; safevue-tsc88543exit0. Latestprogressivemodule79233 remainslive at /tmp/sweep-contact-curved-arc-module-native-20261005.log, snapshotpredatessharedcontacttest.
- Sharedcontact7179 remainslive: /tmp/sweep-contact-curved-arc-shared-native-20261005.log,2profilepositive/64profile16spanaggregateexhaustion at unchanged10000/E2. Nativeactualbody63286 remainslive: /tmp/sweep-contact-curved-arc-body-native-20261005.log,small hollow contactbody with actualvolume gate and explicitcapcorrection. No positivebodyclaim before terminalevidence.
- Prepared original Rush source arc-length-curved-contact-progressive-sweep.r and arc-length-curved-contact-body-boundary.r. Public surface regression added; package7514 does NOT include contact changes. No published contactWASM/Rush/viewport/Solid proof yet. Next qualifyshared/body, finisholdpackageUI, then solepublisher and newpackage gates. No fullgoalcompletion claim.


### Guided full surface matrix and contact endpoint-domain theorem

- Guided full surface1362 exit0:56cases842checks (actual count, not expected844). Artifact provenance7514 source/public/dist/worker verified. Combined package body18cases250checks plus surface56cases842checks; finitefixtures only, held dispatch cancellation scope unchanged.
- Sharedcontact7179 exit0,1/1,213.38s:2profiles accepted and64profiles16zero-twistspans do not promote partial aggregate work. Default10000/E2 preserved.
- Nativecontactbody63286 exit101,23.49s: wallcontinuous certified but filled_cap_error_upper absent due original profile_domain unconditional contact refusal. Implemented endpoint positive-fit theorem in Rust: original reference-anchorownership sameacrossallprofiles; complete original endpoint fit value with positive lowerbound; invertible diag(scale*axisX*fit,scale*axisY,scale*axisZ) source material mapping preservesholes. Contactfit recorded separately and included in inverse-transpose originalnormal transport. No original endpoint contactidentity/global embedding inferred.
- Oblique source-plane regression54447 exit0,1/1,.01s: independent inverse-transpose with fit1/2.1 for source normal(-.1,0,1); originalsourceguide differentdomain,curvedpath,exactsourceplane and one-cellshort refusal. Existing profiledomain34714 exit0,9/9,.03s (before addedobliquetest).
- Updatedcontactbody49947 exit101,23.30s: filledcapbound now present but boundary_error_within_budget Some(false) at unchangedE2. Do not callbodycomplete. Diagnostic79647 live at /tmp/sweep-contact-curved-arc-body-bound-diagnostic-native-20261005.log will printwall/filled/boundary/normaldot before deciding refinement; no silentbudgetincrease or positivepublish. Runtime contactchanges remainunpublished.
- Latest contactmodule79233 stilllive on earliercontact-relative snapshot; no restart for idle dyld. Newbody/oblique snapshots compiledsuccessfully and targetedresultrecorded. New source/fixture tests require new package gates after continuousbodybound issue isresolved.


### Contact body complete boundary bound restored at unchanged E2; sole publisher dispatched

- Diagnostic79647 terminalexit101,23.49s: wall1.943455064222962,filledcap2.4527920613034517,boundary2.4527920613034517. The source of excess was the conservative station-wide endpoint-displacement bound reused for whole-sweep endpoint contours, not missingcapdomain or badsurface. Normal-dot magnitudes .03 reflect unnormalizedretainedplane coefficients and were nonzero; no denominator/cap theorem was weakened.
- Rust SectionInterpolationReport now has separate cap_endpoint_displacement_upper; PatchErrorReport endpoint_contour_error_upper composes eachendpoint withmandatorydecomposition. All existing station-wide original/retained endpoint bounds remain unchanged, including their provenance. Ordinary modes retain conservative fallback. Contact normalizedlength evaluates two originalsourceendpoints,wholebatchoriginalq/anchor/laws/fit/frame; comparesactual storedendpointcontrols with outwardoriginalpathbounds andresidual; unchangedpositiveweights extend poleerrors tocompletecontours. Additional endpointwork is charged to same10000 aggregate and incompletework publishesno positivecertificate. No TS formula.
- Native19901 exit0,1/1,11.20s: originalcurvedjointlawcontact proof incl separatelyboundedcaps<1e-8, independentanalyticgeometry,actualpreview,onecellshort refusal. Native70964 exit0,1/1,23.86s: fullactualcontact hollowbodybound now≤unchanged2, explicitcorrection≤1e-9included, exactretainedcaps/wholewalls,holes,nativeboundary/nesting/orientation/volume admission passed. Former63286/49947/79647 failures remain historicaldiagnostics.
- Previous contactmodule79233 exit0,104/104,558.63s; snapshotpredateslater sharedcontact/profiledomain/endpoint changes. Latest fullcore8951 live at /tmp/sweep-contact-endpoint-full-native-20261005.log,expected1182tests. Latest fullBRep51057 live at /tmp/sweep-contact-endpoint-full-brep-native-20261005.log,expected704pass2manualtimingignored. Sourcecurve_measure integration48348 live at /tmp/sweep-contact-endpoint-length-integration-native-20261005.log,expected11.
- Safe types87925exit0; targeteddiffcheck andUIscript syntaxexit0. Preparedneworiginalcontact Rushsurface/body,public regressioncases (expected89tests10suites),STEP arc-curved-contact nativeSolidrequired case (expected64),UIbody10modes/surface29modes. Allthinfixtures/options only.
- Sole geometrypublisher10035 dispatched at /tmp/sweep-contact-endpoint-geometry-wasm-20261005.log. Do not qualifycontact with old7514. Final languagefdf unchanged; actualneworiginalRushsource mustpasscompile/runtime onnewgeometryafterbuild. Vite/public/STEP/UInewpackage gates remain pending.
- Original fullobjective stillactive; selectedcontactnormalizedlength proofs do not close spatial/closedRMF,closed/multispan/movingframe G1/G2,continuouscontactidentity forallguides,all-modeglobalgeometry or comprehensivecombinationmatrix.


### Published contact geometry and actual public/source/body UI qualification

- Solepublisher10035exit0: raw12392944 ->11021104bytes; geometrySHA6517e65c6fdc3b107e2b9431251bf77d34d34797d90b5b22eda7106fa50e1e40. Packed/public fromsamecompletedpublisher; languagefdf unchanged. Vite65848exit0. No newruntimegeometryedits afterpublication.
- Public98432exit0:10suites89/89,87.37s, including originalcontact Rushsurface/body, retainedcontinuousbound,strictrefusal,actualnativeSolid. Sourcecompileartifact contact-arc-source-compile-current.json verifiesbothactualRushfiles withfinal languagefdf, anchors1/0,originalguide,spacingarc,E2,max17 andexplicitnativecapcorrectionbody. Compile-onlyscope distinctfromruntimequalification.
- Focusedheadedbody41134exit0:2cases28checks,1440/600,actualsuccessfulSolid and strict/invalid/budgetrefusals,helddispatchcancel/sourcechange/restoration. UIartifactprovenance6517 verifies source/public/dist/worker. Scope remains helddispatch lifecycle,not arbitrarynativecancellationlatency. Focusedsurface42225live at /tmp/sweep-contact-endpoint-ui-surface-20261005.log,root ui-contact-arc-surface-current.
- STEPexport44091exit0:64cases,manifestartifact6517/publicAndPackedVerifiedtrue. New contact case34faces,actual5retainedsections (max17),nativeSolidRequiredtrue,originalstationendpointbound1.7343858993876857 staysseparate from twoactualendcontourbounds[5.25365465357235e-14,8.271974135812902e-14]. Actualfilledcaps[8.019380669703938e-13,8.446235504439891e-13],correction5.145192987106734e-13,boundary1.9434550642234771≤unchanged2. No constructorrefit/budgetincrease.
- Independentrationalvolume54367exit0; generatorreference dispatched at /tmp/sweep-contact-endpoint-generator-volume-20261005.log. OCCT mustfollowsequentially;64/64notclaimed before actualverifier. FiniteGaussianconvergence remains numericaloracle,notintervaltheorem.
- Latestfullcore8951/fullBRep51057 stilllive; corecurrentlysharedcontactaggregation, BRepvolumetests. Curve_measure48348terminalexit0:11/11,2.06s,includingoriginaltwo-sourceinverses andprefixcache. No re-execution on observationtimeouts.
- Remainingfullobjective unchanged:generalcontactidentity,spatial/closedcorrected RMF,applicablemovingframe/multispan/closedG1/G2,all-modeglobalcertificationandallcombinationqualification. No completionclaim from currentfinitepackage.


### Contact package independent STEP and full BRep gates complete

- Independentrational54367,generatorreference andOCCT1427 terminalexit0. OCCTroot external-step-contact-arc-current:64/64 overallpassedtrue,artifact6517 matchingmanifest/public/packed. Inspect actualnewcontactcase properties in opencascade-sweep.json; finiteoriginalcases do not proveallmodeembedding.
- LatestfullBRep51057exit0:704passed,2existing manualtimingignored,468.56s. Fullcore8951 stilllive atsharedcontactaggregate gate; no restart.
- Focusedsurface42225exit0:2cases30checks1440/600,source-onlySolidrefusal correct. Surfaceartifact6517source/public/dist/workerverified. Alongwithfocusedbody28checks this gives58newcontactUIchecks.
- Fullnewpackagebody19730live at /tmp/sweep-contact-endpoint-ui-body-matrix-20261005.log,root ui-contact-arc-body-matrix-current,expected20cases278checks. Fullsurface58cases remains nextserialUIgate. Old7514completebody18cases250/surface56cases842 remainshistoricalevidence ratherthannew6517matrix.
- Allsixoriginalgoalparts remainactive beyondselected contact qualification:generalcontinuouscontactidentity,spatial/closedRMF/correction,applicablemovingframe/multispan/closedG1/G2,all-modeembedding/holes/nesting/orientationandcomprehensivecombinationqualification. No completegoalclaim.


### Open original planar RMF normalized-length theorem dispatched after contact publication

- Package6517contactfullcore8951 andfullbodyUI19730 remainlive. No oldprocessrestart; oldcompilednative snapshotkeptdistinctfromfollowingRust sourcechanges. Source/public/dist/worker7514/6517 qualification remainsbyteowned.
- AddednativeRmfPlanaroriginalarc dispatch gatedbyexistingoriginal coefficientplane/axialnormal identity,open/nonperiodic,noauthoredframe/noorientationguide/no contact. Constant plane normal is the Bishopnormal; independent original FixedNormal value restriction encloses idealplanarRMF frame, actualRMFretainedcontrols preserveconstructiondrift. Original inverse-lengthtotal/stationbrackets,sourceq,affine/center/twist,retainedbasis/decomposition/Lipschitzchord work all mandatory. No spatialRMF/nonaxialplane/closedholonomy proof inferred.
- Corrected seedownership: RmfPlanar uses original RotationMinimizing initialframe certificate ratherthan claiming FixedNormal constructororientation. MultiSweep sharesoriginalpath lengthonce acrossplanarRMFprofiles; perprofilepose/decomposition chargedagainstunchanged10000.
- Native71400exit0,1/1,5.73s: independentanalytic originalcurvedpathinverse +jointscalar/axes/center/twist; actualRMFpreview acceptedE2/max17,one-cellshort Unresolved. Native59135exit0,1/1,2.94s: priorparameterretainedproof plusoriginalcurvedarc33section certificate. cargocheckexit0.
- NewsharedplanarRMF12456live at /tmp/sweep-planar-rmf-curved-arc-shared-native-20261005.log:64profiles16authoredzero-twistspansmustrefusepartialaggregate. Addedsourceguards smallestbinary64 offplanepole andclosedcirclewithoutcorrection; latestfocusedgate at /tmp/sweep-planar-rmf-curved-arc-source-guards-native-20261005.log remainspending. No newRMFWASM publisher orpublicclaimyet.
- Allsixoriginalgoalparts remainactive. This selectedopenplanarRMF arc theorem extendscoverage; it doesnot replace requestedgeneralspatial/closed/movingframe/smoothness/global geometry guarantees.


- LatestplanarRMFsourceguards86565exit0,1/1,5.67s: independentcurvedjointlaws andexactlength oracle passed; smallestbinary64offplanepole leavesRMF correspondenceunproved;closedcirclewithoutcorrection remainsUnresolved; onecellshort noE. Selectedopenaxialplane theorem only,notgeneralnonaxial/spatial/closedclaim.
- Prepared original RushplanarRMFsurface/body arc sources; both actualstrictcompile pass withfinal languagefdf:rmf,arc_length,E2,max17. No newgeometrypublicationyet. Added nativeactualsmallhollowbody gate7800 at /tmp/sweep-planar-rmf-curved-arc-body-native-20261005.log,live. Shared64profile gate12456 stilllive. Existingcontactpackage6517core8951/bodyUI19730 stillliveon compiled/package snapshots;sourceeditsnotrelabeled asoldartifactproof.


- SharedplanarRMF12456exit0,1/1,86.53s:64profiles16zero-twistspans exhaustaggregate and leavecontinuousEabsent. NativeplanarRMFactualbody7800exit0,1/1,4.88s: originalcurvedarc smallhollowbody,wholewalls+filledcaps+explicitcorrection withinE2,actualnativeboundary/nesting/orientation/volumeproof passed. These nativeproofs areafter6517publication; nonewWASMclaim. StartedallnativeRMFregressions at /tmp/sweep-planar-rmf-curved-arc-all-rmf-native-20261005.log; pending.


### Contact snapshot full core/body UI terminal; full surface dispatched

- Contactpackage6517compiledsnapshotfullcore8951exit0,1182/1182,971.67s. This precedeslatestplanarRMFarc sourcechanges; do not callcurrentnewsourcefullqualified. NativeBRep704pass2manualtimingignored,public89/89,STEP64/64 remainqualified6517.
- Fullheadedcontactbody19730exit0:20cases278checks1440/600; actualmatrixartifactsource/public/dist/worker6517verified. Fullsurface97253liveat /tmp/sweep-contact-endpoint-ui-surface-matrix-20261005.log,root ui-contact-arc-surface-matrix-current,expected58cases872checks. Keep6517public/dist stableuntilterminalUI; no planarRMFpublisher yet. NewRMFallregressions65774stilllive.
- Fullobjectiveactive: finitecontactpackage checks andnativeopenplanarRMFarc additions do not provegeneralspatial/closedRMF,smoothness,contactidentity,all-modeglobalgeometry/comprehensivecombinations.


### Contact full UI matrix complete; planar RMF package publisher dispatched

- Contactsurface97253exit0:58cases872checks1440/600. Actualartifact6517source/public/dist/worker verified. Combined contactpackageUIbody20/278+surface58/872=78cases1150checks. Public89/89,core1182/1182,BRep704pass2manualtimingignored,independentSTEP64/64 qualifiedthisfinitepackage; notfullgoal.
- LatestRMFregressions65774exit0:12/12,98.82s, includesopenarcindependentoracle,wrongplane/closedsourceguards, parameter/closed-existingcases,shared64profileexhaustion andendpointmaterialdomains. Safe types23489/32648exit0. Newlatestfullcore39589 at /tmp/sweep-planar-rmf-curved-arc-full-native-20261005.log expected1184; fullBRep49143 at /tmp/sweep-planar-rmf-curved-arc-full-brep-native-20261005.log expected705pass2manualtimingignored remainlive.
- Preparedpublic92tests10suites: neworiginalplanarRMFarc surface/body, plusfoldedplanarRMFbody withE2 expectedgeometrySolidrefusal. BoundaryEproof cannot authorizeglobalmaterialadmission; no unexecutedpositive/refusalclaim. Sourcecompileartifact planar-rmf-arc-source-compile-current.json verifies3originalRushsourcesonfinal languagefdf (rmf,arc_length,max17,E2); runtimegeometrygatesseparate.
- PreparednativeSolidrequiredSTEP arc-curved-planar-rmf fixture,expected65cases:smallholloworiginalcurvedpath,axes2/3/1,explicitnativeendpointcapcorrection and independentrationalboundaryvolume. FullUIconfig nowincludesoriginalnewmode (body11modes/surface30modes), editedonlyafteroldcontactUIterminal; budgetE2anddegeneratesourceguardincluded. Scriptsyntax/targeteddiffchecks pass.
- Solegeometrypublisher21844live at /tmp/sweep-planar-rmf-curved-arc-geometry-wasm-20261005.log. Do notqualifynewplanarRMFarc withold6517. Sourcecompiledsnapshotnoothergeometrymutationsplannedwhilepublisher/gatesrun. Vite/public/STEP/UInewbytequalificationpending.
- Originalfullscope remainsactive; opencoordinateplanarRMFarc doesnotclosegeneralnonaxial/spatial/closedRMF/correction,applicableG1/G2,movingframes/multispanseams,continuouscontactidentity,all-modeglobalproofsorallcombinationqualification.


### Published planar RMF arc package; actual public/STEP/UI dispatch

- Solepublisher21844exit0: raw12393759 ->11021852bytes,geometrySHAd473a56675dde420dc638d116f8c079ef667e89abfbb9a301eaebbd29fd37d63. Final languagefdf unchanged. Vite32279exit0,975ms. No runtimegeometrysource mutationsafterthispublication.
- Public17185live /tmp/sweep-planar-rmf-curved-arc-public-20261005.log,expected92tests10suites. STEP57163live /tmp/sweep-planar-rmf-curved-arc-step-export-20261005.log,root external-step-planar-rmf-arc-current,expected65; independentrational/generator/OCCTmustfollowsequentiallybeforeanyqualificationclaim.
- FocusedheadedoriginalplanarRMFbody dispatched /tmp/sweep-planar-rmf-curved-arc-ui-body-20261005.log,root ui-planar-rmf-arc-body-current,1440/600. Publicsuccess/Solid/refusals/cancellation/restorationremainunverifieduntilterminalactualartifacts. Laterfocusedsurface/fullbody/fullsurfacematrixserial.
- Latestfullcore39589 andfullBRep49143 remainliveonexactnewpublishedsource. Oldcontact6517fullcore1182/BRep704/UI78cases1150/STEP64 arehistoricalandnotnewpackagegates. Contractmatrixupdatedwithnewsource theorem,pendingnewpackagequalificationandexplicitnoinversejets/generalRMF/smoothnessclaim.
- Originalsixgoalparts remainactivebeyondfinitecoordinateplanarRMFarc:generalspatial/nonaxial/closedcorrection,Bishopholonomy,movingframe/multispan/closedG1/G2,continuouscontactidentity,all-modeglobalgeometryandcomprehensivecombinationqualification.


### Planar RMF package public/full BRep/body UI gates terminal

- Public17185exit0:10suites92/92,110.97s. IncludesneworiginalRushplanarRMFsurface/body, foldedbodywithprovedE2butactualSolidrefusal,strict/refinementrefusal,ownership/cap/streamintegration. Thisfinitecasecoverage doesnotestablishall-modeglobalclaims.
- LatestfullBRep49143exit0:705passed,2existingmanualtimingignored,476.82s. Fullcore39589stillliveonexactpublishedsource; no fullnativeclaimyet.
- Focusedheadedbody92077exit0:2cases28checks1440/600,actualsuccessfulSolid. Actualmatrixartifactd473source/public/dist/worker verified. Focusedsurface45565live /tmp/sweep-planar-rmf-curved-arc-ui-surface-20261005.log;fullbody/fullsurfacefollowserially. Held-dispatchlifecyclecancellation remainsdistinctfromarbitrarynativeinterruptionlatency.
- STEP57163exit0:65cases,manifestartifactd473/publicAndPackedVerifiedtrue. Independentrational44139 at /tmp/sweep-planar-rmf-curved-arc-rational-volume-20261005.log; generator/OCCTmustfollowsequentially. No65/65claimbeforeactualOCCTartifact. Originalgeometry/control/basis/UV/volume comparisonsremainmandatory.
- Targeteddiffcheckspass. Alloriginalgoalpartsremainactivebeyond finiteopencoordinateplanarRMFarc:generalRMF/nonaxial/spatial/closedcorrection,movingframe/multispan/closedsmoothness,inverse-lengthjets,continuouscontactidentity,all-modeglobalgeometry/comprehensivecombinationqualification.


- Independentrational44139 andgeneratorreference terminalexit0. OCCT39765 at /tmp/sweep-planar-rmf-curved-arc-occt-20261005.log ispending; no65/65claimuntilterminalartifact. Focusedsurface45565exit0:2cases30checks1440/600,source-onlySolidrefusalcorrect;artifactd473source/public/dist/worker verified. CombinedfocusednewRMFbody28+surface30=58checks.
- InspectedactualnewSTEPplanarRMFcase18faces/nativeSolidRequiredtrue:boundary0.7534935266912707≤unchangedE2,filledcaps[0.5415812904397882,0.5415812904397882],correction5.144996729292706e-13. Actualretainedsections/sourcebasis/pathcurvature remainconstructorowned; independentwholeboundaryvolume/OCCTnotinferredfromnativeflags.
- FullbodyUI73497live /tmp/sweep-planar-rmf-curved-arc-ui-body-matrix-20261005.log,root ui-planar-rmf-arc-body-matrix-current,expected22cases306checks. Fullsurface60casesstillnextserialUIstage. Fullcore39589remainliveonexactd473source. No newruntimegeometry/sourceeditswhilepackagegatesrun.


### Planar RMF independent STEP matrix terminal

- OCCT39765terminalexit0:actualopencascade-sweep.json65/65overallpassedtrue,artifactd473matchingpublic/packed/manifest. NewplanarRMFcase18faces/40edges,1closedoutwardshell,2holecaps,nativevolume/materialagreementtrue,sourcebasis/controlnet/UV/full-domainwall/capcoedgeagreements true. IndependentrationalboundaryGaussianconvergence remainsnumericaloracle,notanintervaltheorem. Noall-modegeometryclaimfromthismatrix.
- Fullcore39589/fullbodyUI73497stilllive;fullsurfacependingnextserialgate. Fullobjectiveunchangedandactive.


### Planar RMF package full core/body UI terminal; bounded nonaxial plane prerequisite

- Fullcore39589exit0,1184/1184,1072.33s onexactpublishedd473source. FullBRep705pass2manualtimingignored,public92/92,STEP65/65andfocusedUI58checks remainqualifiedfinitepackage.
- Fullbody73497exit0,22cases306checks1440/600. Actualartifactd473source/public/dist/workerverified. Fullsurface34814liveat /tmp/sweep-planar-rmf-curved-arc-ui-surface-matrix-20261005.log,root ui-planar-rmf-arc-surface-matrix-current,expected60cases902checks. Keepcurrentpublic/dist stableuntilterminalUI.
- NewRustprivate source_plane prerequisite added AFTER published/core snapshot: exactdot oforiginalnonzeronormal with eachoriginalpole difference usingcad-predicatesSourceArena,exactrationalzero constant,explicitmax_work≤1m,finiteadmissioncapacity andchargedwork. No fittednormal,roundedpole subtraction,sampling orregularity/transport/seamclaim. Originalpositiveweightedrationalcontrolplaneidentity extendsacrossthewholesourcecurve. No newRMFadmissionwiredyet; mustintegrateexactworkwithowningaggregatebudgetbeforeexpandingnonaxialcontinuousBound.
- Native73457exit0,3/3,.00s at /tmp/sweep-original-nonaxial-plane-premise-native-20261005.log: nonaxialrationalplane/nonunitdomain independentBernstein equation;onebinary64step offplane givesnonzero exactpredicate;oneworkunitshort andzero budgetrefuse;constantpathcanbeplanarbutmakesnoregulartransportclaim;invalidnormal/budgetremaininputerrors. Latestsourcefullnativecountnowdiffersfromhistoric1184; no global/fullnewsourceclaimfromthese3tests.
- Fulloriginalgoal remainsactive: generalnonaxial/spatial/closedRMF/correction,inverse-lengthjets/applicableG1/G2,movingframe/multispanseams,continuouscontactidentity,all-modeglobalgeometry/comprehensivecombinationqualification.


### Exact nonaxial plane premise integration and surface UI completion

Full surface UI session34814 exited0:60 cases passed on geometry d473a56675dde420dc638d116f8c079ef667e89abfbb9a301eaebbd29fd37d63; matrix reports passed=true. This qualifies that published snapshot only.

New Rust source connects original-coefficient exact plane predicates to open arc-length RMF surface bounds, charging exact work to the same aggregate budget and subtracting it before inverse-length work. Parameter-spacing and cap-domain premises remain unchanged. No new WASM publication or nonaxial all-mode claim. Existing independent curved RMF oracle and one-cell-short/off-plane/closed-path refusal regression passed (session14276, /tmp/sweep-nonaxial-plane-integration-native-20261005.log). A new nonaxial independent geometry oracle and complete current-source gates remain required.


### Independent nonaxial RMF original-image oracle

New native regression nonaxial_planar_rmf_arc_length_encloses_independent_original_image passed (session73439, /tmp/sweep-nonaxial-rmf-independent-native-20261005.log;1/1,5.13s). Original C(u)=(u,-u,u²), constant normal(1,1,0), analytic length primitive and independent64-step inverse, explicit Bishop binormal, simultaneous scalar/affine/center/twist laws. Grid checks retained geometry against the independent image within the certified bound; this grid is an oracle regression, not the continuous theorem. One-cell-short and one-binary64-step off-plane input remain Unresolved without error_upper.

The continuous report is Certified at4009 cells but conservative E=2.5879462556746002 exceeds unchanged requested tolerance2:within_budget=false and preview accepted=false, continuous_bound=true. Tightening nonaxial interval correlation remains required; no admission tolerance was increased. No new WASM/UI/STEP claim. Nonaxial cap-domain premise remains unproved.


### Correlated proved-plane RMF values

Rust relative-value bound now uses the constant normalized original seed and B=T×N only after the owning RMF theorem has proved the original coefficient-plane identity (cheap axial identity or charged exact predicates). General FixedNormal path values and jets keep their prior projection/normalization path. The source tangent remains interval-certified nonzero. This removes redundant normalization and normal reconstruction dependency rather than relaxing geometric tolerance.

Session46214 exited0:2/2 independent RMF original-image regressions passed,10.35s, /tmp/sweep-nonaxial-rmf-correlated-native-20261005.log. Nonaxial upper=0.8854752401771256 at4009 cells versus previous2.5879462556746002 at4009, unchanged tolerance2/17 sections. The oracle also asserts preview continuous_bound=true/accepted=true, one-cell-short refusal and binary64 off-plane refusal. Edited-file git diff --check passed. Complete RMF regressions dispatched to /tmp/sweep-nonaxial-rmf-correlated-regressions-20261005.log; current source still requires broad qualification. Caps, new WASM, UI, and STEP remain outstanding for nonaxial extension.


### Nonaxial RMF endpoint material domains and shared path measure

Session18873 completed14/14 RMF regressions,103.47s, on the correlated-value source snapshot before endpoint extension. New original_frame endpoint transport owns and charges exact original-plane predicates before admitting nonaxial FixedNormal-equivalent RMF endpoint values; unproved premise retains Unresolved with charged work. Endpoint hole-domain and analytical tangent normals regression passed1/1 in0.01s; full progressive_sweep::profile_domain module passed11/11 in0.04s.

MultiSweep now measures the shared open original arc-length path once for nonaxial RMF candidates, retaining per-profile exact plane/frame work and original identity checks. Session72417 completed1/1 in20.03s, /tmp/sweep-nonaxial-rmf-shared-endpoints-native-20261005.log: eight profiles (outer/hole) preview continuous_bound/accepted, original endpoint material domains, analytic normals, one-cell-short and off-plane refusal. Edited-file diff check passed. All-RMF regression rerun dispatched to /tmp/sweep-nonaxial-rmf-endpoint-all-regressions-20261005.log. Full filled-cap boundary correction, actual native Solid volume admission, new artifact/UI/STEP qualification still outstanding.


### Nonaxial hollow body actual native admission and public integration preparation

Session93752 exited0:15/15 current RMF regressions,124.91s. New small_nonaxial_planar_rmf_arc_length_hollow_body_requires_native_volume_admission passed1/1 (session1732,8.17s); paired axial/nonaxial native BRep regression passed2/2 (session19696,12.49s), /tmp/sweep-nonaxial-rmf-body-pair-native-20261005.log. Nonaxial body18faces, two caps each with one hole; full boundary upper0.7047134918707226 within unchanged budget2, filled caps[0.35986279125960774,0.35986279125960774], explicit correction6.301388578267012e-13 within1e-9. Actual inspect_sweep volume proven with boundary embedding, nesting roles and outward orientations required by the native test. This is a finite fixture, not all-mode global proof.

Added Rush examples arc-length-curved-nonaxial-planar-rmf-progressive-sweep.r and arc-length-curved-nonaxial-planar-rmf-body-boundary.r plus existing public suite matrix entries. TS changes are fixtures/adapters/assertions only; geometry and admission remain Rust. Scoped diff check passed. Sole geometry publisher session55945 confirmed live, /tmp/sweep-nonaxial-rmf-geometry-wasm-20261005.log. New public/WASM/UI/STEP evidence not yet available; retain published d473 historical qualification boundary.


### Nonaxial published WASM, public/UI focus and independent STEP qualification

Sole publisher55945 exited0. Geometry72caaf1a6c2c496da49d2f2c5ed5ac14616a155c52f96cc01a1144c813ed38c3,11025712 bytes (raw12398510); Vite5808 exited0,872ms; vue-tsc36187 exited0. Public31897 exited0:10 suites94/94 tests,117.63s on this artifact. Focus body UI56667 passed2cases28assertions (1440/600 successfulSolid); focus surface78297 passed2cases30assertions (source-only Solid refused); both matrix artifacts verify the new geometry SHA. UI cancellation remains held-dispatch lifecycle scope.

STEP export23999 exited0,66fixtures; rational reference26040 and generator reference exited0; independent OCCT40138 exited0,66/66 passed in external-step-nonaxial-rmf-current/opencascade-sweep.json, provenance SHA matches. New nonaxial fixture18faces40edges,1closed outward shell,2holecaps: actual native volume and external material agreement, exact basis/control net, wall full-domain distance/UV preservation, cap full-domain coedge distance all passed. OCCT volume0.10067697072705874 versus independent converged rational reference0.10067697072705892, relative1.7919812266768857e-15. Numerical Gaussian convergence is independent numerical validation, not an interval integration theorem.

Current live handles confirmed: full core7592 (/tmp/sweep-nonaxial-rmf-full-native-20261005.log), full BRep14960 (/tmp/sweep-nonaxial-rmf-full-brep-native-20261005.log), full body UI94632 (/tmp/sweep-nonaxial-rmf-body-ui-matrix-20261005.log). Do not restart based on idle output. Full surface UI remains to dispatch after body terminal. Broad all-mode/global/smoothness/full continuousBound completion remains unproven despite finite qualification.


### Open nonaxial parameter-spacing RMF source extension

After the72ca published snapshot, Rust parameter-spacing RMF now admits an exact original-coefficient plane premise with charged work. Nonaxial closed/periodic paths remain refused before claiming original correspondence; general spatial input stays unresolved. Existing value and derivative trajectory bounds remain original FixedNormal projection fields, with actual RMF retained endpoint displacement and decomposition included.

Independent source helper covers both spacings on C(u)=(u,-u,u²) with joint scalar/affine/center/twist. Session5042 passed parameter oracle1/1,0.05s, upper0.021250638472303577 at962 cells. Session97002 passed2/2,5.50s after adding exact-plane closed-source refusal to both spacing cases; arc upper remains0.8854752401771256 at4009. Both assert actual preview accepted/continuousBound, sampled independent analytical image within continuous upper, one-cell-short and binary64 off-plane refusal, and closed nonaxial source Unresolved without upper. Scope diff check passed. All-RMF rerun dispatched to /tmp/sweep-nonaxial-rmf-parameter-regressions-20261005.log. This extension is not in current72ca WASM or its full native/UI qualification.

Earlier published snapshot handles7592(full core),14960(full BRep),94632(full body UI) confirmed live this turn. Full body UI completed wide-window mode cases, narrow cases pending. No replacement publisher or UI script mutations while matrix live. Full surface matrix remains undispatched until body terminal. Goal completion remains unproven.


### Published BRep gate and parameter-source refusal regression

Published72ca full BRep session14960 exited0:706passed/2ignored,489.82s, /tmp/sweep-nonaxial-rmf-full-brep-native-20261005.log. Ignores are prior manual timing tests. Full core7592 and body UI94632 still confirmed live; narrow body cases in progress.

New post-publication parameter-source RMF rerun35614 exited101:15passed/1failed in128.92s. Sole failure was stale expected reason in planar_rmf_premise_uses_original_coefficients_and_regular_tangent: represented min-subnormal off-plane pole now consumes the exact predicate budget and returns source-plane-exact-work-unproved instead of generic rmf-original-frame-correspondence-unproved. Corrected assertion still requires Unresolved, no error_upper, charged cells in(0,10000], and only exact-plane refusal reasons. Focus34846 exited0,1/1,.06s. Complete corrected RMF rerun dispatched to /tmp/sweep-nonaxial-rmf-parameter-regressions-fixed-20261005.log. No geometry tolerance/budget relaxed.

Smoothness source audit confirms analysis/span_continuity_map.rs explicitly sampled/advisory, not proof; it cannot establish requested G1/G2. Remaining proof must couple full-domain retained seam data with nonzero surface regularity and original moving-frame laws. No smoothness completion claim.


### Exact retained station seam G1/G2 audit and terminal body UI

Published72ca body UI94632 exited0:24cases334checks, all passed and matrix SHA matches. Full surface UI19655 dispatched to /tmp/sweep-nonaxial-rmf-surface-ui-matrix-20261005.log and confirmed live. Full published core7592 remains live. Corrected post-publication parameter RMF rerun49997 exited0:16/16,131.50s.

New Rust Level::certify_retained_station_seams uses existing exact projective strip-jet identity plus independently certified full-boundary nonzero normal. Degree-one station strips are literal original columns/weights/knots, without insertion/refitting; aggregate exact budget and100000-seam admission cap apply. Original patches() groups station chunks of at most31 spans per profile part. The audit checks literal consecutive chunk domain correspondence, shared station edges across chunks, and one final-to-first closure per profile part. Unsupported bases/chunk ordering and partial work remain unresolved. This is retained station-seam proof only: not original moving-frame law smoothness, profile/cap joins or whole surface regularity. No new WASM/viewport admission integration yet.

Session88383 exited0:2/2,0.10s, /tmp/sweep-retained-station-smoothness-native-20261005.log. Straight actual progressive sweep proves G2 at3internal seams and63station seams across65sections/multiple chunks; one-work-short/zero work, actual coefficient kink, and degenerate normal refuse certification. Closed actual coordinate-planar RMF circle checks32seams including cross-chunk seam and a single closure: all C0 coefficient identities, no blanketG1. Tests initially exposed mistaken per-chunk closure assumption; the audit now matches native chunk topology. Scoped diff check passed and new module rustfmt completed. This source extension follows published72ca and is not covered by its pending full core/UI gates.


### Completed72ca qualification and native retained seam transport

Published72ca full core7592 exited0:1189/1189,1120.56s. Full surface UI19655 exited0:62cases932checks, matrix passed=true/provenance SHA matches. Together full body24cases334checks and surface62cases932checks qualify86cases1266checks on72ca. This completes finite package gates (core1189, BRep706+2manual ignored, public94, STEP66, UI86), not the six-item goal or post-publication source.

Native retained seam source adds rational cubic multi-span/nonunit-weight positiveG2, one-work-short and missing actual station-chunk refusal. Initial rational-circle test exposed profile_parts retaining small multi-span profiles whole: quadratic repeated-knot rational-circle along-seam G2 needs additional geometric along-knot proof and is not supported by current sufficient strip basis checks. The cubic simple-knot test covers a basis with guaranteedC2; rational-circle blanket smoothness is not claimed.

Added Rust surface_progressive_sweep_station_seams transport operation, with requestedOrder/allStationSeamsCertified, exact work and per-seam C0/regularity/closure status. Native serialization explicitly separates retained-station-only scope from unproved source-frame/profile/cap joins and Solid. Session23677 exited0:4/4,0.21s, /tmp/sweep-retained-station-smoothness-transport-native-20261005.log; scoped diff check passed. New sole geometry publisher dispatched to /tmp/sweep-retained-smoothness-geometry-wasm-20261005.log, pending. New parameter-spacing and retained-seam source extensions still need WASM/public/UI/STEP qualification; all-mode/global/smoothness goal remains active/unproven.


### Packaged retained station seam API and parameter RMF public path

New geometry publisher15446 exited0:5f2126b49a6fe7dc3913919a5a0a483ce0291f06472993b38a051982508018a7,11033476 bytes (raw12407334). Vite90882 exited0,899ms. Types19040/14296 exited0. TS adds typed transport-only inspectProgressiveRetainedStationSeams; geometry/proof stays in Rust. Public tests cover exactG2 at63seams including station chunks, shared one-work-short/zero budget, source/Solid scope flags, and moving twist interpolation not promoted toG1.

Full public54988 exited1:96passed/1failed of97,11suites,118.39s. Sole failure was stale min-subnormal plane refusal reason assertion; corrected to exact-plane refusal while retaining continuousBound=false/errorUpper=null. Focus78063 exited0:8/8 across2suites,1.32s, including both new seam API cases and corrected6 planar cases. Other96 complete-run cases unchanged/passed; no claim of a single fully green11-suite run after assertion correction.

Added nonaxial-planar-rmf-progressive-sweep.r parameter Rush example (native upper0.021250638472303577/requested0.05). Focus UI77839 exited0:2cases26checks on1440/600, correct source-onlySolid refusal; matrix verifies5f212 artifact. Scoped diff check passed.

Current snapshot broad gates dispatched: core60325 (/tmp/sweep-retained-smoothness-full-native-20261005.log), BRep45035 (/tmp/sweep-retained-smoothness-full-brep-native-20261005.log), STEP export24897 (/tmp/sweep-retained-smoothness-step-export-20261005.log), full body UI60464 (/tmp/sweep-retained-smoothness-body-ui-matrix-20261005.log). Independent references/OCCT wait for exporter terminal; full surface UI waits for body terminal. No new publisher/UI source mutation while UI live. General rational repeated-knot along-profile G2, source-frame smoothness, spatial/closed nonaxial RMF and all-mode geometry guarantees remain outstanding.


###5f212 STEP terminal, UI loader correction, native projective along chains

STEP exporter24897 exited0; rational reference44539 and generator reference exited0; OCCT37344 exited0:66/66 passed, artifact provenance5f2126b49a6fe7dc3913919a5a0a483ce0291f06472993b38a051982508018a7 verified in external-step-retained-smoothness-current/opencascade-sweep.json.

Initial full UI body60464 exited1 before any sweep load:30000ms timeout on exact accept=.scad,.r,text/plain; failure screenshot/body text show healthy loaded app, pageErrors=[], zero assertions. Compiled dist detect chunk exposes .scad,.r,.mg,text/plain. Source-loader selector now requires file type and both.scad/.r support instead of an exact format list. This changes qualification automation, not application geometry. Failed artifact preserved in ui-retained-smoothness-body-matrix-current. Retry74588 dispatched into ui-retained-smoothness-body-matrix-retry-current, confirmed progress through body mode cases; no UI source mutation while live.

Post5f212 Rust projective strip jet predicate now supports original full-multiplicity Bezier chains with exact along-direction joins. Source pieces copy literal original poles/weights and knot endpoints, without rounded extraction/refitting. Each surface owns exact projective G1/G2 along every internal chain boundary and explicit coefficient end-coincidence closure; independently certified nonzero normals required. Child predicate work consumes the same aggregate exact budget before the cross-seam predicate; regularity bypass of basisC1 is allowed only after those exact along-jet proofs. Generic homogeneous mode unchanged; mixed/unsupported/periodic basis stays unresolved.

Native6253 passed5/5 prior exact-strip regressions,.09s. New circle34520 passed1/1,.03s: actual retained rational circle profile G2, whole profile chain plus closure, one-work-short refusal, one-binary64-step coefficient mutation refusal while retaining stationC0. Complete retained_smoothness module passed5/5 ( /tmp/sweep-projective-along-chain-smoothness-regressions-20261005.log). Scoped diff check passed. These native along-chain extensions are not in5f212 publication or its ongoing broad gates. Full core60325/BRep45035 remain live snapshot handles; body UI retry74588 live; full surface UI awaits retry terminal. Goal remains unproven.

### 2026-10-05 continuation: native terminals and current artifact boundary

- Earlier 5f212 source snapshot: core suite terminal 1194 passed, 0 failed (1110.50s), `/tmp/sweep-retained-smoothness-full-native-20261005.log`; BRep terminal 706 passed, 2 ignored, 0 failed (495.38s), `/tmp/sweep-retained-smoothness-full-brep-native-20261005.log`. These results do not qualify subsequent source edits.
- Body UI retry: 15 cases passed; narrow guided case failed during screenshot capture after six assertions passed, with no page errors. This is not a complete UI matrix pass. Log `/tmp/sweep-retained-smoothness-body-ui-matrix-retry-20261005.log`.
- Shared public/packed WASM changed externally to 7cf7473bece776219dbebebde627b115b009c0bd290fa3d0d6b53fbfca818537. Focused retry correctly refused mismatched older dist. Vite rebuild now terminal success (907ms), `/tmp/sweep-current-7cf-vite-20261005.log`. Inclusion of latest native source changes in 7cf is unverified. Isolated current-artifact guided narrow UI retry dispatched; outcome pending.
- New original_smoothness.rs sufficient open source-frame C1/C2 proof module: focused native tests 2/2 passed, `/tmp/sweep-original-frame-smoothness-native-20261005.log`. Not yet serialized or integrated through WASM/Rush/UI. Closed seams, spatial RMF correspondence, corrected Frenet and guided arc correspondence remain unproved. No whole-goal completion claim.

### Original frame transport continuation

- Matched 7cf guided narrow UI terminal success: 600px, 14 checks, `/tmp/sweep-current-7cf-guided-narrow-ui-20261005.log`. This focused rerun does not establish the whole body matrix.
- Added MultiSweep shared-frame wrapper and native operation `surface_progressive_sweep_frame_smoothness`, with explicit open-original-frame-only scope; retained seams, profile/cap joins, continuousBound and Solid flags remain false.
- Native transport-enabled original_smoothness tests terminal 3/3, including positive C2, zero cell budget, invalid order and closed source refusal: `/tmp/sweep-original-frame-transport-native-20261005.log`.
- Thin TS adapter added; vue-tsc terminal success, `/tmp/sweep-original-frame-adapter-types-20261005.log`. New packaged-WASM regression `tests/nurbsOriginalFrameSmoothness.test.ts` is written but not yet executed. Geometry WASM build is running, `/tmp/sweep-original-frame-wasm-20261005.log`; no publication or integration completion claim yet.

### Frame package preparation: adjacent regression scope

- Expanded packaged-WASM tests cover authored frame positive C2, internal normal-law knot discontinuity, parameter-guide positive proof and guided arc correspondence refusal. Test execution remains pending the active package optimizer.
- Expanded vue-tsc check passed, `/tmp/sweep-original-frame-expanded-types-20261005.log`.
- Current native retained_smoothness regressions with transport passed 5/5, including rational circle along-chain and closure jets, `/tmp/sweep-frame-package-retained-native-20261005.log`.
- Geometry release compilation terminal success (1m14s); optimizer process observed running with CPU activity. No timeout-based restart and no assertion that the new public bytes are already qualified. Contract matrix now documents open-frame-only API and unresolved families.

### Curved original frame regressions

Current native original_smoothness module now passes 4/4 with transport, `/tmp/sweep-original-frame-curved-native-20261005.log`: curved FixedNormal and Frenet C2 checked for parameter and arc-length station spacing, with whole-cover zero-budget refusal. The additional edit is test-only after release snapshot compilation. Public counterpart tests are written, and expanded vue-tsc passed (`/tmp/sweep-frame-public-expanded-types-20261005.log`); public results remain pending ongoing WASM optimization. No general closed/source-spatial correspondence or retained smoothness claim follows from these finite fixtures.

### Original frame packaged public checkpoint

Geometry packaging terminal success, SHA256 053949e57c6fdce1eea656bef132cf5a3ea19fa52e205142b39549bc01c566f6; public and generated bytes identical. Raw12421842 -> optimized11046536 bytes. Log `/tmp/sweep-original-frame-wasm-20261005.log`. Public original-frame, retained-station and planar-RMF regressions passed 11/11 in three files (1.40s), `/tmp/sweep-original-frame-public-20261005.log`. Includes authored/frame knot discontinuity, guides/correspondence refusal, curved FixedNormal/Frenet both spacings, scope and budget refusals. This proves the new API is callable through packaged WASM; it does not prove Rush/viewport presentation integration or whole-goal completion. Current core full serial test remains live, `/tmp/sweep-original-frame-full-core-20261005.log`; Vite rebuild dispatched, `/tmp/sweep-original-frame-vite-20261005.log`.

### Frame report and viewport integration

TS transport glue attaches the Rust sourceFrameSmoothness report to progressive preview/result levels; no numerical proof or admission is implemented in TS. Viewport reads the original frame C2 report only with matching method/order/open-frame scope, valid cell/exact-work bounds and explicit negative retained/profile/cap/error/Solid scope flags. App displays a separate C2 original-frame line.

Initial report fixture failed because its closed quadratic had zero tangent, then because closed circle initial_sections=3 was invalid. Fixture corrected to a regular circle with five initial sections; no production geometry relaxation. Final API+viewport scope regressions 15/15, `/tmp/sweep-frame-viewport-scope-20261005.log`; vue-tsc passed earlier after viewport source edits. Vite rebuild terminal success, `/tmp/sweep-frame-viewport-vite-20261005.log`. Headed nonaxial parameter surface UI dispatched at1440/600 with explicit C2 presentation assertion; `/tmp/sweep-original-frame-c2-ui-20261005.log`, outcome pending. Core full serial suite remains running.

### Original C2 headed viewport checkpoint

Headed nonaxial parameter-RMF surface UI terminal success at1440 and600: 2cases/28checks, `/tmp/sweep-original-frame-c2-ui-20261005.log`. Both include explicit separate original-frame C2 presentation, source-only Solid refusal, cancellation/source replacement/restoration. Current artifact053949e5 provenance verified by harness. This is a focused family result, not the full UI matrix. Full surface matrix now dispatched, `/tmp/sweep-original-frame-surface-ui-matrix-20261005.log`; core full serial remains live.

Guided arc smoothness remains unproved: existing original-frame arc error implementation supplies independent inverse-length brackets and relative value enclosures, but no whole-domain C2 frame theorem is exposed yet. Do not reuse the same-parameter guided cover as a proof for independent arc inversions.

### Guided arc original Ck source proof (native only)

Added independent Cartesian path/guide/twist frame value cover in Rust, recursively covering the entire product domain with charged shared work. Native helper regression passed, including one-work-short and coincident rail refusal. Original frame smoothness arc branch now composes original knot Ck requirements, whole path/guide positive-speed covers, the inverse function theorem for separate normalized-length maps, and full independent frame nondegeneracy. No inverse derivative bounds or retained/Surface/Solid certificates are inferred. Native original_smoothness regressions 5/5, `/tmp/sweep-guided-arc-frame-smoothness-native-20261005.log`; includes positive line-guide C2, one-cell-short and zero-speed guide refusal.

This is a sufficient general product-domain criterion; failure may require tighter correlated arc covering and is not singularity evidence. Latest source is NOT included in published053949e5, which retains prior guided-arc refusal. Running full core/UI qualify earlier snapshots; no republishing while UI matrix is live. Public guided-arc expectation must update only with replacement WASM qualification.

### Full surface UI053949 terminal and curved guided arc source checkpoint

Full headed surface matrix terminal success:64cases/960checks at1440/600, all case states passed, geometry SHA053949e57c6fdce1eea656bef132cf5a3ea19fa52e205142b39549bc01c566f6. Artifact `ui-original-frame-surface-matrix-current/matrix.json`, log `/tmp/sweep-original-frame-surface-ui-matrix-20261005.log`. Scope remains finite surface scenarios with lifecycle cancellation, not mid-kernel interruption latency or body proof.

Guided arc C2 native transport tests now6/6, including curved path/rail with different knot domains and distinct analytic length images, short-work and crossing/zero-speed refusal. `/tmp/sweep-guided-curved-arc-frame-transport-native-20261005.log`. Replacement WASM publisher dispatched only after UI terminal, `/tmp/sweep-guided-arc-frame-wasm-20261005.log`; new public arc-positive/curved/budget/speed tests written but pending replacement bytes. Full core serial run still live and qualifies an earlier compiled snapshot.

### Rational guided arc original frame qualification preparation

Native curved guided arc regression extended to a rational rail with middle weight0.5; positive C2 and one-cell-short refusal passed, `/tmp/sweep-guided-rational-arc-frame-native-20261005.log`. Public counterpart added but not yet executed against replacement bytes. New source-frame theorem explicitly excludes retained and Solid guarantees.

Replacement publisher release compilation terminal success (1m24s); same optimizer observed live with CPU activity, `/tmp/sweep-guided-arc-frame-wasm-20261005.log`. Full core continues through expensive shared arc tests; old and current compiled snapshots are not conflated. Contract matrix now records053949 finite UI qualification and native-only guided arc sufficient product criterion.

### Original frame core terminal and guided UI preparation

Full core serial suite terminal success1199/1199,0failed,1141.14s, `/tmp/sweep-original-frame-full-core-20261005.log`. This run compiled before independent guided arc source changes; those additions have focused6/6 plus separate rational regression, not a substituted broad gate. Replacement publisher remains live in optimizer with observed CPU activity.

Added headed C2 presentation assertions for `arc-length-guided-progressive-sweep.r` and `arc-length-curved-guided-progressive-sweep.r`; execute only after replacement WASM and public proof qualification. Earlier053949 surface64/960 results remain historical and do not qualify the changed assertions.

Internal guide-knot discontinuity regression passes; source-path tangent discontinuity is already refused by Sweep constructor. Initial test wrongly expected construction of a kinked path; corrected to assert the existing early refusal without production relaxation. Native original_smoothness6/6 final log `/tmp/sweep-guided-arc-knot-frame-native-retry-20261005.log`.

### Closed retained seam public checkpoint

Published053949 retained station seam tests now3/3, `/tmp/sweep-closed-retained-public-20261005.log`. Actual closed RMF circle retains32 C0 joins and exactlyone closure; no blanket G1/source-frame/Solid certificate. The checked public hash remains053949e57c6fdce1eea656bef132cf5a3ea19fa52e205142b39549bc01c566f6.

Replacement guided-arc publisher remains live in optimizer with CPU activity; no restart based on elapsed time. New guided arc public/viewport results remain pending replacement bytes, separate from the closed seam result above.

### Guided arc original frame published checkpoint

Replacement geometry WASM terminal success: SHA a3575a0b6a28a5b94db8d92cb8b4323a574e96033fb699aa80c92d754bd5bce3, public/generated identical,11048221bytes optimized from12423702. `/tmp/sweep-guided-arc-frame-wasm-20261005.log`. Public frame/retained/planar/viewport25/25 in4files passed1.39s, `/tmp/sweep-guided-arc-frame-public-20261005.log`, including curved/rational guided arc C2, shared budget/speed refusal and closed C0 scope. Final vue-tsc and Vite rebuild terminal success, respective `/tmp/sweep-guided-arc-final-types-20261005.log` and `/tmp/sweep-guided-arc-frame-vite-20261005.log`.

Headed curved guided arc surface UI at1440/600 dispatched with separate C2 assertion, `/tmp/sweep-guided-arc-frame-c2-ui-20261005.log`, outcome pending. Full current core serial dispatched for newly compiled guided arc source, `/tmp/sweep-guided-arc-frame-full-core-20261005.log`;1199 earlier result is not silently reused. All-mode global/smoothness and complete boundary requirements remain open.

### Guided arc C2 headed viewport terminal

Matched a3575a0b geometry headed curved-guided arc UI terminal success at1440/600:2cases/32checks, `/tmp/sweep-guided-arc-frame-c2-ui-20261005.log`. Explicit original-frame C2 presentation passed, while surface-only Solid refusal and lifecycle actions remained correct. This closes the finite curved guide Rust→WASM→Rush→viewport presentation case, not all-mode surface/solid smoothness.

Current independent STEP export dispatched to `external-step-guided-arc-frame-current`, `/tmp/sweep-guided-arc-frame-step-export-20261005.log`; export outcome, independent rational/generator volume and OCCT results pending. Manifest mutations must remain sequential. Full current core serial remains live, `/tmp/sweep-guided-arc-frame-full-core-20261005.log`.

### a3575a0b public and independent STEP terminal checkpoint

Expanded public sweep/miter/viewport matrix terminal121/121 in13files (51.17s), `/tmp/sweep-guided-arc-frame-public-matrix-20261005.log`. Independent STEP export, rational boundary volume, original generator volume and OCCT commands terminal success sequentially. Actual `external-step-guided-arc-frame-current/opencascade-sweep.json` confirms passed=true,66cases, geometry SHA a3575a0b6a28a5b94db8d92cb8b4323a574e96033fb699aa80c92d754bd5bce3, publicAndPackedVerified=true. Logs `/tmp/sweep-guided-arc-frame-{step-export,rational-volume,generator-volume,occt}-20261005.log`.

Independent volume convergence is numerical evidence, not a continuous-error theorem. OCCT scope explicitly fixture import/topology/analytic volume; global surface/seam/containment scope remains separate. Full body-boundary headed UI dispatched at1440/600, `/tmp/sweep-guided-arc-frame-body-ui-matrix-20261005.log`; core serial remains live. Whole-goal completion is not established.

### Native closed authored source-frame C2 (unpublished)

New original Bezier homogeneous endpoint jet predicate checks C0/C1/C2 by exact signs of original binary64 weight-coordinate products; common degree/domain factors cancel. Full proof is charged by original product terms (56 for degree5 XYZ+weight C2). Single-bit derivative mutation and subnormal product residual refuse; one-work-short refuses. Native endpoint module2/2 passed, `/tmp/sweep-original-endpoint-jets-native-retry-20261005.log`. Initial extreme fixture violated existing coordinate/weight bounds and was replaced with valid large-cancellation/subnormal inputs; no validation relaxation.

New separate `certify_closed_authored_frame_smoothness` proves a sufficient closed original frame field only for Fixed authored orientation without guide/holonomy correction. Requires all original axis/normal/twist endpoint jets and whole-domain nondegeneracy. Moving degree5 axis fixture passed1/1, `/tmp/sweep-closed-authored-frame-c2-native-20261005.log`, including short cell/exact work, one-bit endpoint-jet failure and independent old open-API refusal. Does not certify closed path/trajectory, retained seams, surfaces or Solid. Native source only; transport/WASM/Rush/UI integration pending. Existing a3575 UI/core runs qualify earlier compiled snapshots.

### a357 body UI terminal; closed authored transport preparation

Actual `ui-guided-arc-frame-body-matrix-current/matrix.json`:passed=true,24cases/334checks, SHA a3575a0b6a28a5b94db8d92cb8b4323a574e96033fb699aa80c92d754bd5bce3. Log `/tmp/sweep-guided-arc-frame-body-ui-matrix-20261005.log`. Finite constructor-owned body-boundary UI only; no all-mode completion claim.

Separate native operation `surface_progressive_sweep_closed_frame_smoothness` and shared MultiSweep wrapper added. Positive closed moving authored C2, exact-budget refusal, explicit path/retained/Solid false flags and C1-positive/C2-negative derivative mutation pass focused native test; `/tmp/sweep-closed-authored-frame-orders-native-20261005.log`. Thin TS adapter and `tests/nurbsClosedAuthoredFrameSmoothness.test.ts` added. Replacement publisher dispatched after body UI terminal, `/tmp/sweep-closed-authored-frame-wasm-20261005.log`; public test pending new bytes. Original general closed, RMF holonomy, corrected Frenet and retained/frame correspondence remain independent unresolved obligations.

### Closed frame presentation preparation and rational arc native checkpoint

Previous guided arc snapshot full core terminal1202/1202,1114.38s, `/tmp/sweep-guided-arc-frame-full-core-20261005.log`; this does not qualify subsequent closed API source additions.

Closed authored C2 now attached by TS transport glue only for native closedPath+authored reports; UI uses a separate closed-source-frame scope and never changes Solid admission. Viewport malformed/scope/budget/independent-flag tests12/12 pass, `/tmp/sweep-closed-frame-viewport-scope-20261005.log`; viewport types passed. New Rush fixture `examples/rush/authored-closed-c2-progressive-sweep.r` and headed separate-presentation assertion written, not yet runtime-qualified.

Native closed moving-frame test extended to arc-length spacing, independent knot domain[-3,7] and nonuniform rational weights with exact homogeneous endpoint jets; positive C2 and short exact budget pass, `/tmp/sweep-closed-authored-rational-arc-native-20261005.log`. Public counterpart written; replacement WASM optimizer still live, `/tmp/sweep-closed-authored-frame-wasm-20261005.log`. No new public/UI success claim before replacement execution.

### Closed authored frame packaged public checkpoint

Publisher terminal success, geometry SHA3eb0faa68a222d0b8148c87ea205f30517cd9d70c9a29790a7c6d77b3ead5361; public/generated identical,11052975optimized bytes from12428987. `/tmp/sweep-closed-authored-frame-wasm-20261005.log`. Public closed/open/retained/viewport22/22 in4files passed1.14s, `/tmp/sweep-closed-authored-frame-public-20261005.log`. Includes exact endpoint C2, C1-positive/C2-negative, rational arc and independent domain, shared budget and preview scoped report. Preview preserves old open-frame refusal separately.

Rush frontend new degree5 closed fixture compilation passed, `/tmp/sweep-closed-authored-rush-compile-20261005.log`; current preview types passed. New endpoint module formatted alone; no unrelated source formatting. Vite rebuild dispatched, `/tmp/sweep-closed-authored-frame-vite-20261005.log`; headed closed C2 presentation remains pending. Whole goal and all-mode closed seams remain unproved.

### Closed authored C2 headed UI terminal

Actual matched3eb0faa6 closed authored UI matrix terminal passed=true,2cases/28checks at1440/600. Log `/tmp/sweep-closed-authored-frame-c2-ui-20261005.log`, artifact `ui-closed-authored-frame-c2-current/matrix.json`. Explicit closed source-frame C2 line passes; source-only Solid refuses and cancellation/source replacement/restoration remain correct. This establishes the finite moving authored closed frame Rust→WASM→Rush→viewport case, not closed surface/trajectory/Solid smoothness.

Current full core serial dispatched, `/tmp/sweep-closed-authored-frame-full-core-20261005.log`; source endpoint module was formatted alone and only focused/native/public proofs are terminal for this snapshot. Contract matrix updated with exact Bezier endpoint/regularity premises, product/cell budgets and remaining multispan/holonomy/general closed limitations. Whole objective remains open.


### Original clamped multispan authored-frame endpoint jets (2026-10-05)

Added `cad-predicates::rational_bspline_endpoint_jet_identity` and connected it to the native closed authored-frame certificate for original clamped multispan laws. Exact expansion identities include the original endpoint knot widths; no rounded derivative or extracted Bezier coefficient is an authored premise. The existing Bezier product-term fast path keeps its accounting; multispan proofs charge the predicate expansion work under the same caller budget. Whole-law basis continuity and whole-frame nondegeneracy remain required by the caller.

Validation: cad-predicates 24/24 (`/tmp/sweep-bspline-endpoint-all-20261005.log`); closed authored native test 1/1 (`/tmp/sweep-multispan-closed-frame-native-20261005.log`), including a four-span C2 positive, one-bit C2 refusal/C1 preservation and one-work-short refusal; endpoint-focused native tests 4/4 (`/tmp/sweep-multispan-endpoint-native-20261005.log`). Predicate tests independently reject altered endpoint knot parameterization and zero budget.

This change is native-only at this point: published WASM remains 3eb0faa68a222d0b8148c87ea205f30517cd9d70c9a29790a7c6d77b3ead5361, so its earlier UI evidence does not qualify this new multispan branch. The earlier full-core handle 6614 is still confirmed live and compiled the preceding source. No path-seam, retained-surface, continuousBound or Solid claim follows from this source-frame predicate.


### Multispan WASM qualification preparation (2026-10-05)

Added a public transport/preview regression with original four-span cubic laws: positive C2, one-bit C2 refusal with C1 preservation, changed knot-width refusal and shared-budget exhaustion. Added `authored-closed-multispan-c2-progressive-sweep.r` to the existing headed surface matrix, including the separate closed-frame C2 label assertion at both widths. `vue-tsc --noEmit` passed (`/tmp/sweep-multispan-closed-frame-types-20261005.log`). New unequal-end-span positive exact predicate regression passed, alongside all cad-predicates tests:25/25 (`/tmp/sweep-bspline-asymmetric-endpoint-all-20261005.log`).

WASM publisher handle23618 is confirmed live: Rust release compilation finished in2m31s, optimization/packing remains pending (`/tmp/sweep-multispan-closed-frame-wasm-20261005.log`). Public/viewport qualification of this new package has not run yet. Previous full-core handle6614 remains live, so no full-suite completion is claimed.


### Native multispan JSON boundary validation (2026-10-05)

The closed authored native regression now explicitly dispatches the original four-span cubic through `surface_progressive_sweep_closed_frame_smoothness`. It checks positive C2, identical exact-work accounting, the closed-original-authored-frame-only scope, false path/retained/error/Solid claims and a one-work-short refusal. Passed1/1 (`/tmp/sweep-multispan-closed-frame-transport-20261005.log`). This additional change is test-only; the live WASM publisher23618 does not need restarting. Publisher23618 and preceding full-core6614 are both confirmed live; no public package or full-suite success is inferred from waiting.


### Whole-law and rational multispan regressions (2026-10-05)

Added a negative with identical exact endpoint C2 jets but an internal cubic knot of multiplicity3. The endpoint owner certifies the seam; the composed source-frame certificate correctly refuses with closed-authored-law-knot-continuity-unproved. Added a positive original rational multispan axis with nonuniform interior weights and independent knot domain[-3,5]. Native closed authored regression passes both (`/tmp/sweep-multispan-closed-frame-global-knot-20261005.log`, `/tmp/sweep-multispan-closed-frame-rational-20261005.log`). Public equivalents are added but remain unexecuted pending publisher23618. Confirmed native test edits after publisher dispatch are cfg(test)-only and do not change the pending package implementation. Publisher23618 and preceding full-core6614 still live; no restart or completion inferred.


### Prior closed authored full native suite terminal (2026-10-05)

Handle6614 completed successfully:1205/1205 tests,1174.34s (`/tmp/sweep-closed-authored-frame-full-core-20261005.log`). This binary predates the multispan endpoint predicate; its full-suite success qualifies the prior closed Bezier authored-frame implementation, not the new multispan branch. Started a fresh current-source serial core suite (`/tmp/sweep-multispan-closed-frame-full-core-20261005.log`). WASM publisher23618 remains confirmed live in Binaryen optimization; no restart, package success or UI success is inferred.


### Published original multispan closed frame qualification (2026-10-05)

Publisher23618 completed0. Optimized11061382bytes from12438638bytes. Public and generated WASM SHA256 both f3ff638d8cf96f55ac9f98f011a7852ffa0b06ec15a03cbb872e2a6524ef2af4. Public closed/open source-frame and viewport suites20/20 in3files,1.14s (`/tmp/sweep-multispan-closed-frame-public-20261005.log`), including four-span C2, one-bit C2 refusal/C1 preservation, nonuniform weights with independent domain, internal C0-knot refusal and knot-width/budget refusals. Vite build terminal0. Headed browser1388 terminal0:1440 and600 widths,2cases/28checks, separate closed source-frame C2 line and independent Solid refusal (`ui-closed-multispan-c2-current/matrix.json`, `/tmp/sweep-multispan-closed-frame-ui-20261005.log`). This confirms the new original-frame predicate through native/WASM/Rush/viewport, not full path/surface/Solid closure. Current-source full-core5531 remains live. STEP and full UI matrices have not yet been rerun on f3ff638d.


### Current-package broad qualification dispatched (2026-10-05)

On published geometryf3ff638d, started STEP export handle10533 (`/tmp/sweep-multispan-frame-step-export-20261005.log`, `external-step-multispan-frame-current`) and full headed surface UI handle39052 (`/tmp/sweep-multispan-frame-surface-ui-matrix-20261005.log`, `ui-multispan-frame-surface-matrix-current`). Both are confirmed live; native current-source full-core5531 also remains live. STEP volume references and OCCT verification must follow sequentially after export. These runs are compatibility/finite-matrix evidence, not a replacement for the all-mode geometry objective. Filled-cap composition remains conditional on original material domains, exact retained regions, nonzero projection-normal intervals, endpoint/correction/decomposition bounds; endpoint contour or frame smoothness proof alone supplies none of the missing global premises.


### Independent multispan STEP matrix expansion (2026-10-05)

Current-package STEP export10533 completed0. Rational-boundary reference19405 and generator-volume reference completed0 (`/tmp/sweep-multispan-frame-rational-volume-20261005.log`, `/tmp/sweep-multispan-frame-generator-volume-20261005.log`). OCCT verification54725 dispatched and live (`/tmp/sweep-multispan-frame-occt-20261005.log`). Full headed surface39052 and native full-core5531 remain live.

Added a genuinely new original rational cubic multispan authored-frame body to export-sweep-step-oracle.mts: simple internal knots, nonuniform interior weight, moving axis, original circular material profile and straight path. A direct native constructor probe returns34 faces, accepted retained-original boundary bound1.3867931420292194 under budget2, with endpoint contour bounds3.4389012577091574e-14. This does not prove global Solid; the fixture keeps globalEmbeddingCertified independently and does not requireNativeSolid. Its independent reference integrates the retained rational boundary and filled outward cap contours. Expanded full export92943 dispatched to separate external-step-multispan-expanded-current, preserving the current66-case oracle files being verified. Expanded export/reference/OCCT results are pending.


### Current f3ff638d independent STEP terminal (2026-10-05)

OCCT verifier54725 completed0. Inspected external-step-multispan-frame-current/opencascade-sweep.json:passed=true,66cases,geometryWasmSha256=f3ff638d8cf96f55ac9f98f011a7852ffa0b06ec15a03cbb872e2a6524ef2af4,publicAndPackedVerified=true. This preserves existing frame/guide/affine combinations on the new exact-endpoint implementation. New67-case export92943 remains live and has not emitted its manifest, so no new multispan STEP success is claimed. Full surface39052 and core5531 confirmed live; surfaced guided/contact narrow-window cases have passed, but no overall completion is inferred.


### Expanded multispan STEP and successful Solid UI terminal (2026-10-05)

Expanded export92943, rational-boundary reference93202, generator reference78748 and OCCT22857 all completed0. Inspected external-step-multispan-expanded-current/opencascade-sweep.json:passed=true,67cases onf3ff638d. New original-rational-multispan-authored-frame.step:valid,34faces,1solid/closed oriented shell,68manifold edges with opposite uses, native volume certified and native/external agreement. Volume7.689589530637569 vs independent reference7.689589529449573; relative error1.5449412005626726e-10. Exact shared wall basis and full-domain wall/edge/coedge preservation checked. This is fixture evidence, not all-mode theorem.

Full surface39052 completed0:matrixpassed=true,68cases/1020checks at1440/600 with exact current-package provenance. Added authored-multispan-body-boundary.r; focused headed UI54939 completed0:2cases/28checks, successful Solid publication/restoration, build/Solid cancellation, source replacement, strict boundary-budget refusal and invalid-source/bounded-work refusal.

Broad body UI86324 failed1 after the first14-check positive due page.screenshot timeout30000ms; no mathematical assertion failure was reported and the broad matrix is not passed. Dispatched a terminal-failure retry in separate ui-multispan-frame-body-matrix-retry-current. Current-source full-core5531 still confirmed live.


### Qualification diagnostics and mandatory multispan Solid gate (2026-10-05)

The headed matrix failure handler now saves the primary error before fallible body-text/screenshot diagnostics and catches diagnostic failures separately. node --check passed. This prevents a failure screenshot timeout from replacing the actual failing action; ordinary preview/solid screenshot failures still fail qualification. Live retry57973 had loaded its script before this change and remains unchanged, with the first three wide-window bodies passed.

The new multispan STEP exporter fixture now requiresNativeSolid:true for future exports instead of merely recording its result. Explicit assertions on the currently retained fixture prove its nativeVolume.solidGeometryCertified=true and the independent OCCT case passed/native_external_material_agreement=true. The current67-case manifest predates the mandatory gate field, but its actual native and oracle evidence satisfies the gate; no regenerated manifest is claimed. Full-core5531 and body retry57973 remain confirmed live.


### Open source-frame C2 presentation consistency (2026-10-05)

Viewport now requires reason=null as well as the exact positive open-frame C2 report and independently negative retained/cap/error/Solid scope flags. Previously a contradictory positive report carrying an unproved reason could be displayed as proved. Regression cases explicitly reject non-null and missing reasons; sweepViewportEvidence tests12/12passed (`/tmp/sweep-source-frame-c2-reason-viewport-20261005.log`). Geometry remains Rust; this is presentation validation only. Current dist remains unchanged while body retry57973 is live. Native full-core5531 remains live; UI/core completion is not inferred.


### Current multispan full suite terminal and projective endpoint extension (2026-10-05)

Current-source full core5531 completed0:1205/1205,1138.82s (`/tmp/sweep-multispan-closed-frame-full-core-20261005.log`). This qualifies the equal homogeneous endpoint-jet multispan implementation published asf3ff638d. After this terminal result, extended the multispan endpoint predicate to permit one positive constant homogeneous seam scale: original endpoint weights cross-multiply H0,H1,H2 identities, preserving Cartesian jets without rounded derivatives. This is sufficient constant-projective equality, not general varying projective gauge. The Bezier fast path remains strict equal jets.

Native closed authored test43654 passed1/1, including endpoint weight ratio2 positive, one-work-short refusal, one-bit C2 refusal/C1 preservation. Predicates58351 passed25/25. Added public equivalents; they require the next WASM publication and have not run onf3ff638d. Live body UI57973 continues on its existing immutable dist/package; source viewport reason validation is also not rebuilt into that dist. No live UI bundle mutation or all-mode smoothness claim.


### Projective original Bezier endpoint support (2026-10-05)

Original Bezier laws with unequal endpoint weights now route to the same exact clamped endpoint predicate; equal-weight endpoints retain the existing original-product fast path and56-term example budget. A degree5 law with endpoint weight ratio2 certifies Cartesian C2 under one constant homogeneous seam scale; one-bit second-jet damage refusesC2/preservesC1 and one-work-short refuses. Endpoint native50673 passes5/5 and composed closed-frame8138 passes1/1 (`/tmp/sweep-projective-bezier-endpoint-native-20261005.log`, `/tmp/sweep-projective-bezier-frame-native-20261005.log`). Public Bezier and multispan projective fixtures added, pending next WASM publication. This remains sufficient constant-projective equality, not all varying-gauge rational endpoint identities. UI retry57973 remains live onf3ff638d; no package/dist mutation occurred.


### Broad body UI terminal and projective publisher dispatch (2026-10-05)

Body retry57973 completed0. Inspected matrixpassed=true,26cases/362checks at1440/600,geometryf3ff638d provenance. It includes new rational multispan authored body success plus the existing constructor-boundary modes, refusals, cancellation, source replacement and restoration. The first screenshot-timeout run remains failed and preserved; this complete retry provides the broad evidence.

After all UI handles became terminal, dispatched the projective original-frame WASM publisher (`/tmp/sweep-projective-frame-wasm-20261005.log`). Current-source full core11171 also started (`/tmp/sweep-projective-frame-full-core-20261005.log`), since the exact projective predicate is a new production change. Both pending. Dist rebuild and projective public/Rush checks must use the new published fingerprint; existingf3ff638d matrices qualify their own source revision only.


### Projective native Rush source preservation (2026-10-05)

Added authored-closed-projective-c2-progressive-sweep.r to the surface headed matrix and separate closed-C2 label assertion. Native compileRushFrontend emits the actual progressive_sweep graph op with authored orientation, original cubic knot sequence and exact original weights[1,1,1,1.5,2,2,2]; explicit graph assertions passed. Initial inspection used an incorrect surface_progressive_sweep graph-op name; corrected to the observed native progressive_sweep op without changing the example. Browser syntax check passes. This is frontend source-preservation evidence, not runtime projective geometry proof. Publisher92336 and full-core11171 confirmed live. Their logs remain /tmp/sweep-projective-frame-wasm-20261005.log and /tmp/sweep-projective-frame-full-core-20261005.log. Next published public tests and headed projective C2 checks remain pending.


### Projective original weight-jet rejection regression (2026-10-05)

Added a direct cad-predicates test of the positive original endpoint weight ratio2 and one-bit weight damage. Modifying the penultimate second-jet weight refusesC2 while preservingC1; modifying the first-jet weight refusesC1. A one-work-short exact budget returns Indeterminate. All predicates26/26 passed (`/tmp/sweep-projective-endpoint-weight-jets-predicates-20261005.log`). Matching public weight-jet refusal added pending publication. These new changes are tests only; publisher92336 release compilation completed in1m18s and is still live optimizing/packing. Full-core11171 remains live and compiled the same production implementation. No package success is inferred.


### Closed Fixed authored retained-original bound extension (native, 2026-10-05)

Inspection found the authored Parameter section-bound path unconditionally refused all closed paths, despite Fixed authored construction having zero holonomy correction. The existing proof compares every actual retained section to original source values includingt=1; the copied closing section discrepancy is included in the final interpolation cell. Knot crossings already use original value-image/retained-segment bounds rather than an unjustified global second-derivative remainder. Restricted the removed blanket guard to Fixed only; all other closed correction modes still refuse.

Added a closed rational-circle path with moving cubic authored axis regression. At17sections the proof was Certified but its rigorous upper2.116980848037562 exceeded the unchanged budget2; the initial within-budget assertion failed. Refined to33sections instead of relaxing tolerance. The refined native test passes, verifies the actual copied first/last sections and rejects a one-cell-short proof (`/tmp/sweep-closed-fixed-authored-bound-refined-native-20261005.log`). This extends retained-patch/original-source correspondence, not global embedding, smooth path seams or Solid. Current publisher92336 and core11171 compiled before this production extension; their eventual results must not qualify it. This new extension needs its own native broader checks/publication/transport/UI qualification.


### Closed authored original-bound correspondence and native transport (2026-10-05)

The closed Fixed authored regression now checks1088 actual retained control-trajectory samples over all32intervals against independently evaluated original circle/normalized authored-axis/projected-normal formulas. This finite regression supplements the interval certificate; it does not replace it. Native surface_progressive_sweep_level transport reports continuousBound=true, accepted=true at33sections, unchanged budget2, and retained-patches-relative-to-original-profile-transport scope. Added17-section native level rejection despite a valid bound exceeding2, and a closed RMF authored negative. The initial negative test failed at construction because with_frame_laws already requires Fixed orientation; corrected the regression to assert this constructor refusal. The prior transport/sample regression passed (`/tmp/sweep-closed-fixed-authored-bound-transport-native-20261005.log`); the corrected combined negative regression is pending (`/tmp/sweep-closed-fixed-authored-bound-negative-retry-native-20261005.log`).

Publisher92336/full-core11171 still cover the preceding projective endpoint source, not this later closed-bound production extension. The latter remains native-only pending its own publication and broad qualification. No global embedding, cap or Solid guarantees are inferred from retained-patch source correspondence.


Corrected combined closed Fixed authored regression32332 completed0,1/1; coarse17-section refusal, constructor RMF rejection,1088 samples and native transport all pass (`/tmp/sweep-closed-fixed-authored-bound-negative-retry-native-20261005.log`).


### Projective published WASM and headed Rush terminal (2026-10-05)

Publisher92336 completed0, optimized11062216bytes from12439616. Public and generated WASM SHA256360072e6efaa2e827e6253b9af94978765c2e36e2ca6ff6133429973b3f4474f. Public3files20/20passed1.18s (`/tmp/sweep-projective-frame-public-20261005.log`), including projective Bezier/multispan endpoint weight ratio, point/weight one-bit refusals, budgets and hardened viewport reason validation. Vite build0. Headed49903 completed0:matrixpassed=true,2cases/28checks,exact360072e6 provenance (`ui-closed-projective-frame-c2-current/matrix.json`), proving the separate closed-source-frame C2 presentation and independent Solid refusal at1440/600. Full-core11171 still live for this implementation.

After UI became terminal, dispatched current closed-bound publisher60531 (`/tmp/sweep-closed-authored-bound-wasm-20261005.log`) and closed-regression native60529 (`/tmp/sweep-closed-authored-bound-closed-regressions-20261005.log`). Added nurbsClosedAuthoredSweepBound.test.ts with17-section certified-but-over-budget refusal versus33-section accepted bound under the same budget2 and separate closed source-frame C2. vue-tsc passed; this test requires the new closed-bound package and has not run against360072e6, which intentionally predates that production extension.


### Closed original authored shared budget and Rush scenario (2026-10-05)

Current closed-regression60529 completed0:48/48,10.16s (`/tmp/sweep-closed-authored-bound-closed-regressions-20261005.log`). Extended the native closed Fixed authored regression to2 and64profiles with one shared proof budget:2profiles certify/accept,64profiles refuse continuousBound and numeric upper when incomplete; cells never exceed10000. Passed1/1,0.86s (`/tmp/sweep-closed-authored-bound-shared-native-20261005.log`). Matching2/64public fixtures added, pending the active publisher.

Added closed-authored-source-bound-progressive-sweep.r with world profile4.1..4.2,closed radius4path,moving original cubic authored axis,33maxsections,unchanged2mm budget. Native frontend assertions preserve authored orientation,max_sections33,max_deviation2. Browser matrix includes separate closed-C2 presentation plus the existing rigorous bound/tight-budget refusal/restoration assertions on both widths. Browser syntax and public types passed. Publisher60531 is confirmed live after release compilation1m22s; core11171 still qualifies the preceding projective implementation only. New changes since publisher dispatch are tests/examples, not production geometry.


### Closed authored joint-law and decomposition composition (2026-10-05)

Expanded the closed Fixed authored regression to variable polynomial scalar scale/twist, affine axis scale and center offset with closed endpoints. The retained-original bound certifies under unchanged2mm and the actual level accepts at33sections. A one-cell-short proof refuses. The twist has non-periodic endpoint jets, so the independent closed source-frame C2 certificate correctly remains false. Native joint/decomposition/composed tests passed (`/tmp/sweep-closed-authored-bound-joint-laws-native-20261005.log`, `/tmp/sweep-closed-authored-bound-decomposition-native-20261005.log`, `/tmp/sweep-closed-authored-bound-composed-native-20261005.log`).

A33-control original rational profile with alternating weights exercises the decomposition path: positive exact product count, included decomposition upper, one-product-short refuses with no error_upper. It uses the explicit100000-cell native proof budget; no default UI dense-profile success is claimed. Added the corresponding public joint-law/independent-C2 refusal case, pending publisher60531. Typecheck passes. These additions are tests only; production closed-bound geometry is unchanged since60531 compiled. Publisher60531 and preceding-source full-core11171 remain live. Full global/solid/cap guarantees remain independent.


### Projective full native suite terminal; near-closing authored source distinction (2026-10-05)

Projective full-core11171 completed0:1206/1206,1132.57s (`/tmp/sweep-projective-frame-full-core-20261005.log`). It predates the closed Fixed authored-bound extension. Dispatched current-source full-core27371 (`/tmp/sweep-closed-authored-bound-full-core-20261005.log`); publisher60531 still live.

Added a near-closing authored axis whose final originalx differs by1e-12. Constructor tolerance admits it and copies the first retained section; the original-bound owner certifies only after charging a positive endpoint mismatch greater than1e-13. The exact closed-source-frame C2 owner refuses, so tolerance is not promoted to an exact seam. Native expanded regression83925 passes1/1,2.97s (`/tmp/sweep-closed-authored-bound-near-seam-native-20261005.log`). Matching public regression/typecheck added pending new package. New test edits do not change production geometry already compiled by60531/27371.


### Closed authored published qualification and rotating-axis tube gap (2026-10-05)

Publisher60531 completed0:optimized11062228bytes from12439634. Public/generated SHA256 both464ca66da8631ba9b0a02da66d70f768a114e38ad0cf5b8398e4f9819bcc3e44. Public88349 passed24/24in4files,1.52s (`/tmp/sweep-closed-authored-bound-public-20261005.log`), including all closed-bound shared/joint/near-seam cases and preceding projective cases. Vite9002 completed0. Headed11269 completed0:matrixpassed=true,2cases/32checks,464ca66d provenance (`ui-closed-authored-source-bound-current/matrix.json`), including rigorous positive bound, strict refusal/restoration and separate C2. Current-source full-core27371 still live.

Direct actual-body probe:closed radius4circle,path-tangent rational authored axis,constant transverseZ,0.2circular cross-section centered at[4,0,0]. At initial5/max65,retained bound refuses longitudinal-nonzero-unproved:wide rotating-axis intervals cannot prove nonzero Cartesian norm. At initial17/max65,that coarse issue is avoided but shared default10000-cell budget exhausts at9999/frame-value-unresolved. No boundary/Solid success is claimed for this tube (`/tmp/sweep-closed-authored-tube-probe-20261005.log`, `/tmp/sweep-closed-authored-tube-refined-probe-20261005.log`).

Source inspection identifies repeated original frame/path/law evaluation per profile control in section_interpolation_with_length. Existing native trajectory::control_trajectories_with_fit and control_values_with_fit already share original source premises across qs batches for other modes. Next aligned implementation is authored batch owner entrypoints and bounded per-station batching, preserving actual endpoints, knot fallback and shared incomplete-work refusal; this can address the real rotating-axis budget gap without loosening admission or treating samples as proof.


### Native authored original-source batches close rotating-axis budget gap (2026-10-05)

After interruption, full-core27371 handle was missing and no cargo/nurbs_core process was present; its log has no terminal test result, so full-suite success is not claimed. Published464ca66d remains unchanged.

Added native authored certify_control_values/trajectories batch entrypoints using the existing shared original-source composition owner. Parameter authored section error now charges one original frame/path/law batch per station/interval and applies its native values/jets to every owned control, with actual endpoint displacements and original knot-transition value fallback preserved. Cell work is native source evaluation, shared once; no reconstructed scalar is admitted as authored and no cache crosses source/interval boundaries.

Existing combined closed-bound regression27866 passes1/1 (joint affine/scale/twist/center,decomposition,incomplete budgets,2/64profiles,near seam,1088 correspondence samples,transport). New actual rotating rational authored-axis/circular-profile source regression15504 passes1/1 under unchanged10000-cell budget at33sections; one-cell-short refuses and actual level accepts. This directly addresses the earlier tube's native budget gap. Logs /tmp/sweep-authored-batch-closed-bound-native-20261005.log and /tmp/sweep-authored-batch-rotating-axis-native-20261005.log. New closed regression suite dispatched, pending. Batch branch is not yet published in464ca66d; tube body global/Solid and independent STEP/UI still require separate validation.


### Authored batch equivalence verified and publication dispatched (2026-10-05)

Closed suite84426 completed0:49/49,10.33s (`/tmp/sweep-authored-batch-closed-regressions-20261005.log`). Direct batch equivalence initial fixture failed because geometric primitive line rejects coincident endpoints; fixed only the fixture to use ordinary original constant-law Curve storage. Retry85665 completed0:1/1,0.01s (`/tmp/sweep-authored-batch-equivalence-retry-native-20261005.log`), and its authoritative log was read successfully. Every value/first/second jet and single_span flag equals the corresponding original single-control owner output; shared cells are below the independent-source-cell sum; one-cell-short refuses. No geometric validation was relaxed.

Default-directory tool reads remained waiting without output; explicit workdir/tmp read the log and explicit project workdir executes normally. Do not infer test failure from those waiting reads. Dispatched publisher84330 (`/tmp/sweep-authored-batch-wasm-20261005.log`) and full current-source core94708 (`/tmp/sweep-authored-batch-full-core-20261005.log`) using explicit project workdir; both pending. Current public WASM remains464ca66d and does not include batch source sharing. Rotating tube source-bound native positive still requires public/body/Solid/STEP/UI qualification.

### Original authored batch publication and closed tube qualification (2026-10-05)

Published optimized geometry SHA256 `7132c7c81fb02c226bc2e8d48015c3461b28b0a253c93b53f2161e02c0dcbb51`; generated and public WASM bytes match. Publisher exited successfully; optimized size11063005 bytes. Focused public matrix25/25 passed in four files (`/tmp/sweep-authored-batch-public-20261005.log`). Added actual closed authored tube B-rep/default-budget material regression; its file6/6 passed (`/tmp/sweep-authored-batch-tube-body-tests-20261005.log`). Direct typecheck passed.

Closed original rational rotating-axis tube has128 retained faces, complete constructor-owned wall coverage, no caps, original continuous boundary upper1.1972926248859694 at budget2, and1547 source certificate cells. Native default-budget volume audit proves all faces injective, all pairs classified, boundary embedding, consistent nesting and outward material orientation; `solidGeometryCertified=true`. Probe evidence: `/tmp/sweep-authored-batch-tube-brep-retry-20261005.log`. Initial diagnostic probe stopped at a separate cap-contact embedding API requiring1..16 cap faces; corrected the probe to use the volume audit's supported empty-cap scope. No production admission rule was relaxed.

Added closed tube STEP fixture with native Solid gate and independent rational-boundary volume reference, plus Rush body example and UI-matrix inclusion. Export3675 and Vite build36284 are in progress; OCCT and UI results are not yet established. Full current-source Rust suite94708 remains running. This finite closed Fixed/authored case does not close all-mode continuous error, moving-frame corrections, G1/G2, hole/global guarantees or the full integration matrices.

### Closed authored tube independent STEP and headed UI completion

New package7132c7c8 finite STEP matrix68/68 passed: `external-step-closed-authored-batch-current/opencascade-sweep.json` has `passed=true`. Closed tube case valid,1 solid,1 closed oriented shell,128 faces,256 manifold edges, opposite uses and retained full-domain wall/edge/pcurve ownership preserved. OCCT volume3.137895209178169 agrees with independent rational-boundary quadrature3.1378952084431004; relative error2.3425525765635796e-10.512 material-side probes agree with the native material certificate. Finite sampling supplements separate full-domain preservation predicates; it is not universal geometric proof.

Headed Rush/UI closed tube retry passed2 cases/28 checks at1440x1000 and600x1000: `ui-closed-authored-tube-retry-current/matrix.json`, matching source/public/dist package7132c7c8. Initial run failed in negative-fixture construction because the harness only rewrote line/Bezier paths; added a circle-path-to-zero-line negative fixture and preserved the failed initial artifact. Retry verifies original successful Solid, boundary budget refusal, lifecycle cancellation, invalid source refusal with body preservation and restoration. Held dispatch tests do not establish mid-kernel cancellation latency. Full Rust94708 remains running; full finite surface/body matrices on this new package and remaining all-mode mathematical guarantees are still open.

### Full-core batch regression accounting correction

Full current batch run94708 terminated with1207 passed/2 failed in1143.48s (`/tmp/sweep-authored-batch-full-core-20261005.log`). Both failures exposed previous resource-accounting expectations: independent-domain authored interpolation now consumes65 shared source cells rather than120; the former32-profile exhaustion fixture now succeeds. Updated exact accounting and the exhaustion fixture to64 profiles, without relaxing10000-cell admission or retaining a partial error. Separate targeted retries passed each failing test (`/tmp/sweep-authored-batch-cell-count-retry-20261005.log`, `/tmp/sweep-authored-batch-multi-budget-retry2-20261005.log`). An initial test-only edit accidentally changed the first unrelated32-profile fixture; inspection found and restored it before the passing targeted retry. Production Rust/WASM bytes unchanged.

Full-core retry75345, filtered authored run86061 and full surface UI38199 are live; no successful full-core result claimed. Strengthened the exact same budget regression with explicit32-profile positive acceptance and64-profile incomplete refusal; new targeted run57053 is pending. Current STEP68/68 and closed tube headed UI2/28 remain completed fixture evidence, not all-mode proof.

Targeted strengthened budget regression57053 finished successfully1/1 in1.11s. It explicitly verifies32-profile positive continuous acceptance and64-profile refusal with numeric full-bound fields absent, shared source cells≤10000 and strict sampled-zero case still refused. Evidence: `/tmp/sweep-authored-batch-multi-budget-both-sides-20261005.log`.

### Closed authored arc-length native extension and finite surface UI

Published7132c7c8 full headed finite surface UI completed72 cases/1080 checks: `ui-authored-batch-surface-matrix-current/matrix.json`, passed=true. Full body UI launched separately; package/dist are preserved while it runs.

New native source permits the original length-Lipschitz section bound for closed nonperiodic Fixed/authored frames only: no holonomy correction, and copied closing section is compared to original endpoint pose. Other closed corrected modes and periodic basis remain unproved. Native closed C0 square path passed within10000 source cells, including one-cell-short refusal and a1e-12 authored endpoint mismatch charged numerically while closed source C2 remains false (`/tmp/sweep-closed-authored-arc-near-seam-native-20261005.log`,1/1,.28s). Initial rational circle experiments exhausted10000 and100000 cells in length division at tolerance.001; no numeric bound was admitted. The circle inverse-length resource gap remains open; replacing the regression with a C0 polygon proves the extension premise without claiming circle success. This new production extension is not yet in package7132c7c8. Existing full-core retry75345 and filtered86061 were started before the arc extension and cannot qualify it.

### Original length partition reuse closes circular native arc proof

Rust measure_partition retains the original whole-cell integration partition for one exact source division. Prefix inversion sums certified complete cells and evaluates only the cut cell; if its prefix tolerance is insufficient it falls back to full original integration, charging all work. No total-minus-prefix subtraction or cross-source reuse. Native circle32-segment division now fits10000 cells; independent angular-length/2π reference and insufficient-work refusal pass (`/tmp/sweep-circle-length-independent-native-20261005.log`,1/1,.45s). Full closed Fixed/authored circular sweep now proves original retained error within10000 cells and accepts33-section level alongside C0 polygon/copied near-seam regression (`/tmp/sweep-closed-authored-arc-circle-native-20261005.log`,1/1,2.65s). Native arc regression22392 and new WASM build are running; public arc test added but not yet verified on a replacement package.

Full body UI29277 on7132c7c8 failed after four1440-width cases at a120s strict-budget build timeout (`/tmp/sweep-authored-batch-body-matrix-20261005.log`). No full body matrix success claimed. Keep failed evidence; new length reuse must be qualified before repeating the matrix. Existing full-core75345/filtered86061 predate this new production code and cannot qualify it.

### Guided arc regression expectations after original partition reuse

Arc suite22392 found two previous refusal expectations contradicted by complete new certificates. Curved guide on original straight path at9 sections now certifies; distinct curved path/guide at17 sections now certifies within10000 cells and error upper<2. Updated these expectations and preserved explicit1-cell incomplete refusal with no numeric error. Existing independent original-frame samples for the distinct-curvature case remain in place. Targeted retries pass1/1 each (`/tmp/sweep-length-reuse-guided-line-retry-20261005.log`,.36s; `/tmp/sweep-length-reuse-guided-curved-retry-20261005.log`,.67s). The first all-arc run uses its pre-correction test binary and cannot be reported fully successful. New publisher54126 is confirmed live, Rust release completed1m22s and optimizer continues. No replacement WASM/public result yet.

### Closed authored arc regression migration

Third old arc refusal fixture used initial_sections3 for a now-supported closed authored path, violating the closed constructor minimum. Updated it to5 and verified complete17-section original bound≤.1 rather than preserving an obsolete rejection. Targeted test44413 passed1/1,.25s (`/tmp/sweep-length-reuse-authored-closed-retry-20261005.log`). Arc retry92673 was compiled before this last test correction and finished26/27; current retry38337 runs all27. Earlier authored86061 predated its32→64 budget fixture correction and ended131/132; this is not current success evidence. Already verified targeted corrected budget fixture remains authoritative.

Added `examples/rush/closed-authored-arc-length-progressive-sweep.r` and surface harness inclusion for next replacement-package UI. Public native arc adapter test exists but waits for live publisher54126. Full-core75345 still runs its pre-arc binary; no current full-core claim.

### Current original-length regression qualification

All current arc-length native regressions38337 passed27/27 in55.25s (`/tmp/sweep-length-partition-arc-regressions-final-20261005.log`). Includes closed Fixed authored source path, independent curved path/guide images, joint affine laws and explicit incomplete shared-budget refusal. Direct typecheck passed. Current-source full core77720 launched after all production/test corrections; earlier full-core75345 remains historical pre-arc evidence. Publisher54126 still confirmed live optimizing the current original-length production code. Surface harness explicitly checks separate closed source-frame C2 for the new arc example, while its retained seams and Solid remain independent. No new public package result yet.

### Completed pre-arc full-core baseline

Full-core retry75345 completed1209/1209 in1157.51s (`/tmp/sweep-authored-batch-full-core-retry-20261005.log`, terminal0). This qualifies production authored batch implementation before the closed arc/length partition changes, matching7132c7c8 production scope. Current-source1211-test full run77720 remains live. Publisher54126 still confirmed live; probe supports an explicit `--arc-length` choice for actual closed circular-profile body qualification once replacement bytes are published, and direct typecheck passes.

### Closed authored arc WASM publication

Publisher54126 exited0. Generated/public replacement WASM SHA2564a5dab632e0ae8ac07d1a8b3ab5d989aa73db71f0b11d7a74d2952e98bc290a7 match, optimized11063899 bytes. Focused public matrix27/27 in four files2.47s (`/tmp/sweep-closed-authored-arc-public-20261005.log`), Vite build3248 succeeded. New closed authored arc surface headed UI launched separately; no UI result yet.

Actual rotating-axis circular-profile arc-length tube probe completed (`/tmp/sweep-closed-authored-arc-tube-probe-20261005.log`):64 retained faces, default native Solid material/embedding all certified, but constructor boundaryContinuousBound=false with reason arc-length-relative-pose-remainder-unproved and5399 cells at17 sections. Numeric full boundary upper remains null; sampled accepted=true is not complete boundary proof. Whole relative-pose interpolation across original frame knots remains a concrete next gap. New constant-frame closed circular arc surface public test is positive; do not substitute it for full rotating tube coverage.

### Arc authored whole-pose knot fallback (native, next publication)

Native original arc section owner now shares source value/jet batches across controls. At original frame knot transitions it encloses the original relative value image versus the hull of retained relative endpoints, subtracting actual endpoint position intervals enlarged by certified length residual. Original position chord defect L*h/2 is added separately. No global C2 remainder is used across knots. Native rotating-axis circular-profile arc case17 sections is Certified but upper10.96308130117187 exceeds2; refinement to33 satisfies2 within10000 cells and one-cell-short refuses (`/tmp/sweep-arc-rotating-authored-knot-refined-native-20261005.log`,1/1,3.23s). Earlier failed17-section acceptance assertion is retained as evidence; tolerance was not increased.

Published4a5dab63 constant-authored closed arc surface headed UI passed2 cases/32 checks, matrix `ui-closed-authored-arc-current/matrix.json`; Solid remains refused for its open profile, correctly. Added public body regression for both parameter/arc_length modes; arc success must wait for new batch/fallback WASM publisher5926. Native arc batch regressions27159 run. Existing full-core77720 qualifies pre-batch arc production only, not this new change.

### Current arc authored batch regressions and pre-batch full core

Arc authored batch suite27159 completed27/27 in54.26s (`/tmp/sweep-arc-authored-batch-regressions-20261005.log`). Pre-batch length full-core77720 ended1210 passed/1 failed in273.59s: previous Frenet arc refusal expectation now returns complete certificate. Updated that expectation and retained explicit1-cell incomplete refusal; targeted15854 passed1/1,1.21s (`/tmp/sweep-length-reuse-frenet-retry-20261005.log`). No successful current full suite claimed; new full current-source arc-batch run launched after this test correction.

Added independent STEP arc tube fixture and `closed-arc-length-authored-tube-body-boundary.r` with headed body-harness inclusion. These are prepared artifacts pending replacement batch/fallback package5926; exporter has not been run on the old4a5dab63 package because its arc tube boundary remains unproved. Complete native rotating proof requires33 sections under unchanged2mm budget. Current packet/state qualification remains separate from general moving-frame corrections, G1/G2 and all-mode global guarantees.

### Independent original rotating arc pose supplement

Expanded native rotating-axis arc regression with4896 retained-control samples (32 intervals×17 fractions×9 original controls). Expected original pose uses independent rational quadratic quarter-circle equations with the original source weight, normalized axis direction and analytic normalized circle arc-length position, without kernel evaluation of original frame/path. All retained segment errors fit the complete interval upper; test96586 passed1/1 in3.48s (`/tmp/sweep-arc-rotating-authored-independent-native-20261005.log`). This supplements the whole-domain original interval certificate and does not substitute finite samples for it. Production unchanged; publisher5926/current full-core93366 remain running.

### Full current arc authored batch core completed

Current production full-core93366 completed1211/1211 in277.75s (`/tmp/sweep-arc-authored-batch-full-core-20261005.log`, terminal0). Includes original integration partition reuse, closed Fixed/authored normalized arc-length position, shared authored value/jet source work and original-knot relative value-image fallback. Independent4896-sample supplement was added after this full run compiled and passed separately; no production change followed full-run compilation. Publisher5926 is confirmed live in wasm-opt, so replacement public/STEP/UI results remain pending. Original all-mode/corrected frames/global/G1/G2 requirements remain unachieved despite full finite native regression success.

### Rotating authored arc tube public boundary and Solid success

Publisher5926 exited0; optimized11065527 bytes. Generated/public geometry SHA256569781c109cb510d82139edd1dedfcf91b830974b3262a5c7270a2dae626ed7c match. Public matrix47701 completed28/28 in four files3.64s, including actual tube boundary/material in both parameter/arc-length modes (`/tmp/sweep-arc-authored-batch-public-20261005.log`). Actual arc tube30361 probe:128 retained faces,33 sections, original continuous body boundary upper0.6445141839409965≤2,3226 source cells, original endpoint error0.0009878858431778541 including length residual. Complete native default-budget volume audit certifies injectivity, all pairs, nesting and outward material orientation; Solid=true (`/tmp/sweep-arc-authored-batch-tube-probe-20261005.log`). Vite17520 succeeded. Independent69-fixture STEP export91901 and headed arc tube body UI launched; no external/UI result yet.

### Rotating arc tube headed UI and joint-law native coverage

Headed rotating arc tube52571 completed2 cases/28 checks at1440x1000 and600x1000 (`ui-arc-authored-tube-current/matrix.json`, passed=true, package569781c1). Native rotating arc proof supplemented with simultaneous quadratic scale, twist, affine axes and center laws under10000 cells/2mm; one-cell-short refuses and source frame C2 is not falsely promoted. Targeted24297 passed1/1,5.85s (`/tmp/sweep-arc-rotating-joint-laws-native-20261005.log`). Test-only additions do not change published production bytes. Expanded public joint-law test to parameter/arc-length modes; its run is pending. Full headed body matrix44967 and STEP export91901 remain live. No full body matrix or independent69-case STEP result yet.

### Current independent69-case STEP completion

Published569781c1 independent STEP pipeline completed: export91901, rational boundary86869, generator reference18417 and OCCT5350 terminal0. `external-step-arc-authored-batch-current/opencascade-sweep.json` passed69/69 with matching generated/public package provenance. New closed original arc-length authored tube:128 faces,256 manifold edges,1 valid closed outward shell/solid, source loop/edge/pcurve ownership and full-domain wall preservation certified. Native/external material agreement and512 independent side probes agree. OCCT volume3.1378136354800654 vs independent boundary integral3.1378136347450325, relative2.3425002532868006e-10. Independent integration error5.941381100437602e-16. This finite matrix establishes listed fixture retention/material agreement, not all-mode original/global/G1/G2 proofs.

Expanded joint public file96898 passed9/9 (`/tmp/sweep-arc-authored-joint-laws-public-20261005.log`,3.93s). Full body UI44967 remains live, now processing600-width cases; no whole-matrix success yet.

### Current full finite body UI completion and native trajectory consistency

Published569781c1 full headed body matrix44967 completed30 cases/418 checks (`ui-arc-authored-batch-body-matrix-current/matrix.json`, passed=true). Includes new arc tube, previous closed tube, multispan authored, curved arc modes and hollow/unsegmented profiles at1440/600 with explicit successful Solid/refusal/cancel/source/restoration behavior. Earlier7132c7c8 strict-refusal timeout is retained historical failure; current full body matrix is positive. Held dispatch remains lifecycle evidence, not mid-kernel latency proof.

Native public Rust helper `authored_control_trajectory` no longer rejects all closed paths when orientationFixed has no correction. Its scope is original source trajectory, not retained copied seam; single-control original jet and one-cell-short refusal passed in expanded rotating regression51361,1/1,5.49s (`/tmp/sweep-closed-authored-trajectory-native-20261005.log`). Curved arc-length full trajectory jets still refuse without inverse-derivative proof; retained error uses the separate Lipschitz construction. This helper source change is not yet in a rebuilt release package; production browser reports remain qualified against569781c1. Launched native closed regression suite and full current-package surface matrix separately; results pending.

### Current full surface UI and closed native completion; background policy

Current package569781c1 full surface matrix80603 completed before process reinspection: `ui-arc-authored-batch-surface-matrix-current/matrix.json` passed=true,74 cases/1112 checks at1440/600, no failed assertions. Exact test process is absent from live process inventory. This historical run was headed; future visible runs require direct user authorization. Closed native suite81210 completed50/50 in18.92s (`/tmp/sweep-closed-authored-trajectory-regressions-20261005.log`), including native-only closed Fixed trajectory helper change. The helper is still not included in rebuilt public569 package.

UI harness now ignores inherited SWEEP_UI_HEADED: only explicit --headed opens a visible browser; default remains headless. `node --check scripts/check-sweep-miter-matrix-browser.mjs` passed. Headless CPU fallback remains insufficient for GPU qualification. Existing native retained smoothness owner covers station seams only, explicitly excluding profile/decomposition joins, caps and source frames; finite matrix success does not close the full all-mode goal.

### Native explicit retained profile-join owner

Added Rust Level::certify_retained_profile_join(left_patch,right_patch,order,transverse_scale,max_work). Validated distinct patch indices and identical station degree/knots/periodicity/column count establish matching station ownership before the existing exact projective uMax/uMin strip predicate. Local sufficient G1/G2 plus independent regularity do not certify all source adjacency, source error, caps or global shell regularity. No TS geometry logic added; API is not yet transported or in published569 WASM.

Focused native fixture verifies planar G2, one-work-short refusal, mismatched station interval rejection, sharp C0 corner refusal and self-pair rejection. `/tmp/sweep-profile-join-native-20261005.log` terminal0,1/1; all retained smoothness regressions `/tmp/sweep-profile-join-smoothness-regressions-20261005.log` terminal0,6/6,.23s. Target file diff whitespace check passes. Automatic complete profile/decomposition adjacency coverage and public transport remain open.

### Native profile-join transport and public wrapper

Added surface_progressive_sweep_profile_join native dispatch reconstructing the shared-source MultiSweep preview, auditing explicit left/right uMax/uMin pair, and returning profilePatchRanges plus local certificate/work. Scope explicitly excludes all-profile joins, original frame smoothness, caps, continuousBound and Solid. TS inspectProgressiveRetainedProfileJoin only serializes source/options and native result; no numerical geometry/admission added. Native retained smoothness regressions passed7/7,.25s (`/tmp/sweep-profile-join-transport-native-20261005.log`). vue-tsc passes both before and after new public regression fixture; targeted diff whitespace passes.

Public fixture tests planar G2, insufficient shared work, invalid self-pair and sharp corner. It has not yet run against new package. Background WASM publisher session36904 is confirmed live (`/tmp/sweep-profile-join-wasm-20261005.log`); no browser launched. Current published569 package remains previous evidence until publisher terminal status and byte provenance checked. Complete automatic decomposition/profile adjacency and Rush/viewport display still remain required.

### Actual dense-profile decomposition qualification and exact station limitation

New native regression constructs an original33-pole linear profile (native32-part decomposition) with65 stations (three chunks per part). It traverses all93 explicit interior profile/chunk pairs under one shared1000000 work budget and verifies wrong flattened-neighbor station ownership rejection and a true sharp profile corner refusal. Initial assumption all93 exact G2 failed at part7/chunk1 with linear-along-jet-smoothness-unproved (`/tmp/sweep-profile-join-decomposition-native-20261005.log`, terminal101). This is retained evidence: rounded represented station jets need their own proof, nominal planar source does not imply exact represented G2.

Regression now requires both successful exact certificates and explicit unresolved jet results rather than mislabeling all joins; retry89243 passed1/1,.65s (`/tmp/sweep-profile-join-decomposition-native-retry-20261005.log`). Production code unchanged by this test-only qualification. Complete original requirements still include applicable rounded station/profile join smoothness. Native full suite97940 and WASM publisher36904 confirmed live; publisher is in wasm-opt. Full suite compiled before this test-only addition; no all-suite completion yet. Public new test remains pending publication.

### Profile-join full native regression completed

Full native suite97940 exited0:1213/1213 passed in277.96s (`/tmp/sweep-profile-join-full-core-20261005.log`). Covers latest closed Fixed helper plus explicit profile-join Rust API and source-reconstructing transport. It compiled before the subsequent dense-profile test-only fixture; that fixture passed separately1/1. No production changes followed full-suite compilation.

Publisher36904 remains confirmed live in Binaryen wasm-opt process42918 (100% CPU,07:16 elapsed at last inspection). No restart or duplicate publisher; public wrapper fixture is still pending new package. This verified ongoing work does not establish public WASM qualification or goal completion.

### Explicit profile-join WASM publication and native automatic pair ownership

Publisher36904 completed0. Optimized11068634 bytes from12446790; generated/public SHA256 b990469bbb475746e70a666ea2efe1fb414dae73226a1341e1ab27d97bb5d2ff match. Includes explicit retained profile-join transport and closed Fixed original trajectory helper. Public78975 completed30/30 in5files,5.04s (`/tmp/sweep-profile-join-public-20261005.log`): new profile-join public wrapper plus closed authored bound, closed/original frame smoothness and viewport evidence. No headed browser launched. Current dist/independent STEP qualification still belongs to569 package; no new UI or STEP result claimed for b990.

Added native-only MultiLevel::retained_decomposition_join_pairs with bounded64 profile ranges/4096patches, contiguous complete ownership, validated literal linear station basis, full[0,1]chunk cover and exact matching adjacent part chunk knots. Authored profile boundaries and profile closure intentionally require separate loop ownership; no all-profile/global claim. Two dense33-pole original profiles produce186 internal pairs without cross-profile joins; missing chunks/orphan ranges rejected. Native smoothness suite87972 passed8/8,1.45s (`/tmp/sweep-decomposition-owner-smoothness-native-20261005.log`). This production pair-owner addition followed b990 compilation and is NOT included in published package or previous1213 full-core result. Complete automatic certificate aggregation and transport remain pending.

### Automatic native decomposition certificate aggregation and transport

Added MultiLevel::certify_retained_decomposition_joins(order,transverse_scale,max_work), using exhaustive owned pairs and one shared exact work budget. It reports expected/checked joins, coverageComplete independently from all sufficient G1/G2 identities/regularity; absent joins do not produce a positive smoothness flag. Partial work never certifies the union. Constant positive transverse scale remains a sufficient relation, not a universal G1/G2 solver. Cross-authored profile boundaries, profile closure and caps remain outside this owner.

Native transport surface_progressive_sweep_decomposition_joins reconstructs sources before aggregation and preserves method/scope, patch ranges, per-pair diagnostics and explicit false allProfileJoins/closedProfileSeams/sourceFrame/cap/continuousBound/Solid flags. TS wrapper is transport only. Positive two-profile/two-station dense decomposition62 joins certified; work-1 and zero refuse. 65-station186-pair case exposes finite shared-budget partial coverage and exact rounded station limitations. Initial test assertions wrongly assumed full coverage at1million and used preview2 below initial5; corrected qualification retains failure logs, no predicate weakened.

Native transport/smoothness22214 passed8/8,1.58s (`/tmp/sweep-decomposition-aggregate-transport-native-20261005.log`); typecheck26281 and target diff checks pass. Public regression added but awaits new WASM. New background full-core and publisher launched; no result yet. Current published b990 package excludes native automatic pair/aggregate additions; its earlier30 public checks remain separate evidence.

### Decomposition report attachment and viewport presentation prepared

Accepted constructor/preview reports now request native decomposition G2 at their exact retained section count and attach the scoped result only to the current/final level (not earlier station levels). Existing Rush construction-report/artifact path carries this field. Geometry, adjacency extraction, exact predicates, budgets and certificate admission remain Rust; TS attaches/validates transport evidence for UI only. Viewport adds G2 between parts of one profile with expected join count; no applicable joins hide this indication. Source frame, surface regularity, closed profile/caps, continuous error and Solid stay independent.

Viewport evidence requires native method/scope/order, complete coverage, all local identities/regularity, unique pairs, bounded work and summed shared work, plus explicit false outside-scope flags.13/13 viewport regressions5865 passed (`/tmp/sweep-decomposition-viewport-evidence-20261005.log`); typechecks48486/12590 pass, script syntax and scoped whitespace checks pass. Added real33-point Rush fixture decomposed-profile-g2-progressive-sweep.r and public Rush-to-viewport test; added UI fixture corner refusal/restoration assertion at1440/600. These integration/UI tests have NOT run against new WASM yet. No visible browser launched.

Full current Rust suite33161 completed1214/1214,287.78s (`/tmp/sweep-decomposition-aggregate-full-core-20261005.log`), including automatic pair ownership, aggregate certificate and transport. Native production unchanged since compile; new UI/TS fixture additions are separate. Publisher49682 confirmed live in wasm-opt53783 (100%CPU,04:30 elapsed); no duplicate/restart. Current b990 package does not support new aggregate opcode, so public/new UI qualification must wait.

### Contract reconciliation while aggregate publisher remains live

Updated design/sweep-contract-matrix.md to remove obsolete pending claims for published569 closed Fixed authored arc-length and original frame-knot error qualification: finite69 STEP,30/418 body UI and74/1112 surface UI evidence is now stated with its exact scope. Added current native decomposition pair/aggregate contract, separate expected/checked/full coverage states, shared work, sufficient constant reparameterization limitation, source-profile-only ownership and independent excluded proof obligations. No all-mode completion inferred.

Aggregate publisher49682 remains confirmed live in wasm-opt53783 (100%CPU,09:43 elapsed at last process inspection). Existing immutable input continues; no restart/new publisher, no browser launched. New public Rush/viewport integration tests still await package completion. Native1214 suite and viewport13 evidence are terminal passing results from prior checkpoint.

### Aggregate WASM/public/Rush completion and background UI evidence

Publisher49682 completed0: optimized11075355 bytes from12454379, generated/public SHA256 f84ac06ac159a580c80203474a5273680b3c79cc492e62566d85f0f707498739 match. Includes native pair ownership, aggregate certificate and transport. Initial public13436 ran32/33: the Rush test used synchronous tessellated mesh construction without source artifact. Corrected the test to the actual async UI build path; no product or predicate weakened. Retry86455 passed33/33 in5files,4.44s (`/tmp/sweep-decomposition-aggregate-public-retry-20261005.log`). Async fixture typecheck82934 passes; Vite22597 succeeds.

Strict headless UI12109 failed at missing WebGPU adapter; full rendering remains unqualified. Explicit --evidence-only surface/headless mode preserves that exclusion and only checks UI evidence/lifecycle. First evidence-only80707 passed initial four checks then failed because no-GPU alert z-index8 covered dock-close. Product fix lowers that viewport-only alert below controls/reports to z-index1; its retry button remains interactive. Rebuild61320 succeeds. Retry60803 (`ui-decomposition-g2-evidence-only-overlay-current/matrix.json`) passed=true,headed=false,evidenceOnly=true:2 cases/26 checks,1440/600; viewportRenderingQualified=false both cases. Positive31-join G2, true sharp-corner refusal/restoration, panel interaction, source/cancellation/lifecycle and Solid refusal are covered. No visible browser launched or test forces/bypasses UI click interception.

This closes finite background evidence presentation for this fixture, not all-mode/GPU UI qualification, all authored-profile loop/closure/cap joins or original global/corrected-frame continuous bounds. Independent STEP remains69 qualified on569 package; no replacement external matrix claimed for f84. Goal remains active with original scope.

### Native G1 fallback distinct from G2 and prepared Rush/viewport integration

Added MultiLevel::certify_retained_decomposition_smoothness: G2 uses at most half the caller exact budget, reserving the remainder for an independently complete G1 traversal if G2 cannot be certified. Successful G2 implies G1 natively without a second audit; otherwise native g1/g2 reports and total exact work remain separate. Total<=maxExactWork<=1000000; incomplete/absent joins never promote G1. Same source-profile-only ownership and explicit false profile-closure/cap/source/global/Solid scope remain. New native transport opcode returns nested reports plus owned profile ranges; TS only routes/validates evidence.

Actual original33-pole quadratic16-piece profile has exactly matching first jets and alternating curvature. Native proves15 G1 joins while refusing G2; a middle-control mutation refuses G1 and zero budget refuses both. Native smoothness/transport9837 passed9/9,1.56s (`/tmp/sweep-decomposition-g1-fallback-transport-native-20261005.log`); focused50033 passed1/1,.02s. Typechecks26690/26536/42852 pass, viewport evidence20202 passes14/14,.844s (`/tmp/sweep-decomposition-g1-viewport-evidence-20261005.log`).

Constructor attachment now uses native combined audit at the exact current/final station count. Viewport retains independent G2 and displays G1 separately when G2 is unproved, requiring nested complete coverage, consistent total/shared/reserved work and explicit native G1 conclusion. Added public G1 fixture and actual16-Bezier Rush example decomposed-profile-g1-progressive-sweep.r plus headless evidence-only corner refusal/restoration UI case. These public/new UI tests remain pending new package. No visible browser launched.

Background publisher52946 (`/tmp/sweep-decomposition-g1-wasm-20261005.log`) and full-core19261 (`/tmp/sweep-decomposition-g1-full-core-20261005.log`) launched with current production changes; no terminal results yet. Current published f84 package still qualifies previous aggregate G2/public33 and evidence-only UI2/26, not new combined opcode. Goal full all-mode continuous/global/closed-profile/cap/GPU/STEP obligations remain open.

### G1 full native completion and older TS collection ownership found

G1 full-core19261 completed1215/1215,282.66s (`/tmp/sweep-decomposition-g1-full-core-20261005.log`). Publisher52946 remains confirmed live in wasm-opt; public/Rush G1 tests still pending package completion.

Source inspection found older inspectSweepProjectiveSeams in nurbsSweepAudit.ts still aggregating native per-strip results and making set-level certificate/order decisions in TS. Individual geometry/jet predicates were native, but this collection admission violates the intended all-Rust ownership boundary. Added native join::continuity::inspect_projective_seam_collection and sweep_projective_seams_audit transport: entire declared set validated before work, bounded4096 patches/seams, shared maximum2million, each exact predicate<=1million, complete unresolved-index reporting, certifiedOrder native min across positive requested orders, empty sets refuse. Explicit declarations do not infer omitted/closure/trim adjacency.

Native owner test34760 passed1/1; owner+transport4465 passed2/2,.01s (`/tmp/sweep-projective-collection-transport-native-20261005.log`). Older TS loop has NOT been switched yet and remains an ownership migration item. New generic source/transport was added after1215 full-core and G1 WASM compilation; not qualified by those results and not in pending package. New full-core launched (`/tmp/sweep-projective-collection-full-core-20261005.log`) to cover current source. No parallel/restarted WASM publisher; first finish52946, qualify G1, then package generic collection and route TS wrapper.

### G1/G2 public and background UI completion; exact collection owner added

G1 publisher52946 exited0: optimized11082684 bytes from12462892, generated/public SHA2567db0a584ff4a0d68aeb32b4ac818cb03c09066122dcb31bd261408c75dbf29fe match. Public86789 passed40/41, failing only a stale projective-affine fixture expectation continuousBound=false. That affine source already has an independent complete native original-error owner. Updated regression to require continuousBound/rounding true within original transport scope/budget and independently verify automatic decomposition report does not claim authored-loop/closure joins. Retry79451 passed41/41,6files,5.31s (`/tmp/sweep-decomposition-g1-public-retry-20261005.log`). Vite97542 succeeded.

Headless evidence-only UI63129 and40111 completed0, each2 cases/26 checks at1440/600: `ui-decomposition-g1-evidence-only-current/matrix.json` and `ui-decomposition-g2-shared-fallback-evidence-current/matrix.json`, passed=true,headed=false,evidenceOnly=true. Total4 cases/52 checks verifies G1-positive/G2-negative curvature jumps, true tangent corner refusal/restoration and G2-positive straight decomposition plus lifecycle/Solid refusal. ViewportRenderingQualified remainsfalse; no GPU or full UI rendering claim. No visible browser launched.

Extended pending native collection owner with exact homogeneous collection path alongside projective: sweep_exact_seams_audit preserves maximum1million while projective collection supports shared2million with each predicate<=1million. Both validate entire declarations and native empty/incomplete/order/budget decisions. Focused39717 and35270 owner/transport tests2/2 pass; both serialized protocol branches covered (`/tmp/sweep-exact-projective-collection-transport-native-20261005.log`). Earlier full-core76115 completed1217/1217,299.16s before exact extension; fresh full core and publisher launched for current exact/projective source. These collection opcodes are not in7db; old TS aggregation loops intentionally remain until new package can support replacement calls.

Remaining Rust ownership work includes topology extraction and profile/station set completeness rules in miterProfileSmoothness.ts/miterStationSmoothness.ts, which still drive mathematical scope; per-strip predicates are native but this is not yet all-Rust ownership. Original all-mode error/global/profile-loop/closure/cap and independent STEP/GPU completion obligations remain open.

### 2026-10-05 — background checks and native seam collection ownership

Focus policy: no headed browser, visible tabs or foreground navigation. Headless evidence-only UI remains explicitly unqualified for viewport rendering.

Published generic exact/projective seam collection package: SHA256 `3d922da3cb4d872340f7c3df3099159b2bc64d3b1c062bce4804d96cfb3ce452`, optimized WASM 11088866 bytes. Publisher completed successfully. `inspectSweepExactSeams` and `inspectSweepProjectiveSeams` now transport the entire declaration to native Rust; validation, shared work accounting, unresolved set, minimum certified order and complete-set admission no longer run in TS. Abort checks remain before/after the synchronous call and do not prove mid-kernel cancellation latency.

Validation: current nurbs-core 1217/1217 in 296.15s (`/tmp/sweep-exact-projective-collection-full-core-20261005.log`); public exact/projective tests 10/10; existing miter profile/station regression 13/13; vue-tsc passed. These miter regressions still exercise the legacy TS topology extraction with the new Rust collection owner.

New native BRep `sweep_smoothness` owns actual retained profile/station boundary extraction, cap exclusion, paired edges, G2/G1 fallback and a shared 2m budget. Two focused tests pass: actual circle wall/cap sets; malformed UV, invalid topology/cap scope, exhaustion and zero-budget refusal. Geometry bridge serialization compiles and its shared-work/unequal-span/zero-budget/duplicate-cap protocol test passes. This is retained natural wall boundary smoothness only; trim/world-edge identity, cap G1/G2, source error and global shell validity remain separate.

A single new publisher is running (`/tmp/sweep-brep-smoothness-owner-wasm-20261005.log`, session 64697) to include the BRep owner. TS miter topology extraction is intentionally still active until that package is published and tested; do not claim its migration complete. STEP matrix remains qualified on earlier 569781c1 package; no new STEP or GPU viewport qualification is implied. Full original goal remains open.

### 2026-10-05 — native BRep smoothness negative/closed-set qualification

Previous goal turn made concrete progress (published generic collection owner, TS transport migration, passing native/public qualification and new BRep owner/bridge). Current continuation confirms publisher session64697 is live and the installed public/generated package still matches `3d922da3cb4d872340f7c3df3099159b2bc64d3b1c062bce4804d96cfb3ce452`. No restart or second publisher.

Native retained BRep smoothness targeted suite is now 4/4 in0.52s (`/tmp/sweep-brep-smoothness-owner-closed-native-20261005.log`). Added an actual cubic station G1-positive/G2-negative case; G1 consumes only G2's remaining shared budget and zero remainder stays unproved. Periodic/malformed UV boundaries, invalid loop/edge references, duplicate cap scope and over-limit budgets refuse. Closed hollow miter has two shells,32 profile edges and32 station edges including closure, no caps; profile G2 positive and sharp stations explicitly C0. This does not qualify shell nesting/orientation or global geometry.

Full brep-core session95423 (`/tmp/sweep-brep-smoothness-owner-full-native-20261005.log`) is confirmed live. It compiled before the fourth closed-set test; targeted4/4 separately covers that later test. Native source added after publisher's compile consists only of tests, so the pending package includes the unchanged qualified owner/bridge implementation. TS miter topology extraction remains active until the new owner opcodes are actually published and public regressions pass. Next action: poll64697, verify public/generated SHA, replace both TS miter auditors with transport-only wrappers, run13 existing miter regressions plus viewport evidence/typecheck, and record the exact qualification scope. No UI focus operations were performed.

### 2026-10-05 — published native retained miter topology owner

Prior continuation made concrete progress with four native smoothness tests, exact protocol qualification and the newly implemented native chart collection owner. Publisher64697 and full-native95423 are terminal0. Full BRep suite:709 passed,2 ignored,368.32s (`/tmp/sweep-brep-smoothness-owner-full-native-20261005.log`). This suite compiled before the later fourth closed-set test and chart owner; later targets cover those separately.

Published/public/generated SHA256 `45d7d527e0a3d44775025b8b7ee1a221291d39b36fe1a3f202902cb4296da44f`,11105035bytes. Both TS miter profile/station auditors now only serialize input, bracket synchronous calls with abort checks and return native reports. Native owns complete retained topology extraction, cap exclusions, union completeness, G1/G2 fallback and shared-work admission. Public qualification90958 terminal0:37/37 across miterProfileSmoothness, miterStationSmoothness, nurbsSweepProjectiveStripJets, nurbsSweepExactStripJets and sweepViewportEvidence. Typecheck94247 and Vite91841 terminal0.

Headless UI66052/71998 terminal0: `ui-native-miter-g1-owner-current/matrix.json` and `ui-native-miter-station-g2-owner-current/matrix.json`, bothpassedtrue/headedfalse, same45d7 package provenance. Combined4 cases/48 checks at1440x1000 and600x1000. G1-positive/G2-unproved profile and G2 station evidence reaches presentation; Solid publication, held-dispatch lifecycle cancellation/source change and restored BRep identity pass. Renderer isCPU/SVG; this finite matrix does not qualify WebGPU rendering or unconstrained mid-kernel cancellation latency.

`brep-core::sweep_retained_charts` now owns one complete selected wall-chart injectivity union with shared max100000cells and actual face IDs. Targeted native tests (`/tmp/sweep-native-retained-chart-empty-union-20261005.log`) pass hollow16-wall complete set, every unresolved exhausted suffix, fold refusal, invalid/duplicate caps and nonvacuous empty selection. Bridge protocol95065 terminal0 passes. This logic is not in45d7 because it was added after that publisher's native compile. A single following publisher (`/tmp/sweep-native-retained-chart-owner-wasm-20261005.log`) is launched after all UI jobs completed. Do not switch the chart TS wrapper before its opcode is published and qualified.

Still open: chart transport migration; native ownership of the remaining retained original/decomposed wall/cap admission orchestration; all-mode original continuousBound/global geometry/smoothness completion; current independent STEP qualification; complete viewport renderer/UI coverage. Original full goal stays active.

### 2026-10-05 — native diagnostic original retained correspondence owner

Previous turn classified as progress: published/qualified native miter topology owner and native chart collection implementation. Current publisher98069 is confirmed live (Binaryen PID12431 processing its immutable input at high CPU); no restart or duplicate publisher. Installed package remains45d7d527. This in-flight batch includes native retained chart union and excludes the new correspondence implementation below because the latter was added after its compile.

`brep-core::sweep_retained_walls::inspect_correspondence` now owns the exact diagnostic original-section route end to end: complete actual body face ownership, bounded segmented source sections, entire coefficient family, full actual UV wall domains and world coedge identity with shared exact work. Its report preserves the existing frontend diagnostic reasons. Safe checked loop/edge references cannot panic. Existing constructor-owned `inspect` remains unchanged. Native original_sections_require_owned_domains_world_edges_and_shared_exact_work passes1/1 (`/tmp/sweep-native-original-correspondence-owner-20261005.log`): positive circle walls, exact-work cutoff, omitted shell face, edited world edge, invalid loop and altered source weights. Geometry bridge check3483 and current protocol32622 terminal0; protocol checks positive exact report/wall upper0 and short-budget null upper. The initial protocol run87644 lacked the new added assertions;32622 supersedes its scope.

TS correspondence remains active until its opcode is published; do not claim migration complete. TS decomposition wall/cap orchestration also remains to migrate. Current pending chart publication must finish and be qualified before starting another publisher. No UI work is live, no foreground/focus actions occurred. Full goal still open; no completion claim is supported by these targeted tests.

### 2026-10-05 — published retained chart union and native wall/cap decomposition batch

Previous turn was progress (native exact correspondence owner/protocol), not a blocker. Publisher98069 terminal0: installed public/generated SHA256908dd496b4d4a9391b1453cc86db99dfbed6f70fffeb760cc9ea05da60bd5036,11109013bytes. TS retained wall chart audit is transport-only; Rust owns selection validation, actual face IDs, shared cells, complete unresolved suffix and nonempty union admission. Public74515 terminal0:40/40 across nurbsSweepRetainedCharts,nurbsMiterSweep,miterSmoothStationWalls,miterProfileSmoothness,miterStationSmoothness,sweepViewportEvidence. Typecheck88860 and Vite32020 terminal0. Headless UI49112 terminal0:ui-native-retained-chart-moving-owner-current/matrix.json passedtrue/headedfalse,2 cases/24 checks at1440/600 with908 package source/public/dist/worker identity. Actual reconstructed moving-frame/guide/affine hollow Solid publishes/restores, station G2 reaches reports. Solid rendererCPU/SVG; no WebGPU or unconstrained mid-kernel latency qualification.

New `sweep_retained_decomposition` owns original profile partition, every actual ruled wall domain/world edge, endpoint span correspondence, finite equal transverse weights, complete wall coverage and shared maximum product error in Rust. Native tests certify low-multiplicity original spans; shifted actual walls have bound>=0.125; insufficient products/exact work, edited world edges and missing face coverage refuse without a partial bound. Its separate cap route prepares original endpoint spans, shares products across both endpoints and all rings, then requires existing independently owned retained planar regions. Native tests additionally cover numeric decomposition at simple internal knots, hollow caps, omitted holes, broken actual edges and edge/product cutoffs. These are endpoint curve errors and retained-region premises; they do not alone establish original contour regularity, original filled-region transport, complete continuousBound or global geometry.

Current native BRep target88734 terminal0:2/2 in1.64s (`/tmp/sweep-native-retained-wall-cap-owners-hollow-20261005.log`). Bridge57391 terminal0: shared exact correspondence, wall decomposition, cap decomposition, chart selection and seam ownership protocol; null cap/wall bounds on zero products, cap region complete-bound/global flags remainfalse. Core generic `curve_decomposition_certificate::inspect_batch` also owns the declared pair maximum and one product budget, including nonvacuous empty refusal and failed final-span invalidation; native/protocol24432 terminal0,1/1. Its TS implementation remains active until this opcode is published.

All UI jobs completed before the next single publisher55716 was launched (`/tmp/sweep-native-retained-bound-owners-wasm-20261005.log`). This batch includes exact correspondence, ruled-wall decomposition, cap decomposition and generic curve batch transport together. Full latest native suites also launched:core7852 (`/tmp/sweep-native-retained-bound-owners-full-core-20261005.log`),BRep50499 (`/tmp/sweep-native-retained-bound-owners-full-brep-20261005.log`). Results are pending, not qualified. Do not duplicate/restart these jobs from observation timeouts. No visible browser or focus changes.

Next: inspect these exact live handles, verify final public/generated identity, migrate remaining correspondence/decomposition/cap and generic batch TS wrappers to native transport, run existing detailed refusal/ownership/Rush/viewport regressions, then qualify relevant STEP/UI scope. The complete original goal remains active with all-mode continuous/global/smoothness and actual viewport rendering requirements still unproved.

### 2026-10-05 — full native retained-bound regression and boundary union owner

Previous turn was concrete progress (published chart owner, tested native wall/cap/batch owners). Current full-core7852 terminal0:1218/1218,308.80s. Full-BRep50499 terminal0:714 passed,2 ignored,393.43s. Both compiled before the later conditional boundary certificate addition; do not call them full current1219 qualification. Publisher55716 remains confirmed live (Binaryen PID26337 at high CPU on its immutable input), no restart/duplicate. Its compiled batch includes exact retained correspondence, ruled-wall/cap decomposition and generic curve batch, but not the later boundary certificate opcode.

New native `filled_cap_error::compose_boundary` owns conditional whole-boundary maximum/completeness, tolerance acceptance and structured reasons. A valid complete bound can exceed tolerance while continuousBound remainstrue/withinBudgetfalse. Missing open caps, invalid wall bounds, null/nonpositive/nonfinite budgets and closed cap-free obligations are separate outcomes. Transport parses serialized invalid/null caps conservatively. Original source ownership, full cap premises and global geometry remain independent. Initial protocol83174 failed compilation because custom value-codec JSON array macros do not accept bare null expressions; fixed the fixture with Option::None. Retry69417 terminal0,1/1 passes native and transport cases (`/tmp/sweep-native-boundary-union-protocol-retry-20261005.log`). The TS composer is still active until the new opcode is published; no native end-to-end claim for this new path yet.

Removed duplicate boundary maxima/budget comparisons from both synchronous/streamed TS BRep reports: fields now come from the one existing boundary certificate. Existing `sweepBoundaryCertificate.test.ts`77204 terminal0,5/5,14.56s, covers corrected moving-frame cap budget/Rush/JSON/Solid behavior as well as scalar refusals; typecheck58271 passed. Existing source TS constructor/WeakMap ownership and progression policy are still present and require native migration; do not claim the entire sweep construction is transport-only.

Prepared transport-only retained correspondence/decomposition/cap replacement at `/tmp/sweep-native-retained-correspondence-transport.ts`; repository callers remain on the old implementation until pending opcodes are actually published. Generic curve batch wrapper is likewise not switched prematurely. Next: poll55716, verify package identity, install these wrappers and native batch transport, run detailed original/domain/world-edge/hole/budget/Rush regressions. Then publish/qualify boundary union transport and continue toward native construction ownership plus the full original continuous/global/smoothness/STEP/UI scope. No UI/STEP jobs are live; no visible browser/focus operations occurred.

### 2026-10-05 — published native retained-bound transport, STEP and UI

Previous turn made concrete progress with conditional boundary owner,1218 native core and714+2ignored BRep evidence. Retained-bound publisher55716 terminal0. Installed public/generated SHA256954c5468bd59c2eb857f56809a10b240ba7ac5958822008f486c954edac6d6b5,11130513bytes. Repository `sweepRetainedCorrespondence.ts` now only transports native exact correspondence, ruled-wall decomposition, cap decomposition and actual cap-region audit, with explicit null-bound error responses. Removed TS topology/domain extraction, numeric span maximum and product-budget admission from these routes. Generic `inspectNurbsDecompositionBatch` now transports the entire declaration to Rust.

Public52670 terminal0:54 passed,1todo,7 files,26.92s (`/tmp/sweep-native-retained-bound-owners-public-20261005.log`). Includes detailed source coefficient/domain/world-edge/direction/partition/coverage/hole/budget refusals, original periodic refusals, corrections, Rush/viewport/Solid and source immutability. Typecheck52098 and Vite44512 terminal0. Explicittodo remains `certifies filled caps and admits the moving-axis periodic Rush fixture into Solid`; source miter-periodic-moving-axis-guide-affine-hollow.r is not claimed complete. Current uncorrected case still hasfilled-cap-bound-unproved/cap-region-unproved and Solid correctly refuses.

Headless UI87790 terminal0:ui-native-retained-bound-owners-current/matrix.json passedtrue/headedfalse, same954c provenance,2 cases/26 checks at1440x1000/600x1000. Corrected moving-frame/guide/affine hollow build/Solid, full-boundary cutoff refusal, cancellation/source-change lifecycle and restored actual BRep identity pass. RendererCPU/SVG, not WebGPU qualification; held dispatch does not establish unconstrained native cancellation latency.

Independent STEP pipeline terminal0: export84722; rational boundary reference1125; generator reference completed immediately; OCCT66852. `external-step-native-retained-bound-owners-current/opencascade-sweep.json` passes69/69 and its manifest/output artifactProvenance both match954c/11130513/publicAndPackedVerifiedtrue. Existing OCCT interpreter `/tmp/sweep-structure-occt-venv/bin/python` verified OCP/numpy imports; no dependency install. Ordinary project/global Python lackedOCP and was not used for the oracle. This requalifies listed retained geometry/topology/material/volume fixtures for the new package; original/all-mode smoothness and continuous/global proofs remain independent.

After all UI/STEP jobs completed, one following publisher1199 launched (`/tmp/sweep-native-boundary-union-owner-wasm-20261005.log`) to include the new native boundary certificate opcode. Current full-core58026 launched (`/tmp/sweep-native-boundary-union-owner-full-core-20261005.log`) covers the added1219th test; results pending. Prepared its transport-only TS replacement at `/tmp/sweep-native-boundary-certificate-transport.ts`; current TS composer remains until publication. Both synchronous/streamed BRep diagnostic fields already read the one certificate instead of separately comparing budgets.

Still required: publish/qualify native boundary composition; native source construction/proof ownership and progression policy; solve periodic moving-axis actual filled caps/regularity/global Solid rather than dropping the todo; remaining full original continuousBound, all-mode global/smoothness, broader applicable independent STEP and actual renderer/UI matrix. Original goal remains active. No visible browser/focus operations and no other publishers are live.


### 2026-10-05 — native boundary certificate published; ordinary periodic progression

Publisher1199 completed successfully. Public/generated WASM hashes match bf51c05378bd00506ae412ac65d3dc1bbd6694ee474b87914247e3a5210c2eac,11133233bytes. Installed native `sweep_boundary_certificate` transport in sweepBoundaryCertificate.ts. Rust now owns complete-union maximum, premise validation, budget comparison and refusal reason. Binary transport maps nonfinite numbers to absent numerical premises before encoding; this is wire normalization, not proof admission. Full current native nurbs-core58026:1219 passed,0failed,289.01s.

Public69356:53passed,1failed,1todo across7files. Failure was binary codec rejection of nonfinite test input before Rust. After wire normalization, focused24087:5/5 boundary tests pass,14.49s. Other six files passed in69356. Typecheck18312 and wire-final29776 pass; Vite94771 passes. No proof predicate/test expectation weakened. Independent STEP69/69 recorded above still belongs to954c package; not relabeled as current bf51.

Diagnostic75204 ran the explicit authored-plane periodic moving-axis/guide/affine/hollow fixture with initial steps1,2,4,8,16. All converge to16steps, continuousBoundtrue, boundary upper0.0018079013492281058, actual volume Solid certified and128wallcharts certified using3228cells. Changed authored-caps fixture to initial_steps1 and strengthened positive test to assert initial1/result16. Regression97949:15passed,1todo. Removed own temporary probe source. Raw uncorrected periodic fixture still refuses filled caps; its todo remains. This establishes ordinary progression for explicit authored correction, not automatic correction or all-mode closure.

Current headless UI77836 launched for bf51 corrected moving-frame/guide/affine hollow case at both widths; result pending. No visible browser or focus action. Original full goal remains active: native construction/proof ownership and refinement policy, raw periodic actual-cap completion, full original continuous/global/smoothness scope, remaining independent STEP and renderer/UI combinations.

UI77836 terminal0: ui-native-boundary-owner-current/matrix.json passedtrue, headedfalse; corrected moving-frame guide affine hollow lifecycle at wide/narrow widths on bf51. CPU/SVG scope retained; no WebGPU or unconstrained kernel cancellation latency claim.


### 2026-10-05 — native miter resource/station/preview layout prepared

Previous turn made authoritative progress: boundary union native transport installed, bf51 package, nonfinite wire regression fixed, headless current UI passed and RAGrev249. Added registered public `brep_core::sweep_miter_layout` module and four geometry bridge adapters/opcodes: brep_miter_body_plan, brep_miter_section_partition, brep_miter_sharp_stations, brep_miter_wall_preview. Source decomposition span counting, whole face/cap resource limits, complete ring partition, uniform station/sharp vertex indexing (including cyclic seam), and complete profile/station/span preview surfaces now have native implementations. These are layout/construction candidates, not native proof provenance or new global/Solid certificates.

Focused native98025:3/3 passed; bridge1499:1/1 passed. Reviewing existing native Sweep::new revealed valid refinements beyond64 for short paths. Removed the accidental fixed64 ceiling and added regression for one-edge max256 capped to255 actual body steps. Native50339:3/3 passes with this correction. Superseded publisher90135 was explicitly terminated before publication because its compiled source might contain the erroneous64 limit, not because of observation timeout. Exact tree96361/96383/96824 terminated, session exited143. Public package remains bf51; never installed invalid policy. New single publisher28748 uses corrected source, release compiled1m12s and optimizer live at98.9%CPU (process98677). No other publisher live.

Full native BRep81432 is live but compiled before the later255 regression/policy fix; it is not evidence of full final corrected source. Prepared miterBodyLayout.ts transport and3 public tests; these remain uninvoked until actual opcodes publish. Prepared typecheck8266 passed. Existing geometry/brep.ts consumers still use old TS layout; switch sync/stream plan, section partition, sharp indices and wall preview after publication, then run public layout, stream ownership and smoothness/Rush/Solid regressions. Keep WeakMap/native source-proof ownership and orchestration migration explicitly open.

Extended the existing headless UI source matrix with miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r (ordinary initial1 progression fixture); this addition is not yet qualified. Next publication qualification must include this wide/narrow Solid/cancel/source-change/refusal/restoration case. Existing independent STEP exporter already includes authored periodic case; previous69/69 remains954c provenance. No visible browser/focus operation occurred. Goal active and original full scope retained.


### 2026-10-05 — published native miter layout and prepared native correction union

Previous turn was concrete native layout implementation and focused qualification. Publisher28748 terminal0; public/generated SHA256384f02e1d8874443b61edec692f6c3780aca95193ade319e5b15ea559e9426c1,11144191bytes. Installed miterBodyLayout transport in both sync/stream geometry/brep.ts routes: native source span/face-budget plan, whole ring partition, original sharp station indices and complete native preview wall surfaces. Removed these TS geometry/resource calculations, preserving original request snapshot and preview cancellation boundaries. Source proof ownership remains TS WeakMap, and higher construction/admission orchestration remains to migrate.

Public45686:74passed,1todo,7files,51.20s. Includes new native layout transport tests, refinements beyond64, hollow/cyclic station ownership, omitted profiles, stream mutation/preview-final separation, combined miter laws, filled caps/decomposition and Rush/Solid refusals. Typecheck60264 and Vite35694 pass. Full BRep81432:717passed,2ignored,356.71s; this compiled before the later255 regression and is not claimed full final corrected-source evidence. Corrected native3/3 and bridge1/1 tests were recorded above.

Found another proof component still in TS: shared work, same positive rational basis and maximum displacement across explicitly projected sections. Added registered public nurbs-core sweep_section_projection module plus sweep_project_sections and sweep_project_miter_caps opcodes. Native complete declaration/uniqueness/basis validation; both ordinary path endpoint planes and independently parameterized authored axes; one shared exact-work budget; no partial corrected geometry/certificate on failure. Native57104:4/4 and full57634:1223/1223 passed before endpoint routing addition. Final routing8859:5/5 passes. An interim endpoint test incorrectly guessed equal work at differently positioned planes; corrected it to measure the actual first cap work and exhaust precisely before second cap, preserving shared-budget refusal assertions. No mathematical predicate weakened.

The current384f package compiled before these new correction opcodes. Prepared replacement /tmp/sweep-native-section-projection-transport.ts; do not install until following publication. Existing TS batch projection/endpoint-plane construction still active; next publish must remove those geometry decisions while preserving cap/circle correction negative tests. Native source/provenance ownership and full original scope remain open.

Headless periodic authored-cap UI70961 and independent STEP exporter launched for384f; results pending. Both use actual current initial_steps1 fixture. No publisher or artifact writer may run while these qualification stages are live. No visible browser/focus operation. Original uncorrected periodic todo retained, full goal active.

UI70961 terminal0: current384f periodic moving-axis authored-cap/guide/affine/hollow initial1 fixture passes2cases/22checks at1440/600 widths, headedfalse, CPU/SVG rendering. Build/Solid publication, cancellation, source-change, actual restored BRep identity, invalid-path and bounded correction work refusal pass. This positive explicit correction case does not close original raw periodic todo or WebGPU qualification.


### 2026-10-05 — independent layout STEP qualified; native correction pipeline and headless Metal

Previous turn made authoritative native layout publication/transport progress. Current384f STEP exporter36767 terminal0:69cases, manifest provenance384f/11144191/publicAndPackedVerifiedtrue. Sequential independent rational reference93479 and generator reference both terminal0; OCCT44331 terminal0 and opencascade-sweep.json passedtrue69/69 with same provenance. Includes the periodic authored-cap source now beginning at initial_steps1. This is listed geometry/topology/material/volume evidence, not original raw periodic completion or all-mode proof.

Native full routing7044 terminal0:1224/1224,270.45s (before circle union/pipeline additions). Added registered native circle_sweep_repair, sweep_repair_circle_sections opcode: same positive rational basis over every station, shared work and complete-union displacement maximum. Native48612:3/3 passes (whole hollow family216work, cutoff215, basis, budget and late displacement refusal). Initial failure fixture used a source already exactly representable on its dyadic grid; changed the final station's actual off-grid pole by0.01 to exercise late displacement failure and no partial geometry. No proof predicates relaxed.

Added registered miter_section_correction and sweep_correct_miter_sections opcode: native repair-first/project-caps-last policy, independently requested component work budgets, exact cap-section ownership and outward sum of all applied displacement bounds. Native35472:3/3 passes, including protocol/no-correction/closed-cap/cutoff refusals. Prepared /tmp/sweep-native-section-projection-transport.ts (batch and whole miter path) and /tmp/sweep-native-circle-section-repair-transport.ts; repository consumers remain unchanged until next package. Full current1230 native core60078 and one publisher33522 launched. Publisher release compiled1m13s, optimizer14125 confirmedlive98-100%CPU. No UI/STEP jobs remain live; no second publisher.

Separate isolated headless WebGPU probe /tmp/sweep-headless-webgpu-adapter-probe-20261005.log changes the next UI action: Chromium with --use-webgpu-adapter=swiftshader obtains google/swiftshader adapter, creates device and reads back42; with --use-angle=metal plus WebGPU feature obtains apple/metal-3 adapter, creates device and reads back42. Both headlesstrue with intercepted empty localhost page, no foreground UI/focus action. Earlier default headless CPU/SVG observations remain historical, not evidence that headless GPU is impossible.

Extended existing UI harness with explicit --gpu-backend=auto|metal|swiftshader (defaultauto), recorded actual adapter metadata, and strict GPU readiness/actual Solid WebGPU requirements for a requested backend. No existing geometry/refusal/lifecycle predicate weakened; auto CPU/SVG qualification remains distinct. Syntaxcheckpasses. No product GPU behavior changed. After publisher33522 completion and wrapper installation/public regression/Vite build, run wide/narrow authored periodic and moving frame/circle cases with --gpu-backend=metal; surface/body GPU renderer matrices can now be attempted without visible windows or evidence-only exclusions. Adapter probe alone is not a rendered product qualification.

Native source proof ownership/TS WeakMap and high-level BRep construction/admission orchestration still require migration. Original uncorrected periodic todo, full original continuous/global/smoothness and broader applicable STEP/UI scope stay open. Goal active.

Full current native correction pipeline60078 terminal0:1230/1230,273.17s. Prepared TS batch/circle/whole miter correction adapters typechecked68476 successfully using temporary colocated source copies; those two owned temporary files removed immediately. No repository consumer switched before publication. Publisher33522 still live, no restart.


### 2026-10-05 — native complete correction pipeline published; actual headless GPU qualification launched

Previous turn made authoritative native correction and independent69STEP progress. Publisher33522 terminal0; public/generated SHA256d06c7e0f4633a1dc9b1ec46c50559c7478de65140dd70318cbba2139700320d5,11161261bytes. Installed native batch section projection and circle-section union adapters. geometry/brep.ts now delegates both plain and progressive miter cap/circle correction to native sweep_correct_miter_sections, including endpoint-plane selection, correction order, shared phase work, basis validation and outward sum. Removed TS endpoint plane arithmetic, whole-family displacement maximum and circle→cap orchestration; transport keeps source snapshots/abort boundaries. All downstream retained/global/Solid audits remain required. Native source-proof ownership WeakMap and higher BRep construction/admission orchestration remain to migrate.

Public19732 terminal0:83passed,1todo,9files,52.08s. Includes actual spatial/periodic/authored cap projection, shared budget/no partial geometry, circle+cap ordering, affine laws, Rush units/viewport evidence/Solid, sync-stream parity and original request mutation protection. Typecheck83687 and Vite98417 terminal0, scoped diffcheck clean. Original raw periodic Solid todo remains.

Extended GPU harness source hashes with all active correction/layout/boundary transports. Explicit GPU-backend runs require a real adapter, main viewport canvas ready and actual Solid renderer WebGPU; defaultauto keeps historical CPU/SVG qualification distinct. No prior predicate weakened. Two headless Metal jobs launched: authored periodic moving-axis guide affine hollow at both widths, and decomposed-profile-G1 surface at both widths without evidence-only exclusion. Results pending. No publisher or artifact writer is live; no visible/foreground window operation.

Prior STEP69/69 belongs384f; no claim that it qualifies this newd06c correction package. Next: inspect actual Metal UI result/screenshots, broaden relevant correction/moving-frame/closed-source GPU cases, qualify new current independent STEP as needed, then continue native ownership and original full continuous/global/smoothness scope. Goal active.


### 2026-10-05 — actual headless Metal viewport/Solid qualification on d06c

Four completed current-package GPU matrices at1440/600: periodic authored moving-axis+guide+affine hollow (24checks), decomposed G1 surface (28checks), moving-frame+guide+affine corrected hollow (28checks), spatial circle+cap corrected hollow (26checks). All passed, source/public/dist provenance d06c7e0f4633a1dc9b1ec46c50559c7478de65140dd70318cbba2139700320d5/11161261bytes, headedfalse, actual apple/metal-3 adapter. Surface used actual WebGPU viewport without evidence-only. All six miter Solid publications report actual rendererwebgpu. Cancellation is held-dispatch lifecycle evidence, not mid-kernel latency. Refusals, source changes, restoration and retained identity preserved by unchanged assertions. Artifacts: ui-native-correction-metal-{periodic,surface,moving,circle}-current/matrix.json.

Visual inspection of wide surface preview showed intermittent browser-storage error toast/red unsaved draft; periodic Solid screenshot showed actual rendered hollow Solid. Added diagnostic previewWorkspacePersistence.error reading actual error title. Isolated wide surface repro ui-metal-surface-persistence-probe-current passed with errornull; moving/circle both widths also errornull. The initial intermittent error is not resolved or relabeled as quota; no persistence product change made. Suspected stale async save status is unconfirmed.

Current d06c independent STEP exporter37300 launched, results pending. Additional closed projective C2 surface headless Metal matrix launched, results pending. Existing previous384f69STEP remains historical until current export/reference/OCCT completes. No foreground/focus action, no publisher active. Native constructor/source proof ownership, raw periodic caps todo and original full continuous/global/smoothness scope remain open; goal active.

Current d06c qualification completed: exporter37300, independent rational reference10382, generator reference and OCCT39894 all terminal0 sequentially. external-step-native-correction-current/manifest.json and opencascade-sweep.json confirm69/69, passedtrue, SHA256d06c7e0f4633a1dc9b1ec46c50559c7478de65140dd70318cbba2139700320d5/11161261bytes/publicAndPackedVerifiedtrue; manifestSHAf9016d63ab37ecc8bb1ed414396830e10c03832f7d92a29bd316cef336a78f84. Scope is listed fixture import/topology/analytic volume, not all-mode continuous/seam/containment proof.

Closed authored projective C2 surface Metal88782 terminal0: ui-native-correction-metal-closed-projective-current/matrix.json passedtrue,28checks at1440/600, actualapple/metal-3, no pageerrors, persistenceerrornull, legitimate surface Solid refusal. Total five current headless Metal matrices134checks,10width/mode cases. Scoped diffcheck clean. No jobs remain live. Intermittent surface storage toast still unresolved. Original six-point goal remains active and incomplete.


### 2026-10-05 — native atomic station reconstruction correspondence

Prior turn is progress: actual headless Metal134checks and currentd06c independent69STEP. Inspected current TS smoothCertifiedMiterBody: baseline sections equality and complete preserved cap/trim/edge/vertex comparison still made there. Added registered brep-core/sweep_station_reconstruction.rs: reconstruct validates actual source, reconstructs baseline from requested sections, requires complete Model equality including native topology identities, creates bounded quintic candidate and resulting BRep, validates unchanged filled caps and every cap loop/coedge/world-edge/vertex natively. Candidate failures return modelNone, no partial geometry. Closed mode reconstructs periodic baseline. This is source/cap correspondence only, not boundary premise provenance/global/Solid admission.

Native3877 terminal0: targeted reconstruction test passed including successful source/cap preservation, displacement/work refusal, changed interior section rejection and changed cap rejection. Bridge47001 terminal0: reconstruction_transport test passed, zero-work no model/sides and unrelated sections refusal. Opcode brep_miter_reconstruct_stations registered; dedicated modules formatted, scoped diffcheck clean. TS consumer remains unchanged until new WASM publication; full WeakMap provenance and admission remain open. One publisher launched /tmp/sweep-station-reconstruction-wasm-20261005.log; no UI/STEP job active and no second publisher. Next switch smoothCertifiedMiterBody correspondence/candidate construction to single native result after publication, public mutation/refusal regressions, current artifact evidence. No focus operations. Goal active.

Native station reconstruction continuation: publisher8843 confirmedlive; release finished1m13s, optimizer32903 actual100%CPU, no restart. Prepared /tmp/sweep-station-reconstruction-brep-consumer.ts removing TS baseline construction/JSON equality and cap loop/edge/vertex checks in favor of one native brep_miter_reconstruct_stations result. Typecheck90750 terminal0 using dedicated temporary colocated source; owned temporary immediately removed. Repository consumer deliberately unchanged until opcode package exists. Added tests/miterStationReconstruction.test.ts for actual opcode source/cap/trim/edge/vertex preservation, altered sections, zero-work/displacement atomic refusal, periodic source and open/closed mismatch. Typecheck46086 terminal0, runtime test NOT YET run (old public WASM lacks opcode). Native existing smooth-station regressions57519 terminal0:2/2 shared quintic atomic limits and closed wrap station jets. Scoped diffcheck clean. Next wait exactpublisher8843, install prepared consumer only after package terminalsuccess/current source comparison, run new opcode plus existing miterSmoothStationWalls regression and broader relevant current regressions. No UI/STEP/artifact job may race publisher. Goal active, prior turn progress.

Station reconstruction current full native BRep95186 terminal0:718passed,2ignored,0failed,83.18s, current added reconstruction source included. Current default-feature suite scope only; old814feature scope not relabeled. Publisher8843 still confirmedlive, wasm-opt32903 at5m08s/99.1%CPU. No source mutation/publisher restart/public opcode test during optimization. Inspected raw moving-axis periodic retained cap failure path: native sweep_retained_caps demands exact planar filled region and cap contacts; incomplete raw fixture remains genuine scope, not replaced by corrected positive fixture or weaker predicate. TS prepared consumer remains /tmp/sweep-station-reconstruction-brep-consumer.ts; do not wholesale overwrite after concurrent source changes without comparing. Goal active; this turn yielded full native regression evidence and verified publisher wait.


### 2026-10-05 — native station reconstruction installed and qualified through Solid

Full bridge67484 terminal0:350passed1ignored0failed3.28s, complements full BRep718passed2ignored. Publisher8843 terminal0, wasm-opt12560685→11166947bytes, current public/generated SHA2566aab206b4bb2e6b381397036dd8b3fef5bd1ae33eb80eb9306d475f4ed1a6006. Compared prepared diff against current source then installed only smoothCertifiedMiterBody native reconstruction call; removed TS baseline/JSONmodel correspondence/cap loop-edge-vertex comparisons and second construction call. Native operation returns candidate/model atomically; old WeakMap full-proof ownership and high-level bound/material admission remain TS, not claimed migrated.

Initial new public run78246:15existingpassed2newfailed due fixture errors: single endpoint mutation opened profile before correspondence check; initial closed fixture had first ruling within cap plane and violated native construction premise. Fixed fixtures by translating all poles of interior section and using closed stations with initial cross-plane ruling; no native predicate relaxed. Rerun2526 terminal0:17/17 in4files8.87s. Includes actual opcode complete cap/trim/edge/vertex preservation, source mismatch, atomic work/displacement refusals, closed periodic source/open-mode mismatch and existing owned station/G2/STEP cases. Installedtypecheck34910 and Vite29608 terminal0. Broader public97458 terminal0:73passed1todo7files64.17s. Combined90publicpasses11files1rawperiodictodo. Scoped diffcheck clean.

Actual headless Metal moving-frame+guide+affine hollow reconstruction UI43724 terminal0: ui-native-station-reconstruction-metal-moving-current/matrix.json passedtrue,26checks at1440/600, actualapple/metal-3, source/public/distverified current6aab/11166947, Solidwebgpu66faces1shell same actual geometryhash81d200c58a68257eb8f4e8035ebd99f226013555bebf53d748f8a1c4daef0880, previewpersistencenull both widths, headedfalse. Cancellation still held dispatch lifecycle scope.

Current closed reconstructed frame+guide+affine Metal UI58887 live /tmp/sweep-native-station-reconstruction-metal-closed-ui-20261005.log; current independent STEP exporter53441 live /tmp/sweep-native-station-reconstruction-step-20261005.log → external-step-native-station-reconstruction-current. Poll exacthandles; rational then generator references must run sequentially before OCCT. Do not rebuild/mutate public/dist during these jobs. Historicald06c69STEP and134GPUchecks remain tied to thatpackage. Full raw periodic caps, native source-proof ownership, original continuous/global/all-mode guarantees and intermittent storage error remain open. Goal active, this turn concrete native/WASM/TS/Rush/viewport/Solid progress. No focus actions.


### 2026-10-05 — station reconstruction current STEP/closed GPU completed; native affine boundary transport

Closed Metal UI58887 terminal0: current6aab ui-native-station-reconstruction-metal-closed-current/matrix.json passedtrue26checks1440/600, actualapple/metal-3, WebGPUSolid128faces2shells, stablegeometryhashc97073a3e6ea1637a0fd5b5edcafb918df4265a7473866d38c66351d95259967, no pageerrors/persistenceerrors. Combined current station reconstruction GPU52checks4width/modecases. Exporter53441 terminal0; sequential rational18904/generator/OCCT42353 all terminal0, current6aab/11166947/publicAndPackedVerifiedtrue69/69, passedtrue, manifestSHA53d14a4a89adabeeb7ee3d1815462c676baee38d2872a2c860f8a1ed1a26e500. Listed fixture geometry/topology/volume scope, not all-mode proof.

Added registered native brep-core/sweep_affine_boundary.rs and bridge opcode brep_miter_affine_boundary: source boundary numerical premises recomposed against original budget, exact lattice placement, native outward norm scaling of entire wall and both filled-cap errors, complete new budget decision, no transformed model on budget/work/unsupported arithmetic/overflow failure. This is conditional boundary transport; it does not establish source premise provenance or Solid geometry. WeakMap constructor guard and downstream chart/material audits remain required. Prepared /tmp/sweep-affine-boundary-brep-consumer.ts, source consumer NOT installed before package. Native initialcompile73261 failed wrong error_upper root import; corrected to numerics::error_upper. Native11705 and unique final native run terminal0, full cap scaling/sourcecaps/work/budget/overflow test passed. Original log had overlapping old process output; use /tmp/sweep-affine-boundary-native-final-20261005.log as clean evidence.

Bridge68965/37991 failed unrelated current mechanics Model missing supports initializer in frame adapter after shared workspace change. Added supports:Vec::new() to old strict frame request initializer, preserving old supported request behavior. Bridge83766 affine wire test terminal0 passed. Frame compatibility compilation/test command terminal0; inspect log for actual matching test count, do not infer numerical coverage. No unrelated source reverted.

Prepared typecheck initially failed because node_modules absent; npmci ignore scripts restored production-only deps, reran npmci --include=dev --ignore-scripts, terminal0, lockfile not rewritten. Prepared typecheck36842 terminal0 using owned temporary colocated source then removing it. Scoped diffcheck clean. New sole publisher60069 live /tmp/sweep-affine-boundary-wasm-20261005.log. No UI/STEP live, no second publisher. Next poll60069, install precise prepared affine diff only after success/current comparison, run affine ownership/budget tests and applicable current native/WASM/Rush/GPU/STEP. Original full proof ownership, raw periodic filled caps, continuous/global/all-mode requirements and intermittent storage issue remain open. Goal active; this turn concrete qualification and native implementation progress.

Frame compatibility log confirms6/6 analytical/release/strict-field/singular/envelope regressions passed after empty-support initializer. No mechanical capability added by this compatibility fix.

Affine-boundary continuation: publisher60069 stilllive releasefinished1m27s; optimizer57932 at3m48s93.1%CPU, no restart. Added direct actual-source opcode test to tests/sweepAffineLattice.test.ts for both filled caps, no body on budget failure, shared work0/1 refusal, missing caps refusal, originalsourceimmutability. Runtime NOT YET run, package still6aab lacks new opcode. Current global typecheck38315 terminal2 due newly changed MainSketchTools.vue nullable current/handle closure errors; do not claim new test globally typechecked (earlier preparedconsumer36842 passed before these changes).

Full bridge58197 terminal101 due current shared mechanics-core/frame.rs assemble changed to4args but old calls at1068/1119 still pass3; unrelated to new affine boundary opcode targeted bridge83766 previously passed. Do not discard spring support behavior with empty arguments to force a green sweep check. Full BRep94235 still live, actualprocess59156 at2m21s92.8%CPU; currently long volume_validity affine_closed_hollow exact-witness test. Publisherandnativeprocess confirmedlive by ps, observationtimeout not terminal. Scoped diffcheck clean. No source consumer changed and no second publisher. Native source proof provenance/raw periodic caps/fullcontinuousglobal/smoothness remain open; activegoal. Next exactpoll60069/94235, install precise /tmp/sweep-affine-boundary-brep-consumer.ts after package success and current comparison, run sweepAffineLattice plus station/fullboundary ownership regressions; revalidate whole workspace typecheck/bridge errors from authoritative current source rather than assume persistent.

Affine native full BRep94235 terminal0:719passed2ignored0failed151.45s, current affine transport included. Revalidated globaltypecheck78599 terminal0; previous MainSketchTools errors resolved in authoritative source. Bridge23533 terminal101 on new unrelated manifold-core/repair.rs missing functions (including split_pinched_vertices/fill_boundary_loops), replacing previous mechanics failure after ongoing source changes. Do not claim full bridge green or alter incomplete mesh-repair semantics to force sweep qualification. Updated docs/design/sweep-contract-matrix.md with package6aab native reconstruction90public/69STEP/52actualMetalGPU scope and explicit pending affine/nativeprovenance/rawperiodic/fullguarantee/storage limitations. Publisher60069 remains live, no restart/new publisher, consumer unchanged. Prior turn progress plus exact live wait.


### 2026-10-05 — native affine boundary transport published and installed

Publisher60069 terminal0, optimized12569212→11174499bytes, currentSHA15fab1e36a00d453194e0cbdb80253cfe7b79db8f13dfb36fd80da60e5434fb6. Compared precise prepared diff to currentsource then installed transformCertifiedMiterBody single native brep_miter_affine_boundary call; removed TS norm scaling/cap component traversal/complete-bound recomposition sequence and unused multiply import. Ownership WeakMap and downstream profile/chart/volume audits remain, so complete native provenance/admission is not claimed. Added boundarycertificate mutation refusal to existing ownership test. Typecheck95334/installed22315 terminal0; Vite45828 terminal0. Current public43260 terminal0:16/16,4files14.46s including direct native affine opcode caps/work/budget/missingpremises/originalimmutability, sourcegeometry/certificate/copy ownership, reflection/Rush sync-stream/repeated placement, retained station reconstruction and boundary budget/Solid. Scoped diffcheck clean.

Revalidated whole bridge43335 terminal101: manifold missinghelper errors resolved, current mechanics frame.rs force_n undefined1320 and dereferencing f64factor1354 remain. This is a changing shared source failure, not affine native test failure; do not force spring/load semantics. Prior fullBRep719current passed.

Initial GPU launch failed missing isolated qualification Playwright package. npmci --prefix tools/browser-qualification --ignore-scripts --include=dev from existinglock terminal0. Next launch failed missing pinned chromium_headless_shell-1234 binary; installed only Chromium headless shell using packagecli, job70096 terminal0. No geometry predicates/GPU checks weakened and no headed/focus action. Third headless Metal oblique corrected hollow UI48030 nowlive /tmp/sweep-native-affine-boundary-metal-oblique-ui-20261005.log → ui-native-affine-boundary-metal-oblique-current. New independentSTEPexporter24633live /tmp/sweep-native-affine-boundary-step-20261005.log → external-step-native-affine-boundary-current. No publisherlive and no artifact writer may race jobs. After exporter finishes run rationalreference, generatorreference, OCCT sequentially. Native original provenance/rawperiodic/fullguarantee/storage issue remainopen. Goalactive; thisturn concrete affine native→WASM→TS/Rush qualification progress.


### 2026-10-05 — current affine GPU/STEP and whole bridge verification complete

UI48030 terminal0: ui-native-affine-boundary-metal-oblique-current/matrix.json passedtrue26checks at1440/600, actualapple/metal-3, current15fab/11174499/sourcePublicDistVerifiedtrue, actualSolidWebGPU10faces1shell geometryhash85f6376856840a7bba5a0d800d47de8d2594d1d456b760e9c1cf8e03ae3b7813 both widths, no pageerrors/previewpersistenceerrors. Exporter24633 terminal0; sequential rational43884/generator36238/OCCT34472 terminal0; current external-step-native-affine-boundary-current/opencascade-sweep.json passedtrue69/69,15fab/11174499/publicAndPackedVerifiedtrue, manifestSHA73b74bb78d4b1cf16ede66a40980222e7bd669a456f37461056e386c6f9943ac. Finite fixture import/topology/volume scope, not universal/seam/containment proof. No live artifact jobs/publisher.

Refreshed bridge initially failed current mechanics compile at force_n1320/factor1354. Two expression-only repairs use bound force_n_per_mm in Uniform branch and already-valued factor in load combination; no spring/load behavior discarded. New solve_envelopes supports argument required old strict frame adapter compatibility: pass empty slice alongside existing old request fields (plain solve empty supports initializer already present). Initial core frame regression36760 failed four missing support fields in old test models. A guarded line-based patch detected concurrent source change and stopped before writing any core test fixture; those fixtures were updated in current source independently. Frame regression34790 terminal0:17/17 analytical/loads/combination/singularity/limits. After adapter compatibility, whole geometry-bridge33059 terminal0:351passed1ignored0failed3.41s. Whole BRep719passed2ignored evidence remains currentnewaffine module. Scoped diffcheck clean.

Updated design contract matrix pending-affine paragraph to installedcurrent15fab and exact current16public/26Metal/69STEP/native719/bridge351 qualification. No claim that previous90public tests were rerun on15fab. WeakMap originalmodel/certificate/ownedsections and high-level constructor/final admission remain TS; next principal migration is root native constructor-owned proof provenance rather than claiming allgeometrynative now. Raw periodic captodo/fullcontinuousglobalallmodes/smoothness and intermittent storage problem remain open. Goalactive; this turn concrete current GPU/STEP and whole bridge qualification plus compatibility repairs.


### 2026-10-05 — original-request-owned native miter boundary constructor

Added registered brep-core/sweep_miter_owned.rs with Request containing only original loops/path/scalar laws/optional affine/frame/guide laws/correction budgets and numerical audit limits. Native construction performs actual source-span resource plan/refinement, source frame/tangent/wall admission, ordered whole-family correction, ring partition, retained BRep construction, actual complete wall correspondence or decomposition, exact cap region/decomposition ownership, original ideal endpoint domains and nonsingular cap projection/parallelism, outward wall/correction/decomposition and full cap-bound composition. BoundaryBody keeps model/sections/levels/boundary/sharp/closed/budget private; readonly accessors cannot manufacture or overwrite a certificate from caller-authored scalars. complete_boundary exposes only complete/in-budget bound, explicitly not Solid/material validity. Owned place() derives affine premises from its private actual native body/certificate, not caller numerical evidence. No stateful handle registry yet; native root not bridged/published/installed in TS.

Check14195 terminal0. Initialtest46859 terminal101 due scalar fixture wrongly using one-coordinate controls; corrected fixture to native declared[value,0,0] scalar law, no production rule relaxed. Native73220 then joint97503 terminal0: original corrected hollow10faces completewall/caps, originalprofileimmutability, atomic correctionwork refusal, rooted affine success/newbudget refusal, joint static authoredframe+guide+affine completeboundary with native flags. Currently1 composite targeted test; closed/moving-frame/nonzero-law/caps/resource/mutation/source ownership bridge/WASM/TS qualification remain to expand. Scoped diffcheck clean. No publisher/artifactjobs active. Existing published15fab69STEP/26Metal/16public/719BRep/351bridge refers to before this new root module; do not relabel as full current root qualification. Goalactive; thisturn native root provenance implementation progress. Rawperiodic caps/fullcontinuousglobal/smoothness/storage stillopen. Next expand native root mode/refusal tests, then bridge originalrequest-owned constructor and replace TS proof construction/ownership without exposing a scalar registration shortcut.


### 2026-10-05 — owned constructor closed/moving coverage and original-only bridge

Expanded native root tests: closed square path owns16walls/sharpstations[0,1,2,3], has no cap obligation and refuses requested cap correction; periodic moving-axis+guide+twist+affine original source startsinitial1, rawcaps remain filled-cap-bound-unproved, explicit authored cap correction obtainscompleteboundary at16steps. Combined native83073 terminal0:3/3 in9.16s, alongside previous correctedhollow/ownedaffine/refusal/staticjoint scope. No rawtodo removed or corrected positive relabeled rawproof.

Added registered cad_miter_owned.rs opcode brep_miter_owned_construct accepting original loops/path/law curves/options/limits only. Native defaults preserve named separate audit stage budgets; optional affine components supplied neutral law defaults, framepair presence checked. Explicitly rejects model/sections/boundaryCertificate/sourceCertificate fields, including caller-authored evidence rather than offering scalar-registration route. Encodes actual native model/sections/sharp/closed/budget/completeboundary and limitedleveldiagnostics, explicitlyfalseSolid/globalembedding. NativeprivateBoundaryBody remains constructor-owned; no lifetime/handle registry and TS WeakMap replacement yet. Check63378, bridge75702 and routeddispatch95342 terminal0; protocol/routing and injection refusals passed.

New public tests/miterOwnedBoundary.test.ts compares actual original-request native model and boundary against existing source pipeline, originalrequestimmutability/injectedmodel-section-certificate/sourcecertificate rejection/atomiccircleworkrefusal. Typecheck95228 terminal0; runtime NOT YET run before opcode publication. Scoped diffcheck clean. Solepublisher28702 live /tmp/sweep-miter-owned-wasm-20261005.log; fullBRep82429 live /tmp/sweep-miter-owned-full-brep-20261005.log. No UI/STEP live, no secondpublisher. Next exactpoll28702/82429, publicnativeownedparity/refusaltest, then integrate originalnativeconstructor result and nativeproof ownership with clear worker/lifetime semantics; do not fake provenance by registering caller scalars. Existing15fab finitequalification remains historicalpackage; fulloriginal scope/rawperiodic/storage issue open. Goalactive; thisturn native coverage and bridge implementation progress.


### 2026-10-05 — original-only WASM parity and full native miter diagnostics

Previous goal turn was a verified wait: exact publisher28702 reached terminal0, while a separately launched native optimizer95261/public packer remained live. Waited for both public writers to terminate before runtime verification; did not interrupt external work or use foreground UI. Resulting installed f88b523dd48381ef83523529cfa32d6b05da1794fe44ac19f532f9c26f57da8f held the same public SHA before and after focused runtime tests. Public26139 terminal0:17/17 in5files/22.56s. Original-only native constructor matches current legacy hollow model and boundary certificate exactly, rejects injected model/sections/boundaryCertificate/sourceCertificate and atomic circle-work exhaustion; existing affine/station/boundary regressions also pass. This package predates the following diagnostics/independent-cap-budget changes. Full prior current BRep82429 terminal0:722passed2ignored; full bridge15185 terminal0:355passed1ignored. Typed original-request TS transport and53084 typecheck pass; transport explicitly disclaims serialized-copy authentication or Solid admission. Existing production constructor and proof WeakMap remain.

Removed duplicate hand-maintained native report mappings: progressive_miter::Report now implements shared value_codec::Serialize with every existing level field, including phase/frame/error/regularity reasons and cells, unresolved wall patches, closed holonomy, law application/method and explicit C0/open semantics. Existing curve transport and owned-body bridge both use that full native serializer. No promotion of per-level continuousBound/roundingCertified (both remainfalse); full boundary remains separate. TS owned report type reuses ProgressiveMiterReport. Added independent native domain_exact_work and projection_exact_work; optional camelcase domainExactWork/projectionExactWork fall back to existing exactWork for compatibility, without changing retained-wall/cap-region budgets.

Native74614 terminal0:3/3 root tests, including independent cap-phase exact-work exhaustion retaining the same wall error but withholding complete_boundary; bridge24800 terminal0: source-only routing/injection checks and exact full-level equality to existing progressive curve transport. Native26092 terminal0:109/109 progressive_miter regression. Typechecks80002/13837 terminal0. Structure4653 terminal1 due newly concurrent unrelated manifold-ops direct path dependency ownership (polygon-core/math-core); no sweep module registration error or unrelated manifest edit. Earlier structure17176 passed at42packages; current checker sees43packages. Expanded public test source adds full history parity, affine+authored+guide original construction, distinct cap-phase budgets and closed C0/no-caps/refusal; runtime pending updated publication.

Publisher84601 currently owns the new diagnostics WASM publication, using installed matching native Binaryen executable through the existing build script. Release finished1m40s; optimizer4903 completed12604982→11205769bytes; packaging remains live. No second publisher/UI/STEP jobs launched by this task. Next poll exact84601, fingerprint final package, expanded public regressions; then complete original-request production adoption/native admission provenance, keeping stream/cancel/full diagnostic budgets intact. Raw periodic cap/Solid todo, all-mode continuous/global/smoothness and intermittent persistence failure remain unproved. Goalactive, no focus theft.


### 2026-10-05 — full diagnostics WASM and moving-source qualification complete

Publisher84601 terminal0. Final public and packed WASM match SHA659562122ad80895938e92cb599ca6ddfe850169985754c0c168116f65fb90d6,11205769bytes. Current diagnostics public78946 terminal0:11files/78passed1todo in56.60s; includes original-only full model/boundary/level parity, independent cap-domain/projection budget refusal, simultaneous affine/authored-frame/guide reports, closed sharp C0/cap-free construction, existing Rush/law/wall/station/reconstruction/reflection/retained-decomposition suites. Same publicSHA confirmed afterward. One raw moving-axis periodic cap/Solid todo remains intentionally open.

Added a direct original-only public moving periodic fixture (original degree2 periodic outer+reversed quarter hole, varying authored axis+guide+nonzero twist+affine axes+center). Public69455 terminal0:4/4 root API tests in4.11s. Raw fullcap bound remains filled-cap-bound-unproved; explicit authored cap correction yields16steps/fullcontinuous/in-budget<.01 with both capbounds, retains originalrequestimmutability and explicitlyfalseSolid. This is not raw cap closure or native material admission. Latest typecheck76380 terminal0. Updated design contract records private ownership scope, shared full diagnostics, independent cap budgets, current transport qualification and remaining production/WeakMap/native admission work. Current global structure failure remains the separately changed manifold-ops dependency-owner manifest issue, not swept under a green result. No publisher/UI/STEP job live. Full original objective remains active: production root adoption and native source ownership/admission; raw/fullcontinuous/all-mode global/moving smoothness; complete STEP/UI requirements and intermittent storage failure remain unproven.


## Production original-only miter constructor qualification (2026-10-05)

The synchronous and streamed progressive miter factories now construct from original requests through `brep_miter_owned_construct`. Rust owns refinement, section correction, retained decomposition, complete wall/cap error composition, tolerance gates and corrected wall chart regularity. TS normalizes transport units and attaches native diagnostics. Streamed previews retain cancellation and immutable source snapshots; preview/source mutation tests pass. Returned JSON is not an authenticated resident native body. TS WeakMap provenance comparisons and later smoothing/transform admission remain completion obligations; general sweep ownership also remains open.

Installed public WASM SHA-256: `23e8ba33e5d250ff924c1d50ca2b9881f028ea32939fbeb43b48e9b887ddff7f`, 11,209,402 bytes. Public regression: 102 passed, one explicit raw periodic filled-cap/Solid todo. Five direct original-only tests include immutable pre-adoption model/certificate baseline, joint affine/frame/guide laws, closed sharp C0, explicit periodic authored-cap correction, and streamed mutation/cancellation. Typecheck, Vite and library structure checks pass. Native full suites passed 722 BRep tests (2 ignored) and 355 bridge tests (1 ignored), before final refusal-text changes; subsequent targeted root 3/3 and bridge 1/1 passed.

Three headless Apple Metal/WebGPU UI matrices passed 78 checks across six cases at 1440/600 widths: moving corrected hollow (66 faces/1 shell), closed reconstructed hollow (128 faces/2 shells), and periodic explicit authored caps (130 faces/1 shell). All recorded null page/persistence errors. Held-dispatch cancellation proves lifecycle handling, not interruption latency inside a running WASM kernel. Earlier intermittent persistence failure remains undiagnosed. No foreground window activation is required.

Independent rational-boundary and generator references followed by OCCT passed all 69 STEP fixtures on this package. Scope: fixture import, topology and analytic volume; surface/seam/containment matrix remains separate. Evidence: `external-step-native-owned-constructor-current`, `ui-native-owned-constructor-moving-current`, `ui-native-owned-constructor-closed-current`, `ui-native-owned-constructor-periodic-current`.

The station reconstruction fixture has a certified complete bound about 0.900000000000032 mm. Independent legacy/native phase parity shows the source wall bound remains 0.5000000000000305 mm with identical caps; 1 mm is therefore a legitimate success. The regression refusal budget is 0.8 mm and preserves source-error accounting. Evidence: `native-owned-constructor-bound-parity.json`. Native refusal wording preserves existing product compatibility. A minimal manifold-ops manifest repair uses workspace math-core/polygon-core ownership and explicitly preserves polygon codec; locked cargo check passes.

Remaining full-goal obligations: native certificate ownership/admission across modifiers and worker lifetime; raw periodic caps; all-mode continuous/global/regularity/nesting/orientation guarantees; applicable moving-frame/multispan/closed-seam G1/G2; wider independent STEP and UI coverage. Finite green matrices do not establish universal guarantees.


### Private native station-boundary reconstruction (2026-10-05)

`BoundaryBody::reconstruct_stations` now derives model, sections, sharp stations and source wall/cap bounds solely from private constructor state. It composes outward-rounded wall displacement with source error, preserves cap bounds, and withholds candidate geometry on an unproved/exceeded target budget. Missing source completeness or exhausted candidate work returns no admitted reconstruction. Cargo check and all three owned-root tests pass, including new positive, zero-budget atomic refusal, cap-bound preservation and zero-work cases. This is native API evidence only: bridge/worker ownership, WASM publication and production smoothing migration remain pending. No Solid/global admission is implied.


### Original-request replay station reconstruction in production (2026-10-05)

`brep_miter_owned_reconstruct` replays original construction in the receiving realm, then calls private native station-boundary reconstruction. Supplied model, sections, sharp indices and numerical certificate fields are rejected. Native bridge replay tests pass including budget/work refusals and cap-bound preservation. Production smoothing now retains an immutable original wire request and consumes the native complete-boundary result; TS error addition/composition has been removed. TS WeakMap unchanged-object and requested-section checks, plus actual-chart/Solid admission gates remain and must still migrate; no complete native provenance/admission claim is made. Original replay incurs repeated construction work and is not interruption inside a synchronous kernel.

Published WASM SHA-256 `0be36bd5584319bd0438df6a3ed1237ac5b9c1b84f265c0273040562da2590bc`, 11,216,267 bytes. Typecheck passed. Four public files passed all 22 tests: miterOwnedBoundary, miterSmoothStationWalls, miterStationGraph, miterStationReconstruction. This includes Rush synchronous/asynchronous reconstruction and viewport evidence, immutable original inputs, injected-proof refusals, cap preservation and exhausted budget/work. Current UI/STEP matrices remain those of the previous package and must be refreshed before qualifying this package for those scopes.


### Native material admission for original-request station replay (2026-10-05)

The original-only replay opcode optionally requires native material admission. Rust derives cap selection from the owned body, inspects the complete wall chart set, and invokes native sweep volume validity on the actual reconstructed model. Derived model/cap fields overwrite any budget-object fields before inspection. Candidate geometry is withheld unless wall regularity and material volume both certify. Boundary evidence remains separately visible on material refusal. Production station reconstruction now requests this mode, consumes its retained charts and volume reports, and removes TS source-bound, target-budget and chart/volume admission predicates. TS provenance/section comparisons and affine admission still remain.

Bridge positive material and zero-wall-cell atomic refusal tests pass. Published WASM SHA-256 `42c172e036e9e05f473ad42258531aa6a22a45813c02515af9150f0e186cea52`, 11,218,193 bytes. Typecheck and Vite pass. Five public files passed25 tests, including original-only transport, smooth station walls, Rush station graph, station reconstruction and closed combined miter. A headless Metal UI run for closed reconstructed frame/guide/affine hollow is started but not yet qualified here. General all-mode/source-global guarantees and raw periodic caps remain open.


### Native admitted reconstruction UI and original-only affine transport (2026-10-05)

The preceding42c172e0 package passed the closed reconstructed frame/guide/affine hollow headless UI matrix:26 checks at1440/600, actual Apple metal-3 adapters, WebGPU Solid,128 faces and2 shells; geometry SHA `c97073a3e6ea1637a0fd5b5edcafb918df4265a7473866d38c66351d95259967`. Evidence: `ui-native-owned-solid-reconstruction-current/matrix.json`. Held dispatch still proves lifecycle rather than mid-kernel interruption. This does not qualify subsequent source edits or all-mode geometry.

`brep_miter_owned_place` now reconstructs original source privately and derives affine numerical premises through `BoundaryBody::place`; supplied model/sections/sourceCertificate/boundaryCertificate fields are rejected. The existing affine report codec is shared without changing its shape. Native original-only tests pass success, zero-target-budget, zero-work and injected-evidence refusal, and the legacy conditional affine transport regression passes. This opcode remains unpublished and production affine use/material admission/provenance still need migration. No full-goal closure is claimed.


### Original-only affine replay and material gate integration (2026-10-05, pending publication)

Native affine replay now accepts a bounded following-placement history, derives each intermediate numerical premise from the preceding native result, and requires actual final wall-chart and sweep-volume evidence when requested. Failed material admission clears candidate geometry. Native tests pass repeated placement, successful material admission, zero-chart-work refusal and oversized-history refusal. TS affine use routes original construction and immutable placement history through this opcode; source-bound, target-bound and material numerical gates no longer decide affine admission in TS. Typecheck and scoped diff-check pass. WASM publisher is running; current installed package42c172e0 does not yet contain this opcode, so runtime qualification is pending.

Remaining provenance policy uses TS WeakMap comparisons. The general operation history must also cover affine placement following station reconstruction (smoothed body original/history transfer is not implemented in the current replay path); full mixed-operation parity is not established. Existing raw periodic-cap and all-mode continuous/global/smoothness requirements remain open.


### Affine replay runtime qualification and mixed reconstruction replay (2026-10-05)

Published original-only affine replay package `de9f7b5b8be3f85ac9154b586e355494cada250f89ff4085419c0d9a462ba93a` (11,223,068 bytes) passed24 tests in sweepAffineLattice, miterOwnedBoundary, miterStationGraph and miterSmoothStationWalls. Tests cover corrected hollow reflections/shears, repeated Rush placements, sync/async viewport evidence, budget/work/arithmetic refusals, immutable original input and direct injected-proof/material refusal. Typecheck passes. UI/STEP qualification for this package remains pending.

Subsequent native edits add original-only reconstruction-before-affine replay, deriving placement premises from the private native reconstruction result. TS carries immutable reconstruction settings along with original request and affine history. Native tests pass positive combined material admission and exhausted reconstruction refusal; typecheck and diff-check pass. This mixed path is not yet published or publicly runtime-qualified; the new WASM publisher is running. TS object binding and further modifier operation combinations remain open, alongside original all-mode geometric requirements.


### Mixed reconstruction/affine public qualification (2026-10-05)

Published WASM `2dfe7aa4b90ce6a6f0abbcc7026e94186ee97713b60ba3fd011e004d20d826aa` (11,224,643 bytes) includes native reconstruction-before-placement replay. New public mixed-operation regression passes reconstruction followed by shear and reflection, complete bound/material preservation, original geometry immutability and zero-target refusal. Typecheck passes. Parallel14-file run:103 passed,1 explicit raw periodic-cap todo,1 streamed Rush topology test timeout at30s. The exact timed-out test passed in isolation in21.43s without changing timeout or predicates; this supports contention as a cause but is not a green aggregate run. A no-file-parallelism rerun of the same14 files is started and pending. Native private proof/model binding and all-mode guarantees still remain open.


### Native reconstruction source-binding checks (2026-10-05, unpublished)

Original-only reconstruction now optionally validates expectedSourceModel, expectedSections and expectedSharp against the privately replayed original body before work. These fields are comparisons only, never numerical proof premises. Production smoothing sends actual source model and requested sections/sharp indices; TS section/sharp JSON equality is removed. Native transport tests pass valid equality and unrelated-model, empty-section and changed-sharp refusals. Typecheck passes. TS unchanged model/certificate WeakMap guards still remain. Publication and direct public refusal qualification of these new fields are pending. The prior package2dfe7aa4 sequential14-file test process remains live; TS/source edits during that run mean it cannot qualify the new native-binding behavior. Do not publish while its installed-WASM checks are running; finish that process, publish binding changes, then run focused runtime checks with stable source/artifact identity.


### Reconstruction proof binding and numeric wire regression (2026-10-05)

Native reconstruction additionally compares the supplied expectedSourceCertificate to the replay-derived certificate. Production smoothing removes TS model/certificate JSON comparison; WeakMap remains only identity/source-request/metadata routing for this path. Native model/certificate/section/sharp positive and tamper-refusal tests pass. Affine object provenance comparison remains TS.

Prior sequential14-file run finished103 passed/1todo/1failed; the newly edited native-binding public test loaded against the old installed opcode and failed to reject an altered model. The new binding WASM then produced18 passed/7failed in four focused files: the numeric codec distinguishes integer2 from float2.0, causing integral-budget certificates to reject after a TS JSON round-trip. Native comparison now uses exact f64 semantics for certificate numeric fields, with strict array/object/string/boolean/null structure and key coverage. The replay-derived values remain the only proof premises. A native integral-budget round-trip regression passes, alongside actual certificate/model/sharp/section tampering refusals. Corrected WASM publisher is running; runtime requalification is pending. Do not describe either failed public run as green.


### Fixed native reconstruction binding runtime qualification (2026-10-05)

Corrected package SHA-256 `a4dd6940cc7d3272078cb28fd9c358eb36c8c52506f1f4b0acc557944716540f`, 11,231,621 bytes. Four public files passed all25 tests: miterStationGraph, miterOwnedBoundary, miterSmoothStationWalls, sweepAffineLattice. This requalifies integer-budget JSON round-trips and actual model/certificate/section/sharp tamper refusals, plus reconstruction, native material gate and mixed/repeated affine replay. Public WASM, bridge source, production constructor/transport and owned-boundary test hashes were identical before/after the run. The previous18/7failed regression run remains historical failure evidence.

A stable-source no-file-parallelism14-file matrix is now running; its aggregate result is pending. Do not mutate proof sources or installed artifacts during this qualification. Native reconstruction provenance equality and admission live in Rust; TS still routes identity, original inputs and operation history. Affine unchanged-object equality still resides in TS. Raw periodic caps, all-mode continuous/global/regularity/smoothness guarantees and refreshed STEP/UI scopes remain incomplete.


### Stable full public matrix after native reconstruction binding (2026-10-05)

On package `a4dd6940cc7d3272078cb28fd9c358eb36c8c52506f1f4b0acc557944716540f`, the same14-file matrix passed104 tests with one explicit raw periodic-cap todo, no failures or timeouts, under no-file-parallelism. Duration165.40s; native source/production transport/owned-boundary test and public WASM hashes remained stable during qualification. This supersedes the earlier mixed-source failed aggregate runs for these tested scopes. Full original all-mode requirements remain incomplete. A fresh STEP exporter for `external-step-native-bound-proof-current` is running; independent rational/generator/OCCT stages must follow sequentially. Affine source binding remains TS and is the next native migration obligation after stable qualification.


### Independent STEP matrix for native reconstruction proof binding (2026-10-05)

Package `a4dd6940cc7d3272078cb28fd9c358eb36c8c52506f1f4b0acc557944716540f` passed the fresh69-case independent STEP matrix. Export, independent rational-boundary volume, independent source-generator volume and OCCT stages finished successfully and sequentially. Manifest public/packed identity matches this exact11,231,621-byte package. Evidence: `external-step-native-bound-proof-current`. Scope remains fixture import, topology and analytic volume; general surface/seam/containment coverage is separate. Vite build passed. Headless Metal UI qualification for the current closed reconstructed frame/guide/affine hollow fixture is running in `ui-native-bound-proof-current`; no UI completion is claimed yet. Sources and installed WASM remain stable.


### Qualified reconstruction UI and native affine source binding (2026-10-05)

Stable package a4dd6940 passed26 headless UI checks at1440/600 in `ui-native-bound-proof-current/matrix.json`: Apple metal-3 adapters, WebGPU Solid,128 faces and2 shells, geometry SHA `c97073a3e6ea1637a0fd5b5edcafb918df4265a7473866d38c66351d95259967`. The run ended before subsequent source edits. Existing104 public tests and69 independent STEP fixtures qualify the same package for their declared finite scopes.

Subsequent affine transport edits compare optional expectedSourceModel/expectedSourceCertificate against native original/reconstructed geometry, or the replay result immediately before the final placement in a following-placement history. Numeric certificate comparison uses exact f64 semantics across wire number tags. Native positive/altered-model/altered-certificate tests pass, and repeated history binding accepts the actual intermediate source while rejecting the original root in its place. Production affine removes TS JSON equality; obsolete saved model/certificate JSON is removed from WeakMap. WeakMap now routes object identity/original request/station metadata only. Final typecheck is running/pending at this audit entry; a new WASM publisher is running. This new binding still requires public runtime qualification; earlier104/69/26 results are historical to a4dd6940, not proof of the new source state. Full geometry/global/smoothness requirements and raw periodic-cap todo remain open.


### Published native affine binding qualification (2026-10-05)

Package `b3df9d2b2e64e06cad9b4084541b75f9a7d9ba0b4d7abb806084e55bcd1cac98` (11,234,020 bytes) passed26 tests across sweepAffineLattice, miterOwnedBoundary, miterStationGraph and miterSmoothStationWalls. New intermediate-source regression mutates an already placed certificate and model before another placement; Rust rejects both against native replay. Original/root, repeated and reconstruction-before-affine positive cases pass. Typecheck and scoped diff-check pass; public WASM, production factory, native bridge and proof test hashes stayed identical before/after runtime tests.

Neither reconstruction nor affine consumers retain TS JSON model/certificate evidence comparisons. WeakMap routes identities and immutable original inputs/station/operation metadata; native replay reconstructs geometry/proofs, compares requested actual source, and gates resulting boundary/material admission. This is not a resident transferable body arena, and general sweep/source ownership, arbitrary operation histories, raw periodic caps and all-mode continuous/global/smoothness guarantees remain distinct incomplete obligations. Previous104/69/26 qualification belongs to a4dd6940; this b3df9d2b package still needs refreshed broader STEP/UI qualification.


### Raw periodic moving-axis failure isolated (2026-10-06)

Current b3df9d2b package diagnostic evidence is saved in `raw-periodic-cap-diagnostics-current.json`, with source and WASM SHA identities, original phase reports, actual cap surfaces and independent exact binary64 planarity determinants. Ideal endpoint material domains certify. Raw complete bound fails filled-cap obligation: retained cap decomposition reports cap-region-unproved; original projection/parallelism reports retained-plane-unproved; terminal contour geometry reports planar-contours-unproved. The start bilinear cap controls are exactly coplanar; the final four controls have a nonzero exact rational determinant (approx -7.944109290391252e-16). A stronger planarity predicate cannot certify this represented nonplanar cap. Geometry construction must change, with its displacement charged to the full bound and actual coedges/material regions requalified. Existing explicit authored cap correction remains a distinct successful mode, not closure of the raw fixture.

Raw wall chart union consumes the default1000-cell budget and leaves faces44..127 unproved through cell-budget-exhausted. Reinspection of the identical native model at10000 cells certifies all128 walls using3228 cells. This isolates work exhaustion from invalid wall geometry; no caller-declared budget was silently increased for admission. Raw filled-cap and global/Solid obligations remain open. Next work is native cap construction/controlled correction plus explicit whole-wall work policy, rather than weakening proof gates.


### Native bounded stabilization of nonplanar miter caps (2026-10-06, runtime pending)

Original-only construction now checks actual bilinear caps with a bounded exact binary64 orient3d predicate. A proven nonzero determinant, open body and omitted explicit cap correction trigger native endpoint projection on a2^-40 lattice, with displacement tolerance min(maxDeviation,1e-9). Authored axes are used when present. Detection work is included in the correction report and subtracted from the bounded automatic correction work allowance. Original circle correction is composed with cap displacement; original requests/level evidence remain intact. The corrected model is reconstructed before correspondence, cap regions, source projection, complete wall/cap error union and wall regularity gates. No predicate is softened. Correction report reason: automatic-bounded-cap-planarity. Unsupported or exhausted stages retain refusal/unproved outcomes.

All3 native original-owner tests pass, including the formerly raw moving-axis periodic case now deriving an in-budget complete boundary, zero cap-proof work withholding completeness and zero wall-cell work refusing geometry. Bridge source-only regression and typecheck pass. Omitted wallCells/retainedWallMaxInjectivityCells default increases from1000 to10000; explicit caller budgets remain unchanged. The existing original periodic Rush fixture is unchanged. Public tests now require its complete bound and actual Solid admission, replacing the former TODO, but these assertions are not yet runtime-qualified. A new WASM publisher is running. General cap/frame families, broader regressions and independent STEP/UI still need verification; this entry does not close the raw fixture until public Solid and independent evidence pass.


### Automatic nonplanar caps: public, independent STEP and headless Metal qualification (2026-10-06)

Published WASM SHA256 4e58d9e402de07b80a6100ac3d564a1c130fce4ef2dd80b4c16ff76e53c292f8, 11237267 bytes. First serial four-file public run:58 passed/1 obsolete expectation failed; corrected the refusal test to explicitly request wall injectivity limit1000 (old default). Recheck decomposition15/15 passed; together59 targeted cases passed. Original raw Rush fixture now has native automatic-bounded-cap-planarity, complete/in-budget boundary and successful native Solid admission. No original fixture geometry or predicates weakened. Native3 owned-body tests,1 bridge test, typecheck and scoped diffcheck passed earlier.

Independent STEP exporter now includes automatic caps separately from authored caps. Export56220, rational reference5536, generator reference92598 and OCCT4617 all terminal0, sequential manifest stages. external-step-automatic-caps-current manifest and opencascade-sweep.json match exact current public/packed WASM and pass70/70. New automatic cap case:130 faces,264 manifold opposite-use edges,1 solid/closed oriented shell,2 holed cap faces; exact cap coplanarity, full-domain cap16coedges and wall512pcurves preserved. OCCT volume22.92781437975148 vs independent boundary integral22.92781437975149, relative error3.0990426038498033e-16.512 material-side probes agree. Listed finite fixtures only; general all-mode/smoothness/containment proof remains separate.

Vite build17752 terminal0. Headless Metal UI40211 terminal0; ui-automatic-caps-current/matrix.json passedtrue on same current WASM. Original unchanged miter-periodic-moving-axis-guide-affine-hollow.r passed12scenario assertions at1440 and12at600, including actual renderer/Solid, refusal, held-dispatch cancellation, source change and restoration. Held-dispatch cancellation is lifecycle evidence, not mid-kernel interruption latency. No foreground/focus operation.

Full original goal remains active: broaden all-mode continuous/global/regularity/smoothness guarantees, broader fresh native/public matrices and general operation ownership; finite new STEP/UI success does not establish universal completion.


### Expanded automatic-cap regression and fail-closed composition guard (2026-10-06)

Full BRep run92843 terminal0:722passed,2ignored,121.25s (`/tmp/sweep-auto-cap-full-native.log`). Public serial86246 terminal0:136passed in19files,245.51s (`/tmp/sweep-auto-cap-expanded-public.log`), installed4e58d9e4 package unchanged. Expanded regression includes original/retained/closed-frame smoothness, reconstruction, affine, correspondence/wall audits and viewport evidence, not universal all-mode proof. Updated docs/design/sweep-contract-matrix.md to supersede historical raw-cap and TS proof-comparison limitations while preserving general modifier/worker/all-mode obligations.

Subsequent Rust source hardening: automatic correction now explicitly errors if outward composition of previous and cap displacement cannot produce a finite upper bound. Previously an absent composed bound could reach a later unwrap_or(0); no demonstrated admitted fixture overflow, but missing evidence must never become zero error. Focused source run41340 terminal0:3owned-root tests passed,73.86s. This guard is not yet in installed WASM; all136/70STEP/24UI claims above remain tied to pre-guard4e58d9e4 artifact. Concurrent checkout additions are visible: the later focused native runner reports730filtered tests, so the earlier722 full suite is not claimed to cover every current unrelated addition. Goal active; publish and requalify guard, then extend actual all-mode requirements.


### Fail-closed automatic displacement composition published (2026-10-06)

Publisher84126 terminal0, `/tmp/sweep-cap-bound-guard-publish.log`: Rust release1m48s, actual wasm-opt optimization completed without restart. Installed public/packed SHA256377fc1944ea2444d6a33c2c7cebbe574b7073f40176dc509c71fe494613f2005,11237511bytes. Explicit native failure on missing/nonfinite composed automatic-cap displacement now reaches WASM. Serial public87638 terminal0:29/29 in4files (ownedboundary,retaineddecomposition,affinelattice,stationreconstruction),97.72s. Raw original periodic Rush→Solid, explicit-work refusal, source model/certificate binding and mixed reconstruction/affine history passed. The earlier136public/722native/70STEP/24UI aggregate remains tied to previous4e58d9e4 package; no new broad STEP/UI claim. Full goal active.


### Automatic-cap affine combination exposed missing native arithmetic coverage (2026-10-06)

New original periodic automatic-cap Rush shear+reflection test initially built through generic transforms but failed actual Solid boundary embedding (`/tmp/sweep-auto-cap-affine-combined.log`). Diagnostic recomputation:allFacesInjectivefalse/allPairsClassifiedfalse,nextPair[43,49]; only root construction report existed. Root cause: exactMiterPlacements registered only explicit circle-corrected constructors. Automatic-cap originals therefore bypassed native proof-aware placement. No claim of transformed validity is made.

Rush now registers automatic-bounded-cap-planarity bodies for original-request native affine replay, using the existing placement quantum/work options. Other previously generic paths remain unchanged pending proper native coverage. Native replay explicitly refuses the new source with point-lattice; corrected endpoints alone do not place all interior coefficients on the quantum lattice. Safe routing regression12422 passes7tests/1explicit success TODO; original raw body still admits Solid, graph immutability checked, no silent generic transformation bypass. Added native automatic correction reason to transport type. Positive affine+automatic periodic caps remains a real full-goal gap requiring native bounded arithmetic outside the exact placement lattice, not a completed feature. No geometric algorithm moved to TS and no predicate weakened.


### Native non-lattice affine arithmetic implementation (2026-10-06, publication pending)

Rust affine_lattice now proves zero arithmetic error for signed axis permutations with zero translation on arbitrary finite represented source coordinates, independently of the lattice. Non-lattice integer-matrix shear now has place_bounded: interval evaluation of ideal transformed poles versus actual native transformed poles gives an outward Euclidean maximum. Positive rational weights extend that maximum to the full curve/surface image, including trimmed subsets; topological/geometric material validity remains separate and must be rerun. Exact-path work and bounded-path work share the caller allowance; exhausted work returns no model. sweep_affine_boundary adds arithmetic displacement outward to each wall/cap transported error and withholds geometry if the complete target boundary budget fails. Fractional matrices and other unsupported matrix/translation predicates retain explicit refusal.

Native focused affine_lattice71766 terminal0:3passed (off-grid reflected axis permutations, prior exact shear/refusal, bounded non-lattice shear and atomic work/target refusal). Initial new test used zero source bound/budget and correctly received no source certificate; fixed its premises to a nonzero admitted source budget, not a weakened predicate. Dedicated composition65868 terminal0:1passed. rustfmt selected modules, scoped diffcheck clean. Broader affine-filter61624 still running a long material-validity test; its completion is not assumed. Publisher started for new source (`/tmp/sweep-bounded-affine-publish.log`), pending. Installed377fc194 still has point-lattice refusal; the public automatic-cap combined success TODO remains unclosed pending new package and independent actual-material checks. No complete all-mode claim.


### Non-lattice affine WASM qualification and actual remaining material gap (2026-10-06)

Publisher44624 terminal0: geometry SHA2561aa44ce108dc19827c3c5b63cb88cb9e841127f824e56dba7b0ce8944a3857cf,11245318bytes. Broad native affine61624 terminal0:17passed/1ignored,184.90s (pre-final added bounded test compilation); subsequent dedicated3lattice and1composition tests passed. Publication includes native signed-axis permutations and bounded integer shear arithmetic.

First public30112:7passed/1failed/1todo. Only failure was obsolete point-lattice expected refusal: actual automatic-cap shear reaches native material guard and refuses affine-solid-geometry-unproved. New exact off-lattice reflection passed original periodic Rush→Solid, zero arithmetic error, complete boundary, actual native volume admission and immutable graph. No claim of shear material success. Updated generic product error wording from Exact affine placement to Affine placement because bounded paths are now supported; updated refusal regression to actual native material failure. Final public49059 terminal0:8passed/1explicit combined-success TODO,59.35s. Typecheck78171 and scoped diffcheck passed. Remaining concrete gap: automatically corrected moving-axis periodic body under shear has a composed bound but native actual geometric material validity remains unproved; retain refusal while diagnosing wall/cap/embedding predicates. Broader STEP/UI matrix not yet refreshed for1aa44ce1. Full goal active.


### Bounded shear material refusal localized to charts/pairs (2026-10-06)

Native diagnostic57807 terminal0 on1aa44ce1, evidence bounded-affine-periodic-diagnostics-current.json. Both actual caps and exact boundary checks pass. Bounded arithmetic5.4434904016048645e-15; complete boundary0.004042590313493079 under0.01.12wall charts (faces35,43,51,59,67,75,83,91,99,107,115,123) exhaust subdivision depth despite100000 total cell allowance;95pairs unclassified,nextPair[43,49]. Therefore greater global work alone is not a fix. Material gate correctly withholds geometry.

Tried common dual-row normalization candidate; it still failed face35 and was removed. Retained diagnostic regression uses exact actual transformed face35 coefficients. Existing candidate search still explicitly unproved. A projection obtained from original chart through inverse shear, [[0,17,-1],[-15,-14,16]], freshly proves the complete transformed face in native intervals. Expanded allowed fixed integer projection range from16to64 (bounded i16 cross products remain safe); no predicate weakened. Focused58227 and full surface_linear_monotonicity module tests pass. This range/source hint improvement is source-only, not in installed WASM. Next implementation: transport private original chart proposals into the actual native affine chart/embedding audits, share budget, rerun each full proof and pair classification; never inherit source validity. Original/full goal and combined shear Solid TODO remain open.


### Native transported-projection actual-material success (2026-10-06, runtime pending)

Added inspect_candidate_with_hint: every proposal is re-proved on the actual full surface with the shared remaining cell allowance, followed by generic fallback. Native affine replay transports private root chart projections through inverse first/following affine maps only as bounded integer proposals; reconstruction or changed face geometry can never inherit certification. Core retained charts, face injectivity, boundary embedding and volume validity accept optional proposal paths; existing public diagnostic APIs keep empty proposals. No TS geometry/proof computation. Native proposal calculation is finite/bounded; rounded proposed rows provide no proof premises.

Bridge1014 terminal0: actual original periodic automatic-cap shear fixture passes all retained charts and full solidGeometryCertifiedtrue with actual geometry present; wallCells0 still withholds geometry.44.32s. Original-only request fixture saved at crates/geometry-bridge/src/fixtures/periodic-automatic-cap-shear-request.json. New negative regression reuses positive chart proposals against a folded actual wall and still refuses. Core65183:5passed; retained-chart final84989:1passed; brep/bridge checks and scoped diffcheck passed. Publisher started (`/tmp/sweep-affine-projection-hints-publish.log`), runtime pending.

Remaining critical path: standalone Solid recomputes a generic actual-model audit without original transported proposal metadata, so successful native owned placement is not yet claimed as Rush→viewport→Solid/UI success. Must propagate replayable source/operation routing with final actual-model binding or add independently proved generic candidate coverage; cannot inherit a serialized positive certificate. Existing combined public test/TODO may require updating after publication. Full all-mode objective remains active.


### Standalone Solid original replay with final-model binding (2026-10-06, runtime pending)

Projection publisher95056 terminal0,11255033bytes. Added optional expectedResultModel to native owned affine replay: compare actual final BRep before material audit; typed mismatch errors, success explicitly emits resultModelBoundtrue. No comparison field supplies numerical or material premises. Native36205 terminal0:bounded periodic shear fixture, actual final-model equality, unrelated cuboid substitution refusal and zero-wall-budget withholding all pass,108.75s.

TS exports replayable original inputs/operation history by owned object identity and serializes sweepMiterReplay into nativeGeometry artifacts for affine owners. Standalone Solid routes those inputs to original-only native replay, supplies actual expectedResultModel, requires explicit resultModelBound and overall native Solid certification; serialized positive evidence is never admitted. Old installed packages without the binding marker refuse this new path. Generic non-affine audits remain unchanged. Public combined regression now expects actual shear+reflection Solid and rejects altered control points/operation history; outcome pending and prior TODO removal is not yet qualified. No geometric/proof algorithm added to TS. Typecheck before regression edits passed; refreshed typecheck pending, scoped diffcheck clean.

Second sequential publisher84536 live (`/tmp/sweep-final-result-binding-publish.log`) includes final-model binding; do not restart/publish concurrently. After terminal, run combined public tests before claiming Rush/viewport/Solid success. Independent expanded STEP/UI matrices still required. Full goal active.


### Bounded automatic-cap shear/reflection closes finite Rush→Metal→Solid/STEP route (2026-10-06)

Final-binding publisher84536 terminal0. Exact installed public/packed geometry SHA2564da85fce2cca074b511b363890f4fbb033c8ab0bce338e8047b0c5293819b056,11255484bytes. Public24477 terminal0:14/14 in2files,46.92s. Combined original periodic automatic caps, integer shear and reflection passes actual standalone Solid replay; altered final control pole and changed operation history reject. Reflection-only and existing reconstruction/source-binding cases pass. Old combined-success TODO is removed only after this runtime evidence.

Added original-input example miter-periodic-moving-axis-guide-affine-hollow-placed.r. Vite79804 and typecheck70267 terminal0. Headless Metal40524 terminal0:24scenario assertions across1440/600, ui-bounded-affine-replay-current/matrix.json passedtrue, matching current4da artifact/source/public/dist, actualapple/metal-3/WebGPU,130faces/1shell, geometrySHAdd02dce234367447c7f2c60adb7f5802b0997549cec63199038c26df50c36088 both widths, no pageerrors. Includes success/refusal/held-dispatch cancellation/sourcechange/restoration. No window activation. Held dispatch is not mid-kernel latency evidence.

First STEP exporter67123 failed because the new fixture requested a nativeGeometry artifact without display arguments; corrected the exporter invocation only. Retry75080 terminal0, rational17388/generator40567/OCCT57734 terminal0 sequentially. external-step-bounded-affine-replay-current/manifest.json and opencascade-sweep.json confirm71/71 with exact current public/packed provenance. Exporter independently repeats original native replay with actual final-model binding for transformed case; generic no-hint chart/volume diagnostics remain separately stored, never promoted by a flag. External verifier predicates unchanged. New automatic-affine case valid130faces/264manifold opposite-use edges/1closed oriented shell andsolid/2holedcaps; exact coplanar cap control nets, full-domain wall512pcurves/cap16coedges and edge ownership preserved. OCCT volume22.92781437975148 vs independent polynomial boundary22.92781437975149, relative3.0990426038498033e-16,512material-side probes agree. Finite listed fixture evidence, not an all-mode theorem.

Full original goal remains active: spatial/discrete frame and applicable smoothness/closed seams, broader all-mode continuous/global/nesting/orientation coverage, general operations and fresh broad regressions remain unfinished. Successful native hint/replay route does not solve generic standalone geometric audits without source history.


### Periodic moving-frame profile G1 preservation in bounded native correction (2026-10-06, runtime pending)

Actual4da runtime smoothness diagnostic `/tmp/sweep-placed-smoothness.json` showed original root and placed periodic case profile G1/G2 unproved, explicit constant-projective-jet-relation-different rather than a simple work exhaustion. Solid/boundary success did not imply smoothness. Added private periodic_profile_lattice to native automatic-cap repair: only uniform-weight periodic section curves are quantized before Bezier extraction, preserving common polynomial profile relations. Interval maximum pole displacement is composed outward with prior and cap corrections into complete wall/cap error; shared repair allowance includes detection, quantization and cap projection work. Rational nonuniform weights skip this sufficient repair. Native actual retained profile jet audit supplies every positive G1 claim; input periodicity or lattice membership alone supplies none.

Native16764 terminal0:4owned-root tests,31.41s, including actual periodic moving-frame profile G1, complete bound/filledcaps/retainedcharts, original immutability, exhausted work and zero displacement refusal, nonuniform rational skip. Bridge99030 terminal0:original periodic shear material admission, actual final-model binding/substitution refusal and zero-wall work still pass,79.74s. Source-only positive; installed4da unchanged before publisher. Strengthened public combined case to require profileG1Certified across original and placed native reports; not yet executed on new WASM. Selected rustfmt and scoped diffcheck clean.

Publisher started `/tmp/sweep-periodic-profile-g1-publish.log`, pending. New correction changes actual finite fixture geometry inside its charged bound, so earlier14public/71STEP/24UI qualification is historical4da evidence and cannot be silently promoted. Full objective remains active: applicable station/moving-frame/closed-seam G2 and broader all-mode continuous/global/regularity guarantees require separate work.


## Periodic profile G1 correction qualified on current runtime (2026-10-06)

Public WASM SHA-256: `934863648f08af1097a616193e715d1c20dd93e88ca54949973b00819a74d660`, 11,259,764 bytes. Rust quantizes uniform-weight periodic profile poles on a bounded lattice before automatic nonplanar cap correction, charges outward displacement and work, rebuilds the actual body, and reruns actual retained seam and material predicates. Original requests remain immutable. This correction currently applies only to the automatic nonplanar-cap branch; explicit-cap, closed-no-cap and nonuniform rational families are not covered by this change.

Focused native tests passed: four owned-root tests and one bridge bounded-periodic-shear test. Current public tests passed 29/29 in three files. Actual original, sheared and reflected bodies certify profile G1; profile G2 remains false and station continuity remains C0. Diagnostic evidence: `periodic-profile-g1-smoothness-current.json`. No moving-frame or universal smoothness theorem is inferred.

Headless actual Apple Metal UI passed 24 assertions at 1440/600 widths (`ui-periodic-profile-g1-current`), with 130 faces and one shell. No foreground activation was used. Held-dispatch cancellation remains lifecycle evidence only.

Independent rational-boundary and generator references, followed by OpenCascade, passed 71/71 STEP fixtures on the current artifact (`external-step-periodic-profile-g1-current`). The automatic affine case has 130 faces, 264 manifold opposite-use edges, one closed oriented solid/shell and two holed caps; full-domain retained wall/cap-coedge checks and native/external material agreement pass. Volume 22.927814379750586 versus independent 22.92781437975059 (relative error 1.5495213019249625e-16). These finite results do not establish all-mode continuous/global/regularity/nesting/orientation guarantees. Broader current-artifact regression remains outstanding.


## Coupled periodic profile lattice: native G2 proof (runtime pending, 2026-10-06)

The automatic nonplanar-cap branch now proposes coupled lattice center/axes for uniform-weight, uniform-knot quadratic six-pole periodic profiles. Every proposed pole is compared with its original source using outward interval displacement. The proposal is used only within the caller correction tolerance and work budget; asymmetric families retain independent lattice rounding. No G2 certificate is inferred from symmetry. Actual retained seam jets independently prove G2 after endpoint projection and after integer shear. The displacement remains included in the complete wall/cap error and actual chart/material predicates remain mandatory.

Current native owned-root suite: 4/4 passed, 23.67 s (`/tmp/sweep-symmetric-profile-g2-native.log`). It asserts actual profile G2, complete boundary within tolerance, bounded positive displacement, original immutability, refusal under exhausted work and preservation of asymmetric/nonuniform rational behavior. Bridge bounded-periodic-shear: 1/1 passed, 74.19 s (`/tmp/sweep-symmetric-profile-g2-bridge.log`), including actual transformed profile G2, station C0, material validity, final model binding and tampering refusal. Typecheck and scoped diff check pass.

WASM publisher is running (`/tmp/sweep-symmetric-profile-g2-publish.log`); current public/UI/STEP artifact 93486364 remains the preceding G1 checkpoint. Updated public regression assertions await the newly published package. No runtime G2 or new STEP/UI qualification is claimed yet. Explicit caps, closed-no-cap and other profile families remain uncovered by this correction. Full all-mode requirements remain incomplete.


## Coupled profile G2 package installed; broader regression snapshot (2026-10-06)

Publisher completed successfully. Public/generated WASM hashes agree: `28aec982784c76ef0cdb6120e83c39949a588753fd66de1be58cec8cf368b205`, 11,262,391 bytes. Release compilation 1m16s; wasm-opt 12,674,133→11,262,391 bytes. Current 29-case public regression and Vite build are running; runtime G2/UI/STEP qualification is still pending. A prematurely started public run against the previous package was terminated and is not evidence.

Broader BRep library snapshot: 749 passed, 2 failed, 2 ignored, 182.47s (`/tmp/sweep-symmetric-profile-g2-brep-regression.log`). Failures were AAG fillet fixture selecting a nonvertical edge and mutated face area expectation. The current shared files had changed by inspection; a fresh complete AAG subset passes18/18 (`/tmp/sweep-symmetric-profile-g2-aag-recheck.log`). No AAG source was edited by this sweep task. These two runs are reported separately; no fresh all-library green result is inferred. All sweep-owned root and shell-orientation checks in the broader snapshot passed.


## Coupled periodic profile G2 qualified through Rush and viewport (2026-10-06)

Current WASM SHA-256 `28aec982784c76ef0cdb6120e83c39949a588753fd66de1be58cec8cf368b205` (11,262,391 bytes) supersedes the previous G1-only checkpoint for the automatic nonplanar-cap periodic family. Coupled native lattice center/axes preserve the required quadratic profile relations, with outward displacement charged against the original controls. Actual post-cap and affine surfaces independently pass exact regular projective strip-jet predicates; symmetry is not a certificate premise. Uniform weights/knots, applicable quadratic periodic profiles and automatic cap correction remain the supported scope.

Public 29/29 tests in three files passed on this artifact, including root/shear/reflection G2, standalone Solid, model/history tampering refusal and immutable authoring graphs. Native root4/4 and bridge1/1 passed; typecheck and Vite pass. Actual saved reports (`periodic-profile-g2-smoothness-current.json`) certify 128 profile seams, with total work1,020,424 root and1,023,480 each placement. Complete error upper bounds0.0018079013505209848,0.004042590316384045 and0.007001971822163115 mm remain within0.01mm. Station/cap continuity is explicitly C0.

Headless actual Apple Metal/WebGPU matrix (`ui-periodic-profile-g2-current`) passed24assertions at1440/600, with no page errors. Both widths display profile G1/G2 positive,128seams, station/caps C0 and complete error within budget. Held dispatch cancellation is lifecycle evidence, not kernel interruption latency. New independent STEP qualification is running; earlier71-case STEP success belongs to the previous artifact. Full all-mode geometry, applicable station/frame smoothness and other cap/profile families remain incomplete.


## Independent STEP qualification of coupled periodic profile G2 (2026-10-06)

Current artifact `28aec982784c76ef0cdb6120e83c39949a588753fd66de1be58cec8cf368b205`,11,262,391 bytes, completed all independent reference phases and OpenCascade verification:71/71 (`external-step-periodic-profile-g2-current`). Public/packed artifact provenance agrees. Both automatic-cap and automatic-affine-cap manifest reports certify actual retained profile G2; these are native seam proofs, not inferred from successful STEP import.

Automatic affine case:130faces,264manifold opposite-use edges,1closed oriented shell/solid,2holed caps; native/external material agreement and full-domain wall/cap-coedge preservation pass. OpenCascade volume22.927814379753826 equals the independent generator reference at represented binary64 output (relative error0.0). This fixture agreement does not establish general volume or embedding theorems.

This checkpoint completes qualification of the new automatic periodic G2 correction through native→WASM→Rush→actual Metal viewport→standalone Solid→independent STEP. Public29/29 and UI24/24 are on the same hash. The complete library goal remains active: explicit-cap/closed-no-cap/other profile correction coverage; applicable station/frame/multispan/closed-seam smoothness; all-mode continuous/global/regularity/nesting/orientation; broader operation ownership and cancellation remain separate obligations. Caps and the current moving-axis station joints remain explicitly C0. Historical finite matrices do not prove universal completion.


## Explicit authored-cap periodic G2: native phase (runtime pending, 2026-10-06)

Original-request construction now shares an explicit cap correction's caller work/tolerance allowance with periodic profile preservation. For applicable uniform-weight periodic families without separate circle repair, bounded profile correction precedes cap projection; its outward displacement and work are added to the native report. Combined displacement must stay within the original explicit cap tolerance and the projection receives only remaining work. Closed-cap refusal, zero/nonfinite work behavior and circle correction order remain separate. Original source sections are immutable; no positive seam proof is inherited from the proposal.

Native owned-root4/4 passed in42.17s (`/tmp/sweep-explicit-profile-g2-native.log`), including actual authored-cap G2, complete bound within0.01mm, combined correction within1e-9mm and insufficient work refusal. Bridge explicit-periodic-cap shear1/1 passed in27.87s (`/tmp/sweep-explicit-profile-g2-bridge.log`): actual post-shear profile G2 and material admission, station C0. Typecheck and scoped diff check pass. Temporary bridge compilation failures in concurrently changing structural/curvature consumers resolved in the shared source; this sweep task did not edit those consumers.

TS transport routing now includes explicit cap requests in original-only native affine replay. Public root/shear/reflection test is parameterized for automatic and authored caps. New example `miter-periodic-moving-axis-guide-affine-hollow-authored-caps-placed.r` is registered in headless UI and the independent STEP exporter as `rush-periodic-moving-axis-authored-affine-caps.step` (72planned cases). Geometry/correction/admission remain Rust-owned.

WASM publisher is running (`/tmp/sweep-explicit-profile-g2-publish.log`). Current qualified artifact28aec982 and its29public/24UI/71STEP results belong to the preceding automatic-cap G2 phase; explicit-cap runtime/72-case qualification is pending. Other profiles, closed-no-cap coverage, applicable station/frame smoothness and full all-mode continuous/global guarantees remain incomplete.


## Closed periodic profile baseline and explicit-cap publication wait (2026-10-06)

Current old artifact28aec982 is the qualified automatic-cap G2 package; explicit-cap WASM publisher remains live in native optimization (`/tmp/sweep-explicit-profile-g2-publish.log`). Frontend dependencies disappeared from this shared checkout during unrelated changes. Locked `npm ci --ignore-scripts` restored them successfully without modifying the manifests/lockfile; fresh typecheck passed before disappearance. Explicit-cap runtime/72-case oracle qualification remains pending.

A readonly closed-path probe substitutes uniform-weight quadratic periodic outer/hole profiles in the existing closed authored-axis/guide/affine example. Actual native reports certify continuousBound, but profileG1Certified=false and exact profileG2=false, with stationC0. Complete original source, artifact hash and reports are saved in `closed-periodic-profile-baseline-current.json`. This changes the next correction target: closed-no-cap periodic lattice preservation with charged displacement and fresh retained seam/material audits, rather than inferring closed G1/G2 from periodic input flags. No source implementation change for this closed path is claimed yet.


## Explicit authored-cap G2 runtime and UI checkpoint (2026-10-06)

Publisher completed: public/generated WASM SHA-256 `991f79fc05d15333218c551dbe9892f89fd8c72aaa5f39a43c456fa44466bfe4`,11,272,037 bytes. Release compilation2m23s; wasm-opt12,684,088→11,272,037 bytes. Vite and typecheck pass. Missing isolated Playwright runtime was restored through its pinned qualification-package lockfile; package admission and actual Metal launch succeeded.

Public assertions: initial30-case run passed27 and timed out3 at the harness's30s limit (no geometric assertion failures). Only those3 were rerun with CLI testTimeout120s, preserving every assertion, and passed3/3 (`/tmp/sweep-explicit-profile-g2-timeout-recheck.log`,127.41s total). This is reported as two runs, not a fresh single all-green run. It covers original/explicit caps, source immutability, root/shear/reflection G2, actual final native replay binding and model/history tampering refusal. Independent original authored-cap Solid probe also passed (`/tmp/sweep-explicit-root-solid-probe.log`).

Headless actual Apple Metal/WebGPU UI passed24assertions at1440/600 (`ui-explicit-profile-g2-current`), null page errors, profile G1/G2 positive for128seams and station/capC0. Actual reports in `explicit-profile-g2-smoothness-current.json` show root/shear/reflection G2 and complete errors0.0018079013505209848,0.004042590316384045,0.007001971822163115mm, all within0.01mm. Held dispatch cancellation remains lifecycle evidence only.

Independent72-case STEP export is still live (`/tmp/sweep-explicit-profile-g2-step-export.log`); reference/OCCT stages have not yet qualified this artifact. No72-case STEP success is claimed. Closed-no-cap periodic profile preservation remains the next demonstrated native gap; other all-mode continuous/global/station-frame smoothness obligations remain active.


## Explicit authored-cap G2 independent STEP qualification completed (2026-10-06)

Current WASM991f79fc05d15333218c551dbe9892f89fd8c72aaa5f39a43c456fa44466bfe4 (11,272,037 bytes) completed independent rational-boundary and generator references followed by OpenCascade72/72 (`external-step-explicit-profile-g2-current`). The artifact provenance matches public/packed bytes. New `rush-periodic-moving-axis-authored-affine-caps.step` has130faces,264manifold opposite-use edges,oneclosed oriented shell/solid,twoholed caps, positive native/external material agreement and full-domain wall/cap-coedge preservation. OCCT and independent generator volumes both22.927814379753826 (represented relative error0.0). Its native manifest separately certifies profile G2; successful import alone is not a seam proof.

Explicit periodic authored caps are now qualified through native→WASM→Rush→actual Metal viewport→standalone Solid→independent STEP for this applicable family. UI24/24 and public27initial+3timeout rechecks are on the same artifact; their distinct run scopes are preserved above. Finite72-case oracle evidence is not an all-mode guarantee. Nonuniform rational/other miter profile G2 remains independently unproved where reports say so.

Next demonstrated gap: `closed-periodic-profile-baseline-current.json` already has actual retained charts/material positive and continuousBound=true, but actual profile G1/G2 false. Closed-no-cap bounded profile preservation must charge original-to-retained displacement, then rerun actual seams, charts and material. Station/cap continuity in the current moving-axis family remains C0; full all-mode continuous/global/regularity/nesting/orientation and applicable station/frame/multispan smoothness obligations remain active.


## Closed no-cap periodic profile G2: native correction and global admission (runtime pending, 2026-10-06)

Closed original-request construction now applies bounded periodic lattice preservation to applicable uniform-weight profiles without circle/cap correction. Original controls remain immutable. Outward original-to-retained displacement is included in complete wall error; work uses the native projection exact-work allowance. Actual retained G2 and wall charts are independently audited. Nonuniform rational families retain their previous path. No artificial caps are added.

Signed binary axis permutations now carry exact Euclidean operator norm1 instead of conservative sqrt3. This preserves the source error budget under reflection; arbitrary integer matrices retain their outward Frobenius bound. Native affine tests3/3 passed, including off-grid exact reflection and work refusal.

The first closed shear/reflection material run correctly refused after exhausting the default100,000 contact cells (next pair58,62), despite positive full boundary error and wall charts. Closed sweep volume admission now uses the existing exact positive rational control-hull separation path, previously limited to cap sweeps, with every supporting plane proved on actual coefficients. Plain generic face-contact work accounting remains unchanged; no budget was raised. The closed body now passes fresh material/orientation and actual final-model binding after shear/reflection, with128faces/two shells and bound8.672191698922221mm within10mm.

Native evidence: closed original-rootG2 test1/1; all owned-root5/5 (26.06s); affine3/3; generic contact3/3; embedding4/4; volume validity7/7 (80.13s); bridge closed shear/reflection1/1 (33.74s). Typecheck and scoped diff check pass. Actual station joints retain C0.

New examples `closed-periodic-frame-guide-affine-hollow.r` and `closed-periodic-frame-guide-affine-hollow-placed.r` extend public original/shear/reflection immutability/tampering assertions, headless UI selection and independent polynomial boundary volume references (`rush-closed-periodic-profile-root.step`, `rush-closed-periodic-profile-placed.step`;74planned fixtures). TS changes only transport the new native correction reason and route original-only affine replay. WASM publisher is live (`/tmp/sweep-closed-profile-g2-publish.log`); current qualified991f79fc artifact still belongs to the preceding explicit-cap phase. New runtime/UI/STEP results are pending. All-mode continuous/global/regularity/nesting/orientation and applicable frame/station/multispan smoothness remain separate full-goal obligations.


## Closed periodic profile: packaged WASM and Metal UI checkpoint (2026-10-06)

Publisher completed successfully. Public and generated WASM plus identity agree on SHA-256736792db5071ba08af272e024e746f95dfea9a74c7fd0648f1ae748ffde78b45,11,272,611bytes. Fresh Vite build passed. Headless Metal closed-periodic-frame-guide-affine-hollow-placed UI passed24assertions across1440/600widths with no page errors; matrix artifact provenance verifies source/public/dist bytes (`ui-closed-profile-g2-current/matrix.json`). All checks ran without foreground navigation or focus changes. Held-dispatch cancellation is lifecycle evidence only. Public31-case checks and74-case independent STEP qualification remain running and are not yet reported as passed. Full-goal all-mode and smoothness limitations remain active.

Current packaged-WASM public qualification completed:31/31tests across sweepAffineLattice, miterOwnedBoundary and sweepRetainedDecomposition passed in225.94s with120s per-test allowance (`/tmp/sweep-closed-profile-g2-public.log`). This is one complete run on736792db artifact. STEP export remains active; no independent74/74 claim is made.


## Closed periodic profile independent STEP complete; periodic station reconstruction native phase (2026-10-06)

Artifact736792db5071ba08af272e024e746f95dfea9a74c7fd0648f1ae748ffde78b45 (11,272,611bytes) passed one complete31/31public test run,24/24headless Metal UI assertions and74/74independent STEP cases. Rational and polynomial generator volume references completed before OpenCascade. Both new closed root/placed bodies have128faces,256manifold opposite-use edges,one solid/two closed oriented shells and512full-domain wall pcurve uses. Shell volumes have opposite signs, with native/external material agreement. Independent volume6.117932035577874mm³; OCCT6.117932035577876/6.117932035577880, relative errors4.36e-16/1.02e-15. Evidence: external-step-closed-profile-g2-current and ui-closed-profile-g2-current. Sampled material probes do not prove universal containment. This is finite qualification of the original closed family, not all-mode completion.

Next native change extends station reconstruction to canonical six-pole periodic quadratic unit-weight profiles. An exact half-grid Bezier representation is proposed and compared against every actual native source-decomposition coefficient; source grid/range, explicit repeated controls and conversion work are checked. Conversion and reconstruction share one caller budget. Quintic station candidates still charge outward displacement, preserve original caps and rerun actual seam audits. No profile/regularity/Solid theorem is inferred from representation conversion.

Native reconstruction3/3 passed after the half-grid improvement (0.75s): actual open profile+station G2, actual closed station/wrap G2, unchanged source/caps, half-grid profiles, work and tolerance refusal, sharp C0 and valid nonuniform rational refusal. Canonical station regression2/2 passed before the half-grid-only refinement. Typecheck passed. New public owned curved-center periodic reconstruction regression and Rush example are prepared. WASM publisher is active in /tmp/sweep-periodic-station-g2-publish.log; current74STEP/31public/24UI evidence belongs to preceding736792db package, not the new station implementation. Concurrent unrelated work replaced shared generated artifacts after qualification; new package identity must be verified again before public tests. Full-goal all-mode and moving-frame smoothness obligations remain open.

MCP-RAG ingest of the previous UI checkpoint timed out at the client after300s, but a subsequent authoritative get_document confirmed that checkpoint was committed. No duplicate restart was used to resolve that timeout; this latest update still requires ingest.


## Periodic station reconstruction: exact diagonal contact admission (2026-10-06, runtime pending)

Native original-only curved-center reconstruction initially had complete bound0.9000000000000311mm within2mm and all eight wall charts, exact boundary, valid trims and both cap certificates, but eight adjacent profile-wall pairs remained unresolved. Original axis-only synchronized side comparisons could not classify diagonally opposed profile strips. No proof budget was raised. Shared-boundary separation now additionally tests two fixed integer diagonal normals using cad-predicates direction_dot3d on original retained controls, with bounded intrinsic work. Identical strictly monotone V-coordinate and positive weights constant along V force any contact to equal V; exact strict opposite signed offsets then restrict it to the actual shared edge. Rounded offsets or normals are not premises.

Native diagonal tests2/2 pass, including swap symmetry, crossed strips, loss of synchronization and varying-weight refusal. Plain generic contact tests3/3 preserve frozen work accounting. Fresh original-only bridge test1/1 passes (0.43s), certifying actual model G2 along profile/stations, complete bound, fresh charts/material/Solid, immutable original input, zero-work withholding and injected-model refusal. Previous failed native/public targets on the first station-only b9bebb45 package are retained as before-fix evidence, not counted as passing.

New Rush periodic reconstruction example is registered for headless UI; independent STEP case requires complete bound and actual profile/stationG2 before export, giving75planned cases. Public target awaits the new diagonal package. First station-only publication completed b9bebb4517848cf7b0bc24b1254f305fa70223ad12918e32aa3a6a94d32e4dbc (11,294,872bytes); diagonal publisher is active /tmp/sweep-periodic-station-g2-diagonal-publish.log. Earlier74STEP/31public/24UI qualification remains736792db evidence. No full all-mode guarantee or new75-case success is claimed.


## Periodic station diagonal package installed (runtime qualification running, 2026-10-06)

Publisher completed successfully; public/generated bytes and identity agree:6fd74d28ee8c1dbabca7c4a43f0a065465b283f5baa584aba87701f999a417bb,11,298,514bytes. Release1m24s; wasm-opt12,713,766→11,298,514. Vite passed. Complete shared-boundary subset9/9, plain contact3/3, native reconstruction3/3 and original-only bridge/Solid1/1 pass. Four-file public suite, headless Metal new periodic reconstruction UI and75-case STEP export are now running. No new public/UI/75STEP success is claimed until their own terminals/results are inspected. Earlier74-case successful oracle remains736792db evidence.


## Periodic station reconstruction runtime and UI checkpoint (2026-10-06)

Current6fd74d28ee8c1dbabca7c4a43f0a065465b283f5baa584aba87701f999a417bb package passed headless Apple Metal UI24/24 at1440/600, with no page errors and source/public/dist provenance verified (ui-periodic-station-g2-current/matrix.json). The new periodic curved-center reconstruction succeeds through actual viewport and standalone Solid. Source replacement/refusal/cancellation/restoration scenarios are finite UI evidence; held dispatch does not prove kernel interruption latency.

Initial four-file public run passed24 and failed one newly written test because it read a nonexistent profile.g2.certified field. A short attempted correction to profile.certified was also wrong. The actual public schema is profile.exactG1G2Certified plus certifiedOrder. After correcting the test and the STEP exporter to require true/order2, the full changed miterSmoothStationWalls file passed6/6 (5.71s); the other three unchanged files passed19/19 in the original run. Report these distinct runs rather than one fresh four-file25/25 run. They qualify actual profile/stationG2, complete within-budget boundary, Solid, unchanged source/caps, work refusal, STEP roundtrip smoothness/material and existing affine replay/tampering cases.

The first75-case export failed at the same nonexistent test/export report property; no geometry/refusal premise was weakened. Corrected export is now live (/tmp/sweep-periodic-station-g2-step-export-final.log). Reference/OCCT75-case qualification remains pending. No75/75 claim is made. Full goal remains active; current generic/nonuniform/other frame and all-mode obligations are unchanged.


## General rational station reconstruction and exact contour chart (2026-10-06)

Independent periodic station STEP qualification completed:75/75 exported cases passed independent rational/generator reference and OCCT checks on package6fd74d28ee8c1dbabca7c4a43f0a065465b283f5baa584aba87701f999a417bb (11,298,514bytes). Evidence: external-step-periodic-station-g2-current/manifest.json and /tmp/sweep-periodic-station-g2-step-occt.log. This is finite fixture evidence, not all-mode certification.

New generic Rust reconstruction remains unpublished. It supports compatible multiple profile segments, holes, and nonuniform rational weights; actual shared quintic station jets are checked with exact predicates under the caller's shared work allowance. Native reconstruction5/5 passed. Original-only hollow polygon and nonuniform periodic rational bridge targets both now pass full within-budget continuousBound and Solid. Fresh general_ bridge subset3/3 passed, including one separate multispan boolean regression.

The rational target initially refused source-bound-unproved before reconstruction. Diagnostics isolated ideal cap-domain contour-decomposition-unproved: retained wall bound3.376309819460577e-14 and filled cap decomposition1.9825487283453366e-14 existed, with exact parallel planes and positive projection. Nonuniform homogeneous Bernstein coefficients were exact but Euclidean poles included1/3, so direct binary64 exact decomposition refused. Rust contour audit now tries a bounded set of common positive integer proof charts for all loops. Every original-coordinate multiplication must be exactly representable; existing exact decomposition must certify every coefficient. Positive invertible scaling preserves simplicity/containment only, never supplies model geometry or a displacement bound. Candidate-chart attempts consume the contour cell allowance. Original curves/model are unchanged. No diagnostic environment hooks remain.

Remaining: contour negative/budget regressions, public WASM/Rush/viewport/STEP qualification of this unpublished generic implementation, and all remaining full-goal mode/global/smoothness/cancellation obligations. Do not attribute new native changes to the previously qualified6fd WASM.


### Exact chart regression completion (2026-10-06)

Fresh contour audit subset5/5 passed, including nonuniform periodic outer/hole ownership, original immutability, open contour refusal, exact multiplication refusal and zero pair/cell budgets. Existing exact curve segmentation subset3/3 also passed. General bridge subset3/3 includes both intended original-only Solid targets. git diff --check passed. Logs: /tmp/sweep-rational-contour-exact-chart-tests-final.log, /tmp/sweep-exact-curve-chart-regression.log, /tmp/sweep-general-bridge-exact-chart-final.log. Generic Rust implementation remains unpublished; its WASM/Rush/viewport/independent STEP qualification is the next required stage.


## Generic reconstruction WASM/Rush/viewport checkpoint (2026-10-06)

New Rust source published successfully as0f18eb7da6684488ade3faf4536095f85d53c6683d742e551b6835ba90f19c02,11,316,630bytes. Rust release1m23s; wasm-opt12,735,750→11,316,630. Fresh typecheck and Vite builds passed. New public tests cover native-owned hollow polygon and nonuniform periodic rational reconstruction, complete within-budget bound, actual stationG2, original/cap immutability, work refusal and STEP roundtrip material/station smoothness. First run8/9 passed; the additional Rush routing assertion exposed missing original-request replay for unplaced reconstructions.

TS routing now sends an identity placement with original source plus native reconstruction arguments when a reconstruction has no affine history. It keeps the authored reconstruction work allowance; identity is audited by Rust and the transferred actual model must match native original-source replay. Solid provenance traversal now follows brep_smooth_miter_stations as well as transforms. No geometric algorithm/certificate/admission moved into TS. Native positive replay passed; tampered retained control was correctly refused with Owned affine final model differs from original replay. An intermediate9-case rerun failed only because its expected error regex omitted that precise native message; corrected regression is running, so no final9/9 claim yet.

New Rush example progressive-miter-reconstructed-rational-profile.r is registered for the UI matrix. Headless Apple Metal UI passed24/24 at1440/600 (12checks each), no focus switches. ui-general-rational-current/matrix.json passed=true and binds source/public/dist to0f18eb7. Held-dispatch cancel remains lifecycle evidence rather than mid-kernel latency.

Full four-file public regression is live: /tmp/sweep-general-rational-public-regression.log, session81628. STEP exporter adds independent rational-boundary volume case rush-reconstructed-rational-profile.step; intended76cases export live: /tmp/sweep-general-rational-step-export.log, session49410. Independent references/OCCT for this new package remain pending. Previous75/75 remains6fd-only evidence. Full all-mode goal is active.


### Public regression complete on general rational package (2026-10-06)

Fresh serial four-file regression28/28 passed in98.49s: miterSmoothStationWalls, miterStationReconstruction, miterStationSmoothness, sweepAffineLattice. Log /tmp/sweep-general-rational-public-regression.log, terminal0. Includes the corrected exact native tampering refusal, both generic Solid targets and Rush/viewport station evidence. Headless UI24/24 remains0f18eb7-bound. git diff --check passed. STEP76 exporter session49410 remains confirmed live, with emitted STEP files and logged unsegmented cases; wait for its terminal before running sequential rational-reference, generator-reference and OCCT scripts. Full objective remains active, with all-mode/global and remaining smoothness/cancellation obligations unchanged.


## Independent general rational STEP matrix completed (2026-10-06)

Export terminal0 with76cases. Sequential independent rational boundary reference and canonical generator reference terminal0. Fresh independent OCCT76/76 passed; manifest and opencascade-sweep.json both bind0f18eb7da6684488ade3faf4536095f85d53c6683d742e551b6835ba90f19c02. Artifacts: external-step-general-rational-current. Logs: /tmp/sweep-general-rational-step-export.log, /tmp/sweep-general-rational-step-reference-final.log, /tmp/sweep-general-rational-step-generator.log, /tmp/sweep-general-rational-step-occt-final.log.

New rush-reconstructed-rational-profile.step is valid,10faces/20manifold opposite-use edges,1solid/1closed oriented shell,2planar caps. Whole-domain wall/edge/cap coedge preservation and native-external material agreement pass. Independent16/32/64quadrature values16.19874128761843/16.198741287618436/16.19874128761844mm3; OCCT16.198741287618432, relativeerror4.386407086476564e-16. This numerical convergence is independent validation, not a new interval/global certificate. Native stationG2 passes; native profile certifiedOrder remainsNone and is still a full-goal smoothness obligation.

Two oracle/fixture defects were repaired with no geometry/tolerance weakening. First reference lacked outwardCapContours for new fixture; exporter now supplies actual shell-oriented cap coedges. Current manifest was supplemented by replaying the single Rush case and verifying exact regenerated STEP SHA256 matches exported bytes, so no re-export or model change occurred. Initial Python supplement failed because manifest deliberately omits full model; it wrote no manifest. Second initial OCCT75/76 failed only full-domain edge coefficient gate because constant positive rational weights were normalized by OCCT. Independent helper now proves exact common positive projective weight scale with Fraction cross-products, rejecting rounded ratios/nonpositive/missing/mismatched data. Knot, pole, parametrization, topology and tolerance gates remain mandatory. Helper unit tests2/2 pass, including one-ULP unequal ratio refusal. Final76/76 replaces initial75/76 for this exact package; earlier75/75 remains6fd historical evidence.

Fresh public regression28/28 and headless Apple Metal wide/narrowUI24/24 qualify the same package. Typecheck/Vite/diff checks passed. Full objective is not complete: all-mode continuous/global/regularity/nesting/orientation, remaining rational/moving-frame/multispan/closed seam smoothness, wider combinations and true mid-kernel cancellation evidence remain separate requirements.


### Rational profile G1 localization (2026-10-06)

Added native regression rational_profile_smoothness_requires_actual_retained_jets. It compares two distinct diagnostic inputs with identical periodic degree2 basis and nonuniform weights: unit radius decomposition has non-dyadic2/3,1/3 poles and actual retained profileG1 is unproved; radius3 has exactly representable Bernstein poles and actual retained profileG1 passes. This second input is diagnostic only, not a replacement acceptance target or a repair of the unit profile. Original unit profile requirement remains unchanged. Fresh sweep_station_reconstruction native subset6/6 passes in4.13s, /tmp/sweep-rational-profile-jets-regression.log; diff check passed. Existing0f18eb7 runtime76STEP/28public/24UI evidence remains unchanged because only native test/source diagnostics were added.

Next obligation is actual unit-profile smoothness: bounded native pole correction with full displacement/cap certificate propagation, or preserving original shared global NURBS basis instead of rounded Euclidean Bezier poles. Native seam audit must still prove retained geometry; original mathematical C1 cannot be substituted for actual retained G1. No relaxed tolerance, silent profile resizing, or positive profileG1 certificate was introduced. Full goal remains active.


## Bounded native rational-profile G1 correction (2026-10-06, WASM pending)

Implemented rational_periodic_lattice for nonuniform positive degree2 periodic profiles, finite control nets and bounded caller work. It tries finite integer lattice strides1/3/5/7/15/21/35/105 on varying coordinates, preserving constant coordinates, weights and knots. Native exact original-to-Bezier extraction must accept every candidate coefficient; rounded homogeneous operations/divisions withhold that candidate. New nurbs-core exact_bezier_controls facade validates/limits copied control rows before invoking existing exact dyadic extraction. This is a numerical proposal, not a positive smoothness certificate.

For open original-only miter bodies without explicit/circle correction, accepted proposals are tagged bounded-rational-periodic-profile-interpolation. Outward same-basis positive-weight control displacement is charged to existing complete wall/cap boundary union and original max_deviation; region, projection, chart and Solid gates remain native and mandatory. Corrected actual retained model independently proves profileG1, stationG2 and Solid in original unit-radius nonuniform rational bridge target; no target resize or relaxed seam predicate. Closed/moving/explicit-cap nonuniform families remain separate obligations.

New correction test passes: immutable original, identical knots/weights, preserved constant Z, exact decomposition12row success/11row refusal, positive displacement<1e-9, zero tolerance refusal, zero/one-short work refusal and exact-work-limit success, actual retained G1. Full owned subset6/6 passed23.47s; fresh general bridge3/3 passed4.08s (both general profiles plus multispan boolean). Logs /tmp/sweep-rational-profile-lattice-budget-final.log, /tmp/sweep-rational-profile-lattice-owned-regression.log, /tmp/sweep-rational-profile-lattice-general-bridge.log. An initial new-test compile failed on shorthand .5 Rust literals, fixed to0.5 before passing.

TS adds only correction-reason routing for constructor-owned affine transport and new public profileG1 assertion. All geometry/correction/proof stays Rust. Publisher live /tmp/sweep-rational-profile-g1-publish.log; wait terminal before public/UI/STEP. Previous0f18eb7 package76STEP/28public/24UI qualifies pre-repair package only. This new G1 behavior is native-only until publication and artifact-specific verification. Full goal remains active.


## Rational profile G1 runtime qualification (2026-10-06, STEP pending)

Publisher terminal0 installed8a746ea687fd6103bae96f6847f8419e849e546fe9e289c60dfa2924a0bb62d2,11,319,915bytes. Release1m19s, wasm-opt12,739,412→11,319,915. This package includes native bounded rational periodic correction and exact coefficient extraction. It supersedes0f18eb7 for current runtime behavior; older76STEP/28public/24UI evidence is historical package-specific.

Initial typecheck identified missing bounded-rational-periodic-profile-interpolation tag in TS correction-result union. Added transport type only, no geometry change; fresh typecheck terminal0 and Vite passed. Fresh serial four-file public regression28/28 passed89.32s, /tmp/sweep-rational-profile-g1-public.log. Updated general rational public target now requires native profileG1; Rush test requires constructor reportG1 and viewportG1 plus stationG2/full bound/Solid, native original-source replay and tampered final-model refusal. The polygon retains profileC0. Full corrected-source caps/original immutability and independent STEP roundtrip station/material tests pass.

Enhanced headless Apple Metal UI26/26 passes at1440/600 (13checks each), no page errors, headed=false. Explicit new rational-profile-G1-and-station-G2 assertion verifies rendered actual scoped profileG1/pathG2 in addition to baseline lifecycle/Solid/refusal/recovery. Evidence ui-rational-profile-g1-current/matrix.json passed=true; artifactProvenance sourcePublicDistVerified=true and exact8a746ea/11,319,915byte binding. No focus changes. Held-dispatch cancellation still does not qualify in-kernel interruption latency.

STEP exporter strengthened new rational case requirement to profileG1 in addition to complete bound/stationG2. Current76case export remains authoritative live session78473, PID19058, /tmp/sweep-rational-profile-g1-step-export.log; no new independent76/76 claim yet. Wait export terminal, then sequential reference-rational-boundary-volume.py, reference-sweep-generator-volume.py and verify-sweep-step-occt.py in external-step-rational-profile-g1-current. Full remaining closed/moving/explicit rational and all-mode/global/smoothness/cancellation obligations remain active.


## Rational profile G1 independent STEP completed; closed combined target added (2026-10-06)

Current8a746ea687fd6103bae96f6847f8419e849e546fe9e289c60dfa2924a0bb62d2 package76/76 independent STEP cases passed. Export/session78473 terminal0, then rational-reference/session6573 terminal0, generator-reference/session49498 terminal0 and OCCT/session17522 terminal0. Manifest/opencascade-sweep.json bind11,319,915bytes/public-packed identity. Artifacts external-step-rational-profile-g1-current; logs /tmp/sweep-rational-profile-g1-step-{export,reference,generator,occt}.log. New rational case independently records native profileG1=true, stationG2=true, Solid=true; OCCT valid10faces/20manifoldopposite-useedges/1closedorientedshell/1solid, full-domain wall/edge/cap preservation and material agreement. Quadrature16.198741287588966/16.19874128758897/16.198741287588973mm3; OCCT16.198741287588966, relative4.3864070864845426e-16. Correction changes volume within its separately certified displacement, not by silently retaining old expected volume. Same packagepublic28/28 andUI26/26 previously passed.

Added next native target closed_nonuniform_periodic_profile_keeps_full_bound_g1_and_material_after_placement from existing closed hollow authored-frame/guide/axisScale source and shear/reflection replay, replacing only both periodic profile weight vectors with valid repeated[1,.5,1,1,1,.5]. Target retains original closed geometry, full bound, profileG1 and Solid/material requirements. Initial implementation from_str transport compile correction fixed an incorrect from_slice call before tests. Baseline test failed after49.80s: full within-budget continuous bound8.672191698920885<=10, but affine-solid-geometry-unproved. Extended the same native rational lattice correction to closed no-cap bodies (no positive flags inherited); retest failed50.65s at same Solid gate with charged bound8.672191698925102<=10. This does not complete closed mode or prove geometry invalid. Keep desired test, do not weaken acceptance.

Closed extension is unpublished native source only;8a746ea does not contain it. Focused enriched stage diagnostic/session90626 live /tmp/sweep-closed-rational-profile-stage-diagnosis.log will distinguish charts, injectivity, pair classification, nesting and orientation. Need exact global material proof before qualified closed native/WASM/Rush/UI/STEP claim. Full objective remains active.


### Closed rational combined stage diagnosis complete (2026-10-06)

Enriched native target/session90626 terminal101 confirms the precise unproved stages after charged rational correction and shear/reflection: retainedWallCharts.allChartsCertified=false; volume.boundaryEmbeddingCertified=false; allFacesInjective=false; allPairsClassified=false. Nesting=null and both shell orientation queries have0attempts/outward=null, so nesting/orientation are blocked downstream rather than independently disproved. Complete within-budget closed boundary8.672191698925102<=10 remains true. Evidence /tmp/sweep-closed-rational-profile-stage-diagnosis.log. Do not claim closed combined Solid/G1 qualified from open8a746ea package76/28/26 success. Next meaningful native work is rational retained-wall regularity/injectivity and complete contact classification for this unchanged closed authored-frame/guide/affine hollow case; preserve the positive Solid target and actual native audits. Goal is active, not blocked: no external input is required to continue proof work.


## Closed rational affine chart proof repaired (2026-10-06, native unpublished)

Added focused closed_nonuniform_transported_charts_are_reproved_on_actual_poles, preserving original closed hollow frame/guide/axisScale source and shear/reflection. Before fix actual chart audit consumed100000shared cells: faces13/79 depth-exhausted, face109 cell-budget-exhausted and110..127 unvisited suffix; transported hints included[2,0,2]/[-2,0,0] and[0,-2,2]/[0,-2,0]. Evidence /tmp/sweep-closed-rational-charts-detail.log. This isolates projected rational enclosure loss, not an actual fold or a budget increase requirement.

Native surface_linear_monotonicity now first attempts exact integer projection of original retained Euclidean poles using checked dyadic exact_add/exact_mul. Weights, knots, degrees and source model remain unchanged. Exact pole projection commutes with the positive rational basis; outward projected jets are evaluated on that exact projected net before interval monotonicity/SPD proof. If any multiply/add rounds, exact projection is withheld and the original world-jet interval projection remains fallback. No rounded projected pole, source certificate or sampled projection is a positive premise. Projected injectivity implies actual surface injectivity; all original complete-domain and caller cell-budget tests remain mandatory. Arithmetic helpers are crate-private native reuse only, not TS geometry.

Focused128wall closed/shear/reflection chart test now passes1/1 in3.84s, same100000shared allowance; /tmp/sweep-closed-rational-charts-exact-projection.log. Native surface_linear_monotonicity subset6/6 passes0.02s including exact pole projection/rational basis preservation, 1+2^-54 rounding refusal, folded chart, generic unproved case and spatial rational projections. Retained-chart whole-body/budget suffix regression1/1 passes1.59s; diff check passed. Logs /tmp/sweep-rational-exact-projection-regression.log and /tmp/sweep-rational-projection-retained-chart-regression.log.

Full closed combined material target still refuses after17.93s, but stage evidence changed: charts=true and allFacesInjective=true; allPairsClassified=false and embedding=false. Nesting=null; shell orientations outward=null/attempts0. Bound8.672191698925102<=10 remains complete. Next real obligation is all distinct face-pair classification, then downstream nesting/orientation, without weakening Solid target or caller budgets. Evidence /tmp/sweep-closed-rational-material-exact-projection.log. Native closed correction/projection changes are unpublished; current8a746ea76STEP/28public/26UI evidence qualifies the prior open G1 package only. Full goal remains active; no external blocker.


## Closed rational contact proof completed in native Rust (2026-10-06)

The unchanged closed hollow authored-frame/guide/axisScale source with nonuniform positive profile weights and shear/reflection now passes complete within-budget continuousBound, profile G1, all face pairs, nesting, shell orientation and Solid. Before the repair, precisely 16 outer/inner pairs (0/6 through 120/126) remained unresolved despite exact boundaries, trims and all 128 injective charts. Centroid-direction-only pruning was insufficient. The sweep-specific trimmed search now tests outward restricted rational control enclosures along 13 fixed integer normals; a strictly positive whole-cell separation proves absence, while touching/crossing remains unresolved. Original cell budgets and frozen plain search remain unchanged. Native regressions: closed combined/chart tests 2/2 (18.39s), contact search 5/5, plain face contacts 3/3, oblique-gap touching/crossing refusal 1/1. The combined test also requires two closed shells, consistent nesting/orientation, zero-work withholding and immutable source. Logs: /tmp/sweep-closed-rational-global-bridge-regression.log, /tmp/sweep-rational-contact-search-regression.log, /tmp/sweep-rational-plain-contact-regression.log. New WASM publication is running; this native milestone is not yet public runtime/UI/STEP qualification. Full goal remains active.


## Closed rational public runtime and headless UI qualified (2026-10-06)

Publisher terminal0 produced WASM SHA256 915e2ef37e803126eb3170b252bd35e200153881d9c52b3ecbafd6ecf9e369b2, 11323237 bytes (release1m18s; optimized12743316 to11323237). Current package contains closed rational correction, exact projected charts and sweep-specific outward oblique contact pruning. Public four-file serial regression29/29 passed86.33s, including unchanged closed hollow frame/guide/axisScale/shear/reflection Rush source, profileG1/continuousBound/actualSolid replay, two closed shells and altered-pole replay refusal. Typecheck, Vite, browser-script syntax and diff checks passed. New headless Apple Metal UI13checks each at1440/600 passed26/26, headed=false, source/public/dist kernel binding verified; rendered profileG1 and sharp pathC0 are explicit. Artifact directory ui-closed-rational-current; new source examples/rush/closed-rational-frame-guide-affine-hollow-placed.r. This cancellation qualification proves held-dispatch lifecycle, not in-kernel interruption latency. Independent77case export is running; no new STEP success yet. Full six-part goal remains active, including all-mode/global and applicable smoothness scope beyond this family.


## Closed rational independent STEP milestone complete (2026-10-06)

All77cases pass sequential export, independent rational-boundary reference, polynomial-generator reference and OpenCascade validation on WASM915e2ef37e803126eb3170b252bd35e200153881d9c52b3ecbafd6ecf9e369b2 (11323237bytes). Evidence external-step-closed-rational-current/manifest.json and opencascade-sweep.json; logs /tmp/sweep-closed-rational-step-{export,reference,generator,occt}.log. New rush-closed-periodic-profile-rational-placed.step is valid1Solid/2closedoriented shells/128faces/256manifold opposite-use edges. All512wall coedges retain full UV domain, actual edge/surface bases and ownership agree, full-domain geometry checks pass and native/external material agreement is true. Independent rational quadrature16/32/64 gives[5.94616789558641,5.946167895586427,5.946167895586383]mm3; OCCT5.946167810009072, relativeerror1.4392010463538973e-8, within existing oracle tolerance (no tolerance increase). Sampled probes are supplementary, not whole-domain certificate premises. No caps in this closed case. Thus this unchanged nonuniform rational closed hollow frame/guide/axisScale/shear/reflection family now qualifies native→WASM→Rush→headless Apple Metal viewport→Solid→independent STEP, complete bound/profileG1/global material obligations included, sharp path C0 explicit. Public29/29/UI26/26/STEP77/77 all bind this package. Full goal remains active: finite family qualification is not all-mode global/continuous/smoothness proof, and held-dispatch cancellation is not in-kernel latency qualification.


## Running native cancellation, wide/narrow UI (2026-10-06)

Added an independent --native-running-cancel qualifier using CDP CPU sampling instead of Debugger.pause. Every accepted observation verifies the exact executing WASM bytecode SHA, exported abi_request function index87 and sample ancestry, requires at least5 native samples and an authenticated ABI stack in the final CPU sample with<=5ms tail-to-profile-end. Earlier/native-initialization-only or idle tails refuse. No synthetic Worker hold, Debugger suspension or altered miter geometry supplies this running observation. First attempt correctly refused after the build finished before cancellation; the final body stress request uses real max_sections513 and strict deviation, while the rational miter keeps original geometry and changes only source revision. The qualifier also records actual Worker.terminate invocation separately from CDP target-detachment confirmation; existing production cancellation immediately calls terminate and has no2second grace in exactSolidClient. No production geometry/admission algorithms changed for this qualifier.

Current closed authored tube source passed21checks each at1440/600,42/42 total: ui-native-running-cancel-final-current/matrix.json; /tmp/sweep-native-running-cancel-final-ui.log. Current unchanged closed nonuniform rational hollow frame/guide/axisScale/shear/reflection miter source passed19checks each,38/38 total: ui-closed-rational-native-cancel-current/matrix.json; /tmp/sweep-closed-rational-native-cancel-ui.log. Both are headless Apple/metal-3 and bind WASM915e2ef37e803126eb3170b252bd35e200153881d9c52b3ecbafd6ecf9e369b2/11323237bytes, public/packed/dist verified. Real running-build cancel, running-Solid cancel and running-Solid source supersession all terminate the attached Worker, publish no stale Solid and restore successfully; existing lifecycle/refusal/material/geometry presentation checks still pass. For rational miter terminate() is called1ms after build cancellation initiation,16..19ms for Solid cancellation and4..10ms for source replacement; CDP confirms target destruction70..83ms for build and2013..2025ms for Solid. For tube invocation0..34ms and detachment44..2005ms. Observation-to-cancellation initiation is0ms in measured runs. This is sampled native execution plus unpaused Worker cancellation with measured teardown; it is not a hard universal latency bound, cooperative in-kernel polling proof or an assertion that the exact sampled stack persists until the physical UI event. Sharp path C0 remains explicit.

The UI obligation now has actual unpaused native-execution evidence for these two qualified body families rather than only held-dispatch/Debugger-paused lifecycle evidence. The original full objective remains active: remaining all-mode E/R/I/J, applicable moving-frame/closed-seam smoothness, and complete combination qualification cannot be inferred from these finite cases. Previous package29public/77STEP proofs remain separately bound to the same WASM; no redundant kernel rebuild was required.


## Original piecewise-Bezier moving-frame jets, native milestone (2026-10-06)

Current source adds cad-predicates rational_piecewise_bezier_knot_jet_identity over original Euclidean poles, positive weights and original knots. All clamped piecewise-Bezier spans and every interior join must be covered; only original knot multiplicities degree or degree+1 are eligible. Endpoint positive projective scale, homogeneous value/first/second jets and original span widths are cross-multiplied in exact expansions; no rounded extraction or derivatives are positive premises. General non-piecewise-Bezier insufficient bases remain unproved. Continuous source-frame certification now first uses guaranteed basis smoothness, then this bounded exact actual-jet proof for authored axis/normal/twist or guide laws. Knot proof and existing closed endpoint proof share original maxExactWork; planar-source proof receives only the remaining allowance. Whole-frame regularity still separately required.

Native original_smoothness9/9 and cad-predicates full27/27 pass, including open moving laws with unequal span widths, nonuniform rational weights, fully repeated knot with positive projective scale, one-bit C2-only mismatch retaining C1, C0 mismatch, changed widths, nonpositive weights, zero/exact-one-under work refusal, zero regularity cells and closed moving piecewise-Bezier internal plus endpoint joins for parameter/arc_length. Logs /tmp/sweep-original-piecewise-frame-regression.log and /tmp/sweep-knot-predicates-full-regression.log. New closed authored Rush surface fixture authored-closed-piecewise-c2-progressive-sweep.r is staged alongside public negative/budget tests. New WASM publisher /tmp/sweep-piecewise-frame-kernel-publish.log live; vue-tsc and diff check pass. No public new-jet/viewport qualification yet. Historical915e2ef77STEP/29public/42+38runningUI pertains to previous package, not the new unpublished code. Original-frame C2 is independent of path/retained/profile/cap seams, continuous reconstruction error and Solid. Full goal remains active.


## Original piecewise closed moving-frame C2 publicly qualified (2026-10-06)

Publisher terminal0: release1m22s, wasm-opt12754776->11333068bytes. Current WASM SHA25607bf24f5d32e4629bdb23434febfa91870fa56a898ef6397844cb590e36da4fa contains original piecewise-Bezier knot jets. Public frame suites18/18 in3.04s pass: nurbsOriginalFrameSmoothness, nurbsClosedAuthoredFrameSmoothness, nurbsAuthoredFrameRegularity, nurbsRetainedSweepSmoothness. They cover exact C2 over repeated internal knots and independent widths, one-bit internal second-jet refusal preserving C1, nonuniform rational input, zero/one-under exact budgets, original source-frame-only scope and parameter/arc_length closed preview. Typecheck/Vite/browser syntax/diff checks pass. Logs /tmp/sweep-piecewise-frame-{kernel-publish,typecheck,public,vite}.log.

Headless Apple/metal-3 UI15checks at1440 and600,30/30 total, passed with no page errors and verified source/packed/public/dist SHA binding. Artifact ui-piecewise-closed-frame-current/matrix.json, log /tmp/sweep-piecewise-closed-frame-ui.log. New original authored-closed-piecewise-c2-progressive-sweep.r renders closed source-frame C2 from the actual native proof; changing an interior pole by one binary64 step removes the C2 presentation, restoring original source restores it. Surface-only Solid refusal, cancellation, replacement and restoration still pass. Source-frame C2 does not promote retained wall/path/profile/cap smoothness, whole boundary error, global material guarantees or Solid. Historical915e2ef77STEP/29miter public/42+38runningUI remains explicitly previous-package evidence and is not relabelled to07bf24f5. Full original six-part goal remains active. Remaining source-knot proof paths currently still use basis-continuity guards for required path orders3/4; arbitrary non-piecewise-Bezier repeated-knot jets, spatial RMF transport and corrected-Frenet fallback transitions remain separate obligations, along with remaining all-mode E/R/I/J and combination qualification.


## General original B-spline internal C1..C4 jets, native milestone (2026-10-06)

New cad-predicates rational_bspline_internal_jet_identity evaluates exact formal one-sided de Boor power series over original authored Euclidean poles, positive weights and knots. Rational expansion numerators/denominators carry all operations without floating-point division; Cartesian quotient-series jets are compared by exact cross products. Every interior nonempty-span boundary, mixed/repeated knots and orders1..4 are covered. No rounded knot insertion, computed derivative or sampled value is a positive premise. Bounded arithmetic/work/precision exhaustion remains indeterminate. Unlike the earlier sufficient piecewise-Bezier constant homogeneous scale condition, constant Cartesian fields with unequal homogeneous derivative scales are certifiable.

Native endpoint_jets::certify_knots keeps the prior fast piecewise-Bezier proof for orders<=2, then falls back to general Cartesian jets using the same remaining PredicateContext allowance. Required original path continuity for C2 FixedNormal/planarRMF and Frenet now uses actual C3/C4 path jets, respectively, rather than basis multiplicity alone. Source frame nondegeneracy/speed/curvature and arc inverse prerequisites remain mandatory. Higher original path jets do not certify retained seams or Solid. Native original_smoothness11/11 and full cad-predicates31/31 pass (logs /tmp/sweep-general-source-frames-full.log and /tmp/sweep-general-jet-predicates-full.log), including exact curved polynomial/rational simple knots, mixed multiplicities in both station spacings, one-bit and changed-width refusals, positive Cartesian identity despite nonmatching homogeneous jets, all-joints coverage/one-under shared work, and an independently changed fourth jet that preserves C3 but refuses C4.

New public original-frame suite and mixed-knots-curved-frenet-progressive-sweep.r are staged; vue-tsc and diff checks pass. WASM publisher /tmp/sweep-general-source-jets-kernel-publish.log is live; no public new-general-jet/UI qualification yet. Prior07bf24f5/18frame-public/30piecewise-UI evidence remains previous-package evidence, not this unpublished extension. Full original goal remains active, including spatial RMF transport, corrected-Frenet fallback transitions, full all-mode E/R/I/J and combination qualifications.


## General original path jets publicly qualified (2026-10-06)

Publisher terminal0: release1m29s, wasm-opt12772141->11347314bytes. Current WASM SHA2568131831c22d1041e0cf33687c2053b99673cc2887618d40d3828245e19b64e79 includes exact Cartesian internal B-spline jets and required C3/C4 path guards. Four-file serial public frame regression19/19 passes3.05s, including polynomial mixed knots and rational simple knots in FixedNormal/Frenet/planarRMF, parameter/arc_length, one-bit C2 refusal and exact-one-under shared work. Logs /tmp/sweep-general-source-jets-{kernel-publish,typecheck,public,vite}.log. Native11/11 frame and31/31 predicate regressions remain passing; source-frame/retained/profile/cap/Solid scopes stay independent.

Headless Apple/metal-3 UI15checks each at1440/600,30/30 total, passes with no page errors and source/packed/public/dist SHA verification: ui-general-source-jets-current/matrix.json, /tmp/sweep-general-source-jets-ui.log. New mixed-knots-curved-frenet-progressive-sweep.r retains original curved Frenet geometry with scale/twist/axisScale/center laws; source-frame C2 is rendered, a single-bit original path-pole change removes it, and restoration restores it. Surface-only Solid refusal, cancel, source replacement, persistence/restoration and invalid-path refusal all pass. Initial run correctly stopped because the negative-path recipe did not mutate a nurbs_curve source; the recipe now explicitly replaces an authored NURBS path by a zero-length Bezier for the refusal case. No production input path or geometry is changed by this test recipe. Browser syntax, typecheck, Vite and diff checks pass.

This resolves the earlier source-path orders3/4 basis-multiplicity limitation and general non-piecewise-Bezier internal knot jets for bounded nonperiodic inputs. It does not resolve closed Cartesian endpoint jets outside the existing sufficient clamped/homogeneous-scale certificate, spatial RMF transport or corrected-Frenet fallback transitions. All-mode continuous body/global material and applicable retained G1/G2 obligations, plus their STEP/UI combinations, remain active. Prior07bf24f5 and915e2ef STEP/UI evidence is preserved with its actual package identity, not relabelled to8131831c. Full original goal remains active.


## General Cartesian closed endpoint jets, native milestone (2026-10-06)

New rational_bspline_cartesian_endpoint_jet_identity reuses exact original de Boor/Cartesian quotient series at both active-domain endpoints, including nonclamped knot vectors. Positive original weights, finite sorted knots and shared exact work/precision checks are mandatory. It compares real Cartesian value and derivatives through requested order; unequal homogeneous endpoint derivatives are not a refusal premise. Endpoint and internal-join proofs remain separate. Native endpoint_jets::certify preserves the prior sufficient homogeneous/Bezier fast path, then gives only remaining work to Cartesian endpoint fallback; zero/one-under budget never succeeds. Periodic curve semantics and degree-zero handling remain explicit prior refusals. No rounded endpoint derivative or knot insertion is a positive premise.

New original quintic moving-axis fixture has weights[1,.5,1,1,2,1] and original Euclidean controls[[0,0,1],[.125,0,1],[.046875,.125,1],[-.28125,.125,1],[-.03125,0,1],[0,0,1]]. Its Cartesian C1/C2 endpoints match, while homogeneous weight derivatives differ; no fixed homogeneous seam scale proves it. Nonclamped constant-direction degree2 law with knots[-2,-1,0,.5,1,2,3] and weights[1,2,3,4] also proves source closed-frame C2. Both parameter/arc_length variants, one-bit C2 refusal retaining C1, exact-one-under/zero work and original whole-frame regularity pass. Native original_smoothness12/12, predicates32/32 and endpoint-jets regression6/6 pass; logs /tmp/sweep-cartesian-endpoint-{frames-full,predicates-full,native}.log.

New authored-closed-cartesian-c2-progressive-sweep.r and authored-closed-nonclamped-c2-progressive-sweep.r are staged with public preview, scope and budget tests and UI presentation/refusal/restoration cases. Typecheck and diff pass. Publisher /tmp/sweep-cartesian-closed-frame-kernel-publish.log is live; no new public/UI qualification yet. Previous8131831c/19public/30mixed-pathUI remains prior-package evidence. Source closed-frame C2 still does not prove path, retained, profile or cap seams, full boundary error, global material or Solid. Full goal remains active.


## Cartesian closed source-frame endpoints publicly qualified (2026-10-06)

Publisher terminal0: release1m19s, wasm-opt12775921->11350601bytes; current WASM SHA2568fe9df5add9151ad67f72e8d9e9b23b0b2414642b3d63350d1c873eb3502bf3f. Serial four-file public frame regression20/20 passes3.73s, including actual nonnull preview for both new source law families at parameter/arc_length spacing, exact-one-under budget refusal, and one-bit C2 refusal preserving C1. Native frame12/12, predicate32/32 and endpoint6/6 regressions pass. Typecheck/Vite/browser syntax/diff checks pass. Logs /tmp/sweep-cartesian-closed-frame-{kernel-publish,typecheck,public,vite}.log.

Headless Apple/metal-3 current-package UI: authored-closed-cartesian-c2-progressive-sweep.r15checks at1440/600 (30/30), authored-closed-nonclamped-c2-progressive-sweep.r14checks each (28/28);58/58 total, no page errors, source/packed/public/dist SHA verified. Artifacts ui-cartesian-closed-frame-current/matrix.json and ui-nonclamped-closed-frame-current/matrix.json; logs /tmp/sweep-cartesian-closed-frame-ui.log and /tmp/sweep-nonclamped-closed-frame-ui.log. Actual closed source-frame C2 presentation, one-bit second-jet refusal/restoration, cancellation/source supersession, viewport material controls, persistence/restoration, invalid path refusal and surface-only Solid refusal all pass. Source Cartesian closed-frame C2 remains independent of closed path/retained/profile/cap joins, whole boundary error, global embedding and Solid.

This resolves the previously sufficient-only constant homogeneous endpoint-scale and clamped-basis limitations for bounded nonperiodic original source law endpoint jets. It does not claim full closed-frame coverage in other moving-frame orientations or periodic law semantics. Spatial RMF transport, corrected-Frenet fallback transitions, applicable retained smoothness and all-mode continuous/global body guarantees plus full STEP/UI combinations remain active requirements. Earlier8131831c and915e2ef qualification artifacts keep their original WASM bindings; no old77STEP result is relabelled to8fe9df5a. Full original objective remains active.


## Corrected-Frenet planar transport degeneracy, native milestone (2026-10-06)

A new regular original planar quartic exposes a real constructor defect at two stations: opposite endpoint tangents parallel to the endpoint chord make the second double-reflection direction zero, and corrected-Frenet reverses the profile binormal. The regression failed before the change (/tmp/sweep-corrected-antiparallel-before.log, count=2). The source speed is independently covered by curve_regularity::inspect; the test does not assume regularity from sampled tangents.

For corrected-Frenet without authored frame/guide overrides, exact original coefficient planarity now permits transport in the constant-plane Bishop basis B and B cross T. Carrying the two transverse coefficients preserves the normal phase even at an antiparallel or zero-second-reflection step. The same transport is used while back-resolving the initial normal from the first available principal direction and while advancing stations. Planarity is admitted only by the existing exact axial coefficient identity or the bounded original source_plane predicate (1000000 work); unproved/spatial inputs keep the previous transport. No fitted/sample plane, profile resizing, source mutation or tolerance change is used.

Native corrected-Frenet6/6 and full progressive_sweep143/143 pass. Regressions cover axial/oblique source planes, independent regularity, parameter/arc_length spacing, counts2/3/5/9/17, prior inflection/initial straight/C1 join/closed full-turn behavior, immutable source coefficients, and one-bit off-plane refusal of the exact plane premise. Logs /tmp/sweep-corrected-planar-{native-final,regression}.log. Typecheck, browser-script syntax and diff checks pass. Public WASM/Rush regressions and a separate corrected-frenet-antiparallel-progressive-sweep.r UI fixture are prepared; current publisher /tmp/sweep-corrected-planar-publish.log remains live and public qualification is not yet claimed.

This fixes actual preview-frame construction, not the source-frame C1/C2 certificate or full continuousBound. Corrected-Frenet source-frame smoothness and original-transfer error remain explicitly unproved. The UI case must preserve these statuses and refuse Solid; ordinary Frenet regularity must not be reused to certify an inflection/fallback mode. Spatial corrected transport and continuous fallback/correspondence estimates, all-mode global guarantees, applicable retained smoothness and full STEP/UI coverage remain active. Earlier8fe9df5a and915e2ef qualification stays bound to its original packages. The original six-part goal is not complete.


## Corrected-Frenet planar transport publicly qualified (2026-10-06)

Publisher terminal0: release1m24s, wasm-opt12776731->11351377bytes. Current source/packed/public/dist WASM SHA256b361a3ce3d3263ca96d245e56bbbc0e41b857895c3cbc67bb6c3c8b0a9f0471f. Five serial public files pass61/61 in8.30s: nurbsProgressiveSweep, nurbsOriginalFrameSmoothness, nurbsClosedAuthoredFrameSmoothness, nurbsAuthoredFrameRegularity and nurbsRetainedSweepSmoothness. Public WASM regressions preserve axial/oblique planar binormal offsets for parameter/arc_length and counts2/3/5/9/17; Rush compiles/builds the formerly failing two-station quartic with correct offsets. Source-frame C2 and continuousBound remain false. Native final corrected-Frenet6/6 and progressive_sweep143/143, typecheck/Vite/browser syntax/diff checks pass. Logs /tmp/sweep-corrected-planar-{publish,public,native-final,regression,typecheck,vite}.log.

Headless Apple/metal-3 UI14checks each at1440/600,28/28 total, passes with no page errors and source/packed/public/dist SHA verification. Artifacts ui-corrected-planar-antiparallel-current/matrix.json, /tmp/sweep-corrected-planar-ui.log. Actual nonaxial corrected-Frenet preview, explicit unproved source C2/error presentation, viewport/material controls, surface-only Solid refusal, invalid-path refusal, cancellation/source supersession and restoration pass. This run's cancellation tests use held worker dispatch and prove lifecycle behavior only, not mid-native interruption latency. Prior915e2ef running-native cancellation and77STEP evidence remain bound to that earlier package, not relabelled to b361a3ce.

This closes the reproduced planar antiparallel transport reversal in native/WASM/Rush/viewport preview construction. It does not close original continuous corrected-Frenet fallback/correspondence error, source-frame smoothness certification, spatial RMF/corrected transport, full all-mode regularity/global material/smoothness or STEP/UI combinations. Full original six-part completion contract remains active.


## Corrected-Frenet original planar source C1/C2, native milestone (2026-10-06)

The source-frame smoothness audit now covers open corrected-Frenet on exact original planar paths, including curvature zeros and inflections. Actual original internal Cartesian path C(k+1) and twist Ck are mandatory; a constant source plane normal B and positive speed give Ck unit tangent T and basis B cross T, B. The signed principal normal has this continuous planar extension through zero curvature; planar RMF fallback preserves the basis or a constant transverse phase on an entirely straight source. Original degree-one rational lines additionally use their exact constant tangent/seed premise. Arc-length composition uses the proved positive speed and the inverse-function theorem, not sampled inverse derivatives.

Exact source_plane proof and any actual internal-knot jets share the one caller maxExactWork; plane proof never restarts that budget. The complete fixed-normal adaptive cover is used only as a boolean nondegeneracy/speed premise. Its T/B interval jets are neither returned as corrected-Frenet jets nor used for retained correction/error. Closed and spatial corrected sources still refuse this source certificate. Scope remains open-original-frame-only, with retained/profile/cap joins, full continuousBound and Solid explicitly false.

Native original_smoothness13/13 and full progressive_sweep144/144 pass; logs /tmp/sweep-corrected-source-{frames-native,regression}.log. Positive polynomial/rational inflections, zero principal curvature at the midpoint, parameter/arc_length, C1/C2, actual C3 mixed-knot path jets and immutable original poles/weights/knots are covered. Zero/one-under exact work and adaptive cells, one-bit original off-plane coefficients, open stationary source speed and closed path refuse. Ordinary Frenet nonzero-curvature admission is not reused as corrected-Frenet evidence. Typecheck/browser-script syntax/diff pass. New public scope/budget tests and wide/narrow off-plane-refusal/restoration UI checks are staged; publisher /tmp/sweep-corrected-source-publish.log is live. No new public/STEP/UI result is claimed before its actual completion.

This advances applicable original moving-frame smoothness, not full continuous error, spatial transport, retained multispan/closed/cap smoothness or all-mode global material. All six original requirements remain active; previous b361a3ce preview construction/UI evidence keeps its prior package identity.


## Corrected-Frenet original planar C1/C2 publicly qualified (2026-10-06)

Publisher terminal0: release2m11s, wasm-opt12777324->11351964bytes; current WASM SHA256ed3b4fba24a0a2440c31e586eb20cd266680397d0e18fe5a57e6dd2ba4f2fe3e. Five serial public frame/progressive files pass62/62 in9.36s, including polynomial/rational midpoint curvature-zero source C1/C2 at parameter/arc_length spacing, zero/one-under cell/exact-work refusal, off-plane original-pole refusal, stationary source-speed refusal, actual higher Cartesian mixed-knot jets and a nonnull preview whose source C2 is true while full continuousBound/Solid scope stays false. Ordinary Frenet refuses the same inflection. Native original_smoothness13/13 and whole progressive_sweep144/144 pass; typecheck/Vite/browser syntax/diff checks pass. Logs /tmp/sweep-corrected-source-{publish,frames-native,regression,public,typecheck,vite}.log.

Headless Apple/metal-3 UI: corrected-frenet-sweep.r14checks each at1440/600 (28/28), corrected-frenet-antiparallel-progressive-sweep.r14checks each (28/28);56/56 total. Both reports bind current SHA and verify source/packed/public/dist identity with no page errors. Artifacts ui-corrected-source-inflection-current/matrix.json and ui-corrected-source-antiparallel-current/matrix.json; logs /tmp/sweep-corrected-source-ui-inflection-final2.log and /tmp/sweep-corrected-source-ui-antiparallel.log. They verify real source-C2 presentation independently of unproved original-transfer error, off-plane/source refusal and restoration, material/viewport controls, surface-only Solid refusal, lifecycle cancellation/source supersession, invalid-path refusal and empty-project restoration. Cancellation remains held-dispatch lifecycle evidence, not mid-native latency.

The first inflection UI negative recipe incorrectly expected successful sampled refinement after a tiny spatial pole perturbation. The real constructor correctly refused: sampled refinement2.121320396571442mm exceeds the unchanged0.001mm budget at257sections. The scenario now checks that exact refusal, clears the previous planar C2 evidence and restores the original source with C2 certified/error unproved. No source budget, tolerance, section limit or production refusal was weakened. The oblique antiparallel negative changes one binary64 x pole and verifies source C2 unproved with accepted preview, then restores it. Both layouts pass these scoped expectations.

Fresh77/77 independent STEP qualification on ed3b4fba completes export, rational-boundary quadrature, polynomial generator reference and OCCT import/geometry/topology/volume validation, each terminal0 and sequentially mutating the same manifest. Manifest and opencascade-sweep.json both bind the actual current11351964byte public/packed package. Evidence external-step-corrected-source-current; logs /tmp/sweep-corrected-source-step-{export,reference,generator,occt}.log. This requalifies the existing finite77-case matrix after the new WASM publication; it does not add a corrected-Frenet Solid case or prove all-mode material/continuous/smoothness. New source-C2-positive corrected previews still have explicit unproved full error, retained/profile/cap joins and Solid.

Applicable open planar corrected-Frenet original-frame C1/C2 is now covered, including inflection/fallback extension, exact source planarity, source regularity and original Ck path/law knots under shared limits. Closed/spatial corrected source smoothness, numerical reference/correspondence through general fallback, general RMF, applicable retained multispan/closed/cap joins, all-mode regularity/intersections/holes/nesting/orientation and remaining STEP/UI combinations remain active. Next quantitative obligation is an original corrected planar transport reference with endpoint arithmetic and whole station/law/retained decomposition error, rather than reusing a source-smoothness boolean for continuousBound. Scalar-scale/twist planar world transport is invariant under constant transverse frame phase; anisotropic affine/center laws need their own authored-frame-coordinate correspondence and must not be silently aliased to RMF. The six-part original goal remains incomplete.


## Corrected planar continuous error and complete contact coverage, qualified milestone (2026-10-06)

Current published/packed/public/dist geometry WASM SHA256 `6478253977ba26f0772db3f288e5956fdfa44a59237fa1697e9fa8fc5e806913`, 11366028 bytes. Publisher and Vite both completed successfully. All geometry, continuous estimates, face grouping, nesting and Solid admission below are Rust; TypeScript changes are transport, diagnostics and qualification only. Verification stayed terminal/headless and did not switch foreground windows.

Open exact-original planar corrected-Frenet scalar-scale/twist now has a complete original-reference continuous bound, including source/law/frame transport, arithmetic, retained decomposition/correction and filled original-profile caps. Constant transverse phase preserves the world reference. Exact positive rational degree-one straight sources additionally support affine axis scale and center displacement. Curved corrected affine/center still refuses with `corrected-affine-frame-phase-unproved`; closed corrected correspondence and unproved spatial cases also refuse. Authored/guide modes retain their own reference ownership. This is qualified coverage of these modes, not a universal continuousBound certificate.

Physical source-parameter speed lower bounds account for adaptive cell width. Positive original speed tightens early arc-inversion brackets from residual/speed; zero/partial regularity covers never supply a lower bound. Complete adaptive frame value covers share their caller budget. Original exact plane correlation supplies the constant-binormal basis and avoids false oblique interval overestimation. The additional complete value bound is requested lazily when the existing jet bound is insufficient, preserving valid closed-RMF work budgets. Independent oblique quartic reference checks cover counts 2/3/5/9/17 without changing source tolerances or station limits.

Native full original positive-weight surface hulls and valid in-domain trim hulls now certify strictly separated contiguous face groups. Every preparation pole, tree node and traversal attempt consumes the same geometry-cell budget. Groups and individual records partition every unordered face pair; maxPairs remains the individual-classification limit, and no budget is restarted. Exact joint hull contact certificates are constructed on demand only under fresh native boundary/trim/orientation/injectivity guards. Cross-shell nesting consumes actual scalar/group coverage. Fresh qualitative separation does not invent a numeric clearance: distance lower remains zero, upper absent and unconverged when no quantitative distance was proved.

The 258-face corrected inflection arc-length hollow body covers all 33153 unordered pairs within the original 10000 individual-pair / 100000 geometry-cell / 1000000 UV-cell limits and passes fresh native Solid admission. Closed planar full-turn RMF also passes Solid in the original 10000 individual-pair limit. Native tests retain input immutability, zero/one-under limits, touching/invalid trims and wrong shell orientation refusal. No all-mode global claim follows from these fixtures.

Validation: progressive_sweep 147/147; brep-core 787 passed with 2 existing ignored; the closed-RMF default-10000 regression passes; published public tests 103/103 in nine serial files. Typecheck, browser syntax and git diff checks pass. Logs `/tmp/sweep-proved-planar-values-native-final2.log`, `/tmp/sweep-proved-planar-brep-native-final.log`, `/tmp/sweep-proved-planar-closed-default-native.log`, `/tmp/sweep-final-public.log`.

Independent STEP export, rational-boundary quadrature, exact polynomial generator reference and OCCT qualification ran sequentially on the same current manifest: 80/80 pass, with current package identity verified. Evidence `external-step-corrected-bound-current/opencascade-sweep.json`; logs `/tmp/sweep-final-step-{export,reference,generator,occt}.log`. The straight affine fixture previously omitted its affine laws; the independent volume check caught 30 versus the expected 180 mm3. Its real axis_scale [2,3,1] and center [.125,-.25,0] are now authored, graph/bounding-box assertions guard the source, and OCCT confirms 180 mm3. Curved rational-wall reference integration is converged numerical quadrature, not an interval oracle. The finite 80-case matrix does not prove every combination.

Six current headless Apple/metal-3 UI reports pass at widths 1440 and 600: 176 assertions, no page errors, actual source/packed/public/dist identity verified. Reports: `ui-corrected-bound-antiparallel-current`, `ui-corrected-bound-straight-current`, `ui-corrected-bound-inflection-current`, `ui-closed-planar-default-budget-current`, `ui-corrected-bound-surface-current`, `ui-corrected-bound-arc-current` (each `matrix.json`). They cover applicable Solid success, explicit refusal, material controls, cancellation, source replacement and restoration. These successful current cancellation checks hold worker dispatch and qualify lifecycle only. An additional unpaused CPU-sampling cancellation attempt failed to observe authenticated geometry ABI at its profile tail; current mid-native cancellation is therefore NOT qualified. No older package cancellation result is relabelled as current.

The full six-part goal remains active. Remaining obligations include general spatial/closed corrected and RMF continuous/frame correspondence, curved corrected affine/center coordinate phase, applicable retained G1/G2/multispan/closed/profile/cap joins, all-mode regularity/intersections/holes/nesting/orientation, the remaining STEP/UI combinations and current actual-running native cancellation. Sharp miter joins remain explicit C0. Prior package artifacts keep their original identities.


## Unpaused running-native cancellation qualified on current package (2026-10-06)

This supersedes the earlier current-package cancellation limitation only for the following qualified source/scenarios. `ui-running-cancel-current/matrix.json` binds SHA256 `6478253977ba26f0772db3f288e5956fdfa44a59237fa1697e9fa8fc5e806913` and verifies actual source/packed/public/dist identity. Headless Apple/metal-3 at widths 1440/600 passes 21 assertions each (42 total), no page errors. Actual unpaused build cancellation, actual unpaused Solid cancellation and source replacement during actual unpaused Solid each pass at both widths, including restoration, Worker.terminate invocation and absence of stale body publication.

The qualification probe authenticates the full observed geometry WASM bytes and exported ABI function index 87, requires at least five CPU samples inside that ABI and an ABI-active last sample no more than 5 ms from profile end. It never pauses execution. Module retrieval/hash work invalidates that observation window; after authentication, a fresh CPU profile is mandatory. Profile receipt/processing timestamps and observation-to-UI/terminate/destruction timings are recorded in each assertion. These are measured orchestration observations, not a universal cooperative cancellation or latency guarantee.

Two actual qualification defects were corrected: the CPU profiler now arms before the build click, and the busy arc-length cancellation fixture stays inside the native 257-section ceiling (initial/max 257). The previous max513 workload could fail immediately on unsupported options and never reach geometry ABI execution. Only this cancellation workload uses 257 initial sections and the tighter deviation; accepted product examples and original qualification budgets are unchanged. SHA validation, active-tail admission and minimum native sample count were not weakened. Failed diagnostic runs remain explicitly failed and are not counted as successful checks.

Verification log `/tmp/sweep-running-cancel-final.log`, browser/probe syntax and git diff checks pass. No geometry or admission code changed in this cancellation milestone, so the existing current-package native 147/147 sweep, 787 BRep passed/2 ignored, 103/103 public and 80/80 STEP results retain their exact package binding. The earlier six regular UI reports retain their 176 passed lifecycle/product assertions; this separate 42-assertion report adds actual-running evidence for one arc-length hollow source. It does not qualify every mode or shrink the original six-part goal.

Curved corrected-Frenet affine/center phase remains a genuine proof obligation: the constructor chooses the first available sampled principal normal using its degeneracy threshold and can back-transport it from a later station. A smoothness boolean cannot prove that authored-coordinate phase. General spatial/closed continuous/frame correspondence, remaining applicable G1/G2 and multispan/closed/profile/cap joins, all-mode global material guarantees and remaining STEP/UI combinations stay active.


## Curved corrected-Frenet affine/center principal phase, publicly qualified (2026-10-06)

Published/packed/public/dist current geometry WASM SHA256 `b67abc1e842c438caf5cd214f12da75049a97ef4ddb72a1154ea6bdab3e53043`, 11372080 bytes. Publisher terminal0; raw wasm-opt12799200->11372080. Production changes are entirely Rust. TypeScript additions author/transport fixtures and test the actual Rush inputs; they implement no geometry, error estimate or admission.

For an open exactly proved original planar path with certified nonzero initial curvature, original initial Frenet and canonical plane-frame interval values prove a strict principal-phase sign. The planar principal extension is an exact signed quarter-turn of the canonical plane basis; swaps/sign changes commute with twist and avoid an approximate pi/2, fitted phase or sampled inference. The same source-owned phase supplies original profile coordinates, original values/jets, affine axis-scale/center laws, parameter and independently inverted arc-length reference, retained interpolation/decomposition/correction and filled endpoint-domain transfer. Phase/source/frame work is charged to each caller's existing remaining budget; no new budget or reset is introduced.

This defines the original ideal reference independently of station count. It does not assume that floating-point constructor branch selection or endpoint normalization is exact: actual corrected sections remain the owner of endpoint displacement, so actual initial phase/rounding mismatch is enclosed in the error. Constructor sampling/threshold behavior is unchanged. Zero or unproved initial source curvature still returns `corrected-affine-frame-phase-unproved`; closed/spatial corrected correspondence remains unproved. The previous blanket curved affine/center refusal is therefore superseded only for this proved initial-principal-phase family.

Independent native rational-quotient regressions cover polynomial/rational cubic inflection paths, axial and oblique source planes, opposite/nonunit plane seeds, original path domain [2,5], varying rational anisotropic axis laws on [7,9], varying rational center laws on [11,13], scalar scale and twist, dense retained-versus-original references, unchanged original poles/weights/knots and zero/one-under work refusals. Nonzero source initial curvature is proved separately from later inflections. The initial-zero-curvature negative remains a real refusal. Native progressive_sweep148/148 and latest independent varying-law reference1/1 pass; BRep788 passed/2 existing ignored. A new hollow-body native regression covers parameter and arc-length affine/center through full wall/filled-cap boundary and fresh global Solid admission with original10000 individual pairs/100000 geometry cells/1000000 UV cells, exhaustive scalar+group pair coverage and model immutability. Logs `/tmp/sweep-planar-phase-{native,oblique,body,brep-native}.log`.

Two real Rush examples are added: `corrected-frenet-affine-inflection-hollow-body-boundary.r` and `arc-length-corrected-frenet-affine-inflection-hollow-body-boundary.r`. Actual axis_scale[1.25,.75,1] and center_law[.002,-.003,0] are guarded in both public tests and STEP exporter so the fixture cannot accidentally omit its advertised laws. The two examples preserve the scalar case's original profile/path, maxDeviation.1mm, initial3/max65 sections and original length/cap correction limits. Current nine serial public WASM/Rush/frame/body/embedding/filled-domain files pass104/104 in68.41s. Vue/MCP typechecks, Vite, browser syntax and diff checks pass. Log `/tmp/sweep-planar-phase-public.log`.

Headless Apple/metal-3 UI at widths1440/600 passes72 assertions: parameter15+15 and arc-length21+21, no page errors, actual source/packed/public/dist identity verified. Reports `ui-planar-principal-phase-parameter-current/matrix.json` and `ui-planar-principal-phase-arc-current/matrix.json`. These cover actual Solid success, strict budget/invalid-source refusal, viewport/material controls, cancellation, source supersession and restoration. The arc report additionally proves six actual unpaused Rust ABI interruptions (build cancellation/Solid cancellation/Solid source replacement at each width), with module authentication followed by a fresh active-tail CPU profile and Worker termination. Timings remain measured orchestration observations, not a universal latency guarantee. Earlier647825 reports retain their old package binding.

Independent STEP matrix expands80->82 with the two new parameter/arc affine/center hollow fixtures. Current export, converged rational-boundary volume reference, exact polynomial generator reference and OCCT import/geometry/topology/volume verification ran sequentially and all completed terminal0.82/82 cases pass and bind the current11372080-byte package; every native fixture retains maxPairs10000. Evidence `external-step-planar-principal-phase-current/opencascade-sweep.json`; logs `/tmp/sweep-planar-phase-step-{export,reference,generator,occt}.log`. Numerical rational-wall quadrature is explicitly a converged reference, not an interval oracle. This finite matrix does not prove every mode or combination.

The full original six-part goal remains active. Next phase obligation is original-source correspondence at initially zero principal curvature (including first-principal back resolution and general center laws), followed by spatial/closed corrected and general RMF transport/error. Remaining applicable retained G1/G2, multispan/closed/profile/cap joins, all-mode regularity/intersections/holes/nesting/orientation and unqualified STEP/UI combinations still require their own evidence. Sharp miter remains explicit C0. No all-mode/global completion claim is made.


## Zero-initial-curvature original phase and count-stable constructor, qualified (2026-10-06)

Current source/packed/public/dist geometry WASM SHA256 `a504bdb5f118e854ddff8e1ea2743a44cfde2bec13470c3fce4bb74ae82fcc93`,11374781 bytes. Publisher terminal0; release1m26s, wasm-opt12802448->11374781. Runtime geometry, phase selection and continuous/error/admission proofs remain entirely Rust. TS changes add qualification inputs and assertions only.

The source-owned initial-principal reference now extends to regular nonperiodic clamped rational Bezier paths whose initial curvature vanishes and whose first transverse coefficient is nonzero. After an exact original plane premise and certified nonzero initial tangent, exact cyclic projected orient2d predicates inspect original poles against P0/P1. Exact-zero collinear prefixes are skipped; the first off-tangent coefficient determines the leading curvature sign. The full source plane makes the cross product parallel to its normal, so one nonzero normal component fixes the 3D sign without rounded differences. Positive homogeneous weights make the first transverse Taylor coefficient relative to P0 a positive multiple of the first off-tangent pole. The phase is therefore independent of station count, later curvature signs and fitted/sample planes. Failed Frenet work, initial tangent values and every exact predicate consume the existing shared remaining budget. No tolerance or budget is reset.

A real constructor defect was reproduced before the fix: C(t)=(t,t^3-2t^4,0) has initial principal limit+Y, but counts2/3/5 found their first sampled principal direction after its sign change, while count9 found it before. With anisotropic axes and center[.125,.25,0], the initial first pole incorrectly became[0,.1875,-.0625] instead of[0,.4375,.4375] at count2. Regression `/tmp/sweep-zero-phase-count-before.log` failed on the actual coordinates. For open exactly proved planar paths whose numerical initial principal normal is absent, construction now uses the proved original phase when available; fallback remains for unproved cases. The same regression passes for counts2/3/5/9/17 (`/tmp/sweep-zero-phase-count-after.log`) and through published WASM previews. Nonzero numerical initial principal branches and closed/spatial fallback are preserved. Actual endpoint arithmetic and retained correction/decomposition remain charged in the continuous bound.

Independent native quotient references cover degrees3/4/5, polynomial and positive-weight rational sources, axial/oblique planes, original domain[2,5], affine/center plus scale/twist, dense retained-versus-original comparisons, input immutability and zero/one-under budget refusal. Exact coefficient tests cover a collinear prefix at coordinates1e16, both normal signs, opposite first transverse signs, exhausted exact work and an entirely collinear source. Fully collinear higher-degree curves, unproved initial tangent and unsupported unclamped/multispan initial-leading correspondence retain refusal; this milestone does not silently treat them as two-pole lines. Logs `/tmp/sweep-zero-phase-{reference,exact}.log`.

Final native progressive_sweep150/150 and BRep789 passed/2 existing ignored; the exact leading-predicate regression also passes. Native hollow affine/center with zero initial curvature qualifies parameter and arc_length complete wall/filled-cap error and fresh Solid admission, complete scalar+group face-pair coverage, input immutability and original10000 individual-pair/100000 geometry-cell/1000000 UV-cell limits. Logs `/tmp/sweep-zero-phase-{native-final,brep-native-final,body}.log`.

New actual Rush examples: `corrected-frenet-affine-zero-curvature-hollow-body-boundary.r` and `arc-length-corrected-frenet-affine-zero-curvature-hollow-body-boundary.r`. Their original cubic has first three poles on X and its final pole[3,1,0], outer/hole profiles in YZ, axes[1.25,.75,1] and center[.002,-.003,0]. Existing positive examples are preserved; new fixtures retain maxDeviation.1mm, initial3/max65, length tolerance.001 and original cap correction limits. Public graph guards verify the advertised actual affine/center laws. Nine serial public WASM/Rush/frame/body/embedding/filled-domain files pass107/107 in95.12s, including the published count-stable center-law regression. Vue/MCP typecheck, Vite, browser syntax and diff checks pass. Log `/tmp/sweep-zero-phase-public.log`.

Headless Apple/metal-3 UI at1440/600 passes72 assertions with no page errors and actual source/packed/public/dist verification: parameter15+15, arc_length21+21. Reports `ui-zero-principal-phase-parameter-current/matrix.json` and `ui-zero-principal-phase-arc-current/matrix.json`; logs `/tmp/sweep-zero-phase-ui-{parameter,arc}.log`. They cover real Solid publication, strict/invalid-input refusal, material/viewport, cancellation, source replacement and restoration. The arc report adds six authenticated unpaused native interruptions (build/Solid cancellation/Solid source replacement at each width), fresh-after-hash CPU profiles and Worker destruction. Timings are measured observations, not a universal latency guarantee.

Independent STEP expands82->84 with the two new cases. Export, converged rational-boundary reference, exact polynomial generator reference and OCCT import/geometry/topology/volume validation all ran sequentially and completed terminal0.84/84 pass with the current package identity and original maxPairs10000 throughout. Evidence `external-step-zero-principal-phase-current/opencascade-sweep.json`; logs `/tmp/sweep-zero-phase-step-{export,reference,generator,occt}.log`. Rational boundary quadrature is a converged numerical reference, not an interval oracle. Olderb67abc1e and647825 evidence retains its original identity; none is relabelled as current.

The original six-part goal stays active. Source correspondence for fully collinear higher-degree paths and initially straight/multispan/unclamped sources still needs exact geometry plus whole-source directional regularity; arbitrary rational collinear poles must not be promoted to a monotone two-pole arc-length reference without that proof. General spatial/closed corrected and RMF continuous/frame guarantees, applicable retained G1/G2 and multispan/closed/profile/cap joins, all-mode global regularity/intersections/holes/nesting/orientation and unqualified STEP/UI combinations remain open. Sharp miter remains explicit C0. This is progress toward the full contract, not all-mode completion.


## 2026-10-06: exact forward collinear sources, constant-pose work reuse — qualified

Current public WASM SHA256 `29dd6b7d01046be7d89dcc41f7414db2dadf2336e4e018a2f5bf7dbd6faec348`, 11384126 bytes. Publisher `/tmp/sweep-source-line-pose-publish.log` completed terminal0, including optimization, packing and public publication; Vite build completed terminal0. This replaces the interim cffa64a3 package whose parameter examples honestly failed the shared error budget. Earlier packages/reports retain their own identities.

Rust `progressive_sweep/source_line.rs` proves exact original collinearity from all three projected orient2d predicates on original binary64 poles, then whole-source strictly forward motion. Positive weights and clamped endpoints are required. A single rational Bezier with strictly ordered scalar poles uses the positive Bernstein pair expansion of X'W-XW'; this proves interior and endpoint speed sign using exact order comparisons. Other eligible clamped sources use a complete adaptive scalar derivative cover. Stops, reversals, bent coefficients, exhausted and one-under budgets refuse. Collinearity alone never establishes normalized arc correspondence.

Private corrected/RMF collinear modes use the proved constant tangent direction. Parameter translation remains the original rational curve on its original domain; normalized arc reference is the endpoint line only after whole forward-motion proof. Actual constructor endpoint displacement owns floating arithmetic. Original cap-domain projection uses the same fresh line proof. Exact constant scale/twist/affine/center coefficients allow one charged original relative-pose enclosure per profile; every endpoint and derivative interval still certifies original translation. Variable laws retain full original-law certification. No budget reset or increase: the eight-profile regression qualifies at the original 10000 shared error cells and 0.01mm target. Geometry and admission remain Rust; TS changes are fixtures/transport checks.

Final native progressive_sweep153/153 (`/tmp/sweep-source-line-pose-native.log`), BRep789 passed/2 existing ignored (`/tmp/sweep-source-line-pose-brep.log`), published public108/108 in9 files (`/tmp/sweep-source-line-pose-public.log`); Vue typecheck, browser syntax and scoped diff checks pass. Independent native polynomial/rational quotient references cover parameter and arc_length, corrected/RMF, affine/center plus variable scale/twist, original domain[2,5], dense retained displacement and source immutability. Full shared eight-profile work regression passes all four mode combinations.

Four actual Rush sources qualify full wall/filled-cap/hole boundary error and fresh Solid admission: corrected-frenet-rational-straight-affine-hollow-body-boundary.r, arc-length-corrected-frenet-rational-straight-affine-hollow-body-boundary.r, rmf-rational-straight-affine-hollow-body-boundary.r, arc-length-rmf-rational-straight-affine-hollow-body-boundary.r. They use degree3 positive rational Z poles[0,1,7,10], weights[1,2,2,1], domain[2,5], seed[1,0,1], axes[2,3,1], center[.125,-.25,0], outer2x2/hole1x1, exact prism volume180mm3 and original source bounds. Public tests verify source arguments, retained bounds, complete individual+group face-pair partition and immutable source.

STEP matrix84->88: export, converged rational-boundary reference, independent generator reference and OCCT import/geometry/topology/volume checks ran sequentially, all terminal0.88/88 pass in `external-step-rational-line-current/opencascade-sweep.json`; all cases retain native maxPairs10000. All four new expected volumes remain180mm3. Logs `/tmp/sweep-source-line-pose-step-{export,reference,generator,occt}.log`. Numerical quadrature convergence is not an interval-certified volume oracle; finite STEP agreement does not imply universal continuous guarantees.

Headless Metal UI144 checks pass at widths1440/600: `ui-rational-line-corrected-parameter-current`30, `ui-rational-line-corrected-arc-current`42, `ui-rational-line-rmf-parameter-current`30, `ui-rational-line-rmf-arc-current`42. Every report verifies the current source/packed/public/dist identity and actual Apple/metal-3 adapter, with no page errors. Covers Solid success, invalid/limited-source refusal, held-dispatch cancellation, source change and restoration. Both arc modes additionally prove12 actual unpaused native interruptions total (buildcancel, Solidcancel, Solid-source-change at each width/mode): authenticated current-module abi_request CPU samples, fresh native tails <=5ms, >=5 native samples, no debugger pause, actual Worker termination and restoration. Worker destruction observations include UI scheduling; no universal latency theorem. No foreground window or focus operation used.

The complete six-part goal remains active. Still unproved: general spatial/closed corrected/RMF original continuous/frame correspondence; full applicable retained G1/G2 including multispan/closed/profile/cap joins; all-mode regularity/contact/holes/nesting/material orientation; remaining independently qualified STEP/UI combinations. Higher-degree line-image admission does not by itself certify positional C2 or retained seam smoothness. Sharp miter remains explicit C0.


## 2026-10-06: forward line-image original-frame C1/C2 — qualified

Current public WASM SHA256 `f184db1246cd8148714c2945f1a8a97484fb19a6c1ebfd46194aa2d0295949fd`, 11384981 bytes. Publisher `/tmp/sweep-line-frame-c2-publish.log` completed terminal0 through optimized pack/public publication; Vite and Vue typecheck completed terminal0. Native progressive_sweep155/155 (`/tmp/sweep-line-frame-c2-full-native.log`), BRep789 passed/2 existing ignored (`/tmp/sweep-line-frame-c2-brep.log`), public109/109 in9 files (`/tmp/sweep-line-frame-c2-public.log`). Dedicated public original-frame tests9/9 (`/tmp/sweep-line-frame-c2-public-frame.log`). Scoped diff checks pass.

Rust original_smoothness uses fresh exact line-image/whole strict forward-motion proof for open corrected-Frenet/RMF sources. Constant tangent orientation extends across source speed knots; smooth twist supplies C1/C2 frame rotation. The endpoint line is used only to prove the constant frame direction, not to replace original parameter translation or to assert positional C2. Failed line work remains charged before the original curved-frame fallback. Conservatively all line proof work is charged to both geometry-cell and exact-work limits, preventing zero/partial exact budgets from silently publishing a frame certificate. Existing two-pole structural line cases retain their previous theorem and budgets.

Native/public tests cover rational cubic source domain[2,5], nonperpendicular seed[1,0,1], moving twist, and a connected degree-one two-span path whose speed changes at its interior knot. Both requested orders1/2, corrected/RMF and parameter/arc_length qualify. Zero/one-under cell and exact-work budgets refuse; backward tangent construction refuses; public inputs remain immutable. Separate native validation regression confirms disconnected full-multiplicity curves are rejected before line admission, while a connected positive-weight rational two-span source qualifies. This confirms an existing Curve validation premise; it is not a new generic disconnected-curve implementation.

WASM standalone inspection and actual progressive preview preserve `scope:open-original-frame-only`, with retainedSeamsCertified/profileJoinsCertified/capJoinsCertified/continuousBound/solidCertified all false in the source-frame report. The new boolean cannot promote retained G1/G2, position C2, boundary error or Solid; their independent reports continue to own those guarantees.

Fresh headless Metal UI `ui-forward-line-frame-c2-current/matrix.json` passes30 checks at widths1440/600 for corrected rational straight affine hollow body: preview/Solid success, refusal, lifecycle cancellation/source change/restoration, no page errors; source/packed/public/dist identity and actual Apple/metal-3 verified. These held-dispatch cancellation checks do not establish mid-kernel latency. Full prior STEP88/88 and UI144 (including12 actual unpaused native interruptions) remain bound to29dd6b7d and their original reports; they are not relabelled as f184db12 qualification. No focus/window operations were used.

The complete goal remains active: general spatial and closed corrected/RMF original continuous/frame correspondence; applicable retained G1/G2 including multispan/closed/profile/cap joins; all-mode regularity/contact/holes/nesting/material orientation; remaining independently qualified STEP/UI combinations. Source-frame C2 is one prerequisite, not completion of retained surface smoothness. Sharp miter remains explicit C0.


## 2026-10-06: closed original path-frame C1/C2 — native qualification, transport pending

New Rust `ClosedPathFrameSmoothnessReport` and Sweep/MultiSweep `certify_closed_path_frame_smoothness` qualify sufficient closed original frame C1/C2. Endpoint jets now support exact Cartesian recurrence through orders3/4, without incorrectly reusing the homogeneous order1/2 shortcut. Exact original C(k+1) path seam/internal knots and Ck twist seam/internal knots are combined with a complete nonzero-speed/nonparallel-seed frame cover. Original planar source proof is additionally required for RMF/corrected constant-phase transport; FixedNormal supports regular spatial paths under its whole-frame cover. Closed planar ideal transport has no residual holonomy; this does not certify constructor numerical closure correction.

Native full progressive_sweep157/157 passes (`/tmp/sweep-closed-path-frame-full-native.log`), followed by expanded moving-twist and rational-path regression (`/tmp/sweep-closed-path-frame-moving-native.log`, `/tmp/sweep-closed-path-frame-rational-native.log`, each terminal0). Exact high-order endpoint regression and all7 endpoint-related tests pass (`/tmp/sweep-closed-endpoint-high-{native,all}.log`). Source paths include regular polynomial degree7 and positive rational degree9, original domain[2,5], moving degree5 twist on domain[7,9], orders1/2, FixedNormal/RMF/corrected, parameter/arc_length. Exact one-bit path C3 or twist C2 damage preserves the applicable C1 result but refuses C2. Zero/one-under cell and exact-work budgets refuse. Spatial perturbations qualify FixedNormal and refuse RMF/corrected without an exact plane proof.

This is a Rust original-frame-only theorem. Periodic-flag sources, guides/authored overrides, unproved plane/endpoint/regularity premises refuse. No retained seam, profile/cap join, continuousBound, material embedding or Solid promotion follows. WASM/Rush/preview transport for this new report remains pending. Current published f184db12 package and its existing qualification retain their identities; no native-only result is relabelled as public proof. The full six-part goal stays active.


## 2026-10-06: closed path-frame native → WASM → Rush → viewport — qualified

Current public WASM SHA256 `d408cb84282b1b926a97e7faf864138d5685e19f16e05772f7cf7cee1886a39b`, 11388738 bytes. Publisher `/tmp/sweep-closed-path-transport-publish.log` completed terminal0, including optimization, packing and public publication. Vite, Vue typecheck, browser syntax and scoped diff checks pass. Native progressive_sweep158/158 (`/tmp/sweep-closed-path-transport-final-native.log`); separate new dispatcher test passes (`/tmp/sweep-closed-path-transport-api.log`). Public124/124 in10 files (`/tmp/sweep-closed-path-public-full.log`); focused original-frame/viewport24/24 (`/tmp/sweep-closed-path-public-focused.log`).

New Rust transport op `surface_progressive_sweep_closed_path_frame_smoothness` calls the shared original path-frame theorem. The TS inspector only transports typed input/output, and progressive preview attaches the report for closed path-owned RMF/FixedNormal/corrected frames without authored/guide overrides. Existing authored report/op/scope remain compatible. New method/scope pair is `exact-original-path-twist-endpoint-jets-and-frame-cover` / `closed-original-path-frame-only`. All pathSeamCertified/retainedSeamsCertified/profileJoinsCertified/capJoinsCertified/continuousBound/solidCertified promotion flags remain false. UI validates the exact corresponding method/scope pair, positive bounded cell/exact work, success reason and independent false flags; mixed or promoted reports refuse. Caption now says closed original/source frame, correctly covering both authored and path-owned proofs.

Public checks exercise the degree7 closed source domain[2,5], moving degree5 twist on domain[7,9], all three path frame modes, parameter/arc_length, exact C3 one-bit damage, zero/one-under budgets, immutable source and actual preview report. Closed retained preview requires at least four sections; the fixture uses initial5/max17. Native degree9 positive rational/spatial/moving-twist cases retain their prior dedicated qualification; current public test is not described as exhaustive rational-mode coverage.

Actual Rush example `examples/rush/closed-planar-path-frame-c2-progressive-sweep.r` qualifies source-frame C2 separately from retained patch regularity/error. Fresh headless Metal UI `ui-closed-path-frame-c2-current/matrix.json` passes32 checks at widths1440/600, including native-proved C2 presentation, single-bit source C3 refusal and restoration, surface-only Solid refusal, lifecycle cancellation/source change/restoration. Existing authored Cartesian mode passes30 compatibility checks in `ui-closed-path-author-compat-current/matrix.json`, including source-bit refusal/restoration. Both reports verify current source/packed/public/dist identity, actual Apple/metal-3 and no page errors. These are held-dispatch lifecycle checks, not new running-native cancellation/latency proof. No focus/window operations used.

The milestone completes public transport for this sufficient closed original-frame theorem, not retained G1/G2 or general spatial RMF/corrected transport/error. Periodic-flag sources, guide overrides and weaker geometric/reparameterized seam conditions remain pending. Prior STEP88/88 and UI144/12 unpaused native interruptions stay bound to29dd6b7d; f184db12 UI/report evidence retains its identity. No older evidence is relabelled as current d408cb84 STEP or native-running cancellation qualification. The complete six-part goal remains active.


## 2026-10-06: closed guided original-frame transport and UI

Rust `certify_closed_guided_frame_smoothness` now owns exact path C(k+1), guide Ck and twist Ck endpoint/internal-knot jets plus a whole joint nondegenerate frame cover. Independent arc-length parameterization additionally owns positive-speed covers for both original curves and a Cartesian product frame cover. Shared cells/exact-work budgets include partial failed work; zero/one-under budgets refuse. Tests cover C1/C2, polynomial closed sources on distinct physical domains, moving twist, projection collapse and one-bit guide C2 damage retaining applicable C1. Constant zero-extent guides refuse instead of receiving a certificate. Native progressive_sweep160/160 and dispatcher regression pass (`/tmp/sweep-closed-guided-final-native.log`, `/tmp/sweep-closed-guided-api.log`).

Published WASM SHA256 `1234fa1bd95e685092f25be06d6f668055d0cfafe1b0b4e5bde9c806a74aed51`, 11392182 bytes; publisher completed terminal0 (`/tmp/sweep-closed-guided-publish.log`). New op `surface_progressive_sweep_closed_guided_frame_smoothness` and typed inspector attach an independent closed-frame report to guided preview. Method/scope `exact-original-guided-endpoint-jets-and-joint-frame-cover` / `closed-original-guided-frame-only`. All retained/path/profile/cap/error/Solid promotion flags remain false. Vue/MCP types, Vite and viewport14/14 pass. Ten-file public WASM regression is still running at this checkpoint; it is not counted as passed.

Rush `closed-planar-guided-frame-c2-progressive-sweep.r` and headless actual Metal UI pass28 checks at1440/600 (`ui-closed-guided-frame-c2-current/matrix.json`): C2 presentation, exact guide seam damage refusal/restoration, held-dispatch lifecycle cancellation/source change/restoration and surface-only Solid refusal. Source/packed/public/dist identity is verified and page errors are empty. Original continuousBound remains explicitly unproved; the browser script now asserts that independent limitation instead of assuming every closed example proves it. These are not running-native latency tests.

Full six-part completion remains unproved. Retained guided G1/G2, numerical closure correction/error, general spatial transport, all-mode topology/global guarantees and full independent STEP/UI combinations remain open. Prior STEP88 and actual interruption evidence retain their original29dd6b7d package identity.


## 2026-10-06: public regression closed; closed guided arc error native-qualified

The `1234fa1b` published package passes157/157 across10 files (`/tmp/sweep-closed-guided-public-final.log`, terminal0). Initial regression correctly exposed two stale closed-RMF assertions: strict-budget refusal may report sampled refinement before continuous proof; grouped original-hull certificates now qualify the closed body within default individual-pair limits. Updated tests prove complete individual+grouped pair count, positive grouped coverage, default-budget admission and refusal with positive minimal pair/cell budgets. Focused3/3 and complete157/157 pass; no runtime admission was relaxed.

New unpublished Rust closed guided arc-length error proof now admits nonperiodic closed path/guide because guided frames bypass RMF holonomy correction. The retained copied closing station is enclosed against original source values, never assumed exact. Both original path and guide inverse-length brackets are narrowed using independently proved physical-speed lower bounds and measured length residuals. All speed work is charged to the shared cell owner. Polynomial degree7 path domain[2,5], translated guide domain[-3,7], and rational circular closed sources qualify continuous retained-patch error at33 sections within2mm and10000cells; one-under budget refuses. Full native progressive_sweep162/162 (`/tmp/sweep-closed-guided-arc-full-native.log`) also retains open guided/contact shared-budget coverage.

Publisher `/tmp/sweep-closed-guided-arc-publish.log` is running; public WASM/UI evidence above remains bound to1234fa1b and does not prove this new native error route. No full-body filled-cap, correction, global embedding, retained smoothness or all-mode claims follow from these selected closed guided bounds. The full six-part goal remains active.


## 2026-10-06: closed guided arc-length public bound and Solid UI qualified

Current public WASM SHA256 `cd5f755be99efb84b53ff77f3cbf93e2f10d2c42bf8bc76725c8bdb67494517f`, 11392984bytes; publisher terminal0 (`/tmp/sweep-closed-guided-arc-publish.log`). Full public158/158 across10 files (`/tmp/sweep-closed-guided-arc-public-full.log`); focused38/38 (`/tmp/sweep-closed-guided-arc-public.log`). Added both-parameter closed guided tube material tests pass12/12 in their containing file (`/tmp/sweep-closed-guided-body-public.log`), including exact total pair coverage and default native Solid budgets, shell roles and minimal positive budget refusal. Vite, Vue types, browser syntax and scoped diff checks pass. Native162/162 remains the matching original-source qualification.

`closed-arc-length-guided-frame-c2-progressive-sweep.r` passes32 headless Metal UI checks at1440/600 (`ui-closed-guided-arc-frame-c2-current/matrix.json`): source C2 and continuous retained error presentation, exact guide-bit C2 refusal/restoration, surface-only Solid refusal and held-dispatch lifecycle. `closed-arc-length-guided-tube-body-boundary.r` passes30 at1440/600 (`ui-closed-guided-arc-body-current/matrix.json`): actual successful Solid, WebGPU rendering, source change/refusal, held cancellation and recovery/persistence. Actual body has256faces/1shell and identical geometry hash `478a96aede1f6069aa2e193cbe4526c5e6549f349f441f0abc117a6b000bbd16` at both widths. Both matrix reports verify source/packed/public/dist identity and Apple/metal-3; pageErrors empty. No foreground/focus operations used. These are not fresh unpaused native interruption latency measurements.

Independent STEP matrix adds ordinary-parameter and arc-length closed guided tubes. A33-section tube retained bound measured2.963494910472693mm parameter /2.0406155358699842mm arc_length, so the fixture correctly refuses2mm. It refines17→33→65 while retaining the original2mm budget; no certificate bypass or tolerance relaxation. Export `/tmp/sweep-closed-guided-step-export.log` is currently running (session32606), not yet an independent STEP pass. Prior88-case OCCT evidence retains29dd6b7d identity.

Closed guided retained-bound coverage does not imply retained G1/G2, caps (these closed cases have none), general spatial RMF/corrected transport, all-mode material guarantees or full six-part completion. The full goal remains active.


## 2026-10-06: independent STEP90/90 on current closed-guided package

Sequential export, rational-boundary Gauss reference, generator-volume reference and OpenCascade verifier all completed terminal0. `external-step-closed-guided-current/opencascade-sweep.json` passes90/90 with verified public/packed WASM `cd5f755be99efb84b53ff77f3cbf93e2f10d2c42bf8bc76725c8bdb67494517f`,11392984bytes. Logs `/tmp/sweep-closed-guided-step-{export,rational-reference,generator-reference,occt}.log`.

New ordinary-parameter and arc-length guided torus cases each have256faces/1solid/1shell, positive native volume admission, valid closed/manifold imported topology and shell orientation. Full-domain coefficient/shared-basis wall distance and edge distance gates pass, alongside pcurve/ownership/material-side fixture checks. Ordinary reference volume3.153169678135626 versus OCCT3.153169678874247mm³; arc reference3.1532024060224626 versus OCCT3.1532024067610993mm³. Relative discrepancies approximately2.3425e-10. Reference quadrature converges independently on original binary64 retained rational poles; this is numerical validation, not an interval volume theorem or universal global embedding proof. Final65-section retained bounds are0.1562619912209298mm parameter /0.6802673103900417mm arc_length, both below the unchanged2mm tolerance and within shared10000cells.

This supersedes the previous running-export status for the named current matrix only. Earlier88-case packages retain their provenance. Full six-part completion remains unproved: general spatial RMF/corrected reference error/correction, weaker reparameterized moving-frame seam premises, applicable retained G1/G2, all-mode global and full UI/STEP combinations still require work.


## 2026-10-06: exact direction-jet frame fallback — native qualified

New cad-predicates `rational_bspline_direction_jet_identity` proves C1/C2 of the unit direction chart at original internal knots or active endpoints. Formal original Cartesian source jets are compared without rounded derivatives or square roots. For arc length, positive endpoint speed ratio owns first chart jets; second jets compare r_tt/Q-r_t(v·a)/Q² with Q=v·v. A common nonzero positively oriented velocity component owns the chart. Endpoint/internal direction claims do not establish path position continuity, whole-domain regularity, retained joins or holonomy. Exact zero/common-denominator rational simplification prevents redundant denominator growth without dropping budget work or precision refusal.

All34 cad-predicates tests pass (`/tmp/sweep-direction-jets-all-native.log`). Regression separates a constant direction across speed jumps from reversal/stops and one-under exact-work budgets. A closed positive rational conic with binary64 weight0.5 proves C1 arc direction but refuses C2 because its second arc jets differ; this is not mislabeled an exact circle.

Closed path-owned source-frame theorem now retains the sufficient Cartesian route and charges failed work before the exact direction fallback, under the same maxExactWork owner. Twist endpoint/internal jets, source plane where required, and complete speed/nonparallel-seed cover remain mandatory. The closed conic qualifies original frame C1 in parameter/arc spacing and refuses C2/one-under work. All163 progressive_sweep tests pass (`/tmp/sweep-closed-direction-full-native.log`), including existing closed path/guided compatibility. Guided frame theorem retains its stronger position/guide jets: direction smoothness alone cannot replace their positional premises.

Publisher `/tmp/sweep-closed-direction-publish.log` is running. Current public cd5f755b evidence (158 public,90STEP,62 new UI) retains its original identity and does not prove this native-only fallback. Full six-part goal remains active.


## 2026-10-06: direction-chart source C1 native → WASM → Rush → viewport qualified

Current WASM SHA256 `168274249a724a91ae45c3c015d637a8e26c30446ec243b81c8e9d701380ee33`,11409449bytes; publisher terminal0 (`/tmp/sweep-closed-direction-publish.log`). All162 public tests pass in10 files (`/tmp/sweep-closed-direction-public-full.log`); viewport15/15, Vue/MCP types, Vite, browser syntax and scoped diff checks pass. Matching native163 progressive_sweep and34 cad-predicates retain their prior logs.

Public test proves closed positive rational conic original direction C1 for parameter/arc-length spacing, refuses C2 and one-under work, and preserves immutable source input. Typed transport requests a separate order1 report when the closed path-owned order2 report refuses. `closedSourceFrameSmoothnessC1` is an actual native certificate, never a relabeled order2 report. Viewport independently checks requested order1, corresponding native method/scope, bounded positive cells/work, null success reason and all retained/path/profile/cap/error/Solid flags false. Wrong-order/scope/promoted reports refuse. Product caption says C1 closed original/source frame separately from C2 and retained error.

Rush `closed-conic-direction-frame-c1-progressive-sweep.r` passes28 headless actual Metal checks at1440/600 (`ui-closed-direction-c1-current/matrix.json`): C1 certified/C2 unproved, exact one-bit direction seam refusal/restoration, retained error explicitly unproved, surface-only Solid refusal and held-dispatch lifecycle. Source/packed/public/dist identity matches16827424, no pageErrors and no focus/window operations. These are not new actual running-native interruption/latency measurements.

No C2 or continuousBound is fabricated for the conic fixture. Its closed planar RMF arc-length retained error still needs the separate closure/identity correspondence route; source direction smoothness cannot replace that premise. Prior STEP90 and62 UI evidence remain bound tocd5f755b, not relabeled as current16827424. Full six-part completion remains unproved and active.


## 2026-10-06: closed planar RMF arc-length retained error — native qualified

The original arc-length frame-image bound now admits closed RMF only for original axial planar sources with no guide/authored/contact overrides and constructor-owned actual frame identity. Every sampled base normal/tangent and zero holonomy correction are checked by the existing sections_with_frame_identity owner. Copied closing controls remain compared to original endpoint image and length residual, never assumed identical. Fresh speed lower bound narrows original inverse-length brackets and is charged to the same10000cell owner. Nonperiodic source, whole source/law value cover, retained basis and decomposition premises remain mandatory. General spatial/nonaxial/Corrected closed transport still refuses.

Native circular and positive rational conic closed examples qualify within2mm at33sections, with zero twist and joint full-turn twist+affine scale[2,3,1]+center[.125,-.25,0]. One-under budget and nonaxial seed refuse. The previous blanket closed-RMF refusal test now covers nonaxial unproved transport with a valid initial5sections; it previously used an invalid initial3section configuration that the newly reached constructor correctly rejected. Full progressive_sweep164/164 passes (`/tmp/sweep-closed-rmf-arc-full-native.log`), retaining existing open/reference/guided/contact coverage. New public ordinary circle/conic and joint-law test is prepared; Vue types pass.

Publisher `/tmp/sweep-closed-rmf-arc-publish.log` is running. Current16827424 public162/UI28 source-C1 evidence and priorcd5f755b STEP90 retain their existing identities; neither is relabeled as published proof of this new retained error route. Full six-part goal remains active.


## 2026-10-06: closed planar RMF arc-length bound native → WASM → Rush → Solid — qualified

Current public WASM SHA256 `9a002eaf54fd9630c203d30c988db80139e7f1c835921cd3254f06273b125331`,11409609bytes; publisher terminal0 (`/tmp/sweep-closed-rmf-arc-publish.log`). Focused45/45 public tests pass (`/tmp/sweep-closed-rmf-arc-public.log`) for circle/conic source, separate C1/C2/frame scope, full-turn+affine+center laws and both native RMF tube material cases under default budgets. Matching native164/164 retains its log. Types, Vite, browser syntax and scoped diff checks pass. Full10-file public regression is still running (`/tmp/sweep-closed-rmf-arc-public-full.log`, session1009), not yet counted as complete.

Headless actual Metal UI passes32 at1440/600 (`ui-closed-rmf-arc-conic-current/matrix.json`): conic source C1 certified/C2 unproved now has certified retained continuous error≤2mm, exact source-bit C1 refusal/restoration and independent surface-only Solid refusal/lifecycle. The earlier16827424 unproved-bound presentation remains accurate only for that package. New `closed-arc-length-rmf-tube-body-boundary.r` passes30 at1440/600 (`ui-closed-rmf-arc-body-current/matrix.json`): actual successful Solid/WebGPU, persistence, source change and held cancellation/refusal/recovery. Body has64faces/1shell with matching geometry hash `807c19dba068faa562e47c30fb751a6e2161e1d12daa085ccd95c7f182d89474` at both widths. Source/packed/public/dist identity is verified, pageErrors empty, no focus/window operations. These are held-dispatch lifecycle checks, not new actual running-native latency proof.

New STEP fixtures add arc-length RMF circular and conic torus cases to the90-case matrix. Current92-case export `/tmp/sweep-closed-rmf-arc-step-export.log` is running (session45035); independent reference/OCCT stages are pending. Prior90/90 remains bound tocd5f755b. Full six-part goal, all-mode guarantees, nonaxial/general spatial/corrected closed transport/correction and applicable retained smoothness remain open.


Full public165/165 across10 files now completed terminal0 (`/tmp/sweep-closed-rmf-arc-public-full.log`), superseding the running status above. Current STEP92 export also completed terminal0; sequential rational reference stage is running. Independent OCCT qualification remains pending.


## 2026-10-06: independent closed planar RMF arc-length STEP matrix — 92/92 qualified

All 92 cases in `external-step-closed-rmf-arc-current/opencascade-sweep.json` pass independent OpenCascade import, topology, material orientation, retained geometry correspondence and reference-volume checks. Artifact provenance is the current public/packed WASM SHA256 `9a002eaf54fd9630c203d30c988db80139e7f1c835921cd3254f06273b125331`, 11409609 bytes; manifest SHA256 `cf43fffa6b757aa319900287ebc9bf05e0e9d1747e608faba9c70ce0c8f2bf09`. Export, rational-boundary reference, generator reference and OCCT stages each completed with exit0, sequentially against this manifest. Logs: `/tmp/sweep-closed-rmf-arc-step-{export,rational-reference,generator-reference,occt}.log`.

`closed-original-arc-length-rmf-circle-tube.step`: 64 faces, 1 shell(s), 1 solid(s); OCCT volume 3.0777227175932063 mm³ versus independent reference 3.0777227168722487 mm³, relative difference 2.3425031499984527e-10.

`closed-original-arc-length-rmf-conic-tube.step`: 128 faces, 1 shell(s), 1 solid(s); OCCT volume 3.0464244189379666 mm³ versus independent reference 3.0464244182243432 mm³, relative difference 2.3424949535586273e-10.

This supersedes the pending STEP status above. Current milestone also retains native164/164, public165/165 and 62 headless actual-Metal UI checks at1440/600. Reference-volume convergence is numerical validation, not an interval certificate. The 92 fixtures do not establish every possible mode or combination. Full six-part completion remains open, including nonaxial/general spatial closed transport and correction, remaining continuous-bound coverage, all-mode global guarantees and applicable retained G1/G2/seam guarantees. Newer UI cancellation cases use held dispatch; they do not replace prior actual-running native interruption measurements. No foreground/focus operations were used.


## 2026-10-06: closed fixed-normal arc-length original error — native milestone

Rust `original_frame_arc_length_section_interpolation` now admits closed nonperiodic FixedNormal transport with no authored frame, guide or contact override. The actual constructor applies no holonomy correction in this mode. The existing original fixed-normal interval value cover and comparisons against actual stored sections own the copied closing section; no zero seam displacement is assumed. A fresh source speed cover tightens inverse-length brackets and charges the same work owner. Periodic and unsupported closed transport routes remain unresolved.

Native progressive_sweep165/165 passed, exit0 (`/tmp/sweep-closed-fixed-normal-full-native.log`). New regression covers circle/conic sources, axial and nonaxial seeds, constant versus joint affine/center/full-turn twist, one-under cell-budget refusal and certified bounds exceeding a strict positive tolerance. Some nonaxial joint-law bounds exceed2mm and correctly fail tolerance admission; certification does not imply acceptance. Scoped diff check passes. Public transport regression is prepared in `tests/nurbsClosedPlanarRmfSweep.test.ts`, not yet run against the new package.

Publisher `/tmp/sweep-closed-fixed-normal-publish.log` is live (terminal session75848). The currently published9a002eaf native164/public165/UI62/STEP92 evidence retains its existing identity. New WASM/Rush/viewport/Solid/STEP/UI qualification is pending. Full six-part goal remains active.


Closed FixedNormal public qualification preparation: added Rush `closed-arc-length-fixed-normal-tube-body-boundary.r` and registered it in the existing headless body UI matrix. Existing native STEP fixture generator now includes circle and conic FixedNormal arc tubes (planned94 cases); these are prepared fixtures, not independently qualified yet. Public tests now cover both RMF and FixedNormal circle/conic material admission plus FixedNormal joint-law tolerance refusal. Vue and MCP TypeScript checks, browser syntax and scoped diff pass. Publisher session75848 remains live: release compile completed, optimizer/pack/public stages remain pending. Existing92 STEP evidence is unchanged; no94-pass claim.


## 2026-10-06: closed FixedNormal arc error public/Rush/Solid/UI — qualified

Publisher session75848 completed exit0. Current packed/public/dist WASM SHA256 `10dcb84d8bd98d30b616da3515dac37c5faeb146af52d5a6762c8e784aed3504`,11409664bytes; all three identities checked. Focused public10/10 passes (`/tmp/sweep-closed-fixed-normal-public.log`), including circle/conic bounds, joint affine/center/full-turn twist, strict tolerance refusal, closed RMF/FixedNormal material certificates and Rush strict-source refusal. Native165/165 remains valid. Vue/MCP types, Vite, browser syntax and scoped diff pass.

Headless UI `ui-closed-fixed-normal-arc-body-current/matrix.json` passes30 assertions (15 each at1440 and600) with actual Apple/metal-3 and WebGPU Solid. Both widths yield64faces/1shell and geometry SHA256 `807c19dba068faa562e47c30fb751a6e2161e1d12daa085ccd95c7f182d89474`; pageErrors empty, persistence/source/refusal/recovery pass. Cancellation is held-dispatch lifecycle coverage, not actual mid-native latency proof. No foreground focus operations.

Broader9-file serial public regression `/tmp/sweep-closed-fixed-normal-public-full.log` is running session29009. Extended94-case STEP export `external-step-closed-fixed-normal-arc-current` is running session25516 (`/tmp/sweep-closed-fixed-normal-step-export.log`); independent rational/generator/OCCT stages pending. Prior92/92 STEP remains bound to9a002eaf, not the new10dcb84d package. Full six-part goal remains open.


## 2026-10-06: FixedNormal STEP94 complete; closed corrected planar arc error native milestone

Current `external-step-closed-fixed-normal-arc-current/opencascade-sweep.json` passes94/94; each export, rational reference, generator reference and OCCT stage completed exit0. Artifact is10dcb84d8bd98d30b616da3515dac37c5faeb146af52d5a6762c8e784aed3504,11409664bytes,public/packed verified. New FixedNormal circle/conic tubes have64/128 faces and1 closed outward shell/solid. OCCT/reference volume relative differences are2.3425031499984527e-10 and2.3424949535586273e-10. Numerical Gauss reference convergence is validation, not an interval certificate. Logs `/tmp/sweep-closed-fixed-normal-step-{export,rational-reference,generator-reference,occt}.log`. Broader serial public9-file regression105/105 also completed exit0, superseding running status above.

New Rust original-frame arc error admits closed nonperiodic axial-original-plane CorrectedFrenet with no authored frame/guide/contact overrides. This constructor mode has no RMF holonomy correction. Existing original planar reference interval bounds, exact initial principal-phase proof for anisotropic affine laws, actual stored-section displacement and copied closing endpoint ownership remain required. The certificate does not assume principal-sign or seam displacement exact. Nonaxial closed and parameter-spacing corrected routes still refuse.

New native regression covers circle/conic, joint affine/center/full-turn twist, actual preview continuousBound/admission, strict positive tolerance refusal, one-under proof-work refusal and nonaxial/parameter refusal. Full progressive_sweep166/166 passes (`/tmp/sweep-closed-corrected-arc-full-native.log`). Types/scoped diff pass. Prepared public corrected regression is unqualified until new publication. Publisher `/tmp/sweep-closed-corrected-arc-publish.log` is running; new WASM/Rush/viewport/Solid/STEP/UI stages pending. Completed94 STEP/30 UI/public105 retain10dcb84d provenance, not evidence for unpublished corrected changes. Full six-part goal remains active.


## 2026-10-06: closed corrected planar arc native→WASM→Rush→Solid/UI milestone

Publisher session17540 completed exit0. Packed/public/dist all match WASM891f8eaebf1d90826c87ac3abe17af89e3f415223601b10145f5f5e6426c8acf,11409781bytes. Focused public13/13 passes (`/tmp/sweep-closed-corrected-arc-public-body.log`), including circle/conic native material admission, affine/center/full-turn original bounds and strict refusal. Native166/166 remains bound to this source milestone. Vue types, Vite and scoped diff pass.

Rush `closed-arc-length-corrected-planar-tube-body-boundary.r` headless actual Apple/metal-3 UI passes30 assertions,15 each at1440/600 (`ui-closed-corrected-arc-body-current/matrix.json`). Actual WebGPU Solid has64faces/1shell and geometryhashb5356863dfc40a232b086575013889ea05e4c89ce05dab4b42c6f3fadf1d43a1 at both widths. pageErrors empty. Successful publication, persistence, refusal, source changes, held-dispatch cancellation and restoration pass; this run does not prove native running cancellation latency.

Additional authenticated running-native cancellation UI is live session22750 (`/tmp/sweep-closed-corrected-arc-running-cancel-ui.log`), not yet qualified. Nine-file serial public regression session82513 is live (`/tmp/sweep-closed-corrected-arc-public-full.log`). New96-case STEP export session32030 is live (`/tmp/sweep-closed-corrected-arc-step-export.log`); references/OCCT pending. Prior94/94 retains10dcb84d provenance. Spatial/nonaxial closed reference/correction, all-mode global guarantees and applicable retained G1/G2/seams remain open; full goal is unchanged.


## 2026-10-06: corrected closed actual-running native UI; nonaxial original-plane error progress

`ui-closed-corrected-arc-running-cancel-current/matrix.json` completed and passed42 checks (21 at1440,21 at600), bound to891f8eaebf1d90826c87ac3abe17af89e3f415223601b10145f5f5e6426c8acf. Actual native abi_request CPU sampling, with no Debugger pause, authenticates build cancellation, Solid cancellation and source-change termination at both widths. Native samples11–18 per observation, profile-tail age0.084–1.492ms. terminate() called1–33ms after cancellation initiation; worker destruction confirmed1243–1666ms later. Restoration passes, stale Solid prevented. These measured scheduling/destruction costs are not cooperative Rust polling latency. pageErrors empty. Broader public9-file108/108 passed on this package.

96-case corrected STEP export and rational-boundary reference completed exit0. Generator reference is running (`/tmp/sweep-closed-corrected-arc-step-generator-reference.log`); OCCT remains pending. Existing94 independent cases retain their previous package identity.

Current new Rust edit removes the axial-only prerequisite for closed CorrectedPlanar arc transport while retaining a fresh exact original-coefficient source-plane proof for nonaxial input, charged to the same owner before initial phase and original frame intervals. No-plane, one-bit off-plane, periodic, parameter-spacing and override cases stay unproved. Test `closed_nonaxial_corrected_arc_owns_exact_original_plane` passes for closed transformed circle/conic on plane x+y=0 with anisotropic affine/center/full-turn laws, positive bound≤4mm, one-under work refusal and one-bit source-plane mutation refusal. Full native regression running (`/tmp/sweep-closed-nonaxial-corrected-full-native.log`); new nonaxial code is not yet published or qualified through public/Rush/STEP/UI. Scoped diff check passes. Full spatial RMF/correction, all-mode global guarantees and retained G1/G2/seams remain open. Full goal unchanged; Git integration remains stopped at reviewed conflicts.


## 2026-10-06: nonaxial closed corrected original bounds revalidated on current package

Temporary old process handles/logs were missing; revalidated current source rather than assuming prior completion. Current package identity isacaaf4989b0641e665331bbb79778ce639e7755d6070c185e6e36305e3059794,11537829bytes; prior891f8e evidence retains its previous identity. Native progressive_sweep167/167 passed in persistent `nonaxial-corrected-revalidation/native.log`. New focused public nonaxial original-plane test passes, including circle/conic full-turn twist plus anisotropic axes/center, positive bound≤4mm,≤10000 certificate cells, input immutability and one-bit off-plane failure to prove continuousBound. New Rush `closed-nonaxial-arc-length-corrected-tube-body-boundary.r` builds; the two focused nonaxial/Rush cases pass in `nonaxial-corrected-revalidation/nonaxial-rush-public.log`. Vue types and scoped diff pass.

Initial public mutation fixture used a larger coordinate change and failed earlier at tangent discontinuity. Corrected one-bit mutation is now represented explicitly at a nonzero pole; it produces unproved continuousBound, not constructor failure. Historical failed public logs are retained; no full15-test pass claimed. Rush build alone does not establish global material Solid admission or UI success. Both remain next work for this new nonaxial tube. Full spatial RMF/correction, all-mode global guarantees, retained G1/G2/seams and combined miter-law matrix remain open.96-case STEP still lacks OCCT report; old artifact scope is not promoted.


## 2026-10-06: nonaxial closed corrected Solid and actual Metal UI qualified

Current packageacaaf4989b0641e665331bbb79778ce639e7755d6070c185e6e36305e3059794,11537829bytes. Four focused public nonaxial/Rush/material cases pass (`nonaxial-corrected-revalidation/material-public.log`): circle/conic boundary error≤2mm, retained walls, closed cap-free ownership, all faces injective, all contact pairs classified, individual+grouped pair counts equal total pairs, consistent shell roles and default Solid admission. maxCells1/maxPairs1 refuses Solid. Existing native167/167 remains current source evidence. Vue types, browser syntax and scoped diff pass.

Actual headless Apple/metal-3 UI passes30 assertions,15 each at1440/600 (`ui-closed-nonaxial-corrected-body-current/matrix.json`). Solid WebGPU has64faces/1shell andgeometryhashe4c3fa4d5545af40dc6338b3d0f989f867f557a1ad54385a25a69f8055a093df at both widths. pageErrors empty; publication, refusal, source-change, persistence, held-dispatch cancel and recovery pass. This does not extend prior actual-running cancellation latency proof to this new case. Initial UI attempt exposed missing nurbs_curve negative-path fixture routing; generalized body negative fixture now replaces original NURBS path with zero-length line, preserving refusal assertions, and full rerun passes.

Two new nonaxial circle/conic fixtures extend planned STEP matrix to98. Export session55211 is live (`nonaxial-corrected-revalidation/step-export.log`), references/OCCT pending. Full17-case public file regression session67086 is live (`nonaxial-corrected-revalidation/public-full.log`), no full pass yet. Source file `closed-nonaxial-arc-length-corrected-tube-body-boundary.r` now owns qualified UI example. General spatial RMF/correction, all-mode global guarantees, retained G1/G2/seams and combined miter-law matrix remain open; finite fixtures are not universal proof.

Full public17/17 now passed exit0, superseding running status above; persistent public-full.log records21.40s. STEP98 export remains pending.


## 2026-10-06: nonaxial closed planar RMF original error progress; STEP98 references

STEP98 export completed; fresh rational-boundary reference completed exit0 using restored persistent Python runtime `/Users/themoretheless/.cache/sweep-occt-qualification` (numpy2.5.3,cadquery-ocp8.0.1.1.0). Original temporary venv was absent; initial command exit127 is retained in history, not counted as success. Generator reference running (`nonaxial-corrected-revalidation/step-generator-reference.log`), OCCT pending. All98 artifacts retainacaaf498 package provenance.

New Rust closed RmfPlanar arc route admits nonaxial exact original source planes with original seed normal perpendicular to that plane. The fresh source-plane proof is charged before ideal Bishop normal/value intervals. Numerical constructor reflections/holonomy correction are not assumed identical to the ideal field: actual stored section poles and copied endpoint are directly compared against the original interval image. Thus the error route no longer requires constructor bitwise closed-frame identity. That flag remains separate diagnostic evidence. This does not admit inclined seed normals outside the source-plane normal, nor general spatial Bishop/holonomy transport.

New native `closed_nonaxial_rmf_arc_charges_actual_holonomy_and_original_plane` passes circle/conic, anisotropic axes/center/full-turn twist, bound≤4mm, no bitwise identity, one-under work refusal and one-bit exact-plane damage refusal (`nonaxial-corrected-revalidation/rmf-native.log`). No measured nonzero holonomy claim is made. Initial full regression167pass/1fail exposed legacy blanket closed refusal on a fixture with unmatched closing laws; replaced that negative fixture with non-plane seed normal, preserving original plane-refusal assertion. Full168-case repeat running (`rmf-full-native.log`), public/package/Rush/Solid/UI qualification still pending. Scope remains the full goal including spatial transport, all-mode global correctness, retained smoothness and joint miter laws.


## 2026-10-06: nonaxial closed planar RMF native→WASM→Rush→Solid and running UI qualified

Current packed/public/dist WASM66b9067e540d1df5bf49346d9d6d10677e8b0a78981205214b1c2a6f23ce42dd,11533848bytes, identities verified. Publisher completed (old terminal handle expired; persistent log and output identity verified). Native168/168 passed. Public full constructor/material file20/20 passes (`nonaxial-corrected-revalidation/rmf-public.log`): nonaxial circle/conic, joint axes/center/full-turn twist, exact-plane mutation refusal, immutable source and both RMF tube default native volume admissions, allface injectivity/contact classification/roles and exhausted-budget refusal. Vue/Vite/browser syntax/scoped diff pass. General inclined seed and spatial RMF remain unproved.

Headless actual Apple/metal-3 UI `ui-closed-nonaxial-rmf-running-current/matrix.json` passes42 (21 at1440,21 at600). Successful WebGPU Solid has64faces/1shell, geometryhashca55edb7dd1d2b308646c16377f0494a9c37d13440cabe9dc5ea106393bffbc2 at both widths. pageErrors empty, persistence/refusal/source/restore pass. Actual unpaused native abi_request CPU probes authenticate build cancel, Solid cancel and source-change cancellation:16–19 native samples, tail0.061–0.97ms; terminate()0–22ms, destruction confirmed1424–1506ms. Scheduling/destruction measurements are not cooperative kernel-poll latency. No focus operations.

Prior independent STEP98 completed all stages and passes98/98 (`external-step-closed-nonaxial-corrected-current/opencascade-sweep.json`) with prioracaaf498 package. New100-case export session62438 is live (`rmf-step-export.log`); references/OCCT pending. Broader9-file serial public regression session19026 is live (`rmf-public-full.log`); no broad pass yet. These pending stages do not weaken the full objective or relabel old evidence. General spatial closed RMF/holonomy, all-mode global correctness, retained G1/G2/multispan/seams and full miter joint-law matrix remain open.


## 2026-10-07: nonuniform retained station G2 native progress

Broad9-file public regression115/115 completed on66b9067e package (`rmf-public-full.log`), replacing prior running status. STEP100 export, rational-boundary reference and generator reference each completed; OCCT live session35895 (`rmf-step-occt.log`), not yet qualified. RAG previous checkpoint update completed revision54.

Rust retained station seam recorder now proposes a transverse reparameterization scale from actual stored-strip derivatives instead of assuming1. Proposal is not evidence: exact original projective jet identities over the full boundary and regularity still own acceptance and work. This admits applicable G2 with nonuniform station lengths without promoting moving source-frame smoothness or approximate jet equality. The proposal is computed once and the same exact-budget owner remains.

New native `nonuniform_retained_stations_require_exact_scaled_g2_not_equal_speed` passes: actual RMF constructor on piecewise straight degree1 path with stationsz=0,1,3,7,15 produces3 regular C0-owned G2 seams; one-under budget and genuine cross-section kink refuse. Log `nonuniform-g2-native.log`. Source formatted, scoped diff clean. Full native progressive_sweep regression is running session81387 (`nonuniform-g2-full-native.log`); publication and public/Rush/viewport qualification of this new smoothness change remain pending. Existing168 native/115public/42UI/STEP98 keep prior provenance. Full spatial RMF, all-mode global validity, applicable general retained G1/G2/closed seams and combined miter matrix remain open.


## 2026-10-07: STEP100 and nonuniform G2 native regression complete

Independent STEP100 passes100/100 (`external-step-closed-nonaxial-rmf-current/opencascade-sweep.json`), bound to66b9067e540d1df5bf49346d9d6d10677e8b0a78981205214b1c2a6f23ce42dd,11533848bytes. Export, rational reference, generator reference and OCCT each completed; previous pending status superseded. Broad115/115 public remains same package evidence. Numerical volume oracle is not universal interval/embedding proof.

New nonuniform retained-G2 full native169/169 passed exit0 (`nonuniform-g2-full-native.log`,91.66s). New public `nurbsRetainedSweepSmoothness` regression prepared: G2 actual nonuniform stations, exact-work one-under refusal, moving-twist retained-G1 refusal and input immutability, with source/profile/cap/Solid promotions explicitly false. Publisher session51376 is live (`nonuniform-g2-publish.log`) compiling newer Rust source; pending package/public/Rush qualification.

Local node_modules disappeared; initial types command exit127 does not count as verification. Restored dependencies using lockfile npm ci --ignore-scripts (exit0), avoiding publisher duplication; typecheck is now running. No application windows/focus operations. Original four requirements remain open: general spatial closed RMF/holonomy, all-mode global correctness, applicable retained/moving/multispan/closed smoothness and full affine/authored/guide miter matrix.


## 2026-10-07: retained nonuniform G2 six-mode public matrix prepared

Publisher session51376 remains live in geometry-wasm release compilation, not yet packed; old66b9067e artifact identity is unchanged. Prepared public G2 matrix now includes RMF, Fixed, FixedNormal, CorrectedFrenet, authored constant axes and guided original rail, all with anisotropic affine scale and center offset. Actual nonuniform stations must satisfy full exact retained projective jets and regularity; each case checks exact-work one-under refusal and explicitly false source/Solid promotion. Frenet on this zero-curvature fixture is not applicable and not silently included as a positive case. Vue types pass. Tests await new WASM; no six-mode pass claimed.

Ollama nomic-embed-text restoration completed exit0; /api/tags confirms137M GGUF digest0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f. Dependencies restored from lockfile with install scripts disabled. All qualification source/docs preserved. STEP100 independent and native169 remain verified milestones; full spatial closed RMF, global/all-mode guarantees, applicable moving/retained/closed smoothness and full joint miter matrix remain open.


## 2026-10-07 published nonuniform retained G2 and applicable moving authored frame

The singleton publisher completed successfully, including optimization, packing and public WASM. Public and generated identity SHA-256 is `870a265406e647d60cf4b65fd6743e60b6c02eefb9fc8b36c53c4bcba80d0705`, 11,533,949 bytes. The Rust implementation proposes a positive station normal scale from actual retained strips, then independently requires exact projective strip-jet identities and whole-strip regularity under the same exact-work budget. A proposed scale alone is not a certificate.

Packaged WASM retained smoothness tests: **11/11 passed**, including nonuniform station speeds in RMF, Fixed, FixedNormal, CorrectedFrenet, authored and guided modes with affine scale/center laws. Frenet on a zero-curvature line is inapplicable and is not promoted. One-less-than-required exact-work budgets refuse. The new applicable moving authored frame has constant longitudinal Y axis and transverse normal from X to X+Z: the longitudinal profile retains a regular planar YZ image and passes G2, while a transverse-sensitive profile changes image and refuses G1 with shared C0 preserved. The native test independently verifies actual stored poles equal the expected image and passes **1/1**. This adds only a test after publication, not new release geometry.

Six-file packaged-WASM smoothness/decomposition regression: **39/39 passed** (11.16s). Vue type checking and scoped diff whitespace validation pass. Logs: `nonaxial-corrected-revalidation/nonuniform-g2-publish.log`, `nonuniform-g2-public.log`, `nonuniform-g2-regression.log`, `moving-authored-retained-g2-native.log`, `nonuniform-g2-types.log`. The previously completed full native sweep run remains **169/169** before the additional moving authored test.

Scope remains retained station seams only: source-frame, profile/decomposition joins, caps, global embedding and Solid cannot be inferred from these seam reports. General spatial closed RMF/holonomy continuous bounds, all-mode global correctness, complete applicable moving/multispan/closed retained smoothness and the full miter matrix remain open. STEP **100/100** and UI **42/42** remain explicitly tied to the older `66b9067e...` package; they have not yet been requalified on `870a2654...`. No foreground UI was used.

Published `870a2654...` body/miter regression completed successfully: **61/61**, four files (`nurbsClosedCombinedMiter`, `nurbsProgressiveBodyBoundary`, `nurbsRetainedBodyCoverage`, `nurbsClosedPlanarRmfSweep`), 76.99s. Together with the disjoint smoothness/decomposition suite, this milestone has **100/100 packaged-WASM regression tests**. This is not a STEP or visual UI result and does not certify every advertised combination. Log: `nonaxial-corrected-revalidation/nonuniform-g2-body-miter-regression.log`.


## 2026-10-07 current package UI requalification

Current package `870a265406e647d60cf4b65fd6743e60b6c02eefb9fc8b36c53c4bcba80d0705` rebuilt through Vite and independently verified at source/public/dist. Headless UI for `closed-nonaxial-arc-length-rmf-tube-body-boundary.r` passes **42/42** checks (21 at 1440 and 21 at 600). Both actual adapters report Apple / metal-3. Successful Solid has 64 faces, one shell, geometry hash `ca55edb7dd1d2b308646c16377f0494a9c37d13440cabe9dc5ea106393bffbc2`. Both pageErrors lists are empty.

Build cancellation, Solid cancellation and source changes were authenticated with 15–17 native CPU samples, unpaused ABI execution and sample tail 0.055–0.609ms. Worker terminate() was called 1–19ms after initiation; destruction 1159–1267ms includes scheduling and is not a cooperative Rust polling latency claim. Restoration and stale-result prevention passed at both widths. No window was foregrounded. Artifact: `ui-nonuniform-retained-g2-rmf-current/matrix.json`; log: `nonaxial-corrected-revalidation/nonuniform-g2-ui.log`.

The isolated locked browser test runtime was restored after a missing-package failure; source/lock contracts were unchanged. STEP100 export is currently running, not yet a completed independent OCCT result. Full all-mode/UI/global/continuousBound scope remains open.

Current `870a2654...` joint closed miter UI (`closed-miter-frame-guide-affine-hollow.r`) also passes **38/38**, 19 checks at each width 1440/600 with actual Apple/metal-3. Affine, authored frame and guide laws reach Rush, viewport continuous-boundary/profile-G2 presentation and successful Solid (128 faces, two shells, geometry SHA-256 `3f4f4ae677b20920e691564c2a8d7aa231527700a7c9da370171894e10f6e982`). Actual unpaused native build/Solid cancellation, source change, restoration and bounded-work/invalid-path refusals pass. No page errors. Artifact: `ui-nonuniform-retained-g2-joint-miter-current/matrix.json`. Combined with the closed nonaxial RMF body mode, **80/80 UI checks** are verified on this package; this is two modes, not the complete UI matrix.


## 2026-10-07 nonbinary station ratios: reproduced refusal and Rust correction

A fresh native constructor regression with retained station positions `[0,3,10,21,34]` reproduced a false G2 refusal in the previous normal-scale proposal: normalized strip speed ratios 7/3, 11/7 and 13/11 cannot be represented exactly as binary64. The stored straight regular surfaces are geometrically smooth. Reproduction log: `nonaxial-corrected-revalidation/nonuniform-rational-scale-before.log`.

Rust now offers a bounded continued-fraction rational scalar proposal, kept internal to retained station audit. The candidate is an exact reduced `AuthoredScalar::RationalConstant` consumed by the existing projective whole-strip coefficient identities under the shared exact-work owner. Numeric closeness selects a candidate only and is never an admission predicate. One-bit mutations are refused when the candidate does not satisfy the original represented coefficients; no snapping or fitting is introduced. Existing explicit binary64 public strip-audit APIs preserve their semantics. Proposals exceeding bounded numerator/denominator limits remain binary64 and can remain unproved; this is not a universal arbitrary-ratio completeness claim.

Native progressive sweep **170/170** pass (33.44s), including dyadic and nonbinary station ratios, one-under-work refusal, one-bit no-snap refusal and geometric kink rejection. Native join continuity **29/29** pass, including bounded/reduced proposal edge cases, exact strip regression and regularity. Vue types and scoped diff checks pass. Public retained G2 tests now include nonbinary stations in all six existing modes, but have not yet been run on a published package containing this correction. Singleton publisher session 94322 is building it; current package remains `870a2654...`. Logs: `nonuniform-rational-scale-after.log`, `nonuniform-rational-scale-native-full.log`, `nonuniform-rational-scale-join-native.log`, `nonuniform-rational-scale-types.log`, `nonuniform-rational-scale-publish.log`.

Separately, independent STEP qualification for `870a265406e647d60cf4b65fd6743e60b6c02eefb9fc8b36c53c4bcba80d0705` completed all four stages in order: export, rational-boundary reference, generator reference, OpenCascade. **100/100 passed** for import, topology and independent reference volumes. Artifact: `external-step-nonuniform-retained-g2-current/opencascade-sweep.json`; logs `nonuniform-g2-step-{export,rational-reference,generator-reference,occt}.log`. Numerical reference convergence is validation, not an interval global-embedding proof. These results do not apply yet to the rational-scale source change or future publication.

The full original scope remains active: general spatial closed RMF/holonomy continuousBound; all-mode regularity/intersections/holes/nesting/orientation; all applicable moving/multispan/closed retained G1/G2; complete joint miter and UI matrices. No Rust geometry was moved to TypeScript and no foreground UI was used.


## 2026-10-07 geometric scaling inside retained multispan Bezier chains

A new source regression reproduced a retained G2 false refusal on a regular planar degree-2 Bezier chain with control X positions `[0,1.5,3,6.5,10,15.5,21]`, full-multiplicity internal knots, and nonuniform positive cross-jet speed ratios 7/3 and 11/7. The previous internal along-chain audit used scale=1, incorrectly requiring equal normalized speeds. Reproduction: `nonaxial-corrected-revalidation/nonuniform-along-chain-before.log`.

Internal along-chain and copied closure jets now use a bounded rational scale proposal from actual original strips. U boundaries are handled through exact index transposition for the proposal only. Independent projective coefficient identities, every internal/closed seam, positive scaling, unchanged original coefficients and whole-span normal regularity remain mandatory. Explicit binary64 public strip-audit input semantics remain unchanged. Source curves, arbitrary sheared reparameterizations, general spatial closed RMF or Solid are not promoted.

Native continuity **30/30** passed. Progressive sweep **170/170** passed (32.49s), now including actual native constructors combining both dyadic/nonbinary station speeds with a nonuniform multispan profile. One-under-work, one-bit candidate/no-snap rejection and kink refusals remain tested. Vue types and scoped diff checks pass. Logs: `nonuniform-along-chain-native.log`, `nonuniform-along-chain-sweep-native.log`, `nonuniform-along-chain-types.log`.

A new public test for the joint nonuniform retained profile/station case is prepared in `tests/nurbsRetainedSweepSmoothness.test.ts`, but has not passed a packaged WASM yet. Publisher session 94322 finished release compilation (1m37s) before the along-chain source change and is still optimizing its earlier rational-station-only binary. Do not attribute along-chain changes to that package. Current published identity remains `870a2654...` until the singleton publisher completes. A subsequent publication must include the along-chain source and run the full prepared public suite. No second publisher was started concurrently.

This is a sufficient geometric jet extension, not a universal G1/G2 claim. The full objective remains active and all mode/global/continuousBound/UI limits from earlier milestones remain explicit.


## 2026-10-07 rational station publication and topology-owned BRep propagation

Publisher 94322 completed optimization, packing and public WASM. Rational-station-only package SHA-256 `00bc28877f56c53c575bd1cb6a008088609b38b5fb5a8c656ce70376b0eab4b1`, 11,535,127 bytes. Packaged public retained station tests **12/12 passed**, one explicit skip for the later nonuniform internal Bezier-chain test which was not in that compiled package. This pass includes nonbinary speeds in six modes. Log: `nonaxial-corrected-revalidation/nonuniform-rational-scale-public.log`. The second singleton publisher (38514) was started only after the first completed; it contains the internal Bezier-chain correction and has completed release compilation (1m26s), currently optimizing.

A fresh topology-owned BRep regression with circle sections at Z `[0,3,10,21,34]` reproduced the same false station G2 refusal: the BRep smoothness owner still passed a rounded binary64 ratio to explicit-scale collection audit. Before log: `nonuniform-owned-stations-before.log`. Rust now has a separate rational-proposal seam-collection entrypoint. Explicit authored binary-scale collection APIs retain their semantics. The topology owner proposes scales from actual U/V boundary strips, then requires exact projective identities and independent regularity for every owned seam. G2, G1 fallback and profile/station groups continue sharing one owner budget; adjacency/cap classification is still required.

Native BRep sweep smoothness **4/4** passed, including nonbinary station speeds, every owned station edge, exact budget, invalid trim/topology and closed sharp-miter C0. Log: `nonuniform-owned-stations-after.log`. Full BRep suite is running (67197); geometry-bridge serialization test with nonbinary stations is running (95623). A public BRep transport regression is prepared in `tests/nurbsOwnedStationRationalG2.test.ts` but has not run on a package containing the owner change. These owner changes occurred after publisher 38514 compiled its dependencies, so that package must not be credited with them; another serial publication is required before public verification.

Original full-mode/global/continuousBound/smoothness/miter/UI requirements remain active. Current STEP100 and UI80 qualifications remain tied to `870a2654...`, not `00bc2887...` or newer source. No foreground UI was used.


## 2026-10-07 retained chain public pass and repaired BRep regression

The second publisher completed successfully. Internal-chain package SHA-256 `71566735f0ea08b9d05ecfe8f606639edc081faba0008811ef5923a0dbc5ad4f`, 11,537,141 bytes. All prepared retained station/chain public tests **13/13 passed**, including nonbinary station speeds, six modes and joint nonuniform Bezier profile/station speeds. No skip remains in that suite. Log: `nonuniform-along-chain-public.log`. This package predates the topology-owned rational collection change and its near-unit candidate repair.

Broad BRep regression initially exposed two failures (810 passed, two failed, two ignored) in existing rational-profile G1 qualifications. Numerical proposal rounding near exact one bypassed the exact-one candidate. The candidate proposer now retains a binary64 dyadic proposal only if it exactly equals the rational candidate; otherwise the exact reduced candidate must independently satisfy whole-strip identities. This changes proposals, not stored coefficients or acceptance tolerances. Added native near-unit proposal regression. Explicit authored binary64 strip/collection APIs are unchanged.

After repair, the full BRep suite **812 passed, zero failed, two existing ignored**, 82.60s. Both formerly failing profile tests pass. Native join continuity **30/30** passed (0.20s). Geometry bridge serialization for actual nonbinary circle stations **1/1** passed; stationG2/G2, capC0, fullBoundarySmoothness=false and shared exact-work accounting remain explicit. Vue types and scoped diff checks pass. Logs: `nonuniform-owned-stations-brep-full.log` (initial failures), `nonuniform-owned-stations-profile-regression.log`, `nonuniform-owned-stations-brep-full-repaired.log`, `nonuniform-owned-stations-join-repaired.log`, `nonuniform-owned-stations-bridge.log`, `nonuniform-owned-stations-types.log`.

A third singleton publisher is now building the full owner-propagated source, after the second publisher completed. Its required public test is `tests/nurbsOwnedStationRationalG2.test.ts`; it must run together with the retained suite after complete packing/publication. STEP100 and UI80 remain tied to `870a2654...`; no new package inherits those results automatically. The full original spatial RMF/holonomy, global correctness, applicable smoothness and miter/UI matrix requirements remain open.


## 2026-10-07 nonuniform station Rush/viewport/Solid and STEP fixture prepared

Added `examples/rush/nonuniform-station-g2-hollow-miter.r`: original annulus radii 0.5/0.25 and straight source stations `[0,3,10,21,34]`, explicit constant scale/twist, bounded circle correction and proof work. Public test `tests/nurbsOwnedStationRationalG2.test.ts` requires native continuous boundary/Solid, viewport stationG2 and at least 24 station seams. A before-publication run on package `71566735...` reproduced the expected gap: continuousBound, boundary budget and Solid succeed but stationContinuity=C0/stationG2=false. Log: `nonuniform-owned-stations-rush-before-publication.log`. This is not a failure of the new owner-propagated package, which is not yet published.

Registered the same Rush mode in the headless Metal UI matrix, including mandatory station-G2 presentation, lifecycle, native running cancellation, invalid-path and bounded-work refusals. STEP exporter now includes this fixture and explicitly requires native G2 plus the complete station seam set before export. Its independent source volume is `pi * (0.5^2 - 0.25^2) * 34`; this does not use exported geometry to manufacture an expected volume. Matrix size becomes 101 only once the new export completes; no STEP101 result is claimed yet.

Vue types, browser script syntax and scoped diff checks pass. Publisher 73170 completed release compilation (1m49s) and its authenticated wasm-opt process remains active on the owner-propagated source; no duplicate publisher was started. Next: complete packing/publication, run retained+owned/Rush suites, build Vite, run the new headless Metal UI mode, then serial export/reference/reference/OCCT.

Full general spatial RMF/holonomy continuousBound, global all-mode guarantees, applicable smoothness and the full joint miter/UI matrix remain open. No foreground UI was used.


## 2026-10-07 owner-propagated public/Rush/UI milestone and STEP-found regression

Owner-propagated publisher 73170 completed. Package SHA-256 `84d336f927d99a82650d5a4277b917a5889872cc0e6d011ffb743f2891179b34`, 11,537,423 bytes. Public retained/owned/Rush tests **15/15 passed**, including actual original nonuniform source stations, full owned edge set, exact shared budgets, source preservation and separate Solid/continuous-bound admission. Rush viewport evidence now reports stationG2 and at least 24 station seams. Five-file public regression **51/51 passed** (65.47s). Vite and types passed. Logs: `nonuniform-owned-stations-public.log`, `nonuniform-owned-stations-public-regression.log`, `nonuniform-owned-stations-vite.log`, `nonuniform-owned-stations-rush-types.log`.

Headless Metal UI for `nonuniform-station-g2-hollow-miter.r`: **38/38 passed** (19 each at widths1440/600), actual Apple/metal-3 adapters, source/public/dist identity verified, no pageErrors. Solid has 34 faces, one shell and geometry SHA-256 `9ef35179f4fa646f0b8a795647887401d3f41497f6405a823be107000ffa2858`. Station G2 presentation, authenticated running native cancellation, refusal, source change and restoration pass. Artifact: `ui-owned-nonuniform-station-g2-current/matrix.json`; log: `nonuniform-owned-stations-ui.log`. No foreground window.

Expanded STEP101 export on this package failed before completing its manifest at the pre-existing corrected spatial circle fixture: profileG2 was lost under the new numerical-first profile proposal. Log: `nonuniform-owned-stations-step-export.log`. This is a genuine uncovered regression; neither STEP101 nor universal compatibility is claimed for `84d336f9...` despite the finite public/UI positives.

Rust owner profile seams now keep exact nominal scale1 as their first candidate. Only after its exact identity fails can a different numerical/rational candidate be tried under the same remaining budget. No successful nominal proof is replaced by sampling. Independent nominal-failure/rational-fallback, explicit-scale compatibility, negative kink and one-under-total-work regression added. Native join **31/31 passed** (0.27s). Full BRep **812 passed, zero failed, two pre-existing ignored** (63.66s). Types and scoped diff pass. Logs: `nonuniform-owned-nominal-first-join.log`, `nonuniform-owned-nominal-first-brep.log`, `nonuniform-owned-nominal-first-types.log`.

Added a public corrected-spatial-circle regression in `nurbsOwnedStationRationalG2.test.ts`. A new singleton publisher is building nominal-first source after the previous publication completed. It must finish, then run all16 retained/owned/Rush tests, the corrected spatial source, new UI and the entire serial STEP101 pipeline. No public success of this final repair is claimed yet. Full spatial RMF/holonomy continuousBound, all-mode global and applicable smoothness/miter/UI scope remain open.


## 2026-10-07 nominal-first published compatibility and paired UI proof

Singleton publisher79084 completed optimization, packing and public publication. Package SHA-256 `cf0d159dccba44397292858a40c9b5657691a2061a923d04248ee842bf4b3b1f`, 11,538,032 bytes. All retained/owned/Rush public tests **16/16 passed**, including the corrected spatial circle regression that previously stopped STEP, nonuniform station G2 and shared-budget/refusal ownership checks. Six-file broader public regression **83/83 passed** (118.32s). Combined disjoint suites: **99/99**. Vite build passed and the current WASM was verified at source/public/dist by both UI reports.

Paired headless Metal UI, two widths1440/600 each: nonuniform-station hollow miter **38/38** and corrected spatial-circle hollow miter **38/38**. Total **76/76** on this exact package. Both actual adapters Apple/metal-3; pageErrors empty. Authenticated unpaused running-native cancellation, lifecycle restoration, source changes, bounded-work/invalid-path refusals, G2 presentation and successful Solid passed. Nonuniform body 34 faces/one shell, geometry hash `9ef35179f4fa646f0b8a795647887401d3f41497f6405a823be107000ffa2858`; spatial body26 faces/one shell, hash `c5bf1887c489e6ca360c388a9c3f60a71a27564694d494fbe0abf24b866e8a82`. Artifacts `ui-owned-nominal-first-nonuniform-current/matrix.json`, `ui-owned-nominal-first-spatial-current/matrix.json`.

Logs under `nonaxial-corrected-revalidation/`: `nonuniform-owned-nominal-first-public.log`, `...-public-regression.log`, `...-vite.log`, `...-ui.log`, `...-spatial-ui.log`. The new STEP101 export (64602) has passed the previous spatial-G2 failure and is actively computing the full matrix, not yet a completed STEP101/OCCT result. Output `external-step-owned-nominal-first-current`; log `nonuniform-owned-nominal-first-step-export.log`. After export, rational-boundary reference, generator reference and OCCT must run sequentially against that manifest.

Compatibility of the selected original spatial case is now positively revalidated in public WASM and actual viewport/Solid; native source had already passed31 continuity and812 BRep (two pre-existing ignored). This does not promote universal arbitrary-ratio/shear G2, general spatial closed RMF/holonomy continuousBound, global all-mode regularity/intersections/nesting/orientation or the entire miter/UI matrix. The full goal stays active. No focus-stealing UI or Git cleanup was performed.


## 2026-10-07 STEP101 independently completed on nominal-first package

The complete sequential pipeline on package `cf0d159dccba44397292858a40c9b5657691a2061a923d04248ee842bf4b3b1f` terminated successfully: export **101** cases, rational-boundary reference, generator reference, then OpenCascade. Independent report `external-step-owned-nominal-first-current/opencascade-sweep.json` asserts **101/101 passed**, all case flags true and exact matching package provenance (11,538,032 bytes). The previous corrected-spatial-circle G2 regression no longer blocks export. Logs under `nonaxial-corrected-revalidation/`: `nonuniform-owned-nominal-first-step-export.log`, `...-step-rational-reference.log`, `...-step-generator-reference.log`, `...-step-occt.log`.

New `rush-nonuniform-station-g2-hollow-miter.step`: valid Solid, 34 faces, one closed shell, 72 matched source edges, two cap-hole faces. Manifold/opposite edge uses, same-parameter, source face-loop ownership, shared basis and full-domain wall/cap/edge distance checks pass. Cap exact coplanarity agrees at both ends. OCCT volume **20.027653169762516**, independent source volume **20.02765316663493**, relative error **1.5616334454326369e-10**, reported numerical integration error **1.3889669385403023e-13**. Sampled surface/derivative errors ~3.6e-15 and pcurve position error ~7.1e-15 are numerical checks, not general continuous-smoothness or material-containment proof. The fixture also required native retained station G2 and complete >=24 station seam ownership before export.

Current milestone evidence is now **99/99 packaged public regression tests**, **76/76 paired actual-Metal UI checks**, **STEP101/101 independent OCCT checks**, alongside native31 continuity and812 BRep (two pre-existing ignored). Every package attribution is explicit; old results are not inherited onto future binaries. No pending process remains in this milestone.

This completes this finite rational/nonuniform G2 compatibility milestone, not the full objective. General spatial closed RMF/holonomy continuousBound; all-mode regularity/intersections/holes/nesting/orientation; all applicable moving/multispan/closed G1/G2 including general shear reparameterization; the complete affine/authored/guide miter and broad UI matrix remain unproved where earlier audits identify limits. No foreground UI was used.


### 2026-10-07 — current-package STEP101 and spatial RMF Metal UI

Geometry WASM 83b0690d262cffe239c0f7058d8804d93caeb443282e6ea7c8b869d9991692de, 11565836 bytes, and language WASM 9b929825fb749982c3fbcbccca3330514553c81b4c02395056aebfa38e997791 are the current qualification basis. Production Vite build passed. Re-exported the existing 101-body STEP matrix under external-step-spatial-rmf-policy-current; export, independent rational boundary volume, independent generator reference and OpenCascade were strictly sequential and all terminated successfully. OpenCascade report passed 101/101, with source/public/packed provenance verified. This refresh qualifies the current package for these existing fixtures; it does not add a closed spatial RMF body case or prove universal continuous error/global correctness.

Added closed-spatial-rmf-affine-certified.r to the existing surface UI matrix. Headless actual Apple/metal-3 ran at 1440x1000 and 600x1000: 15 assertions each, 30/30 total, no page errors, source/public/dist/worker hash identity verified. UI shows scoped original retained-profile error within 0.25, separate regularity presentation, strict-budget refusal/restoration, material panel/fit, source replacement, invalid-path refusals and explicit surface Solid refusal. Build/Solid cancellation and source-change are held worker-dispatch lifecycle probes, not active native mid-kernel cancellation qualification. The browser was never headed and did not take user focus. Corrected the generic assertion label to closed-spatial-rmf-configured-0.25mm-evidence before final matrix regeneration.

Evidence: external-step-spatial-rmf-policy-current/opencascade-sweep.json and ui-spatial-rmf-policy-current/matrix.json. Logs under nonaxial-corrected-revalidation: spatial-rmf-policy-step-export.log, spatial-rmf-policy-step-rational-reference.log, spatial-rmf-policy-step-generator-reference.log, spatial-rmf-policy-step-occt.log, spatial-rmf-policy-vite.log and spatial-rmf-policy-ui.log. Scoped whitespace check passes.

Remaining: source/retained spatial RMF regularity and global embedding across applicable closed profile/hole modes; successful closed spatial RMF Solid (the current open profile fixture must refuse); full cap/decomposition/correction continuous coverage; applicable retained G1/G2 moving/multispan/closed seams; all joint miter-law combinations; full current-package UI matrix including actual native cancellation. Completion is not claimed.


### 2026-10-07 — closed spatial RMF circle: native owned boundary and volume

One asymmetric closed Cartesian-C1 cubic spatial source, nonzero holonomy, circular profile radius 0.05, constant scale 1.1/twist 0.125 radians/affine axes (1,1.25,0.75)/centre (0.01,-0.02,0.03), 129 stations, original arc-length tolerance 0.0001 and 4096 transport cells.

Native retained continuous wall error is 0.11707728618662984 (target 0.25). All retained Jacobians are certified in 1910 cells; one-under budget remains incomplete. The independently assembled periodic section loft has 512 owned faces, one shell and one body, valid topology, exact retained wall coefficient coverage (122880 work). Separate `volume_validity::inspect_sweep` confirms boundary embedding, consistent nesting and outward orientation. The regression now requires the separate boundary and volume proofs.

Evidence: `nonaxial-corrected-revalidation/spatial-rmf-closed-profile-regularity-native.log` and `nonaxial-corrected-revalidation/spatial-rmf-closed-owned-topology-native.log`; source `crates/brep-core/src/spatial_rmf_qualification.rs`. This is a native fixture only, with no caps. It does not establish all-mode/global guarantees, filled-cap error, G1/G2, the miter combination matrix, or successful spatial RMF Solid via published WASM/Rush. The current STEP101 excludes this body. No production package changed in this milestone.


### 2026-10-07 — constructor-owned spatial RMF policy through the body API

Added native `approximate_spatial_rmf_profiles` (one shared finite owner) and backward-compatible `progressive_profile_body_with_rmf_policy`. Original callers retain their prior policy. Explicit closed arc-length RMF policy excludes authored/guide frames, rejects incomplete/non-dyadic/excess work controls, and routes aggregate admission through the original retained-patch proof. Body construction separately binds retained wall coefficients and decomposition to its original boundary error.

The native circle fixture now uses the actual progressive body constructor, rather than independently assembling the loft. Boundary error fits 0.25, retained walls are certified, and separate full volume validity remains proven on 512 owned faces. Native regression passed 1/1, Rust frontend 13/13, Rust bridge/frontend check and TS type check passed. Geometry bridge, Rush lowering/schema, TS transport and native-budget metadata now carry the policy for `brep_progressive_sweep`. Added `closed-spatial-rmf-affine-body-certified.r`, a public Rush/body/Solid admission regression, and the 102nd STEP fixture. At this checkpoint public WASM validation and STEP102 remain pending; the existing STEP101 evidence must not be attributed to this body or new package. Full goal remains open.

Logs: `nonaxial-corrected-revalidation/spatial-rmf-body-policy-{native,check,frontend,types,wasm,language}.log`.


### 2026-10-07 — published spatial RMF body policy: public qualification

Both publishers completed. Geometry SHA-256 `faebe42bb97c71752aac11308ddec29c10da0b3e594e314449fc4764099d428b` (11568090 bytes); language SHA-256 `f13c9d88c65dd92d0043b97d48d88f0b3c94d1493dad320f8ed462c7ed53c0ee` (2284113 bytes). Four public WASM regression files passed 76/76 in 99.59 seconds: spatial RMF error/body policy, closed planar RMF compatibility, progressive body boundary compatibility and viewport evidence. The new circle tube passes Rush lowering, native body construction, complete boundary-error presentation (budget 0.25), and fresh Solid admission on the transferred 512-face BRep with all pairs classified. Zero work refuses construction; non-dyadic controls refuse Rush compilation. Vite and TS type checks passed.

STEP102 export and actual Metal wide/narrow body UI lifecycle qualification are running in `external-step-spatial-rmf-body-current` and `ui-spatial-rmf-body-current`. Neither is claimed passed yet. Prior STEP101 belongs to an earlier package. New public admission does not establish the entire all-mode goal, cap correction, source/retained G2 or all miter combinations.


### 2026-10-07 — spatial RMF body Metal UI30

`ui-spatial-rmf-body-current/matrix.json` completed successfully: 30 assertions, 1440x1000 and 600x1000, actual Apple metal-3/WebGPU, no page errors. Published/source/dist/worker provenance matches geometry SHA-256 faebe42bb97c71752aac11308ddec29c10da0b3e594e314449fc4764099d428b. Both windows publish a 512-face, one-shell Solid; boundary estimate, strict refusal/restoration, bounded-work refusal preserving the existing body, invalid source refusal, source changes and held worker-dispatch lifecycle cancellation pass. This run is headless and does not steal focus.

Held-dispatch cancellation is not native interruption evidence. A separate `ui-spatial-rmf-body-native-cancel-current` run with authenticated CPU sampling and `--native-running-cancel` is in progress. STEP102 export is also still in progress. Independent rational reference and OpenCascade stages have not started for the new export. Full goal remains open.


### 2026-10-07 — current STEP102, retained G2 and running native cancellation

Sequential STEP102 export, independent rational-boundary reference, independent canonical-generator reference and OpenCascade verification all completed successfully on geometry SHA-256 faebe42bb97c71752aac11308ddec29c10da0b3e594e314449fc4764099d428b. `external-step-spatial-rmf-body-current/opencascade-sweep.json` passed 102/102. The new 512-face spatial RMF tube is a valid, one-solid, closed-shell STEP with retained rational surfaces, pcurves, full-domain edge/surface distance gates, manifold opposite edge uses and outward material orientation. Independent retained-boundary volume 0.08304581512039794 versus OCCT 0.08304581513985104 (relative error 2.3424546757322323e-10). This qualifies the retained model, not universal source geometry or all combinations.

Published retained-surface/station G2 regression files passed 20/20, including unequal station steps, exact scaled station jets, negative work/geometry cases and sharp C0 preservation. All-mode smoothness is still not established.

`ui-spatial-rmf-body-native-cancel-valid-current/matrix.json` passed 42 assertions on 1440x1000 and 600x1000, Apple metal-3, no page errors, published/source/dist/worker hash binding. Native cancellation observes authenticated CPU samples of geometry abi_request without Debugger suspension; actual Worker termination prevents stale build/Solid and source replacement results. The busy build retains the source's legal 129 stations. The earlier native-cancel run with 257 initial sections exceeded the body face ceiling and is not accepted as valid body-build cancellation qualification. Timings include UI scheduling and fixed observation overhead; no cooperative kernel latency claim.

Spatial hollow extension is now under diagnosis. Its two contour error and retained regularity pass, but default global admission exhausts contact work at 100000 cells, leaves nextPair [31,39], and does not prove every chart injective. No nesting/orientation promotion occurs. New native positive regression intentionally remains failing until complete global proof is obtained. An explicit larger finite-work diagnostic is running; budgets or proof strategy must remain bounded and evidence-owned. `closed-spatial-rmf-affine-hollow-body-certified.r` is an unqualified pending fixture; 2 tessellation segments fit the existing 20000-triangle display budget.


### 2026-10-07 — complete retained spatial RMF hollow boundary under bounded work

Resolved the hollow admission failure without weakening geometry predicates. Source/retained regularity and error were already complete; old default global limits covered only 1000 contraction spans, 20000 projection cells and 10000 individual pairs. The 1024-face two-shell body requires 1024 spans, 52949 projection cells, 13008 individual pairs and 113910 total contact cells. New finite default volume policy: maxSpans 1024, maxLinearCells 100000, maxPairs 20000, maxCells 200000; independent native validation and all refusal gates remain intact.

Fresh public native volume proof classifies all 523776 pairs (13008 individual + 510768 grouped), certifies all charts injective, proves parents [null,0] and material roles, and proves outer outward=true / cavity outward=false. Exact owned-wall coefficient work is 245760 and complete retained boundary error remains 0.11707728618662984 <= 0.25. Shared original RMF and decomposition budgets are unchanged.

The native positive regressions now pass 2/2 in 160.60 seconds, including the single and hollow constructors. Updated public spatial/body/planar compatibility files pass 62/62 in 159.78 seconds, with hollow Rush/body/viewport/Solid admission and explicit exhausted-global-budget refusal. Type checks, Vite and scoped whitespace checks pass. Geometry and language WASM hashes remain faebe42b / f13c9d88; this milestone changes resource configuration, qualification and fixtures, with all geometry and certificate algorithms still in Rust.

Added pending STEP103 fixture (`closed-original-spatial-rmf-holonomy-affine-hollow-tube.step`) with explicit recorded larger finite audit budgets; legacy STEP cases retain their recorded pair/contact limits. STEP103 export and full Metal wide/narrow hollow UI including running native cancellation are now executing. STEP102 and single-body UI42 are verified earlier results, not yet proof of this pending hollow matrix. Full sweep/miter goal remains open.


### 2026-10-07 — hollow spatial RMF full Metal UI42

`ui-spatial-rmf-hollow-native-cancel-current/matrix.json` completed successfully: 42 assertions across 1440x1000 and 600x1000, actual Apple metal-3/WebGPU, no page errors, public/source/dist/worker geometry hash binding. Both windows publish the 1024-face, two-shell hollow Solid and verify boundary-error presentation, strict and bounded-work refusals with restoration, invalid-path refusal preserving the existing body, source replacement, actual unpaused native build/Solid cancellation and post-cancellation restoration. The valid 129-station workload is preserved; CPU tail authentication excludes Debugger pause. This proves the finite hollow scenario only, not all modes or cooperative kernel cancellation latency.

STEP103 export remains executing in `external-step-spatial-rmf-hollow-current`; no independent reference/OCCT claim for that matrix yet. All previously failing hollow native assertions are now passing with the recorded finite work policy. Full goal remains open: general frame/law/cap coverage, all-mode global guarantees, applicable full retained smoothness and complete miter matrix still require evidence.


### 2026-10-07 — STEP103 and simultaneous rational spatial RMF laws

Sequential export, independent rational-boundary volume, independent canonical-generator volume and OpenCascade stages completed for `external-step-spatial-rmf-hollow-current`: 103/103 passed on published geometry faebe42bb97c71752aac11308ddec29c10da0b3e594e314449fc4764099d428b. New hollow spatial RMF: valid 1024-face / two-shell / one-solid STEP, preserved rational surfaces/coedges, manifold topology and material orientation. Independent volume 0.06975848470113427 versus OCCT 0.06975848471747483, relative error 2.3424467976884425e-10. This qualifies retained geometry with the recorded finite native work policy.

Added simultaneous degree-two rational scale, twist, affine axes and centre laws, each weights [1,2,1] and matching end values, on the same spatial C1 source with nonzero holonomy and two circle contours. Native full constructor/regularity/owned wall/global volume regression passed 1/1 in 139.05 seconds. Boundary error 0.11940106525066174 <= 0.25, 1024 owned faces, 245760 exact wall work, consistent shell nesting and outward=true / inner=false. Public WASM spatial suite passed 6/6 in 87.72 seconds, including the varying-law Rush/body/viewport/Solid path and explicit exhausted-global-work refusal. Type and scoped whitespace checks passed. Geometry/certificates remain Rust-owned; no production WASM change was needed for this input extension.

Added `closed-spatial-rmf-varying-affine-hollow-body.r`, pending STEP104 fixture and pending actual Metal wide/narrow native-cancel UI. Their export/UI jobs are running; do not attribute STEP103 or the earlier hollow UI42 to the varying laws.

General RMF completion remains explicitly unproved: source inspection confirms `with_spatial_rmf_error_limits`, original arc-length frame error dispatch and `endpoint_jets` exclude periodic curves. Supporting original periodic Cartesian jets/closure with charged extraction ownership is a next kernel gap; merely lifting guards would be unsound. No all-mode or full-goal claim is made.


### 2026-10-07 — STEP104/UI42 complete; original periodic spatial RMF native proof

Previous varying rational scale/twist/axis/centre hollow scenario completed sequential STEP104 export, independent rational and generator references, and OpenCascade: 104/104 passed in external-step-spatial-rmf-varying-current. Metal UI native cancellation passed 42/42 across wide/narrow windows in ui-spatial-rmf-varying-native-cancel-current, no page errors, authentic unpaused native execution and worker destruction. These results use pre-periodic published geometry faebe42bb97c71752aac11308ddec29c10da0b3e594e314449fc4764099d428b.

Original periodic B-spline Cartesian endpoint jets and internal continuity are now handled by native exact predicates. Interior C^(p-m) follows original positive-weight basis continuity, with input work charged; independent exact endpoint expansions still prove closure. The explicit spatial RMF policy now accepts periodic sources only after these proofs. A one-bit exterior knot mutation validates numerically but fails exact source closure and strict continuous-error body admission. No proxy clamping, budget enlargement or sampled fallback is used.

Native periodic cubic spatial body passed: complete retained boundary error 0.04221549508184847 <= 0.25, 512 owned faces, 122880 wall work, separately proven regularity, boundary embedding, nesting and outward orientation. Full native sweep suite 175/175; exact interior predicates 7/7; original periodic jets 1/1; periodic body positive and adversarial refusal 1/1. Logs: nonaxial-corrected-revalidation/spatial-rmf-periodic-{body-native,sweep-native,predicates-native,jets-native}.log. Geometry/certificates remain Rust-owned.

New periodic Rush fixture, public regression, STEP105 entry and headless UI entry are prepared. WASM publication is running; public periodic Rush/viewport/Solid, STEP105 and periodic UI are not yet qualified. Default legacy periodic planar RMF policy and universal all-mode/cap/smoothness/miter guarantees remain open.


### 2026-10-07 — periodic spatial RMF published Rush/viewport/Solid

Geometry publisher completed, SHA-256 8a907bacd049b3d9495079269875612b4af7eb525a87a6791b769e8837691ed2, 11568404 bytes. Public spatial RMF suite passed 7/7 in 133.54 seconds, covering original periodic body Rush construction, complete boundary error presentation, fresh complete native Solid admission, one-bit exterior-knot rejection and bounded-work refusal, plus prior constant/hollow/varying-law compatibility. Preliminary async UI transport now labels explicit RMF proof failure as continuous retained-patch error instead of sampled error; admission still comes from native Rust. Type check, Vite, script syntax and scoped whitespace passed.

STEP105 export is executing in external-step-periodic-spatial-rmf-current; independent references/OCCT remain pending. Headless Metal native-cancellation UI is executing in ui-periodic-spatial-rmf-native-cancel-current: wide window 21/21 passed, narrow window pending. These partial runs are not full matrix qualification. Language WASM remains f13c9d88; no language lowering change was needed. Full goal remains active and incomplete: all applicable combinations/caps, all-mode global proofs, retained smoothness and miter matrix still need coverage.


### 2026-10-07 — periodic joint laws/hollow/UI and open spatial RMF filled caps

Periodic constant body UI42 completed on geometry 8a907bacd049b3d9495079269875612b4af7eb525a87a6791b769e8837691ed2. Added periodic spatial RMF with simultaneous rational scale/twist/axis/centre laws and cavity: native full owned body/global proof passed 1/1 in 146.22 seconds, retained error 0.043239748684575696 <= 0.25, 1024 faces, 245760 wall work, proven consistent nesting and outer/inner orientation. Targeted public Rush/viewport/fresh Solid regression passed 1/1 (seven other tests skipped) in 38.67 seconds. Its separate headless Apple/metal-3 native-cancellation UI passed42 across both windows with package hash binding. STEP105 (constant periodic source) export and both independent reference stages finished; OpenCascade is executing. STEP106 joint periodic entry is prepared but not yet exported.

New native explicit open spatial RMF policy reuses the original Bishop source cover without closed holonomy and now charges inverse arc-length speed brackets. Refactored the original RMF twisted frame query for both retained error and endpoint material proof, preserving one caller's charged interval/exact work. Original cap domain and inverse-transpose plane transport use that same source frame evidence. Constructor cap-proof source retains the explicit policy instead of dropping it.

Open degree-five nonplanar source with circular contour, constant scale/twist/anisotropic axes/centre: native continuous wall test passed (error 0.02411436574029989, 23852 cells). Actual corrected 514-face body passed complete boundary error 0.023621149813405448 <=0.25, filled caps [0.0050434455017521924,0.0050434455017521924], correction 5.817484852432731e-13. Separately proved full boundary/volume; one-under endpoint-plane work discards normals. Full native sweep regression176/176 passed after endpoint-frame refactor. No sampled fallback or geometry in TS.

Open RMF production WASM publisher is executing; Rush fixture/public regression are prepared but not qualified on the future package. Existing 8a907bac STEP/UI evidence must not be relabelled to new unpublished sources. All-mode/cap combination coverage, applicable retained smoothness and full miter matrix remain unproved.


### 2026-10-07 — independent periodic STEP105 completed

Sequential STEP105 export, rational-boundary reference, generator reference and OpenCascade completed: external-step-periodic-spatial-rmf-current/opencascade-sweep.json passed105/105 on geometry 8a907bacd049b3d9495079269875612b4af7eb525a87a6791b769e8837691ed2. New constant periodic RMF body retained512faces/1024edges/oneclosed-shell/onesolid, full-domain wall and edge distance gates, original shared bases, pcurves and manifold opposite coedge uses. Independent retained volume0.07286925327964143 vs OCCT0.07286925329671075, relative2.3424582266026443e-10. Sampled material-side probes remain sampled; this is not all-mode source proof.

Open spatial filled-cap native regression including separate complete global volume and one-under endpoint-plane refusal passed1/1 in100.46s. Full native sweep176/176 in72.98s. New geometry publisher remains live; prepared STEP107 adds joint periodic varying hollow and open corrected spatial RMF, but that future export has not started. Full objective stays active.


### 2026-10-07 — open spatial RMF published full boundary and current G2/UI

Geometry publisher completed: SHA-256 02bcfa6e84919a46aa0909efc6e990271a103fe079f8ae6ebaa95b59043cc8e6 (11569050 bytes). Three public suites passed29/29 in174.58s: spatial RMF9, retained sweep/station smoothness and miter station smoothness20. Open corrected spatial body is qualified through Rush, viewport complete boundary-error evidence and fresh native Solid admission, with514ownedfaces and all pairs classified. Cap correction one-work refusal, original periodic one-bit knot refusal, finite RMF work refusal, constant/hollow/simultaneous rational law compatibility remain intact. Current unequal station G2/moving-frame applicable cases and sharp C0 preservation are verified on this same package. No source-frame proof is promoted to retained smoothness.

Open spatial RMF headless Apple/metal-3 UI completed42checks across1440x1000 and600x1000, no page errors and verified source/public/dist/worker hash identity. Actual unpaused native build/Solid cancellation, source replacement, strict/bounded-work refusals and restoration passed. No focus changes and no cooperative native cancellation latency claim.

STEP107 export is running in external-step-open-spatial-rmf-current, adding periodic simultaneous rational laws/hollow body and open spatial RMF corrected filled caps. Independent reference and OCCT stages are pending. STEP105 belongs to previous8a907bacgeometry and is not current02bcfa6equalification. Type checks, Vite, script syntax, schema export and scoped whitespace checks passed. Language WASM unchanged f13c9d88. Full goal remains active: universal applicable wall/decomposition/cap combinations, all-mode global proof coverage, full retained G1/G2 and complete joint miter matrix still require evidence.


### 2026-10-07 — open varying hollow filled caps and full-boundary miter refinement

Open spatial RMF with cavity and simultaneous rational scale/twist/axes/centre passed native full constructor, corrected filled caps, one-under endpoint-plane work refusal and separate full boundary/volume proof:1010faces, error0.024925909132565326 <=0.25, filled caps[0.006350943909829395,0.006350943909829395], correction6.125788891721717e-13, native1/1 in145.25s. Targeted published Rush/viewport/fresh Solid path passed1/1 (nine other tests skipped) in46.51s on02bcfa6egeometry. Source open-spatial-rmf-varying-affine-hollow-corrected-body.r retains127stations within1024face ceiling. UI and independent STEP108 entry are prepared; not yet qualified. Existing STEP107 export/reference stages completed and OCCT is running on02bcfa6egeometry.

A new eight-way miter law matrix exposed early constructor refusal: source wall error was accepted before adding endpoint correction, then the complete body budget was rejected without attempting finer permitted levels. Native sweep_miter_owned::construct now preserves rejected level history and doubles permitted steps when a finite corrected wall/full-boundary bound exceeds the unchanged overall tolerance. Missing component proofs still refuse. Exact correction work is charged cumulatively across attempts and subtracted from supplied allowances; failed candidates never reset the correction owner. Final displacement bounds use only the retained accepted geometry, while reported work includes discarded attempts.

Native owned miter regressions7/7 passed in26.14s. New asymmetric moving authored-frame/affine/centre/scale/twist/cavity case progresses through3levels to4steps, complete error1.1363565237433364 <=2, total correction work30204. One-under work30203 and maxSteps1 both refuse; original inputs unchanged. Correction tolerance2 and overall tolerance2 are explicit fixture budgets, not weakened predicates. Initial generated fixture reachability errors were corrected by removing unused guide nodes.

New geometry publisher is running. Expanded matrix16covers all eight affine/authored/guide selections on open and closed sources, moving scale/twist and centre, with closed matching endpoints and sharp C0. Published matrix remains pending; do not claim its currently failing pre-publication case passed. Full all-mode/cap/smoothness/miter/STEP/UI objective remains active.


### 2026-10-07 — published complete-boundary miter refinement and all16 law selections

Geometry publisher completed, SHA-256 e5d3059c1c6723b97de3e2a895a7f544093fe2333153a80b9e4b20c3ba18b280 (11572627 bytes). Four public suites passed39/39 in56.76s: all16 open/closed affine/authored/guide selections with changing scale/twist and centre offset, existing closed combined miter and retained/station G2 regressions. Every matrix case requires continuous boundary error within its explicit budget, matching applied modes, fresh complete Solid, immutable source and strict error-budget refusal; closed sharp seams remainC0. Miter law matrix scripts/miter-law-matrix-sources.ts is shared by public and independent STEP qualification. After source sharing matrix16/16 revalidated in40.13s.

Current-package open varying hollow spatial RMF targeted regression1/1 passed in50.82s (nine others skipped). Its separate headless Apple/metal-3 UI42 passed across1440x1000 and600x1000, verified source/public/dist/worker geometry identity, no page errors, real unpaused native build/Solid interruption and restoration. Miter-moving-affine-full-boundary-refined.r UI36 also passed both windows on the same package, with actual native cancellation. These UI runs are finite cases, not all16 modes.

STEP107 initially failed10cap-count expectations: adding helper's second argument accidentally consumed Array.map indices as capHoleFaces. The source-declared11hollow fixtures require2annular caps; exporter now uses an explicit callback. Failed manifest and oracle report preserved as manifest-failed-cap-count-metadata.json and opencascade-failed-cap-count-metadata.json. Restored input expectations without modifying STEP or independent volumes, then full OCCT rerun passed107/107. Periodic varying hollow retained volume0.06940641161579968 vs OCCT0.069406411632058 (relative2.3424821480778975e-10); open corrected RMF0.06088025396234022 vs0.060880253971963366 (relative1.580668422545555e-10). This matrix remains bound to previous02bcfa6egeometry.

Current e5d3059c STEP124 export is executing in external-step-full-miter-laws-current:108prior cases plus all16 matrix selections. Original source texts, mode flags, source hashes and complete native boundary/volume evidence are retained per case; independent rational-boundary reference and OCCT are still pending. First export attempt stopped on missing native artifact because the display policy was omitted; explicit display policy now preserves the native source artifact and viewport evidence. No component gate was removed. Full goal stays active: broader applicable geometry/law/decomposition/cap coverage, all-mode global proof completeness, full retained smoothness and full STEP/UI scope are not yet proven.


### 2026-10-07 — current STEP124 admission policy and completed varying-hollow UI

Current e5d3059c varying-hollow open RMF UI42 is complete: wide/narrow windows, Apple/metal-3, no page errors, source/public/dist/worker binding, actual unpaused native build/Solid cancellation, invalid input, strict refusal and recovery. Miter UI36 remains a separate current-package finite run. Updated guide/schema and Vite completed after the runs; numeric kernel/package unchanged.

STEP124's later export reached fresh native volume admission and refused the1010face open varying-hollow RMF under legacy100000contact cells/10000pairs. Runtime/native regressions qualified this body with explicit finite200000cells/20000pairs and1024spans/100000projection cells. Added that exact per-fixture policy to the export; other cases retain their prior limits and mandatory fresh native Solid gate. Export restarted in external-step-full-miter-laws-current and is currently live. No independent STEP124 reference or OCCT success is claimed.


### 2026-10-07 — STEP124 exported and independent references complete

STEP124 export completed on geometry e5d3059c1c6723b97de3e2a895a7f544093fe2333153a80b9e4b20c3ba18b280 (11572627 bytes); manifest contains124cases including16open/closed boolean miter law selections. Mandatory fresh native Solid admission passed with recorded finite per-fixture budgets. Independent rational-boundary Gauss reference and source-generator reference both completed successfully. OpenCascade is currently executing; no STEP124 OCCT success is yet claimed. Current native/public/UI results and scope limitations above remain unchanged. Scoped whitespace check passed.


### 2026-10-07 — STEP124 independently confirmed on e5d3059c

OpenCascade completed successfully:124/124cases passed on e5d3059c1c6723b97de3e2a895a7f544093fe2333153a80b9e4b20c3ba18b280. Manifest and opencascade-sweep.json are under external-step-full-miter-laws-current. Both independent reference stages completed before OCCT. All16selected open/closed affine/authored/guide law selections retained full geometry/topology/volume checks; gates unchanged. This numerical fixture matrix is not universal continuous geometry or material-containment proof. Native parameter-spaced spatial RMF implementation is now being extended and tested; it is not yet published and is not the artifact covered by STEP124.


### 2026-10-07 — native parameter spatial RMF extension

Added Rust whole-original-image retained section error for parameter-spaced spatial RMF. Original Cartesian path/law images, positive unchanged profile basis, actual retained section hulls, copied closing seam, original Bishop transport and original arc-length holonomy phase are all charged. Closed correction is not assumed bitwise zero. Parameter multi-profile requests reuse exactly the same original source/normal/policy transport once; each profile's initial coordinates, image queries and decomposition consume the remaining aggregate work. No partial proof admits a boundary. Native progressive sweep177/177 passed, including independent Bernstein+RK4 frame/arc-phase comparison, one-under total-work refusal, exact seam single-bit refusal and JSON transport scope. Full bodies passed: closed varying hollow1024faces/error0.12189255240513755; periodic varying hollow1024faces/error0.06659529711106883 with nesting/orientation proof; open varying hollow1010faces/fullerror0.043618571621586284, filledcaps[0.00038059283895672036,0.0010575277320397014], endpoint correction6.237387965564742e-13. All fit unchanged0.25budget. Initial periodic per-profile recomputation exhausted the shared work; fixed by owned same-source reuse without increasing budgets or relaxing proofs. WASM publisher is executing; public/UI/STEP127 extension not yet verified. STEP124 remains confirmed only on prior e5d3059c artifact.


### 2026-10-07 — parameter spatial RMF WASM published; qualifications executing

Publisher completed successfully on geometry a50208948ee452eff93681f7d9417b2e2782af8daa90c85332224d8aab27a492,11579007bytes; packed/public hashes match. Native177 passed; types, schema export, script syntax and Vite completed. New public spatial/miter/retained-smoothness qualification is executing. Parameter periodic hollow UI wide/narrow actual-native-running cancellation is executing headlessly on requested Metal. STEP127 export is executing in external-step-parameter-spatial-rmf-current; it adds closed varying hollow, closed periodic varying hollow and open corrected filled-cap varying hollow parameter sources. These three native bodies are independently qualified above. No new public/UI/STEP127 success is yet claimed. Sole live handles:public52487,UI process (see terminal),STEP77548; publisher87070 terminal0. Prior STEP124 remains passed on e5d3059c, not relabelled to a5020894.


### 2026-10-07 — current parameter spatial RMF UI126 complete

On a50208948ee452eff93681f7d9417b2e2782af8daa90c85332224d8aab27a492 all three new bodies passed wide1440/narrow600 UI42each=126checks: closed varying hollow, closed periodic varying hollow, and open corrected filled-cap varying hollow. Reports under ui-parameter-{closed,periodic,open}-spatial-rmf-hollow-native-cancel-current. Both cases in every report have status passed, no pageErrors, actual Apple/metal-3, source/public/dist/worker provenance, unpaused actual-native build/Solid cancellation and Worker destruction. This is the finite new-mode UI scope, not the remaining all-mode UI matrix. Broad public qualification and STEP127 export are still executing. Current types/schema/script syntax/Vite/scoped whitespace are successful; prior STEP124 remains confirmed on e5d3059c.


### 2026-10-07 — current public53 complete; STEP127 export remains live

Current a50208948ee452eff93681f7d9417b2e2782af8daa90c85332224d8aab27a492 public suite completed successfully:5files/53tests,407.44s. Files: nurbsSpatialRmfError, nurbsMiterLawMatrix, nurbsClosedCombinedMiter, nurbsRetainedSweepSmoothness, miterStationSmoothness. This confirms parameter original transport, all three new retained bodies through Rush/viewport/fresh Solid, whole-work refusal and single-bit closure refusal, current16law miter family and applicable unequal-station retained G2 positive/negative checks after publication. Current UI126 also complete. Native177/types/schema/Vite/script syntax/scoped whitespace successful. Sole remaining active process this milestone: STEP127 export session77548, log spatial-rmf-parameter-step-export.log. It is confirmed live and must not be restarted on an observation timeout. Next sequential steps after export terminal0: reference-rational-boundary-volume.py, reference-sweep-generator-volume.py, verify-sweep-step-occt.py, all against external-step-parameter-spatial-rmf-current. No new STEP127 oracle result claimed. All goal-wide component coverage/global-all-mode/retained smoothness/miter/STEP/UI requirements remain in scope and goal stays active; finite qualification is not universal proof.


### 2026-10-07 — STEP127 completed; shared arc proof and global work in progress

OpenCascade completed all 127 cases with `passed=true` against published geometry WASM `a50208948ee452eff93681f7d9417b2e2782af8daa90c85332224d8aab27a492`. Evidence: `nonaxial-corrected-revalidation/spatial-rmf-parameter-occt.log`. The original export failed at V8 maximum string length; the retry retained every case and field using an atomic streaming manifest. Rational and generator reference stages also completed before OCCT.

Newer, unpublished Rust source shares the original closed spatial RMF arc transport and source arc division across contours. Four-contour continuous source proof completed with 62663 cells and bound 0.07323245566026543; one-under and zero-budget refusal remain enforced. The progressive native suite passed 178 tests before subsequent injectivity changes.

Actual three-disjoint-hole periodic spatial RMF bodies pass source error and regularity but their prior global certificates exhausted face injectivity work. Stable original-span jets plus exact station-position separation now certify a retained problematic wall within 71 cells. Seven focused injectivity tests pass, including folded geometry refusal and unchanged finite budgets. The full arc/parameter three-hole body regression is still running; no positive global certificate, Solid admission, publication or UI qualification is claimed for this newer source.

Remaining goal obligations include full all-mode continuous bounds and global correctness, applicable retained G1/G2 and closed seams, and complete combined miter coverage through Solid. Previously published public53/UI126 evidence applies only to the a5020894 artifact.


Final three-hole recheck: both arc and parameter regressions failed after 235.54 seconds. Exact boundary agreement and trims pass, but aggregate injectivity and pair proofs remain unproved; nesting and orientation certification therefore remain unavailable. The 71-cell local wall result does not establish the full body. No new WASM published. Evidence: `nonaxial-corrected-revalidation/spatial-rmf-three-holes-correlated-jets-native.log`.


### 2026-10-07 — whole retained three-hole chart injectivity proven

Saved actual arc-spacing three-hole model: all 1024 retained faces passed native injectivity with the unchanged shared 100000-cell budget, consuming 92949 cells and 1024 contraction spans. Previous candidate ordering exhausted the budget at face 642. Bounded midpoint proposal search now receives up to 512 remaining cells before original-candidate fallback; this reallocates existing work and never changes the full-domain proof or total budget. Focused native monotonicity regressions passed 7/7, including folded-chart and zero-budget refusal. Diagnostic evidence: `nonaxial-corrected-revalidation/spatial-rmf-three-holes-injectivity-work-512.log`; focused regressions: `spatial-rmf-injectivity-512-regressions.log`. Full arc/parameter body global recheck is currently running in `spatial-rmf-three-holes-global-512.log`; contacts, nesting, orientation, publication and Solid are not yet claimed.


### 2026-10-07 — three-hole complete global native proof passed

Both actual 1024-face periodic spatial RMF bodies, arc and parameter spacing, passed the unchanged complete global gate: volume, exact boundary agreement, trims, face injectivity, pair contacts, nesting and orientation. Parent/role assertions for outer and three inner shells remain mandatory. Continuous errors: arc 0.07326900336118376, parameter 0.12179534341497596. Evidence: `nonaxial-corrected-revalidation/spatial-rmf-three-holes-global-512.log`, 2/2 tests, 273.36 seconds. This proves these fixtures, not all modes.

Full nurbs-core native suite initially passed 1287 and failed 2. Fixed an actual unresolved fixed-normal value report retaining partial adaptive-cover bounds: all three vector values are now cleared on incomplete cover while preserving work count. Periodic-chart test now requires outward containment and a tight rounding envelope rather than exact equality of interval arithmetic. A full 1289-test rerun is active in `spatial-rmf-shared-arc-station-jets-full-nurbs-repaired.log`. New Rust source is still unpublished; public/WASM/UI/STEP verification against the new kernel remains required.


### 2026-10-07 — full native regression repaired and WASM publication active

Full nurbs-core rerun passed 1289/1289 in 152.56 seconds; evidence `nonaxial-corrected-revalidation/spatial-rmf-shared-arc-station-jets-full-nurbs-repaired.log`. Four face-injectivity BREP regressions passed; one explicit-fixture diagnostic remains ignored by ordinary runs and was separately executed successfully. Improved spatial-miter proof now succeeds within the former 10000-cell negative budget, so resource refusal is checked with a genuine one-cell budget instead; the full positive global-domain test is preserved. Evidence `spatial-rmf-face-injectivity-regressions-current.log`.

The sole geometry publisher is actively compiling newer Rust source; log `spatial-rmf-shared-arc-multiple-holes-wasm.log`. No new published hash or public acceptance is claimed before terminal successful publication. Prior STEP127 evidence remains attached to the older a5020894 kernel. Next required stages: new packed/public hash agreement, Rush/Solid public matrix including both multiple-hole fixtures, retained smoothness/miter regressions, headless actual-native-cancellation UI and independent STEP against the new artifact.


### 2026-10-07 — new kernel published; downstream qualification active

Sole publisher completed successfully. Public geometry WASM and generated packed identity agree on SHA-256 `480987fb002a312479473768cdb72584e6cc655f7116474621e60d8598c21e3e`, 11587271 bytes. Evidence `nonaxial-corrected-revalidation/spatial-rmf-shared-arc-multiple-holes-wasm.log` and generated identity. Vite distribution rebuild completed; types passed.

Active downstream runs: five-file public spatial/miter/retained-G1-G2 matrix (`spatial-rmf-three-holes-public-matrix-current.log`); independent STEP129 export (`spatial-rmf-three-holes-step129-export.log`); two headless actual-native-cancellation UI runs covering arc/parameter three-hole sources on wide/narrow windows (`spatial-rmf-three-holes-arc-ui-current.log`, `spatial-rmf-three-holes-parameter-ui-current.log`). None yet has a terminal positive result. STEP reference and OCCT stages must follow export sequentially. Prior STEP127 is older a5020894 evidence. The original complete library goal remains open.


### 2026-10-07 — UI84 passed; joint miter regression blocks STEP129

On published 480987fb002a312479473768cdb72584e6cc655f7116474621e60d8598c21e3e kernel, both three-hole spatial RMF bodies passed 42 headless actual-native-cancellation UI checks each, on 1440 and 600 widths (84 total). Logs `spatial-rmf-three-holes-{arc,parameter}-ui-current.log`; both processes terminal0. Full public five-file matrix remains active.

STEP129 exporter terminated1 before its manifest: open simultaneous affine/authored/guide miter failed retained-wall regularity at the unchanged 10000-cell budget. Previous seven native miter tests passed but did not include this exact combination. Extended the native complete-boundary regression to add the actual guide and reproduced failure: budget reaches face52 of64. More accurate midpoint integer proposal (64 instead of16) also failed, so reverted that experiment. No budget inflation or bypass. Saved-model diagnostic export is executing (`joint-miter-unproved-model-export.log`), preserving full refusal; subsequent native work must optimize proof search without losing three-hole92,949-cell success or old miter combination coverage. STEP independent references and OCCT have not started for129, and no success is claimed.


### 2026-10-07 — independent Jacobian enclosure intersection fixes native joint miter

Published480987fb full public matrix completed:49 passed,6 failed of55 across5files. One joint affine/authored/guide miter refused wall regularity; five earlier closed spatial RMF hollow bodies refused fresh boundary embedding. Retained smoothness suites passed, but the kernel is not fully qualified. No STEP129 reference/OCCT success claimed.

Rust original-span derivative enclosure can be wider than restrict-then-differentiate on corrected miter. Production surface_linear_monotonicity now intersects these two independent outward projected Jacobian enclosures after each is normalized by its own correct knot/query width; an empty intersection refuses. Fixed map, positive symmetric Jacobian and full-domain cover remain mandatory. No total budget increase or sampled acceptance.

Exact joint miter regression now passes within unchanged10000cells,1/1 in19.80seconds; eight monotonicity positive/negative/work/shifted-domain regressions pass. Saved actual1024face three-hole model proves all faces within47277 of100000cells (previous92949),1/1 in25.91seconds. Evidence `joint-miter-jacobian-intersection-native.log`, `jacobian-intersection-monotonicity-native.log`, `three-holes-jacobian-intersection-native.log`. The failed64precision experiment was reverted.

Active: full12spatial-body native qualification in `jacobian-intersection-all-spatial-bodies-native.log`, full1289NURBS regression in `jacobian-intersection-full-nurbs-native.log`. This latest correction is not yet published; no new public/UI/STEP success is attributed to it. Full goal remains active.


### 2026-10-07 — Jacobian intersection full NURBS regression passed

Full native nurbs-core suite passed1289/1289 in221.32seconds after dual outward Jacobian enclosure intersection. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-full-nurbs-native.log`. The12body spatial qualification remains live; confirmed complete global successes so far are closed512face tube, periodic512face tube and open514face corrected filled-cap body. No remaining-body success is claimed before terminal evidence. Publisher has not been restarted for this latest correction; published480987fb retains its six public refusals, and STEP129 remains incomplete.


### 2026-10-07 — all12 spatial bodies passed complete native gates

Full spatial_rmf_qualification completed12/12,404.21seconds. Includes previous five hollow sources that failed published480987fb fresh Solid, both three-hole bodies, constant/varying periodic/nonperiodic sources, both spacing policies and open corrected filled caps. Every fixture retains required boundary/global/nesting/orientation gates; no budgets or admission conditions relaxed. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-all-spatial-bodies-native.log`. Together with full1289NURBS and exact joint miter regression this verifies latest Jacobian intersection natively. The sole publisher is now active in `jacobian-intersection-wasm-publish.log`. Current public module remains480987fb until successful terminal publication; STEP129 and public55 must be revalidated afterward on the new artifact. Goal still active: this finite source set does not prove all-mode guarantees.


Latest native downstream regressions on Jacobian intersection: owned miter7/7 (38.10s), retained charts1/1 (2.58s), face injectivity4/4 (2.55s, explicit-fixture diagnostic ignored by ordinary runner and separately proven above). Logs `jacobian-intersection-{owned-miter,retained-chart,face}-regressions.log`. Sole publisher remains live compiling latest geometry; no new hash assigned before terminal publication.


### 2026-10-07 — Jacobian intersection WASM published; full UI matrices dispatched

Publisher terminal0: public and packed geometry SHA51327eec556f974ecbde1b8f9874a900b8ab038018e88e1bea30efb883e6bcbe,11588327bytes. Vite distribution rebuild terminal0. Native baseline remains1289NURBS,12fullspatialbodies,7ownedmiter,1retainedchart,4faceinjectivity regressions passed.

Active downstream qualification against51327eec: public55 in `jacobian-intersection-public55.log`; STEP129 export in `jacobian-intersection-step129-export.log`, directory `external-step-jacobian-intersection-current`; full existing body-boundary UI matrix and full existing miter UI matrix, each wide/narrow, headless Metal and actual-native-running cancellation, logs `jacobian-intersection-{body,miter}-ui-full.log`, reports `ui-jacobian-intersection-{body,miter}-full`. These runs have not completed. All selected sources and requirements remain mandatory; no narrowed positive subset substitutes for full matrices. Reference volume and OCCT stages must follow successful STEP export sequentially. Earlier480987fb public49/55 is not current51327qualification. Goal remains active.


### 2026-10-07 — current full-matrix partial progress

On51327eec, current partial UI reports confirm body2cases/42assertions and miter11cases/208assertions (250total), zero pageErrors in completed cases. Neither matrix has its final passed flag yet. Authoritative partial reports `ui-jacobian-intersection-{body,miter}-full/matrix.json`. STEP129 export30438 and public5590088 remain live; no observation timeout was treated as terminal or caused a restart. STEP log advanced to unsegmented miter affine/frame/guide modes but manifest and downstream independent stages remain pending.


### 2026-10-07 — current public55 fully passed

On published51327eec556f974ecbde1b8f9874a900b8ab038018e88e1bea30efb883e6bcbe, all55 public tests across5files passed,688.89seconds. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-public55.log`. Restores all six480987fb failures: exact joint open affine/authored/guide miter and five previous closed hollow spatial RMF body Solid gates. Scope includes all16 law selections, new three-hole arc/parameter bodies, full source-bound/budget refusal, original seam refusal and current applicable retained G1/G2 with unequal stations/moving frames/sharp C0 negatives.

Full UI matrices remain running; current partial counts: [('ui-jacobian-intersection-body-full', 7, 147, 0), ('ui-jacobian-intersection-miter-full', 24, 446, 0)]. Both runs must finish wide/narrow before a full UI success claim. STEP129 export30438 remains live; manifest/reference/OCCT stages pending. No old STEP result is promoted to51327eec. Full goal remains active.


### 2026-10-07 — full surface UI added without conflating cancellation scopes

Launched full existing surface UI matrix, headless requested Metal, on current51327eec. Log `nonaxial-corrected-revalidation/jacobian-intersection-surface-ui-full.log`, report `ui-jacobian-intersection-surface-full/matrix.json`. Surface matrix verifies actual rendered previews, separate patch/frame/retained smoothness evidence, mandatory surface-to-Solid refusal, lifecycle cancellation/source replacement/restoration on wide/narrow. It omits native-running-cancel flag because current explicit native probe requires retained body sources; actual native build/Solid cancellation continues in full body and miter matrices. Surface held-dispatch cancellation must not be promoted to mid-native proof.

Snapshot: body10cases/210checks, miter39cases/713checks, surface24cases/356checks,1279checks total. All matrices remain active, no final passed flag. STEP export30438 remains live with98STEP files and no manifest yet; independent reference/OCCT pending. Public55 already terminal0 on current kernel. Full original objective remains active.


### 2026-10-07 — STEP129 references passed; OCCT and full surface revalidation active

STEP129 export terminal0, manifest244546447bytes,129cases, verified51327eec556f974ecbde1b8f9874a900b8ab038018e88e1bea30efb883e6bcbe provenance. Both independent Python rational-boundary and retained-generator reference stages terminal0. Logs `jacobian-intersection-step129-{export,rational-reference,generator-reference}.log`. OCCT verification is live in `jacobian-intersection-step129-occt.log`; no oracle success claimed yet.

First full surface UI run terminated1 at closed-planar RMF full-turn strict-refusal assertion:1e-30mm failed earlier sampled refinement, not the required continuous gate. Kept the exact continuous-refusal regex; fixture now uses0.0006mm, measured sampled0.0005467112658295121 <0.0006 <continuous0.0006280154389414278 at257stations. Direct packaged-source probe confirmed0.0006refusal and0.001/0.002acceptance. Wide/narrow focused recheck passed30checks, preserving no final patch evidence after refusal and original-source restoration. No product geometry/admission/budget logic changed. Full surface matrix restarted only after the original handle was terminal, into separate `ui-jacobian-intersection-surface-full-revalidated`, log `jacobian-intersection-surface-ui-full-revalidated.log`; original failed evidence preserved. Full body/miter matrices remain live.

Scripts syntax and whitespace checks passed. All original component/global/smoothness/miter/STEP/UI requirements remain in scope; finite passing fixtures are not universal proof.


### 2026-10-07 — current STEP129 and full surface/miter UI completed

Current geometry SHA51327eec556f974ecbde1b8f9874a900b8ab038018e88e1bea30efb883e6bcbe: independent rational and generator references followed by OpenCascade completed terminal0,129/129cases passed (`external-step-jacobian-intersection-current/opencascade-sweep.json`). This is fixture import/topology/volume and the recorded surface/cap checks, not a universal all-mode theorem.

Full surface revalidation passed102/102cases; full miter UI passed96/96cases. Both final matrix.json reports record the current kernel, zero page errors, and Apple metal-3. Miter includes authenticated running native build/Solid cancellation; surface lifecycle cancellation has its separately recorded scope. Reports: `ui-jacobian-intersection-surface-full-revalidated`, `ui-jacobian-intersection-miter-full`.

Full body UI terminated1 at wide closed-authored-tube-body-boundary.r native-running cancellation: tiny error budget finished/refused before sampling enough native work (maximum1sample); cancellation is unqualified. Preserve failed report. A valid 65-station workload retaining2mm deviation was independently built via packaged asynchronous Rust resolver:256faces, certified fresh Solid. Prior synchronous probe's Missing native body was misuse of the synchronous resolver, not a geometry failure. Harness now selects this valid denser workload only for the named authored fixture; native profiler authentication, freshness, tail and minimum5samples remain unchanged. Focused wide/narrow cancellation recheck is active in `ui-jacobian-intersection-authored-native-cancel-recheck`; full body revalidation still required. No product geometry changes or WASM republish in this checkpoint.

Full goal remains active. After body UI, audit remaining general spatial RMF/holonomy, complete-bound combinations, all-mode global certificates, applicable retained smoothness and law combinations against original requirements. Passing finite matrices alone do not establish unrestricted guarantees.


Authored cancellation follow-up: focused wide/narrow recheck terminal0,2/2cases and42checks passed, including authenticated running native build/Solid cancellation, restoration and actual Metal rendering. Evidence `ui-jacobian-intersection-authored-native-cancel-recheck/matrix.json`. Full body matrix revalidation launched into separate `ui-jacobian-intersection-body-full-revalidated`, log `jacobian-intersection-body-ui-full-revalidated.log`; not yet complete. Original failed body report is retained.


2026-10-07 completion audit: `completion-traceability-2026-10-07.md` records inspected proof owners and remaining gates without narrowing the objective. Exact16 open/closed affine/authored/guide rendered UI matrix launched in `ui-jacobian-intersection-law-matrix`; generated sources share the existing public/STEP fixture generator. Full body revalidation remains live.


### 2026-10-07 — exact law STEP crosswalk and global validity audit

Verified all16 canonical open/closed affine/authored/guide request sources byte-for-byte between generated rendered-UI fixtures and current STEP129 exported sources. Each matching OpenCascade case passed native-volume certification, imported validity and shell orientation; maximum relative volume error2.3425513395226053e-10. Evidence `miter-law-step-crosswalk-current.json`. Exact32case UI matrix remains running and is not replaced by STEP/public success.

Targeted current-source native `volume_validity::tests` suite launched, log `nonaxial-corrected-revalidation/jacobian-intersection-volume-validity-native.log`. Covers boundary-before-nesting/material gates, invalid/exhausted limits, corrected affine inflection and zero-curvature modes, hollow periodic miter, preserved inward cavities and nested partitioned boundaries. Running result is not yet final. Full body UI and exact law UI remain live. Completion remains unproven; all original requirements retained.


Global validity follow-up: current-source volume_validity::tests terminal0,10/10passed,71.41seconds. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-volume-validity-native.log`. All original limits and negative boundary/nesting/orientation admission gates retained. Boundary embedding prerequisite suite now launched in `jacobian-intersection-boundary-embedding-native.log`; result pending. Both UI matrices continue, snapshot3body and11exact-law cases complete, not full success.

Boundary prerequisite follow-up: boundary_embedding::tests terminal0,4/4passed,0.41seconds. Exact contacts require all joint prerequisites; tolerance-accepted gaps and unsupported chart shortcuts remain unproved. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-boundary-embedding-native.log`.


### 2026-10-07 — full boundary constructor/transport audit revalidated

Inspected `analytic/rational_loft.rs` complete construction: actual source sections (including affine/authored/guide/RMF policy) precede bounded endpoint correction; actual model/retained partition precede exact cap regions and original-domain projection; original retained wall error plus constructor decomposition and correction precede complete wall/cap union. Closed sweeps carry no artificial cap obligation. Component-only retained-cap transport flags remain explicitly false for complete/global claims. Bridge output publishes separate boundary completeness/tolerance and never promotes it to global embedding.

Current native full `analytic::rational_loft::` suite terminal0,21/21passed,14.03seconds: rational extraction, variable-basis/partial refusal, curved frame modes, fixed-normal, nonaxial planar RMF, guided/contact/Frenet hollow actual-volume admission, exhausted cap correction, exact retained edges and closed full-turn no-cap body. Evidence `nonaxial-corrected-revalidation/jacobian-intersection-rational-loft-boundary-native.log`.

Current published-WASM public complete-boundary suites terminal0,3files/9tests passed,31.75seconds: `sweepBoundaryCertificate`, `nurbsProgressiveSweepCapProjection`, `nurbsSweepCapBoundaryAudit`; evidence `jacobian-intersection-complete-boundary-public.log`. Native bridge retained-cap integration terminal0,2/2passed,1.03seconds; evidence `jacobian-intersection-retained-caps-bridge.log`. Covers actual ownership, source modes, source extraction displacement, holes and no-promoted-guarantees/refusals. No product changes or republish; current geometry remains51327eec.

Both full body and exact-law rendered UI matrices remain active; finite current fixtures do not alone establish unrestricted all-mode completion.


### 2026-10-07 — actual retained smoothness and reconstruction audit

Inspected `sweep_smoothness.rs::inspect_axis`: actual wall pcurve boundary ownership selects actual surface seam pairs; unclassified faces or unpaired edges invalidate extraction; station speed scale is proposed then exact projective whole-seam G2 is audited; G1 fallback uses only the remaining shared work. Cap-owned edges remain separate. Current native smoothness4/4passed,0.76seconds (`jacobian-intersection-sweep-smoothness-native.log`): G1 versus G2, malformed topology, shared-work refusal, all closed hollow sharp stations/closure stay C0.

Inspected `sweep_station_reconstruction.rs::reconstruct`: source sections must exactly reproduce the certified source body; bounded reconstructed wall displacement is mandatory; candidate actual caps must be preserved. Current6/6native reconstruction tests passed,6.18seconds (`jacobian-intersection-station-reconstruction-native.log`), including applicable closed periodic seam, multisegment hollow profiles and general nonuniform rational retained G2, plus source/budget/actual-jet refusal. Existing current packaged public55 remains the WASM/moving/unequal-step qualification. These proofs do not promote a source-frame certificate or an actual sharp C0 seam to G1/G2.

Current native `geometry-bridge --lib cad_miter_owned::` suite is running (`jacobian-intersection-owned-miter-bridge-native.log`), covering owned source-only protocol, evidence injection rejection, transported actual poles, corrected/placed/reconstructed boundary/model binding and fresh material admission. Both browser matrices remain active. No product geometry changes or republish in this audit.


### 2026-10-07 — exact law UI complete; rational nonuniform RMF coverage extended

Exact canonical16law sources × wide/narrow passed32/32cases and576assertions. Current51327eec, all Apple metal-3, zero page errors, no missing/duplicate source/viewport pairs. Report `ui-jacobian-intersection-law-matrix/matrix.json`. Each case includes authenticated running native build/Solid cancellation and source replacement/restoration, successful Solid and bounded-work/invalid-path refusals. Combined with byte-identical16case STEP crosswalk and public16 law tests this closes the explicit Boolean miter law-selection matrix through native/WASM/Rush/rendered viewport/Solid for this qualification scope.

Current owned-miter native bridge9/9passed,103.88seconds (`jacobian-intersection-owned-miter-bridge-native.log`): source-only protocol/injected-evidence refusal, periodic/general/nonuniform reconstructed geometry, exact final model binding and fresh material proof after bounded placement/shear/reflection. Do not conflate this with all unrestricted geometry.

Added a Rust regression for an original closed rational spatial path with interior weights1.25, four unequal dyadic spans and offset-adjusted exact original C1. Both parameter and arc-length retained patch bounds with affine laws are certified; whole original transport has4096cells, work exhaustion discards all partial covers/holonomy, and a one-bit weight change loses C1 certification. Native1/1passed,7.72seconds; current packaged WASM2/2newfocused tests passed,1.12seconds test execution,16existing tests explicitly skipped in this focused run. Logs `rational-nonuniform-closed-rmf-{native,public}.log`. Changes are test-only; published production kernel remains51327eec and was not rebuilt.

New rational nonuniform hollow-body fixture is under qualification (`closed-rational-nonuniform-spatial-rmf-affine-hollow-body.r`). Initial0.0001mm arc residual consumed52331shared cells and left original closed C1 exact work unproved; native preview correctly refused full admission. Recheck uses the same0.001mm arc residual policy as the successful native/WASM source fixture, retaining0.25mm requested full error and original shared100000/1000000 limits. Fresh body/global Solid recheck remains active in `rational-nonuniform-closed-rmf-body-public-recheck.log`; no positive body claim yet. Original failed probe and preview diagnostic retained. Full existing body UI revalidation remains live.


Rational hollow-body follow-up:0.001mm residual recheck also terminated1 with continuous retained-patch refusal; native preview reason remains `rmf-original-closed-C1-unproved`,50784chargedcells, no complete/endpoint/decomposition bound. Evidence `rational-nonuniform-closed-rmf-body-preview-recheck-diagnostic.log` and `rational-nonuniform-closed-rmf-body-public-recheck.log`. This is not qualified positive body evidence.

Inspected `MultiSweep::preview_at` shared source allocation: original arc division is charged first, then the remaining shared allowance is split50/50 between exact knot/closure work and interval transport. Single-profile arc source certifies RMF before charging original division. Thus the new unequal rational source demonstrates a source-work allocation asymmetry across single/multi ownership. Next work: measure exact-stage usage and implement a dynamically shared Rust owner so unused interval allowance can support necessary original C1 work, without increasing total cells/products or performing work beyond the owner. All original gates stay mandatory. New fixture remains pending, not substituted with a planar/polynomial source.


### 2026-10-07 — dynamic shared RMF source owner implemented in Rust

Added `certify_original_rmf_transport_shared`: exact original knot/closure premises use the aggregate allowance first; only their actual expenditure is subtracted from the interval-cell allowance. Every subsequent jet, rotation, closing-angle and holonomy-correction cell stays under the remainder. Existing independent-stage public source API is unchanged. MultiSweep shared transport and single arc/parameter source owners now use the same aggregate policy instead of arbitrary50/50 reservations. No increased total cells/products, changed geometry, omitted gate or certified partial cover.

Focused native regression terminal0,1/1passed,28.18seconds (`rational-nonuniform-rmf-shared-owner-native.log`). Complete shared original transport certifies at its exact measured total budget; zero and one-unit-short refuse with no partial cover/holonomy and expenditures never exceed the owner. Two actual circular contours of the unequal rational closed path now have complete retained source bound0.3215932367577691mm in86856/100000cells. Requested0.25mm remains explicitly rejected while retaining the proved bound; separately requested0.4mm accepts. New example explicitly uses0.4mm, with prior0.25refusals preserved in logs and regression. This is a caller tolerance distinction, not weakened internal work/geometry admission.

Full current1290NURBS native regression launched in `rmf-shared-owner-full-nurbs-native.log`; must finish before publishing the production change. Public/packed/dist kernel still51327eec; its earlier129STEP and full surface/miter/exact-law UI evidence are not evidence for the unpublished owner. Old full-body UI72917 remains live on its immutable51327distribution. New rational hollow-body/global Solid, rebuilt WASM/Rush/rendered UI and independent STEP qualification remain required. Goal stays active.


### 2026-10-07 — shared-owner full native baseline and rational hollow Solid

Full current NURBS suite terminal0,1290/1290passed,143.70seconds, evidence `rmf-shared-owner-full-nurbs-native.log`. New rational unequal-span hollow body native terminal0,1/1passed,71.30seconds (`rational-nonuniform-rmf-shared-owner-body-native.log`):512faces, complete boundary0.3215932367577691mm, retained regularity, all pairs, actual shell nesting and outward/inward material orientations. Requested0.4mm;0.25mm strict refusal remains separately covered. Original work/face/global gates retained. Full updated13-case spatial body native matrix is running in `rmf-shared-owner-all-spatial-bodies-native.log` before WASM publication. Current types passed. Production WASM remains51327eec.

Prepared public/Rush/fresh-Solid regression for the new rational hollow source, including exact two-shell roles/orientation, unchanged document,0.25mm error refusal and zero-work refusal; pending rebuilt WASM. Existing STEP exporter includes the rational nonuniform source as case130, preserving all original129cases and both independent reference gates. Added source to complete body UI selection, now45sources × two viewports; not yet run on new artifact.

Old51327full-body UI revalidation terminated1 at authored-multispan-body-boundary.r, after31successful widecases. Actual native samples were observed but the latest profiler tail was idle; cancellation remains unqualified. Failed evidence preserved in `ui-jacobian-intersection-body-full-revalidated`. Valid65station multispan request independently passed old-packaged build and fresh Solid with258faces; log `authored-multispan-valid-native-cancel-workload.log`. Harness selects this valid workload only for the named fixture; minimum5authenticated fresh tail samples and actual Worker destruction unchanged. Focused wide/narrow actual-native cancellation recheck running in `ui-authored-multispan-valid-native-cancel-recheck`. No full UI or published-new-kernel claim.

Authored multispan cancellation follow-up terminal0: focused wide/narrow2/2cases,42checks passed on51327eec and Apple metal-3, including authenticated running native build/Solid cancellation, source replacement and restoration. Evidence `ui-authored-multispan-valid-native-cancel-recheck/matrix.json`. This qualifies the denser workload for the next full-body matrix; failed broad old report remains preserved. New shared-owner full13body regression still active; no new WASM publication.


Shared-owner full spatial matrix follow-up terminal0:13/13native complete-body cases passed,319.18seconds (`rmf-shared-owner-all-spatial-bodies-native.log`), including all original12 and new rational unequal-span hollow source. Whole boundary, actual retained regularity, all-pair/global embedding, shell roles and material orientations remain mandatory. Current1290NURBS and types also passed. Sole WASM publisher now launched in `rmf-shared-owner-wasm-publish.log`; source/public/packed artifact identity is not yet updated/qualified until successful terminal publication. Next: new-artifact public58, STEP130 sequential reference/OCCT stages, full updated body90/surface102/miter96/exact-law32UI matrices. Previous51327qualification remains historical and must not be relabelled as current new-owner qualification.

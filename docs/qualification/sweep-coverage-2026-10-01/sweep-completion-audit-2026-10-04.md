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

`tests/exactSolidWorker.test.ts` + `tests/modelGraphText.test.ts`: 32/32 passed on intermediate patch-error WASM (`/tmp/sweep-authored-patch-intermediate-solid.log`). Includes real authored-cap Solid construction, validation, exhausted correction refusal and selected periodic/frame-guide-affine hole cases. This is selected regression coverage, not all-mode or full UI qualification. Updated current E and STEP status cells in `docs/design/sweep-contract-matrix.md` to distinguish retained-patch proof, selected complete miter boundary examples, prior-artifact 38-case STEP evidence and pending current-artifact qualification.

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

# Exact periodic original-domain qualification

Rust exact blossom extraction now accepts periodic originals only when all
active-span controls are exactly representable and the active endpoint points
agree exactly. Periodic input validation's exterior-knot tolerance is not the
seam proof. A valid almost-periodic exterior-knot perturbation refuses this
identity route. Individual decomposition bounds cover periodic active spans;
they do not establish closure or topology on their own.

Rust section loft now decomposes periodic input curves while retaining bounded
3D input, span correspondence, exact joins and cap orientation audit. Native
tests cover a valid periodic hollow body and refusal for a broken join or wrong
hole orientation. Rust Rush compilation exposes optional `periodic` on
`nurbs_curve`, default false, preserving it through canonical compilation.

Evidence:

- Native exact extraction 2/2, decomposition 3/3, sweep 153/153, rational loft
  9/9, Rush frontend 9/9 and runtime 56/56 passed.
- Public geometry/Rush suites passed 44/44; the full constructor and Rush source
  `examples/rush/miter-periodic-hollow.r` certify `continuousBound=true`, within
  budget, including retained wall and filled-cap decomposition.
- Additional periodic profile-smoothness regression passed in the four-test
  retained-decomposition suite: eight complete profile seams have exact G2
  (implying G1). Work is 3464; a budget of 3463 refuses G2. This does not certify
  smooth cap transitions or moving-frame/station seams.
- Fresh `external-step-periodic` matrix passed independent OCCT 36/36, with
  520 whole-domain cap coedge checks. Its manifest binds the tested artifact.

Geometry WASM SHA256:
`6cfe87add9229b03991147ada749311bd52c74ce94f4a4ceab47619d31f8764e`,
10,743,643 bytes; generated/public/packed copies agree.

The new periodic STEP case is a valid Solid with one closed shell, 10 faces,
24 edges, same-parameter agreement and opposite manifold edge uses. Whole-domain
wall coefficient/pcurve and cap-coedge gates pass. Native material roles,
orientation, face injectivity, pair classification, boundary embedding and Solid
certificates are true. OCCT volume is 15.625000000000002 mm³ against independent
125/8 mm³, relative error 1.1368683772161603e-16. The reference derives from four
quadratic sectors of area 5/12, a quarter-scale hole and 10 mm extrusion.

Authoritative files: `external-step-periodic/manifest.json`,
`external-step-periodic/opencascade-sweep.json`, `sweep-periodic-step-export.log`,
`sweep-periodic-step-occt.log`, `sweep-periodic-section-loft-public.log`,
`sweep-periodic-profile-smoothness-public.log` and the native logs with matching
prefixes. Finite OCCT material-side probes remain sampled checks.

Periodic UI qualification, arbitrary rounded original-profile topology and the
original all-mode/global coverage remain open. This fixture closes a sufficient
exact periodic path, not every periodic geometry or the whole library goal.

## UI follow-up, 2026-10-04

Wide-window checks confirmed rejection of zero scale, successful rebuild after
restoring the periodic source, source preservation in the group editor, and
Solid volume 15.6250 mm³. The temporary group was removed using Undo.
Evidence: `sweep-periodic-refusal-ui.jpg`, `sweep-periodic-solid-ui.jpg`.

At 720×900 the Solid dock is hidden and the wrapped File trigger previously
placed its right-aligned menu outside the left viewport boundary. The narrow
menu now anchors left and exposes the existing New group from source workflow.
Actual browser checks confirmed the full menu is visible, opening the group
editor with periodic source works, and cancelling restores the original source
without constructing a group. Evidence: `sweep-narrow-file-menu.jpg`,
`sweep-narrow-group-editor.jpg`. Direct vue-tsc and scoped diff checks passed.
The temporary viewport override was reset and the test tab closed.

Narrow periodic build success/refusal and cancellation during active work are
still pending; cancelling the editor is not evidence of active-build cancellation.

### Narrow build follow-up

At 720×900, zero scale was rejected with the native scalar-law message. Replacing
the same editor source with the valid periodic fixture built a selected Solid
in group `Sweep periodic narrow`; isolation and ISO fit displayed the body.
Evidence: `sweep-periodic-narrow-refusal.jpg`, `sweep-periodic-narrow-solid.jpg`.
Undo removed the temporary group. Reload at the restored default viewport
confirmed scene count 1, only group `model`, original `Body 1` B-rep, no selection,
and hidden source panel. The temporary tab was closed.

An attempted dense-tessellation cancellation check used 128 then 64 subdivisions.
128 was rejected by schema bounds. The 64 attempt showed the disabled busy build
button before Cancel; no group was published and reload restored the original
scene. Because the earlier schema error remained visible after Cancel, this is
not accepted as proof of cancellation during a valid long geometry operation.
That check remains open and needs an explicitly schema-valid workload.

### Cancellation workload validation

The authoritative `brep_tessellate` schema permits 1..32 segments. A fresh real
`runExactSolidRequest` on the periodic hollow fixture succeeds at 8 and 16 and
fails at 32 with `Invalid body mesh`; the 32 failure is independently reproduced
outside the browser. Its exact validation cause remains to be investigated.
The earlier 64/128 attempts are outside the schema and cannot prove cancellation.

`tests/exactSolidWorker.test.ts` now tests the 16-segment fixture through real
construction, worker response handling and receiving-document validation,
checking one body with ten B-rep faces. All 13 tests passed (3.44 seconds total).
This proves the workload is accepted, not cancellation while its native geometry
is executing. The browser 32-segment attempt completed with a mesh error before
Cancel, so that attempt also is not accepted as cancellation evidence. Scene
count 1 and group `model` were restored after reload, the viewport reset and the
temporary tab closed. UI active cancellation on the accepted workload remains open.

### Supported 32-segment Solid admission fixed

Inspection before document parsing proved the 32-segment mesh has 152,748 finite
coordinate components, 50,916 valid index entries and ten B-rep faces. The former
150,000-component per-body cache budget caused the rejection; geometry was valid.
The shared document limit now permits 300,000 components per positions/indices
array (100,000 vertices / triangles), retaining the existing finite, divisibility,
index-range, B-rep and 64 MiB document checks. This is a bounded cache expansion,
not a geometric approximation or change to the Rust sweep.

Real worker / receiving-document regressions now pass at both 16 and 32 segments.
The worker, async parser, normalization and direct-modeling suites passed 43 tests
before adding the two additional over-budget parser regressions. Direct vue-tsc
passed. The new regressions require both sync and async parsers to reject each
array above the shared limit; the updated normalization suite passed 11/11,
including both new regressions. UI cancellation remains to be verified separately.

### Active Solid cancellation and retry, narrow and wide

After the cache-budget fix, the browser used the same accepted periodic fixture
with 32 segments. At 720×900 the build button was observed disabled/busy before
Cancel was clicked in the same automation call. The editor closed, the original
source remained, there was no error and no cancellation group appeared. A new
build of the identical source without Cancel then completed successfully; the
Solid scene contained exactly the original model and one retry group, with no
late cancellation group. The selected retry body is shown in
`sweep-periodic-narrow-retry32.jpg`.

After Undo of the retry group and returning to the default wide viewport, the
same busy-state-before-Cancel sequence was confirmed again. This qualifies
active Solid request cancellation in both layouts and successful retry in the
narrow layout. It does not locate the cancellation inside a particular native
Rust instruction or replace the separate viewport-build cancellation matrix.
The source panel was hidden and the scene reloaded for final restoration.
The final restored accessibility state confirmed one scene item, only group
`model`, original `Body 1` B-rep and no selection. The test tab was closed with
the viewport override reset.

### Periodic wall-loop ownership premise hardened

The Rust `inspect_wall_geometry_with_loops` previously skipped endpoint closure
for a loop containing one periodic curve. That flag alone is insufficient:
periodic input validation allows rounded exterior-knot translations. The shortcut
is removed, so every loop passes the exact active-domain endpoint closure gate.
The regression compares the exact periodic fixture with a valid near-periodic
fixture whose exterior knot is moved by one binary64 step; exact closure passes,
while the unresolved case refuses before downstream wall analysis. Zero budgets
isolate endpoint ownership and do not assert wall injectivity or pair separation.

Native exact extraction tests passed 2/2 and all sweep tests passed 153/153.
WASM packaging was started with output in
`sweep-periodic-wall-closure-wasm.log`; browser qualification of this source change
is pending until the new artifact is packaged and checked.

The public adapter regression `requires exact periodic active-domain closure
before wall-loop analysis` was added to `tests/nurbsProgressiveMiter.test.ts`.
Before packaging the new WASM, it fails specifically because the near-periodic
case does not throw the required closure refusal. The exact case passes its
ownership gate. This demonstrates the old public artifact's shortcut rather than
only asserting a native implementation detail. Vue type checking passed.
The Rust release build finished in 1m11s; the optimizer was observed live at
100% CPU before packaging. Public green verification is still pending.

The build subsequently completed successfully: 10,743,557 bytes, public SHA256
`675feaabbe364be7c4dcf9e57934ef960999010d316af316877df9c85e3a8099`.
All five public suites passed 45/45 in 49.31 seconds, including the previously
failing periodic closure regression, periodic/unclamped complete boundary
constructor gates and retained wall/cap decomposition tests. Evidence:
`sweep-periodic-wall-closure-public.log`. Fresh STEP export on this artifact was
started in `external-step-periodic-wall-closure`; independent OCCT verification
remains pending. The historical 36-case report is not reassigned to this artifact.

Fresh export and independent OCCT verification subsequently passed 36/36 cases
on the new artifact, with 520 full-domain cap coedge checks. Manifest provenance
binds SHA256 `675feaabbe364be7c4dcf9e57934ef960999010d316af316877df9c85e3a8099`
and verifies the packed/public bytes. The periodic Solid again has ten faces,
24 edges, one closed shell, two hole caps and volume 15.625000000000002 mm³
against the independent 125/8 reference. All declared selected-fixture gates
passed; material-side probes remain sampled and all-mode guarantees remain open.
Evidence: `external-step-periodic-wall-closure/manifest.json`,
`external-step-periodic-wall-closure/opencascade-sweep.json`,
`sweep-periodic-wall-closure-step-export.log`,
`sweep-periodic-wall-closure-step-occt.log`.

### Periodic profile with joint frame, guide and affine laws

`examples/rush/miter-periodic-frame-guide-affine-hollow.r` combines the exact
periodic hollow profile with independently parameterized authored axis/normal,
orientation rail, positive transverse affine scale and center offset. The axes
and rail are constant relative to the straight path; this fixture does not claim
general moving-frame transport. Rush preserves both periodic profile flags and
the constructor reports authored frames, guide and affine laws applied, with
complete boundary `continuousBound=true` within 0.001 mm.

The public regression also binds that evidence into the displayed native artifact
and checks source graph immutability. Real worker construction and receiving Solid
document validation produce one body with ten B-rep faces. Both selected suites
passed 20/20, and vue-tsc passed; see `sweep-periodic-joint-public.log`.

The STEP exporter now requires all three mode flags and a complete boundary gate
for this new case. Its independent reference is 125/4 mm³: original volume 125/8
times the constant transverse area multiplier 2, unaffected by translation or
constant frame/guide. Fresh 37-case export is in flight at
`external-step-periodic-joint`; independent OCCT and actual UI checks for the new
joint periodic case remain pending.

The expanded STEP matrix subsequently passed 37/37 independent OCCT cases with
536 whole-domain cap coedge checks, bound to the same verified WASM artifact.
The joint periodic Solid volume is 31.250000000000004 mm³ against independent
reference 31.25 mm³. Evidence: `external-step-periodic-joint/manifest.json`,
`external-step-periodic-joint/opencascade-sweep.json`,
`sweep-periodic-joint-step-export.log`, `sweep-periodic-joint-step-occt.log`.

Three public source mutations additionally verify refusal of a collapsed
authored longitudinal axis, a guide parallel to the path and zero affine scale.
Assertions check the native reason, not an arbitrary exception, and source graph
immutability. The updated retained-decomposition and real-worker suites passed
23/23 in 9.27 seconds. Actual wide/narrow UI qualification of this joint periodic
fixture remains open; the earlier plain-periodic UI matrix does not cover it.

### Joint periodic UI success/refusal and rebuild

The actual browser now qualifies this fixture in the default wide layout and
720×900. Zero affine scale produces the native positive-scale refusal. Correcting
the same editor source succeeds; wide Solid mass properties report 31.2500 mm³
and center [0.125, 0.250, 5.000]. These UI mass properties are numerical and do
not independently certify closure. Native/OCCT closure evidence remains separate.

Reopening the group confirms byte-for-byte source preservation. At 720×900,
mutating scale to zero refuses; restoring the valid source successfully rebuilds
the existing group without adding a duplicate (scene count remains 2, including
the original model). Screenshots: `sweep-periodic-joint-wide-refusal.jpg`,
`sweep-periodic-joint-wide-solid.jpg`, `sweep-periodic-joint-narrow-refusal.jpg`,
`sweep-periodic-joint-narrow-solid.jpg`.

Undo of the rebuild followed by Undo of group creation restores scene count 1.
The temporary viewport override was reset and the source panel hidden before
reload. Active cancellation and viewport-build/source-change checks for this
joint periodic fixture are still separate open UI obligations.

A fresh background browser tab subsequently completed persisted recovery: scene
count 1, only group `model` and `Body 1` B-rep, no selected bodies and no joint
test group. Screenshot: `sweep-periodic-joint-restored.jpg`. The temporary tab
was closed after this observation. This proves persisted restoration for this
fixture; it does not close the remaining cross-mode UI obligations.

# Full Round: source-bound reference and implementation gap

Investigation date: 2026-11-01. Full native-library parity remains open.
This record describes static observations, not execution or numerical parity.

## Reference identity

Plasticity Beta 26.2.0-beta.39, installed at `/Applications/plasticity-beta.app`.
Selected ARM64 bridge: `Contents/Resources/app.asar.unpacked/.webpack/renderer/pk.node`.
SHA-256: `5f00e0c6bcf0497e2e27674629492aeb37228c06dfad1ddda455e221c1ca7f1d`.
Provider: Ghidra 12.1.4. Analysis-profile digest:
`066f9d6c56d7a436e2061afaa7f68aa9085572cd83e3172cf52ecd30a6101bae`.

## Observed static chain

| Observation | Evidence |
| --- | --- |
| Argument diagnostic for `left` in `ShellBody::FullRound` at `0x17027cc` has incoming reference `0x1121b24` | `ev_45267b229ff684b3d453f090b4736299dfcfd80c4ac5f16d94d49c693d01a18a` |
| Ghidra assigns that instruction to function `0x11218d8`; its dossier contains the left/right/center argument checks and a call to `0x1100f60` | `ev_1f5d91000627dbc8294f2eb5afea31e87a8e4a1b4b8b867c1157e81759cd2a75` |
| Callees of `0x1100f60` include `_PK_FACE_make_3_face_blend`, its result-release function, and topology-tracking result release | `ev_a3cb29966d5b8470b27664a01ebf5ecbc014d09545784b072626c577bba8d4a0`; callee view `ev_040bb725a7b74085ac0c4ed00b8d82af2d60a57c3c12732718143ccd59792efd` |

The native implementation therefore delegates this operation to the Parasolid
three-face-blend entry point. This is stronger than an import-name match.
It does not establish all accepted geometries or the algorithm inside Parasolid.
One unresolved call remains in the argument-wrapper dossier; the native callee
dossier reports zero unresolved calls but truncated value-flow observations.
No whole-program completeness or runtime behavior is inferred.

A separate pretty-function string names three `FaceCollection` arguments.
That string alone does not prove that multiple faces in each collection are
accepted, or that arbitrary curved supports succeed.

The portable 23-record Evidence bundle was exported outside the source tree:
`/Users/themoretheless/.codex/research/plasticity-beta-26.2.0-beta.39/fullround-static-trace.json`.
No foreign implementation or decompilation is included in this repository.

## Current independent implementation

`crates/brep-core/src/feature_ops.rs::full_round` takes `[usize; 3]`.
Its cuboid branch admits one orthogonal cuboid in an arbitrary rigid frame: eight vertices,
twelve edges, six faces, one shell and one body, with three distinct selected
faces. It checks straight edges, planar supports and absence of holes.
The selected planar normals define an orthonormal local frame; corner and bounding-plane checks run in that frame, with coordinates relative to a source vertex. The side-face separation fixes the radius. It constructs an independent
rational half-cylinder and audits the resulting solid.

`crates/brep-core/tests/feature_ops.rs` contains analytical-cylinder/volume,
source-preservation, invalid-selection, insufficient-depth, translated-frame
and both-middle-normal cases. These are existing source tests, inspected here;
they were not rerun during the initial static reference investigation; subsequent implementation runs are recorded below.

## Remaining work and required evidence

### Runtime access observation, 2026-11-01

Launching the installed app through the desktop app interface reaches the
`Plasticity License` window, not the modeling workspace. Its visible error is:
`Bad request. /meta/scope/fingerprints/0 type mismatch (received integer expected string)`;
the displayed error code is `N/A`. Therefore no reference modeling input was
submitted, no Full Round operation executed and no comparison outcome exists.
The UI offers account creation and license-token entry. No credentials were
entered, account created, purchase made or license mechanism modified.

REA fuse evidence
`ev_fbedca18312c53df112c111d3466cd07ab6de5d9a4ac2a97a0125eded92e7b65`
observes disabled `RunAsNode`, `EnableNodeOptionsEnvironmentVariable` and
`EnableNodeCliInspectArguments`, plus enabled ASAR integrity validation and
`OnlyLoadAppFromAsar`. These facts rule out the ordinary Node CLI Inspector
route for this packaged build; they do not prove absence of every renderer
debugging route. Runtime reference comparisons await working licensed access.
Independent Rust implementation and static investigation can continue.

1. Record reference execution on identical saved inputs: rotated cuboid,
   non-box planar body, curved supports and multiple selected faces per group.
   Record both success and refusal, selected face identities and resulting
   topology/geometry. None of these reference outcomes is established yet.
2. Implement independent generalized three-support contact construction,
   support extension, trimming and topology replacement according to the
   confirmed contracts. Merely accepting collections would not close this gap.
3. Validate tangency over complete supports, remaining-face preservation,
   closed/manifold material boundaries and self-intersection handling.
4. Compare reference and own geometry/topology under declared tolerances.
   Own tests and matching operation names cannot prove this comparison.

The broader fillet/chamfer, Boolean, offset/replace, quad, constraints, history
and exchange requirements remain in `plasticity-remaining-2026-11-02.md`.
This finding neither completes that plan nor narrows it to Full Round.

## Independent implementation follow-up

The world-axis restriction was removed from `full_round`; rotated and reflected
orthogonal cuboids use the same geometric construction. This is independent
implementation progress, not an observed Plasticity comparison. General
three-support blends and grouped selections outside the extrusion domain below remain open.
Dedicated regression source: `crates/brep-core/tests/full_round_frame.rs`,
covering geometric covariance of rational carrier control points, radius,
solid audit, immutable inputs and refusal of a sheared cuboid.

Validation: `cargo test --manifest-path crates/Cargo.toml -p brep-core
--test full_round_frame --test feature_ops full_round` passed all four selected
tests (two prior Full Round tests and two new frame tests). Test profile used
opt-level 2 with opt-level 1 overrides for `nurbs-core` and `brep-core`, debug
assertions and overflow checks enabled, isolated target directory
`/tmp/rea-r53-test-opt2`. Frozen clean verifier passed: 11 files unchanged.
This focused run does not qualify the entire concurrently edited worktree.

### Retained profile extrusion branch

`feature_ops/full_round_prism.rs` extends the standard `full_round` dispatch
to recognized XY profile extrusions beyond cuboids. One body and three distinct
planar vertical selected sides are required. The opposite supports and middle
must be consecutive straight segments of the outer profile. The radius is
fixed by their separation; two rational quarter arcs replace the end, and the
neighboring support segments are shortened to their contact lines.

Other original rational profile curves and through-hole profiles are cloned,
with complete positive-weight control-hull bounds proving that they stay
behind the replacement strip. An intrusion or unresolved recognition refuses
atomically. The owner prism constructor rebuilds cap trims and a closed body;
the result must pass validation and have `audit.ok`. The same explicit audit
admission is now applied to the cuboid branch.

Regression source `tests/full_round_prism.rs` covers a nonrectangular rear
with a through-hole and analytical volume, reversed support selection,
retained rational rear arcs and analytical volume, and rejection of a rear
notch intruding into the round strip. This branch is not a general rotated
freeform extrusion recognizer; arbitrary support surfaces, multiple selected
curved faces per group, strip interference resolution and general replacement
topology remain open. Runtime comparison with Plasticity is still unavailable.

Final regression run passed all 9 tests across `feature_ops`,
`full_round_frame` and `full_round_prism`, including reblend/pattern regression
tests. Source digests, test log and the unrelated existing architecture-gate
diagnostic are saved in `docs/qualification/full-round-prism-2026-11-01/`.
All four changed implementation/test source files are below 800 lines.

### Bind face selection to actual topology

The extrusion branch now verifies that each of the three selected original
faces owns the corresponding complete top-rim profile edge. Coplanarity alone
is insufficient: a rear face and an adjoining front face can occupy the same
support plane after a side split. Previously the rear face could be selected
while the adjoining front segment was actually trimmed. This mismatch now
refuses before constructing the result. A dedicated regression uses two
coplanar left-side faces, checks atomic rejection of the rear selection, then
checks successful construction with the adjoining selection.

This fixes selection fidelity for the scalar API. The separate grouped API
below extends that domain; neither change proves reference behavior parity.

Validation: all 10 tests passed across `feature_ops`, `full_round_frame` and
`full_round_prism` after this selection fix. Logs and current source hashes:
`docs/qualification/full-round-selection-2026-11-01/`. Frozen clean verifier:
11 files unchanged. The architecture gate still reports only the existing
`mainSolidResult.ts` line-count diagnostic (595 versus pin 594).

### Independent grouped-face operation

`feature_ops::full_round_groups(source, [left, middle, right])` accepts three
nonempty disjoint slices of original face indices on one recognized XY prism.
Every selected face must own exactly one complete straight outer-profile rim
segment. Within a group the faces must have consistent coplanar orientation
and form one continuous cyclic profile chain. Input collection order is not
used to determine topology order.

The operation collapses selected coplanar profile seams, leaving unselected
curves and holes intact, rebinds each collapsed segment to its authored side,
then uses the same isolated-strip Full Round builder. Invalid selections fail
before publishing any model. `FullRoundGroupsReport` returns the original
replaced/support face indices, radius, owned result and solid audit. Temporary
canonical indices do not escape in the selection report.

This is an independently designed grouped-face implementation. The reference
pretty-function string mentions `FaceCollection`, but reference execution of
multi-face groups has not been observed. General face networks, arbitrary
curved support groups, seam-preserving selected-face history and non-XY group
recognition remain open; the full library-parity scope is unchanged.

Final validation passed all 12 tests across `feature_ops`, `full_round_frame`,
`full_round_groups` and `full_round_prism`. The grouped regression includes
three left-side faces, two middle faces, two right-side faces, arbitrary input
order, reversed supports, original selection indices, analytical result volume
and atomic refusal of invalid selections. Source hashes and logs are retained
in `docs/qualification/full-round-groups-2026-11-01/`. Clean baseline: 11 files
unchanged. The existing architecture diagnostic remains 595 versus 594 lines
in `mainSolidResult.ts`; it is not resolved by these feature changes.

### Whole-seam G1 and continuous boundary agreement

Both scalar Full Round branches and the grouped operation now require
qualification of all three longitudinal joins: support/arc, arc/arc and
arc/support. Each authored endpoint pair must identify one distinct original
result edge, with exactly two owning face coedges. `surface_g1_pcurves::inspect`
uses those complete edge curves, owned UV pcurves and original result surfaces
to prove positions, surface regularity and tangent-plane agreement. A failed
or unresolved proof refuses the operation rather than publishing the body.

`watertight::inspect_with_exact` must additionally qualify whole-boundary
edge/surface agreement. All three tangent checks and closure share bounds of
100,000 cells and 1,000,000 exact-work units; the normal tolerance is 1e-5,
matching the existing profile-blend default. These bounds were not enlarged
to make a case pass.

`FullRoundReport::tangency` and `FullRoundGroupsReport::tangency` expose an
immutable `Arc<Qualification>` containing the checked model snapshot, seam
edge identities, complete G1 reports, closure report and consumed work. Its
observations remain bound to that snapshot even if the caller later modifies
the separate mutable result model. Grouped topology IDs are inherited before
qualification, so the exposed snapshot matches the delivered result.

This establishes bounded local G1 and continuous boundary agreement, not
global embedding, complete self-intersection resolution, general curved
support construction or Plasticity runtime equivalence.

Validation passed all 12 tests across the four feature/Full Round targets.
Tests on the rotated/reflected, nonrectangular/hole and grouped branches also
check that the immutable qualification snapshot equals the delivered result
and contains three seam reports. Source hashes, shared proof bounds and logs:
`docs/qualification/full-round-seams-2026-11-01/`. Frozen clean baseline remains
unchanged (11 files). The existing unrelated architecture diagnostic persists.

### Whole-side radius and radial/normal envelopes, 2026-11-02

The two faces owning the authored central seam must each qualify against the
original construction's axis-center line and constant radius law.
`moving_radius::qualify` bounds `|S(u,v)-C(u)|^2-r(u)^2` on every original knot
cell. This distance relation alone is not a radial-normal envelope.
`moving_envelope::qualify` then independently checks the radial/normal relation
over the entire unchanged surface, with maximum squared sine 1e-10.

Only an owned envelope from both checks is admitted. The immutable
`Qualification::envelopes()` exposes each result face index and its envelope;
`Envelope::radius()` retains the original surface, center curve, radius law
and distance-error bound. The surface is the exact transpose of the retained
result carrier, not a fitted proxy.

The two distance/envelope checks share the existing 100,000-cell and
1,000,000-work bounds with the three G1 checks and full boundary agreement.
Envelope surface-span and center-span observations are also charged to that
shared cell allowance. A missing radius/envelope certificate refuses the
operation. These local relations do not prove global self-intersection
absence, general support construction or reference runtime equivalence.

Final validation completed: test session 63847 terminated with exit code 0;
all 12 tests passed across `feature_ops`, `full_round_frame`,
`full_round_groups` and `full_round_prism`. The six pending implementation/test
source hashes match the saved checkpoint, and the grouped implementation
matches its prior qualified snapshot. Frozen clean verifier: 11 files unchanged.
The earlier stale-API build failure remains retained separately; it is not
counted as a passing run. Final receipt and log:
`docs/qualification/full-round-radius-2026-11-02/results.json` and `tests.log`.
These focused checks do not qualify the entire concurrently edited worktree
or close the remaining general geometry and reference comparison requirements.

### Full boundary embedding admission, 2026-11-02

`Qualification::qualify_embedding` freshly runs the existing joint original
boundary inspector on the immutable construction snapshot. It admits an
`EmbeddedBoundary` only when exact stored edge/carrier agreement, exact joins,
trim-region validity and winding, injectivity of every face and classification
of every distinct face pair all prove. Insufficient pair coverage or an
unresolved relation returns a refusal, not a partially qualified boundary.
The caller supplies explicit embedding-stage budgets independent of the
earlier local seam/radius/closure budget.

The immutable result retains both the original local qualification and the
full embedding report. It does not assert filled-material roles, cavity
nesting or a general Parasolid solid-volume equivalent.

A regression on the existing rear-trapezoid/through-hole fixture exposed
loss of exact carrier agreement when normalized cap UVs divided original
coordinates by a height of nine. Full Round now authors its fresh rectangular
XY cap carriers with physical XY knot domains and copies the original rim XY
coefficients directly into the cap pcurves. Carrier control points and original
edge curves are unchanged; no sampled replacement or predicate relaxation is
used. This is local to newly built Full Round bodies, not a global change of
the prism constructor. Arbitrary transformed carriers and general support
networks still require their own complete proofs.

Final validation: all 14 tests passed across the six Full Round/feature targets,
including complete embedding of the original height-nine nonrectangular
profile with a through-hole and refusal with insufficient pair coverage.
The prior exact-agreement failure is retained with the passing final log and
source hashes in `docs/qualification/full-round-embedding-2026-11-02/`.
Frozen clean baseline: 11 files unchanged. The unrelated existing architecture
line-count diagnostic remains open; the full concurrent worktree is not qualified.

### Explicit filled-volume admission, 2026-11-02

`Qualification::qualify_volume` freshly invokes `volume_validity::inspect_with_linear`
on the immutable Full Round snapshot. It admits `QualifiedVolume` only after
joint boundary embedding, complete shell nesting/role consistency and physical
shell orientation all prove. Caller-supplied stage budgets are validated by
the owning inspector. A local seam/radius report, triangle volume estimate or
previous embedding boolean cannot substitute for these proofs.

`QualifiedVolume` retains the same construction snapshot and full native
volume report. Its interface exposes immutable model/qualification/report
references. The constructor still admits only its previously documented
Full Round families; this does not add general curved supports, nested inner
shell construction or arbitrary cavity transfer. Through-holes of the tested
prism are part of its single outer shell, not separate inner cavity shells.

Regression source `tests/full_round_embedding_prism.rs` checks a nonrectangular
height-nine extrusion with a through-hole, consistent shell roles and outward
orientation. It also independently reverses the physical shell uses: boundary
embedding must still prove, but volume admission must fail with inward
orientation. Insufficient face-pair work must refuse typed volume admission.

Validation: all 14 tests across the six feature/Full Round targets passed,
including the independent inward-shell rejection with successful boundary
embedding. Frozen clean baseline: 11 files unchanged. Source hashes, logs and
the existing unrelated architecture diagnostic are retained in
`docs/qualification/full-round-volume-2026-11-02/`. General support construction,
nested cavity authoring/transfer and reference runtime comparison remain open.

### Curved profile construction and paused handoff

Independent `offset::profile_corner::full_round::propose` now solves original
polynomial/rational support contacts and radius with a bounded analytic-Jacobian
local solve. `feature_ops::full_round_curved::build` binds actual selected
source faces, retains source restrictions and admits the authored XY prism
only after whole G1/radius/envelope/closure checks. Both test weight variants
also passed explicit embedding/material-volume qualification.

The contact proposal alone remains numeric evidence. The current body domain
still requires a straight middle, one XY prism and two single-span arc sides;
general spatial supports, branch census and source-material subtraction
equivalence remain open. The crown constraint and numeric construction are
independent choices, not recovered Parasolid algorithms or verified placement
semantics. Latest receipt: `docs/qualification/full-round-curved-2026-11-02/`.

User paused the full goal. Results, reference-to-own-API provenance and the
complete remaining scope are saved in
[plasticity-transfer-status-2026-11-02.md](plasticity-transfer-status-2026-11-02.md).
No automatic implementation continuation is authorized by this document.

### Resumed: complete multi-span arc-chain qualification

The user explicitly resumed the goal after the saved pause. The construction
now enumerates every active knot span in both proposed contact arcs. Each
consecutive pair of longitudinal seams must own exactly one distinct authored
side face. Every seam needs fresh original-pcurve G1/regularity/position proof;
every resulting circular side needs its own radius and radial/normal envelope.
The previous two-single-span construction restriction is removed.

`Qualification::all_edges()` retains the complete ordered seam chain.
`edges()` remains the two support contacts and crown for compatibility; the
crown index is inferred from the actual span decomposition, not fixed at one.
The immutable model, full seam reports, envelope reports and closure remain
owned together. Missing/duplicate edge ownership or unresolved geometry refuses.

The existing shared limits remain 100,000 cells and 1,000,000 work units;
additional spans do not reset or enlarge them. Tests include 1+1, 2+2, 2+1
and 1+2 arc-span configurations on polynomial/rational supports, with complete
volume qualification requested for each fixture. General spatial construction,
global branch census and source-material subtraction equivalence remain open.

Final validation passed 17 tests across eight Full Round/feature targets.
The volume regression covers six symmetric/asymmetric polynomial/rational
fixtures, including 1+1, 2+2, 2+1 and 1+2 arc-span chains. Each passed complete
embedding, shell roles and outward orientation. Logs and source hashes:
`docs/qualification/full-round-multispan-2026-11-02/`. Frozen clean baseline:
11 files unchanged. The unrelated existing architecture diagnostic remains.

### Pointwise original-source arc cover

`trim_domain::curve_cover::inspect` constructs a cover of every original
candidate knot interval. Each accepted leaf either has a full rectangle
nonzero-winding interior proof, an exact literal original-boundary identity,
or an outward-rounded distance bound from the entire candidate interval to
one actual original boundary point. Numeric projection only proposes the
point; it never authorizes a leaf. A gap, outside-separated interval, exhausted
budget or precision limit retains pending domains and yields no certificate.
Source loops must have exact endpoint joins and positive weights.

`CurvedFullRound::qualify_source_arc_cover` applies this to EVERY actual arc
face's stored top rim, rather than a display mesh or the preliminary proposal.
Same-height XY extrusion carries the side-boundary relation along the axis.
Actual source BRep rim curves are copied with their original knot clocks;
oriented reversal uses exact sign reflection of reversed knots, avoiding a
normalizing division or rounded `a+b-t` knot conversion. The result rims keep
their original stored edge definitions. Source/result models and all curve
certificates are retained immutably.

The declared band is exactly `source.tolerance_mm`. Shared stage limits cover
ALL arcs: 100,000 curve cells, 1,000,000 classification units and 1,000,000
witness units. They are independent of the preceding local construction work.
This proves pointwise membership in the source cap's original winding region
OR proximity to its boundary. It is deliberately NOT exact set inclusion or
proof that all filled result material is contained in the source. A complete
signed material partition/subtraction relation remains open.

Validation passed 30 tests: 12 NURBS, 17 BRep/Full Round and one actual stored
source-clock regression. The latter re-clocks original edges to `[0.1,1.9]`,
reverses one stored edge to `[-1.9,-0.1]` with consistent vertex/coedge uses,
and verifies retained original clocks and exact actual result-edge definitions.
All six symmetric/asymmetric polynomial/rational fixtures passed complete arc
cover and result-volume admission under the shared bounds. Logs and hashes:
`docs/qualification/source-arc-cover-2026-11-02/`. Clean baseline: 11 files
unchanged; API map check: 242 public roots, 200 catalog entries. The existing
unrelated architecture diagnostic remains. Exact filled-material subtraction
and full Plasticity parity are still unproved.


### Exact regularized profile material relation (implementation stage)

`profile_certificate::material_relation::inspect` constructs a shared exact
original-curve intersection/overlap graph, complete port rotation and cycles,
and certified side/vertex-sector winding for both source and result. It compares
nonzero-winding filled regions, not winding multiplicity or sampled boundary
points. Private immutable `NoAddition` is issued only when the complete census
has no added-material sector. Holes participate in the same original graph.

`CurvedFullRound::inspect_source_profile_relation` reads actual stored source
and result cap edges with original parameter clocks. Its `NoAddedProfile` owns
both models and the exact relation. This is a two-dimensional regularized
profile certificate; independent three-dimensional volume/embedding admission
is still required.

The completed focused run in `/tmp/rea-exact-profile-relation-final.log` passed
five tests: actual straight-support Full Round inclusion, equality/removal,
hole-filling counterexample, refusal on exhausted port budget, and equality of
filled regions with different winding multiplicities. These results precede
subsequent concurrent worktree changes and do not qualify the whole project.

The rational curved-support diagnostic remains UNPROVEN: exact algebraic
intersection projection did not produce a complete representation within the
existing budgets. This is neither proof of added material nor a successful
inclusion certificate. Reproduction: example `full_round_material_relation`
(default weight 0.7). No predicates or budgets were weakened. The next step is
to identify and resolve the exact original-curve pair/projection limitation,
then rerun the focused tests and Full Round regressions. Full parity is open.


The completed pair diagnostic now localizes the rational refusal to original
curve pair `(1,5)` (four source curves, nine combined curves; degrees 2 and 2).
Compilation succeeded; diagnostic execution returned exit 1, as required for
UNPROVEN. Log: `exact-profile-material-relation-2026-11-02/rational-pair-diagnostic.log`.
This changes the next action from a general census search to inspection of
that exact original pair. It does not establish a missing or added region.


The stored-curve dump identifies pair `(1,5)` as the original rational support
and its numerically restricted retained segment. Source controls are
`[(4,0),(5,4),(4,8)]`, weights `[1,0.7,1]`; result controls are
`[(4,0),(4.623874036455391,2.4954961458215634),(4.334003755927576,5.858517992989387)]`,
weights `[1,0.7890327060493011,0.8747800728778061]`. These are actual stored
curves. Their approximate restriction relationship cannot authorize exact
common-component ownership or material inclusion. Dump log:
`exact-profile-material-relation-2026-11-02/rational-original-curves.log`.
Projection failures now retain individual causes in the refusal message;
projection choices, arithmetic budgets and verification predicates are unchanged.
New crossing-region and disconnected-added-island regressions are pending.


Independent exact rational diagnosis (`conic-pair-residual.py`, Python stdlib)
constructs the original rational quadratic's homogeneous conic equation by an
exact 3x3 inverse. Five exact original-curve samples have zero residual. Interior
and exterior reference residuals have opposite signs. Actual retained-result
points at parameters 1/4, 1/2 and 3/4 all have the exterior sign, with exact
nonzero rational residuals. Therefore literal exact coincidence of the retained
curve is false; an approximate common-component shortcut would be unsound.
This is a pointwise conic diagnostic, not a complete source-region/material
certificate. Next implementation must preserve original trim ownership or
explicitly prove the declared tolerance/material relation of stored geometry;
it must not silently replace actual result curves with ideal original ones.


Latest focused validation passed seven tests (six generic material relations
and one actual straight-support Full Round). Crossing profiles have both added
and removed sectors; a disconnected added island is rejected despite retaining
the complete source component. Log: `seven-tests.log` in the exact-profile
receipt. The standalone exact conic diagnostic is now whole-interval: its
quartic Bernstein residual coefficients have signs `[0,+,+,+,+]`, proving the
entire open retained-result segment lies on the exterior conic side. This is
not a whole-body material classification. Source-path/visibility repairs for
the extracted `same_support_contact::network` module restored compilation;
the dedicated polygon-network regression is being checked separately.


### Domain-excluded projection factors

The exact failure causes were obtained: projection 0 needed separation of
algebraic fibers; sheared projections exceeded the unchanged arithmetic budget.
The engine now retains factors with zero real projection roots in `[0,1+k]`
in the complete eliminant product while omitting their parameter maps. Exact
Sturm count includes both endpoints. Independent verification repeats this
exclusion, still verifies squarefreeness/disjoint factors and the full eliminant
identity, and refuses invented events. Factors with actual real projection roots
still require the previous complete fiber/map checks. No arithmetic limit or
predicate was weakened.

Eight focused tests passed after this change: the original rational pair now
has exactly one endpoint event and no overlap; deleting that true event/map
from the certificate is rejected. Six material-region tests and actual
straight-support Full Round inclusion also passed. This rerun overlaps prior
material tests and is not an additional disjoint eight-test count. Full curved
Full Round is being rerun separately. Receipt log:
`exact-profile-material-relation-2026-11-02/projection-exclusion-tests.log`.


### Whole-profile replay after projection exclusion

Existing event regressions passed 11 tests across `curve_event_nodes` (9) and
`bezier_algebraic_arrangement` (2). The complete rational Full Round replay now
passes the previous intersection obstruction and reaches exact side ownership;
it refuses with `Unified original side proof remains unresolved within radius
budget`. This is a later stage, not a successful material-inclusion certificate.
Logs: `event-regressions.log`, `full-round-after-exclusion.log` in the receipt.

Implemented an exact-distance initial transverse-radius proposal using the
complete normal-line intersection census and rational lower bounds; coincident
normal cells require positive-weight whole-cell separation for a proposal.
Unresolved proposals fall back to the previous unit radius. The unchanged full
transverse proof, exact center ownership, all wire winding jumps and source
replay remain the only authority. The number of radius halvings is unchanged.
A thin removed-band regression (gap 2^-45) and the existing eight focused tests
are being checked; this new radius stage is not yet qualified.


### Complete rational material census — strict inclusion disproved

The default rational Full Round (weight 0.7) completed the exact original-curve
profile census after both fixes. Result: 10 graph edges, 3 added-material
sectors and 6 removed-material sectors. An exact rational witness has source
winding zero and result winding one. `require_no_addition` correctly refuses
with `Result contains exact added-material sectors` (execution exit 1).
This is a completed negative strict-inclusion result, not UNPROVEN census.
Actual stored curves must not be replaced by ideal restrictions to hide it.
Declared-tolerance geometry and three-dimensional source material relation
remain separate obligations. Log: `full-round-after-radius.log` in the receipt.

The radius-stage broad regression passed 17 additional tests on four targets
(2 algebraic arrangements, 3 higher-degree profiles, 9 event-node tests, 3
original-carrier side tests). Together with the latest 9 focused tests this is
26 disjoint tests; prior overlapping reruns are not added. A one-second process
sample observed filled-region construction replaying exact winding proofs;
it is diagnostic stage evidence, not a performance benchmark.

Next: source-preserving trim construction or explicit one-sided, tolerance-
qualified material construction must address the exact added sectors. Preserve
the negative fixture and do not authorize `NoAddedProfile` for this result.
Full original scope, reference-runtime comparison and global 3D parity remain
open.


### Exact retained-source restrictions (implementation stage)

`profile_certificate::original_restriction` now owns an immutable original
Curve and exact rational natural-parameter bounds. It reuses existing exact
homogeneous span extraction, retains positive rational weights and all internal
source knot cuts, supports reversed traversal, and evaluates rational points
without binary64 control reconstruction. Source bits are validated first;
empty/outside intervals and excessive span counts refuse. Replay re-extracts
all exact coefficients from the retained original definition.

CurvedFullRound now retains both exact support restrictions using its solved
contact parameters, exposed by `retained_supports()`. This is prerequisite data
for source-preserving trim construction; the current stored Model is still
independent rounded geometry. These restrictions do NOT authorize NoAddedProfile
for the known negative result and do not certify exact contact joins, a closed
wire, embedding or a Body. New exact nonbinary/reversed restriction tests and
existing curved Full Round construction/volume regressions are running.


Exact restriction final qualification passed six tests (2 curved Full Round,
1 volume target with multiple fixtures, 3 original restriction tests). Original
knot clocks, reversed/nonbinary parameters, exact continuity across internal
knots, refusal of discontinuous source, and actual Full Round retained-owner
replay passed. Source hashes and final log are saved in
`docs/qualification/original-curve-restriction-2026-11-02/`. The stored Model
still has the known negative strict-material relation; exact contact sewing
and source-preserving body construction remain next, not completed by this API.


### Exact source-preserving G0 profile construction (implementation stage)

`original_restriction::wire::sew` now constructs a closed rational wire from
immutable original restrictions and an explicit editable-piece mask. Fixed
source pieces must retain identical homogeneous coefficients. At fixed/flexible
joins only the flexible endpoint moves; flexible/flexible joins use the preceding
endpoint as canonical. Fixed/fixed gaps refuse. Positive weights remain unchanged,
so the maximum Cartesian endpoint displacement bounds the entire edited rational
Bezier spans by convexity. An insufficient declared tolerance refuses; replay
re-extracts original data and repeats every edit/closure/bound check.

CurvedFullRound retains these wires as `exact_profile()`: both retained supports
and all unaffected outer/hole pieces are fixed; only the two bridge pieces are
editable. This is exact G0 source-preserving construction, not G1 qualification,
material inclusion, embedding, extrusion or a repaired stored Model. The previous
rounded Model and its known exact added-material sectors remain distinct and are
still used by its existing material checker. Regression tests are running.


### Exact source G0 wire qualification — 2026-11-02

Eight tests passed: 2 exact-wire tests, 3 restriction tests, 2 curved Full Round
construction tests and 1 multi-fixture volume test. New Full Round assertions
replay exact wires and compare all fixed pieces with their original restricted
homogeneous data. A nonbinary rational contact is shared without rounding the
fixed support; insufficient edit tolerance and fixed/fixed gaps refuse. Full
span displacement is bounded by unchanged-positive-weight convexity.
Receipt and source hashes: `docs/qualification/exact-source-wire-2026-11-02/`.
Clean baseline remains 11 files unchanged; API map check passed.

Next: G1/regularity admission on the new exact rational pieces, then downstream
source-preserving body construction and material/embedding proofs. The existing
stored Model is still independent rounded geometry with its known negative
strict material inclusion. NoAddedProfile has not been authorized for it.
Full original native-library parity scope remains active and open.


### Exact rational wire regularity and G1 qualification — 2026-11-02

`Wire::qualify_smooth_edits` now verifies every rational span has no common
zero of its two derivative numerators on the CLOSED parameter interval, using
exact gcd/Sturm census, including endpoint and even-multiplicity stationary
points. Every join touching an editable piece and every internal editable-span
join requires positive tangent dot product and an exact squared-sine bound.
Unchanged original corners explicitly retain C0. Immutable SmoothEdits owns its
wire and replays all checks. CurvedFullRound retains this qualification as
`exact_smoothness()`, with declared sin^2 <= 1/10^10 (not zero-angle G1).

Twelve focused tests passed: 4 smoothness tests (complete rational circle,
reversed collinear tangent, hidden/endpoint stationary points, original C0
corners), 2 exact-wire tests, 3 restrictions, 2 Full Round construction tests
and 1 multi-fixture volume test. Actual Full Round assertions compare the
qualified wire with exact_profile and check complete span/junction coverage.
Receipt: `docs/qualification/exact-source-wire-g1-2026-11-02/`.

Next: construct the 3D shell from exact rational data, then independently prove
embedding and source/result material relation. The old rounded Model and its
known added-material sectors remain unchanged; this profile qualification does
not authorize NoAddedProfile or full native-library parity.


### Exact rational 3D source shell — 2026-11-02

`original_restriction::wire::prism::extrude` now constructs exact rational
vertices, Bezier edges, plane caps, ruled extrusion surfaces and pcurves on
existing generic `brep_topology::Model`. Profile coefficients and nonbinary
heights are never rounded to binary64. Source loop/piece/span provenance maps
to every wall. Closed incidence, vertex fans, regular profile spans and exact
edge endpoints are checked. Full oriented edge/surface-pcurve agreement uses
projective polynomial identities on the whole interval; positive-weight control
hulls prove UV chart containment. Replay rebuilds and compares all source data,
topology and geometry. ShellGeometry deliberately has no Body role.

CurvedFullRound retains this actual 3D rational geometry as `exact_shell()`;
its source profiles match exact_profile and qualified contact data. Fifteen
focused tests passed, including nonbinary contact/height, hole shell Euler
characteristic, refusal of invalid height, and geometry-tampering cases that
mere incidence would accept. Legacy Full Round volume tests passed as regression
coverage, not admission of the new shell's volume. Clean 11 files preserved;
API-map check passed. Receipt: `docs/qualification/exact-source-shell-2026-11-02/`.

Next: exact trim Jordan/embedding/nesting and source/result material relation,
then solid admission and downstream integration. The legacy rounded Model and
its known added-material sectors remain separate; new rational shell construction
does not authorize its NoAddedProfile or establish whole-library parity.


### Exact rational Jordan profile census — 2026-11-02

Added `wire::jordan::qualify` and `ShellGeometry::qualify_profile_jordan`.
Every exact span is regular and checked for self-intersection; every unordered
span pair is checked. Only actual adjacent endpoint contacts on the same wire
are admitted, including both contacts in a two-span lens and the closing
contact. Positive-weight hull separation, exact implicit endpoint identities
and the independently verified algebraic engine prove the complete pair census.
Non-endpoint crossings, interior tangencies, unauthored contacts, overlaps,
unknown algebraic representations and exhausted budgets issue no certificate.
Immutable JordanWires owns all originals and replays the whole census.

Eleven tests passed: 6 Jordan tests, 3 exact shell tests, 2 Full Round tests.
Coverage includes collinear subdivision, disjoint hole boundaries, bowtie,
loop contacts/overlap, interior quadratic tangency, a regular cubic with an
internal self-crossing, rational circle closure, complete pair counts and low
budget refusal. Actual polynomial/rational Full Round assertions replay the
new profile proof. Receipt: `docs/qualification/exact-source-jordan-2026-11-02/`.

Next: certify outer/hole orientation and nesting by exact rational ray winding.
A point on each Jordan loop is valid for containment queries against every
OTHER disjoint loop. Own-loop orientation needs an exact regular transverse
probe plus complete winding, not a sampled area sign. Require CCW outer,
CW holes contained in the outer, and pairwise nonnested holes for the current
single-shell cap layout. Existing generic ray_winding primitives accept exact
homogeneous nets and can be reused. Only then can the Cartesian-product
extrusion proof establish embedded bounded material and assign a Body role.
Source/result material inclusion, old rounded-Model defects, pipeline delivery
and full original native-library parity remain open.


### Exact rational extrusion solid admission — 2026-11-02

Added exact transverse orientation and ray-winding containment in
`wire::nesting`: CCW outer, CW contained holes, no nested holes, with shared
ray-cell budget and unchanged transverse allowances. Own-loop orientation is
proved using a complete transverse census and exact left/right winding;
other-loop containment queries use an exact point on the disjoint Jordan loop.
No sampled polygon area or rounded point supplies admission.

`ShellGeometry::qualify_solid` assigns one Body role only after these proofs.
For this Cartesian-product family, simple disjoint correctly nested profiles
and positive height imply embedded plane/ruled boundary patches, outward
orientation and a bounded positive-volume region. Internal profile holes are
through-holes, not independent cavity shells. Immutable Solid retains all
sources and replays source geometry, Jordan, nesting and role construction.
CurvedFullRound exposes the new geometry through `qualify_exact_solid`; the
legacy rounded Model is not substituted or qualified by that API.

Sixteen tests passed: 5 solid/nesting, 6 Jordan, 3 shell, 2 actual Full Round
construction tests. Cases include two through-holes, nonbinary heights, a hole
of width 2^-45, wrong orientations, outside/nested holes, budget exhaustion,
bowtie refusal and replay of polynomial/rational Full Round solid promotion.
Receipt: `docs/qualification/exact-source-solid-2026-11-02/`.

Next: audit raw BRep source-clock binding (prism::directed_edge normalizes knot
clocks; retained exact owners must be traced to actual raw definitions), then
prove source/result material relation and target Full Round radius fidelity on
new rational data. Current source restrictions exactly preserve the Curve they
own, but recognized profiles are not a substitute for raw-clock ownership.
Generic extrusion admission is not general NURBS parity. Legacy added-material
sectors, converter/pipeline delivery and full original scope remain open.


### Raw indexed source binding and exact clocks — 2026-11-02

Full Round now binds each recognized hint to the actual top-cap coedge and edge
ID, accounting for face-use reversal. Raw coefficients/knots are retained;
reversal uses exact t -> -t reflection. Contact parameters transfer to raw
natural clocks in rational arithmetic, without a floating affine intermediate.
The normalized curve is only a numeric proposal, never source authority.
Unmodified outer and hole pieces now use raw originals as well. Selected middle
and support face ownership uses actual native edge IDs. RawProfileCurve replay
checks indexed coedge, traversal, trim and raw definition against the source.

Eleven tests passed: 10 across raw binding, source clocks, Full Round and exact
solid tests, plus the volume regression. Cases include different nonbinary
clocks on every edge, a reversed edge, a reversed top carrier/coedge traversal,
a contact value not representable as binary64, and source-definition tampering.
The six volume fixtures now also admit/replay the new exact Solid (not only
legacy rounded geometry). Receipt: `docs/qualification/full-round-raw-binding-2026-11-02/`.

Next: source/result material relation and exact original SIDE SURFACE binding.
Raw rim ownership does not alone prove whole-support surface equivalence;
new-geometry radius admission, downstream integration and full original scope
remain open. The old rounded Model remains independently negative for strict
material inclusion; no tolerance or ideal-curve substitution hides that result.


### Whole original side-support image qualification — 2026-11-02

Added `CurvedFullRound::qualify_source_supports` and standalone source qualifier.
Exact tensor factorization proves XY independence across the whole height chart;
positive constant cross-column weight ratio gives a monotone projective Z map.
Source UV boundaries must form an exactly closed rectangle, with complete
nonoverlapping side coverage and the full native height range. Positive-weight
rational straight pcurves are admitted by their entire segment image, not sampled
positions. Every native top rim is matched to its actual side/profile interval
using existing source-bound exact multi-span or bidirectional image certificates.
The full top-edge census covers every original side surface; no source carrier
is replaced by a normalized/trim-fitted surrogate.

Ten tests passed: 6 support cases, 2 raw binding, 2 Full Round construction.
Cases include exact degree elevation, projective height reparameterization with
correct inverse vertical pcurves, transposed UV axes, and rejection of a warped
height interior despite an unchanged top rim, UV endpoint gaps, and complete
profile mismatch. Model validation was retained: the initial projective test
needed inverse pcurve clocks rather than weakening the boundary checker.
Receipt: `docs/qualification/full-round-source-supports-2026-11-02/`.

This proves source SIDE SURFACE IMAGE equality for the extrusion family, not
oriented source-solid validity or general freeform support equivalence. Next:
source caps/complete boundary authority and source/result material relation;
new exact radius fidelity and downstream integration remain open. Full original
native-library parity scope is unchanged and active.


### Native source cap charts and honest seam modes — 2026-11-02

Added `original_restriction::plane` and `CurvedFullRound::qualify_source_caps`.
The original positive constant-weight bilinear chart is retained, including a
nonzero mixed coefficient; four strict Jacobian signs and convex boundary turns
prove global chart regularity/injectivity. Actual pcurve containment uses exact
polynomial sign/multiplicity census, admitting even boundary tangencies without
requiring the control hull to fit. Original UV loops pass complete Jordan,
orientation and hole nesting checks. Native physical cap normal signs are checked.

Each native world rim is compared with actual surface/pcurve composition over
common rational knot cells. Exact Bernstein products and positive denominators
bound the WHOLE parameterized displacement. Zero and nonzero bounds remain
distinct: SourceCaps::require_exact refuses a tolerated nonzero seam. No affine
surrogate, samples or idealized replacement supplies the cap geometry.

Thirteen focused tests passed (5 cap/plane, 2 raw binding, 6 side supports).
An additional 2-test raw-clock/reversed-carrier replay passed and overlaps the
main count. A box has exact cap identity; the default curved source has a
nonzero certified seam within model tolerance. Mixed bilinear data, actual
chart containment, folded-chart refusal and strict-mode refusal were tested.
Receipt: `docs/qualification/full-round-source-caps-2026-11-02/`.

Next: explicit source-bound cap pullback/canonicalization preserving original
3D rims and carriers, with a separate repair receipt for legacy UV auxiliaries;
verify lower-rim/side compatibility before any strict original-source solid
claim. A nonzero paired-parameter seam is not hidden as exact closure, and it
alone is not a universal image-set mismatch proof. Then compare source/result
material on exact rational geometry. Original input and old rounded result stay
available; full initial scope, new radius qualification and delivery remain open.


### Exact original-carrier cap pullbacks — 2026-11-02

Added `plane::pullback`: the unchanged original world curve is primary.
Exact whole-span half-plane sign checks prove containment in the actual convex
bilinear chart image. Affine carriers expose exact rational UV Bezier controls;
genuinely bilinear carriers retain a unique implicit inverse and return exact
rational isolating UV boxes. Root isolation, domain checks and explicit refinement
allowances fail closed. No rounded UV controls, replaced surface or sampled
containment supplies admission.

`SourceCaps::canonical_pullbacks` retains the original SourceCaps and its legacy
UV/seam receipt, binds each new inverse to the actual indexed boundary, and proves
Jordan/orientation/nesting on the unchanged world rims. The chart homeomorphism
transfers these region properties to the implicit UV boundaries. Original Model,
world curves and surfaces are unchanged; old nonzero seams remain nonzero.

Ten tests passed (3 inverse/containment, 5 native caps, 2 raw binding). Irrational inversion
checks bracket signs and requested width; zero-budget refusal, exact 5/6 UV
controls, even boundary tangency, outside excursion and actual curved Full Round
repair/replay are covered. Distinct raw clocks, reversed edge and transposed/
reversed carrier canonicalization/replay also passed. Clean baseline: 11 files
unchanged. API map check passed (242 roots, 200 catalog entries; not parity).
Receipt: `docs/qualification/full-round-cap-pullbacks-2026-11-02/`.

Next: verify lower-rim/side compatibility and construct explicit complete source
boundary authority, then source/result material relation on exact rational data.
This auxiliary UV canonicalization does not admit an original source Solid or
prove radius fidelity. General freeform charts, complete original library scope,
full regressions, pipeline delivery and external runtime comparison remain open.


### Complete original lower-rim image binding — 2026-11-02

Added `SourceSupports::qualify_lower_rims` with immutable source ownership and
replay. Every actual lower-cap coedge must have one unique original side owner;
its original world curve, with actual knot clock and traversal, is certified
against the native side profile on the authored bottom-isocurve window. Exact
parametric or bidirectional curve-image certificates authorize agreement, not
model tolerance. The full lower-cap census must cover every side bottom edge.
Original curves/carriers are unchanged; partial/lifted coedges are refused.

Eight tests passed: the six source-support cases plus two lower-rim regressions.
Positive cases include original nonbinary lower knot clocks, exact degree
elevation, projective vertical reparameterization and transposed/reversed side
axes. A 1e-10 lower-curve interior perturbation still passes Model validation and
top/whole-support qualification but correctly refuses exact lower-rim binding.
These eight overlap previous support qualification; do not add counts across
receipts. Clean baseline verified (11 files unchanged), API-map check passed.
Receipt: `docs/qualification/full-round-lower-rims-2026-11-02/`.

Next: exact vertical native edge/side-seam compatibility and complete source
boundary authority, including orientation/incidence. Lower-rim image binding is
not a source Solid certificate and does not prove coupled-parameter equality.
Source/result material relation, target radius on new rational geometry, global
spatial families, complete original scope and downstream delivery remain open.


### Exact native vertical seams and complete edge census — 2026-11-02

Added `original_restriction::vertical` and
`LowerRims::qualify_vertical_seams`. Original vertical edges retain full native
curves/weights/clocks; exact XY equals the side profile evaluated at the actual
constant UV coordinate. Each side's positive projective vertical map supplies
exact authored endpoint heights, including partial height intervals. Both side
uses must agree, traverse oppositely in the shell, and have exact native vertex
endpoints. Complete rational derivative-root census on every original Z span
proves strict monotonicity and regularity; even stationary roots and endpoint
stationarity refuse. Thus a certified edge maps one-to-one onto its segment.

Top, lower and vertical bindings must form a complete disjoint census of ALL
original edges. Legacy pcurves are retained; image agreement does not assert
coupled-parameter identity. Immutable owners replay all definitions and facts.

Twelve tests passed (9 source-support cases, 3 exact vertical tests). Positive
cases cover rational weights, nonbinary native clocks, both traversal directions,
degree-elevated/projective/transposed source surfaces. Tiny vertical interior
XY bow still passes Model validation and exact horizontal-rim proofs but refuses
vertical admission. Complete even-root, endpoint and backtracking refusals pass.
Clean frozen baseline: 11 files unchanged. Receipt:
`docs/qualification/full-round-vertical-seams-2026-11-02/`.

Next: complete oriented source-boundary authority: original side-profile
regularity/injectivity on actual trimmed windows, coherent carrier orientation
and complete cap/side assembly. Exact edge census alone is not source-solid
admission. Then prove source/result material relation and new exact radius.
Full original library scope, spatial/general freeform families, pipeline gates
and external runtime comparison remain open.


### Native carrier profile regularity, simplicity and outward normals — 2026-11-02

Added `Wire::qualify_source_regularity`: exact rational derivative census on every
source span and exact oriented G1 at internal knots, while authored corners
BETWEEN separate source pieces remain allowed. No sampling or nonzero angular
allowance authorizes internal carrier joins.

`SourceSupports::qualify_native_profiles` restricts the actual original SIDE
SURFACE profile on indexed top-rim windows, retaining native coefficients/knots.
Exact directed endpoints must match the original raw rim. Closed fixed wires,
whole Jordan pair census, orientation and hole nesting prove a simple native
profile region. UV-axis permutation, projective height direction, profile window
direction and shell face-use together must yield the outward physical normal.
This does not substitute a top rim for the source surface profile.

Thirteen tests passed (9 supports, 2 exact source regularity, 2 raw binding).
Coverage includes internal collinear knots, authored corners, internal kink and
even stationary-root refusal, actual degree-elevated carrier coefficients,
projective height, transposed side axes, different raw clocks and reversed cap.
Clean baseline: 11 files unchanged; API-map check passed. Receipt:
`docs/qualification/full-round-native-profiles-2026-11-02/`.

Next: exact LOWER directed endpoint/native vertex compatibility and regularity
of original WORLD cap rims, then combine cap pullbacks, native profiles and
vertical seams into complete source-boundary authority. Image-set agreement
alone must not be used as a directed endpoint identity. No source Solid admission,
material inclusion, new radius fidelity or full original parity is claimed here.
General spatial families, full regression/pipeline and runtime comparison remain
open under the unchanged original scope.


### Complete canonical source boundary and material reference — 2026-11-02

Added `CurvedFullRound::qualify_source_boundary`: original native profiles,
vertical seams and explicit cap pullbacks share the SAME unchanged Model.
Lower-rim image certificates now also require exact directed native/rim endpoints.
All horizontal world endpoints must equal their indexed native vertices. Original
world cap wires pass source regularity/internal-knot G1. Complete disjoint cap/side
face census and two opposite DISTINCT face uses for every original edge bind the
geometry to actual source incidence. No sampled/tolerance-only fact supplies
these exact endpoint or regularity admissions.

`SourceBoundary::canonical_material` constructs a SEPARATE exact rational body
from the certified native profile region times the original positive height.
Original side factorization and full rim/seam coverage identify this material;
cap domains use the explicit original-carrier inverse expressions. This is an
explicit auxiliary-UV canonicalization reference, NOT a claim that old nonzero
pcurves were identical, not a mutation of original Model, and not admission of
the old rounded Full Round result. Immutable material replay rebuilds the body
and checks exact source profile geometry/ownership. Through-holes remain holes
of the profile region in one body, not independent cavity shells.

Seventeen focused tests passed (4 boundary/material, 2 raw, 9 support, 2 source
regularity). Positive cases include polynomial/rational source data, actual raw
clocks, reversed cap, degree elevation, projective/transposed side charts and a
through-hole. Tolerance-valid vertex drift and stationary original world curve
refuse complete boundary admission. Fixture corrections were explicit: retain
the actual exact UV receipt where the fixture has binary-exact coefficients;
scale the stationary fixture by exact 1/256 to stay within the original allowed
model tolerance. No validator or predicate was weakened. Logs retain initial
fixture failures and final successful run. Clean baseline: 11 files unchanged;
API-map check passed. Receipt:
`docs/qualification/full-round-source-boundary-2026-11-02/`.

NEXT: source/result material partition on the NEW exact rational Wire data,
using this explicit canonical source reference and preserving the old rounded
negative material counterexample. Then exact target-radius/tangent fidelity on
new bridge geometry. General spatial/freeform families, complete original native
library scope, pipeline/WASM/full regressions and external comparison remain
open. A canonical source reference does not itself prove no material addition
or full parity.


### Exact rational Wire material census; NEW Full Round counterexamples — 2026-11-02

Added `wire::material::inspect` on already certified Jordan/nesting owners.
It consumes actual rational homogeneous nets, not rounded f64 Curve surrogates.
Each selected span is cut at a complete projected critical set from every
opposite-region span: isolated-system resultants plus common-image branch
endpoints/leading/discriminant critical values. Checked common-component
factorization retains residual isolated intersections. Pairwise coprime
squarefree factors retain the whole root union, with exact algebraic ordering;
no large global polynomial product is needed for root isolation.

Each open parameter atom has a rational representative. Complete point-preimage
census declares ALL coincident center aliases; unchanged transverse proof excludes
all other branches before exact ray winding labels both sides. For two bounded
simple oriented Jordan regions, every nonempty open difference component has an
open boundary atom; finite critical vertices cannot hide an open material cell.
Immutable replay reconstructs projections, root order, sample census and labels.
NoAddition refuses any source-winding-zero/result-winding-nonzero witness.

`CurvedFullRound::inspect_exact_source_material` owns the explicit canonical
original-source material body, NEW exact result Solid and equal-height proof.
It does NOT compare or authorize the old rounded Model. Product-family planar
inclusion transfers to equal-height material; source UV canonicalization remains
explicit and original Model/legacy pcurves remain unchanged.

Six focused tests passed (5 exact material, 1 actual Full Round test covering
both weights plus replays). Cases cover exact 1/3 controls, identical/partial
common boundaries, crossing rectangles, two hidden quadratic crossings, even
tangency, filling a source hole with identical outer boundary, and a 2^-45 added
band. Pair-budget exhaustion refuses. Real Full Round test/replays took 60.67s.

IMPORTANT NEGATIVE RESULT: the NEW source-preserving rational Full Round is
also NOT strictly inside canonical source material. Weight 1 has 7 added and
10 removed SIDE-ATOM WITNESSES; weight 0.7 has 4 added and 11 removed witnesses.
These are not distinct connected-component counts. Exact witness coordinates,
parameters, winding [0,1], logs and source hashes are saved. NoAddition correctly
refuses both. This is distinct from the previous old-rounded 3-added-sector
counterexample, which remains valid and available.

Runtime sampling located global-product Sturm isolation; the owned initial run
was intentionally interrupted for this evidenced optimization after weight 1
comparison completed. The final run uses smaller coprime factors, exact ordering
and unchanged engine budgets; both comparisons and replays passed. Receipt:
`docs/qualification/exact-wire-material-2026-11-02/`. Clean 11-file baseline and
API-map checks passed. Focused receipts do not qualify the whole shared worktree.

NEXT: repair rational bridge tangent controls/geometry while preserving exact
retained source restrictions, then re-run this strict material census and target
radius proof. Current endpoint-only repair and tolerance-G1 are NOT no-addition
proofs. Do not mask the counterexample with a tolerance band or return a false
success certificate. Full original library scope, spatial/freeform families,
pipeline/WASM/full regression and external runtime parity remain open.


### Exact edited tangents, original crown contact and whole-span radius — 2026-11-02

Replaced endpoint-only adjustment of the NEW exact Wire by explicit quadratic
control repair. Each edited span's middle control is the exact intersection of
its desired endpoint tangent lines. Fixed original source tangents control the
external joins; a shared original proposal tangent controls internal arc joins.
Positive intersection distances prevent reversal. All original positive weights,
fixed source restrictions and source natural clocks remain unchanged.

An explicit LineContact owns the actual raw middle source segment. The crown
junction is projected onto that finite original segment in exact rational
arithmetic (strict interior), and both incident arc tangents use the original
line direction. No normalized surrogate or support extension supplies contact.
Every modified Cartesian control is included in the displacement bound, which
bounds the WHOLE rational curve by positive-weight convexity. Immutable replay
retains the repair mode and source-owned contacts. Parallel/invalid tangent
intersections, reversed directions or displacement beyond the ORIGINAL tolerance
refuse. `qualify_smooth_edits` now uses ZERO angular allowance for new Full Round.

Added `Wire::qualify_edited_radius` / `CurvedFullRound::exact_radius`: exact
Bernstein coefficients of (X-Cx W)^2+(Y-Cy W)^2-r^2 W^2 bound the whole radial
deviation by max|N|/(min(W)^2*r). Every edited span is checked to the unchanged
proposal center/radius within source.tolerance_mm/32. This is a rigorous radial
ERROR BOUND, not an assertion of exact analytic-circle identity when nonzero.
The certificate owns/replays the actual repaired Wire, not old f64 bridge curves.

Ten focused tests passed. Full Round strict material/replays now yield ZERO
added witnesses for both weight 1 (8 removed witnesses) and weight 0.7 (10 removed).
NoAddedExactMaterial admits/replays both. The same test reconstructs the previous
endpoint-only Wire from unchanged proposal sources and finds 7 added witnesses;
NoAddition still refuses it. The predicates/limits were not weakened. Tangent-only
repair had left a crown overshoot at y>8 in the polynomial case; exact original
line contact eliminated it. Intermediate/final logs and own source snapshots are
retained. Old rounded Model and all prior negative receipts remain separate.

The 10-test set includes 2 restriction, 2 tangent/control-bound, 2 construction,
1 volume (six symmetric/asymmetric polynomial/rational multispan fixtures),
1 material (two full cases plus replays and old-G0 negative), 2 raw binding tests.
A middle-control-only move has exact squared bound 1/64 and rejects a smaller
allowance; parallel tangent refusal passes. Volume fixtures now also run the new
radius admission at construction. Strict material census was run on the TWO
explicit symmetric cases, NOT all six volume variants. Clean baseline 11 files
unchanged; API-map check passed. Receipt:
`docs/qualification/full-round-exact-tangent-radius-2026-11-02/`.

NEXT: strict exact material/radius qualification on remaining asymmetric and
multispan fixtures, then connect admitted new rational geometry into downstream
Model/pipeline without silently returning the old rounded construction. General
spatial/freeform support families, variable-radius/high-valence blend networks,
full regression/WASM/external comparison and complete original parity scope
remain open. Two admitted cases do not complete the full Full Round family.


### Six-case strict family and admitted exact native entry — 2026-11-02

Extended strict NEW rational material qualification to all six existing volume
fixtures: symmetric/asymmetric polynomial/rational supports with arc span layouts
1+1, 2+2, 2+1 and 1+2. EVERY case has zero added-material witnesses; NoAddition
and all source/result material certificates replay. Whole edited-span radial
bounds and ZERO-allowance G1 are also checked on each actual output. The count
of removed side-atom witnesses varies by case and is not a component count.

Added public `feature_ops::full_round_exact` and `AdmittedFullRound`. The API
returns the ACTUAL rational `prism::Topology` owned by NoAddedExactMaterial,
with matched radius and tangency owners. Geometry equality binds these proofs
to the output profile. No f64 conversion or old rounded Model is returned as a
fallback. Original source and selected-face identities remain accessible, with
immutable full replay. Existing legacy `full_round` / `CurvedFullRound::model`
remain separate; downstream consumers have NOT been silently migrated.

Usage: `feature_ops::full_round_exact(&source, faces, Default::default())?`;
`result.model()` is exact rational topology, `result.reverify()` replays proofs.
This is a native library entry, NOT completed ModelGraph/WASM pipeline wiring.

Nine distinct focused tests passed: six family cases plus the latest three
public-entry cases. Earlier overlapping entry reruns are not added. Entry tests
prove returned-model pointer ownership, correct source retention, no fallback on
missing material budget/wrong original selection, and strict admission with a
preserved through-hole (one body, two cap holes, no cavity shell). Six-case run
27.34s; latest entry run 23.69s. Own code snapshots/hashes and logs:
`docs/qualification/full-round-admitted-family-2026-11-02/`.
Clean baseline: 11 files unchanged; API-map check passed (not a parity count).

NEXT: rational topology transport/ownership in ModelGraph/WASM/main pipeline,
source-bound topology lineage and exact native exchange/roundtrip. Preserve the
actual rational payload and proof linkage; rounding it into the old Model does
not transfer material admission. General spatial/non-extrusion support families,
variable-radius/high-valence/reblend networks, full regressions and external
runtime comparison remain open under the full original scope.


### Frozen-source native/WASM transport — 2026-11-02

Added an explicit frozen Recipe (original natural-clock retained ranges, original
numeric bridge proposals, exact target center/radius). `recipe::restore` reads
these as UNTRUSTED proposal inputs, binds selected faces to actual raw original
rims, refuses reversed/changed unselected source intervals, reconstructs exact
tangent/crown geometry, and freshly proves original source boundary, result Solid,
whole radius, zero-G1 and strict no-addition. It DOES NOT rerun floating Newton/
trigonometric contact generation. This is essential for identical geometry across
native and WASM math implementations. Source/range ownership is never inferred
from a caller success field.

Codec formats `admitted-full-round/1` and `exact-rational-nurbs-brep/1` retain the
ACTUAL output topology: rational vertex/homogeneous controls/knots are integer
string pairs, together with exact indexed incidence and source wall mapping.
Curve spans are normalized clamped Beziers; surface native parameter domains are
preserved (including low/high-corner -> per-axis knot mapping). Source Model,
selected faces and frozen recipe stay explicit. Import rebuilds and proves the
body, then compares complete output/evidence; serialized flags cannot authorize
geometry. Transport has an explicit <=4 MiB byte budget. JSON numeric tag changes
are permitted only as numeric semantics; rational coefficients remain strings.

Shared geometry-bridge router now exposes `cad_full_round_exact` and
`cad_full_round_exact_restore`. Unknown request success fields and incomplete
material budgets refuse. No f64/legacy Model fallback is returned. This is the
actual native/WASM operation router, not yet ModelGraph schema/result plumbing.

TWELVE Rust tests passed: 2 codec/recipe, 2 public bridge, 6 multispan family,
2 raw-clock/reversed-carrier. The filtered fixture replay overlaps one codec test
and is not added. Frozen reconstruction reproduces identical exact topology in
all six families (1+1, 2+2, 2+1, 1+2) and with nonbinary clocks/reversed source.

FRESH REAL WASM was built (wasm32-unknown-unknown, dev opt1/debug0, import-free)
and exercised through operation 0 / MGV1 binary ABI using the actual TS codec.
Six cases passed: native frozen artifact -> identical WASM rational geometry;
original-source WASM construction; WASM JSON frozen roundtrip; forged exact
coefficient refusal; caller-success-key refusal; changed original interval
refusal. SHA256:
`69eb251798727f99df9c2a5b88691a56b5db3ce4ac2122edadc565a06af2adf6`.
Own native artifact, source snapshots/hashes, runtime timings, logs and build
recipe are saved at `docs/qualification/full-round-frozen-recipe-transport-2026-11-02/`.
The 43 MiB custom module stays at the recorded /tmp build path; the packaged
application kernel/identity were NOT replaced. Clean 11-file baseline and API
map checks passed. This is focused qualification, not a shared-worktree full gate.

NEXT: ModelGraph exact-rational carrier/schema and downstream operation/result
handling, persistent topology IDs/source-owned lineage, exact payload consumers
for display/edit/exchange, then packaged source-bound kernel and real worker
integration. Do not coerce rational geometry into the legacy f64 Model and assume
proofs transfer. General spatial/freeform families, global blend networks, full
regressions/external comparison and original whole-library parity remain open.


### Indexed source-face lineage and portable exact topology IDs — 2026-11-02

Added `AdmittedFullRound::lineage`. Parent selection follows the ACTUAL indexed
constructor plan and certified original rim/side bindings, never a nearest/similar
geometry fingerprint search. Retained-side records own original face/rim/loop
TopoIds, actual coedge slot, source piece and exact oriented AND original stored
natural-clock span ranges; reversed raw clocks map back by exact negation.
Generated blend records keep all three selected source-face contributors and
bridge/span indices. Cap parents are the unique certified original height owners.

Every output vertex/edge/loop/face/shell/body receives an ID. SHA namespaces use
all current source identities and exact output homogeneous geometry; they only
disambiguate new names, never establish ancestry. IDs are checked unique and
newly generated IDs disjoint from current input IDs. An unchanged whole side
with one child and complete single-rim source coverage retains its original face
ID. Partial one-child sides/caps are Modified; multi-child source-face replacement
is Split. Blend face contributors stay explicit; the consumed middle face is the
replacement parent in the ChangeSet. Source change history is retained and the
combined ChangeSet validates. Shell/body identities are modified from the original
single owners. Vertex/edge/loop IDs are currently GENERATED, not claimed preserved
under later edits or complete source edge/vertex ancestry.

Transport adds optional derived `geometry.topologyIds` / `faceOrigins` to v1.
Current exports include them; supplied naming metadata must equal fresh native
reconstruction. Older v1 artifacts lacking BOTH fields still receive complete
geometry/material reconstruction and upgrade to newly derived metadata. Partial
or forged ancestry cannot authorize an input. Native/WASM IDs are identical for
the same frozen source/recipe. General occurrence/branch/edit naming invariance
has not been qualified.

FOURTEEN Rust tests passed (4 admitted/naming, 2 codec, 6 multispan family,
2 raw binding). Coverage includes an authored source-ID rename that would alias
a former output child without source-context rebinding, forged ID/parent refusal,
whole-face persistence, hole faces, all source span layouts, reversed/native
clocks and exact original stored-domain membership. EIGHT real MGV1 WASM ABI
cases passed, including native naming equality, legacy naming upgrade and forged
source-lineage refusal. Fresh import-free WASM SHA256:
`6335e44e7345131f2cccd316f3b5875cd9ff4f674144cf935e134c482413ffe5`.
A hash-named module copy is retained outside Git at the path in the receipt.
Own source snapshots, native artifact, logs and reports:
`docs/qualification/full-round-source-lineage-2026-11-02/`.
Clean baseline: 11 unchanged files; API map check passed. The packaged application
kernel is still NOT replaced; this is focused custom-WASM qualification.

NEXT: ModelGraph exact-rational carrier/schema/result handling and operation
execution. Relevant paths are `modelgraph-runtime/src/nurbs/compile*`,
`src/services/rushGraphNurbs.ts`, `rushGraphNurbsKernel/createOwnNurbsResolver.ts`,
and native `geometry-bridge/src/brep_semantic.rs` / `brep_session.rs` for the
SemanticProgram DAG. Their current solid consumers deserialize legacy f64 Model;
do NOT smuggle exact string pairs through that type. Exact result admission must
retain/reconstruct the frozen Rust-owned body, and session/worker budgets must
charge the complete artifact. Then extend edge/vertex/loop source ancestry and
edit/occurrence stability, source-bound packaged kernel/worker delivery, display/
edit/exchange consumers and full regressions. Whole original parity remains open.


### Native semantic DAG exact Full Round carrier — 2026-11-02

Added unary native semantic node `full-round-exact` (`input`, original indexed
`faces`, optional material limits/transport byte budget). It consumes the actual
owned upstream source and invokes the material-admitted native factory. The
result uses the EXISTING shared `rational-brep` / `representation-preserving`
carrier, geometryKind solid, d3. The actual result is rational topology plus its
complete frozen artifact/IDs, never a cast of the old f64 Model.

`brep_session` recognizes the new reference arity and rational carrier. Before
publishing a nonempty exact lease it requires matching model/artifact geometry
and freshly reconstructs the native recipe/material/G1/radius proofs. The owned
snapshot retains the whole artifact. Byte and aggregate character budgets charge
that COMPLETE value, not only the visible model. An allowance that fits source
and model but omits frozen source/recipe/evidence correctly refuses.

Legacy operations explicitly reject the exact-rational encoding with
BREP_UNSUPPORTED_SEMANTIC; they do not round it into Model. A legacy primitive
cannot claim rational output by changing its declared carrier. Wrong Full Round
carrier and missing material work refuse. Failed direct-session evaluation keeps
prior entries until close/abort, as existing transaction semantics require, but
cannot publish or commit; the public graph runner aborts all entries on failure.
Initial tests incorrectly expected immediate clearing on direct Session failure;
those expectations were corrected, not the existing transaction contract.

FIFTEEN Rust tests passed (4 native graph integration, 11 existing Session
regressions). Tests cover public `brep_graph`, exact frozen output, proper carrier,
full retained-geometry budget, zero material budget, cast refusal, unsupported
legacy transform, owner/release/commit/typed-empty behavior. SIX actual WASM ABI
cases passed on a fresh custom import-free build; public graph execution produces
rational strings/IDs and every negative path refuses with the expected category.
WASM SHA256:
`20c3db1d2ae0a1226a182b1bbef51afadc2d7510e4bbc41ed3c3fb512a427f21`.
Logs, own source snapshots, timings and hash-named retained module path:
`docs/qualification/full-round-native-graph-2026-11-02/`.
Clean baseline 11 files unchanged; API-map check passed. Packaged kernel unchanged.

SCOPE: this enables the NATIVE semantic DAG/session/router path. High-level
ModelGraph/SemanticProgram node unions, frontend schema, capability closure,
operation identity/lowering and result/display readers are NOT yet migrated.
Requests with full envelopes must retain strict validation; do not bypass them
just to route an unknown operation. Relevant next files include
`src/core/semanticProgram.ts`, `semanticProgram/semanticResultItems.ts`, host
validators/backends, `geometry-bridge/src/brep_envelope.rs`, plus the separate
NURBS `rushGraphNurbs` schema/resolver pipeline. The existing rational carrier
is already declared, but the new operation kind is not. Then rational downstream
edits/exchange, edge/vertex/loop lineage, packaged worker delivery/full regressions
and all original spatial/freeform parity obligations remain open.

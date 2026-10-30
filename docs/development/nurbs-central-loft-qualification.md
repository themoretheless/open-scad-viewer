# Central trihedral loft qualification

The standalone central loft remains a proposal. Native sewing, embedding,
material and volume admission are tracked separately in
[native sewn corner qualification](nurbs-sewn-corner-qualification.md).
This work continues the five-part NURBS plan; it does not close SS/Boolean,
central G2 corners, general offsets, simplification, or editor/STEP delivery.

## Original stock

`brep_core::source_box::qualify` qualifies the actual six-plane rectangular
stock without requiring an XY graph parameterization. It checks literal
corners and edge endpoints, six distinct planar supports, exact original
curve/pcurve/surface agreement, fresh source incidence, closed embedding,
orientation, and volume admission. A tolerant `Model::validate` or a cached
Boolean flag is not sufficient. Original source data is borrowed immutably.
Each geometry phase retains its explicit existing limits; structural/exact
stock work has its own charged budget.

## Central proposal

`propose_trihedral_center` builds six Bernstein rows between the two halves
of the actual six-curve rim. It retains rational weights and accommodates the
real degree-eight rim, rather than replacing it with sampled quintics.
The two U boundaries collapse to declared poles.

All final control points lie in the qualified original stock, and all rational
weights are positive. The convex-hull property therefore bounds the whole
mathematical surface inside the stock. This claim does not depend on a sample
grid, seam residuals, STEP validity, or a fitted graph.

The boundary proposal alone may project tiny control excursions back to the
stock by at most position tolerance / 32 per coordinate. That displacement is
reported; the original rim remains unchanged. Separate continuous
curve/surface bounds compare the original and candidate supports against the
same rim, each at half the requested positional tolerance. A large interior
handle is rejected, never clipped into stock to manufacture admission.

## Gates tracked separately

These gates were open at the initial proposal milestone. The bounded native
sewing result and its current evidence are recorded in the linked qualification
document; broader adapter and general-geometry gates remain separate.

- Fresh full original-support G2 inspection, including singular endpoints.
- A quotient or regular atlas proof at the collapsed poles.
- Regularity, embedding and seam compatibility across every internal knot.
- Whole-region contacts with original supports and surrounding strips.
- Sewn trimmed corner topology, original-material containment, and volume.
- Native/WASM/Rush/UI edited-body integration and STEP round-trip gates.

Passing the stock or positional proofs must not be promoted to any of these
claims. `inspect_original_g2` computes fresh reports and exposes unresolved
results without inheriting a G2 flag.

## Numerical boundary regression

An origin fixed at control point (0,0), outside the active basis support, can
cause cancellation on an exact rational boundary: x=0 becomes approximately
-1.39e-17 in a bilinear example. The real central fixture exhibited x=-2.78e-17.
The direct evaluator now selects an origin from its active basis support, as
the cached evaluator already does. This removes cancellation from an inactive
corner without changing material tolerances or the control net. The source fixture and direct,
cached, and immutable snapshot evaluation paths are checked separately.

## Verification record (2026-10-30)

- Regression before the numerical fix: the independent bilinear rational
  boundary test fails at v=0.3 with x=-1.3877787807814457e-17.
- After the fix, the four native libraries pass 4225 tests, with zero failures
  and six existing ignores (brep 2015, nurbs 2002, predicates 100, intersect 108).
  Log: `central-loft-all-native-r9.log` in the local qualification directory.
- Public map/catalog consistency passes: 239 modules, 200 entries. Catalog
  statuses remain 79 integrated and 121 native; none is promoted to complete
  multi-adapter qualification by this milestone.
- Ten architecture/test-partition infrastructure tests pass. The actual global
  architecture audit still reports 69 size violations outside this change;
  no limits, pinned sizes, or layer rules have been relaxed. All files touched
  by this milestone pass the per-file size check after extracting the cached
  evaluator and construction tests from `core/surface.rs`.
- The final module layout also passes the same four native libraries:
  4225 passed, zero failures, six existing ignores. Log:
  `central-loft-all-native-final.log`.
- Fresh six-seam diagnostic: all six original positions are proven; all six
  geometric G2 decisions remain `Unresolved`, with regularity unproven and
  1000 cells per seam (6000 shared). This is neither a G2 admission nor a
  demonstrated geometric counterexample. Log: `central-loft-g2-diagnostic-final.log`.
- Git reference reads currently block inside the operating-system `open` call.
  A remote-only query succeeded and found both main and the recovery branch
  at `4d065b3b60f969f72ea7645911ab832eb1fbefe5`. A local source archive is a
  preservation measure, not a successful commit or push.

## One-sided original-span shape bounds

`nurbs_core::surface_shape_operator::span_rectangle_bounds` identifies one
literal original nonzero U/V span and bounds a closed rectangle inside it.
Endpoints use that explicit span's one-sided derivatives. The source surface
is never cropped or refitted. A midpoint shape only proposes a conditioning
frame; full original interval jets and its projected Jacobian must verify the
complete query. A missing regularity bound returns no normal/tensor.

This API does not remove the C2 guard in ordinary `rectangle_bounds`, certify
an internal join, or qualify a collapsed pole. Its purpose is to support the
original-span atlas needed for those separate proofs. Negative tests compare
the two sides of an actual planar kink, reject a cross-span query, and retain
a refusal for singular geometry.

Central tests also evaluate four finite parameter values per seam using
interval pcurve evaluation, intersecting UV enclosures only after verifying
positive-control-hull containment in the source natural domain. Every
intersecting literal original span is included. Normal and signed tensor use
a coherent orientation across the four values. These are finite-point
checks, not whole-seam G2 evidence and not an internal-seam or pole proof.

### Original-span verification result

The final four native libraries pass **4229 tests, zero failures, six existing
ignores** (brep 2015, nurbs 2006, predicates 100, intersect 108). The four new
span tests include a rectangle whose midpoint is regular but which crosses an
actual fold: the whole rectangle must remain unqualified. This preserves the
distinction between a conditioning proposal and full-region evidence.

The final central diagnostic covers 24 finite seam parameters on actual
original spans. With a coherent normal/tensor orientation on each seam, the
largest normal-difference upper bound is below 7e-12 and tensor upper bound
below 4e-10. These values remain finite-point evidence only. All six existing
whole-seam G2 reports remain `Unresolved`; no candidate, pole or sewn corner
has been promoted to admitted status.

Logs: `span-shape-all-native-final.log`, `span-shape-central-final.log`,
`span-shape-focused.log`, `span-shape-node-tests.log`, and
`span-shape-architecture-audit.log`. Public map/catalog consistency still
passes with 239 modules and 200 entries. Ten Node infrastructure tests pass;
the global architecture audit still has 69 unrelated size violations.
A fresh bounded Git status probe also timed out and was terminated without
modifying repository state.

## Continuous original-span field coverage

`surface_g2_pcurves::inspect_span_fields` bounds the full original common
curve/pcurve/surface positions and covers every intersected original span's
one-sided normal and signed world curvature tensor. A single orientation must
hold over the entire trace. The return type is `FieldCoverage`, deliberately
separate from joined G2 admission: `Enclosed` grants field bounds only, not a
C2 chart join, a pole quotient, embedding, or a sewn solid.

Interval pcurve images are tightened only by positive-weight control hulls.
Every intersected original span is queried without refitting. A midpoint may
supply a lower counterexample, but only if the complete UV uncertainty belongs
to the queried original span; rounding across a knot cannot turn an unrelated
one-sided jet into an actual witness.

Mean-value acceleration uses third original jets only when the complete image
lies in one original surface span and one pcurve span. Crossing or touching an
interior surface knot declines that acceleration and retains the direct
one-sided interval cover. Conditioning through a local world graph verifies
the complete original projected Jacobian; a sampled normal merely proposes the
frame. Directional dual arithmetic differentiates the inverse, gradient and
Hessian, preserving original signed orientation under reversed projections.
The generic original-jet bound and projected bound are intersected whenever
both exist, so a wider projected enclosure cannot replace the tighter
original directional enclosure. Disjoint purported proofs cause an error.
An available original bound remains the fallback.

Work and exact-position budgets are shared. Exhaustion retains the old parent
cover and all proved counterexamples; it cannot turn an unvisited child into a
positive result. The existing strict joined-G2 inspector and its C2 guard have
not been relaxed.

The central adapter `inspect_original_span_fields` freshly runs all six
original rim traces with one shared budget. Before graph conditioning, its
6000-cell run enclosed the complete fields of seam 1 in 55 cells; all other
seams remained unresolved. This is stronger than finite-point evidence but
still not a completed central corner.


### Field coverage qualification

The first final four-library run passed 4235 native tests, zero failures,
six existing ignores (brep 2015, nurbs 2012, predicates 100, intersect 108).
Six new field tests cover complete C0 traces, actual one-sided kink witnesses,
work/position refusal, a defect outside a good midpoint, rescaled reversed
pcurve clocks, and all world derivative components under two projections and
both original parameter orientations. The last two use independent analytic
normal/curvature formulas for a parabolic surface.

Projection alone widened the curved seam 4 enclosure at the same shared
6000-cell budget; it was therefore replaced by componentwise intersection
with the original metric enclosure. The remaining full-source verification
is recorded separately in `span-field-intersection-all-native.log`. No
position, normal, curvature, work, architecture or publication limits were
increased to obtain a positive field report.

### Third-order projection correction

Intersecting the independent metric and projected directional bounds exposed
an inconsistency on an actual rational central trace. The previous projected
jet function intentionally calculates only total order two; using its zero
uncomputed third entries for curvature derivatives was incorrect. Thus the
earlier passing projection tests were insufficient qualification of the
rational third-order path.

The directional calculation now requests explicit third-order original
projected jets. The existing second-order function retains its contract and
cost. A dyadic cubic graph regression uses x=3u/4, z=x^3: at u=0 its xx tensor
directional derivative is exactly 4.5, whereas omitting third derivatives
incorrectly gives zero. The source metric/projected intersection remains
mandatory. No disjoint bounds are silently discarded.

The failing integration log `span-field-intersection-all-native.log` is retained
as negative evidence. Final verification after the third-order correction is
recorded in `span-field-third-all-native.log`; the earlier 4235-test projection
run is superseded for this path.

### Final corrected native evidence

After explicit third-order projection, all four libraries pass **4236 tests,
zero failures, six existing ignores**: brep 2015, nurbs 2013, predicates 100,
intersect 108. The actual rational central trace also passes the independent
enclosure intersection; the cubic derivative regression passes.

The final 6000-cell central adapter run proves positions on all six seams.
Seam 1 has complete one-sided field coverage in 55 cells, with coherent
opposite orientation, normal upper bound 1.433e-13 and tensor upper bound
5.965e-11. Five other seam field reports remain `Unresolved`. Seam 4 has
proved one-sided regularity but shape bounds remain above the requested
tolerances (normal 0.030306, tensor 5.744601). The strict joined-G2 inspection
still leaves all six seams unresolved; neither a completed pole quotient nor
a sewn corner is claimed.

Logs: `span-field-third-all-native.log`, `span-field-third-central.log`,
`span-field-third-minimal.log`, `span-field-third-node.log`, and
`span-field-third-architecture.log`. Map/catalog consistency passes with 239
modules and 200 entries; 10 Node tests pass. The global architecture audit
still reports the same 69 violations outside this change. A fresh bounded
Git reference/status read again timed out; its own read-only process was
terminated without modifying repository state. No push is recorded.

## Correlated internal join proof

`inspect_original_span_join` compares actual original left/right jets on a
literal shared C0 control column. This bounded family has nonperiodic U
Bernstein pieces (degree 2..16) and one original V Bernstein span (degree
1..16); other layouts are refused rather than assumed to have literal shared
coefficients. The common original column gives exact G0 without a fitted iso
curve or a cached position flag.

Homogeneous derivative differences are formed as Bernstein coefficient
polynomials before taking V hulls. The rational quotient identities retain
shared N/W factors, including weight derivatives. DU, DUU and DUV differences
are bounded; DV and DVV differences are exactly zero on the common column.
Paired interval arithmetic carries both actual jet enclosures and their
correlated difference through cross products, norm division, inverse metrics,
and the world curvature tensor. Both original sides must independently have
nonzero area. Matching differences cannot manufacture regularity.

The reported result is geometric G2 **within the supplied tolerances**, not
exact parametric C2 and not a sewn-corner/embedding/volume certificate. No
strict C2-basis guard elsewhere has been removed. Zero/one-cell budgets count
no unvisited source queries as performed work. Parent covers and negative
witnesses survive incomplete subdivision.

The real unequal-radius central fixture has four internal U knots. The
focused run proves all four joins with 200 original source queries in the
existing shared budget of 6000:

| U knot | Normal jump upper | Tensor jump upper | Queries |
|---|---:|---:|---:|
| 0.1850649648209695 | 7.095e-12 | 1.969e-5 | 68 |
| 0.35227262305754015 | 3.888e-11 | 5.319e-5 | 44 |
| 0.4032923357407631 | 1.529e-11 | 2.090e-6 | 44 |
| 0.867398296832364 | 7.301e-12 | 1.010e-5 | 44 |

Tolerances remain normal 1e-5 and curvature 1e-4. A regression requires every
actual internal join to pass, not merely a nonempty report list. Exterior
seams, collapsed-pole quotients, whole-surface embedding and sewn volume
remain open.

### Final internal-join verification

All four native libraries pass **4240 tests, zero failures, six existing
ignores** (brep 2015, nurbs 2017, predicates 100, intersect 108). The new
original-join tests include common weight scales 1e-12, 1 and 1e12, a
nonuniform rational V clock with weights [1,2,1], exact translations, actual
kinks, singular geometry and exhausted/one-query budgets. Source coefficients
are checked unchanged.

The actual central fixture regression requires all four internal joins to
qualify over their complete V ranges at the unchanged tolerances. This closes
that internal-join item only. It does not turn the six exterior seam reports
or the two collapsed poles into proven geometry, and it does not establish
whole-surface embedding, a sewn corner, thickness, volume, STEP or UI delivery.
Logs: `jet-join-all-native.log`, `jet-join-central-final.log`,
`jet-join-minimal.log`, `jet-join-node.log`, `jet-join-architecture.log`.

### Conditional next pole construction proof (not implemented)

First inspect literal original coefficients; do not assume almost-flat rows
are exact zeros. A single original pole span with a shared collapsed first
row and one constant height coordinate in its first three rows admits the
exact form, in normalized pole coordinates,

`S(u,v)-P0 = (u*Gx(u,v), u*Gy(u,v), u^3*H(u,v))`.

The rational functions share the original positive denominator. Factoring
these known original zero rows before interval division avoids dividing by
a vanishing tangent. The regularized tangent pair is `Su` and `Sv/u`.
A complete positive lower bound for their XY determinant over a pole band
would prove a regular limiting tangent plane. Its normal difference then
has an `u^2` factor and its curvature tensor an `u` factor, with bounded
coefficients. The proof still needs an original-source pole link/quotient
argument and comparison with the actual surrounding supports. A local
normal/curvature limit alone must not be promoted to embedding or volume.
If those literal zero-row conditions fail, inspect the actual lower-order
terms; do not round controls into zeros to manufacture this factorization.

## Original flat-pole quotient and G2 limits

The conditional pole sketch above is now implemented in Rust. The constructor
keeps exact zero polynomial factors through composition and original-cell
extraction. This uses literal input identities, never an epsilon-based control
snap: a genuine minimum-subnormal nonzero coefficient still refuses the flat
factor proof. Public Interval arithmetic and its frozen binary bounds remain
unchanged. A coordinate translation is applied before homogeneous extraction;
where a complete original Bernstein row has only literal zero coordinates,
linear extraction preserves that row as exact zero.

The actual unequal-radius central fixture now proves both original poles.
The first uses 87 cells, an angular upper bound 3.96619389712317 radians and a
positive leading-reference neighborhood radius 0.0005266426647244537 mm. The
second uses 93 cells, angle upper bound 3.994990580598484 radians and radius
0.00047269412507705143 mm. The respective quotient lower bounds are
0.9334104132947293 and 0.9745987714001494. These are local reference-plane
neighborhoods, not a radius claim for the whole corner.

The original positive rational denominator is retained. In normalized pole
coordinates, the literal factors are `(u*Gx,u*Gy,u^3*H)`; regularized tangents
are `Su` and `Sv/u`. Whole-band positive projected determinant bounds prove
regularity away from the collapsed row. The normal error has a bounded `u^2`
factor and the signed world curvature tensor a bounded `u` factor. The
leading fan has monotone nonzero angular speed and total angle below four
radians, so it cannot wind over itself. Its sector is 2-quasiconvex. A bounded
perturbation derivative smaller than one quarter gives a positive lower
Lipschitz bound on an explicitly positive neighborhood. This supplies the
local quotient argument in addition to the differential limits.

Certificates privately borrow the actual original surface and end; source
geometry cannot be substituted. Reverification repeats the complete proof.
Native negative tests retain genuine folds, zero work, nonzero subnormal
coefficients and a winding leading fan as refusals.

## Exterior original two-jet comparisons

The new native path compares original source strips, not an assumed exact
extrusion. In the actual fixture some strip profiles vary along their length
and two pcurves have small but genuine motion in both UV coordinates. Those
coefficients and clocks remain unchanged.

For a curved central boundary, the actual first three V rows must have literal
matching weights and perpendicular coordinates. Their axial displacements
must have a coherent sign; an unowned zero endpoint is refused. This proves
that the boundary two-jet has the geometry of its projected rational profile
extruded along that axis, despite axial shear or scale. A collapsed endpoint
requires a native pole certificate borrowing the very same surface and end,
and the profile's limiting normal must match that owned pole normal.

The source trace is composed from the original rational surface cell and the
actual affine pcurve, restricted by interval blossom to each original central
span. Source and candidate homogeneous profile differences are subtracted
before interval evaluation. Differentiating the cross-multiplied numerator
and denominator encloses correlated first- and second-derivative differences.
A paired-jet computation then bounds normal and complete signed world-tensor
changes. An independent paired comparison bounds the actual source two-jet
against the cylinder over its exact projected trace. The two bounds are
combined by the triangle inequality; neither nearly axial controls nor nearly
constant pcurves are replaced by an ideal model.

For planar connectors, literal constant heights in the original source and
three central boundary rows prove zero curvature tensor. Whole original
Jacobian bounds must have one coherent sign; the owned pole bands cover the
collapsed endpoints. Source pcurves may have degree up to 16: their whole
positive-weight control hull is enclosed, not only their endpoints. Fresh
full three-dimensional curve/pcurve/surface position checks remain mandatory
for both types of boundary.

All six exterior comparisons share a bounded cell and exact-position budget
with the pole proofs. Internal U joins remain separately inspected. These
local results do not prove whole-surface embedding, contacts with neighboring
faces, sewn material topology, thickness, volume, STEP or editor integration.

### Actual exterior qualification matrix

The actual unequal-radius fixture now requires every exterior report to be
`WithinTolerance`, regular, without pending cells, and with fresh whole-trace
G0 evidence. The original tolerances remain position 1e-7 mm, unit-normal
error 1e-5, and signed world-tensor Frobenius error 1e-4 per mm.

| Seam | Orientation to source | Normal upper | Tensor upper | Cells |
| --- | --- | --- | --- | --- |
| 0 curved | Same | 4.496239036402298e-11 | 2.3821986744915733e-6 | 68 |
| 1 planar | Opposite | 0 | 0 | 40 |
| 2 curved | Same | 2.935102360825272e-10 | 4.07846205623794e-5 | 69 |
| 3 planar | Opposite | 0 | 0 | 20 |
| 4 curved | Same | 5.793720427938173e-11 | 1.48181732201604e-6 | 86 |
| 5 planar | Opposite | 0 | 0 | 24 |

The exterior API consumes 487 cells including the 180 cells for both native
pole proofs, within its unchanged shared limit of 6000. All four original
internal joins also remain within the requested normal/tensor tolerances.
This is continuous original-coefficient evidence on this central fixture,
not sampled G2. The stricter generic C2-chart inspector still refuses to
pretend the collapsed rows are regular parameter charts.

The first full regression attempt exposed an unsigned interval division on
a reversed boundary clock. The code now uses the existing signed division;
public interval arithmetic was not changed. Another new regression fixture
incorrectly requested a weight 2e12, beyond the unchanged maximum 1e12. Its
valid largest common gauge is now 5e11 with weights [5e11,1e12,5e11]; the
out-of-range [1e12,2e12,1e12] case is explicitly rejected. Minimum 1e-12
weights and nonuniform rational profiles remain covered.

Detailed pole, internal-join and six-seam evidence is retained in
`exterior-central-qualified.log`. Earlier failing command logs are retained
alongside the final checks instead of overwritten as passing evidence.

### Final native checks and delivery boundary

Final unchanged-source aggregate: **4255 passed, zero failures, six existing
ignores** — brep 2015, nurbs 2032, predicates 100, intersect 108. The final
aggregate command is recorded in `exterior-all-native-verified.log` and exits
successfully. The separate final core run `exterior-nurbs-final.log` also
passes. The no-default-features library check for nurbs-core and brep-core
passes (`exterior-minimal.log`). Map/catalog checks pass with 239 public
modules and 200 catalog entries. Ten Node infrastructure tests pass.

The staged-file architecture audit still fails with exactly 69 unchanged
size errors. None concerns any of the 62 paths changed since origin/main,
including both earlier local Boolean commits. The baseline, numeric limits,
weights, tolerances, work caps and publication gates were not increased.
Both the complete audit output and this scope comparison are retained as
`exterior-architecture-staged.log` and
`exterior-architecture-delivery-scope.json`. This is an actual failed audit,
not a passing global architecture qualification.

No main push is claimed while that required check fails. Other worktrees and
all three stashes are retained. Whole-surface embedding, surrounding-contact
qualification, sewn material topology, corner thickness/volume, WASM/UI and
edited-body STEP delivery remain open and are not inferred from these local
pole and G2 proofs.

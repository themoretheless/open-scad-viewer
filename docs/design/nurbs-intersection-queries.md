# Native NURBS intersection queries

`brep-core::intersections` provides read-only queries against retained rational
definitions. The Rust/WASM bridge and TypeScript adapter expose the same results:

```ts
import {createBrepCylinder} from '../../src/services/geometry/brep'
import {
  intersectNurbsSurfacePlane,
  evaluateIntersectionTrace,
} from '../../src/services/geometry/intersections'

const cylinder = createBrepCylinder(2, 4)
const side = cylinder.faces.find(face => face.surface.degreeU === 2)!.surface
const report = intersectNurbsSurfacePlane(side, {
  normal: [0.2, -0.1, 1],
  offset: 2,
})
for (const component of report.components) {
  if (component.kind === 'curve') {
    const midpoint = evaluateIntersectionTrace(component.trace, 0.5)
    console.log(midpoint.uv, midpoint.point, midpoint.planeResidual)
  }
}
```

Plane equations are `normal · point = offset`; input normals need not be unit
vectors. Distances and residuals use a normalized plane and model length units.
Parameter intervals use the source knot coordinates, rather than a fabricated
normalized domain.

## Supported queries

| Query | Current admitted geometry |
| --- | --- |
| `intersectNurbsCurvePlane` | Validated 3D positive-weight NURBS curves, over the complete active knot domain. Native knot decomposition and bounded homogeneous Bernstein subdivision isolate roots. |
| `intersectNurbsSurfaceSurface` | Two finite affine rectangular patches. Transverse segment or point with paired UV traces; coplanar overlap retains a convex boundary in both UV domains. Curved pairs remain unresolved. |
| `intersectNurbsSurfacePlane` | Untrimmed affine rectangular patches; ruled rational patches linear in V or U, including unequal positive endpoint weights. This includes transverse and oblique cylinder/frustum sections, and generator lines when a cutting plane is parallel to every ruling. |
| `intersectNurbsCurveSegment` | Positive-weight 3D NURBS curve versus a finite nondegenerate segment; source curve parameters, segment parameters and line residuals. Complete coincident intervals are retained; degree-one partial overlaps are clipped in rational source parameters, while higher-degree clipping isolates boundary roots and retains uncertain parameter bands. |
| `intersectNurbsCurveSurface` | Affine rectangular planar support surfaces. Coplanar curves are clipped by isolating the four boundary-plane roots and classifying source-parameter cells; uncertain boundary bands remain unresolved. |
| `intersectionTraceToNurbsCurve` | Algebraic conversion of affine/iso traces, single-tensor-span UV diagonals (result degree U+V ≤25), and bounded multi-span rational ruled sections (curved degree ≤12). Positive output weights, explicit seam/conditioning refusal; no sample fitting. |
| `evaluateIntersectionTrace` | A serialized line or ruled procedural trace returned by a section query. Geometry, plane and source parameterization remain in the trace. |

Surface section curves retain a procedural definition and original UV trace.
The nine report samples provide residual evidence and convenient previews;
they do not define a fitted intersection curve. Query results contain no mesh
geometry, and triangles do not participate in intersection decisions.

## Evidence and refusal behavior

Every report contains `coverage`, `unresolved`, `boxesVisited`,
`bernsteinExcluded`, `evidence: 'numerical_uncertified'`, and
`permitsTopologyChange: false`. `numerically_resolved` means that the numerical
query handled all regions without pending work. It does **not** mean that an
independent completeness certificate has been established.

Root events retain isolating parameter intervals and reevaluated plane
residuals. Surface branches retain their supporting parameter boxes. Tangent or
multiple roots, precision ambiguity, curved unsupported support surfaces,
coincident trim clipping, and exhausted budgets remain explicit unresolved
regions. An empty component list with unresolved regions must not be interpreted
as disjoint geometry. Distinct nearby roots are not merged by world-space or
parameter tolerances.

Default options are absolute distance tolerance `1e-9`, absolute parameter
tolerance `1e-10`, maximum subdivision depth 48 and 8192 visited boxes. Positive
finite tolerances, depth 1–64 and box budgets 1–65536 are accepted. Surface
reports are additionally bounded to 1024 components. Reaching a budget returns
an incomplete report, preserving pending regions. Tolerances do not grow
during the query.

Outward interval arithmetic protects sign exclusion on the current Bernstein
coefficients. An independent bound on preceding knot-edit roundoff, certified
root and branch correspondence, tangency classification, UV trim arrangements,
branch sewing, and global coverage verification remain unimplemented. The
queries do not produce a trimmed B-rep section or enable general curved
Booleans. In particular, a report for an underlying cap surface still needs to
be clipped against that face's circular or polygonal trims.

Native tests cover three-root curves, rational arcs, close roots under knot
insertion/reversal/degree elevation, tangencies, budgets, affine clipping,
oblique cylinder/frustum sections and generator lines. WASM tests additionally
exercise serialized trace roundtrips and explicit evidence/refusal fields.

### Ruled surface sections in either parameter direction

Surface/plane queries now admit ruled rational surfaces linear in U with matching
row weights, in addition to the existing linear-V case. The numerical algorithm
runs on the transposed retained surface and maps parameter boxes, unresolved
regions, samples and procedural traces back to original UV coordinates.
The new serialized `ruled_u` trace retains the source surface and V interval;
the TypeScript trace union includes this representation.

Tests cover horizontal/oblique cylinder sections and generator lines with shifted,
scaled knot domains, serialized trace evaluation, original-surface residuals and
budget-exhausted UV regions. All 10 intersection unit tests pass in debug and
release; native workspace check passes. Transposed floating-point evaluation can
differ in the last bit, so geometry is checked numerically, not claimed exact.
Coverage remains numerical_uncertified with topology changes forbidden. Browser
WASM packaging has not been rebuilt in this checkpoint; general surface/surface
intersection and curved Boolean certification remain open.

### Linear-U sections reach the packaged WASM adapter

The geometry WASM was rebuilt with linear-U sections. Four TypeScript/WASM
intersection tests pass, including JSON roundtrip of ruled_u traces, shifted
UV domains, cylinder section residuals, generator lines, invalid fractions and
budget-exhausted parameter regions. vue-tsc --noEmit passes. The packed geometry
literal was decoded through verifyPackedWasmChunk and matched the raw module
byte-for-byte (6,446,961 bytes). This verifies the generated adapter artifact;
a fresh production dist/browser interaction was not exercised here. General
curved Boolean and certification work remains open.

### Whole-domain admission of retained section traces

Trace evaluation now validates both line endpoints or both ruled interval ends
against the retained surface domain before interpolation. A valid requested
fraction cannot hide a malformed unused endpoint. Finite, in-domain reversed
intervals remain admitted for oriented traversal. Linear-U traces inherit the
same checks through parameter transposition.

All 11 native intersection tests pass in debug/release, including nonfinite and
out-of-domain endpoints and reverse traversal. Geometry WASM was rebuilt; all
four adapter tests pass with malformed serialized U/line traces rejected even
at fraction zero. vue-tsc --noEmit passes. These are parameter admission checks,
not a geometric completeness certificate; general curved Boolean remains open.

### Rational rulings with unequal endpoint weights

Surface/plane sections now admit linear-U/V rational rulings with unequal
positive endpoint weights. Trace evaluation computes both boundary homogeneous
weights using one normalization and solves the weighted plane equation for the
ruling parameter. Control-coefficient exclusion and unresolved-region handling
remain in place; no tessellation or fitted curve is introduced.

Native tests verify both parameter directions and serialized traces against the
analytic half-height parameter 1/4 for a 1:3 boundary weight ratio. WASM tests also
vary the ratio along the surface and compare UV against an independent quadratic
Bernstein formula. All 12 native intersection tests pass in debug/release; after
WASM rebuild all five adapter tests pass, and vue-tsc --noEmit passes. Query
coverage remains numerical_uncertified and cannot authorize topology changes.
General curved surface/surface Boolean remains incomplete.

### Knot-refined linear-U surfaces

Surface/plane orientation dispatch now uses degree one in U/V rather than
requiring exactly two global control rows/columns. Knot-refined linear-U
surfaces are transposed and decomposed into retained knot-span patches before
sectioning, so geometry-preserving knot insertion no longer disables support.
Tests insert a midpoint knot in a cylinder's linear direction and verify
sections on both sides, original UV, patch parameter boxes and cylinder residuals.

All 13 native intersection tests pass in debug/release. Rebuilt WASM passes all
six intersection adapter tests; vue-tsc --noEmit passes. This does not establish
sewing/completeness at knot-boundary contacts or authorize topology changes;
full curved Boolean and B-rep certification remain incomplete.

### Sections coincident with a ruling-span boundary

When one entire boundary coefficient row lies numerically on the plane and the
opposite row has a strict Bernstein side, sectioning emits the retained UV
isocurve instead of repeatedly subdividing an unresolved boundary band. A
continuous interior knot is owned by the preceding span to avoid duplicate
curves. This uses the query's existing numerical zero convention, not a new
exact certificate. Disconnected full-multiplicity knots remain rejected by
NURBS admission; they require separate geometry nodes.

All 14 native intersection tests pass in debug/release, including a section
exactly through an inserted knot and disconnected-input rejection. Rebuilt WASM
passes six adapter tests, now covering both exterior boundaries and the shared
knot. General branch sewing, curved Boolean and full certification remain open.

### Generator sections with proportionally weighted boundaries

The generator reduction now also recognizes a finite positive common ratio
between boundary weights when corresponding control points have equal plane
distances. In this admitted numerical case both boundary plane equations share
the same roots, so unequal weights no longer force an unresolved subdivision
band. The retained surface still controls rational traversal along the ruling.

All 15 native intersection tests pass in debug/release. Analytic cylinder tests
cover U/V orientation and z(t)=12t/(1+2t) for a 1:3 weight ratio. Rebuilt WASM
passes seven intersection adapter tests; vue-tsc --noEmit passes. The common
ratio/equality checks are numerical and do not establish certified completeness
or authorize topology changes. General curve/surface and surface/surface work
remains open.

### Nonproportional-weight section oracle

A regression now varies upper boundary weights by 2, 3 and 5 and cuts the
result with a vertical plane. It verifies that nonempty retained ruled branches
are not mislabeled as whole generator lines. Samples are checked against an
independent quadratic rational Bernstein formula for X and Z. With 128 boxes,
endpoint branch bands remain explicitly unresolved and coverage is incomplete;
this is an observed remaining solver limitation, not a complete section claim.

All 16 native intersection tests pass in debug/release, and eight tests pass
through the current WASM adapter. No runtime algorithm or WASM artifact changed
in this checkpoint. Complete endpoint isolation/branch joining and general
surface intersection remain required for the full B-rep objective.

### Fair bounded traversal across surface spans

A failing regression demonstrated that depth-first section traversal exhausted
eight boxes refining a difficult second span before processing a simple first
span. Surface subdivision now uses a FIFO queue: original spans are visited
before child refinement, then subdivision proceeds by depth. Existing box and
component budgets and explicit unresolved regions remain. Component traversal
order may change; components have no persistent topology identity.

The regression now retains the entire simple span within the same eight-box
budget. All 17 native intersection tests pass in debug/release, rebuilt WASM
passes nine adapter tests, and vue-tsc --noEmit passes. Fair scheduling improves
partial results; it does not resolve endpoint bands or certify completeness.
General intersection/branch joining and full B-rep remain open.

### Fair curve/plane isolation across knot spans

A regression reproduced starvation of an exact endpoint in a later curve span
while four boxes were consumed refining earlier roots. Curve/plane isolation
now uses one FIFO queue across source spans and their subdivisions. Source-span
coefficients are computed lazily after the box-budget check. Existing root
sorting, endpoint admission, initial-span overlap detection and explicit
unresolved outcomes remain in place.

The later endpoint is now retained within the same four-box budget. All 18
native intersection tests pass in debug/release. Rebuilt WASM passes ten adapter
tests; vue-tsc --noEmit passes. The query remains numerically uncertified;
fair scheduling does not establish full root/branch completeness or B-rep
solid certification.

### Native/WASM NURBS curve versus finite segment

Added curve_segment and intersectNurbsCurveSegment, extending the finite CC
query matrix beyond plane targets. Two supporting plane queries share the box
budget. Points retain source curve intervals, segment parameters and geometric
line residuals. Positive-weight control hulls handle finite segment admission;
whole coincident intervals are retained, while partial coincidence and uncertain
boundary bands remain explicit unresolved regions. No polyline approximation
or topology-changing certificate is introduced.

Native tests cover transverse hits, skew/disjoint cases, finite exclusion,
whole/partial overlap, shared-budget exhaustion, degenerate segment rejection,
and a rational circle arc with reversed segment correspondence. All 20 native
intersection tests pass in debug/release; native workspace check passes.
After bridge integration and WASM rebuild, eleven adapter tests pass and
vue-tsc --noEmit passes. General curve/curve pairs, coincident clipping,
certified root correspondence and full curved Boolean remain incomplete.

### Degree-one rational coincidence clipping

Curve/segment queries now clip partial degree-one coincident intervals by
inverting rational parameterization with normalized endpoint weights. Both
segment directions preserve ascending source-curve parameter intervals;
endpoint-only contact is returned as a point. A finite overlap whose parameter
interval collapses numerically remains unresolved. Higher-degree partial
coincidence remains CoincidentTrim rather than being approximated.

For weights 1:3, the independent analytic fixture maps geometric [1/4,3/4]
to source parameters [1/10,1/2]. Tests cover reversal and endpoint-only contact.
All 21 native intersection tests pass in debug/release; rebuilt WASM passes 12
adapter tests and vue-tsc --noEmit passes. This extends the numerical finite
query matrix, not certified B-rep topology operations; general coincidence and
curved Boolean remain open.

### Shared-knot contact uniqueness after rational clipping

A failing regression returned two identical parameter events for a curve that
touches a segment endpoint at its internal knot. Clipped singleton contacts now
use the known segment boundary parameter and reject duplicate source-parameter
events. Deduplication does not compare spatial coordinates: a second regression
keeps two visits to the same point at distinct curve parameters 1/4 and 3/4.

All 22 native intersection tests pass in debug/release; the extended repeated-
visit regression also passes in both profiles. Rebuilt WASM passes 13 adapter
tests and vue-tsc --noEmit passes. This fixes query event identity within a
single curve, not persistent topology naming or certified contact semantics.
Full B-rep completion remains unproved.

### Higher-degree coincident clipping with explicit root bands

Partial curve/segment coincidences of higher degree now isolate roots against
both segment boundary planes under the existing shared box budget. Source
parameter cells are classified using positive-weight control hulls; admitted
interior cells retain their original curve intervals. Root uncertainty and
unresolved solver regions remain explicit bands, never collapsed to guessed
trim parameters. Exact numerical boundary events are retained when not already
covered by an overlap interval. Cell classification also consumes box budget.

The independent x(t)=t² fixture maps segment [1/4,9/16] to [1/2,3/4], including
reversed segment direction. Nondyadic roots and low budgets remain incomplete.
All 23 native intersection tests pass in debug/release; rebuilt WASM passes
14 adapter tests and vue-tsc --noEmit passes. These numerical query results
still cannot authorize B-rep topology edits. General certified coincidence,
curve/curve and surface/surface completeness remain open.

### Backtracking coincidence and authoritative trim endpoints

An independent x(t)=4t(1-t) regression exposed a false CoincidentTrim band for
one of two visits to the same segment: knot insertion rounded the trimmed
control endpoint outside the boundary. Cell hull classification now evaluates
its two endpoints on the authoritative source curve at the retained parameters,
while retaining interior trimmed controls. No tolerance snapping was added.

The two visits are retained separately as [1/8,1/4] and [3/4,7/8]. Nondyadic
fixtures independently verify all four analytic boundary roots are covered by
unresolved bands and accepted intervals remain inside the segment. All 24
native intersection tests pass in debug/release; rebuilt WASM passes 15 adapter
tests and vue-tsc --noEmit passes. These remain numerical checks, not certified
trim or solid completeness; the full B-rep goal remains open.

### Coplanar NURBS curve clipping to affine surface domains

Curve/surface queries now clip coplanar curves to finite affine rectangles by
isolating roots against all four boundary planes, using the frame's dual vectors
and a shared query budget. Retained source-parameter cells are classified by
positive-weight control hulls with authoritative endpoint evaluation. Curves
along rectangle edges remain admissible; unresolved root bands remain explicit.
Isolated admitted boundary contacts are retained when no overlap covers them.

The x=t,y=t² fixture is clipped to [1/4,3/4] with shifted/scaled surface knot
domains; edge coincidence and two-box refusal are also tested. All 25 native
intersection tests pass in debug/release. Rebuilt WASM passes 16 adapter tests;
vue-tsc --noEmit passes. This is finite affine-domain numerical clipping, not
general lifted UV arrangements, curved-face trimming or certified Boolean.

### Affine clipping shear and corner evidence

Additional native and WASM regressions verify the same source interval after
shearing both the curve and rectangle, and a single parameter/UV event when a
curve touches the common corner of two boundaries. This exercises the dual-frame
projection and boundary-event deduplication rather than only axis-aligned cuts.

All 26 native intersection tests pass in debug/release; 17 tests pass through
the existing WASM adapter, and vue-tsc --noEmit passes. Runtime implementation
and WASM bytes did not change in this checkpoint. The main matrix now also
records finite-segment and affine-domain coincidence clipping; general lifted
UV arrangements and certified B-rep completion remain open.

### Finite affine surface/surface query in Rust and WASM

Added surface_surface / intersectNurbsSurfaceSurface for two finite affine
rectangular patches. Transverse segments retain a trace on each source surface;
the same fraction evaluates corresponding UV/3D points. Finite clipping also
returns isolated contact points and disjoint results. Coplanar area overlap and
curved pairs remain explicitly unresolved. Pair unresolved boxes concatenate
both original UV domains. Correspondence residuals are sampled and numerical;
reports continue to forbid topology changes.

All 27 native intersection tests pass in debug/release, including operand swap
and shared budget. Rebuilt WASM passes 18 adapter tests, covering paired trace
JSON replay, finite endpoint contact, bounded disjointness, parallel separation,
coplanar refusal and unsupported curved pairs; vue-tsc --noEmit passes. General
surface intersection, coplanar areas and certified curved Boolean remain open.

### Coplanar affine surface overlap with paired UV boundaries

Surface/surface queries now clip a coplanar affine rectangle against the four
boundary halfspaces of the second patch. Area results retain an implicitly
closed convex polygon in both UV domains and source-evaluated 3D vertices;
edge and point contacts retain their lower-dimensional result types. Each
clipping pass consumes the shared box budget. Numerically degenerate area or
failed correspondence remains unresolved rather than becoming a solid face.

The square/diamond oracle produces eight vertices and area 3.5. Self-overlap,
shared edge, corner, disjointness and budget limits are covered. All 28 native
intersection tests pass in debug/release; rebuilt WASM passes 19 adapter tests
and vue-tsc --noEmit passes. Release/WASM compilation was slow but verified
live and completed without restart. General curved surface pairs, UV/DCEL
assembly and certified B-rep operations remain incomplete.

### Coplanar pair invariance and UV correspondence matrix

Eight combinations of operand swap, U reversal and UV transposition now verify
the same square/diamond intersection. Source knot domains are shifted/scaled
and one surface's weights are uniformly scaled by 32. Each case checks eight
vertices, physical area 3.5, containment in both analytic polygons, and equality
of each stored 3D vertex with evaluations in both returned UV systems. Boundary
ordering may follow the first UV orientation; geometry is not compared by array
order or guessed identity.

All 29 native intersection tests pass in debug/release; 20 tests pass through
the existing WASM adapter and vue-tsc --noEmit passes. Runtime code and WASM
bytes did not change. The main completion matrix records the finite affine SS
capability; general curved pairs and certified solid completion remain open.

### Algebraic section-trace conversion to NURBS curves

SurfaceTrace::to_curve and intersectionTraceToNurbsCurve now convert supported
traces to retained rational curves without fitting samples. Affine traces and
isoparametric lines use native curve extraction; single-Bezier ruled sections
form homogeneous Bernstein products of the two boundaries and plane equations.
The resulting degree is twice the curved direction's degree, bounded to 24;
a positive-weight representation and normal NURBS validation are required.
Unsupported degree/weight representations fail explicitly. Multi-span support
was subsequently added as described below.

Tests compare converted curves to retained procedural traces in U/V orientations
with unequal boundary weights, oblique/horizontal cuts and generators. Independent
cylinder/plane residual checks use the query's 1e-9 tolerance; generator roots
already carry numerical isolation error and are not claimed exact plane points.
All 30 native intersection tests pass in debug/release, rebuilt WASM passes
21 adapter tests, and vue-tsc --noEmit passes. Floating-point algebra is not a
certified construction, and general curved B-rep completeness remains open.

### Reversed section conversion and ambiguous-ruling regression

Conversion regressions now cover reversed trace fractions on shifted U/V knot
domains for both surface orientations and generator/isoparametric sections.
Converted NURBS points agree with the original trace at the reversed fraction
within 1e-10. A ruling family whose endpoint evaluations succeed but whose
homogeneous conversion has mixed-sign weights is explicitly rejected. This is
a conservative representation refusal, not a claim that endpoint sampling
certifies the interior.

The debug and release intersection suites each pass 32 tests; the existing WASM artifact passes
23 adapter tests, and vue-tsc --noEmit passes. These changes add regression tests
only and require no new runtime artifact. General curved intersections and
certified topology-changing operations remain incomplete.

### Multi-span ruled-trace conversion

Ruled trace conversion now decomposes the requested interval at source knots,
converts each Bezier span algebraically, and assembles one C0 rational curve
with the original parameter domain. Adjacent spans rescale their homogeneous
weights to share an endpoint weight. A shared endpoint is admitted only when
its Cartesian control coordinates are numerically identical; no proximity
welding is performed. Nonmatching endpoints, invalid weight conditioning and
results exceeding 256 control points are explicitly rejected. Reverse traversal
is applied to the assembled curve. This extends retained section geometry,
without certifying intersections or publishing new B-rep topology.

Validation: 33 native intersection tests pass in debug and release, including a valid
degree-12 surface whose converted representation exceeds the output budget.
The rebuilt WASM passes 24 adapter tests; TypeScript checking and packed-WASM
byte verification pass. Multi-span point comparisons include interior knot
locations and reversed traversal.

### Varying-weight multi-span regression

Native and WASM tests additionally cover independently varying weights on both
ruled boundaries with an oblique cutting plane, transposed surface orientation
and reversed traversal. Source-trace agreement and an independent plane residual
are checked to 1e-10, including the interior knot. The intersection suite passes
34 tests in release; the WASM adapter passes 25 tests and TypeScript checking
passes. This regression adds no runtime code and uses the previously rebuilt
artifact.

The full debug command `cargo test --offline --locked -p brep-core
-p brep-topology -p nurbs-core --manifest-path crates/Cargo.toml` completes with
150 passing tests and no failures. This covers current constructor, supported
Boolean, mass-property, identity, transaction and NURBS regressions, in addition
to the query tests; it does not prove the open completion-matrix requirements.

### Tensor-patch UV diagonal lifting

`SurfaceTrace::Line` now converts non-isoparametric UV segments on a single
tensor Bezier patch to a rational curve of degree p+q (maximum 25). The source
is trimmed to the segment's UV bounding rectangle; homogeneous Bernstein
products restrict its tensor polynomial to the diagonal, accounting for both
UV directions. Output parameter fraction is [0,1]. No sampled fitting is used.
Multi-span diagonals remain an explicit refusal; iso-parametric and affine
paths retain their existing behavior. A zero-length UV segment produces a
constant degree-one curve.

This operation lifts the supplied UV path. It does not establish that a
caller-supplied path lies in the trace's plane; evaluated plane residuals remain
separate numerical evidence. Sphere tests independently verify the radius,
source agreement, partial shifted domains, all four UV directions, degree limits
and multi-span refusal. General curved intersection and trim certification
remain open.

Validation: 35 native intersection tests pass in debug and release. The rebuilt WASM passes
26 adapter tests; TypeScript checking and packed-byte verification pass.

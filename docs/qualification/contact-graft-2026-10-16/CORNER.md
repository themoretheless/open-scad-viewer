# Four-sided rational corner construction

`nurbs_core::corner_blend::g2` constructs one tensor-product rational patch
from four complete natural support boundaries, ordered UMin, UMax, VMin, VMax.
Each support has its own orientation and positive transverse gain. There is no
equal-radius assumption or sphere substitution. Radius laws are not inferred.

`g2_traces` additionally accepts a fixed original cross-parameter and a retained
along-parameter interval for each support. These may lie inside the original
chart. Trimmed numerical homogeneous fields are only construction proposals.
The final correlated proof evaluates original knot-cell jets at the actual
trace parameter, including cross derivatives through order two; it never treats
a rounded cropped support as the original. Retained-side span selection and
stored C2 requirements apply at internal source knots. Tests cover interior
partial traces on extended polynomial/rational supports, nonunit charts and
cuts at a C2 internal source knot, while asserting immutable original geometry.

The author extracts homogeneous position and first/second transverse derivative
fields from each original support. Opposing along-boundary fields are degree
elevated and aligned in a common basis. Constant positive projective gauges
align corner weights without changing Cartesian support geometry. Three control
layers are imposed at each boundary; inconsistent overlapping corner blocks
refuse. Optional unconstrained interior controls use a homogeneous bilinear
proposal. Internal knot multiplicities must retain at least C2 continuity.

The compatibility tolerance is only a construction gate. It never grants
continuity. All four final boundaries are independently audited against the
original support definitions using continuous position, normal and world
curvature tensor bounds, within a shared maximum of 100,000 cells and 2,000,000
Bernstein coefficient products and 1,000,000 exact-predicate work. Original homogeneous jets are enclosed on knot
cells, converted to rational Cartesian jet polynomials, and subtracted before
interval evaluation. Bounds on these correlated residuals control cross
products, normalized normals, dual tangent vectors and the world curvature
tensor. This avoids comparing two independent moving spatial boxes. Zero work,
an unresolved seam or a failed seam cannot set `within_tolerance`.
Source controls and final tensor controls are bounded at 4,096 each.
Elevated piecewise fields can be refitted into a C2 proposal basis; only the
final original-support proof admits the resulting surface. Original interior
knots now require fresh exact affine identities before entering the common
seam partition. Nonunit domains and reversed multispan charts qualify when
those identities hold. A merely rounded normalized knot and a non-C2 original
seam basis remain explicitly unresolved. Span selection uses containment of
the complete common interval, avoiding a rounded midpoint crossing a knot.
`g2_with_exact_limit` lets the B-rep operation share its exact work ledger with
the subsequent full-shell audit. Zero exact work cannot qualify a multispan
knot map; no unexamined interval is silently removed.

Regression cases include polynomial and rational source patches, unequal
directional curvature, reversed along-boundary orientation, nonunit transverse
parameter scale, incompatible jets and deliberately permissive construction
tolerance followed by strict original-support auditing.

## Whole-candidate interior admission

`inspect_candidate` freshly verifies an actual candidate against all original
support traces, then audits its entire chart with the original-surface integer
linear projection certificate. A successful certificate proves a regular
injective chart; its work is charged to the same 100,000-cell allowance. Stored
interior knots must preserve C2. Failure of this sufficient certificate remains
unresolved, rather than being labeled a detected self-intersection.

`complete` and `within_tolerance` retain their seam-only meaning. The new
`admitted` flag additionally requires `interior.certified`.
`corner_close::close` requires both seam and interior certificates before
assembling a closed body candidate. The manifest records these separately from
the still-false global body embedding flag.

An adversarial degree-six plane patch moves the central x control by 8. Its
added term is `8 B_3^6(u) B_3^6(v)`: all four position/first/second boundary jets stay
unchanged, while the interior x derivative becomes negative at (0.75,0.5).
The fresh four-seam G2 audit passes, but the whole-chart audit does not certify
it, so `admitted=false`. The positive polynomial/rational and owned closed
controls still admit. Caller control-count and shared work budgets remain gates.

## Contact diagnostics of the inserted face

`face_contacts::inspect_incident` inspects every pair involving one selected
face using the same immutable-model traversal, original surfaces and cumulative
budgets as the full audit. Its separate `IncidentReport` exposes incident-only
classification flags, first unvisited incident pair and explicit scope. It
does not excuse adjacent faces or infer a body-wide result. Regression cases
compare first/middle/last selected faces to full traversal and preserve the
first unvisited pair when the pair budget is exhausted.

The baseline incident search of the interior-trimmed control has 29 pairs incident to inserted face 29.
Twenty-one are proven disjoint. Eight remain unresolved: [1,29], [3,29],
[5,29], [7,29], [10,29], [12,29], [14,29], [16,29]. Four share owned edges and
four share only owned vertices. These relationships are diagnostic context,
not certificates authorizing removal of the unresolved cells. Every pair was
visited; there is no unvisited suffix. Contact search used 8,021 geometry cells
and 67,730 UV-domain cells. The fresh face audit proves injectivity of all 30
original face charts in 30 spans, with no linear refinement cells.

`corner_contact_diagnostics <report.json>` records this result with the actual
serialized model's SHA-256. The report is in
`output/qualification/corner-close-interior-2026-10-16/contact-diagnostics.json`.
Its global-body claim remains false. Full-body pairs outside the inserted
face's scope, precise ownership of the eight remaining contacts and thickness
qualification are still required.

### Metric locality for the four owned-edge neighbors

`trimmed_edge_contact::inspect` freshly recognizes the actual hole-free source
UV rectangle, verifies both saved uses of the actual shared 3D edge, and bounds
strict coordinate flow on the immutable original surface. The opposite face's
positive control hull supplies a global coordinate bound. Any real penetration
of that hull is included explicitly; it is not rounded away. With lift error
delta, hull penetration eta and original flow factor M/L, every possible pair
contact is within `(M/L)*(delta+eta)+delta` of the actual owned edge. Both trim
regions are independently audited before this metric certificate is returned.

The inserted-corner control qualifies [3,29], [7,29], [10,29], [14,29] with maximum
distance upper 3.0177669529663795e-8 mm, below the existing 1e-7 mm model tolerance.
The y-min control hull penetration is 1.1102230246251568e-16 mm and is charged
to that same bound. Added work is 94 cells and 67,702 exact work, shared with
the existing construction/contact diagnostic allowance. Tests cover all four
edges, zero work, a vertex-only pair, source immutability and a hull crossing
large enough to prevent qualification.

These are contact-locality certificates, not exact contact ownership or absence
certificates. The edge-locality method alone leaves all eight exact statuses unresolved. The four
vertex-only pairs are handled by the separate certificate below. The diagnostic
JSON records `metricLocality` separately and leaves exact/global flags unchanged.

### Metric locality at the four owned vertices

`trimmed_vertex_contact::inspect` admits only one shared actual vertex and a
literal source UV rectangle. Two distinct projected coordinates must be
literally independent of the opposite source parameter, including the source
weights; a small sampled cross derivative cannot substitute for this property.
Fresh original-span coordinate flow bounds both legs from any possible contact
to the corner. Constant-curve/constant-pcurve checks independently bound the
actual vertex's positions on both original faces. Both trim regions are freshly
audited. Positive opposite-face control hull bounds include actual penetration.

If delta bounds vertex position error and eta_i bounds hull penetration, the
contact distance is at most `delta + sum_i (M_i/L_i)*(delta+eta_i)`. This is a
metric proximity certificate, not a claim of exact contact ownership or a
complete general NURBS vertex classifier. Coupled coordinate fields, multiple
shared vertices and insufficient work remain unqualified.

The control now qualifies [1,29], [5,29], [12,29], [16,29] within
1.10382737213861e-8 mm of owned vertices 21,22,25,26. Together with the four edge
certificates, all eight previously unresolved incident pairs have metric
locality bounds within the unchanged 1e-7 mm tolerance. Combined edge/vertex
locality work is 192 cells and 98,118 exact work. Tests cover every vertex,
zero work, source immutability, an edge-neighbor refusal, excessive hull
penetration and failure of literal coordinate independence.

The exact incident-pair flag and global-body flag remain false. The 21 disjoint
pairs retain their exact search results. The JSON stores `metricVertexPairs`
and owned-vertex balls separately from original exact pair statuses, bound to
the same actual model SHA-256.

### Exact owned-vertex refinement

`exact_vertex_contact` requires zero hull penetration in both projected
coordinates and fresh exact identity decisions for the actual owned vertex on
both original surfaces. Strict independent coordinate flow then forces both
source parameters to the actual corner. Thus every pair contact equals the
owned vertex, and exact lifts put that vertex on both actual outer boundaries.
A small metric upper bound alone cannot grant this flag.

`inspect_incident_with_vertices` promotes only these fresh exact certificates
to `SharedBoundary::OwnedVertex`, reason `shared-vertex`. Their actual geometry
and exact work are charged cumulatively; `vertex_exact_work` names the extra
vertex work separately from legacy search work. Zero additional exact work
cannot promote a pair. Exact vertex contact remains a real contact, so this
never promotes disjointness or a full-body volume/embedding certificate.

The current control classifies [5,29] and [16,29] exactly at vertices 22 and 26.
Its native incident results are now 21 disjoint, 2 exact shared-vertex contacts
and 6 unresolved. The six remaining pairs retain 4 edge-tube and 2 vertex-ball
metric certificates. Exact refinement adds 98 geometry cells and 30,416 exact
work to the baseline search. The following metric checks use 144 cells and
82,188 exact work. The JSON records `exactVertexPairs`, `exactOwnedVertex` and
`vertexExactWork`; model hash and the false global-body flag remain unchanged.
Regression checks prevent promotion with zero exact work or tiny nonzero hull
penetration, and preserve legacy incident/full traversal behavior.
The combined vertex primitive conservatively charges all its geometry/trim
work to the remaining UV allowance too. A caller allowing only one additional
UV work unit cannot obtain either new exact classification. The diagnostic
reserves construction and all subsequent locality work from both allowances.

## Selected opposing-face clearance

`wall_clearance::inspect` bounds the global minimum distance from one original
trimmed face to every face in a supplied opposing set. It shares 100,000
geometry and 1,000,000 UV-domain cells across all pairs. Unvisited targets stay
explicit and contribute zero to the global lower bound. An upper bound always
references an admitted original-face witness, including parameters and spatial
enclosures. A below-threshold witness may prove a defect even with incomplete
coverage; a passing decision requires complete selected-set coverage, a finite
lower bound and an admitted witness. Threshold equality inside an interval
remains unresolved. Global interval convergence is reported separately.

The control's inserted face is checked against all nine original bottom faces.
Its global interval is [2.9999999999999987, 3.0000000000000018] mm, with all nine
pairs contributing and a witness on target face 0. Work: 6,503 geometry cells,
33,028 UV-domain cells. Required clearance 2.9 mm is confirmed. A separate
closed control moves the complete bottom boundary to z=-0.05, preserving all
edge/vertex/side-face incidence; the same scan finds 0.05 mm below a 0.1 mm
requirement. Regressions cover zero work, unvisited targets, threshold equality,
duplicate targets, complete coverage, witness availability and immutability.

`corner_wall_diagnostics <report.json>` writes the model-bound native result to
`output/qualification/corner-close-interior-2026-10-16/wall-clearance.json`.
`scripts/verify-corner-wall-occt.py <directory>` independently identifies the
corner and nine floor faces by their STEP geometry and measures 3.0 mm using
OpenCascade. Its 1e-7 mm comparison allowance is explicitly separate from the
native interval. The report is `wall-clearance-occt.json`; both model and STEP
hashes are checked.

This certifies clearance to the supplied face set, not automatic wall pairing,
normal-ray thickness, material-chord containment or thickness of the entire
solid. Those claims remain false and are still required by the roadmap.

## Closed native and independent STEP control artifacts

`brep_core::corner_close::close` fills the sole four-edge opening of an owned
open shell. It reuses original rim edges and vertices, checks both the original
and new saved face uses continuously, audits trim regions, and requires fresh
whole-shell closure. Existing vertices, edges, loops, faces and their identities
survive; the shell gets Modify lineage and a new body is recorded. No input is
mutated on failure. Source rim uses currently must be clamped linear
equal-weight isoparametric pcurves. Each original rim supplies its exact fixed
parameter and actual endpoint interval, including partial traces inside a
larger source chart. This ensures that the actual owned seam carries the
same affine correspondence as the G2 proof. General nonlinear rim pcurves
cannot inherit that proof and refuse. Corner-cycle planning and arbitrary
trimmed sides remain.

### Interior trimmed-source STEP control

The second control stores every original top face on the same extended
surface, chart [2,8]². Existing face loops retain their subregions. Its four
corner rim edges are at fixed parameters 4 or 6 over [4,6], strictly inside
that original chart. Inserting the corner preserves all original faces and
their surface definitions, edges, vertices and identities. New retained-face
planning is not implied: the input shell already owns the trimmed opening.

Native run: 317 cells, 36,716 coefficient products and 230,747 exact work.
Independent OpenCascade imports a valid solid with 30 faces, 60 unique edges
and 32 unique vertices, volume 30.375000000000007 mm³. Analytic volume error
is 7.105427357601002e-15 mm³; maximum bounds error is
1.0000000005838672e-7 mm. Artifacts:
`output/qualification/corner-close-interior-2026-10-16/`.
STEP SHA-256: `39d87232fc8f9df6e5c3d0ea6b024724234fc2e2429dc9ac9b046bf7eff3453b`.
Reproduce with `corner_close_acceptance <directory> extended`, then the same
independent verifier. The global native embedding and radius flags remain false.

### Original natural-boundary STEP control

The original closed control is bounded by bottom z=-3, x/y in [-1,2], and a top of
z=x²/4+y²/8. It starts with one missing top face; the corner operation creates
the thirtieth face. All four inserted seams pass the continuous G2 bounds.
The analytic design volume is 27 + 9/4 + 9/8 = 30.375 mm³.

Native run: 317 cells, 36,716 coefficient products, 46,099 exact work. Independent
OpenCascade STEP validation: one valid solid, 30 faces, 60 unique edges and 32
unique vertices. Volume 30.375000000000007 mm³; volume error
7.105427357601002e-15 mm³; bounds error 1.0000000005838672e-7 mm under the independent
1e-6 mm acceptance threshold. Artifacts are in
`output/qualification/corner-close-2026-10-16/`.

STEP SHA-256: `d5b7e7f2d32af3170b963bb8a135c51fefec64d0eaa87ba2f8673995f3dfc607`.

Reproduce with `corner_close_acceptance`, then
`scripts/verify-cad-step-occt.py <directory> --no-preview` in the OCP environment.
The artifact manifest explicitly leaves native global embedding and whole
radius claims false. Independent OCCT validity does not promote those flags.

### Projected 3D cuts on original retained faces

`face_cut::partition_projected` accepts one or more owned 3D cut segments and
projects them into the selected immutable original surface. It binds the first
and last UV endpoint to the original outer-loop endpoint authority and makes
internal projected endpoints agree. Those edits invalidate the proposal proof:
the actual saved pcurves, both resulting trim sectors and hole assignments are
freshly checked by `partition`. Original curves, surface definitions and shared
source edges survive. Work is shared across projection and partition with a
100,000-cell limit; exhaustion refuses atomically.

The face-cut and graft regression run passes nine tests. Projected cuts are
exercised on a bowed rational cubic, a two-segment cut sharing an interior vertex,
and a nonplanar original surface after shared-edge root partitioning. Tests also
check closed boundary admission, unchanged original surfaces and input immutability.
An off-surface cut and insufficient work refuse. Endpoints still need explicit
original vertex ownership; periodic charts, unresolved projection and ambiguous
material sectors remain unsupported. This stage builds both sectors; it does not
automatically discard material or construct a prescribed-radius fillet corner.

### Multiple projected cuts on one original face

`face_cut::partition_projected_network` now tracks descendants of one original
face across several cuts. For every next cut it infers the unique current sector
from owned endpoint occurrences, projects the 3D segments and freshly audits the
result. Geometry work is shared across the full network (at most 100,000 cells),
and exact agreement work uses the remaining shared allowance (at most 1,000,000).
The accumulated change set is relative to the original input. The operation keeps
both sides of each cut; it prepares topology for material planning and grafting.

The regression uses a bowed rational cubic followed by a curved quadratic sharing
one original vertex. Its three descendant faces keep the original surface and
source edge identities; the assembled boundary passes continuous closure checks.
Repeated ambiguous cuts, a crossing endpoint pair and exhausted work refuse
without changing the source. The face-cut and contact-graft run passes ten tests.

The initial network exposed an incomplete adjacent-curve separation test. The
trim simplicity checker now proposes normalized control-cone separators and
checks strict outward interval signs for every non-join pole of both original
rational curves. At most 256 directions are proposed; failure leaves the pair
unproven. It does not infer separation from tangents or sampled points. Positive
rational basis weights and existing curve injectivity checks remain mandatory.

This removes manual descendant-face bookkeeping, but does not complete repeated
target grafting, automatic retained-material selection or unequal-radius corners.

### Material selection from original-chart interior witnesses

`face_cut::plan_projected_retention` builds a fresh projected trim network and
classifies each supplied original-chart UV seed against every descendant sector.
One sector must prove positive interior winding; all others must prove outside.
Boundary bands, exhausted work, overlapping memberships, missing membership and
non-finite inputs refuse. The complete classification shares at most 1,000,000
UV cells with per-query allocations from the remaining allowance.
Each individual winding query is capped at the classifier's 100,000-cell limit.
The network regression selects two distinct sectors out of three from strict
interior seeds and checks that the complete partition geometry stays unchanged.
Original-boundary, out-of-domain and NaN seeds and insufficient classification
work refuse atomically. The focused face-cut and contact-graft run passes ten
tests with these checks included.

The returned `retained_faces`, `discard_faces` and `seed_faces` are a material
plan. The partition model still contains all sectors and remains closed. Removal
must be composed atomically with authored transition insertion and a new complete
trim/closure check; the plan alone does not establish a replacement solid or its
radius, thickness, injectivity or freedom from self-intersections.

### Atomic replacement of connected selected sectors

`region_replacement::replace` replaces a connected set of selected sectors in
one closed shell by one authored nonperiodic transition surface. Replacement
pcurves refer to actual existing 3D rim edges. Their complete oriented edge set
must equal the exposed boundary of the selected region; repeated, missing or
reversed authorities refuse. Internal region edges and unused vertices are
removed, with explicit original-to-result index maps. Retained source surfaces
and rim curve definitions survive. Face merge and shell/body modification
lineage and the source-relative change set are recorded.

The actual saved model must pass full trim-region audit, continuous boundary
closure and persistent naming admission. Work stays within 100,000 shared cells,
1,000,000 exact work and 4,096 replacement control points. Collapsed boundaries,
disconnected regions, periodic replacement charts and inconclusive checks refuse.

The new regression replaces two sectors of a split cuboid top by a curved
biquadratic patch whose boundary is unchanged. It removes the internal cut,
preserves surviving edge and face identities, confirms the new center height
7.9 mm, checks merge lineage and roundtrips the complete model through the owned
value codec. Missing rim edges, wrong orientation, a mismatched surface boundary,
duplicate/disconnected selections and inadequate work refuse without changing
the input. Thirteen focused tests pass across region replacement, face replacement,
projected cuts and contact grafting.

The transition surface and its pcurves are still supplied by the caller. This
stage composes removal and insertion, but does not construct unequal-radius
corners, certify the new chart's interior or establish whole-solid embedding,
wall thickness, G2 continuity or the named-part STEP/UI acceptance gates.

### G2 transition construction composed with region replacement

`region_replacement::replace_g2` now constructs the transition from four original
retained support traces, then atomically replaces the selected region. The
open-hole and closed-region paths share `corner_close::prepare`; neither copies
an old patch's certificate. Actual original support edge uses, all four G2 seams
and whole candidate chart injectivity are freshly admitted before final region
boundary, trim and closed-shell checks. Support faces cannot belong to the
removed region. Construction and assembly share the same remaining geometry and
exact-work allowances rather than each receiving a fresh budget.

The extended-chart regression starts with a closed control, elevates its old
corner chart to degree six and changes its central control by 0.1 mm while keeping
the first three boundary layers. Replacement recomputes the corner exclusively
from retained original neighbors, restores the original center within 1e-12 mm,
passes all four original-trace G2 bounds and chart injectivity, and preserves
every retained face definition and identity. A removed face supplied as a support
and exhausted work refuse without mutating the input. Sixteen tests pass across
corner closure, region replacement, face replacement, face cutting and grafting.

This path currently needs four affine isoparametric owned rim-side chains with compatible
original support jets. It does not yet infer arbitrary curved UV correspondence,
prescribed unequal-radius laws or higher-valence
corner topology. Whole-solid thickness/self-intersection and STEP/UI acceptance
remain separate gates. The 0.1 mm control perturbation is a test input, not a
qualified 0.1 mm fillet radius or transition size.

### Piecewise root-bound sides of a G2 replacement

`replace_g2_sides` accepts up to 64 original edges on each of four sides, with
at most 256 total rim edges. Each side belongs to one original retained support
chart. Its affine pcurves must share a fixed original chart coordinate; sorted
along-intervals must meet exactly, and consecutive segments must share the actual
same source vertex. Duplicate edges, gaps, overlaps and different vertex owners
refuse. Input edge ordering does not choose connectivity.

The constructor proves G2 on the complete combined original support trace.
Actual per-segment pcurves are authored in the new patch chart; their rounded
normalization does not inherit that proof. Both the old and new saved uses are
freshly checked over every complete 3D edge before region assembly. Original
edge curves and root vertices are preserved, with no fitting or nearest welding.

The new regression splits all four closed-control rim edges using the shared-edge
partitioner, updating both adjacent face uses. It uses original edge fractions
0.37, 0.5, 0.61 and 0.5, supplies every side in reversed input order, then performs
the G2 replacement. Its new face has eight boundary coedges; every source edge
curve/identity and all four root vertex definitions/identities survive. Complete
boundary closure and original-trace G2/chart admission pass. Duplicate and missing
segments refuse atomically. Geometry, exact and coefficient budgets remain at
100,000, 1,000,000 and 2,000,000 respectively.
The focused corner/region/face/cut/graft regression run passes seventeen tests.

Each side still requires one original chart and an affine isoparametric trace.
Multiple different support charts on one side, curved UV correspondence and
prescribed unequal-radius corner construction remain incomplete.

### Qualification of authored candidates with curved UV boundaries

`replace_g2_candidate` now qualifies an authored surface with general saved
original pcurves before region replacement. Every rim edge must have exactly one
retained support use. Whole-edge common-curve positions and coherent normal and
curvature tensor bounds are fresh; unknowns refuse. The candidate also requires
a C2 interior basis and a complete injective-chart certificate. These checks,
final trim audit and closure share the geometry/exact budgets. Candidate and
support control counts are capped at 4,096.

For identical original charts and traces, the pcurve verifier now correlates
their shape fields exactly, after proving original regularity. Reversed trace
identity requires exact reflection of every actual knot; rounded knot reflection
cannot grant the shortcut. Actual 3D common-curve position proof is still mandatory.
Nonidentical charts use the general interval shape verifier.

The curved-boundary control is a lens window on the nonplanar quadratic graph
z=u²/4+v²/8. Its two quadratic UV arcs are lifted to degree-four 3D proposals and
freshly checked. The candidate passes G2 at normal tolerance 1e-5 and curvature
tolerance 1e-4, chart injectivity and closed region replacement. An additional
quartic deformation vanishes on both complete curved arcs, so position proof
passes, while its changed transverse normals produce a confirmed G2 violation.
The region operation refuses it. Off-boundary geometry and insufficient work also
refuse without source mutation.

Twenty-three focused tests pass: eighteen corner/region/face/cut/graft tests,
four original-pcurve G2 tests and one identity/regularity/position regression.
The identity test includes a valid reversed spline whose binary64 knot reflection
is not exact, mismatched controls, a broken 3D lift and a singular original chart.

This extends candidate admission to curved pcurves. The positive control uses
one common original chart; it is not general corner construction or proof that
all different-chart NURBS seams converge within budget. Automatic curved-boundary
jet construction, prescribed unequal-radius corners, whole-solid intersections,
thickness and STEP/UI/named-part qualification remain incomplete.

### Variable nominal-radius G2 construction and contact-ball law

`variable_g2_fillet` and `fillet_contact::build_variable_g2` now use the existing
variable offset-spine proposal pipeline with quintic endpoint-G2 sections and
fresh C2 skin preparation/contact admission. The constant-radius entry point
delegates to it, preserving the original constant G2 seed radius exactly.
Section samples and `VerifiedContacts` alone still grant no whole-skin G2 or
geometric radius certificate.

`fillet_radius::inspect` independently bounds the complete contact-center law
A(t)+side_a*N_a(t)*r(t) versus B(t)+side_b*N_b(t)*r(t). Original support normals,
fresh actual support/skin position proofs and the actual common rational curve
basis are required. Constant and linear laws use outward original-knot Greville
arithmetic and polynomial B-spline linear precision. The actual common weights
are preserved: an additional bound (wmax-wmin)*max(r)/wmin accounts for rational
linear-precision drift, multiplied by the maximum two-unit normal difference.
No nearly-unit weight is snapped. Position errors are included in the upper
bound; clamped endpoints provide certified lower violation witnesses.

The law is scoped to the shared normalized contact-curve parameter. It does not
certify true centerline arc-length parameterization or circular cross sections of
the quintic G2 skin. Spline laws, incompatible contact bases, unresolved original
normals and insufficient work remain unconfirmed. Modified skin geometry must
pass fresh checks even when stored contact reports remain positive.

On perpendicular plane supports, linear nominal laws 0.5→1.5, 1.5→0.5 and
0.1→0.2 mm pass complete contact lifts, normal tolerance 1e-5, curvature tolerance
1e-4 and maximum contact-ball center discrepancy ≤1e-6 mm. Construction contacts,
fresh G2 and the law proof share the test's remaining 100,000 geometry cells and
1,000,000 exact work. A requested constant 2 mm law produces a confirmed endpoint
violation. Zero work, spline-law qualification and a changed skin do not inherit
admission from old reports.

The new variable-body regression uses automatic contact root binding, two end
curves and retained-face grafting on a cuboid. It returns one closed body, proves
both full longitudinal G2 seams, preserves paired contact sizes at both ends,
confirms their size changes, and roundtrips the owned model codec. End boundaries
are closure-qualified; cap-to-skin G2 is not asserted. Thirty focused tests pass
across eleven native fillet tests and nineteen contact/binding/end/graft/region
tests, including the new closed-body control.

The contact-ball law above is proved on the original generated skin. General
post-trim law qualification, general NURBS support pairs, mixed-radius corner
assembly, whole-body self-intersections/thickness and independent STEP/UI
acceptance remain incomplete.

### Contact-ball law on actual trimmed B-rep edges

`fillet_radius::inspect_model` now reads actual saved skin/support coedges,
their orientation flags and owned 3D contact curves after grafting. It uses the
authored original law parameter domain, rather than assigning the full radius
range to the shortened curve again. Every saved original/skin lift is freshly
checked. Results include an explicit reason and a bound for the radius correction
caused by independently isolated root parameter intervals.

The trimmed control's original endpoint parameters differ by about 4e-13, and
its rational weights differ in their last bits. Shared-basis coefficient matching
therefore cannot grant its proof. The new independent affine-hull bound includes
each actual curve's complete deviation from its endpoint chord, actual rational
weight spread, original normal uncertainty and the original-law parameter drift.
Every interior control pole participates. Curved contacts with loose bounds
remain unresolved; endpoint agreement alone never grants a radius certificate.

The variable closed-body control passes full post-trim contact-ball agreement
within 1e-6 mm, with parameter correction below 1e-10 mm. Grafting, full G2 seam
diagnosis and the fresh radius check share 100,000 cells and 1,000,000 exact work.
After owned-codec geometry reload the same supplied authoring law passes again.
Incorrectly renormalizing that law to the shortened edge interval produces a
confirmed endpoint violation. The law/domain authoring metadata is supplied
separately; its CAD-document/Undo/UI persistence is not established by this test.

Seventeen focused tests pass: four graft tests, five contact tests, six root/end
tests and two affine-hull unit tests. The latter cover rational weight variation,
zero work and a middle contact-size defect whose endpoints remain correct. The
middle defect remains unconfirmed rather than being accepted from endpoint data.
Qualification across all curved NURBS families, mixed-radius corners, full-solid diagnostics,
independent STEP and WASM/worker/UI qualification remain open.

### Adaptive contact-ball bounds on curved rational contacts

`curve_secant_trace` encloses original rational contact positions and their
complete secant error from second jets. `surface_normal_trace` differentiates
original surface jets through order three and original pcurve jets through order
two. Differentiating the normalized cross product yields a complete bound for
the second normal derivative on each inspected interval. The secant remainder
is bounded by M/8 in the interval's unit parameter. Original regularity and
local C2/C1 hypotheses are mandatory; source knots outside the covered image do
not restrict the result. Whole range bounds handle pcurve derivative-jump bands
without granting a second-derivative certificate there.

The radius checker now adaptively combines these original position/normal
fields. Linear radius times linear normal is enclosed as a quadratic Bernstein
field. Complete secant remainders and actual support/skin position errors are
included. Matching curve bases additionally preserve the B−A cancellation in
a retained difference carrier, with outward control-rounding error added to
every result. Different bases use independent original curve bounds. Source
definitions and weights stay unchanged. Partial coverage never grants a global
upper bound; certified violations have a normalized-parameter/lower-bound witness.

The new control has two curved rational quarter-circle contacts on concentric
original cylindrical charts and an authored half-torus skin. Normals rotate
through 90 degrees along the contacts. The constant 1 mm contact-ball law passes
with maximum center discrepancy upper bound 5.874912353978459e-7 mm. Verification
uses 65,523 of 100,000 cells and 12,384 of 1,000,000 exact work. Constant 2 mm
produces a confirmed violation witness. A 200-cell partial run stays unresolved
and exposes no complete maximum-discrepancy bound. This rolling-ball control
does not assert G2 between its torus and cylinders.

The existing native G2 plane/rational-cylinder constructor control now also
passes the fresh contact-ball law check. Sixteen focused tests pass: four
original secant/normal tests, ten contact/graft tests and two affine-hull tests.
They cover reverse traversal, full interval coverage, local source knot handling,
missing work, a fold with regular endpoints, invalid chart membership and
derivative jumps. Numerical samples check the certificates; they do not grant
the bounds.

Automatic general NURBS corner construction, unequal-radius corner topology,
spline radius laws, full-solid diagnostics and STEP/WASM/worker/UI admission
remain incomplete. Curvature, sharp weight variation and unsupported source
continuity can still exhaust or refuse a complete radius certificate within the
fixed budgets.

### Automatic opposing-face candidates and clearance

`wall_opposition::inspect` now classifies original surface normal intervals in
one selected closed shell. Shell-use orientation participates in the dot product.
Only complete interval proofs exclude faces; uncertain, periodic and exhausted
queries remain candidates. No sample or world-axis plane test authorizes exclusion.
`wall_clearance::inspect_shell` reserves one quarter of the shared geometry budget
for discovery and passes all retained candidates into trimmed-face distance.

On the extended-chart control, cutoff -0.5 proves nine opposing floors and
excludes twenty faces, using 70 geometry cells. Clearance uses 6,503 further cells
and 33,028 UV cells. Its interval is
[2.9999999999999987, 3.0000000000000018] mm. Independent STEP measurement is
3.0 mm; the oracle also requires complete automatic selection metadata and the
same independently identified nine-face scope. The updated results are saved
beside the STEP control in `wall-clearance.json` and `wall-clearance-occt.json`.

Seven focused tests pass: two corner closures, three selected-clearance tests
and two opposition tests. They cover rotated candidate discovery, a 0.05 mm
thin closed control, zero-work preservation of unknowns, threshold ambiguity,
source immutability and shared work limits. Rotation qualifies discovery only.

This is an angular candidate filter followed by selected-face clearance. It does
not prove material pairing, cross-shell cavity pairing, normal-ray thickness or
whole-solid wall thickness. An unresolved angular candidate cannot independently
authorize a thin-wall classification. Automatic arbitrary B-rep trimming and
prescribed unequal-radius corner construction remain unqualified.

This is a corner-surface constructor and qualified hole closure, not a completed
arbitrary fillet feature.
Remaining work includes general curved UV boundaries beyond isoparametric traces,
unequal-radius fillet-generated boundary cycles, triangular and higher-valence
corner decomposition, automatic retained-region planning, prescribed-radius bounds,
general interior proofs beyond the sufficient projection certificate, thickness,
global self-intersections and named-part STEP acceptance.
The full roadmap also retains WASM/worker/UI and the named-part/P0/P2/P3 gates.

### Variable curved-support refusal evidence

Before quintic station interpolation, the plane/rational-cylinder proposal with
a linear 1 to 1.5 mm law and five sections failed original-cylinder contact
qualification. At normalized parameter
0.125 the certified distance is [0.0009718692879724248,
0.0009718692880034149] mm, exceeding the unchanged 1e-6 mm tolerance.
This is a confirmed construction mismatch, not merely proof-budget exhaustion.

`fillet_contact::Report::contact_attempts` now retains each visited candidate
edge, its two pcurves and the fresh positional evidence, including refusals.
Unvisited sides remain absent; the admitted `contacts` field still requires both
whole-interval lifts. The retained cubic arc-path regression reconstructs the
witness on the original support and checks aggregate shared work.

General variable-radius curved-support construction, automatic arbitrary B-rep
trimming and unequal-radius corner construction remain incomplete. No tolerance
or proof-work cap was increased. These native diagnostics have not yet been
transported through WASM/worker/UI.

### Quintic station proposals on curved supports

Variable G2 section proposals now use piecewise quintic Hermite station fields
on curved supports. Matched original-chart UV proposals use the same station
values and inferred derivatives. Five-point root-corrected finite differences
only propose first/second derivatives; they grant no qualification. Work is
bounded to at most six corrections per station, with the existing 32-iteration
Newton cap and existing 11-section cap. Original affine polynomial plane pairs
retain their simpler natural cubic representation. Constant-radius proposals
also retain the previous path.

The unchanged plane/cylinder control (five sections, linear 1 to 1.5 mm law)
now passes both complete original-contact lifts using 56 geometry cells.
Fresh original-normal analysis confirms contact-ball center discrepancy below
1e-6 mm within a shared 100k geometry/1M exact-work budget including contacts.
C2 storage preparation changes the skin by at most 3.8061e-14 mm in this control;
the independent 2M product-work cap remains unchanged.

With independent shape boxes, whole-seam G2 passed on the plane side but the
cylinder side remained unresolved:
100k cells give normal upper 0.000196993 and tensor upper 0.00599834,
above requested 1e-5/1e-4 bounds. This result does not qualify full G2, skin
cross-section curvature radius, a trimmed closed solid, ends or mixed corners.
Twelve native interpolation/fillet tests and twelve contact/graft tests pass.

### Original-chart mean-value G2 qualification

The G2 inspector now also bounds the difference between the original shape
fields using its center value and complete first-derivative intervals. The
derivative uses original third surface jets, original pcurve rates, and outward
interval differentiation of the normalized normal and world-coordinate shape
tensor. This retains a common seam parameter instead of subtracting two
independent whole-cell shape boxes. Each covered original chart must have a C2
basis and proved regularity; incomplete coverage or exhausted work grants no
additional upper bound. Already-qualified cells are not repeatedly refined.
Both comparison orientations still use one coherent normal/tensor sign.

The same five-section plane/cylinder 1 to 1.5 mm control now qualifies G2 on
both whole seams. The cylinder seam uses 57,579 geometry cells; its normal
upper bound is 7.741995724064515e-6 and tensor upper bound is
3.607759096077984e-5, below unchanged 1e-5/1e-4 tolerances. The plane seam
uses 38 cells. A regression confirms contact, both G2 seams and the contact-ball
law together within the shared 100k geometry/1M exact-work budgets.

Twenty-two B-rep tests pass across contact binding, trimming, grafting,
construction and region replacement. They include rejection of a curved UV
patch deformation which keeps positional contact but changes its normal.
Two native tests pass for original-trace identity/regularity and independent
analytic normal/tensor derivative formulas (quadratic/cubic graphs, reverse
rates and a singular chart).

This qualifies the declared variable curved-support control, not arbitrary
NURBS fillet solids or mixed-radius corners. Ends, general automatic trimming,
thickness, global self-intersections and WASM/worker/UI remain separate gates.

### Curved source-body contacts and ends

A second control now constructs the variable 1 to 1.5 mm transition on a
source prism whose actual support pair is a floor and a rational quarter
cylinder of radius 3 mm, axis at z=3 mm. Material occupies y=3..7 mm; the
immutable original support charts extend to y=0..10 mm, with the source trim
loops retaining the original material interval. This differs from the earlier
isolated-support control with cylinder axis at z=4 mm.

Both contacts pass fresh lifts, are automatically root-bound to the source
floor/cylinder end edges, and retain both exact shared endpoint identities.
Both transverse end curves and the four-edge transition patch pass the existing
original-skin/end-plane and trim gates. Source support surfaces remain unchanged
through binding. Transverse roots use explicit larger planar cutter faces;
membership of root bands directly on the source cap boundary is still unproven.

The original cap trim audit previously refused its tangential floor/arc join.
The simplicity proof now admits a positive-weight Bezier arc whose poles all
lie in a closed half-plane, with at least one pole strictly inside: every basis
function is positive in its open interval, so its open image cannot return to
the separator. This additional argument is restricted to Bezier bases.
Negative tests retain refusal for an arc crossing the line and a B-spline with
a flat return segment.

The full solid graft remains refused by `contact_graft::inside` with
`Retained-face closure membership is unresolved`. Its rectangle-classification
path cannot yet prove the new cut's complete inclusion at a curved original
boundary endpoint. No membership tolerance was enlarged and no gate bypassed.
`curved_contact_body_diagnostics` reproduces this remaining refusal.

Thirty-one tests pass: six simplicity, three trim-region audits, four contact
end tests, four contact graft tests, seven face crossings and seven face cuts.
This newly qualifies bound curved-support ends and fixes tangential trim
admission; it does not qualify the curved solid graft, full G2 of this second
control, wall thickness or global self-intersections.

### Boundary census for closed-face inclusion

`contact_graft` now has a second sufficient inclusion proof after rectangle
classification cannot conclude. It first requires the existing fresh original
trim-region audit. Every complete original boundary is then checked against the
actual saved cut pcurve: positive-weight control hull separation, proved exact
endpoint-only separation, or an exact planar root census. Only exactly shared
clamped endpoint roots are permitted. With every boundary excluded from the
open curve, one outward-enclosed interior witness identifies its connected
complement component. The whole curve's natural-chart containment is also
mandatory. Holes participate in the census; a witness alone never admits a cut.

Quadratic/cubic original carriers now accept rational Bezier queries through
degree 8. This covers the actual degree-5 end curve without degree reduction or
a fitted proxy. Exact root isolation has a sufficient Bernstein sign gate:
integer-scaled coefficients of one sign exclude all open-interval roots;
endpoint multiplicities are evaluated exactly. Mixed signs retain the original
root isolation path. A constant nonzero homogeneous denominator skips an
unnecessary position-coordinate gcd. All exact work shares the existing 1M
limit; geometry cells retain the 100k cap.

The new inclusion regression admits an internal arc, refuses an arc crossing a
hole and refuses exhausted work. Existing source/corner/face-cut regressions
remain passing. The curved source-body control is still refused: the remaining
original curved-boundary pair at index 7 cannot finish its exact census within
1M work (last observed consumption 999,996). No solid graft is admitted and no
limit or geometric tolerance was increased. The next construction/proof work
must resolve that near-end curved pair.

Validation: 105 cad-predicates tests, 19 B-rep contact-end/graft/face-cut/region
replacement tests, six native trim-simplicity tests and one native B-rep
boundary-census test pass (131 total).

### Closed curved-support control with explicit bounded fitting

The curved source-body control now assembles successfully after explicit
boundary proposal fitting. The strict attachment API continues to require
fresh inclusion and closure evidence; it does not silently edit input geometry.

The original cap pair has a genuine extra UV crossing near the shared end.
Independent exact rational evaluation of its represented binary64 coefficients
gives opposite conic signs at query parameters `1-2^-16` and `1-2^-20`.
Bernstein bounds keep the complete source inverse strictly inside `(0,1)` on
that interval, so this is a finite-arc event. The exact coefficient snapshot
is retained in the `tiny_finite_cap_crossing` regression. A larger proof budget
would not make the original cut an acceptable inclusion.

`fit_boundaries_with_charts` proposes adjustments to the interior poles of the
two transverse boundaries toward the chosen retained region. Every endpoint
and original support surface remains unchanged. Fresh whole-original-curve
deviation and original source/skin lift checks bound each adjustment by
1.25e-8 mm, below the unchanged 1e-7 mm model tolerance. Only edges 0 and 2
change; both longitudinal contact definitions remain unchanged. The fitting
result is a proposal and must still pass strict attachment. Nonlinear lifts
that cannot qualify remain refused.

The original bound support pcurves now travel through
`fit_boundaries_with_charts` and `attach_with_charts`, rather than being
discarded and approximated again. Endpoint authority is reapplied and the
actual saved curves are verified against the actual original faces. A forged
interior chart pole is rejected. General rational quintic pcurves can also be
lifted into affine original charts with their degree, weights and knots
preserved; complete original agreement still grants admission.

The new direct quadratic-carrier separation predicate works in original
homogeneous Bernstein coordinates. Adjugate coordinates satisfy
`L1² - 4 L0 L2 = 0` on a proper quadratic. Exact coefficient construction and
outward interval subdivision exclude every open-query contact; exact shared
endpoints are checked separately. Every introduced subdivision point must be
proved nonzero. This avoids a power eliminant/inverse fit for the inclusion
certificate. A declined sufficient test remains unproven. The general paired
root census remains available, with computation phases exposed on refusal.

The accepted saved model has one closed shell, **9 faces, 21 edges and 14
vertices**. Its whole original trim regions and continuous boundary closure
pass. Fresh G2 checks of the actual saved longitudinal coedges confirm both
seams. On the cylinder seam the normal upper bound is
9.994194034341862e-6 and tensor upper bound is 4.331265053044821e-5,
below unchanged 1e-5/1e-4 tolerances. The complete contact-ball center-gap
upper bound is 9.7967713292006e-7 mm, below 1e-6 mm.

Fitting, strict attachment and these post-assembly checks together consume
46,843 geometry cells and 580,859 exact work, within the unchanged 100k/1M
admission budgets. The original fixture/fillet construction is a separate
bounded stage. Geometry and identities survive the native codec round trip,
followed by fresh closure inspection; this does not establish document
Undo/Redo or UI recovery.

Artifacts are in `output/qualification/curved-variable-graft-2026-10-16/`:
`model.json`, `curved-variable-graft.step`, `manifest.json` and `occt.json`.
Model SHA256 is
`d37a4feff4c69799a1a4473d12f6b735cfc622d69c3533bba16b780a8c2f2a41`;
STEP SHA256 is
`ebc7a38d6cbd48559c4b8e55f4921b2ef69c32f4c7f7b790bed5d1c55d93f7bb`.
Independent OpenCascade import confirms a valid solid with the same topology,
10 x 4 x 10 mm design bounds (maximum bounding error about 1e-7 mm), and
measured volume 339.5360687356012 mm³. Volume is checked only against positivity
and the original source-volume upper bound, not an exact blend-volume target.

The input linear nominal-radius law is 1 to 1.5 mm on its original reference
domain `[0,1]`. The retained body spans only part of that domain: its authored
endpoint ball radii are 1.1499113534286494 and 1.35000181116024 mm.
Independent STEP contact-vertex measurements infer each radius from both
`x_floor²/12` and `3*z_cylinder/(6-z_cylinder)`. Maximum discrepancies are
9.9141e-10 and 8.0702e-10 mm. These are endpoint measurements; the native
whole-interval proof concerns the contact-ball law, not constant circular
curvature of the quintic G2 skin.

This qualifies the declared plane/rational-cylinder control and its bounded
transverse boundary fitting. **The end seams remain C0**, so smooth end runouts
are still pending. General NURBS fillet bodies, prescribed mixed-radius complex
corners, whole-wall thickness, global self-intersections, named-part STEP/history
acceptance and WASM/worker/UI integration remain separate requirements. The full
P0/P2/P3 roadmap remains active.

### Saved end-seam defect audit

`curved_contact_body_diagnostics --fit --ends` now inspects both actual saved
end edges (17 and 19), using their incident faces and owned pcurves. This is a
separate bounded diagnostic pass: 36 cells and 1041 exact work. Position
agreement is proven within 1e-7 mm on both seams. Both normal orientations
are rejected: minimum branch normal lower bounds are respectively
1.3951244968997096 and 1.3986406210602798, against the unchanged 1e-5 tolerance.
The decision is `CorrespondenceExceedsTolerance`, not budget exhaustion.

The curved-body regression now checks this explicit defect in addition to
the passing longitudinal seams. A smooth runout must replace the direct
skin/planar-cap join with additional geometry and freshly trim the cap;
closure alone cannot promote these seams to G1 or G2. No runout construction
or general mixed-radius corner support is claimed by this diagnostic.

### Interior-trace G2 runout construction

`nurbs_core::surface_blend::g2_traces` now constructs a quintic bridge from
interior isoparametric traces on two immutable original supports. Trace
coordinates, retained sides, along intervals, reversal and transverse gains
are explicit. Original source surfaces are never cropped for qualification.
Every saved support seam uses the actual patch isocurve, its actual pcurve
and a mapped original-chart pcurve; fresh position and geometric proofs share
the unchanged 100000-cell and 1000000-exact-work limits. Internal patch seams
are checked as well. Exhaustion never grants continuity.

The 0.1 mm control replaces a right-angle join between a floor and a cap with
traces at floor y=0.9 mm and cap z=0.1 mm. Both whole seams pass G2 at position
1e-7 mm, normal 1e-5 and curvature-tensor 1e-4. A separate whole-chart linear
projection certificate proves this candidate regular and injective. The
0.1 mm value is a setback, not an independently prescribed fillet radius.
A second control qualifies rational original supports with nonunit parameter
domains, partial traces and reversed correspondence. All 8 surface-blend
integration tests pass, including the existing full-boundary constructors.

This is the runout surface constructor, not a completed B-rep runout. Trimming
the original cap, assembling the additional faces, general nonisoparametric
traces, corner radius constraints and fresh body-level closure/embedding
qualification remain required.

### Closed 0.1 mm interior runout control

The interior-trace constructor now feeds the existing root binding, end-curve,
retained-face trimming and closed-solid graft pipeline in
`examples/support/interior_runout_fixture.rs`. The saved control has one body,
7 faces, 15 edges and 10 vertices. The two retained-support seams freshly pass
G2, original source surfaces are unchanged, and native codec round-trip followed
by fresh closure succeeds. Setup stages each retain their existing bounded
budgets; no end-to-end shared-work total is claimed for this fixture.

This exposed and fixed an orientation assumption in `contact_end::patch`:
actual simple clockwise UV loops are reversed and freshly re-audited before
model validation. Edge/vertex ownership is unchanged. A bounded affine-pcurve
proposal also handles small domain-edge roundoff; the ORIGINAL spatial curve
must pass a fresh whole-interval lift. A 0.01 mm out-of-domain counterexample
is refused, whereas a 1e-14 mm control succeeds. No tolerance is enlarged.

`interior_runout_acceptance` exports `runout.step` and `model.json` under
`output/qualification/interior-runout-2026-10-16`. Independent OpenCascade
import validates the body and topology. Measured setbacks on both end planes
are 0.10000000009313226 mm (maximum target error 9.313225191043273e-11 mm).
Measured volume is 599.9838571413579 mm³, checked only for positivity and against
the 600 mm³ source volume. `scripts/verify-interior-runout-occt.py` preserves
the STEP hash and measurements in `occt.json`. The 22 focused tests pass.

This control qualifies a closed transition between two planar supports with
0.1 mm setbacks; it does not yet replace the sharp ends of the curved-cylinder
control, prescribe a circular radius, smooth its two lateral seams, or solve
general NURBS/mixed-radius corners. Whole-body self-intersection, wall thickness,
WASM/UI and the full roadmap remain open.

### Curved-skin to original end-plane runout surface

`surface_blend::g2_to_plane_trace` constructs an interior source-isocurve to
arbitrary rational planar-pcurve bridge. A ruled chart proposes homogeneous
jets; it supplies no qualification authority. All final seams are inspected
directly against the immutable source skin and original end plane, including
actual saved boundary curves/pcurves and complete curvature tensors. Proposal
construction is separate from qualification so a proxy proof cannot exhaust
the budget before the original-support checks. All limits remain unchanged.

`curved_runout_diagnostics` now applies this to the actual variable-radius
plane/cylinder skin. Source station at 0.30982722974909827 has midpoint y=6.9 mm;
the rational target trace lies on the original end plane y=7 mm and is shifted
0.1 mm in X and Z. Both saved seams qualify at position 1e-7 mm, normal 1e-5,
curvature-tensor 1e-4. On the curved skin the normal upper bound is
9.998916143969928e-6 and tensor upper is 7.847708718006896e-5; on the original
plane they are 2.8281776479292273e-12 and 3.9352765241256116e-9. Whole-chart
injectivity/regularity also qualifies under projection
`[[-14,-2,16],[16,14,11]]`. Shared runout qualification consumes 90662 cells and
93095 exact work, including the 62-cell interior certificate; fixture setup
is a separate bounded stage.

Saved geometry and inputs are under
`output/qualification/curved-runout-2026-10-16`: `surfaces.json`, `supports.json`,
`target-pcurve.json`, and `manifest.json`. The 9 surface-blend and 10 B-rep
contact/end/runout tests pass. The curved-target regression also rejects an
altered original support and leaves zero-budget attempts unqualified.

This is a qualified runout SURFACE, not a closed body. The shifted target ends
require lateral corner surfaces; they cannot be silently welded to the old
floor/cylinder rim. Their construction, original-face trimming, shared-edge
binding and fresh closed-body qualification remain next. The 0.1 mm value
specifies geometric shifts here, not a prescribed rolling-ball radius. General
NURBS/mixed-radius corner support and all other roadmap requirements stay open.

### Runout bridge, worker protocol and staged WASM

`brep_g2_runout_surface` exposes original-support construction through the
geometry bridge. Its report explicitly leaves solid admission, radius laws
and interior embedding uncertified. Client `createG2RunoutSurface` and the
`g2RunoutSurface` main-solid worker job preserve unresolved diagnostics and
validate scope, limits, tolerance identity and bounded surface transport.
The existing disposable worker client supplies cancellation/retry and stale
response handling. One native dispatch test, 33 client/protocol tests and
the application type check pass. These transport tests use controlled ports;
they do not establish a browser/UI end-to-end scenario.

The geometry WASM was rebuilt and packaged under
`output/qualification/runout-wasm-2026-10-16`, without replacing the active
application package. Optimized size is 11522429 bytes, SHA256
`20da49002d3920a59a95ace8a7d56b1d4f08f1f0ddc09c259d55ed543964fe3f`.
Packed bytes decode to the exact raw module. `scripts/check-runout-wasm.mts`
exercises its actual binary ABI: the planar 0.1 mm setback case qualifies
(8 cells, 928 exact work), zero-budget remains unresolved and excess work
limits refuse. The saved curved skin/end-plane input also qualifies in WASM
(90600 cells, 93095 exact work), with the same original-support seam bounds
as native. Its observed runtime in this Node run is 17474.151833 ms; this is
one observation, not a performance qualification. Full receipts are in
`runout-abi.json` beside the staged package.

Lateral corner geometry and the closed curved runout body are still missing.
The stage has not been promoted to the running viewer; actual worker/browser
preview, selection, highlighting and UI cancellation acceptance remain open.

### Real worker-handler qualification on staged WASM

`tests/g2RunoutWasmWorker.test.ts` now uses the exact staged packed module and
identity in the real `createMainSolidWorkerHandler`, including normal kernel
warmup, native ABI dispatch and worker response serialization. The 0.1 mm
planar runout succeeds and its response passes the real main-solid protocol
validator. A zero transverse direction returns a native failure; a fresh
request succeeds afterward. Zero work returns an incomplete, unqualified
receipt. This is a real handler/WASM execution in the test realm, distinct
from the controlled-port cancellation tests. It is still not a browser worker
or user-interface acceptance scenario. The stage-dependent test explicitly
skips when its separately generated qualification artifacts are absent.

The next UI requirement is a command with explicit source/target roles and
preview/apply behavior. Until that exists and is exercised, the working viewer
package is preserved and the runout command is not claimed available to users.

### Promoted G2 end-transition command

The working viewer now includes **Концевой переход G2 / G2 end transition**.
Select two independent surfaces and a spatial end curve, then open the command
palette. The panel has explicit source/plane/curve roles, source edge and seam
position, physical tangent length, and direction controls. World-curve
projection, transverse proposals and conversion of millimetres to source jet
units occur in Rust. Original-curve projection consumes the same shared cell
ledger as final seam qualification. Tangent length is not a prescribed rolling
ball radius. Unproven results cannot produce an applied surface document.

Preview uses the existing worker lane and generation checks. A target-curve
failure highlights the actual curve in red, marks its role invalid and gives
localized corrective text. Incomplete seam reports offer Retry. The command
adds independent surfaces and preserves source identities; multi-patch IDs are
stable within a preview, and repetition preserves explicit input roles.

Qualification passed: 2 native bridge tests, 35 service/worker-client tests,
3 focused command UI tests, 61 adjacent curve/surface/worker runtime tests and
the application type check. Native-WASM UI cases cover English and Russian,
off-plane rejection/highlighting, role correction, confirmed preview, Enter,
Undo/Redo and saved-document restoration. Controlled-response UI coverage also
checks cancellation, Retry and stale responses. These counts concern the
specified suites, not the full P0 matrix.

The second package, under `output/qualification/runout-ui-wasm-2026-10-16`, was
verified before promotion. Raw and packed artifacts match; SHA256 is
`8454bd7ec6aff8a274ceb4ccab6b10a8cdffe6ba4874b57bcee70b7a8c103d4f`, size
11535501 bytes. `promotion.json` records the installed files and prior hashes;
`previous-package/` preserves the old package. The active application now uses
this package. `vitest.runout-stage.config.ts` supports pre-promotion checks.

A real in-app browser at `http://127.0.0.1:5195/` loaded the synthetic
`viewer-fixture.json` through project import. Shift selection, command palette,
the actual browser worker, native WASM and rendered preview produced the
whole-seam G2 status. Both Enter and mouse confirmation added a surface;
reload restored the saved transition. Browser testing exposed a floating
guidance overlay covering the Apply button: runout controls now scroll within
the panel, its actions stay visible, and desktop guidance avoids the panel.
The corrected preview is saved as `preview.jpg`. The automated download tool
timed out during a JSON export attempt; browser download/export acceptance is
not claimed by this milestone. Browser Undo/Redo timing snapshots were not
used as proof; the automated native-WASM UI tests establish those paths here.

This completes a usable open-surface command, not general closed B-rep fillets.
Lateral corner surfaces, arbitrary NURBS/mixed-radius closed corners, full wall
and self-intersection checks, named-part acceptance, and the remaining roadmap
are still required.

### Atomic assembly of prepared corner-sector networks

`region_replacement::replace_g2_network` now replaces up to 16 prepared,
disjoint sets of source-owned sectors as one transaction. It composes original
face/edge/vertex mappings across private intermediate models, retains common
rim identities, verifies each candidate chart's injectivity and C2 basis,
and checks unique seams on the FINAL saved geometry. No intermediate result
is returned on failure. A shared ledger covers all region trim/closure stages
and final seam checks: at most 100000 cells and 1000000 exact work. Global
embedding and automatic boundary/corner construction remain separate claims.

`geometric::inspect_edges_with_exact` accepts the caller's remaining exact
work. Actual saved pcurves now provide one continuous geometry proof for
affine and non-affine uses alike. A one-second process profile identified the
older affine-boundary inspector as the slow stage in the network control;
that specific diagnostic run was terminated after profiling and replaced by
the updated implementation. The complete positive/negative network test then
passed in seconds. No performance ratio is claimed from the interrupted run.

The prepared curved control partitions the existing quintic G2 chart into two
triangular sectors using a degree-10 owned 3D diagonal. Tensor lifting now
supports degree sums through 16, matching the unchanged original-source
verifier range (cross products remain degree <=32, below its exact binomial
range <=48). Existing compact iso proposals retain priority over newly
supported high-degree candidates, preventing a regression in the earlier
plane/cylinder graft. Every selected lift is freshly verified after owned
endpoint edits; work budgets and tolerances are unchanged.

The network test checks shared-edge identity, preserved source vertex/edge
identities, native codec round-trip and fresh closure/G2 after reload. It also
rejects a shifted patch, overlapping sectors, exhausted work and a patch
perturbed by `0.1*u*(1-u)*v*(1-v)*(u-v)`: this perturbation preserves the
position of every outer edge and the shared diagonal but changes transverse
jets. Its refusal occurs at the final saved-seam gate, with source unchanged.

`region_network_acceptance` exported the verified control under
`output/qualification/corner-network-2026-10-16`. Native admission consumes
26086 cells and 182411 exact work, with five unique inspected rim/seam edges.
Independent `verify-corner-network-occt.py` accepts the saved STEP: one solid,
31 faces, 61 edges and 32 vertices, bounds within 1e-6 mm of
`[-1,-1,-3]..[2,2,1.5]`. Measured volume is 30.37500000000001 mm³ against the
independent analytic integral 30.375 mm³. STEP SHA256 is
`ef06353a88d0947c475e976583b1f4ed41b70409d16abe5f74a7ea4c158e4bd0`.

This qualifies assembly of supplied surfaces on prepared owned cuts. It does
not construct prescribed unequal-radius corner geometry, infer arbitrary
new cuts, or prove whole-body thickness/self-intersections. The native network
API and the new lift/seam changes are not yet rebuilt into the active viewer
package; the previously qualified open-runout UI package remains installed.

## Atomic material planning across support faces

`face_cut::plan_retention_network` now projects and classifies cuts on several
original support faces in one transaction. Requests use original face and vertex
indices. The existing whole-curve agreement and trim winding checks run afresh
for every face; cells, exact work, and winding cells are shared across requests
(ceilings 100000 / 1000000 / 100000). Intermediate models are private. The final
change set is refreshed against the original input. All sectors remain present;
this plan alone does not remove material or authorize a replacement solid.

Verification: `retention_network` (1 test) and `face_cut` (7 tests) passed on
2026-10-16. The new test cuts adjacent cuboid support faces, checks original
surface and edge/vertex identity preservation, and refuses a late outside seed,
a duplicate support face, insufficient shared cut work, and insufficient winding
work without changing the input. Existing tests cover a nonplanar original
surface, rational curves, and holes. This does not establish arbitrary B-rep
coverage, automatically generate contact curves, or construct unequal-radius
corners. This native API has not been promoted into the active viewer WASM.

## Cutter-root ownership connected to retained face sectors

`face_cut::plan_rooted_retention` connects fresh trimmed-face intersection
coverage to shared-edge subdivision and support-face cut/retention planning.
Each distinct original edge is subdivided once; selected retained root ordinals
resolve to the resulting shared vertices. No caller-supplied root report or
nearest-point weld grants admission. Root subdivision and face-cut stages share
one 100000-cell ceiling; exact and winding ceilings remain 1000000 and 100000.
Intermediate models are private and the final change set refers to the original
input. A missing root or later trim failure returns no candidate.

The four native suites `edge_partition`, `face_cut`, `retention_network`, and
`rooted_retention` passed: 17 tests. The new chain test intersects two original
cuboid perimeter edges with an actual finite cutter face, builds the transverse
face cut, checks a single shared vertex at each end, two uses of every resulting
edge, closed-boundary position agreement, retained/discarded sectors, persistent
lineage, shared-budget refusal and missing-root rollback. Both source and cutter
remain immutable. This is not general contact-curve generation: the interior
3D cut is still supplied, and a single cutter face defines endpoint roots.
Unequal-radius corner surfaces, global embedding and viewer promotion remain
unqualified by this test.

## Generated cuts for original rectangular support charts

`face_cut::generate_intersection_curve` now constructs a transverse cut without
an authored spatial curve for literal rectangular support charts. Native
`ss_intersection::rectangular_trace_proposal` proposes rounded chart inverses;
these carry no coverage or topology authority. The saved spatial curve is lifted
on the first original support and independently verified against the second
original support along the entire interval, each with half the positional
allowance. Owned endpoint edits precede verification. The established strict SS
coverage query and its refusal of nonbinary exact parameter maps are unchanged.
No larger budget or relaxed tolerance is used.

The rooted retention test now uses the generated curve on the original cutter
whose chart inverse involves tenths. It also confirms that the strict SS query
still reports incomplete coverage for that pair. Generation work is subtracted
from the shared 100000-cell allowance before root binding and face cutting.
Wrong owned endpoints, insufficient work, a bent support and coincident surfaces
refuse. Final run: 9 face-cut/retention tests and 10 SS coverage/graph regressions
passed; the updated rooted test was rerun after its last assertions.

Scope: this generates a local position-qualified cut on rectangular charts,
not arbitrary curved SS branches, cutter-hole membership or automatic branch
ownership. The existing rooted trimming stage still independently verifies source
face cuts and endpoint roots. General NURBS contact generation, unequal-radius
corner construction, global qualification and viewer promotion remain open.

## Curved rational iso-section generation

`face_cut::generate_iso_intersection_curve` now isolates an original
parameter-line probe root against a second original NURBS support and constructs
the corresponding full source iso-section. It verifies the saved section on the
first support, then computes and continuously verifies its second-support
pcurve. Optional owned endpoint edits precede qualification. All root and
agreement cells share the caller's unchanged allowance (at most 100000).
The returned probe-root report proves probe coverage only; it does not imply
whole surface/surface coverage, trim membership, branch ownership or a solid.

The new rational curved control has nonconstant weights and a nonlinear section
in the plane x=0.1. Both natural parameter directions qualify after transposing
the original chart. A surface with the same probe crossing but an iso-section
that leaves the second support refuses, as do a wrong owned endpoint and a
missing root. The simple admitted case also qualifies within eight cells;
smaller-than-supported input allowances refuse. Input surfaces remain immutable.
The four face-cut/retention suites passed (10 tests), and the latest axis-direction
assertions passed in a focused rerun. No viewer WASM promotion was performed.
General non-isoparametric intersections and unequal-radius corner construction
remain open; this constructor does not turn a successful probe into SS authority.

## Non-isoparametric ruled-chart sections

`face_cut::generate_ruled_plane_cut` constructs a nonlinear polynomial UV trace
on a single original polynomial ruled Bezier chart with constant transverse
signed slope. The second support supplies a coordinate-plane proposal. Original
chart degrees 1..3 by 1 and constant weights are admitted for construction;
unsupported charts refuse. The proposed trace must stay inside the natural
source chart. Saved spatial coefficients and any owned endpoint edits undergo
full original-support lift verification and independently qualified second-
support pullback, at half the requested positional allowance per support.
The common root/agreement allowance remains at most 100000 cells. No geometry
sampling, tolerance widening or SS coverage authority is used in construction.

Verification: `ruled_intersection`, `iso_intersection`, `rooted_retention` passed
(3 tests). The new control is z=u²/4+v, sectioned by z=1/2. Its nonlinear trace
v=1/2-u²/4 and saved spatial curve pass both whole-interval verifiers at 1e-7 mm;
additional evaluations match the independent closed form within 1e-10 mm.
Incorrect owned endpoints, nonconstant transverse slope and insufficient input
budget refuse; source definitions remain immutable. These are constructor tests,
not a curved closed-body trim/STEP/UI admission. General rational ruled charts,
nonlinear transverse surfaces and unequal-radius corner surfaces remain open.

## Rational weights in non-isoparametric ruled sections

The ruled-section constructor now accepts distinct longitudinal NURBS weights,
provided the two weights of each transverse ruling are equal. With original
W=sum(B_i*w_i), the solved transverse coordinate is V/W. One degree elevation
stores both that rational coordinate and the unchanged linear longitudinal
parameter in a common homogeneous basis. Weight normalization is proposal-only;
the verifier still receives the immutable original support coefficients.
Whole original-source position proof and the second-support pullback proof
remain mandatory after endpoint authoring. Tolerance and work ceilings are
unchanged. Underflowed/nonpositive proposed weights refuse.

The three section/rooted-retention tests passed. The updated ruled control also
checks rational quadratic and cubic supports, nonunit original parameter domains,
and independent rational closed-form measurements at several parameters in
addition to the complete interval evidence. The cubic proposal has degree four
in UV and passes at 1e-7 mm. Unequal transverse ruling weights refuse explicitly.
The latest focused rerun after adding cubic and reparameterized controls passed.
These are curve-constructor controls; arbitrary rational surfaces, full SS
coverage, closed-body corner assembly and viewer runtime qualification remain
separate open work.

## Unequal transverse weights, varying slope and posed section planes

The previous equal-transverse-weight and constant-slope construction restrictions
are superseded for the rational ruled Bezier case. The constructor now forms
signed homogeneous boundary polynomials A and B directly from both original
ruling weights, proposes normalized v=A/(A-B), and elevates the common basis once
to store the unchanged longitudinal parameter. Candidate denominator controls
must have one strict finite sign and the resulting rational trace must stay
inside the original chart. These are proposal guards, not SS coverage evidence.
The unchanged full original-source lift and second-support pullback remain the
only position admission, after any owned endpoint edits.

A plane carrier can now be proposed from a posed second-support frame rather
than requiring a coordinate axis. No carrier fit proves second-support identity:
the final saved curve is independently verified on the actual original chart.
Tolerance remains 1e-7 mm in the controls and caller work at most 100000 cells.

Verification: five section/retention suites passed (11 tests). The ruled test now
admits unequal transverse weights and varying signed slope, compares the former
against independent homogeneous boundary sums, and repeats admission after a
0.37-radian rigid rotation plus translation. Rotated saved geometry agrees with
the transformed baseline within 1e-10 mm at the additional evaluation stations;
whole-interval reports qualify both original supports. Mixed-sign denominator
controls and incorrect endpoint edits refuse. Cubic and nonunit-domain controls
remain passing. This still covers degree 1..3 by 1 single Bezier charts, not
arbitrary tensor NURBS, full SS coverage, closed-body trim/STEP or corner radii.
No active viewer WASM was changed.

## Closed curved B-rep chain with freshly bound root ends

`face_cut::bind_intersection_ends` assigns a generated curve's endpoints from
actual saved shared vertices and its first pcurve endpoints from uniquely owned
outer-loop uses. It requires clamped representations and freshly verifies the
entire edited curve against both original supports at half the model positional
allowance each. Previous generated-curve reports do not authorize endpoint edits.
A mismatched shared vertex refuses without changing source or proposal.

The new analytic closed control is 0<=x,y<=1, -1<=z<=y+x²/4. The test generates
the nonlinear section z=1/2, isolates retained cutter roots on the original
perimeter edges, subdivides all adjacent uses, binds the actual root vertices,
and partitions the original curved top face. This exposed and fixed the prior
end-coordinate mismatch rather than relaxing exact endpoint ownership.
Every new end has exactly one shared vertex; its three incident edges each have
two face uses. Native boundary agreement, disjoint seed ownership, immutable
source/proposal and codec save/reload checks pass. Four section/rooted suites
passed after the final changes (4 tests).

`curved_trim_acceptance` exported the control using 130 assembly cells under the
unchanged shared 100000-cell ceiling. Independent OCCT import via
`scripts/verify-curved-trim-occt.py` confirms a valid solid with 7 faces, 15 edges,
10 vertices, bounds [[0,0,-1],[1,1,1.25]] within 1.0000000183474356e-7 mm, and
volume 19/12 = 1.5833333333333333 mm³ within the 1e-6 comparison tolerance.
The observed zero volume difference is not an exact arithmetic accuracy claim.
Artifacts: `output/qualification/curved-trim-2026-10-16/`.
STEP SHA256: 71bd9f2831dc0d39fbecdfad6a795d7bd7bb1b02bf3d9350782bdddcae4e6d52.
Model SHA256: 2dcb2c89d049eaadd609d7b938eae7d0a685898c8c9f4b79ce8f68cc6745ed19.

This control retains BOTH face sectors and the original volume. It qualifies
shared-end nonlinear subdivision of a closed body, not material removal, fillet
construction, global intersections/thickness or CAD Undo/Redo. Mixed-radius
corner construction and active viewer WASM promotion remain open.

## Atomic automatic section partition with inferred boundary ends

`face_cut::partition_ruled_at_face` now composes the native construction stages
without caller-selected endpoint edges or vertices. It generates the section,
qualifies the entire second-support pcurve hull inside the actual cutter trim,
queries every original outer boundary edge, subdivides all incident uses,
requires exactly two retained roots, and proves a unique end association using
outward squared-distance bounds to the complete original root boxes. Ambiguous
or additional roots refuse. Fresh owned-end binding and full face partition
follow on private models. The final change set is refreshed against the original
caller model. All generation/query/split/binding/partition cells share 100000;
the final cut's exact allowance remains at most 1000000. The nested Partition
counters describe that stage, while the wrapper reports the total transaction.

Four section/rooted suites passed, and the final automatic codec reload assertions
passed in a focused rerun. The control independently verifies all four boundary
coverage reports, persistent naming, closed boundary agreement after save/reload,
late refusal at one cell below the required work, and refusal when the section
crosses a hole in an actual authored open cutter face. A zero exact-work allowance
also succeeds on this particular control, with recorded exact_work=0; it is not
artificially refused merely because the allowance is small. Sources remain
immutable on every checked path.

The acceptance example now uses this public transaction. New artifacts preserve
the earlier manually composed control at a separate path:
`output/qualification/automatic-curved-trim-2026-10-16/`.
Automatic assembly uses 160 cells. Independent OCCT STEP admission again confirms
one valid solid, 7 faces/15 edges/10 vertices, analytic volume 19/12 mm³ and bounds
within 1.0000000183474356e-7 mm. Comparison tolerance is 1e-6 mm/mm³.
STEP SHA256: c097d68c39a859f63e6440285bbfea00e6aaa75f0a921fcc49c65d78fffb75bb.
Model SHA256: 0d8716f7b8d4aa8cb4c3998a60682e393e11ce72e1f8d0814417f38a67caefff.

This transaction still retains both sectors. Its admitted construction range is
single rational ruled Bezier charts of degree 1..3 by 1 with a qualifying planar
carrier proposal; it does not claim general tensor NURBS cuts, material removal,
full SS coverage, radius/G2/thickness/global intersection proof or UI admission.
The full corner, runtime, acceptance and P0/P2/P3 objective remains open.

## Clipped ruled intersections between actual boundary roots

The automatic transaction now isolates/subdivides all original outer-edge roots
first. Their saved source-chart endpoint coordinates select an increasing
longitudinal interval. `generate_ruled_plane_cut_between` restricts the original
homogeneous trace to that interval and assigns the actual owned UV and spatial
endpoints BEFORE full-source lift admission. This supersedes the previous
full-chart-only construction order. Each saved endpoint is also freshly bounded
against the complete original root-position enclosure; no nearest-point match
selects branch ownership. Exactly two retained roots are still required.

This admits side-face sections whose unconstrained trace leaves the original
chart. Both sides of the analytic closed control now qualify: x=1 from y=1/4 to
1, and x=0 from y=1/2 to 1. The full untrimmed constructor correctly refuses those
same supports, while the root-bounded clipped transaction passes whole original
agreement and closed-boundary checks. All work shares the original 100000 cells;
root ownership, cutter-hole membership and late-budget refusal regressions remain
passing. Four suites passed (5 tests).

The acceptance example can select a source face. The x=1 side control was exported
to `output/qualification/clipped-side-trim-2026-10-16/` using 159 assembly cells.
Independent OCCT confirms a valid solid, topology 1/7/15/10, analytic volume 19/12
mm³, and bounds within 1.0000000183474356e-7 mm (comparison allowance 1e-6).
STEP SHA256: f2e6177f10e768ee3637b3f3331e75dce58b69d6fcca19578d7d05ab10372db2.
Model SHA256: 0653e2f9479361291fe2259e08ff9c5f4d8a39a9636e78852fe5f9c3d6621ff9.
Both sectors and original volume remain; simultaneous neighboring-face cuts,
material removal/capping, mixed-radius corners and viewer admission remain open.

## Adjacent-face network with one shared subdivision per original edge

`face_cut::partition_ruled_network` queries the union of original outer edges
once, records the original root bands and shared saved vertices, and constructs
each face section from those prebound ends. It never rediscovers a crossing on a
rounded child edge. Each face requires exactly two owned roots. All intermediate
models stay private; the final change set refers to the original caller model.
The shared transaction ceilings remain 100000 cells and 1000000 exact work.
The existing one-face operation now uses the same prebound stage internally.

Four adjacent curved-control faces produce four shared root vertices and four
section edges. Each root vertex is an endpoint of two different face cuts and
has four incident edges. Every original boundary edge has one complete coverage
report. Native closure, naming and codec reload pass. A fifth requested face with
no section refuses after the earlier private cuts; duplicate faces and late
budget exhaustion also refuse without changing the original. The network's
independent STEP control is valid with topology 1/10/20/12, unchanged volume
19/12 mm³ and expected bounds. Artifacts:
`output/qualification/automatic-trim-network-2026-10-16/`.
Assembly work: 462 cells, 833114 exact units.
STEP SHA256: 56a7601c5cc0660203d6c0f5c38de6ae922350bca99c98d1bebdafd6ea7b2b5b.
Model SHA256: 0af8c132ce885f42aab8e9c4b6d610443ac394a7f421275f3bd8b2a4387526b6.

## Selected material removal with an owned closed cap

`region_replacement::cap_region` derives the selected region boundary from
oriented source incidence, orders it by vertex identity, and projects each
complete saved edge onto the authored cap chart. A chord-area calculation only
proposes loop orientation. Final saved-pcurve winding, trim validity and original
boundary/closure agreement are freshly admitted by `region_replacement::replace`.
The cap chart also receives a whole injectivity check. Disconnected or ambiguous
rims, shifted caps, misselected sectors and insufficient work refuse.

The analytic control's upper sectors are selected by explicitly supplied original
UV witnesses with unique interval winding ownership. The witnesses are a control
selection, NOT an automatic material-side certificate. Replacing these sectors
with z=1/2 now removes actual material and returns a closed B-rep. Five native
suites passed (7 tests), including rollback and shared-root cases. Native codec
reload precedes export. Total construction/selection/cap work: 657 cells,
999998 exact units; neither established ceiling was raised.

Independent OCCT (`scripts/verify-curved-cap-occt.py`) confirms a valid solid with
7 faces/15 edges/10 vertices. Bounds [[0,0,-1],[1,1,1/2]] differ by at most
1.0000000183474356e-7 mm. Analytic volume is
integral(1+min(1/2,y+x²/4)) dxdy = 677/480 mm³; observed volume is
1.4104166666666647 mm³. Removed volume is 83/480 mm³; observed value is
0.1729166666666686 mm³. Both comparisons pass 1e-6 mm³; observed ~2e-15 differences
are not general accuracy guarantees. Artifacts:
`output/qualification/curved-cap-2026-10-16/`.
STEP SHA256: 3936c4716e630d64f0dd2a2ecaa601c9b89413743150aa98c086bfe2a107f863.
Model SHA256: bce8c0dc75ef53719086710e674759506204ddfa93cccc4056b1ba7e1ded756a.

Scope remains rational ruled Bezier source charts, qualified cutter membership,
and explicitly selected material sectors. This is a C0 planar cap, not a radius
or G2 fillet. Arbitrary tensor NURBS, automatic material-side proof, mixed-radius
corners, global thickness/intersections and viewer/acceptance/P0/P2/P3 remain open.

## Original-chart collars along nonlinear rational pcurves

`nurbs_core::surface_collar::construct` now composes an original rational Bezier
support with P(u)+v*d using two-dimensional homogeneous Bernstein arithmetic.
It constructs the stored collar directly from original controls, without a fit
through sampled positions. Positive rational mapped-control hulls stay inside
the original natural chart. Outward coefficient intervals bound target-versus-
stored homogeneous residuals; a positive denominator lower bound converts them
to a whole-surface Cartesian position bound. Both output degrees are at most 16
and construction work stays at most 2000000. Positions do not certify derivatives,
trim ownership, radii, embedding or a solid.

The polynomial control uses S(u,v)=(u,v,u²/4+v), trace
P(u)=(u,1/2-u²/4), and d=(0,1/10). Whole stored-position error is at most
3.933087563440863e-14 mm, with 7444 coefficient work units. A fresh ORIGINAL-
support saved-edge G2 inspection qualifies the complete nonlinear seam using
6033 cells and 15865 exact units. Normal upper bound is
9.932229388823699e-6 (limit 1e-5), curvature tensor bound
3.6676038265126845e-5 (limit 1e-4). A transverse second-jet defect that preserves
seam positions and tangents is independently refused.

The rational control varies source and pcurve weights. Its whole stored-position
bound is 8.740921770577179e-14 mm, also 7444 coefficient units. Original saved-edge
position agreement qualifies, including nonunit-chart regression checks.
However ORIGINAL-support G2 remains Unresolved after the unchanged 100000-cell
allowance (555249 position exact units); no normal/tensor upper bounds qualify.
The test explicitly records this refusal, not a successful rational G2 admission.
The failed positive G2 attempt was retained as evidence and the diagnostic input
and full stored surface are saved for subsequent proof work.

Three suites passed: corner_blend 8, surface_blend 9, surface_collar 2 (19 tests).
The final focused rerun also passed strict position-error budget and nonunit-
parameter assertions. Artifacts from `surface_collar_diagnostics`:
`output/qualification/surface-collar-2026-10-16/{polynomial,rational}.json`.
Construction and proof ceilings/tolerances were not raised. This is a native
prerequisite for arbitrary-pcurve corner boundary fields; it is not yet connected
to the corner constructor, B-rep assembly or active viewer. Rational G2 proof,
unequal-radius corner geometry and the full thread objective remain unfinished.

## G2 query reuse and proof preservation at work exhaustion

The generic original-pcurve G2 inspector now reuses a finite mean-value query
as both the whole-cell upper enclosure and a midpoint lower witness. Previously
it additionally queried an independent whole rectangle pair and midpoint on
short cells. The new lower witnesses are derived from the same original interval
shape fields already computed at the midpoint; they are not sampled floating
values. Missing mean-value evidence still falls back to the rectangle operator.
Literal original-trace identity keeps its existing direct route. The source
geometry, tolerance and 100000-cell / 1000000-exact ceilings are unchanged.

Midpoint counterexamples are now retained across refinement as witnesses on the
original whole seam. If the final query exhausts work without a child enclosure,
the valid parent envelope is restored rather than replaced by an uninspected
child. This preserves prior regularity/error evidence on Unresolved reports and
does not authorize incomplete G2. An independent graph formula checks the new
midpoint lower/whole-cell upper bounds for normal and world curvature errors.

On the unchanged polynomial collar control, qualification uses 3569 cells rather
than 6033. This is a deterministic proof-query comparison, not a wall-clock speed
claim. Rational G2 still returns Unresolved at 100000 cells, but now retains
finite worst-error bounds: normal upper 5.3208161207935655e-5 versus limit 1e-5,
and tensor upper 0.03923647707181649 versus limit 1e-4. These are conservative
upper bounds, NOT measured physical defects. Saved position evidence remains
valid. Artifacts: `output/qualification/surface-collar-retained-bounds-2026-10-16/`.

Constant tangent-basis preconditioning, dual-basis intersection and reciprocal-
tangent formulations were investigated on the same immutable control. None
qualified the rational case within the established work; those mathematical
changes were removed. Only original-formula query reuse, persistent witnesses
and parent-envelope preservation remain. Intermediate diagnostic artifacts are
retained separately; none is promoted as rational G2 success.

Validation: 32 NURBS integration tests (corner_blend, surface_blend, surface_collar,
surface_g2, surface_shape_operator), 2 mean-value unit tests with independent
analytic formulas, and 6 B-rep contact-graft/region-network regressions passed
(40 tests). This includes corrupted curvature and budget/ownership refusals.
Rational qualification still needs a correlated original-coefficient jet proof;
unequal-radius corner geometry and all broader thread requirements remain open.
No active viewer WASM was rebuilt or promoted by this change.

## Fresh correlated original-coefficient G2 proof

`surface_collar::inspect_g2` now recomposes the original rational surface along
its original rational pcurve and transverse direction, then compares those
homogeneous coefficients with the actual candidate. It accepts no historical
receipt or rounded reference surface as authority. Preparation is private and
reused only inside one immutable inspection call.

Coupled interval arithmetic retains correlations through products, quotients,
Cartesian jets of total order two, normal normalization and reciprocal-tangent
world curvature tensors. A positive constant homogeneous proof gauge leaves
stored candidate weights unchanged. Endpoint Bernstein restriction selects exact
endpoint coefficients only after differentiation; unused transverse columns are
not evaluated. One coherent orientation is required across the entire seam.

The unchanged rational control now returns `WithinTolerance`: 70 cells and
365081 coefficient-work units, with whole-seam upper bounds of
2.8195497254938276e-14 mm position, 1.0167688724078058e-9 normal difference and
2.4744894853039754e-5 world curvature tensor difference. Requested limits remain
1e-7, 1e-5 and 1e-4. Construction uses 7444 additional work units (372525 total).
The limits remain 100000 cells and 2000000 coefficient-work units. Typed work
exhaustion returns `Unresolved` with no global upper certificate. A proved point
counterexample may return `CorrespondenceExceedsTolerance` with a witness.

Artifacts: `output/qualification/surface-collar-correlated-admitted-2026-10-16/`.
`comparison.json` verifies bitwise equality of the source, pcurve, direction and
candidate collar against the retained-bounds baseline. The generic G2 inspector
still returns `Unresolved` for this control. The specialized inspector requires
the composed, clamped Bezier basis and does not claim arbitrary basis support.

Validation includes original-coefficient mutations, damaged second jets, reversed
orientation, positive weight gauge, nonunit source charts, strict cell/work
budgets and independent rational-extrusion shape-operator formulas. Three coupled
arithmetic unit tests and 34 integration tests passed before the final endpoint
optimization; affected focused tests were rerun afterward. An independently
rounded rotation plus translation remains `Unresolved` at the fixed budget and
is explicitly retained as a refusal regression, not a successful qualification.

This certifies a control seam, not a radius or an embedded, closed fillet solid.
General B-rep trim ownership, unequal-radius corner construction, end transitions
and full-body diagnostics still need integration and qualification. These native
changes have not been promoted into the active viewer WASM. The full roadmap
remains open.

### Follow-up: pose-independent proof coordinates

The independently rounded rotation/translation control above is now admitted.
`prepare` chooses the first original source control point as a common proof
origin. Original composition subtracts that origin with outward-rounded interval
arithmetic before homogeneous multiplication; actual candidate coefficients use
exactly the same origin and outward subtraction. Stored controls, weights,
pcurves and construction output remain untouched. A common translation preserves
Cartesian position differences and all first/second derivatives, so the normal
and world curvature tensor proof is unchanged mathematically. Every extra
subtraction is charged to the same coefficient-work limit. Zero origins retain
the previous arithmetic path and construction artifact.

Regression coverage requires the independently rounded 0.37-radian rotation
plus translation [2,-3,1] to return WithinTolerance with Same orientation. An
additional 0.001-mm displacement of only that candidate must return
CorrespondenceExceedsTolerance with a witness. This supersedes the pose refusal
recorded in the preceding section; general corner/solid qualification remains
open.

Follow-up verification completed: all four surface-collar integration tests and
all three coupled arithmetic unit tests passed after proof-coordinate centering.
The posed candidate displacement counterexample also passed. No tolerance or
budget was increased.

## Corner construction from mapped original pcurves

`corner_blend::mapped::g2` connects four original single-Bezier surfaces and
rational pcurves to corner construction through freshly built collars. Each
stored collar is inspected against its immutable original source. The actual
corner is inspected against the stored collars and for interior injectivity.
No caller-supplied qualification report is accepted.

Normal and signed world-tensor errors compose using the coherent orientation
branches and outward-rounded triangle sums. Original-to-collar and
collar-to-corner seams each receive half the requested limits; admission checks
the full outward sum. Construction/collar proof/corner proof share the cell and
coefficient-work ledger. Corner proof now accepts the remaining coefficient
budget internally, retaining the original public API defaults and 2M ceiling.
Unfinished or exhausted proof gives no final original-support upper certificate.

Two native controls cover noniso linear UV traces and genuinely quadratic UV
traces on sheared polynomial charts. Both yield an independently checked planar
rectangle in space: this is not evidence for general spatial unequal-radius
corners. The quadratic control uses 9 cells, 170272 coefficient work, zero exact
predicate work; maximum original-support upper bounds are below 8.58e-14 mm
position, 2.99e-12 normal and 2.16e-12 tensor. Independent grid positions and the
original quadratic pcurve equation are checked. Inputs remain unchanged. A
corrupted pcurve cannot authorize an admitted corner. Eight existing corner
regressions also pass, including incompatible corners, curvature defects,
folded interiors, reversed rational charts and knot-map refusals.

These operations do not implement radius laws, multi-span arbitrary supports,
B-rep replacement, closed-body assembly or viewer promotion. Unequal-radius
spatial corners and end transitions remain required work.

## Spatial mapped-corner control and fresh candidate inspection

Added a spatial control on the immutable polynomial source
S(u,v)=(u+v,u-v,0.3u²-0.2uv+0.3v²). All four input pcurves are noniso in the
original chart. The independently specified output is
z=0.1x²+0.2y² over x=[0.75,1.25], y=[-0.25,0.25]. Twenty-five independently
computed grid positions agree within 1e-10 mm. Original-support seam proofs and
patch interior admission use 9 cells and 144828 coefficient-work units, below
the unchanged limits. Original source coefficients are retained unchanged.
This extends mapped-corner qualification beyond planar spatial images; it does
not establish a prescribed fillet radius law or arbitrary spatial support scope.

`corner_blend::mapped::inspect_candidate` now rechecks an actual supplied patch
against freshly constructed/qualified original-source collars. It accepts no
construction receipt or previous admitted patch. It uses the same shared work
ledger and composed orientation/triangle bounds as construction. A control-net
mutation in the second transverse row preserves boundary positions and first
transverse jets at U=0, but is required to refuse final admission because the
second jet changes. B-rep replacement, radius laws and closed-body checks remain
separate required integration work.

## Owned B-rep mapped-corner replacement

`region_replacement::replace_g2_mapped` now constructs from the actual saved
face-local pcurves of four retained source uses. Each distinct rim edge must
separate selected sectors from exactly one retained support occurrence, with
unique shell ownership. Candidate pcurves on the four new patch sides retain
source parameter domains and explicit reversal correspondence. Orientation is
checked consistently against original shell uses.

All original/candidate saved-curve lifts are freshly inspected before assembly.
The existing atomic region replacement then checks exact owned rims, trim
regions and tolerance-qualified shell closure. Original 3D edge geometry and
persistent edge/vertex IDs are preserved through source maps. Native G2,
curve-use, trim and closure checks share the cell/exact-work remainder; original
collar and corner proof share the coefficient-work ledger. Invalid/repeated
rims refuse before assembly; source is immutable.

The fixed spatial paraboloid-sector control passes with one body, 30 faces,
60 edges and 32 vertices: 321 cells, 65523 exact work, 145100 coefficient work.
Native serialization/reload is valid, names remain complete, and saved-use
closure is freshly rechecked. Five B-rep integration tests passed: the new
mapped control and four existing region-replacement regressions.

Artifacts are in `output/qualification/mapped-corner-brep-2026-10-16/`.
`mapped-corner.step` was independently read by OCCT using
`scripts/verify-mapped-corner-occt.py`. It contains one valid solid; bounds agree
with [-1,-1,-3]..[2,2,1.5] within 2e-7 mm and measured volume is
30.37500000000001 mm³ against the independent integral 243/8 mm³. STEP SHA256:
21932fc6a7087ee8f038613610dbd102bfbf4541af40db87a8c4e62c114fccbd.
`independent-occt.json` records the independent result; the native manifest does
not present native export as an independent measurement.

This control replaces an already prepared sector while preserving its intended
shape. Automatic arbitrary-support cut inference, prescribed unequal radii,
end transitions and general corner embedding/thickness remain unfinished.
No active WASM/UI package was promoted in this step.

## Whole-skin prescribed radius law

`skin_radius::inspect` now bounds the actual complete rational Bezier skin
against an original rational center curve C(t) and positive rational radius law
r(t), using the normalized selected station axis. It computes the bivariate
Bernstein residual of |S-C|²-r² from original homogeneous coefficients, including
all mixed products and positive denominator factors. Outward-rounded coefficient
hulls bound that residual over the entire two-dimensional patch. Division by a
positive lower radius bound yields a sufficient upper bound on |norm(S-C)-r|.
No station samples, analytic type tags or previous receipts authorize admission.

Source charts must be positive-weight, nonperiodic single-Bezier charts of degree
at most 8; radius is encoded [r,0,0]. The 2M coefficient budget is fixed and typed
exhaustion returns no radial upper certificate. A false within_tolerance flag
means qualification was not obtained, not a certified geometric counterexample.
Unsupported or invalid inputs refuse. This measures distance to the supplied
corresponding center, not nearest spine distance, curvature radius, envelope,
contact, regularity, embedding or solid validity.

Rational quarter-circle controls qualify constant 0.1-mm and linear 0.1→0.3-mm
radius laws over the full skin: 5920 coefficient-work units each; radial upper
bounds 2.8883145875014075e-15 and 2.456368441983179e-14 mm. Independent point
formulas, both station axes, independent nonunit center/radius domains, immutable
inputs, a strict one-less-work refusal, a wrong radius law and an actual damaged
skin (independently over 0.001 mm from the prescribed radius) are checked. The
radius test plus three mapped-corner regression tests passed.

This closes a native proof gap between contact-ball radius and the actual skin.
It has not yet been integrated into unequal-radius corner construction, B-rep
fillet admission or WASM/UI. Those requirements remain open.

## Actual multi-span fillet skin: radius defect and repair

`skin_radius::inspect_affine_field` extends full-skin radius qualification to
bounded, positive-weight, nonperiodic multi-span surfaces. Each nonempty
ORIGINAL knot cell is extracted into interval homogeneous Bernstein coefficients;
no rounded temporary surface is substituted. Affine world-coordinate fields
specify the corresponding center C(P) and radius r(P). The polynomial
|H-C(H)|²-r(H)² is bounded over every whole cell, divided by positive weight
factors and a proved positive radius. All original cells must qualify. Blossom
extraction receives a conservative precharged work debit; polynomial operations
share the fixed 2M coefficient ledger. Work exhaustion or unproved radius
positivity returns no global upper certificate. These fields do not by themselves
establish contact-ball geometry, nearest-point distance, G2 or embedding.

Inspection exposed a real defect in the working rolling-ball generator:
constant 1.25-mm sections lofted with radius deviation about 3.84245e-5 mm in a
diagnostic sample. The whole original-coefficient bound was 3.97976e-4 mm, so
qualification correctly failed at the unchanged 1e-7-mm tolerance. Near 90°,
small angle differences selected one versus two rational arcs at adjacent
stations. Although both section curves were circular, their rational parameter
maps differed; aligning knots and interpolating those maps distorted the skin.

`arc_section` now uses a coherent two-arc parameterization for every admitted
angle below pi, with actual authored angle and radius. No angle is snapped and
no requested tolerance is increased. The actual generator's 2x3-degree surface,
with 5x25 controls, now qualifies over all original cells: 628256 coefficient
work and radial error upper 7.423566793544889e-11 mm for constant 1.25 mm. Its
actual variable-radius skin from 0.1 to 0.3 mm qualifies with upper
2.0633082310738824e-12 mm and the same work. The latter uses an explicitly
specified affine station correspondence from immutable endpoint Y values;
no builder receipt authorizes the proof.

Validation: 2 radius integration tests (including whole single-Bezier controls,
actual multi-span constant/variable skins, corrupted final-span controls and
strict budget refusal), 3 generator regressions and 8 B-rep contact regressions
passed, 13 distinct tests. Contact/G2 claims remain separately qualified; circular
sections are not claimed G2 against planar supports. No WASM/UI promotion or
unequal-radius corner-body completion is claimed.

## Saved B-rep skin radius admission and independent STEP control

`fillet_radius::inspect_saved_skin` inspects the actual face of a validated model
with a unique shell owner. It enforces unchanged model tolerance and recomputes
the whole original-cell affine-field radius proof without trusting construction,
contact-center or previous radius receipts. Model topology/lift validation can
refuse changed saved geometry before radial proof; this is not bypassed.

`contact_graft::attach_radius_qualified` privately performs owned graft assembly,
then gates the result on that fresh saved-face proof. Failed or exhausted radius
qualification returns no intermediate model. Closure/trim/source ownership gates
remain mandatory. Nominal quintic G2 setbacks are not presented as circular skin
radius: the nominal-G2 control correctly refuses this additional admission.

The actual circular 1.5-mm fillet is now bound, trimmed and assembled into the
retained source body, serialized/reloaded and freshly radius-checked. Native
assembly uses 3902 cells and 4376 exact work. Saved-skin proof uses 314128
coefficient work, with complete radial upper 5.388964491109614e-11 mm. Seven
B-rep integration tests passed: 2 saved-radius tests and 5 graft regressions,
including variable curved-body assembly. Corrupted saved geometry refuses; inputs
remain immutable. Existing G2 fixtures keep their original construction path.

Artifacts: `output/qualification/saved-rolling-radius-2026-10-16/`, generated by
`examples/saved_radius_acceptance.rs`. Independent OCCT verification via
`scripts/verify-saved-rolling-radius-occt.py` confirms one valid STEP solid and
volume 597.1028752069338 mm³ versus independent
600-6*(1.5²)*(1-pi/4)=597.1028752058655 mm³. Adaptive relative integration request
is 1e-11 (reported estimate 3.757756118798063e-13); the default nonadaptive OCCT
integral was insufficient for this rational face and was not accepted. Independent
169-point skin measurements have maximum radial error 1.2071676991354252e-11 mm;
these samples supplement, not replace, the whole native coefficient proof.
STEP SHA256: 4168020e4c154d81615f4509df15596d0ef9a00c19ca168b81dc1670d5732cbb.

This is a constant circular radius control with plane-end trimming, not a
G2-to-plane join, unequal-radius corner solution or global embedding proof.
Unequal-radius corners/end transitions, full diagnostics, viewer promotion and
all broader roadmap requirements remain open.

## Variable-radius closed saved-skin control

Extended the rolling control through the existing automatic root binding,
plane-end cutting, owned graft assembly and saved-radius gate. The authored
radius is 0.5→1.5 over the FULL original contact parameter domain; retained end
planes cut out Y=[2,8]. The resulting explicit immutable station field is
r(Y)=1.5099896296044402-0.10141139270803481*Y. This produces retained endpoint
radii 1.3071668441883706 and 0.6986984879401616 mm, NOT an assertion that 0.5/1.5
were imposed on the two trim planes. Selection-relative endpoint radius control
remains required work; the actual traced endpoints need not reach full support
bounds.

The complete saved surface and its reload qualify against that explicit field:
radial upper 1.6714813612082556e-11 mm, 314128 coefficient work. Assembly uses
2823 cells and 4376 exact work. Changing the prescribed field by 0.01 mm refuses
radius qualification. Three saved-radius tests and five graft regressions pass.
Existing nominal-G2 fixtures retain their construction path and continue to
refuse an incorrect circular-radius claim.

Artifacts: `output/qualification/saved-variable-radius-2026-10-16/`, from
`examples/saved_radius_acceptance.rs <directory> variable`. The independent
verifier uses the saved explicit input recipe, not native mass properties, to
integrate (1-pi/4)*r(Y)^2 on Y=[2,8]. OCCT reads one valid solid with volume
598.665099219937 mm³ versus independently integrated 598.6650992196171 mm³;
relative adaptive integration estimate 2.0281059848277767e-15. Independent
169-point radial checks have maximum error 1.4586110097525307e-12 mm. Complete
radius proof remains the native original-coefficient certificate, not sampling.
STEP SHA256: 805065bad2ecff0c72678f5317572585d3f9e125de178ebe06af62979a44b7a7.

This is a variable circular skin with prepared planar ends; it does not complete
complex unequal-radius corners, smooth end runouts, global diagnostics or viewer
integration. The full roadmap remains active.

## Radius targets authored at selected coordinate stations

Added native `fillet_surfaces::linear_at_stations` proposal construction and
`fillet_contact::build_linear_at_stations` fresh contact qualification. Inputs
include a world-coordinate index, two ordered station values and two positive
radii. The authored affine world radius field is fixed before construction and
never fitted to output geometry. A bounded solve adjusts the normalized tracing
law to that field. It shares ONE original reference trace across at most eight
skin proposals, within the supplied marching options. Retracing every iteration
was tested and rejected: adaptive endpoint clipping changed the domain of the
law and prevented convergence. No tolerance was increased.

`StationRadiusReport` explicitly separates sampled convergence estimates from
qualification. It records iterations, normalized proposal law, authored world
field and an outward bound on its representation error at the two authored
stations. General nonlinear/reversing traces can remain unconverged; this is
not permission to admit them. The B-rep wrapper rechecks actual original contact
lifts. Whole-skin saved-face proof and owned graft/trim/closure gates remain
mandatory before a radius-qualified assembled model is returned.

The translated/nonunit-chart native control converges in two proposals with
station residual estimate 2.013944566670034e-13 mm and whole-skin radial upper
2.413688783747885e-11 mm. Invalid coordinates and reversed station order refuse.
The actual retained B-rep now targets r(2)=0.7 mm and r(8)=1.3 mm, using the fixed
field r(Y)=0.4999999999999999+0.10000000000000002*Y. This supersedes the earlier
control whose radii were only specified on the untrimmed tracing domain.
Saved/reloaded whole-skin radial upper is 1.3647401134692162e-11 mm; 314128
coefficient work, assembly 3890 cells and 4376 exact work. A changed radius field
refuses. Four native generator and eight B-rep saved-radius/graft tests passed,
12 distinct tests.

Artifacts: `output/qualification/station-radius-targets-2026-10-16/`. Independent
OCCT reads one valid STEP solid, volume 598.6737606500127 mm³ against the authored
station-field integral 598.6737606497962 mm³ (adaptive relative integration
estimate 2.3052361385158626e-15). Independent 169-point radial error is
2.174593838333294e-12 mm; it supplements the native whole-skin proof.
STEP SHA256: 9360f7c886988565d5942a1c3101f06493b1d5616478810dfc404398b145c463.

This establishes selected-coordinate endpoint radius control for the qualified
planar-support case. It does not complete general surface-law correspondence,
complex unequal-radius vertices, smooth end runouts, body thickness/embedding,
WASM/UI integration or the rest of the roadmap.

## Whole original-pcurve G1 on the actual saved variable skin

Added `surface_g1_pcurves::inspect` and saved owned-edge diagnostics in
`sweep_smoothness::tangent::inspect_edges`. Shared 3D curve positions are freshly
verified before normal comparison. Every intersected original knot cell supplies
first derivatives, including all one-sided knot limits; no rounded refit or C2
storage assumption supplies authority. Positive rational pcurve weights and
contained original control hulls are required. An exactly authored constant UV
coordinate is retained as an exact identity, rather than differentiated through
its artificial outward-evaluation halo.

Normal normalization divides by a proved nonzero dominant component using a
signed quotient, then evaluates the correlated direction ratios. This avoids
losing direction through an unrelated magnitude interval. Only first jets are
computed; curvature/G2 is not inferred. One coherent Same/Opposite orientation
must qualify over the complete seam. Budget exhaustion returns no normal upper
certificate. A fresh point query can prove both orientation branches outside
tolerance and returns a correspondence violation with its normalized parameter.
Endpoint normals alone never authorize a seam.

`contact_graft::attach_radius_tangent_qualified` now privately assembles and
radius-checks the actual saved skin, then qualifies G1 on explicit PATCH contact
edge indices mapped to saved edges. Assembly and tangent position/normal checks
share the original cell/exact-work remainder. Failed tangency returns no model.
Sharp caps are explicitly outside this contact scope; requesting them as G1
contacts refuses.

The selected-station 0.7→1.3-mm closed control qualifies both contact seams:
648 additional cells, zero additional exact work, normal chord uppers
1.3572180009755004e-11 and 7.836187186065378e-12. Combined assembly uses 4538
cells and 4376 exact work; complete saved-skin radius upper remains
1.3647401134692162e-11 mm with 314128 radius coefficient work. The native G1
regression covers opposite orientation, strict one-less-cell refusal, and a
surface whose endpoint normals match but whose interior tangent plane differs;
the latter yields a certified violation parameter. Nine B-rep saved-radius,
saved-tangent and graft regressions plus that native regression passed (10 tests).
Affected native/saved tangent checks were rerun after restricting to first jets.

Artifacts: `output/qualification/saved-radius-g1-2026-10-16/`. Independent OCCT
STEP checks retain the previous valid-solid, volume and radial results and now
measure contact derivatives too: maximum sampled unoriented normal chord
3.510868489494055e-12. These independent samples supplement the whole native
normal proof. STEP SHA remains
9360f7c886988565d5942a1c3101f06493b1d5616478810dfc404398b145c463.

This is G1 tolerance qualification of the declared contact seams, not exact C1
storage continuity, G2, smooth cap transitions, full thickness/embedding or an
unrestricted unequal-radius corner solution. No active viewer was promoted.

## Full pair coverage and retained coordinate-plane contact constraints

A fresh diagnostic of the saved variable-radius body confirmed injectivity of
all 7 faces but initially left 8/21 pairs unclassified. The new
`self_intersection::inspect_qualified` combines original face/pair inspectors
with fresh owned-vertex and boundary-tube attempts. Initial reports remain
unchanged; exact vertex proofs, metric localizations, unresolved pairs and actual
interior contacts have separate collections. A detected interior contact or an
unvisited suffix can never produce positive coverage. Contraction spans,
linear cells, pair cells and fallback cells share the geometry ledger; fallback
trim work is also conservatively charged to the UV ledger. Additional exact
vertex/tube work has its own explicit capped ledger (`fallback_exact_work`);
initial pair hull work retains its original geometry/domain accounting.

Six pairs localized immediately. The remaining two involved the circular skin's
contact with coordinate-plane supports. Construction now imposes endpoints from
actual original support contact evaluations and the first tangent pole's
coordinate-plane constraint only when ALL original support controls have that
literal coordinate. These are new proposal constraints derived from source
geometry, not a near-plane test or tolerance-based welding. Contact, full radius,
G1 and saved assembly are freshly requalified. Rational section proposals are
not described as mathematically exact circles after imposing these constraints;
whole-skin coefficient radius proof remains authoritative.

The owned boundary-tube classifier now admits clamped multi-span B-spline
controls only with a sufficient strict-sign condition on all rows beyond the
first degree rows (or before the last degree rows). Positivity of the active
basis excludes interior zeros, including interior knot values. The existing
single-Bernstein endpoint-basis proof remains unchanged. A regression with an
interior zero row refuses this extension; it cannot use the old single-chart
argument to authorize an interior contact.

Updated artifacts: `output/qualification/retained-planar-contact-2026-10-16/`.
All 7 faces qualify as injective; all 21 pairs are covered; 8 pairs have explicit
owned-edge metric-locality bounds (maximum 1e-7 mm); there are no unresolved or
unvisited pairs and no detected interior contact. Diagnostics use 7730 geometry
cells (including 14 contraction spans), 83020 UV/domain cells and 130530
ADDITIONAL exact work. `toleranceContactLocalityProven=true` while
`absenceProven=false`: metric contact localization does not become strict
absence or a valid filled-volume/embedding certificate. Thickness remains
unchecked by this diagnostic.

Source-derived contact constraints reduce saved assembly+G1 work to 2388 cells,
with 4376 exact work; radial upper is 8.199619902981345e-12 mm, normal chord uppers
8.66150383654634e-13 and 1.3322676295501886e-15. Independent OCCT STEP checks again
read one valid solid, volume 598.6737606500152 mm³ versus authored integral
598.6737606497962 mm³, sampled radial error 1.6375789613221059e-12 mm and sampled
contact normal chord zero. STEP SHA256:
166582f8f39f142e6be28a75f9c0461260a305e248875dcdcacaf6301a04526a.

Validation: 6 native radius/generator tests and 12 B-rep graft, saved-radius,
saved-tangent and qualified-diagnostic tests passed (18 distinct tests).
Diagnostic regressions include genuine interior crossing, unvisited pairs,
metric-vs-absence distinction, shared bounded ledgers, immutable inputs and
multi-span interior-zero refusal. Strict absence, complete wall thickness,
smooth end runouts and complex corners remain open, as do the broader roadmap
and active viewer promotion.

## Opposing-face inventory for multi-span skins

The automatic clearance baseline for the retained planar-contact control could
not classify any normal-opposition pair: all six other faces stayed candidates.
The conservative candidate minimum therefore included legitimate adjacent contact
edges and reported BelowRequiredClearance with an upper witness about 3.706e-7
mm. Selection was explicitly incomplete; this was NOT proof of a thin material
wall.

Added `surface_g1_pcurves::rectangle_normal_bounds`, reusing original first-jet
cell evaluation and dominant-component normal normalization over every original
knot cell, including one-sided limits. This permits normal inventories where a
C0-stored multi-span basis has no C2 storage proof. It does not assert exact C1
continuity, curvature, normal cones, material chords or body thickness. A native
regression explicitly checks the stored interior knot where ordinary evaluation
returns no unique normal, while original one-sided bounds cover both limits.

`wall_opposition` now intersects the independently valid existing shape-operator
and new first-jet normal enclosures when both are available. Either complete
bound can stand alone when the other is unproven. Both queries share the existing
cell ledger. Retaining the existing certificate matters for rotated smooth
charts: using only the new Cartesian enclosure lost those qualifications and
was rejected by regression. Unknown/mixed angular faces remain candidates; no
witness sample or source class label excludes them.

Current artifact `wall-clearance-qualified-normals.json` in
`output/qualification/retained-planar-contact-2026-10-16/` excludes four joined/
cap faces and retains candidate faces [1,3]. For the complete candidate set,
clearance is bounded by [8.699955819008643,8.700001462777307] mm, sufficient to
prove the requested 1-mm clearance. The distance has NOT converged to the
requested 1e-7-mm gap: `converged=false`. Work stays at 100000 geometry cells,
900483 domain cells, including 470 normal-selection cells. The two candidate
faces have mixed/unresolved angular opposition, so `selectionComplete=false`.
Neither this interval nor the angular filter is a whole-solid wall thickness
certificate; material chords and cavity pairing remain open.

Validation: 2 native normal-range/G1 tests, 3 opposing-face tests and 3 clearance
tests pass (8 distinct tests). Coverage includes the existing translated/rotated
floor control, zero-work candidate retention, exact-threshold and missing-cover
refusals, an actual thin closed control, and the multi-span variable skin. The
new skin test keeps the full conservative candidate set and proves clearance
with a bounded distance query; it does not require distance convergence or
silently turn incomplete angular selection into material-wall qualification.

## Complete finite-interval coverage and conditional material chords

Added `material_chord::inspect_interval` over the exact represented line
origin+direction*t and a closed finite parameter interval containing t=0. Both
negative and positive line parameters are inspected. Every original face's full
line/surface root cover is intersected with the finite interval and original UV
trims. Only certified outside parameter/trim regions are discarded; root,
trim-boundary, work and endpoint-band unknowns remain explicit. Certified roots
carry original face IDs, UV boxes and parameter bounds. The interval length is
bounded outwards, with arbitrary nonzero direction scale. It is NOT endpoint
wall separation or minimum thickness. An authored interior origin is required
so point parity anchors at exactly that binary64 point, not a rounded computed
midpoint.

`material_chord::inspect` now exposes InsideMaterialInterval, BoundaryCrossing,
OutsideMaterial and Unresolved. A confirmed boundary crossing returns early
without running irrelevant volume/embedding stages; skipped checks are None,
not positive flags. Positive material qualification requires fresh topology/
orientation/cavity-containment audit, complete trim-region audit, exact saved
edge/lift and UV-join closure, full boundary absence, complete finite-interval
coverage and odd origin parity. No old certificate or caller inside flag is
accepted. Numeric stage budgets are shared: <=100000 geometry cells, <=1000000
UV/domain cells and the established bounded exact closure/fallback allowances.
Metric boundary-contact locality never substitutes for the required absence
precondition. This strict current gate does not yet admit general rounded
nonexact B-rep closure via a deformation/error certificate.

Artifacts: `output/qualification/material-chord-2026-10-16/cases.json` from
`examples/material_chord_acceptance.rs`:

- Cube [0,2]^3, origin [1,1,1], direction [1,0,0], interval [-0.99,0.99]: all
  preconditions pass and the entire interval is qualified inside material.
  Its length enclosure is [1.9799999999999989,1.9800000000000018] mm; 78 geometry
  cells and 56 domain cells, including the complete trim/closure gates.
- Actual Boolean cavity [0,10]^3 minus [2,8]^3, origin [1,4,5], interval
  [-0.99,8.99]: the origin is separately confirmed odd-parity, but the complete
  chord crosses cavity boundaries at t≈1 and t≈7. BoundaryCrossing is certified
  in 42 geometry/16 domain cells; material qualification is false. Point parity
  alone cannot authorize the full segment.
- Short cavity-wall interval [-0.99,0.99]: finite-interval coverage is complete
  and boundary-free, origin parity is odd, trims and exact closure pass. The
  Boolean has 60 partitioned faces and 1770 pair candidates; full boundary
  admission is unfinished (49 explicitly unresolved inspected pairs plus a
  pending suffix starting at [22,41]). Therefore material status remains
  Unresolved, not inside material. Current work is 50357 geometry/165652 domain
  cells; the source is not silently simplified to an easier cavity fixture.

Validation: 2 material-interval tests and 3 qualified-boundary tests passed
(5 distinct tests). Coverage includes inside/outside cubes, nonunit direction,
boundary-origin rejection, endpoint bands, tiny budgets, a real Boolean cavity,
inside-point-vs-whole-segment distinction and an even-parity void anchor. The
qualified-boundary inspector now preserves an explicit unvisited suffix when
face inspection exhausts the shared geometry allowance before pair inspection;
it does not call a positive-budget primitive with zero or treat an empty result
as complete coverage.

This adds a necessary material-path primitive, not a complete normal-ray family,
first-exit endpoint binding, cavity-wall pairing or whole-wall minimum search.
The 60-face Boolean boundary audit, general curved-body admission, UI promotion
and the remaining roadmap are still open.

## Joint exact-domain admission of the actual 60-face Boolean cavity

The earlier conservative material-chord traversal left a prefix of the 1770
pairs unfinished because generic trimmed contact searches consumed the budget
before owned-contact fallback. The existing native joint boundary auditor was
inspected and tested on the SAME actual Boolean model. It first recomputes exact
edge/lift agreement, exact UV joins, trim validity, positive outer winding and
whole-face injectivity. Only then can its original-control hull contact
certificates authorize known shared contacts before generic pair search.
No Boolean/sweep tag, supplied certificate or simpler cavity fixture provides
this authority.

`material_chord::inspect` now uses `boundary_embedding::inspect_with_linear`
with explicit remaining geometry/domain/exact allowances after its interval,
trim and saved-closure stages. The full joint report is retained, including
winding prerequisites and original pair coverage. Duplicate prerequisite work
is charged rather than omitted. All numeric geometry/domain stage allocations
remain inside the caller's 100000/1000000 limits; the preliminary exact agreement
work is subtracted from the joint exact allowance. Source topology/orientation/
strict cavity containment remains independently audited.

The actual 60-face [0,10]^3 minus [2,8]^3 model now qualifies: all 1770 pairs,
216 fresh hull-contact certificates, no unresolved/unvisited pair. Standalone
joint contact traversal uses 1554 geometry cells and zero contact-domain cells.
The complete positive material-interval query uses 2877 geometry cells and 984
domain cells. Its original origin [1,4,5], direction [1,0,0], interval
[-0.99,0.99] is qualified wholly inside material. The void interval around
[5,4,5] is qualified OutsideMaterial; the longer cavity-crossing interval remains
BoundaryCrossing in 42 cells. This supersedes the earlier Unresolved short-wall
result without changing the source Boolean or relaxing any tolerance.

Updated `output/qualification/material-chord-2026-10-16/cases.json` records the
new results. The earlier incomplete scheduler receipt is preserved at
`output/qualification/material-chord-before-joint-boundary-2026-10-16/cases.json`.
`examples/cavity_boundary_diagnostics.rs` independently reruns the joint audit
and volume-role audit before exporting `output/qualification/cavity-joint-boundary-2026-10-16/`.
Independent OCCT verification via `scripts/verify-cavity-boundary-occt.py` reads
one valid solid, two shells, 60 faces and bounds [0,0,0]..[10,10,10] within
2e-7 mm. Adaptive volume is 783.9999999999999 mm³ versus the independent
1000-216=784 mm³ target. STEP SHA256:
cdd52af1f6d8bd9fb0d3d370b69ecd4bb0cb324a4036aa7e1ad2123d4de28c03.

Five targeted material/boundary regressions pass, including a new interior
surface fold whose authored boundary curves and ordinary model validation remain
unchanged. Fresh material admission rejects that modified model; an earlier
positive result is never reused. Low UV work, exact endpoint bands, scaled
line directions, cavity crossings and zero remaining pair work retain explicit
unresolved states rather than fabricating a pass.

This establishes one specified interior chord and complete boundary admission
for this partitioned Boolean control. It does not establish a minimum thickness
over a whole face/body, first-exit wall endpoint binding, general NURBS corner
admission, smooth end transitions or viewer integration. Those roadmap
requirements remain open.

## Atomic supported trimming and cap admission

`face_cut::trim_ruled_and_cap` now keeps shared-root partitioning, winding-based
sector selection and `cap_region` admission in one private transaction. Input
seeds refer to immutable original face charts; boundary/unknown seeds refuse.
Original face indices must be distinct, including explicitly removed uncut
faces. No intermediate partition is returned on failure. Geometry and exact
work are shared across all stages, without increasing the 100000/1000000 caps.

The API uses the existing ruled/plane intersection solver. It is not an
arbitrary B-rep Boolean solver and does not infer a material side without user
intent. It does not construct corners, prescribe radii, or establish G2.

`curved_cap_acceptance` now uses the public atomic operation. The independent
STEP receipt in `output/qualification/atomic-trim-cap-2026-10-16/` confirms one
valid solid, 7 faces, 15 edges, 10 vertices and volume 1.4104166666666647 mm³
(target 677/480); absolute volume error is 1.9984014443252818e-15 mm³.
Native construction uses 657 geometry cells and 999998 exact work units.
STEP SHA256: 3936c4716e630d64f0dd2a2ecaa601c9b89413743150aa98c086bfe2a107f863.
Regression coverage includes budget exhaustion, duplicate source ownership,
nonfinite/boundary seeds, incomplete rim selection, source immutability and
serialized reload closure.

Automatic general NURBS intersection/retention, multi-radius corner generation,
smooth ends, whole-body embedding/thickness and WASM/UI promotion remain open.

## Ruled section axis independence on original saved charts

The section proposal now handles ruled charts in either U or V. On bilinear
charts it selects a sign-definite homogeneous carrier denominator instead of
assuming that V is transverse. Automatic root ordering uses the same selected
longitudinal coordinate. Coefficient transposition is an exact proposal-only
permutation; the pcurve is mapped back before continuous lift qualification
against the ORIGINAL saved surface. Source/cutter controls remain unchanged.
Nonperiodic 4096-control bounds and the existing degree 1..3 ruled restriction
remain explicit. No general surface-intersection coverage is inferred.

Regression coverage includes rational unequal ruling weights, nonunit chart
domains, clipped original-chart ends, whole original lifts, and an actual
closed B-rep whose surface charts, local trim winding and shell uses are all
transposed coherently. The atomic cap operation succeeds on that same geometry
with 657 cells and 999994 exact work units. Independent OCCT reads a valid
1-solid/7-face/15-edge/10-vertex result, volume 1.4104166666666649 mm³ versus
677/480, error 1.7763568394002505e-15 mm³.
Artifacts: `output/qualification/transposed-trim-cap-2026-10-16/`.
STEP SHA256: 682311052745685b3f11c5c27d90ff2f8421cc984503db6bc9cd1fc4c240176b.

General curved/curved trimming and generated unequal-radius corner topology
remain unimplemented; axis independence is a prerequisite, not their completion.

## Adaptive original-root construction for curved/curved face cuts

`nurbs-core::ss_intersection::construct_fiber_trace` now constructs actual
piecewise cubic 3D and dual UV curves on a freshly qualified original root
fiber. Newton solves and implicit first derivatives propose Hermite controls;
no sampled residual authorizes a result. Every piece passes fresh uniform
root replay, containment, monotone clock covering the entire free interval,
and original C-S(P) agreement on both supports. A shared cache makes every
adjacent 3D/UV endpoint literally equal. Failure returns no complete chain.
Original support control count is bounded by 4096; the 100000-cell allowance
includes source rectangles and agreement cells. Evaluations, depth and piece
count are independently bounded. An exact-used-budget replay succeeds; one
cell less refuses the complete trace.

The control intersects TWO genuinely curved degree-2-by-2 original surfaces:
S1(u,v)=(1/4+u/4,v,u²/16+1.02v²) and
S2(s,t)=(s,t,1/4+s²/8+.02t²). Thus the original root is
x=1/4+u/4, y=sqrt(1/4+x²/8-u²/16), z=1/4+x²/8+.02y².
For free u in [0,.75], construction yields 16 continuously qualified pieces,
299 proof cells, 150 surface evaluations, and 31 attempts at tolerance 1e-7 mm.
No planar or ruled reduction is used.

A rotation of .37 rad about Y plus translation [2,-3,1] exposed a genuine
preconditioned interval-correlation loss on the SECOND support free clock.
The same full original box is now admitted: `fiber::certify` can freshly apply
R to interval homogeneous coefficients BEFORE coordinate hulls/derivatives.
This is original coefficient arithmetic with a common proof origin, not a
rounded transformed surface. It uses the SAME fixed R/scales and unchanged
geometry/tolerances. The new fallback charges four additional source
rectangles. The rotated second-clock control takes eight rectangles; seven
refuses. Existing inexpensive successful bounds retain their original work.

`brep-core::face_cut::partition_fiber_at_face` discovers the two roots from ALL
original outer edges against the actual trimmed cutter, splits owned edges,
checks saved vertices against whole root position boxes, and derives the free
interval from actual face-local owned endpoints. Caller search boxes grant
no certificate. After constructing the trace it binds actual saved 3D/UV ends
and freshly requalifies every piece before trim and topology admission. Whole
cutter membership includes holes. All stages are private and share one ledger.

The actual B-rep control over u in [0,1] creates 32 section edges, each shared
by two opposite face uses; 2277 proof cells, 300 surface evaluations and an
original-root error upper of 2.499999999999999e-8 mm. Both face sectors survive.
Serialization/reload is equal. A cutter hole interrupting the same branch
refuses instead of treating the cutter as its untrimmed carrier.

Reproduction: `examples/fiber_cut_acceptance.rs` writes
`output/qualification/curved-fiber-cut-2026-10-16/`. The independent
`scripts/verify-curved-fiber-cut.py` evaluates saved coefficients at 32800
sites; maximum sampled analytic-root error is 4.766145811352374e-10 mm.
It independently checks exact endpoints and two opposite uses per saved
section edge. These samples supplement the native WHOLE-piece proof.

This is local original-root construction and atomic face partitioning, not
complete surface/surface branch coverage, arbitrary automatic search-box
selection, a closed solid, unequal-radius corner generation, or WASM/UI
promotion. Those requirements remain open.

Final focused verification for this addition: 33 NURBS integration tests,
12 B-rep integration tests, and the original-operator receipt unit regression
pass (46 total). The saved curved-fiber artifact and independent samples were
regenerated after the correlated-operator change. No active WASM binary or UI
registration was changed in this stage.

## Shared curved-cut network and actual closed cap body

`partition_fiber_network` now inspects the union of ORIGINAL outer edges once.
Every incident face reuses the same root box and authored root vertex. Each
face then constructs and admits its own actual curved branch, with a common
100000-cell, 1000000-exact-work and 4096-evaluation allowance across the network.
`trim_fiber_and_cap` privately performs this network, explicit winding-based
removal intent, and owned cap replacement. No intermediate model is returned
on failure. Returned root vertex indices are mapped into the FINAL compacted
body; their coordinates and persistent identities are regression checked.

Two wide original root boxes failed strict inclusion. New
`FiberSources::certify_contracted` uses only freshly computed original
Krawczyk root images to restrict UNKNOWN coordinates, preserving every possible
root and the ENTIRE original free interval. At most 16 attempts share the
original rectangle ledger. Numerical samples never exclude a root. The
returned fiber names its actual restricted domain; it does not claim a whole
surface intersection census. Tests check the unchanged free interval, analytic
roots, original source immutability and exact-used/one-less work limits.

Adaptive cubic proposals now reserve composition accuracy for the whole parent
operator. Local operators may have easier contraction bounds, which cannot
replace later whole compound admission. Consecutive authored pieces are stored
as compound C0 NURBS curves, preserving their controls/endpoints, with at most
85 pieces per curve to respect the existing 256-control CURVE limit. Every
actual saved compound is freshly checked against its original root chart and
both supports. No source edge identities are welded by proximity.

Whole-root clocks now support positive rational continuously monotone
B-splines: ordered free-coordinate poles, strict progress on every active local
block and interior multiplicity <= degree are mandatory. Original curve/surface
agreement can cover matching clocks at literal knot endpoints, including
exactly representable reverse clocks. This avoids normalized endpoint halos;
all original piece compositions are still verified and charged. Nonmatching
or inexact clocks use the existing bounded traversal. A dedicated regression
covers nonunit compound clocks, reverse traversal, one-less work, interior
damage and a changed knot clock.

The first closed cap attempt exposed a true saved UV closure mismatch:
independent projection solves proposed slightly different UVs for the SAME
owned vertex. `cap_region` now binds endpoint proposals by original vertex
identity and freshly verifies each edited WHOLE pcurve lift. This is no world
welding or tolerance increase. Trim refusal messages also name the failing
face and loop outcome. Neither proof caps nor the 10000 trim-pair limit were
raised: compound curves reduce the contour representation instead.

Actual closed control: stock 0<=x,y<=1, -1<=z<=y+x²/4, trimmed by
h(x,y)=-.2+x²/8+y²/50. All four side charts are sectioned; the uncut old top and
upper sectors are removed. The resulting curved cap shares six compound edges
and four original root vertices with retained walls. These walls are physical
coordinate planes with nonlinear original charts; this control is not evidence
of general curved/curved CORNER construction.

Native final result uses 36982 proof cells, 33356 exact work and 1850 surface
evaluations. Root-error upper is 2.4999999999999996e-8 mm. Saved/reloaded model
is equal, cap injectivity and trim regions pass, and closure is WithinTolerance.
`examples/fiber_cap_acceptance.rs` reproduces
`output/qualification/shared-curved-cap-2026-10-16/`.
Independent `scripts/verify-shared-curved-cap-occt.py` derives volume from
integral(1+h)=.8+1/24+1/150=509/600 mm³, then imports the actual STEP.
OCCT reports one valid solid, 6 faces, 14 edges, 10 vertices,
volume 0.8483333330764017 mm³ (error 2.569316981393399e-10 mm³).
STEP SHA256: 3f5db04c23b76a5fefa8a229420598ccad7e4c2967481d7efb109efb254e9991.

The earlier genuinely curved/curved open-sheet control is retained at
`curved-fiber-cut-2026-10-16/`. Its current compound variant is separately saved
at `curved-fiber-compound-2026-10-16/`: 39 pieces in one actual 3D edge, 1432
proof cells, root-error upper 2.499999999999999e-8 mm. Independent evaluation of
39975 saved-coefficient samples finds maximum analytic-root error
3.0285418617381765e-10 mm. Samples supplement the whole native proof.

Focused current verification: 38 NURBS and 13 B-rep integration tests pass
(51 total), covering contraction, original clocks, actual shared roots, atomic
caps, radius and tangent gates, and reload. No active WASM binary/UI was changed.

Still open: arbitrary branch search/census; root charts whose unknown
coordinate reaches a natural boundary at an endpoint; general curved/curved
closed-solid acceptance; smooth fillet end transitions, unequal-radius complex
corners, full wall minima/global embedding diagnostics, and WASM/UI admission.
The actual cap/wall joins here are sharp, with no radius or G2 claim.

## Unequal coaxial radii: whole G2, station planes and a closed saved body

`nurbs-core::radius_transition` now builds an authored quarter skin between
constant circular radii r0 and r1. Its law is the quintic Bernstein profile
[r0,r0,r0,r1,r1,r1], so first and second radial derivatives vanish at both
interfaces. This is an actual smooth change in radius, not a straight cone.
The centerline is linear; a finite nonparallel authored frame is proposed and
all ACTUAL resulting coefficients require fresh admission. The primary
control uses r0=.1 mm, r1=.3 mm and transition width .1 mm. Reversed radii and
rotated/translated controls are independently exercised natively.

Two real blockers were removed without changing tolerances or proof caps:

- Retained support and bridge cross directions face opposite ways. New
  `surface_collar::inspect_g2_opposite_cross` applies that affine sign to ODD
  transverse homogeneous jets inside the proof only. Source evaluation stays
  in its actual retained chart; no source extrapolation or geometry edit is
  accepted. Reported orientation accounts for the proof frame. The second
  transition proof uses its actual V-reversed chart.
- Actual source composition and candidate may have different Bernstein
  degrees. Fresh original homogeneous INTERVAL nets are elevated to a common
  basis before correlated residual/normal/world-tensor inspection. No rounded
  elevated source surface is substituted. Both degree and coefficient work
  remain bounded. Existing same-basis and mapped-corner regressions still pass.

The first prototype also exposed a false domain refusal: outward 1+(-1)
produced a subnormal halo outside [0,1]. The existing exact dyadic addition
helper now retains a literal endpoint ONLY when exact representability is
proved; other sums remain outward intervals. One ulp beyond the original
retained chart is regression-tested to refuse.

Admission separately checks original band radii, every original-support G2
seam, the entire candidate radius law and whole-chart injectivity. Distance to
a station center alone is insufficient to claim a circular section. A new
original homogeneous coefficient proof also bounds deviation from the
perpendicular station plane, including all U and V. The radial bound is then
converted conservatively to the actual radius projected into that plane.
Physical centerline length AND perpendicular plane spacing qualify the authored
width. No sampled fit or prior receipt grants these flags.

A degree-7 interior perturbation preserves both G2 endpoint certificates but
fails the whole radius gate. A tilted section preserves distance to its center
but fails the station-plane gate. These defects cannot inherit admission from
good endpoints. Original input/source immutability, bad frames/radii, missing
work, signed orientation and exact domain boundaries have regressions.

The default complete skin has 105 proof cells. Whole physical circular-radius
error upper is 2.6020852139652394e-14 mm; station-plane error upper is
1.1775693440128322e-16 mm; width error upper is 1.1102230246251568e-16 mm.
G2 seam [position mm, unit normal chord, signed world tensor 1/mm] uppers:
[7.302909173437289e-15,1.1822369214627024e-9,4.692412100495208e-5] and
[1.639127400960807e-14,3.536774660305006e-10,3.879732522170644e-6].
Limits are [1e-7,1e-5,1e-4]; these are whole-seam bounds, not sampled maxima.

`brep-core::radius_transition_body` assembles the actual source bands, bridge,
radial side faces and planar quarter-disk caps by authored shared vertex/edge
identity. No polar cap singularity or world-proximity welding is introduced.
Trim and saved-use closure are freshly mandatory. Only actual circle edges
[2,10] name the G2 joins; radial/end faces remain sharp. Total saved control:
1 body, 11 faces, 21 edges, 12 vertices, 211 proof cells, 3364 exact work and
79805 shared coefficient work. Serialization/reload is equal; a smaller body
budget refuses, and the reversed-radius rotated body retains closure/naming.

`examples/radius_transition_acceptance.rs` reproduces
`output/qualification/unequal-radius-transition-2026-10-16/`.
Independent `scripts/verify-radius-transition-occt.py` imports the actual STEP,
checks hashes, validity, topology, dimensions, circular radius samples and
signed world curvature tensors on both seams. It derives volume from the
quintic polynomial independently (integral smoothstep5²=181/462).
OCCT: valid 1 solid; volume .011440973240842135 mm³ versus analytic
.011440973246027764 mm³; error 5.185629703419181e-12 mm³. Maximum 507-point
radius sample error is 2.220446049250313e-16 mm; 50 seam samples give normal
chord 8.408927699556912e-15 and tensor difference 1.9496115277262817e-13 1/mm.
Independent samples supplement the WHOLE native proofs. STEP SHA256:
0cedfef366ed18ef3eca79739bbd6a6c9d3731a5e5f0c6896574063beaab8594.

Focused verification: 22 NURBS and 12 B-rep integration tests pass (34 total),
including prior collars/corners/grafts and the new physical-radius/plane gates.
No active WASM binary or UI registration changed.

Scope remains a straight coaxial quarter transition and its closed control,
not arbitrary NURBS-edge filleting or multi-edge corners. Whole-body G2,
rounded radial/end faces, general rolling-ball envelopes, full wall minimum and
self-intersection qualification, UI admission and the three production-part
acceptance chains remain open. The next corner step needs general signed
cross-frame gains and direct actual-patch/original-support admission.

### Direct original-support corner admission

Implemented signed affine transverse jet gains, including non-dyadic gains.
The actual candidate seam is compared directly with the original surface and
its original pcurve. Candidate side charts use only control/weight permutations,
reversals and normalized Bernstein endpoint labels. The proof includes the
whole seam's position, normal and signed world curvature tensor.

Mapped construction reserves half the remaining proof budget for this direct
check. A complete direct check plus actual patch injectivity can admit the
patch independently of an inconclusive auxiliary-collar comparison; a proven
direct violation refuses it. Saved B-rep replacement requires direct admission,
fresh trim checks and closure. No proof caps or geometric tolerances increased.

Regressions cover curved original supports, reversed seam traces, nonunit
candidate parameter labels, signed gains -0.5 and -0.3, hidden second-jet damage,
and exact-used versus one-less cell budgets. The focused suite passes 22 NURBS
and 4 B-rep tests. The mapped closed control uses 325 cells, 65644 exact work
and 210280 coefficient work; reload remains closed with 30 faces, 60 edges and
32 vertices. Largest direct normal bound is 5.237278675822676e-12 and largest
curvature tensor bound is 6.395543492392884e-11 1/mm.

Artifact: `output/qualification/mapped-corner-direct-original-2026-10-16/`.
Independent OCCT import confirms one valid solid and volume
30.37500000000001 mm³ versus 30.375 mm³ expected. STEP SHA256:
3f2e43ffae77f7e426f3bdc00d5656d510eba4770c3c66baf263f894c9203450.

This control does not establish a general unequal-radius corner construction.
Direct chart normalization currently supports single clamped nonperiodic
Bernstein patches. Arbitrary trim branch coverage, general rolling-ball corner
and end transitions, global embedding/thickness and production-part acceptance
remain open. This work has not been promoted to the active WASM or UI.

### Automatic local curved-cut search proposal

`face_cut::partition_fiber_at_face_automatic` now derives its source/cutter
parameter ranges from original knot domains, isolates the original owned edge
roots once, and chooses the free source coordinate by normalized endpoint
displacement. The other search ranges are proposed from the two original
endpoint UV locations and original cutter-root UV boxes, padded by one eighth
of the original domain and clipped to that domain. Endpoint padding is only a
proposal: the unchanged full-fiber, original trace, actual bound endpoint,
cutter trim and partition proofs remain mandatory under one shared budget.

Searching the complete unknown domains was inconclusive on the curved/curved
control; the root-based proposal passes without tolerance or budget changes.
Tests cover both U and transposed V charts, insufficient work, immutable input
models and refusal when the actual cutter has a hole across the trace.

This API constructs one certified local section with exactly two transverse
outer boundary roots. It does not establish a hidden-branch census, closed
intersection loops, cuts through source holes, or automatic whole-body material
selection. Network requests still explicitly supply their search charts.

### Shared-root automatic network and closed cap

Network chart authoring is now optional via
`partition_fiber_network_automatic` and `trim_fiber_and_cap_automatic`.
The latter accepts original face indices, removal-sector UV seeds and explicit
uncut faces to discard. Each original union edge is queried once; neighbors
share root vertices and original root UV boxes. Sections and cutter closure are
built in one transaction. Explicit-chart APIs retain the same implementation.

Initial endpoint neighborhoods were insufficient for nonlinear wall charts.
Automatic search now tries at most four nested proposals, expanding unknown
coordinates within original knot domains while preserving the complete
endpoint-bound free interval. Each attempt runs a fresh original Krawczyk
whole-fiber proof; unsuccessful attempts consume the shared cell allowance too.
Successful proposals are rechecked during actual trace construction and after
endpoint binding. No tolerance, work cap or control budget was increased.

Six integration tests pass: four single-face cases and two shared-cap cases,
including explicit and inferred charts. The automatic control preserves four
shared original roots with three incident edges at every final root vertex;
serialization/reload is equal. Costs: 41306 cells, 33356 exact work and 2056
evaluations; root error upper 2.4999999999999992e-8 mm.

Reproduce with `fiber_cap_acceptance <directory> --automatic`.
Artifact: `output/qualification/automatic-shared-curved-cap-2026-10-16/`.
Independent `verify-shared-curved-cap-occt.py` confirms one valid solid, six
faces, fourteen edges, ten vertices and analytical bounds. Volume is
0.8483333331161901 mm³ versus 509/600 mm³, error
2.1714330333821863e-10 mm³. STEP SHA256:
97d19202a62ac49d1b3ceb1816158ef349c171ae0922ea5647308e96a77b32a3.

Material selection remains explicit. Local chart qualification does not census
hidden branches or support closed intersection loops and source-hole crossings.
The cap has sharp seams and is not a general corner fillet. General unequal-radius
corners, end transitions, whole-body embedding/thickness, production-part
acceptance and active WASM/UI promotion remain unfinished.

### Multiple boundary-connected curved branches and stable native snapshot

`partition_fiber_branches_at_face` queries original outer edges once and matches
up to sixteen transverse roots by complete original-support fiber construction
and actual descendant-face trim admission. Boundary ordering does not grant
connectivity. All roots must be paired, otherwise no partial model escapes.
The report retains refusal endpoints/codes, final sectors and shared branch ends.
Failed proposal proofs expose their actual rectangle work; failed construction
paths conservatively reserve their entire offered counters. Total caps remain
100000 cells, 1000000 exact work and 4096 trace evaluations.

Search proposals now use physical tangent-speed scaling, eight bounded dyadic
sizes and up to three recenterings from fresh original root images. Already
included coordinates are not widened. A proposal never authorizes admission.
When a single uniform operator is insufficient, the free interval is covered
by freshly qualified original fibers (at most 64 intervals, depth at most 8).
Construction binds actual 3D and both UV endpoints at interval joins, then
requalifies the edited pieces and each saved compound chunk. Total trace pieces
remain bounded by 256; compound curves retain the 256-control limit.

The native operator can propose positive norm weights but must recompute every
row sum outward from original derivative bounds. Independent enclosures of the
same fixed-R Krawczyk map may be intersected. A balanced norm is retained only
when it has a proven contraction and improves the verified physical root-error
gain. This avoids artificial amplification from nearly constant UV coordinates.
Original inclusion remains mandatory and operator receipts reverify freshly.

Controls pass for two curved branches/four roots/three sectors and three curved
branches/six roots/four sectors. The cubic case exercises multiple root fibers.
Actual saved curves also pass analytical residual samples; these supplement
the whole original-support proofs. Rejected pair candidates, shared edge uses,
source immutability, insufficient work and codec reload are covered.
Focused verification: 36 native NURBS integration tests, two operator tests and
eight B-rep integration tests pass (46 total).

The existing automatic closed-cap control also passes on current code:
32378 cells, 33356 exact work, 1542 evaluations; four original shared roots,
six faces, fourteen edges and ten vertices. Artifact:
`output/qualification/automatic-shared-curved-cap-current-2026-10-16/`.
STEP SHA256: 1b55686cefc19cb97a9d38e88d0c36045433e02af5a74ada45dea0502fe7af52.
Independent OpenCascade import passes validity/topology/dimensions; volume
0.848333333392638 mm³ differs from 509/600 mm³ by 5.930456126179706e-11 mm³.

Stable native inputs were copied byte-for-byte to
`/private/tmp/osv-cad-stable-20261016T171722Z/crates` with full `manifest.json`,
`SHA256SUMS` and focused test logs in the snapshot root. All 3366 files
(93018563 bytes) matched source both before and after capture. Native fingerprint:
2618f2510c93b3c3ea0afe1e7edc9d35261e03402afae652ded5b86a38e5ef02.
An identical separate development copy is at
`/private/tmp/osv-cad-next-20261016T171722Z/crates`.
Compact repository receipt:
`output/qualification/fiber-branches-stable-2026-10-16/manifest.json`.
The snapshot qualifies these focused native checks, not a full WASM/UI release.

Remaining: complete hidden/closed branch census, source-hole crossings,
automatic whole-body material selection, general unequal-radius corner and end
fillets, full thickness/self-intersection admission, production-part acceptance
and active WASM/UI integration. These controls do not close the roadmap.

### Multiple branches across neighboring faces (isolated development)

Further native development is in
`/private/tmp/osv-cad-next-20261016T171722Z/crates`; the previous stable source
snapshot remains unchanged. New `partition_fiber_branches_network` queries the
union of original outer edges once and passes their shared original roots to
the same private multi-branch constructor used for a single face. Each face
retains all descendant sectors. Counters, including failed pair attempts, share
the existing global caps. Any unresolved requested face refuses the transaction.

The control has two adjacent source charts and two curved branches on each.
Seven original edges are queried, rather than eight face-local queries. There
are six original roots; the two roots on the common edge are each used by a
branch on both faces and retain the identity and point of a direct original
edge query. Both face processing orders pass. Every shared root has four
incident saved edges; codec reload is equal, and low-work/duplicate-face inputs
refuse without mutating source geometry.

Nine focused B-rep regressions pass in the development copy, including previous
single-face branches and explicit/automatic closed-cap controls. The new network
uses 7980 charged cells, zero exact fallback work, 792 trace evaluations and two
refused pair candidates. Result: six open faces, seventeen edges, twelve vertices.

Artifact: `output/qualification/shared-branches-network-2026-10-16/`.
Independent `scripts/verify-shared-branches-network.py` verifies saved endpoints,
opposite edge uses, shared roots and 82000 analytical samples. Maximum sampled
root error is 6.258281670667998e-10 mm. OpenCascade imports the STEP /9 as a valid
open shape with the same topology and area exactly 2 mm². STEP SHA256:
6c64961d65b67489da6dad42237b7bc43a6fb9f52a7d0029927ecbf99665dfb4.
These samples supplement native whole-curve proofs; the control is not a solid.

The reviewable integration package is `integration.patch` in that artifact
directory, with six native files and 18697 bytes. `integration-manifest.json`
records base/development hashes and the source-copy path. Patch application was
checked against the unchanged stable snapshot without applying it. This package
has not been promoted to the main native tree, active WASM or UI.

Remaining: closed/hidden branches, source-hole crossings, material-side selection
and closed-body assembly for multiple rims, general unequal-radius corners and
end transitions, whole-body diagnostics and production-part acceptance.

### Owned cap holes and a closed hollow-body height cut (isolated development)

The development copy's `cap_region` now walks every oriented boundary component
derived from selected source incidence. It retains one UV proposal per actual
owned vertex across all rings. Chord areas only propose the outer ring and
global orientation; the unchanged replacement gate must freshly prove all trim
loops, hole containment/disjointness, original edge lifts and closed-body
incidence. Disconnected multiple outer rims still refuse. Bounds remain sixteen
loops, 256 total edges, 100000 cells and 1000000 exact work.

Controls cover both cap orientations with one and two holes, preserving source
rim edge/vertex identities. A real 4×4×2 mm rectangular tube with a 2×2 mm bore
is cut at z=1 through all eight outer/inner walls. The selected upper wall
sectors and top annulus are replaced by one cap with an owned hole. All eight
shared cut roots retain points and identities; codec reload remains equal.
The operation uses 4337 cells and 24748 exact work. Ten selected regressions
pass, including prior single-rim and shared multi-branch controls.

Artifact: `output/qualification/owned-holed-cap-2026-10-16/`.
Reproduce from the development copy with `OSV_HOLED_CAP_ARTIFACT=<directory>`
and the `cap_with_holes` integration test. Independent
`scripts/verify-owned-holed-cap.py` imports the actual STEP: one valid solid,
ten faces, twenty-four edges and sixteen vertices. Volume is exactly 12 mm³
versus the original 24 mm³; new height is 1 mm. Point classification confirms
retained wall material, the empty bore and removal of the upper wall. Maximum
OCCT bounds inflation is 1.0000000028043132e-7 mm. STEP SHA256:
0de59fbf6ccb7b2cfb6c6f180c7454629fa2a1158f21e1138300f24d039f3636.

The artifact's `integration.patch` contains only the cap implementation and its
new tests (18285 bytes), with base/development hashes in
`integration-manifest.json`. Application was checked against both the stable
snapshot and the main tree without applying it. Stable Rust sources, active
WASM and UI remain unchanged. This is a sharp holed cap, not a radius/G2 fillet.
General closed/hidden SS branches, source-hole-crossing sections, disconnected
material caps, unequal-radius corners/end transitions and full diagnostics
remain unfinished.

### Interior closed-contour face partition (isolated development)

New native `face_cut::partition_closed` admits an authored closed 3D/UV contour
without inventing boundary-edge endpoints. Every consecutive and cyclic pair
must share exact 3D and UV endpoints. Source curves and supports remain original
inputs; both directed uses receive fresh whole-curve lift checks. Complete trim
audits prove the surrounding region, interior region and unique assignment of
every previous hole. The result returns surrounding face first, interior face
second, with original shell orientation and face split lineage. Original edges
and vertices retain identities; new contour vertices are shared by both uses.

Chord area only proposes orientation. Both parameter directions can be tried
under the same allowance with actual trim proofs, including two-arc contours
whose endpoint polygon has zero area. Native lens simplicity now also admits
clamped continuous multi-span arcs when one arc's EVERY interior pole is strictly
on one side of the chord and the other is in the opposite closed half-plane.
Separate injectivity and exact closure remain mandatory. A regression with
interior knot returns to the chord stays unproven; one strict pole alone never
authorizes general B-spline lens separation.

Controls cover four rational quarter arcs, two rational half arcs, both traversal
directions, a gap smaller than model tolerance, malformed curves, crossing an
actual triangular trim, disjoint old holes, a cut inside an old hole (refused),
and moving an old hole into the new interior region. Closure, naming, unchanged
source geometry and codec reload pass. Bounds remain 256 supplied segments,
100000 cells and 1000000 exact work. Twenty-six selected native checks pass:
fifteen B-rep checks, eight simplicity unit tests and three NURBS integrations.

Artifact: `output/qualification/closed-face-contour-2026-10-16/`.
Reproduce with `OSV_CLOSED_CUT_ARTIFACT=<directory>` and `face_cut_closed`.
The four-arc control uses 1598 cells, 5152 exact work and 46 trim pairs.
Independent `scripts/verify-closed-face-contour.py` checks 4100 saved-coefficient
circle samples (maximum error 1.4432899320127035e-15 mm), exact closed endpoints
and opposite uses. OpenCascade imports one valid solid, seven faces, sixteen
edges and twelve vertices. Volume remains 31.999999999999996 mm³ versus 32 mm³;
disk area is 2.0106192982787654 mm² versus π×0.8² mm². STEP SHA256:
5d6f74ac3854119131572bac7872f4efc590a68f17ae9c76987a858762bcf669.

`cumulative-integration.patch` in the artifact contains all isolated native
development since the unchanged stable snapshot: eleven files, 61447 bytes.
It supersedes the earlier shared-branches and holed-cap patches; hashes and
dependencies are recorded in `integration-manifest.json`. Application was
checked against both the stable snapshot and main without applying it. All
3366 frozen input hashes still match. Active WASM/UI promotion remains pending.

This constructs topology from a supplied contour. Automatic closed SS contour
construction, full hidden-branch census, general unequal-radius corner/end
fillets, whole-body diagnostics and production-part acceptance remain open.


## Closed original-support SS contour and curved cap

Isolated native development now constructs a closed plane/paraboloid intersection from four ordered search charts of the original surfaces. Adjacent charts use owned ports with fresh common-root proofs. Actual saved curves and both support pcurves receive fresh qualification after compaction and partition. This does not certify chart discovery or a census of all intersection components.

The control replaces the interior bottom region of a cuboid with the curved support patch. Construction uses 832 raw cubic pieces, 12 saved contour edges, 3,784 source evaluations; the resulting body uses 69,829 cells and 1,104 exact work. Independent OCCT reads a valid solid with 7 faces, 24 edges and 20 vertices. Adaptive exact-surface integration gives volume 4.392699081744112 mm3 against the analytic 4 + pi/8, and all six material probes pass.

Evidence: `output/qualification/closed-ss-curved-cap-2026-10-16/` contains the STEP, native receipt, independent check, regression receipt and cumulative integration patch. The patch passes dry-run against the current main checkout. Promotion remains pending; no WASM/UI, general automatic B-rep trimming, unequal-radius general corner, full intersection census or global embedding claim is made.


### Automatic chart discovery

`discover_fiber_surfaces` now searches the entire original surface parameter product without authored chart boxes. It tries original-cell exclusion before expanded unknown-coordinate fiber proposals, shares a bounded ledger and reserves work for fresh complete mixed-axis coverage. Incomplete discovery or exhausted verification exposes no qualified coverage. Connections and ordered trim ownership are not inferred from coverage.

Seven focused regressions pass, including a closed interior circle with no boundary-root seeds: 22 discovered fibers, 1,403 discovery cells and 41,604 total original work. A nonlinear open control uses seven fibers and 4,048 total work. Fresh coverage replay passes for both; exhausted budget refuses qualification. Evidence is in `discovery.json` and `discovery-regressions.log` beside the cumulative native patch. Next: connect discovered charts into an owned ordered loop before automatic face partition.


### Automatic original-root component connections

Discovery now proposes extended free intervals and retains them only after fresh uniform source proofs. `connect_discovered_fiber_cover` proposes bridges inside literal chart intersections, searches shifted subintervals, and contracts unknown coordinates through original root enclosures. Every retained bridge receives a fresh transition and the final component census independently replays coverage and bridges. Cross-component pairs require strict separation of original root enclosures. No world-space proximity weld is used.

Fixed endpoint queries now refine original root images with fresh source proofs to resolve coordinate orientation; budget exhaustion cannot shorten an accepted proof ledger. Four discovery/connection controls and ten existing component, transition and coverage regressions pass. Closed circle: 18 charts, 17 bridges, one parameter component; discovery 38,666 plus connection 18,930 = 57,596 original work. A distinct two-branch control remains two components with positive separation receipts. Ordered cyclic trim traversal and physical embedding remain unproved and are the next integration work.


### 2026-10-17: bounded cyclic chart and parameter proposals

Automatic graph construction retains all proved pair transitions rather than only a spanning forest. A bounded combinatorial search finds a cyclic ordering visiting each chart once; it does not claim physical loop topology. The closed control uses 18 charts and 20 proved graph edges. Complete discovery/graph work is 60,968 original rectangles; numerical junction proposals cost 176 surface evaluations. The cycle agrees with the freshly proved transition directions at every junction.

One numerical junction proposal is owned by each consecutive chart pair, and each chart receives the free-coordinate interval between its incoming and outgoing proposals. Numerical residual checks are independent control diagnostics, not root/port qualification. Failed numerical work is returned in the proposal result ledger. Fifteen construction, discovery, component and transition regressions pass, including an exhausted site budget. Evidence and the updated cumulative isolated native patch are in `output/qualification/automatic-cycle-proposal-2026-10-17/`. Next: qualify actual trimmed intervals and junction ownership, then pass them into closed face partition. No automatic arbitrary B-rep trimming, closed solid, fillet or WASM/UI claim follows from this proposal stage.


### 2026-10-17: original cyclic ports and covered subgraphs

`qualify_cyclic_parameter_ports` freshly replays source-owned components and transitions, then isolates one fixed-bridge-coordinate root per junction. Both incident charts contain each proven root enclosure. The closed control has 18 ports; maximum parameter enclosure width is 1.01474384450739308e-13. This is not a certificate for the numerical 3D endpoint or saved pcurve coefficients.

`qualify_cyclic_parameter_arcs` freshly qualifies the ordered subgraphs between incident ports and proves complete original-product coverage. It retains positive original bridge bands at the cuts: point-only cuts cannot supply a finite closed-cell cover across a change of free coordinate. The cover splitter now prefers balanced original chart boundaries over first-chart slivers, preserving original caps and tolerances.

Twenty-four focused regressions pass. The full control ledger is 38,042 discovery + 21,678 graph + 6,980 ports + 12,388 subgraph qualification = 79,088 original work, within 100,000. Exhausted port/arc allowances expose no qualified result. Evidence and the cumulative isolated native patch are in `output/qualification/cyclic-root-arcs-2026-10-17/`; dry-run application against main passes. Next: construct and freshly qualify actual saved world curves and both pcurves, bind common saved ports, then partition the face and close the body. General corners, physical embedding, WASM/UI and arbitrary B-rep trimming remain open.


### 2026-10-17: mixed-support saved contour and automatic build gate

Closed contour construction now supports all four original free coordinates, including clocks on the cutter pcurve. Mixed first/second support clocks pass saved-curve geometry and closed-incidence checks. Existing original-support, disconnected-order and low-budget all-or-nothing controls pass: four focused tests.

`partition_automatic_closed_fiber` connects original discovery, complete graph, cyclic ports, covered arcs and actual curve construction under one 100,000 cell / 4,096 numerical-evaluation allowance. No partial partition is returned on refusal; construction without a partial-work receipt conservatively reserves its entire offered allowance. Source models are unchanged.

The full automatic circle control is NOT green: it reaches construction with 20,912 cells offered, then chart 0 refuses after its allocated share (1,084 cells, 88 evaluations, 31 attempts; unresolved interval [0.470703125, 0.47314453125]). The example exits 2. Its terminal result must not be presented as a completed contour or solid. Evidence and cumulative patch: `output/qualification/automatic-closed-build-2026-10-17/`. Next: reduce repeated original proof work / improve discovery and trace allocation within existing caps, then re-run the actual automatic build. No tolerance weakening or cap increase is planned.


### 2026-10-17: proof ownership transfer and root-frame refinement

Native owned component/port admission now carries immutable qualified source proofs between phases. Only native certificate types with private proof/source fields are accepted; no serialized reports or caller-supplied positive flags are trusted. Actual port isolation, shortened arcs and complete coverage remain freshly proved. Independent `reverify` still replays inherited source proofs. Regressions compare owned and replayed source enclosures, arc domains and coverage nodes, and verify their work-accounting difference.

Closed construction narrows redundant unknown-coordinate margins using outward original root enclosures with fresh uniform proofs, preserving the entire free interval. It also refines construction root frames and redistributes soft per-chart shares inside unchanged global caps. Parent and construction-frame refinement do not change source geometry or tolerances.

Twenty-eight regressions pass. The automatic circle now reaches 622 qualified primitive cubics and 18 qualified compact world traces, using 32,748 construction cells and 3,050 numerical evaluations. Owned transfer frees 13,194 work: 34,106 cells offered to construction versus 20,912 before. The complete automatic control remains NOT green and exits 2 at `loop-interior-unproven`: only 1,358 cells remain when topology/trim admission starts. No model is published. Next: reduce remaining proof cost and/or duplicate trim-region work with source-owned proof reuse, then freshly qualify actual saved coefficients and re-run closed-body/STEP acceptance. Evidence: `output/qualification/automatic-closed-proof-transfer-2026-10-17/`. The cumulative patch passes dry-run against main; promotion and WASM/UI remain pending.

Rejected experiments: early full-cell contraction exhausted discovery work; an additional expanded-box contraction changed the chart graph and refused qualification; a different interior-witness proposal did not fix the remaining gate. Those changes were reverted.


### 2026-10-17: automatic closed contour passes native and independent STEP gates

The full automatic original-support discovery → graph → cyclic root ports → covered arcs → curves → face partition control now passes without authored chart boxes. Actual work: 99,507 of 100,000 cells, 3,226 of 4,096 numerical evaluations; 622 primitive cubic pieces compact into 18 contour edges.

Scoped native trim reuse owns the exact original curves and tolerance, caches only completed results, and reuses individual loop simplicity/winding proofs across regions. Direction reversal transfers only when control/weight reversal and bounded integer-knot reflection are mathematically exact; other knot clocks receive ordinary fresh audit. Cross-loop separation and containment remain freshly checked. Changed coefficients, weights or tolerance miss reuse; incomplete results cannot populate it. Returned report mutation cannot poison private entries.

Native world-composition certificates now transfer into partition and saved-edge admission only after literal checks of actual source surfaces, world curves and pcurves. Exact bounded integer-clock reflection is checked for reversed data. Changed coefficients or unsupported clocks fall back to full fresh verification. No untrusted flags or serialized reports authorize proof transfer.

Twenty-two focused tests pass, including full automatic model acceptance, cache mutation/clock/limit checks, ordinary bad-lift and hole-ownership regressions. Codec roundtrip, original edge/vertex identities and closure pass. Independent OCCT STEP reads one valid solid, 7 faces, 30 edges, 26 vertices; volume 3.9999999999999996 mm3 (expected 4), bound error 1e-7 mm, three material probes pass. All 18 imported cubic contour edges satisfy the analytic circle residual within 6.449428768817711e-10. This partitions the original planar face and preserves its cuboid volume; it does not replace the inner region with the curved support, create a fillet, or qualify arbitrary B-rep cases.

Evidence and updated cumulative isolated native patch: `output/qualification/automatic-closed-contour-2026-10-17/`. Dry-run application against main passes. Next: curved-region replacement under the same global ledger, independent material/volume STEP checks, then general unequal-radius corner construction and UI integration. Promotion and WASM/UI qualification remain pending.


### 2026-10-17: automatic curved-region replacement closes the body

Original root-image separation now rejects impossible graph bridges before costly transition proposals. This reduces the complete automatic partition from 99,507 to 97,603 cells while preserving its original root cover and chart traversal.

Closed contour output retains immutable native world-trace certificates. `replace_with_native_traces` and whole-model closure admit an actual use from those certificates only after literal world-curve, source surface, pcurve, orientation and tolerance checks. Reflected clocks require bounded exact integer reflection. Other uses retain ordinary exact/continuous verification; structural checks, complete trim checks and shell incidence remain mandatory. The trim stage redistributes its existing total allowance toward domain classification when native traces are supplied; public ordinary replacement keeps its prior split.

The automatic plane/paraboloid control now performs discovery → owned closed contour → replace inner planar region by curved support → closed body in 99,294 of 100,000 cells, 1,104 exact work and 3,226 of 4,096 numerical evaluations. Codec reload and common contour edge/vertex identities pass. Changed actual surface coefficients or edge poles cannot reuse the original trace; both negative controls refuse closure admission.

Forty-two focused regressions pass: automatic/body controls, original boundary identity/limits, region replacement and holes, seam correspondence, discovery and component separation. Seven unchanged external repository fixtures were copied into the isolated checkout for compiling the boundary unit tests; their hashes are recorded in the artifact.

Independent OCCT STEP reads a valid solid with 7 faces, 30 edges and 26 vertices. Volume 4.392699081827662 mm3 agrees with analytic 4 + pi/8 within 1.28937e-10 mm3. Bounds reach z=-0.5 within 1e-7 mm; all six material probes pass. This is an automatic curved cap control with a C0 rim, not a fillet, radius certificate, global embedding proof or qualification of arbitrary B-rep geometries. General unequal-radius corners, advanced diagnostics, WASM/UI and the broader roadmap remain open.

Evidence and cumulative isolated source patch: `output/qualification/automatic-curved-cap-2026-10-17/`. Patch dry-run against main passes. Source promotion and WASM/UI qualification remain pending. Next: broaden the automatic trimmed-edge/body family and connect unequal-radius transition construction, with whole-seam tangency/thickness/intersection checks.


### 2026-10-17: G2 closure from original curved NURBS rim pcurves

`corner_close::close_mapped` removes the affine-isoparametric restriction for a complete owned four-edge opening. It constructs mapped collars and a corner from original surface/pcurve pairs, requires direct original-support G2 admission and an injective regular patch, then freshly verifies actual saved rim lifts, all trim regions and whole-model seam closure. Source vertices, world curves, faces and persistent identities are retained. No temporary unsupported lid or proximity weld is introduced.

The control nonlinearly reparameterizes two original quadratic support charts, turning their owned rim pcurves into curved quadratic NURBS while preserving physical support geometry. Legacy closure demonstrably refuses this input; the new path closes the body. Native proof covers all four original seams over their complete intervals. Patch degree [5,12]; 333 cells, 980,682 coefficient work, 403,447 exact work, all inside existing caps. Native direct seam maxima are below 8e-14 mm position, 1.3e-11 normal and 3.9e-10 world curvature tensor.

Five focused tests pass, including existing closure/replacement regressions. Repeated/missing rim ownership, changed saved world curves, wrong directions and insufficient cell/coefficient work refuse; a small exact allowance may correctly use continuous admission while staying within every ledger. Codec reload and source edge/vertex identities pass. Independent STEP reads one valid solid, 30 faces, 60 edges and 32 vertices; volume 30.375 mm3 matches the analytic original graph integral. Independent derivatives at 260 seam sites give maxima 7.96856e-15 mm position, 1.88672e-14 normal, 3.95333e-13 signed world curvature tensor. These samples supplement the complete native seam proof.

This does not infer a corner opening, solve prescribed unequal radius laws, handle higher-valence openings or prove whole-body embedding. Evidence and cumulative isolated native patch: `output/qualification/mapped-corner-closure-2026-10-17/`. Patch dry-run against main passes; promotion and WASM/UI remain pending. Next: join prescribed-radius transition skins to this original-NURBS rim path and qualify complex endpoints/corners with full seam/radius/thickness checks.


### 2026-10-17: mapped G2 closure retains split-side root identities

`corner_close::close_mapped_sides` admits four logical sides containing up to sixteen saved fragments each on one original support chart. It requires complete unique rim ownership, exact shared root vertices/UV ports and a common retained natural pcurve clock. Single curves stay unchanged; split Bezier children propose a parent guide by homogeneous extrapolation from the first fragment. This guide is never authority for later fragments: every actual original world curve, original face-local pcurve and saved candidate pcurve receives independent whole-interval position/normal/signed-curvature admission through `surface_g2_pcurves`.

All source edges and vertices, including newly cut root identities, remain unchanged. Actual fragment proof errors identify side and edge. Missing pieces, changed later pcurve coefficients and incompatible natural clocks refuse without model mutation. Non-Bezier or unrelated-clock guide preparation, prescribed radius laws and higher-valence corner inference remain outside this path.

The control splits two curved NURBS rim sides at world parameters 0.37 and 0.41, including opposite use orientation and an independent face-local cut parameter. Six actual fragments qualify. Total including root partition: 23,806 cells, 981,654 coefficient work and 999,997 exact work, within the unchanged 100,000 / 2,000,000 / 1,000,000 caps. A five-fragment control and existing corner/replacement cases also pass: seven focused tests. Codec roundtrip and both original root identities pass.

Independent STEP reads a valid solid with 30 faces, 62 edges and 34 vertices; every candidate boundary edge shares the actual imported topology with its retained support. Volume 30.37499999999999 mm3 agrees with analytic 30.375. At 390 independent seam sites, maxima are 6.36929e-15 mm position, 3.16148e-14 normal and 9.31634e-13 world curvature tensor. Samples supplement complete native fragment proofs. This is a four-sided mapped corner with subdivided sides, not an arbitrary six-sided corner or a prescribed-radius fillet.

Evidence and cumulative isolated native patch: `output/qualification/mapped-side-chains-2026-10-17/`. Patch dry-run against main passes; promotion and WASM/UI remain pending. Next: prescribed-radius end-transition construction and corner networks using these retained trimmed-edge/root identities.

### 2026-10-18: edit an existing closed band with prescribed unequal radii

`radius_transition_body::replace_owned_band` replaces three owned faces (skin and radial side walls) in an existing closed quarter-sector body. It preserves both circular contact curves, every vertex, untouched source faces, and persistent identities. Only two longitudinal edges change; modified entities receive explicit lineage. Construction is atomic and never mutates the source. This is an existing-body operation, rather than another standalone smooth solid constructor.

The actual retained cylinder surfaces receive fresh original-coefficient transverse G2 inspection (`surface_collar::inspect_g2_opposite_cross`) and actual saved world curves/pcurves receive fresh position agreement. Independent generic pcurve shape boxes exhausted the 100,000-cell budget at curvature upper bound 0.000343 for a requested 0.0001. The transverse derivative proof resolves this case without changing tolerances or global budgets. Every updated saved boundary is verified against its actual surface; whole-model trim and closure checks follow. No exported construction flag grants admission.

Control source is a closed conical band between cylinders. The operation replaces its linear radius law with the requested quintic radius transition, 0.1 to 0.3 mm over 0.1 mm. Native totals: 235 cells, 108,257 coefficient work, 3,624 exact work. Reverse radii in a translated/rotated frame also pass (327 cells, 34,467 exact work). Original contacts/vertex IDs, unchanged face IDs and codec roundtrip pass. Wrong width, changed original support and exhausted work budget refuse. Seven focused tests pass across original body, owned edit and mapped corner suites.

Independent STEP is valid: one solid, 11 faces, 21 edges, 12 vertices. Volume 0.011440973240842135 mm3 differs from analytic volume by 5.186e-12 mm3. 507 independent radius samples and 50 seam samples give maxima 2.22e-16 mm radius, 8.41e-15 normal and 1.95e-13 world curvature tensor. Independent source/result material probes verify actual removal and addition, and the source conical volume agrees analytically.

Evidence: `output/qualification/owned-radius-band-2026-10-18/`; independent checker `scripts/verify-owned-radius-band.py`. Cumulative 42-file isolated native patch passes `git apply --check` against current main; it has not been promoted. WASM/worker/UI remain unqualified.

Scope is coaxial quarter bands with complete four-sided unit charts and owned contact circles; radial/end joins remain sharp. Arbitrary B-rep automatic trimming, unrelated NURBS supports, inferred multi-edge unequal-radius corner networks, whole-wall thickness/global self-intersection qualification and UI integration remain open. This phase does not complete the general roadmap.

### 2026-10-18: atomic automatic closed-contour material edit

`face_cut::trim_closed_fiber_and_replace` now composes original-support discovery, complete root-product coverage, cyclic graph/ports/arcs, actual source/cutter trim membership, saved contact generation, explicit material seed selection, replacement-chart injectivity and closed region replacement. The caller provides the two actual faces and a removal seed, without authoring contour charts or stitching the result. Every generated contact edge and root vertex retains its persistent identity through replacement. No partial body is returned on incomplete discovery, wrong removal intent or failed closure. Work without a partial receipt is conservatively charged against the offered allowance.

Material selection exposed a budget bottleneck: independently visiting all 622 original spans for a separated seed consumed 640 cells. `TrimDomain` now retains the convex hull of the original prepared span/endpoint enclosures. If that complete hull excludes the query tolerance band, a single endpoint chord has identical winding by convex homotopy. Overlapping hulls use the unchanged per-span subdivision. Joins and resource stops remain explicit. The operation's material selection now costs 36 cells; the complete operation costs 97,518 cells, 1,104 exact work and 3,226 numerical evaluations under unchanged global caps.

Nine trim-domain tests pass, including weighted curves, holes, periodic paths, join/boundary bands, work stops and comparison with forced original per-span proofs over 98 point/rectangle queries. Thirteen focused B-rep tests pass: atomic edit/refusal, automatic and authored closed contours, stale-trace rejection, split-side G2 corner/root identities and existing-body unequal-radius edits.

Independent STEP of the actual atomic result is valid: one solid, seven faces, 30 edges and 26 vertices. Volume 4.392699081827662 mm3 agrees with 4+pi/8 to 1.29e-10 mm3; bounds and all six inside/outside material probes pass. Native codec roundtrip and source immutability pass. Evidence: `output/qualification/atomic-closed-replace-2026-10-18/`. Cumulative isolated 46-file patch passes dry-run against current main; promotion, WASM and UI remain pending.

This path supports one interior closed component and a certified injective cutter chart. Exterior selection, multiple components, open SS runs across arbitrary face networks, unrelated general NURBS fillets and inferred unequal-radius corner networks remain open. Closure and local chart proofs do not establish global embedding, wall thickness, self-intersection freedom or G2. The full roadmap remains active.

### 2026-10-18: open component chart-order planning and turning-boundary refusal

A new closed source control is the box x,y in [-0.5,0.5], z in [-0.05,0.2], cut by z=y²+0.005x²-0.02. Its top face has two separate open curved branches; both continue into turning arcs on adjacent side faces. This exercises multiple branches and original shared boundary roots in a real solid rather than adjacent open sheets.

For the top face, complete original-product discovery and graph qualification prove two separated components, each containing sixteen original charts. Discovery uses 22,066 cells and the full graph uses 9,972 cells: 32,038 total under the unchanged 100,000 cap. New `QualifiedFiberComponents::open_chart_proposal` gives a bounded endpoint-constrained order visiting every chart exactly once through proved transitions. Proper end charts are selected from actual free-coordinate extrema, not the arbitrary sorted chart identifiers. Each component needs sixteen combinatorial visits. End charts from different components, repeated ends, unknown components and exhausted search refuse. This is an ordered chart proposal; boundary-root ports, parameter intervals and constructed traces still require their own proofs.

Two focused B-rep regressions and three existing root-component regressions pass. Tests inspect every proposed adjacency against actual qualified transitions, cover every component chart, retain component separation and preserve source inputs. The second B-rep test explicitly records the still-unqualified turning case: the existing single-free-axis constructor cannot strictly include endpoint roots touching a natural side-chart boundary. A wider unknown-coordinate search did not resolve this and was reverted. No tolerance, work cap or admission gate changed, and no partial model escapes.

Evidence: `output/qualification/open-boundary-atlas-2026-10-18/`, including source/cutter snapshots and `remaining-boundary-case.json`. Cumulative fifty-file isolated native patch passes dry-run against main. This phase does not construct a new closed solid, qualify general fillets, or add WASM/UI support. Next work is qualified open parameter ports tied to original boundary roots, mixed-axis trace construction through the side turning points, then shared-edge binding and closed-body qualification.

### 2026-10-18: original open root ports, boundary roots and arc census

The open atlas now has immutable native proof types `QualifiedOpenPorts`, `QualifiedOpenBoundaryPorts` and `QualifiedOpenArcs`. Internal junctions are freshly isolated at exact bridge free coordinates using original surface operators; each resulting root enclosure must lie in both incident original chart domains. Boundary roots are freshly isolated at exact natural boundaries computed from the actual original surfaces, not from a potentially partial coverage rectangle. An end chart not reaching the requested natural boundary refuses. Boundary ownership of a trimmed B-rep edge is a separate, still-required proof.

Root isolation refines through up to eight fresh original operators, keeping shared padding so exact linear coordinates do not collapse before nonlinear coordinates. No evaluation-only Newton result grants ownership. Each trimmed arc is freshly certified between its incoming and outgoing root enclosures. Coherent original transition orientation rejects backtracking. Independent `reverify` replays original graph/root/arc operations; native ownership-transfer construction avoids redundant inherited replay while retaining immutable source coefficients.

The two top-face branches each qualify fifteen internal ports, two natural-boundary roots and sixteen regular original arcs. Reversed traversal also qualifies with coherent reversed orientations. A fresh full original-product census over all thirty-two trimmed arcs proves complete coverage of both branches. Shared transaction totals: 37,724 rectangles (22,066 discovery + 9,972 graph + 1,260 internal ports + 168 boundary ports + 512 arcs + 3,746 final census), below the unchanged 100,000 cap. Independent replay work in tests is separate from this transaction receipt.

Three focused regressions pass: original complete component/port/arc qualification, reversed traversal and bounded refusals, and preservation of refusal for the still-unqualified side turning case. Wrong natural-boundary direction, repeated or foreign end charts and exhausted port/arc budgets return no certificate. Source inputs remain unchanged.

`scripts/verify-open-root-ports.py` independently solves the actual saved binary64 quadratic control coefficients with 70-digit decimal arithmetic. All four analytical endpoint roots are inside their saved four-coordinate native enclosures; maximum parameter width is 8.349e-14. This control-specific numerical cross-check supplements native whole-interval proofs and does not establish general B-rep endpoint identity.

Evidence: `output/qualification/open-root-arcs-2026-10-18/`. The cumulative isolated fifty-one-file native patch passes dry-run against main; no main native source was overwritten. The coordination snapshot `docs/qualification/sweep-shared-coordination-20261018-plasticity.json` records that the separately reported Minkowski/profile-certificate changes in main are not owned by this task.

Remaining: bind natural-boundary ports to actual original B-rep edge/root certificates, construct and qualify saved 3D/pcurve chains, handle side arcs requiring mixed free coordinates, and admit the closed resulting body. The top-face proof does not complete that side construction or general fillets. WASM/worker/UI promotion and the wider roadmap remain open.

### 2026-10-18: bind open endpoint certificates to original B-rep roots

`face_cut::bind_open_boundary_roots` binds immutable native natural-boundary ports to actual original source-edge/cutter-face roots. It requires literal equality of both original support surfaces with the actual requested B-rep faces. For each eligible face-local use, fresh exact curve-to-surface composition equality proves the original world edge is the actual source support lift. This is checked per coedge, not only per world-edge index.

The original edge root's natural parameter enclosure is transported through that use's own normalized/reversed pcurve clock with outward rational interval evaluation. A match requires its entire four-coordinate image to be included in the native surface fiber's proven unique-root domain. Overlap or proximity is insufficient. Strictly separated boxes exclude other candidates; overlapping-but-not-included or multiply owned matches refuse.

Each original world edge is queried and partitioned once, retaining complete eligible-root coverage. All adjacent coedges are updated together by the existing all-root partitioner. The returned private native transaction retains the candidate model, immutable root bindings, original source identities/ordinals and shared vertex identities; no partial result escapes on refusal. Rounded vertex/child-edge geometry retains continuous model-tolerance qualification and is not described as exact coordinates or a newly admitted fillet body.

The two open branches bind four endpoint roots to two original edges (indices 7 and 5, each with two eligible roots). Binding costs 206 cells and 96 exact work; complete discovery/graph/ports/arcs/census/binding totals 37,930 cells. Three focused regressions pass, including complete and reversed arc qualification, preservation of the unresolved side-turning refusal, shared root identity under reordered branch input, changed source surfaces, a 1e-9 mm edge perturbation, exhausted exact/geometric budgets, codec reload and fresh watertight inspection.

Independent STEP reads one valid original solid with six faces, sixteen edges and twelve vertices. Each of the four new root vertices is shared by exactly two original adjacent faces and two split edges. Volume 0.25000000000000006 mm3 differs from the original 0.25 mm3 by 5.56e-17. The separate 70-digit endpoint checker still places all four analytical roots inside the native enclosures.

Evidence: `output/qualification/open-brep-root-binding-2026-10-18/`; independent checker `scripts/verify-open-brep-root-binding.py`. The cumulative isolated fifty-two-file patch passes dry-run against main. No main native source was overwritten and no WASM/UI promotion was performed.

Remaining: construct saved 3D/pcurve chains using these exact root references and shared authored vertices, prove their full original-source error bounds, resolve mixed-parameter side turning arcs, then partition/replace retained sectors and qualify the resulting new closed body. This preparation preserves the original body; it does not yet perform the final curved material removal or general fillet construction.

### 2026-10-18: saved root-owned open curves and retained-face partition

`face_cut::construct_bound_open_traces` now constructs actual 3D cubic/NURBS pieces and both original face-local pcurves from immutable qualified open arcs. It compares original endpoint certificates with the actual owned B-rep root transaction, retains complete source/cutter model snapshots and uses the transaction's UV tolerance. Original root vertices supply endpoint world coordinates and source UVs; the actual original edge-root query supplies cutter UV proposals. Every internal junction uses literally shared world and both pcurve controls.

Endpoint edits and reversals never inherit construction authority: each actual saved piece receives a fresh whole-original-root world-error qualification. Actual source/cutter trim membership is checked through complete positive-rational hull containment or conservative region classification. Failed construction/proof without a partial receipt reserves its offered work allowance; no partial qualified chain escapes. Input branch/source mismatch and exhausted geometric/evaluation budgets refuse.

`face_cut::partition_bound_open_traces` partitions the unique descendant sector containing both owned endpoint identities. It uses the saved coefficients, freshly qualifies source-face partitioning, retains all original sectors and rechecks every original root identity after each branch. The two branches divide the top face into three sectors. All thirty-two new contact edges have two face-local uses; all original endpoint vertices remain shared. This is a retained original-body partition, not curved material removal or a newly constructed fillet body.

The control constructs thirty-two saved pieces using 2,176 cells and 512 numerical evaluations. Fresh native error upper bound is 2.5e-8 mm over every complete piece. Face partition adds 1,537 cells and 29,416 exact work. Complete discovery/graph/ports/arcs/census/root-binding/curve/partition totals are 41,643 cells and 29,512 exact work, below unchanged 100,000 / 1,000,000 / 4,096 caps. Independent proof replay in tests is supplementary to that transaction receipt.

Three focused regressions pass, including complete component/root/curve/partition qualification, literal world/pcurve junction equality, fresh saved-piece replay, reversed saved-chain construction, reordered or stale inputs, bounded refusals, original identity preservation and watertight inspection. Existing unresolved mixed-parameter side construction continues to refuse.

Independent STEP reads one valid source solid with eight faces, forty-eight edges and forty-two vertices. Volume 0.25000000000000006 mm3 preserves the original 0.25 mm3. All thirty-two cubic edges are shared by two imported faces. Every original root vertex is now shared by three edges and three faces. At 2,080 independent curve samples, maximum residual of the two original support equations is 2.806e-11 mm; samples supplement native whole-interval bounds.

Evidence: `output/qualification/bound-open-traces-2026-10-18/`, including actual saved chains, model, STEP, counters and independent report. Checker: `scripts/verify-bound-open-traces.py`. Cumulative isolated fifty-four-file patch passes dry-run against main; no main native source was overwritten. WASM/worker/UI promotion remains pending.

Next: mixed-free-coordinate side charts and their saved endpoint transitions, then select retained material sectors and replace them with the actual cutter/transition surface to admit the changed closed body. General unequal-radius corner networks, full wall/self-intersection qualification and the wider acceptance/UI roadmap remain unfinished.

### 2026-10-18: boundary-guided mixed-parameter side atlas

`discover_boundary_fiber_atlas` now follows boundary seed proposals through regular original root charts and switches between source parameters where needed. Numerical sites/tangents propose charts only. Every accepted chart gets a fresh original uniform-root proof; complete original-product coverage and a qualified transition/component graph are mandatory. `BoundOpenBoundary::boundary_seed_proposals` derives candidate coordinates from the actual owned B-rep vertices and face-local clocks, without an analytic formula for the control geometry.

For the existing side-turning control, both side faces now qualify twenty-six charts, including FirstU and FirstV charts. Each side has twenty-five freshly isolated internal ports and two natural-boundary endpoints. Continuation supplies an order proposal; `qualify_open_parameter_ports_for_order_owned` requires exact component membership, actual proved transitions and fresh root isolation. This avoids a 10,000-visit combinatorial search that exhausted its budget despite the already proved consecutive connections. Wrong or incomplete orders refuse.

Native graph ownership transfer avoids replaying an immutable complete coverage proof and retained transition proofs during the same transaction. Independent `reverify` still reconstructs actual original coverage and every transition. New transitive replay-cost accounting includes inherited proofs; borrowed port qualification refuses before replay when the caller cannot afford that cost. Legacy discovery/component tests and the private metadata-relabel refusal regression pass.

Open arc carriers retain positive bridge bands across parameter changes, as the cyclic path already did. Numerical junction/trace-clock proposals remain separate from these proof domains. `refine_mixed_fiber_coverage_owned` proves every original root fiber is covered by the new carriers through a complete closed scalar interval partition. Fresh original root enclosures must fit a target domain or an exact closed interval union of target domains sharing one free axis. It never merges a gap by tolerance. Missing turning-region targets, foreign source coefficients and exhausted work return no certificate. Each side refinement costs 552 cells over 34 inspected intervals.

Top preparation plus BOTH side discovery/graphs/ports/endpoints/carriers/refinements uses 77,712 cells and 340 numerical guide evaluations under unchanged 100,000 / 4,096 caps. The twelve focused checks comprise one combined side regression, three existing open-boundary/curve/partition regressions, four discovery regressions, three component regressions and the private integrity regression. Supplementary independent replay in tests is separate from the construction receipt.

The saved upper-curve constructor now uses port clock proposals and canonical junction proposals, with fresh original-root proof after every endpoint edit. Its existing forward/reverse and watertight partition regressions remain green. Independent STEP of the upper regression still has one valid solid, eight faces, forty-eight edges and forty-two vertices; thirty-two shared cubic edges and all original root identities remain. Volume error is 5.56e-17 mm3; 2,080 samples have maximum original-support residual 2.806e-11 mm.

Independent 70-digit analysis of the actual saved quadratic coefficients confirms all four side endpoint root enclosures and 258 original-root samples inside the qualified carrier domains. Maximum endpoint parameter width is 8.494e-14. Sampling supplements the native full-root proofs; it does not replace them.

Evidence: `output/qualification/guided-side-atlas-2026-10-18/`, including the independent side report and the upper STEP regression. Checker: `scripts/verify-guided-side-root-atlas.py`. The cumulative isolated fifty-nine-file native patch passes dry-run against main. Main native sources, WASM and UI have not been overwritten or promoted.

Remaining: reuse the common original-edge root registry when attaching side traces, qualify saved mixed-parameter side curves and their endpoint bands, partition the side sectors and replace the selected region to admit the changed closed body. This phase proves side ROOT coverage and overlapping support domains; it does not yet claim saved side curves, changed solid geometry, general fillets, G2/thickness/self-intersection qualification or completion of the wider roadmap.

## 2026-10-18: shared immutable boundary-root registry

Native isolated implementation now retains original source/cutter snapshots, original edge query reports and authored root vertices. Adjacent-face binding freshly proves the original face-local edge lift and unique-root-domain inclusion, while reusing the original query and vertex identities. A changed cutter is refused before binding. Both turning side faces reuse the exact candidate model and original top-edge vertices. Four focused tests passed; see `output/qualification/shared-boundary-registry-2026-10-18/native-tests.log`.

This qualifies root ownership on the control family only. Saved side-curve endpoint-band coverage, combined face partitions, material removal and a changed closed body remain pending. General B-rep trimming, prescribed unequal-radius corners, full G2/thickness/self-intersection qualification and WASM/UI are not established. Native sources remain isolated; cumulative patch is provided for later integration.

## 2026-10-18: saved mixed-axis side curves and complete root coverage

Both side faces now construct saved world/source/cutter pcurve chains on the common original-edge registry. Every actual piece receives a fresh continuous original-root/world agreement proof and trim-membership check. Mixed-axis endpoint bands receive fresh uniform root proofs and outward-rounded world-image bounds to the literal saved junction, with iterative unknown-coordinate contraction. Positive band widths are numerical proposals only; admission requires the world error bound. Work is charged to the same global receipt and the fitting budget is recomputed after endpoint checks.

The original full-product root census is independently refined into saved-piece root domains plus proved endpoint-band domains, including both parameter axes and the turning region. Both sides pass native replay: 90,530 total cells, 1,088 total proposal/fitting evaluations, 82 saved segments. Four focused native tests pass, including prior top/reversed controls and stale-cutter rejection. Independent Decimal sampling of 5,330 points gives maximum source-support error about 3.4151e-10 mm and checks literal chain junctions and original root-owned endpoints. Evidence: `output/qualification/guided-side-traces-2026-10-18/`. Sampling supplements the continuous native proofs.

Still pending: combined top/side face partitions, retained-region selection and replacement, changed closed-body STEP qualification, general trim configurations, end fillets, prescribed unequal-radius corners, full G2/thickness/self-intersection qualification, and WASM/UI. No changed-body or general-fillet claim. Native sources remain isolated.

## 2026-10-18: common top/side face partition network

`partition_bound_open_network` now applies distinct source-face chains atomically to one candidate, requiring literal common source/cutter snapshots and rejecting duplicate source faces. Every original bound vertex identity is checked after the complete network. No intermediate model is published. Exact free-coordinate ports need no uncertainty band: the saved clock already contains the exact port fiber. Mixed-coordinate ports retain the full fresh band/world proofs. This removes redundant work without changing any tolerance or cap.

Top and both turning side faces now partition together under the existing global budget: 96,477 cells, 1,630 numerical evaluations; network partition alone uses 5,895 cells and 119,740 exact work. Result: 10 faces, 130 edges, 122 vertices. Codec roundtrip and native continuous watertight checks pass; four focused regression tests pass. Independent OpenCascade STEP read confirms one valid solid with matching topology, volume 0.24999999999999997 mm3 (error 2.7756e-17), 114 shared cubic edges, 7,410 curve samples with max intersection residual 3.4151e-10 mm. All four original edge-root vertices have four incident edges/faces. Independent Decimal side sampling also passes. See `output/qualification/open-face-network-2026-10-18/`.

Still pending: selection/removal of the connected top/side region and actual cutter replacement (source volume is intentionally retained in this evidence), closed changed-body STEP qualification, arbitrary trimmed B-rep configurations, fillet end transitions and unequal-radius complex corners, full result qualification and WASM/UI. All native development remains isolated; cumulative patch is unapplied.

### Prepared during the runtime measurement window (unvalidated)

Isolated cap code now has an optional immutable-native-trace path: literal actual world curve/support/pcurve matches may reuse the native proof; endpoint coefficient edits fall back to fresh lift checks. Final region incidence, trim and continuous closure admission remain mandatory. This edit is not compiled or qualified. `open_network_replace_acceptance` is prepared to select one actual trim sector per source-face seed, then replace the connected region with the cutter using the remaining shared budget. Independent low-cost polygon sampling proposes sectors 7,3,9 only; native exact trim classification remains required. Evidence and deferred checks are in `cap-transfer-pending.json` and `material-selection-proposals.json`. Heavy checks remain deferred until 2026-10-18 13:40 UTC. No changed-body claim.

## 2026-10-18: atomic open-region material replacement and independent STEP

The common top/two-side network now selects actual descendant material sectors by native winding and replaces the connected region with the actual quadratic cutter surface. Native trace reuse requires literal world/support/pcurve coefficients and the matching support composition error, including exact integer-clock reversals. Changed data receives ordinary fresh admission. Owned root vertex IDs and all 114 contact-edge IDs survive the replacement.

Trim verification now uses hierarchical strict original control-hull separation for consecutive curve groups. Every tested hull pair is charged; one proved separated group excludes all of its cross pairs. The full original pair report and limits are retained. A single-loop region does not reserve cells for nonexistent cross-loop checks. The same 100,000-cell and 1,000,000-exact-work caps and tolerances remain. A 64-segment simple-loop control passes boundedly, a crossing loop and insufficient budget refuse.

Final acceptance: 46 tests pass; 96117 total cells, 1630 evaluations, 999995 exact work. Independent STEP: one valid solid, 8 faces,128 edges,122 vertices, 114 shared cubic edges, 7,410 curve samples; volume 0.1128050056670005 mm3, difference 6.3418e-12 mm3 from actual-coefficient independent high-precision quadrature. Native continuous closure and codec checks pass. Independent side sampling passes. Evidence: `output/qualification/open-region-replacement-2026-10-18/`. Initial failures are retained; final logs and qualification-status identify the passing snapshot.

This establishes the control-family automatic multi-face trim/material-replacement chain only. It does not establish arbitrary B-rep trimming, general NURBS fillets, unequal-radius complex corners/end transitions, G2/thickness/global self-intersection qualification, WASM/UI or complete roadmap acceptance. Native code remains isolated and the cumulative patch remains unapplied.

### Native tolerance ownership avoids redundant exact expansions

Mixed boundary admission now avoids polynomial exact-equality expansion for actual uses already matched literally to immutable native tolerance traces. Public exact-only verification is unchanged. Skipped exact decisions remain None, `all_equal` stays false, and actual mixed tolerance agreement still requires original support/world/pcurve matching and composition error checks. No tolerance certificate is relabeled exact. The acceptance example asserts this distinction and caps the replacement's exact work below50,000.

Control operation exact work decreased from999,995 to125,476 with unchanged96,117 cells, geometry, root/contact identities and independent STEP volume. Eight boundary-agreement regressions and the13 focused B-rep controls passed; total relevant evidence now54 tests. Fresh final export and independent STEP qualification passed. General geometry and full roadmap claims remain explicitly open.

## 2026-10-18: atomic saved-contact G2 plus prescribed radius gate

`attach_contact_g2_qualified` and its actual-source-chart variant now keep the graft private until two explicit saved longitudinal contacts pass whole-interval actual-model G2 and the prescribed rolling-ball contact-center radius law. Assembly, G2 and radius stages share one cell/exact-work budget. Original law clocks survive trimming. Source-chart proposals receive ordinary fresh admission. Wrong laws and sharp end seams refuse without publishing an intermediate body. Omitted incident edges remain visible in the scoped G2 report.

Ten focused tests pass for plane and rational curved retained supports after end-boundary fitting, existing graft regressions and circular-skin distinction. Planar saved-model codec reload receives fresh G2 and radius checks. Evidence: `output/qualification/contact-g2-radius-gate-2026-10-18/`.

The prescribed rolling-ball-center law is not a circular cross-section/whole-skin curvature claim. Actual end seams remain sharp, not G2. General NURBS edges, end blends, unequal-radius complex corners, whole-body thickness/self-intersections, independent STEP for this new admission gate, WASM/UI and the complete roadmap remain open. Native changes stay isolated; cumulative patch is unapplied.

## 2026-10-18: closed-region mapped G2 replacement with split sides

Shared mapped-side preparation now supports both the previous open-shell closure and actual closed-body region replacement. `replace_g2_mapped_sides` constructs from retained original charts while excluding selected sectors only from support ownership lookup; final full selected-region rim incidence, connectivity, trim and closure admission remain mandatory. Every actual saved fragment receives its own original-support G2 proof. Final seam indices are remapped through actual source-edge compaction. The body remains private on failure.

Eight focused regressions pass, including replacement of a closed corner with six fragments, two independent original split roots, incomplete-chain refusal, old mapped closures and prior region-replacement behavior. Both root IDs survive, codec roundtrip matches, and independent OpenCascade STEP confirms one valid solid with30 faces,62 edges,34 vertices, volume30.374999999999993 mm3 versus30.375. Native receipt:23,798 cells,981,654 coefficient work,999,997 exact work. Evidence: `output/qualification/closed-mapped-side-corner-2026-10-18/`.

This qualifies the fixed curved-chart/split-side control. It does not construct arbitrary end setbacks, prescribe unequal corner radii, establish general NURBS corners, or prove global thickness/embedding/self-intersections. WASM/UI and full roadmap acceptance remain open. Native code stays isolated; cumulative patch is unapplied.

## 2026-10-18: immutable actual G2-position ownership and bounded reuse

`surface_g2_pcurves::inspect_owned` now retains private curve/pcurve/support/orientation snapshots only after a fresh native position proof. An interval-first tolerance path avoids unnecessary exact polynomial expansion and falls back to the existing exact/interval path with all attempted cells charged. Zero tolerance retains the exact-capable original path. Public mutable reports cannot construct or modify native ownership. Actual saved uses match literal coefficients and orientation and require no tighter tolerance than the stored proof; changes fall back to fresh admission.

Mapped split-side construction transfers those native position proofs into final closed-region admission. Exact decisions for transferred tolerance uses remain None and `all_equal` remains false; topology, trims and G2 geometry stay independently checked. Tests explicitly distinguish sharp cube contacts from position agreement, reject stale geometry/orientation/tighter tolerances and forged public flags, and check zero-tolerance handling.

Twenty-six focused checks pass across native seam/trace tests, boundary exact/interval admission, new closure adapter, mapped corner/region replacement, open-root traces and saved contact G2/radius gates. Closed mapped control exact work drops from999,997 to230,674; cells23,680 and coefficient work981,654 remain bounded. Independent STEP remains valid with identical hash and volume30.374999999999993 mm3. Evidence: `output/qualification/owned-g2-position-transfer-2026-10-18/`. General NURBS end transitions, prescribed unequal corner radii, global body diagnostics and WASM/UI remain unqualified. Native sources remain isolated.

## 2026-10-18: G2 quarter-cylinder/plane end-collar surface at0.1mm

Native `end_collar::build` now constructs a rational angular/quintic meridian surface with explicit positive axial/radial setbacks. Both actual source contact curves/pcurves are inspected against the authored plane/cylinder supports and saved skin; whole-chart injectivity is independently proved by a fixed integer projection. A0.1mm axial/0.1mm radial setback control is admitted in65,705 cells and0 exact work, under unchanged1e-7/1e-5/1e-4 tolerances. Budget distribution leaves actual unused seam work available rather than reserving half the allowance for the inexpensive final chart check; no cap or tolerance is raised.

Two tests pass for canonical and translated/permuted frames, zero/parallel directions, collapsed radial setback and tiny-budget refusal. Independent OpenCascade jets at258 source/skin contact points show max position error4.44e-16mm, normal error2.29e-16, world-curvature-tensor error4.49e-16; canonical axial setback error0 and radial error2.78e-17mm. These samples supplement native continuous seam proofs. Evidence: `output/qualification/end-collar-surface-2026-10-18/`.

This is an end-surface building block only. It is not yet trimmed/grafted into a closed body, does not qualify arbitrary NURBS ends, prescribe a rolling-ball radius or solve unequal-radius multi-edge corners. Next: bind both circular contacts to original quarter-body edges through the existing contact-root/end-patch/graft pipeline and recheck the saved closed model. Native development remains isolated.

### Closed end-collar integration probe

Prepared `end_collar_body_acceptance` binds plane/cylinder contacts, builds the two end curves and attempts actual retained-face grafting into the constant-radius quarter body. The first binding correctly refuses: the contact hits an actual cutter at its natural curve endpoint and trim boundary, while `certified_curve_surface` explicitly admits strict interior Krawczyk roots only. Evidence: `output/qualification/end-collar-body-2026-10-18/native-binding-refusal.log`. No changed body is published. The contact error now identifies face, native coverage, root/unknown locations and cells.

Next construction step is a bounded expanded angular guide/cutting-plane proposal, so physical source-end roots lie inside the proposal domains, followed by original-face/native saved-body admission. No snapping, fake strict-root report, tolerance relaxation or source-body substitution is authorized. The already qualified0.1mm end-surface remains separate from this unqualified body probe.

## 2026-10-18:0.1mm G2 end collar grafted into an actual closed body

The end-collar constructor now separates unqualified proposal generation from actual admission. A bounded homogeneous angular extension places physical end roots strictly inside the guide/cutter domains; enlarged independent cutting planes are proposals only. Both circular contacts bind to original body-edge vertices, the end curves and retained sectors are assembled by the existing root/end/graft pipeline, and actual saved-body G2 is freshly inspected against the unchanged original retained plane/cylinder definitions. The final skin chart is injective, root IDs survive, full trim/closure admission and codec roundtrip pass. No strict boundary root, snapped coordinate or public flag substitutes for a proof.

Control scope: radius1mm quarter body, one end with0.1mm axial and radial setbacks; angular walls remain sharp. Native receipt67,037 cells and999,988 exact work. Eighteen relevant tests pass plus the end-body acceptance example. Independent STEP gives one valid solid,12 faces,24 edges,14 vertices with the original overall bounds. Volume0.46969303464911727mm3 differs from independent actual-meridian/source-area reference by6.693e-11mm3. At258 contact samples, max position difference1.491e-9mm, normal difference1.019e-14 and curvature-tensor difference1.442e-14. Actual axial setback error0 and radial error1.491e-9mm, below the unchanged1e-7mm model tolerance. Serialized retained support definitions match the original body exactly. Evidence: `output/qualification/end-collar-body-2026-10-18/`.

This is a control-family closed end construction, not a general NURBS end operator or prescribed rolling-ball radius. Multi-edge unequal-radius corners, other end families, global thickness/self-intersections, independent full part acceptance, WASM/UI and P0–P3 remain open. Native work stays isolated and the cumulative patch is unapplied.

## 2026-10-18: atomic native end-collar body API

The accepted chain is now `brep_core::end_collar_body::replace`, not code owned only by an example. Original plane/cylinder face roles and four retention vertices are explicit, with independent cutting-plane proposals. Root binding, end construction, role inference, retained-face graft, actual saved G2 and chart injectivity share the caller's bounded cells/exact-work limits. Returned data includes final root vertices/identities and an original-input vertex map. The cumulative ChangeSet is refreshed from the original public input, preserving source history nodes and complete naming evidence rather than exposing only the private intermediate graft. Errors carry construction-stage context and never return a partial body.

Seven focused regressions pass: canonical body, rigid permutation/translation, wrong source roles, duplicated cutting plane, tiny budget, input immutability, root IDs, codec and original history evidence, plus prior graft controls. A no-default-features/native-library check passes; codec-only tests are correctly feature-gated. The acceptance example now calls the real API and fresh independent STEP/volume/contact checks pass with the previous geometry and67,037-cell receipt. Evidence: `output/qualification/end-collar-atomic-api-2026-10-18/`.

This API remains scoped to the qualified quarter-cylinder/plane end controls; it does not infer arbitrary end geometry or establish unequal-radius multi-edge corners, global body diagnostics, actual UI history/reload scenarios or WASM/worker/UI integration. Native code stays isolated, with unapplied cumulative patch evidence.


## 2026-10-18: small unequal-radius body with original-coefficient saved-end G2

Native isolated source: `/private/tmp/osv-cad-next-20261016T171722Z/crates`.
The 0.1 -> 0.3 mm quarter-body control now accepts one lower end collar with axial setback 0.1 mm and radial setback 0.02 mm. Retained-boundary census first uses a strictly positive certified global curve-distance lower bound; unresolved/intersecting cases still require the original exact census. A new coupled proof composes both original surface/pcurve pairs in independent bounded transverse frames and compares their original coefficient jets. Neither stored surface is replaced by a rounded reference. Actual owned-curve position checks remain mandatory.

The saved small cylinder seam qualifies in 7 cells and 113408 coefficient operations (including frame orientation); original API caps remain 100000 cells, 1000000 exact operations and 2000000 coefficient operations. Atomic closed-body receipt: 1466 cells, 999993 exact operations, 113408 coefficient operations. Original retained definitions, bound root identities and codec restoration remain checked. The fixed cylinder frame enters both original charts; this is a qualified chart class, not arbitrary chart-frame discovery.

Independent OCCT STEP: one valid solid, 12 faces, 24 edges, 14 vertices; volume 0.022851527897001612 mm3 versus meridian reference 0.02285152788614951 mm3 (error 1.08521e-11 mm3). Source middle-row axial control discrepancy from the rotational reference is explicitly measured at 1.38778e-17 mm. Saved contacts: 258 independent samples, maximum position error 1.19210e-9 mm, normal error 1.01849e-14, curvature tensor error 1.44126e-13. Sampling is separate from the native whole-seam proof. Canonical R1 evidence also passes the updated independent verifier.

Validation: 4 coupled-jet unit tests, 2 boundary-census tests (including hole crossing/budget refusal), 8 closed-body/contact tests and 10 existing collar/radius tests pass. Evidence and an unapplied cumulative native patch: `output/qualification/end-collar-small-radii-2026-10-18/`; apply-check passes. Native promotion, WASM/worker/UI, arbitrary trimmed B-rep, general NURBS edge fillets, complex unequal-radius corners, full-body thickness and self-intersection qualification remain pending. Sharp angular walls remain in this control.


## 2026-10-18: atomic two-end closure on an unequal-radius body

`end_collar_body::replace_pair` now performs two native end edits privately. Both input role sets refer to the original public body; retention vertices are mapped through the first edit. Cylinder transverse direction is proposed from the requested end axis and original support rail, then freshly checked from both original coefficient charts. No sampled frame choice grants admission.

After the second edit, all four saved contacts are found through persistent edge/skin identities and freshly inspected on the final model. Both final skin charts are rechecked for injectivity. All admitted end roots must retain their identities and literal saved positions. The returned change set is refreshed relative to the public source; codec equality is tested. A failed second edit, removed original retention vertex, repeated incompatible end, missing root/contact identity, or failed final audit returns no partial body.

Control: source radius band 0.1 -> 0.3 mm, lower radial/axial setbacks 0.02/0.1 mm, upper setbacks 0.04/0.1 mm. Each edit preserves its unchanged per-edit caps (100000 cells, 1000000 exact operations, 2000000 coefficient operations); the atomic pair explicitly reports both edit receipts rather than calling their sum a single-edit budget. First edit: 1466 cells / 999993 exact / 113408 coefficients. Second: 1507 / 999995 / 95081. A separately bounded final all-contact audit: 219 / 0 / 208489.

Independent OCCT STEP admits one valid solid with 13 faces, 27 edges and 16 vertices. Volume 0.022667021669229785 mm3; meridian reference 0.02266702165093868 mm3; error 1.82911e-11 mm3. 516 independent contact samples: maximum position error 1.19210e-9 mm, normal error 1.04969e-14, world curvature tensor error 1.44126e-13. Original one-end R1 and small-radius evidence also passes the generalized independent verifier. These samples supplement, rather than replace, the native whole-seam proof.

Validation: 4 end API tests (including second-edit refusal and repeated-end refusal) and 5 contact-graft regressions pass; no-default-features library check passes. Evidence: `output/qualification/end-collar-pair-2026-10-18/`. The cumulative native patch applies cleanly in check mode and remains unapplied; native promotion and WASM/UI remain pending. This control keeps sharp angular walls and does not qualify arbitrary B-rep trimming, general NURBS fillets, complex unequal-radius corners, whole-body thickness or global self-intersections.


## 2026-10-18: complete original NURBS retained-boundary exclusion

The retained-region census now admits continuous clamped nonperiodic original B-spline boundaries and queries of degree 1..8 with at most 256 controls. A strictly positive lower distance bound must cover the complete original span product, including internal knot spans; rounded Bezier extraction and a sampled near-contact do not grant admission. Initial span products must fit the unchanged 64-cell pair allowance. Original control-hull separation remains valid for positive rational weights. Complete region simplicity/hole separation is audited by the caller before this census, and an interval winding witness selects the complement component only after every complete boundary is excluded.

Unsupported contact is still explicit: unresolved multi-span or degree>3 original boundary contacts return `original-spline-contact-unproven`; exact root fallback remains unchanged for its qualified Bezier classes. Continuity/clamping/control-count bounds are required; disconnected or periodic queries cannot inherit admission from one interior witness. No root is identified by proximity or by this inclusion proof.

Three focused native tests pass: authored nonconic rational quintic profile extruded into a validated watertight body with actual original UV census; rational quintic/elevated multi-span boundary plus multi-span query controls; hole crossing and work exhaustion. Crossing, disconnected query and low-budget controls refuse. Nine contact-graft/end API regressions pass, and the no-default-features library check passes. Evidence and unapplied cumulative patch: `output/qualification/nurbs-boundary-census-2026-10-18/`; apply-check passes.

This extends retained-region membership only. General trim-interior root charts, arbitrary B-rep trimming, common NURBS fillet construction, complex unequal-radius corners, full diagnostics and native/WASM/UI promotion remain pending. The current open-boundary atlas still requires source natural-boundary root ports; its interior-trim limitation is not closed by this change.


## 2026-10-18: immutable original-root lifts on interior incident trims

`BoundOpenBoundary::original_face_root_lifts` now transfers the original registered curve/surface roots to another original incident face without requiring a natural-boundary coordinate. Every actual original coedge/support lift is freshly checked for exact equality. The complete original root clock box is mapped through that coedge with outward interval arithmetic; no coordinate is overwritten by a guessed trim boundary. Source edge/vertex identities, root ordinal and original face-local use ordinal remain explicit. Original queries are shared immutably, rather than repeated on rounded child edges. `OriginalFaceRootLifts` owns immutable original/cutter/authored-model snapshots; its factory is native and private, and source matching rejects changed definitions. Report entries are read-only through the owned result. Candidate vertex coordinates remain continuously qualified; they are not relabeled as exact root coordinates.

Native control uses the existing two original open components and the same physical cuboid side faces, with their surface charts expanded exactly so the top trim has V=0.75 inside the chart. Four roots lift onto two side faces; both UV coordinates of every root box lie strictly in (0,1). Each side consumes 2 cells and 620 exact operations. The root vertex identities and original cutter root boxes are retained, and the root-partitioned expanded-chart model remains watertight within its original tolerance. Changed-source matching, insufficient cells and insufficient exact work refuse. Limits remain 100000 cells / 1000000 exact work.

Validation: all 3 open-boundary atlas tests, 1 region-network test and 4 region-replacement tests pass; no-default-features library check passes. Evidence and unapplied cumulative patch: `output/qualification/interior-trim-root-lifts-2026-10-18/`; apply-check passes.

Scope: this proves original edge-root transport to original interior-trim face coordinates. The existing complete surface-intersection atlas still needs natural-boundary ports. Constructing/qualifying its branches with an interior trim curve, arbitrary B-rep trimming, common NURBS fillets, complex unequal-radius corners, full diagnostics and WASM/UI promotion remain pending. No independent STEP or complete arbitrary-trim claim is added by this root-transport control.


## 2026-10-18: complete original atlas seeded from interior trim-root images

`ss_intersection::discover_seeded_fiber_atlas` now accepts interior first-surface seed proposals and walks both directions. The old boundary-only entry point keeps its natural-boundary contract. Initial free-coordinate widths can be reduced in at most eight fresh original-operator attempts; all offered work is charged against the same proposal quota. A common initial fiber is shared between directional walks instead of inserted twice. Every chart still needs its original uniform-root proof, followed by complete original surface-product root coverage and a fresh transition graph. Seeds and numerical continuation grant no root identity.

`OriginalFaceRootLifts::seed_proposals` supplies numerical midpoint proposals from privately owned original root images. On the two expanded-chart side controls, one interior registered root proposal yields a whole-support atlas on each side: face 3 uses 10676 rectangles / 120 evaluations / 21 charts; face 5 uses 10870 / 120 / 21. The two directional orders join through the common initial chart and each passes the native whole-component ordered-port qualifier in another 680 rectangles. Both component certificates reverify on the actual original surface definitions. Limits stay 100000 rectangles, 4096 guide evaluations and 128 charts.

Negative control: proposing only one of two original top-face components walks 21 charts but is rejected by the mandatory whole-product census (3210 rectangles, including 3126 coverage rectangles). Low-work controls publish no certificate; the legacy boundary entry point still rejects interior seed coordinates.

Validation: 1 guided-boundary test, all 3 open-boundary tests, 1 region-network test and 4 region-replacement tests pass; no-default-features library check passes. Evidence and unapplied cumulative native patch: `output/qualification/interior-seeded-atlas-2026-10-18/`; apply-check passes.

This proves the original whole-support branch atlas from interior trim-root proposals. It does not yet clip branch endpoints to the actual original trim-root boxes, author those clipped curves/pcurves into a replacement B-rep, or qualify arbitrary trimmed bodies. General NURBS fillets, complex unequal-radius corners, full diagnostics and WASM/UI promotion remain pending. No independent STEP or closed-body claim is added for this atlas-only control.


## 2026-10-18: original trim-root endpoints bound to ordered branch carriers

`face_cut::bind_original_root_segment` now binds two immutable original incident-face root lifts to a native qualified ordered branch. Every carrier must literally match the original source/cutter definitions. The full original 4D edge-root image must fit a uniform-root carrier domain, and its free-coordinate interval must lie strictly between the complete incoming/outgoing native port enclosures. Midpoints, trace-clock proposals and proximity do not grant endpoint identity. Ambiguous ownership, a junction root needing refinement, overlapping same-carrier endpoint clocks, duplicate root vertices and exhausted discrete checks return no certificate. Requested reverse endpoint order is retained explicitly. The result privately owns both the original root-query token and branch certificate; candidate vertex coordinates are not relabeled exact.

The interior-seeded atlas now removes same-axis proposal charts fully contained in another original chart before the mandatory whole-product census and fresh graph. This resolves redundant initial/end carriers that previously made a natural boundary port coincide with an internal port and correctly blocked arc admission. No proof cap is raised and a missed component still fails complete coverage.

Control: both expanded-chart side branches now have 20 nonredundant carriers. Fresh whole-support atlas, ordered ports, natural support-end roots and native parameter arcs consume 11088 rectangles on face 3 and 11464 on face 5. Each original-root segment binding consumes 60 discrete source/inclusion checks; endpoints bind to carrier ordinals [0,19] and [19,0] respectively. Reversed requested endpoints invert the segment orientation. Wrong-face root tokens, repeated endpoints and one-check budgets refuse. The omitted-top-branch control still fails the complete census (3068 rectangles, including 2984 coverage rectangles).

Validation: 1 guided-boundary, all 3 open-boundary, 1 region-network and 4 region-replacement tests pass; no-default-features library check passes. Evidence and unapplied cumulative native patch: `output/qualification/original-root-bound-segment-2026-10-18/`; apply-check passes.

Scope: this binds endpoint identity and order on the original branch. It does not yet author the root-clipped saved 3D/UV curves, prove every intervening point belongs to the actual trimmed face, or assemble a closed replacement body. Those gates, general NURBS fillets, complex unequal-radius corners, diagnostics and WASM/UI promotion remain pending. No STEP or whole-body qualification is claimed for this binding-only control.


## 2026-10-18: root-clipped original-family covers and stored 3D/UV pieces

`BoundOriginalRootSegment::cover` now creates original uniform-root carriers between the bound endpoint boxes. Incoming/outgoing boxes are the actual original root images and native junction enclosures, not midpoint clocks. Each complete box must fit its original carrier. The retained free interval spans the full endpoint/junction uncertainty; every shortened interval is freshly certified against its original surface pair. Reverse endpoint order reverses carrier order, traversal signs and enclosure order. Invalid/separated-clock and exhausted-work paths return no partial cover.

`OriginalRootSegmentCover::construct_traces` freshly constructs and qualifies stored world curves plus both original pcurves for every interval, atomically across all carriers. It shares the offered cell/evaluation/piece budgets, with unchanged caps of 100000 cells, 4096 evaluations and 1024 total pieces (at most 256 per carrier). World tolerance derives from the original model tolerance divided by four. The output retains original-source world-error certificates; no concatenated B-rep edge or endpoint welding flag is inferred from the pieces.

On each expanded-chart side control: 20 fresh interval carriers require 140 cells; 20 stored 3D/UV pieces require 460 construction cells and 200 evaluations. Every stored piece reverifies from original sources, with native world-error upper bound at most 3.125e-9 mm. Low construction and cover budgets refuse. Reverse covers retain the same original domains with reversed endpoint order.

Independent OCCT conversion of the actual stored coefficients: 1320 sampled parameter sites across 40 pieces (both support compositions at each site) pass. Maximum support deviation is 7.85537e-16 mm; maximum sampled inter-piece junction gap is 5.02675e-16 mm; maximum candidate endpoint deviation is 4.25277e-14 mm. These finite samples supplement the native whole-interval world proofs, and do not authorize welding or whole-face membership. `scripts/verify-root-segment-traces.py` checks saved sources against source/cutter snapshots and explicitly requires welding/membership flags to remain false.

Validation: 1 guided-boundary, all 3 open-boundary, 1 region-network and 4 region-replacement tests pass; no-default-features library check passes. Evidence, stored source/curve coefficients, independent report and unapplied cumulative patch: `output/qualification/root-segment-traces-2026-10-18/`; apply-check passes.

Remaining: canonical shared endpoint authoring and fresh post-authoring source/world-band proofs, full original trimmed-face membership of the stored pieces, insertion into a closed replacement body, general NURBS fillets and complex unequal-radius corners. Global diagnostics, STEP body acceptance and WASM/UI promotion are not closed by this curve-piece control.


## 2026-10-18: canonical 3D/UV root-segment chains with original trim admission

`RootSegmentTracePieces::weld` now privately authors one canonical world point and both source-local UV points at each junction. Original endpoint world positions and first-face UV values come from the actual root-partitioned model; cutter UV endpoint proposals reuse the immutable original root-query boxes. Every following stored piece literally copies the preceding world/pcurve endpoint. Each actual edited clock interval is freshly certified on the original surface pair, and every edited world curve plus both pcurves receives a fresh original-root world-error proof. No inherited pre-edit fit receipt admits modified coefficients.

Complete uncertain endpoint/junction clock bands are separately certified and bounded in original world space around the actual shared point. All resulting saved pcurves must lie in both ORIGINAL trimmed faces, using the UV tolerance stored by the original root registry; the weld caller cannot substitute a looser tolerance. The world model tolerance stays 1e-7 mm. Construction reserves a quarter of it, and post-authoring piece/band proofs use the existing half-model-tolerance allowance (5e-8 mm); this does not change the model tolerance. Failure publishes no partial chain. Native source/band carriers are retained for later root-product refinement.

Control on faces 3 and 5: 20 welded stored pieces per face; 1620 fresh proof cells and 19 junction proposal evaluations per face. The maximum full-band/piece world upper bound is 2.981176130956166e-8 mm. Forward and reverse traversal pass; exact endpoint world positions, literal 3D/UV junctions, source-token provenance, and low cell/evaluation refusal are tested.

Independent checker welded mode: all 3D and both UV junction control points compare literally equal, and chain endpoints literally equal the original candidate root-vertex positions. For these authored rectangular trimmed faces, every positive-rational saved pcurve control hull lies inside the actual rectangle; this whole-curve hull check is separate from the 1320 OCCT support sample sites. Maximum sampled support deviation is 1.02144e-14 mm; measured junction gaps and endpoint deviations are zero. Native whole-interval root/world and band certificates remain the authority for source correspondence.

Validation: 1 guided-boundary, all 3 open-boundary, 1 region-network and 4 region-replacement tests pass, including reverse traversal; no-default-features library check passes. The independent checker also preserves raw-piece mode and its previous control still passes. Evidence, actual welded source/curve coefficients, independent report and unapplied cumulative native patch: `output/qualification/welded-root-segment-2026-10-18/`; apply-check passes.

Remaining: native completeness with respect to all original face trims, partition/insertion of these interior-trim chains into the B-rep network, closed-body admission and independent STEP acceptance. This is a welded saved-chain and membership control, not arbitrary B-rep or general NURBS fillet completion. Complex unequal-radius corners, full diagnostics and WASM/UI promotion remain pending.


## 2026-10-18: interior-trim chain insertion and closed selected-region replacement

`face_cut::qualify_welded_root_traces` transfers private immutable welded source/world/trim proofs into the existing root-owned face-partition input. Inputs must share one exact source/root registry, face, cutter face and UV tolerance; endpoint vertex identities must match the registry. Empty groups, duplicate unordered endpoint pairs, mixed original faces and >1024 saved pieces refuse. Actual B-rep insertion uses the existing fresh sector, boundary and naming gates; the adapter does not assert omitted-component exclusion.

Both expanded-chart incident-face controls partition into two sectors using the interior-trim chain: face 3 costs 503 cells / 20028 exact operations, face 5 costs 491 / 22512. Their original root IDs remain unchanged. The original top chains and the two welded side chains share one immutable registry and form a three-face partition network: 1979 cells / 72172 exact operations, one closed source body with 10 faces / 88 edges / 80 vertices. Selected connected sectors are replaced with the actual original cutter surface in 1822 additional cells / 8678 exact operations. All four original root IDs survive final vertex remapping; persistent naming and exact model codec restoration pass. The result is one closed control body with 8 faces / 86 edges / 80 vertices.

Independent STEP import validates both bodies. Partition preserves volume 0.25 mm3. Changed body volume is 0.11280500566875921 mm3 versus independent actual-coefficient quadrature 0.11280500567334231 mm3, error 4.58310e-12 mm3. All 72 saved cubic intersection edges have two incident faces. 4680 STEP curve sample sites have maximum intersection residual 2.80544e-11 mm; each of the four original root points is one shared STEP vertex with three incident edges and three faces. Native ID preservation and independent geometric sharing are separate checks.

The volume-reference checker now validates the actual six affine face trims and eight physical source vertices, allowing support charts to extend beyond the physical box. Positive-weight rectangular UV trims, physical corner mapping and affine coefficient checks precede the existing high-precision quadratic-control integral. Its original canonical-chart control and existing STEP verifier also pass unchanged volume/topology expectations. This reference uses convergence evidence, not a general interval volume certificate.

Validation: 1 guided-boundary, all 3 open-boundary, 1 region-network and 4 region-replacement tests pass; an extra focused naming/codec check passes; no-default-features library check passes. Evidence, original/candidate/final model coefficients, partition/body STEP, independent reports and unapplied cumulative native patch: `output/qualification/interior-chain-brep-2026-10-18/`; apply-check passes. Independent verifier: `scripts/verify-interior-chain-brep.py`.

Scope remains a connected selected-region control on planar source faces with an actual quadratic cutter and interior UV trims. General original-trim completeness, arbitrary B-rep trimming, general NURBS fillet surfaces, complex unequal-radius corners, global thickness/self-intersection qualification, three-part application acceptance and WASM/UI promotion remain pending.


## 2026-10-18: complete geometric root coverage inside actual face trims

`face_cut::qualify_trimmed_root_coverage` now combines immutable saved-chain root carriers with freshly proved outside-trim carriers and refines the complete original surface-product root census against their union. The parent must be a complete native census on the literal original surface pair. Chains must share exact original/authored source snapshots, face/cutter roles and original UV tolerance. Empty parent families, mismatched sources, excessive targets and exhausted proof work do not publish a coverage certificate.

Outside witnesses use a freshly contracted original root image projected into the actual original source/cutter trim domains; at least one whole projected box must classify Outside. No sampled point or guessed absence excludes a family. Exact saved target clock boundaries guide interval subdivision. Every outside witness retains its original native fiber and projected image and can replay both source proof and trim classification. Final admission still requires the native complete-parent refinement, rather than only the preparatory exclusion scan.

Near an original endpoint, additional source-root families are admitted only after a fresh world-distance proof around the actual saved endpoint vertex. The current model tolerance remains 1e-7 mm and the existing half allowance is retained; the maximum new endpoint-family bound on the controls is 4.850295545755225e-8 mm. These are bounded root families associated with an already owned original endpoint, not a proximity-based root identity or relaxed trim membership claim.

Controls: face 3 costs 10404 cells / 292 intervals; face 5 costs 9829 / 294. Each has 23 certified outside-family witnesses. Both full source refinements and outside witness replays pass. Wrong-surface census and insufficient-work controls refuse. Unresolved results now retain the original parent index, 4D parameter domain and available original-root image for future error highlighting; successful results clear these fields. Limits remain 100000 cells, 32768 total preparation/refinement intervals, depth 28 and 1024 target fibers.

Validation: 1 guided-boundary, all 3 open-boundary, 1 region-network and 4 region-replacement tests pass, including the existing closed interior-chain control; focused unresolved-location assertions also pass; no-default-features library check passes. Evidence and unapplied cumulative native patch: `output/qualification/trim-root-coverage-2026-10-18/`; apply-check passes.

Scope is geometric root-family coverage within the original world tolerance on the two control trims. A complete topology census of every original trim-boundary root remains required, as do broader NURBS/complex-corner qualification, global diagnostics, application part acceptance and native/WASM/UI promotion. This geometric certificate alone does not close arbitrary B-rep trimming or authorize omission of unowned topological roots.


## 2026-10-18: original boundary-root registry and complete-input gate

Native changes remain isolated in `/private/tmp/osv-cad-next-20261016T171722Z/crates`; the frozen reference and shared native sources were not modified. Evidence: `output/qualification/boundary-root-registry-2026-10-18/`. The cumulative patch is unapplied.

- Preparation queries original 3D edges once, retains their root ordinals and shared vertex identities, and projects them through exact original face lifts without requiring natural chart boundary ports. The selected side control uses 8 distinct edges and 4 roots.
- A complete boundary census audits every original coedge and refuses an unregistered eligible root. The complete-input gate requires matching immutable snapshots, geometric trim coverage, and each eligible boundary root used exactly once.
- A zero-root boundary registry is deliberately insufficient to exclude an interior intersection. An interval-sign regression establishes a crossing strictly inside the face. Periodic repeated uses, cutter-boundary roots, unresolved roots and tangencies remain unsupported by this census.
- Ten focused native regressions passed; the library without default features compiled. Independent STEP checking confirms a valid single replacement solid (8 faces, 86 edges, 80 vertices), original shared root vertices, and volume error 4.583097790167301e-12 mm3. Sampling 72 shared cubic edges at 4680 sites gives maximum residual 2.8054308875979928e-11 mm; sampling is not a global interval certificate.

This qualifies the documented plane/quadratic control region replacement. General automatic B-rep trimming, arbitrary NURBS fillets, end transitions and complex unequal-radius corners remain incomplete. No WASM/UI or full part acceptance claim is made.


## 2026-10-18: final original-edge fragment correspondence

The isolated native boundary registry now retains every original edge's final children in increasing original curve parameter order (`original_edges`, `edge_fragments`). After all splits, preparation checks each child's endpoints against original endpoints and ordered shared root vertices, and verifies every original wire use and reversal survives in the cumulative model. A failure publishes no registry. This is topology correspondence, not material-side classification or a general fillet certificate.

The existing interior-trim body control checks shared child vertices and both adjacent uses. The zero-root control checks unchanged original edges. All 10 focused tests passed and the no-default-features library check passed. Fresh independent STEP evidence again confirms a single valid solid and volume error 4.583097790167301e-12 mm3. Evidence and an apply-checked, unapplied cumulative patch are in `output/qualification/boundary-edge-fragments-2026-10-18/`. Shared native sources and WASM were not promoted. General trimming, tangent/cutter-boundary ownership and unequal-radius corner assembly remain incomplete.


## 2026-10-18: retained-sector planning from the original root registry

`OriginalBoundaryRootRegistry::plan_retention` now resolves `RootedFace` endpoints from the immutable original-edge/root-ordinal ledger and feeds the authored shared vertices to the existing projected retention planner. It refuses unprepared faces, roots on nonincident edges, unavailable root ordinals and exhausted shared preparation/planning budgets. Original vertex requests use original indices. Preparation cell/exact work is included in the returned totals, and lineage is refreshed against the original model. No rounded child is re-queried.

The planar cut control now exercises both the old freshly rooted path and the prepared-registry path: one retained and one discarded sector, watertight partition, persistent naming, invalid-root refusal and shared-budget refusal. Source/cutter remain unchanged. Ten focused regressions passed; no-default-features library compilation passed. Evidence and unapplied apply-checked cumulative patch: `output/qualification/registry-retention-2026-10-18/`.

Retention still uses explicit UV keep seeds. Curve generation, cutter-support agreement, full automatic material classification and closed replacement admission remain separate requirements. This API does not qualify general NURBS fillets or complex unequal-radius corners and is not promoted to shared native sources/WASM/UI.


## 2026-10-18: strict cutter-support gate before retention

`OriginalBoundaryRootRegistry::plan_cutter_retention` freshly projects each complete supplied cut onto the immutable cutter support using the existing continuous curve/surface agreement checker with half the original model tolerance. It requires a qualified pcurve and classifies its full positive rational control hull strictly Inside the original cutter trim. An ambiguous or boundary-crossing hull refuses conservatively. It then invokes prepared-root retention with the remaining shared cell and domain budgets; preparation and proof costs are included in the totals.

Regression: a quadratic bowed curve has the same correct boundary-root endpoints and lies wholly on the original planar source support. The ordinary source-support retention path admits it, but the strict cutter path refuses because its interior leaves the cutter plane. A straight cut passes the strict path and gives the same retained/discarded sectors. Ten focused native tests and no-default-features compilation pass. Evidence and apply-checked unapplied cumulative patch: `output/qualification/cutter-retention-2026-10-18/`.

No automatic material selection is claimed: explicit keep UV seeds are still required. Cutter trim control-hull acceptance is conservative and does not cover all valid boundary-touching or curved-trim cases. General NURBS corner/fillet construction, closed replacement admission, WASM/UI and full part acceptance remain open. Native sources remain isolated.


## 2026-10-18: adaptive original-pcurve cutter trim proof

Strict registry retention now checks the qualified original cutter pcurve with outward-rounded interval evaluation on closed parameter intervals. Every accepted interval must classify strictly Inside the original trimmed cutter domain; ambiguous boxes subdivide without changing poles, weights, knots or lift certificates. Outside, depth 28, 32768 intervals, exhausted shared domain work or unsplittable clocks refuse. No midpoint sampling establishes membership.

The hole control distinguishes a valid quadratic detour whose overall control hull overlaps the hole from a genuine crossing and boundary touching. Rational weights `[1,2,1]`, parameter domain `[-3,5]`, reversed traversal and exhausted-budget refusal are checked. Ten focused integration regressions and the new native proof regression pass (11 tests); no-default-features compilation passes. Evidence: `output/qualification/adaptive-cutter-trim-2026-10-18/`, including source hashes and the apply-checked unapplied cumulative patch.

This improves conservative curve-trim qualification; it does not automatically classify material, admit boundary-touching cuts, close arbitrary fillets/corners, or establish WASM/UI delivery. Shared native sources remain unmodified.


## 2026-10-18: whole-face halfspace classification and sector reselection

`classify_face_halfspace` now bounds `normal dot surface + offset` over the complete actual trimmed face with outward interval arithmetic. Only proven Outside UV boxes are omitted. An existing exact rectangular closure narrows the starting domain when available; general trims start at the whole original chart. Original model tolerance is scaled conservatively by an outward lower bound on the original normal length, without replacing the plane by a rounded normalized plane. Unknown boxes, depth/work exhaustion and uncertain mixed-side bounds remain Unresolved. Positive/Negative permit only the original model tolerance across the plane; ToleranceBand is separately reported. Straddles requires certified opposite Inside boxes.

`RetentionNetwork::select_halfspace` atomically reselects all descendants against a supplied plane, includes previous planning costs in the shared totals, and refuses unresolved, straddling or tolerance-band sectors. It preserves the full partition model and only changes retention lists. It does not assert that the supplied plane is the original cutter, classify uncut faces, delete sectors or admit a closed replacement.

The plane-cut control confirms opposite sides, swapped retention under reversed/scaled normal, unchanged topology, tolerance-band refusal, a nonaxis normal and invalid zero-normal refusal. The unpartitioned crossing face cannot be selected as one side. Ten focused regressions and no-default-features compilation pass. Evidence and apply-checked unapplied cumulative patch: `output/qualification/halfspace-selection-2026-10-18/`. Arbitrary curved-cutter material classification, general NURBS fillets/unequal-radius corners and WASM/UI remain open; native changes remain isolated.


## 2026-10-18: original-root partition without retention seeds

`RootedPartitionFace` accepts original face/cut requests without keep UVs. `OriginalBoundaryRootRegistry::partition_halfspace` resolves the already prepared original root ordinals, freshly qualifies complete cut curves on the immutable trimmed cutter, partitions every selected original face, then applies whole-sector halfspace classification. Endpoint resolution and cutter-curve qualification are shared with the existing seeded paths. Preparation, cutter agreement, domain classification, partitions and side selection share the supplied totals; duplicate faces, bowed off-cutter curves and exhausted work refuse atomically. The caller supplies the mathematical selection plane; this API does not assert that it equals the cutter support or generate missing contours.

The seedless plane-cut control chooses the same sides as the old seeded control, retains persistent naming and watertight incidence, and preserves the full partition until replacement. Ten focused integration regressions and the adaptive trim proof regression pass; no-default-features compilation passes. The final export regression also passes.

`scripts/verify-seedless-partition.py` independently reads the actual STEP with OCP: one valid solid, 7 faces, 15 edges, 10 vertices, each edge shared by two faces, volume 512.0000000000001 mm3 against 512, and two top sectors each of area 32 mm2 with Y ranges [0,4] and [4,8]. Evidence and an apply-checked unapplied cumulative native patch: `output/qualification/seedless-partition-2026-10-18/`. This is a control partition and selection, not removed material, a newly capped body, arbitrary curved-cutter classification, general NURBS fillets, unequal-radius corners, or WASM/UI delivery. Native changes remain isolated.


## 2026-10-18: seedless original-root partition through closed cap admission

`RetentionNetwork::cap_discarded` now validates complete disjoint retained/discarded descendant lists, rejects overlapping/duplicate uncut removals and passes the selected connected region to fresh existing `cap_region` admission. Planning and replacement cell/exact budgets are cumulative. Closed incidence, original rim identities, continuous boundary lifts, surface injectivity and fresh trim audit remain mandatory; no partial body is returned. Uncut removal faces remain explicit.

The new `seedless_cap` control generates cuts on four cube faces, uses one original-root registry, partitions/selects without keep UVs, removes the negative-Y side and caps on the original cutter support. Original root identities and literal vertex positions survive through replacement mappings; model codec roundtrip passes. Duplicate and partition-overlapping uncut removals refuse. Eleven focused integration tests and no-default-features compilation pass.

The independent OCP verifier (`scripts/verify-seedless-partition.py ... --cap`) reads the actual final STEP: one valid solid, 6 faces, 12 edges, 8 vertices, every edge shared twice, dimensions 8 x 4 x 8 mm, cap area 64 mm2 and volume 256.00000000000006 mm3 against 256. The previous unremoved partition mode also still passes. Evidence and apply-checked unapplied cumulative patch: `output/qualification/seedless-cap-2026-10-18/`.

This closes the documented planar control workflow. General automatic contour generation, curved-cutter material classification, NURBS fillets, end transitions, unequal-radius complex corners, full diagnostics and WASM/UI delivery remain open. Native changes remain isolated from the shared checkout.


## 2026-10-18: whole-face sidedness against original curved coordinate graphs

`classify_face_graph` classifies complete source trimmed faces above/below an original cutter represented as a polynomial Bezier coordinate graph. It requires a bounded clamped patch, equal weights and exact arithmetic-progression base coordinates along independent U/V axes. Error-free TwoSum differences reject rounded coordinate maps; inverse base-coordinate arithmetic and both surface evaluations use outward intervals. Every projected parameter box must remain strictly Inside the actual original cutter trim. The height residual permits only the source model tolerance along the chosen height axis. General rational weights, ambiguous projections, out-of-chart projections and exhausted/depth-limited proofs never select a side.

The quadratic original-support control checks all six faces above and below the curved graph, a face crossing the graph, out-of-chart refusal, a nonexact base map, nonconstant rational weights, invalid axes and low-budget refusal. Unresolved reports now retain a source UV region, including when completed conservative bounds cannot select one side. Twelve focused integration regressions and no-default-features compilation pass. Evidence and an apply-checked unapplied cumulative patch: `output/qualification/curved-graph-side-2026-10-18/`.

This is a native whole-face diagnostic/selection primitive for the stated coordinate-graph class. It is not connected to arbitrary curved-cutter region replacement and does not qualify general rational NURBS graphs, general fillets, unequal-radius corners, global embedding or WASM/UI delivery. Native changes remain isolated.


## 2026-10-18: actual curved-sector selection audit and immutable source gate

The new `inspect_graph_sectors` diagnostic evaluates actual saved partition/cutter coefficients, not a proxy. Three uncut faces classify Negative. All seven descendants of the three partitioned source charts remain Unresolved near curved seams. Recorded UV cells are about 6.1e-5 wide when depth 28 is reached; interval boxes straddle the thin boundary and cannot establish source-tolerance sidedness. This is an explicit failure to qualify automatic curved material selection, not successful admission. Cell/depth/tolerance caps were unchanged.

`BoundOpenPartition::select_graph_region` now checks every descendant of each requested source and emits per-face side reports and unresolved UV boxes. No selection escapes unless every requested descendant is classified Positive/Negative and at least one is selected. The partition retains the immutable original cutter snapshot, cutter face and UV tolerance; replacement of the cutter or UV tolerance refuses before classification. Duplicate source requests and exhausted shared work also refuse or return no selection.

The actual-control regression verifies seven unresolved diagnostics and no partial selection; mismatched cutter/tolerance refuse. Twelve focused integration regressions and no-default-features compilation pass. Evidence includes diagnostic input hashes, logs and an apply-checked unapplied cumulative patch in `output/qualification/curved-region-selection-2026-10-18/`.

Next geometric work must couple trimmed-side bounds to the saved root-chain/coverage certificates instead of relying only on rectangular interval subdivision near the seams. Automatic curved cap admission, general rational NURBS fillets and unequal-radius corners remain incomplete. Existing explicit-seed control replacement remains a separate qualified workflow. Native work stays isolated; no WASM/UI promotion occurred.


## 2026-10-18: coupled original-support graph residual bands on actual seams

`BoundOpenPartition::graph_seam_bands` matches actual 3D edges, source-support coefficients, coedge pcurves and traversal clocks literally to immutable native world-trace certificates. Reversal is admitted only through the existing bounded integer-knot exact reversal family. Original cutter snapshots and private original-sector support mappings must still match; invalid source geometry refuses.

For the polynomial coordinate graph, outward original derivative coefficient bounds produce a global residual covector norm bound. The triangle sum of both original composition-distance bounds, multiplied by that norm, bounds the vertical graph residual on the complete actual source-lift boundary curve. Both projected source positions and the original graph pcurve must remain in the same convex natural graph chart. No samples, rounded proxy support, stale curve metadata or tolerance increase supply this bound.

The actual closed-control partition qualifies 144 seam uses (72 shared saved edges), with maximum graph residual upper bound 2.836383653454044e-8 mm, below model tolerance 1e-7 mm. Twenty ordinary original boundary uses remain explicitly unmatched. A one-ULP source-pcurve edit removes its inherited match (143 matched/21 unmatched); restoring the literal coefficients restores the original input. A larger invalid pcurve edit is refused by model validation. Exhausted work refuses.

Twelve focused integration regressions and no-default-features compilation pass. Evidence and apply-checked unapplied cumulative patch: `output/qualification/coupled-graph-seams-2026-10-18/`. These are boundary residual bands only: all seven whole-sector classifications near the curved seams remain Unresolved. The next step requires a complete interior/extremum or trim-aware side bound using these bands. No automatic curved cap, arbitrary fillet/G2, global embedding, WASM/UI or full-part acceptance claim is made. Native work remains isolated.


## 2026-10-18: original-chart exclusion of interior graph-residual extrema

`qualify_graph_monotonicity` issues a private owned-support certificate only when one normalized source partial derivative of the graph-height residual is strictly signed over the complete chart. Original graph derivative coefficient intervals, exact affine source identities and outward interval tangent arithmetic establish this bound. Error-free two-component differences admit exact affine source identities even when the original tangent is not a single representable binary64 scalar (e.g. the actual 0.2/-0.05 controls). No rounded tangent is substituted. All source base poles must remain in the graph's full convex coordinate chart.

The certificate owns both original supports and axes and refuses transfer to edited supports. It excludes interior stationary points; it does not establish boundary completeness, material side, trim validity or solid admission. A top chart with possible interior stationary points remains unqualified. Bent source charts, unproven coordinate maps and out-of-chart supports do not inherit the certificate.

Actual seam diagnostics now include four certified monotone wall-sector supports alongside the existing 144 coupled seam-use bands. Two focused graph tests and the actual original-root body-control regression pass (3 targeted tests); no-default-features compilation passes. Broad suites were not rerun in this step. Evidence and an apply-checked unapplied cumulative patch: `output/qualification/graph-interior-monotonicity-2026-10-18/`.

All seven whole-sector side decisions still remain Unresolved. For the four wall sectors, the next proof is complete boundary residual coverage including ordinary source edges; then the extrema theorem can propagate those bounds to the interior. The three top sectors additionally require interior stationary-point/convexity treatment. General fillets, unequal-radius corners, WASM/UI and full-part acceptance remain open. Native changes stay isolated.


## 2026-10-18: complete-boundary propagation resolves four wall sectors

`monotone_graph_sides` now combines the original-support no-stationary-point certificates with complete boundary residual coverage. Every actual source loop must be literally closed, clamped, nonperiodic and contained by the source natural chart. Every coedge needs either its literal native saved-trace seam band or a fresh whole-original-pcurve residual bound. Original cutter trim classification must cover the full source base projection strictly Inside. Only then does the scalar extremum theorem propagate complete boundary bounds to the trimmed interior.

Ordinary edge bounds adapt on one original curve parameter, with unchanged depth 28, 32768-interval and shared cell caps. A boundary crossing, unproven projection, incomplete loop or exhausted work cannot certify a side. No geometry, source coefficient, radius or model tolerance changes.

The actual control now classifies face 3 Positive, face 8 Negative, face 5 Negative and face 9 Positive. The proof uses 180 cells and 32 domain cells, including the existing saved-seam pass. `select_graph_region` consumes these bounds before ordinary rectangular classification and includes all preprocessing costs. Exactly three top sectors remain Unresolved; the complete material selection stays absent.

The focused original-root/body-control test passes, including low cell/domain budgets and an edited boundary endpoint that cannot inherit complete-boundary admission. No-default-features compilation passes; broad suites were not rerun. Evidence includes observed face bounds and an apply-checked unapplied cumulative patch in `output/qualification/monotone-boundary-side-2026-10-18/`. The next step is top-sector concavity/stationary-point treatment. Automatic curved cap admission, general NURBS fillets, unequal-radius corners and WASM/UI remain incomplete; native changes remain isolated.


## 2026-10-18: strict original residual concavity resolves the central top sector

`qualify_graph_concavity` bounds the original graph's physical-coordinate Hessian from outward Bernstein second-difference coefficients. Strictly positive diagonal lower bounds and a positive conservative determinant lower bound establish positive definiteness over the entire original graph chart. A constant-height invertible affine source base then makes the source-minus-graph height residual strictly concave by affine congruence. Owned source/graph coefficients and axes prevent certificate transfer. Nonconstant-height source charts, unsupported maps, source bases outside the graph chart and indefinite graph Hessians remain unqualified.

`boundary_graph_sides` combines complete literal closed-boundary residual bounds with this certificate. Concavity transfers the LOWER boundary bound only. Because an interior maximum can exist, the returned upper bound is conservatively taken from the full original source/graph chart interval; the boundary maximum is never presented as a whole-face maximum. Monotone supports keep the previous two-sided boundary theorem and take precedence when both certificate classes apply.

The actual central top sector (face 7) is now Positive; four wall-sector sides remain qualified. Two outer top sectors (faces 1 and 6) remain Unresolved. The region selector still publishes no partial material selection. Actual preprocessing takes 188 cells and 56 domain cells in this control, with unchanged budgets, source geometry and model tolerance.

Fifteen targeted integration tests, including graph Hessian/saddle/edited/out-of-chart cases, guided/open atlases, region replacement, rooted retention and the seedless cap control, pass. No-default-features compilation and unapplied cumulative patch apply-check pass. This is not the full repository/P0 command suite. Evidence: `output/qualification/concave-boundary-side-2026-10-18/`. Next: qualify/exclude the unique interior stationary maximum for the two outer top sectors. Automatic curved cap admission, general NURBS fillets, unequal-radius corners and WASM/UI remain incomplete; native work stays isolated.


## 2026-10-18: stationary-point exclusion completes automatic control selection and cap

The owned strict-concavity certificate now exposes a source UV enclosure of the unique stationary point only when ORIGINAL graph height coefficients have exact mirror symmetry in both Bernstein axes. Symmetry proves zero gradient at the normalized graph center; strict Hessian positivity proves uniqueness. Outward original-coordinate arithmetic maps that point into the original source knot domains, including nonunit domains. A small asymmetric coefficient edit does not inherit a centered stationary point, even if Hessian positivity survives.

Fresh actual source-trim classification places the stationary box Outside both outer top sectors and Inside the central sector. The outer sectors can therefore transfer their complete boundary upper bounds; all seven control sectors classify (3 Positive, 4 Negative, 0 Unresolved). The complete selection is [7,3,9]. Preprocessing uses 188 cells/200 domain cells in this control.

`BoundOpenPartition::replace_graph_region` now connects complete seedless selection to fresh native-trace cap admission. Unresolved selection returns diagnostics and no replacement. The final cap's entire UV boundary control hull must additionally classify strictly Inside the immutable cutter trim; equality with its support surface alone is insufficient. Cap failures publish no partial model. The result is bitwise equal to the explicit-seed reference model, retains persistent naming/root identities and codec roundtrip evidence, and uses 1682 additional cells plus 208 domain cells after the private partition.

Fifteen targeted integration tests and no-default-features library compilation pass. Tests include symmetry/asymmetry, nonunit source clocks, stationary-box Inside/Outside admission, low-budget no-body refusal and equality with the explicit reference. The fresh automatic-result export is independently checked by OCP using `scripts/verify-interior-chain-brep.py ... --automatic`: one valid replacement solid, 8 faces/86 edges/80 vertices, four original shared root vertices, volume error 4.583097790167301e-12 mm3 against the actual-coefficient reference. There are 72 shared saved cubic edges; 4680 sample sites have maximum sampled residual 2.8054308875979928e-11 mm. Sampling and quadrature convergence are not global interval/embedding certificates.

Evidence, actual automatic STEP/model, native source hashes, logs and an apply-checked unapplied cumulative patch: `output/qualification/automatic-curved-cap-2026-10-18/`. This qualifies the documented polynomial-coordinate-graph control after original root-owned partitioning. Automatic contour generation for arbitrary B-rep, general rational NURBS fillets, unequal-radius complex corners, G2/global diagnostics, WASM/UI and full-part acceptance remain incomplete. The active goal remains broader than this milestone; native sources are still isolated.


## 2026-10-18: asymmetric original graph selection, exact trim attributes and STEP controls

Strict graph-concavity certificates now support exact affine source height slopes over the admitted coordinate bases. Original bilinear height twist must vanish by exact error-free differences; warped sources remain unsupported. For nonsymmetric graph coefficients, outward Bernstein derivative evaluation at the graph center and the global positive-definite original Hessian give the necessary stationary cover p = q - H_avg^-1 (grad h(q) - source_slope). Every possible stationary point is covered; root existence is not asserted. Only fresh Outside classification of the entire box against the actual source trim permits boundary-maximum transfer. A shared 2,000,000-operation stationary budget is enforced; insufficient work returns no box. Exact mirror symmetry remains a fast special case. Tests include a dyadic nonsymmetric polynomial with independently known stationary coordinates, tilted planes, nonunit clocks, unsupported warped sources and exhausted work.

The actual asymmetric B-rep control edits the original graph corner height coefficient by 0.0001 before constructing metadata and certificates. It needs no symmetry assumption and now classifies all seven sectors and caps the selected region automatically, equal to the explicit reference result. Two concrete failures were corrected:

- Overlapping continuation proposals may use larger boxes than their certified root images. A same-axis containing carrier can discard a redundant proposal only when it covers either the whole proposal box or its immutable uniform original root image for the entire covered free range. Deterministic priority prevents mutual deletion for equal ranges. The full original product census and fresh transition graph remain mandatory. A focused private test freshly certifies two distinct roots with identical free ranges and proves neither discards the other; duplicate carriers have a single retained priority.
- Rational curve subdivision introduced 0.7500000000000001 from an exactly constant original trim coordinate 0.75. Edge partition now preserves dimensions whose ALL original Euclidean control coordinates are exactly equal. The rational identity makes this exact for arbitrary weights; no tolerance, snapping or proximity rule is used. Both child curves, shared split position and incident pcurves retain the original constant attribute, followed by existing original-curve/support proofs. A regression covers three nonbinary split parameters, original constant z and all incident constant pcurve coordinates.

The new two-interior-point continuation experiment is explicitly NOT qualified: it returns boundary-seed-chart-unproven after 128 rectangles, with zero charts, before pruning. This is distinct from the direct original-root preservation regression and the successful root-owned boundary control. No components or partial body escape that refusal. Diagnostic evidence is retained; broad seedless arbitrary continuation is not claimed.

Fresh targeted results: 26 B-rep integration tests plus 1 original-root carrier regression pass; no-default-features library compilation passes. These are not the full repository/P0 suite. Independent OCP STEP checks pass for BOTH symmetric and asymmetric actual automatic bodies: one valid solid, 8 faces, 84 edges, 78 vertices; four original shared root vertices survive. The nonredundant representation contains 70 shared saved cubic edges rather than the previous 72. Their complete native source/support admission remains required; 4550 independent sample sites per control supplement that proof. Actual-coefficient Decimal volume references give STEP volume errors 4.583111667955109e-12 mm3 (symmetric) and 4.592146107817996e-12 mm3 (asymmetric). Quadrature convergence and STEP samples are not global interval/embedding certificates.

Artifacts, actual source/cutter snapshots, automatic models/STEP, independent reports, source hashes, native logs, refusal diagnostic and an apply-checked UNAPPLIED cumulative patch covering 101 changed native files: output/qualification/general-stationary-curved-cap-2026-10-18/. Native changes stay isolated. Shared native sources/WASM/UI have not been promoted in this step. Arbitrary rational NURBS trimming, unequal-radius complex corners/end transitions, general fillets/G2, whole-body diagnostics and complete part acceptance remain open.


## 2026-10-18: bounded multiscale recovery of approximate interior intersection seeds

The previously recorded two-interior-seed refusal is now resolved for both actual symmetric and asymmetric control supports. The seed was the midpoint of an outward fixed-fiber root enclosure, not a root coordinate. In the symmetric control, the lower original branch proposal used FirstV approximately 0.04998884, while the true branch lies around 0.03096. The initial +/-1/64 unknown-coordinate box was correctly excluded by the original Krawczyk operator. Repeatedly shrinking only the free interval could never recover that omitted root. The exclusion was valid; the proposal policy was insufficient.

Interior-mode initial attempts now use bounded multiscale boxes: progressively shrink the free interval and widen the three other original-coordinate proposal ranges around the approximate seed, clipped to the original natural domains. There are still at most eight initial attempts and the existing shared proposal/rectangle/evaluation caps. Each attempt must freshly prove original uniform-root uniqueness; an excluded first box says nothing about the rest of the surface. Continuation, root-image-safe carrier pruning, whole original product census and fresh transition graph qualification remain mandatory. The legacy natural-boundary-only entry point keeps its previous proposal policy. No tolerances, depth or work limits were relaxed.

The symmetric interior-seed atlas now completes with 15004 rectangles and 40 charts; asymmetric corner-offset control with 15670 rectangles and 40 charts. Both have two separately owned original components and pass fresh component replay. Reordered/repeated seed proposals retain two components. Omitting a branch refuses the mandatory product census; a 64-rectangle budget publishes no components. Initial narrow boxes remain demonstrably excluded. These seeds are derived in the regression from existing freshly qualified original fibers: this is NOT evidence of seedless discovery for arbitrary rational surfaces or B-rep trim/body admission from these seeds alone.

Fresh verification: 28 B-rep integration tests plus 1 distinct-original-root carrier regression pass; no-default-features compilation passes. Both actual automatic cap STEP exports are regenerated and independently verified again, preserving the prior one-solid/8-face/84-edge/78-vertex geometry and approximately 4.6e-12 mm3 volume error. These saved-body controls use the existing root-owned boundary pipeline; the interior-seed recovery is not silently substituted for B-rep endpoint ownership. The full repository/P0 suite, WASM/worker/UI, general NURBS fillets, unequal-radius corners and global diagnostics remain unqualified.

Evidence, before/after logs, actual source/cutter/models/STEP, independent reports and an apply-checked UNAPPLIED cumulative 102-native-file patch: output/qualification/interior-seed-recovery-2026-10-18/. Native work remains isolated; the full thread goal remains active.


## 2026-10-18: nonconstant-weight rational original-root partition and explicit-seed closed cap

The actual branch fixture now independently varies an original height coefficient and an original positive rational weight before creating the cutter sheet and metadata. The rational control uses corner height offset 0.0001 and weight offset 0.001; all other weights remain 1. Its base-coordinate maps are genuinely rational and coupled, so the polynomial affine-base graph certificates must refuse it. This step expands verified scope; it does not introduce a new production clipping algorithm or claim arbitrary rational support.

The interior-seed atlas freshly certifies both original rational branches: 14292 rectangles, 40 charts, 10172 census rectangles and 3952 graph rectangles. Missing-branch/low-work refusals and reordered/repeated seeds are exercised for this control too. The initial-box exclusion assertion remains specific to the two polynomial controls; a rational control whose first box already qualifies is not forced to fail that legitimate proof.

The actual rational B-rep then passes the original-boundary root registry, incident source UV lifts, full boundary census, original interval covers, coupled saved world/pcurves, canonical welding, whole original trimmed-root coverage and shared network partition. Source/root vertex positions and persistent identities remain literal; codec roundtrips preserve both the partition and cap models. Every saved native trace replays against its immutable original supports. Nonconstant-weight surfaces do not inherit polynomial graph seam/material-side certificates.

With EXPLICIT region seeds, the existing generic native-trace replacement path closes the rational cap. Original root vertices and naming survive; an edited cutter weight and insufficient cap work refuse. This verifies that the generic root-owned subdivision/cap mechanisms support this rational control; automatic rational material selection remains unqualified and is not bypassed.

Thirty targeted B-rep tests pass; after final rational identity/codec/negative-cap assertions, all six tests in open_boundary_atlas pass again. Independent OCP verification reads the fresh rational partition STEP as one valid solid, 10 faces/80 edges/72 vertices and source volume 0.25 mm3. All 64 saved curves are literal model edges shared by two uses; 4160 support sample sites have maximum deviation 2.7467660543502643e-10 mm. Four original roots are shared by four incident edges/faces in the partition. The explicit cap STEP is one valid solid, 8 faces/78 edges/72 vertices; each original root is shared by three edges/faces.

A separate 55-digit Decimal reference reads the ACTUAL original rational coefficients. For this one-corner-weight family it algebraically inverts original X(u,v), brackets both actual height roots at each physical x, and integrates (top-z)*dy/dv at fixed x using tensor Simpson refinements 64/128/256. The final successive difference is approximately 1.0353e-13 mm3. Its changed-volume reference is 0.1128668421836310182683548321094012752746900914063418170 mm3. Independent STEP volume is 0.11286684215953272 mm3, error 2.409830668348434e-11 mm3. Quadrature convergence and independent support samples supplement native whole-piece proofs; they are not interval integration, global embedding, arbitrary NURBS or fillet/G2 certificates. Both previous automatic polynomial cap controls are independently checked again.

New independent tools: scripts/reference-rational-root-cap-volume.py and scripts/verify-rational-root-partition.py. Evidence, actual rational/polynomial geometry, snapshots, STEP, reference, source hashes and an apply-checked UNAPPLIED cumulative 102-native-file patch: output/qualification/rational-root-partition-2026-10-18/. Shared native sources, WASM and UI are not promoted. Next geometric gap: original rational implicit side/inverse-coordinate bounds for automatic material selection; then general fillets, end transitions and unequal-radius corners. Full thread scope remains active.


## 2026-10-18: uniform original rational inverse and whole-trim projected side primitive

New native enclose_projected_inverse supports a globally certified projection of an ORIGINAL rational NURBS chart. The existing common-Jacobian global projection contraction certificate first excludes remote projected roots across the complete original natural chart. Fresh original interval position and Jacobian bounds then construct a fixed-preconditioner two-dimensional Krawczyk map for an entire physical target box. A contraction norm below one and strict image inclusion in a convex original UV box establish existence for EVERY target and a root enclosure; the global certificate supplies uniqueness on the whole chart. Numerical midpoints/preconditioners are proposals only. No sampled solve grants inverse admission. Returned certificates privately own the original surface, axes, target and receipt; replay recomputes the original proof. Unsupported global projections, periodic charts, outside targets and insufficient work return no inverse. Original validation rejects disconnected interior knot multiplicity.

Global refinement and local original-span/center evaluations share the caller's cell budget. A separate shared charged coefficient ledger is capped at 2,000,000 units: each original span charges 32*n*m+256. This is a declared conservative admission cost model, NOT an instruction count or runtime performance measurement. The inverse has 16 attempts and existing global refinement depth 6; no previous depth/tolerance/work caps are raised.

New B-rep classify_face_projected uses this inverse for each source interval image, then requires strict Inside classification of the ENTIRE resulting UV box in the ORIGINAL cutter trim. Its signed height residual uses the original source and original rational cutter interval images. It adaptively covers the actual source trim, sharing source/cutter domain work, image evaluations, inverse costs and the charged coefficient ledger. A source trim not enclosed by its natural chart, periodic source, oversized supports/pcurves, mixed sides, unresolved inverse or exhausted work cannot publish a material side. This is scalar side along the selected height axis; it is not general oriented-solid Boolean admission.

Four new controls pass: nonconstant/coupled rational original inverse with source/target mutation refusal, reverse projection axes, low work and outside targets; continuous multiple-knot-span/nonunit-clock inverse with disconnected-input refusal; folded requested projection refusal; whole-face Positive/Negative and mixed/low-work Unresolved behavior. The rational inverse covers target XY [-0.5,0.5] x [-0.4,0.4] with UV [0.24820634004105172,0.7518560623558681] x [0.2981313776967369,0.7019310247001829], using 3 cells/1632 charged units. Separated source faces use 6 cells/2560 units and produce original residual intervals [4.764759126046043,5.274985175385602] and [-5.235240873953957,-4.725014824614398]. The intersecting source face remains Unresolved, as required.

Fresh verification: 34 targeted B-rep tests plus 17 existing core injectivity tests pass (51 total), and no-default-features compilation passes. All three fresh STEP controls are independently checked again: both automatic polynomial caps and the explicit-seed rational partition/cap. The rational Decimal reference is reused only after verifying its actual cutter coefficient SHA matches the fresh export. The new side primitive has NOT resolved every near-seam descendant and is NOT silently substituted for the existing automatic polynomial selector. Rational implicit derivatives and certified boundary bands are still needed before complete seedless rational material selection/capping can be admitted.

Evidence, fresh geometry/STEP, independent reports, source hashes, logs and an apply-checked UNAPPLIED cumulative 106-native-file patch: output/qualification/projected-rational-side-2026-10-18/. Native sources remain isolated; shared native sources/WASM/UI are not promoted. General rational fillets, unequal-radius corners/end transitions, G2/global diagnostics, complete part acceptance and P0/P2-P3 remain open. Full thread goal stays active.


## 2026-10-18: original rational height gradients, literal seam bands and wall stationary exclusion

An owned uniform inverse certificate now bounds physical height derivatives from ORIGINAL rational Jacobians. For each complete original UV subrectangle, the projected 2x2 Jacobian determinant must be interval-separated from zero; otherwise adaptive refinement continues within the unchanged depth/cell/charged-coefficient limits. The height gradient is [ (hu*yv-hv*yu)/det, (hv*xu-hu*xv)/det ]. Complete coverage, not independent local root guesses, supplies a common bound. The original inverse establishes a height function over the ENTIRE convex physical target rectangle, so sqrt(1+hx^2+hy^2) bounds the residual covector z-h(x,y) along every enclosed physical segment. Private owned supports/axes/target and fresh replay prevent transfer. The new gradient token uses an immutable clone of the original inverse; no sampled derivative grants proof.

For the actual nonconstant-weight rational control and physical target [-0.6,0.6]^2, gradient bounds are [-0.007818949284116479,0.007053966633217063] and [-1.212221263772921,1.2117056673391944]. One original gradient cell suffices; the conservative residual factor is 1.5714774985061437. A derivative sample grid supplements the native interval proof, and a multiple-span one-cell budget publishes no gradient certificate.

BoundOpenPartition::projected_seam_bands couples actual coedges to literal immutable native world curves, original source/cutter supports and common-clock pcurves (including only exact integer-clock reversal). BOTH original lifts must be enclosed in the physical rectangle of the owned rational height proof. Their complete composition-error sum times the original residual factor bounds the source graph-height residual. Missing uses remain diagnostics; this does not establish boundary completeness or whole-sector material side. Source/cutter interval evaluations, inverse and gradient work share the caller cell cap and 2,000,000-unit charged coefficient ledger.

All 128 native seam uses (64 saved curves) qualify on the actual rational control, with 20 ordinary unmatched uses, maximum upper bound 1.9934296833569726e-8 mm, 664 cells and 219264 charged units. An edited original cutter refuses transfer. A too-narrow physical rectangle refuses the call; no partial band report escapes. A one-ULP edit of one actual pcurve interior coefficient removes exactly its band and adds one unmatched use; restoring the coefficient restores the original geometry before cap/export. Low seam work refuses.

The exact-affine-source monotonicity engine is shared with the existing polynomial entry point. A new rational entry point accepts only the owned original height-gradient token and requires every original affine-source base pole to lie in that token's convex physical rectangle. It proves a strictly signed normalized source partial, excluding interior stationary points. Four rational control wall-sector charts now carry this proof; the three top-sector charts remain unqualified. This is NOT a whole-sector side decision: complete ordinary boundary bounds and top-sector stationary/Hessian treatment remain required. The first-inclusion inverse may still be too broad for near-root ordinary edge bounds; tightening that enclosure is the next numerical prerequisite.

Fresh verification: 35 targeted B-rep tests and 17 existing core injectivity tests pass (52 total), including the polynomial monotonicity regressions after the shared-helper refactor. No-default-features compilation passes. All three fresh STEP controls independently pass again: both automatic polynomial caps and the explicit-seed rational partition/cap. The actual rational coefficients match the prior Decimal reference SHA before reuse. There is no new automatic rational cap, G2/radius certificate or global embedding claim.

Evidence, fresh geometry/STEP, independent reports, source hashes, logs and an apply-checked UNAPPLIED cumulative 108-native-file patch: output/qualification/rational-projected-seams-2026-10-18/. Shared native sources/WASM/UI remain untouched. Complete rational material selection, general NURBS fillets, unequal-radius corners/end transitions, whole-body diagnostics, part acceptance and P0/P2-P3 remain open; the full goal remains active.


## 2026-10-18: refined original inverse and complete rational wall-sector boundary propagation

New enclose_projected_inverse_refined keeps the original global uniqueness proof and repeats fresh original Krawczyk operators within the unchanged 16-iteration/cell/charged-coefficient limits. Every accepted cover has fresh strict inclusion and contraction. Intersections of already proven covers retain ALL uniform target preimages. Later proposal boxes pad the proven cover, not the old natural chart, avoiding an artificial width floor. A failed/exhausted later refinement retains the last proven cover and its full work receipt; it never asserts requested precision. Private refinement parameters participate in replay. The default first-inclusion entry point remains unchanged.

The actual rational point target X=0.1,Y=0.45 tightens from UV [0.5480312906330607,0.5515315375106334] x [0.7231185719925556,0.7266188188701286] to [0.5500084217390997,0.5500084217391512] x [0.7250111013833601,0.7250111013834247] in 9 cells. An independent scalar interval bracket uses the actual one-corner weight equation r=delta*(1-u)^2*(1-v)^2, u=(1+x)*(1+r)/2, v=(1+y)*(1+r)/2. Outward residual signs retain a bracket and its transformed UV interval lies inside the refined native enclosure. This is independent algorithmic control for this original coefficient family, not a replacement for generic original-operator proofs. A three-cell/15-refinement request returns the original coarse cover honestly and replays; a refinement limit above 15 refuses.

BoundOpenPartition::projected_boundary_sides now combines owned original rational monotonicity, literal saved seam bands, complete ordinary ORIGINAL source-pcurve residual bounds and fresh original cutter-trim inclusion of the whole inverse target. Every actual loop must remain literally closed, clamped, nonperiodic and within the original natural chart. Ordinary edges adapt on one parameter with refined original inverse/height enclosures. Curve controls remain capped at 256, depth at 28 and intervals at 32768; source/cutter evaluations and all inverse queries share the caller cell cap and 2,000,000-unit charged coefficient ledger. No tolerance or previous work limit is relaxed. Incomplete loops/ranges, work or cutter-domain coverage cannot publish a whole-face side.

All four rational control wall sectors now classify: face 3 Positive [-4.975051332012371e-8,0.21873349367538614]; face 8 Negative [-0.2812765847538364,8.017337765897638e-8]; face 5 Negative [-0.28148655467087186,9.222786728768641e-8]; face 9 Positive [-7.04072650736798e-8,0.2186027019727887]. These are whole-sector bounds with the unchanged model tolerance 1e-7 mm, not exact zero-sign claims. Total work is 1496 cells, 612480 charged units and 8 cutter-domain cells. Only the four certified monotone charts are reported; three top sectors remain unqualified. No partial complete-region selection or automatic rational body is published.

Negative controls reserve only the existing seam budget (all wall sides stay Unresolved), exhaust the cutter-domain budget (no side), and perturb an ACTUAL WALL pcurve endpoint (whole call refuses or that wall stays Unresolved). The focused final control selects a monotone wall seam for this closure mutation, avoiding a vacuous check on an unreported top sector. Geometry is restored before explicit-cap admission and export. Existing one-ULP interior edit, original-support mutation and low seam-work tests remain.

Fresh verification: 36 targeted B-rep tests and 17 core injectivity tests pass (53 total). The focused rational admission is rerun after the final wall-closure/control-cap assertions, and no-default-features compilation passes on final source. Both automatic polynomial STEP caps and the explicit-seed rational partition/cap independently pass again; rational actual-coefficient reference SHA is checked before reuse. No full repository/P0, general fillet, G2, radius/thickness/global embedding or WASM/UI qualification is claimed.

Evidence, fresh geometry/STEP, independent reports, hashes, logs and an apply-checked UNAPPLIED cumulative 109-native-file patch: output/qualification/refined-rational-boundary-2026-10-18/. Native sources stay isolated. Next: rational top-sector Hessian/concavity and stationary candidate handling, then complete rational material selection/capping. General fillets, unequal-radius corners/end transitions, whole-body diagnostics, UI, part acceptance and P0/P2-P3 remain open; full goal remains active.


## 2026-10-19: all rational control sectors classified and automatic closed cap admitted

Original rational second jets now establish strong PHYSICAL height convexity. On each original knot cell, J is the projected first derivative matrix and Q_ab = z_ab - hx*x_ab - hy*y_ab. The physical Hessian is J^-T Q J^-1; original full-span normalized derivative scales cancel consistently. Complete original UV coverage, signed interval determinant separation and positive-definite local Hessian bounds give a common eigenvalue lower bound m = min(det_lower/trace_upper). Original C1 knot continuity is mandatory: positive local Hessians with unknown gradient jumps cannot establish this theorem. The second-derivative bounds include all cell interiors and one-sided knot limits; they do not assert unique second derivatives/G2 at a C1-only knot. Work is bounded by the existing cell/depth limits and a shared 2,000,000-unit coefficient ledger; second jets charge 128*n*m+512 per evaluated original span.

The actual nonconstant-weight rational control qualifies in one second-jet cell (1664 units). Physical Hessian bounds [xx,yy,xy] are [0.009140798632374448,0.011440071418631736], [1.983336777311337,2.015234464283429], [-0.004267854940200035,0.003305862918567113], with m=0.0089363473003429. Tests include actual rational derivatives, a saddle refusal, axis/height permutation, an original C1 multi-span chart with nonunit clocks and independent expected physical curvatures, a one-cell multi-span work refusal, and C0-representation refusal. The C0 refusal remains conservative even where specially matched coefficients could describe a smoother shape; no exact seam-jet gluing is inferred.

Exact full-rank affine source frames may be skew and use nonunit clocks. Strong graph convexity makes affine-source-minus-height strictly concave. At an arbitrary exact physical query point q in the convex target, every possible stationary point p satisfies |p-q| <= |source_slope-grad(h)(q)|/m. Fresh original interval gradient bounds produce this necessary ball, mapped through the original affine source inverse and original knot domains. It asserts neither existence nor a centered root. A zero stationary-work budget retains concavity only, with no stationary cover. An edited/warped source cannot inherit the affine certificate.

The skew-source regression exposed a numerical weakness in the prior gradient enclosure: subtracting tiny restricted control differences and dividing by their tiny parameter width produced enormous but valid derivative intervals. The gradient now differentiates the ORIGINAL full span first, then evaluates its query with stable original first jets. All common parameter scales cancel in the physical inverse. The revised global seam factor is 1.5713546185769838 and all 128 seam uses remain bounded by 1.993273809347635e-8 mm. The skew/nonunit stationary cover is now tight and the former failure log is retained as diagnostic evidence.

monotone_projected_sides retains the prior four-wall scope. projected_boundary_sides additionally handles concave source sectors and reports every actual descendant. All seven rational control sectors now classify: three Positive [7,3,9], four Negative [6,1,8,5], zero Unresolved. The necessary stationary UV box is [0.48454809525110626,0.515451904748894]^2; fresh ACTUAL source-trim classification places it Outside both outer top sectors and Inside the central sector. Only Outside permits boundary-upper transfer. Otherwise an original positive-rational control-hull upper bound is retained; concavity transfers the lower bound only. Total complete-side work: 2255 cells, 974528 charged units, 128 domain cells.

select_projected_region requires every descendant of every requested original source to have a material side. Duplicate sources, edited original supports and tolerance changes cannot transfer admission. replace_projected_region then performs fresh native-trace cap admission and checks the final entire cap UV control hull strictly Inside the immutable original cutter trim. There are NO region seeds in this automatic path. Unresolved selection publishes diagnostics and no body; cap failures publish no partial model. The cap uses the existing separate cell/exact receipts; the charged coefficient counter describes projection/side preprocessing, not cap instruction counts.

The actual automatic rational result selects [7,3,9] and is exactly equal to the explicit-region-seed reference model, including root positions and persistent naming. Additional work is 3528 cells, 974528 preprocessing units and 136 domain cells; the original partition's cells/exact work are charged separately. Low domain work publishes no selection/body. All existing altered-pcurve, broken-loop, wrong support, low-work, root-coverage, identity and codec regressions pass.

Fresh verification: 39 targeted B-rep tests plus 17 core injectivity tests pass (56 total); no-default-features compilation passes. All THREE automatic-result STEP controls pass independent OCP checks: the two polynomial cases and the new rational case. The rational automatic STEP is a valid solid with 8 faces/78 edges/72 vertices, original roots shared correctly, and volume 0.11286684215953272 mm3. Actual-coefficient reference volume is 0.11286684218363102 mm3, error 2.409830668348434e-11 mm3. The unchanged reference cutter SHA is verified before reuse. The independent script now has explicit --automatic admission checks; older explicit-only evidence correctly refuses this mode without modifying its historical report. Metadata alone is not native proof: current source, native test calls and body equality establish which operation produced the export.

Evidence, actual automatic geometry/STEP, independent reports, before/after diagnostics, logs, hashes and an apply-checked UNAPPLIED cumulative 112-native-file patch: output/qualification/automatic-rational-cap-2026-10-19/. This qualifies the documented original rational graphical-cut control AFTER root-owned partitioning. It does NOT complete arbitrary B-rep contour discovery, general rational source/cutter coverage, NURBS fillets, end transitions, unequal-radius corners, G2, global diagnostics or full part acceptance. Shared native sources, WASM and UI remain untouched. The full thread goal remains active.


## 2026-10-19: actual stored and partial contact G2 in signed transverse frames

The historical owned-band radius 0.1 mm refusal is NOT current on this isolated checkout: the pre-change canonical test already passed with 235 cells, and the rotated 0.3->0.1 mm case with 327. This step does not claim to have newly fixed that old geometry failure. Instead it closes a specific verification gap: direct G2 inspection of the ACTUAL stored shared 3D contact edge and BOTH actual incident pcurves initially returned Unresolved after exhausting 2,000,000 coefficient units.

Both mapped collars enter their respective original retained/inserted charts, so their transverse directions are opposite in physical space. The original two-trace verifier assumed a +1 transverse jet correspondence. New inspect_g2_traces_cross_frame and inspect_owned_in_frames_cross_frame expose the existing original-jet signed-gain mechanism for TWO actual traces. The default entry points delegate with +1 and preserve their previous semantics. A finite nonzero gain transforms every order-b target transverse jet by gain^b; positions remain unchanged. Original mapped collars are still proved inside their original Bezier charts. Only seam jets at transverse zero are compared, so a negative gain does NOT admit extrapolated surface points or alter stored geometry. One coherent original-chart orientation applies to both the normal and signed world curvature tensor.

replace_owned_band now requires this fresh actual stored-trace G2 inspection with cross gain -1, IN ADDITION to the existing canonical original-support G2, native radius law, saved boundary lifts, original trims, regularity and closure checks. Owned position snapshots are retained separately from mutable public reports. Position tokens alone never establish G2; admission checks the fresh geometric decision. All original cell/exact/coefficient limits and tolerances remain unchanged and the new proof is charged to the SAME body budget. The operation still changes only the two radial boundary edges; circular contacts, source vertices and all unaffected identities survive.

Whole actual contacts pass at both ends of both poses. Default band contacts use 9 and 6 cells with 115112 and 96785 coefficient units; rotated/reversed-radius contacts use 9 cells/115136 units each. Whole-body admission now uses 250 cells (default) and 345 (rotated 0.3->0.1 mm), including the added mandatory checks. The orientation on both ACTUAL original surface charts is Same, including reversed coedge clocks.

Non-symmetric partial world-clock intervals [0.125,0.625] are also freshly qualified against both original supports. Reversed incident pcurves use the corresponding original clock interval [0.375,0.875], retaining exact Bezier knot domains; no rounded whole-rim proxy is substituted. All four partial-contact controls pass with 3 cells each and fresh owned-position replay. These are continuous partial CURVE G2 proofs, not closed partial-band/corner topology or owned original root-end binding. A middle transverse control perturbation preserves seam positions and first jets but changes second curvature jets: all four controls still prove positions and return CorrespondenceExceedsTolerance for G2. A changed support cannot reuse the original owned-position snapshot.

Fresh regressions: 8 core collar/pcurve unit tests plus 13 B-rep radius-transition/corner/mapped-corner/end-collar tests pass (21 total), including default +1 compatibility and existing neighbours. No-default-features compilation passes. Native owned acceptance exports the actual default 0.1->0.3 mm transition with width 0.1 mm. The independent --actual-owned mode validates the new native receipt fields before performing its existing STEP/material checks, so it cannot silently relabel legacy evidence.

Independent OCP STEP: one valid solid, 11 faces/21 edges/12 vertices; volume error 5.185629703419181e-12 mm3. There are 507 radius samples (maximum error 2.220446049250313e-16 mm), and 50 seam samples (normal chord 8.408927699556912e-15, world tensor difference 1.9496115277262817e-13). Before/after material probes confirm the conical band was actually replaced. Sampling supplements native continuous original-coefficient proofs; rotated controls are native-qualified here, not independently STEP-exported. Only the two circular contacts claim G2. Radial/end contacts remain sharp; whole-body G2, a constant rolling-ball fillet radius and a general corner are explicitly false.

Evidence, actual source/result geometry/STEP, independent reports, before/after diagnostics, logs, hashes and an apply-checked UNAPPLIED cumulative 112-native-file patch: output/qualification/owned-actual-g2-2026-10-19/. Shared native sources/WASM/UI remain untouched. General NURBS fillet construction, partial root-end topology, end transitions and unequal-radius complex corners remain open, together with the broader diagnostics/acceptance/P0/P2-P3 goal.


## 2026-10-19: atomic partition of actual owned G2 contacts

Native isolated implementation adds `edge_partition::partition_g2_contact`.
It checks the original contact, partitions one shared 3D edge and both actual
incident pcurves atomically, then freshly qualifies both child contacts on
original supports. Independent pcurve clocks and repeated partitions are
covered for default and rotated, reversed-radius quarter-band controls.
A discovered identity regression is fixed: boundary subdivision retains the
unchanged wires, faces, shells and bodies by authored operation correspondence;
new edge identities retain explicit split lineage. Original vertices survive.

Evidence: `output/qualification/partition-owned-g2-2026-10-19/`.
31 distinct focused tests passed (32 successful executions), including existing
partition, corner, end-collar and radius-transition regressions; no-default
features check passed. Independent OpenCascade STEP is valid: one solid,
11 faces, 23 edges, 14 vertices; volume error 5.18514e-12 mm3. Two newly authored
vertices each have two incident edges and faces. 99 independent samples yield
maximum position error 6.24501e-17 mm, normal chord 4.96507e-16, signed shape
tensor difference 7.10543e-15. Sampling supplements native continuous checks.
The cumulative 114-file native patch passes apply-check against the frozen
baseline; it remains unapplied to shared native sources.

Limits: supplied cut parameters do not prove root isolation. This is not
arbitrary B-rep trimming, general unequal-radius corner construction, full-body
G2, global intersection/thickness proof, WASM delivery or UI acceptance.


## 2026-10-19: fresh trimmed-face roots bound to actual G2 child contacts

`edge_partition::partition_g2_at_face` now combines a fresh parent-contact G2
inspection, complete original curve/surface root and actual cutter-trim
classification, atomic shared-edge partition, and fresh G2 on both resulting
incident uses. Parent, root/trim/partition and child cells share the 100000-cell
budget. G2 coefficient/exact limits are shared across the three G2 inspections;
the existing root classifier retains its own bounded arithmetic accounting.
No caller-supplied root or G2 receipt authorizes the edit. The authored vertex
is bounded relative to the original root position box, not claimed exact.

Controls use both circular contacts of the unequal-radius quarter body and a
nonplanar bilinear cutter x=c+0.05*y*z. Both original pcurves use independent
nonunit clocks. A hole containing the root, an absent root, and exhausted work
all reject without mutating either source. 18 focused tests passed (partition,
G2 partition and trimmed-face roots); no-default-features check passed.

Evidence: `output/qualification/root-owned-g2-2026-10-19/`.
Independent STEP checks pass for both contacts: one valid solid, 11 faces,
22 edges and 13 vertices each; each new vertex is shared by two edges and two
faces. Original vertices/identities and face/support identities persist.
66 samples per result independently check position, normal and signed shape
tensor. Maximum volume error is 5.813e-12 mm3, position error 3.361e-16 mm,
normal chord 8.467e-15 and tensor difference 1.959e-13. Upper-contact root box
is [5.496558029278126,5.496558029278609] on the original edge clock [2,8];
authored root-position error upper bound is 2.860e-14 mm.
Cumulative 114-file native patch passes apply-check against the frozen baseline.

Scope remains a root-bound contact subdivision, not material-side trimming or
arbitrary B-rep fillet assembly. General unequal-radius corners, end transitions,
global embedding/thickness/intersections, WASM and new-UI acceptance remain open.


## 2026-10-19: root-bound side chains assembled into a closed replacement

`region_replacement::replace_g2_rooted_sides` combines fresh actual-trim root
queries, atomic shared-edge/pcurve splits, propagation of child ownership into
four mapped side chains, removal of selected sectors, and insertion of the
qualified G2 chart. The operation returns no candidate unless all original
fragment G2, trim and closed-incidence gates pass. Root partitioning and final
assembly share a 100000-cell budget. Each final root vertex must keep its exact
stored point and identity; both mapped child curves/identities and their
incidence at the root must survive replacement. Roots are approximations with
certified original position-box error bounds, not exact symbolic intersections.

The nonplanar paraboloid control replaces one selected sector after four
independent automatically isolated crossings. Eight actual saved fragments
pass G2 admission. Source remains immutable; final codec roundtrip preserves
the result; exhausted combined work refuses. Root partition work is 138 cells,
total assembly work 15761. 20 focused tests passed across region replacement,
mapped/network replacement, mapped corners, G2 partitions and face crossings.
The final additional preservation guards passed two repeated mapped tests;
no-default-features check passed.

Evidence: `output/qualification/rooted-region-2026-10-19/`.
Independent OpenCascade STEP: valid single solid, 30 faces, 64 edges, 36
vertices. Volume 30.375 mm3 equals the independent analytic integral. All
original vertex positions survive; 264 saved-seam samples have maximum position
error 5.853e-15 mm, normal chord 6.496e-15, signed shape tensor difference
3.320e-14. Native whole-fragment checks remain primary; sampling is supplemental.
Cumulative 116-file native patch passes apply-check against the frozen baseline.

This closes the root-to-assembly path for supplied four-sided mapped regions.
It does not select material regions automatically, construct general unequal
radius laws/end setbacks, certify global embedding/thickness/intersections,
or promote code to WASM/new UI. These parts of the full goal remain open.


## 2026-10-19: automatic component discovery from retained side ownership

`select_region_from_sides` discovers the complete connected component opposite
four explicitly retained side chains. It checks unique retained shell ownership,
two distinct actual face uses per nondegenerate edge, complete rim incidence,
connectivity of all opposite-face seeds, exclusion of retained supports and
literal equality between the component boundary and the supplied rim. Every
shell-use/incidence/adjacency/traversal visit is bounded; the control uses 241
visits. It does not infer a geometric material side from approximate samples.
`replace_g2_rooted_region` recomputes this selection on the immutable source,
then runs fresh trimmed-face root isolation, shared child propagation and G2
region assembly without a caller-supplied selected-face list.

Nine focused tests pass: mapped/rooted replacement, region/network replacement
and new selection regressions. Connected multi-face selection works. Missing
rim edges, disconnected components, inconsistent retained sides, repeated edges
and exhausted topology work refuse. Automatic selection and explicit reference
produce exactly equal saved Models; export uses the automatic result.
No-default-features check passes.

Evidence: `output/qualification/automatic-side-region-2026-10-19/`.
Independent STEP: valid single solid, 30 faces, 64 edges, 36 vertices; volume
30.375 mm3, analytic error zero. 264 seam samples: maximum position 5.853e-15 mm,
normal chord 6.496e-15, signed tensor difference 3.320e-14. Cumulative 118-file
native patch passes apply-check against the unchanged frozen baseline.

Limits: explicit retained-side/rim inputs remain required. Geometric material
classification, automatic contour discovery on arbitrary B-rep, general unequal
radius laws/end transitions/complex corners, global embedding and WASM/UI remain
open. This topological selection milestone does not close those requirements.


## 2026-10-19: atomic variable-law G2 fillet body on original supports

`variable_fillet_body::replace` constructs variable nominal-radius quintic G2
contact skins directly from two original face charts; freshly orders actual
trimmed-cutter root intervals, binds both contacts to shared original boundary
roots, builds end curves, selects retained sectors from explicit keep vertices,
and performs the closed graft. It then freshly inspects actual saved two-use
contact G2, rolling-ball contact-center law and full saved-chart injectivity.
One immutable original law domain survives trimming; no endpoint-law
renormalization or tolerance increase is hidden. Root points/identities and
original support surfaces must survive, and cumulative naming is refreshed
relative to the public input. No intermediate body escapes on failure.

Controls use an original trimmed cuboid with authored extended planar supports.
Three original-domain laws pass: 0.5 -> 1.5, 0.1 -> 0.2, 1.5 -> 0.5 mm.
Swapping both end cutters and their associated retention seeds produces exactly
the same final Model. Invalid support roles and insufficient work refuse.
Sixteen focused tests pass across variable body, end collars, contact/radius
qualification and saved-skin radius. Final rejection controls and all three
laws pass a further focused run; no-default-features check passes.

Evidence: `output/qualification/variable-fillet-body-2026-10-19/`.
All three independent OpenCascade STEP results are valid single solids with
7 faces, 15 edges and 10 vertices. Bounds remain [0,2,0] .. [10,8,10] mm.
Measured volumes are 599.7424639068823, 599.9943364914566 and
599.7453779529257 mm3. These are measurements, not an analytic volume claim.
Each result has 130 independent seam/law samples. Maximum position error is
1.445e-9 mm, normal chord 4.413e-16, signed shape tensor difference 3.113e-15,
and ball-center gap 2.043e-9 mm. Native whole-interval proofs are primary;
sampling supplements them. Cumulative 120-file native patch passes apply-check
against the unchanged frozen baseline.

Limits: radii above describe the full original contact-law domain. Retained
endpoint values after trimming are separately recorded by the independent
checker; prescribed radii exactly at the new ends require another construction.
This is not a circular-cross-section curvature-radius certificate for quintic
G2 skins. End faces remain sharp; arbitrary NURBS corner junctions, different
radii meeting at complex vertices, global embedding/thickness/intersections,
WASM/new UI, part acceptance and P0 remain open.


## 2026-10-19: radii prescribed at actual retained ends

`variable_fillet_body::replace_with_end_radii` now accepts radii in caller
end-cutter order and returns a closed body only after fresh saved endpoint-law
interval enclosures pass on BOTH actual contact curves. Every candidate is
rebuilt from immutable originals and passes the full existing root/G2/radius/
closure/injectivity gates. At most eight attempts share cells/exact work.
Floating affine correction only proposes the next full law; it grants no proof.
No nonpositive full-domain extension is clamped into an accepted result.
`ClosedVariableFillet::end_order` retains the original cutter correspondence.

The first implementation failed all eight iterations because changing the full
law mean retraced and clipped a different reference spine each time. The new
native `variable_g2_fillet_reference` fixes the positive numerical reference
radius across attempts. Default tracing APIs retain their old behavior.
Actual contact qualification and C2 preparation are repeated for every proposal.
Diagnostic logs preserve the failed iteration evidence; diagnostic printing was
removed from production before final checks.

Controls prescribe actual new-end radii 0.5/1.5, 0.1/0.2 and 1.5/0.5 mm.
Each converges in two attempts (8013,8197,8024 charged cells). Native endpoint
error bounds are <=8.074e-13 mm. Swapped cutters with correspondingly swapped
radius/retention inputs pass. Negative radii, insufficient work, and a law that
needs a nonpositive full-domain extension refuse without mutating source.
16 focused tests and no-default-features check pass on final source.

Evidence: `output/qualification/endpoint-radii-2026-10-19/`.
Three independent OpenCascade STEP results remain valid single solids with
7 faces, 15 edges and 10 vertices. Independent actual endpoint-law error is
<=8.058e-13 mm, saved ball-center gap <=1.318e-9 mm across 130 seam samples per
body. Volumes are 599.730886238046, 599.9942037034873 and
599.7308862381316 mm3. Numerical measures supplement continuous native proofs.
Cumulative 123-file native patch passes apply-check against frozen baseline.

Limits: controls cover planar support pairs with original extended charts.
Positive affine extension over the full reference trace remains required.
The certificate is a rolling contact-center law, not circular quintic-section
curvature. Sharp end caps, arbitrary NURBS junctions and complex mixed-radius
corners, global thickness/intersections, WASM/new UI, part acceptance and P0
remain unfinished parts of the full goal.


## 2026-10-19: positive local reference window for extreme endpoint laws

`replace_with_end_radii` now proposes a shorter stable reference trace when the
full-domain affine extension would require a negative radius. Window margins
are chosen from the positive requested endpoint radii and proposal slope;
no radius, support point or tolerance is clamped. The next candidate is built
from immutable original surfaces, and roots, trimming, saved G2, rolling-center
law, injectivity and closed-graft admission all run again. Final interval
endpoint checks still cover both saved contact curves. At most eight attempts
share the existing cell/exact budget. The returned `reference_window` records
which subset of the numerical reference trace was used.

Native `variable_g2_fillet_reference_window` restricts only the proposal trace.
Interpolated raw endpoints carry no contact/root proof. All support refinement
and final original-coefficient qualification remain mandatory. Closed or
unusable trace windows produce a failed proposal; invalid input is an error.
Existing full-window APIs retain the same tracing behavior.

The prior three controls still pass. New requested-end pairs 0.01/1.5 and
1.5/0.01 mm now produce admitted closed bodies in two attempts, using windows
[0.12620595138743268,0.8063923822214772] and
[0.18691098466019246,0.8670974154938378]. Charged cells are 8231 and 8108.
Native endpoint-law error is <=1.029e-11 mm. The prior refusal expectation was
replaced by actual successful body/radius/closure controls, not just a catalog
change. Sixteen focused tests pass; no-default-features check passes.

Evidence: `output/qualification/window-endpoint-radii-2026-10-19/`.
All five independent STEP results are valid single solids (7 faces,15 edges,
10 vertices), retain original bounds and pass 130 seam/law samples each.
New endpoint-law errors <=1.029e-11 mm; saved ball-center gaps <=1.320e-9 mm.
New measured volumes are 599.8124401300076 and 599.8124401299245 mm3.
Cumulative 124-file native patch passes apply-check against frozen baseline.

This removes the full-reference positive-extension restriction for the tested
planar pairs. It does not prove all positive radius ratios, arbitrary curved
supports, end-cap G2, circular quintic cross sections, general mixed-radius
corner junctions or global embedding. Those, WASM/new UI and the rest of the
full thread goal remain open.


## 2026-10-19: rational variable plane/cylinder collar proposal; saved G2 unresolved

The general interpolating variable builder was tested against the actual
plane/cylinder end control. Its cylinder contact fails whole-curve positional
admission with a witness gap ~4.845e-5 mm, above 1e-7 mm. No tolerance was
relaxed and that proposal was not admitted.

`end_collar::propose_variable` instead multiplies the original rational circular
Bernstein carrier by a linear original-angle setback law. The degree-3 angular,
degree-5 meridian surface retains positive homogeneous weights. Plane and
cylinder contact curves/pcurves are authored from that same representation;
both full original-support lifts pass native interval verification for
0.03/0.05 and 0.05/0.03 mm. The radius law is anchored before angular padding.
`end_collar_body::replace_variable` drives original-root binding and private
trim/graft assembly, but returns no body until fresh saved G2 passes.

Two concrete assembly issues were investigated. Optional exact position
fallbacks consumed all but 16 operations before the boundary census. Variable
construction now reserves the unchanged total exact budget across bind, end,
graft and final gates. A fixed transverse chart collar also left the original
cylinder chart for small setbacks; variable frames now use an inward margin
from the actual original pcurve hull. Existing constant frames retain their
previous scale. All four previous end-body tests still pass.

The remaining saved cylinder G2 inspection returns Unresolved with no proven
regularity/orientation, not ExceedsTolerance. Current coupled constant-frame
proof does not admit this variable transverse-scale case within the existing
coefficient cap. No successful closed variable-collar result is published.
The attempted generic affine-pcurve simplification did not resolve this and
was removed. Next work is a continuous original-coefficient G2 proof allowing
a transverse scale that varies along the actual trace.

Evidence: `output/qualification/rational-variable-collar-2026-10-19/`.
Five focused tests pass: four existing end-body regressions and one explicit
proposal/contact/refusal regression covering both radius orders. Independent
OpenCascade evaluation samples 130 points per proposal: max position error
6.872e-16 mm, normal chord 2.725e-16, signed shape tensor difference 9.993e-16,
ball-center gap 3.555e-16 mm. These are proposal diagnostics only; they grant
neither a continuous G2 certificate nor B-rep/STEP body admission.
No-default-features check and cumulative 126-file patch apply-check pass.
All general-corner, global diagnostics, WASM/UI and remaining goal work stays open.


## 2026-10-19: continuous affine-chart G2 admits saved variable cylinder collars

The preceding variable-collar G2 refusal is resolved for both current controls.
`surface_collar::AffineFrame` maps the transverse collar endpoint to M*p+offset;
original pcurve poles/weights still define rho=0. Composition uses outward
original Bernstein arithmetic and requires every complete mapped control hull
inside the immutable original chart. Identity coordinate rows retain exact
binary input values rather than introducing artificial boundary overhang.
The new frame can scale a transverse coordinate along the trace without
rounding/refitting an original surface or using sampled derivatives.

`inspect_g2_traces_affine_frames` freshly composes both original collars and
uses the existing paired rational jet/shape proof. Original chart orientation
now requires a strict sign over the whole Bernstein numerator of
(pcurve'(s) x transverse(s)); positive weights supply the denominator sign.
A folded or zero transverse frame cannot acquire orientation from its endpoints.
`inspect_owned_in_affine_frames` repeats actual 3D positions, reversal handling
and private ownership snapshots before the coupled G2 proof. All prior constant
and signed-frame entry points delegate through the same path.

The variable cylinder frame contracts its actual height coordinate toward the
original chart base by 1/4; the skin frame contracts v from 1 by 1/8. This aligns
the actual variable first/second transverse jets while retaining all original
supports. Saved G2 now passes with 219336 coefficient-work units, below the
unchanged 2000000 cap. Production `replace_variable` additionally performs the
whole saved rolling-center law inspection and stores that report in
`ClosedEndCollar::radius_law`; constant results retain None. Incorrect side
proposals cannot bypass the complete actual-normal/center proof.

Both radius orders 0.03/0.05 and 0.05/0.03 mm produce admitted closed bodies.
Total cells are 41389/41364, exact work 999994/999997 (limits unchanged).
The complete radius proof uses original-normal secant bounds and is within
1e-7 mm. Root identities, source surfaces and serialization roundtrip survive.
21 focused tests pass: 5 collar coefficient/affine-frame controls, 4 owned
position/G2 controls, and 12 B-rep corner/end/partition/radius controls.
Adversarial frame tests reject a whole-trace orientation reversal, singular
transverse map and changed target curvature. The positive planar frame control
uses matching variable frames; arbitrary unequal frame scales can still be
conservatively unresolved. No-default-features check passes.

Evidence: `output/qualification/affine-variable-collar-2026-10-19/`.
Independent saved STEP: two valid single solids, each 12 faces,24 edges,14
vertices; original bounds [0,0,-0.2] .. [1,1,0.4] mm and original supports retained.
Volumes are 0.4709842455820291 and 0.47098424556925944 mm3 (measurements).
130 independent samples per body: max position 1.193e-9 mm, normal chord and
signed shape tensor difference 3.992e-14, ball-center gap 1.686e-9 mm,
endpoint radius error 5.690e-16 mm. Sampling supplements native continuous
certificates. Cumulative 126-file native patch passes frozen-baseline apply-check.

This qualifies the variable plane/cylinder end-collar controls, not arbitrary
NURBS/cylinder-cylinder junctions, all positive radius ratios, complex mixed
corners, whole-body G2 or global embedding/thickness/intersections. WASM/new UI,
part acceptance, P0 and P2/P3 remain unfinished parts of the full thread goal.


## 2026-10-19: whole saved-boundary diagnostics and local contact bands

`diagnose_saved_boundary` performs a fresh full face/pair audit from saved Model
JSON and records unresolved, localized, exact, interior-contact and pending
coverage separately. Both variable-collar bodies have 12 proven injective faces
and all 66 face pairs visited, with no pending suffix. Initial diagnostics had
14 unresolved pairs per body. No detected interior contact is not treated as
an absence certificate.

A constant-coordinate defect in proposal authoring was corrected: translate
rational angular products after homogeneous division so algebraically zero
local coordinates keep the authored world plane exactly. Contact restriction
also evaluates and retains coordinates identical at every immutable original
rational pole before fresh root-position/lift checks. These are exact original
constraints, not tolerance snapping; source surfaces are not changed. Literal
z=-0.2 is checked over every saved plane-contact control. This admits a native
owned-edge tube for pair [9,11], reducing unresolved pairs from 14 to 13.

`surface_coordinate_flow::inspect_contact_band` now proves all possible
coordinate-plane contacts lie inside a band: eight ORIGINAL-span position
bounds strictly exclude the other strips, while the complete actual end-pcurve
axis hull is included. A bounded 4x4 ORIGINAL-jet refinement proves monotone
flow and displacement factor inside the retained band. Whole-chart flow may
remain unproved due to a distant turning point. Stable jets are centered, so
world-coordinate exclusions restore the authored span origin; derivatives are
scaled by the original span length, never the smaller cell width. Tests cover
translated nonunit charts, a second translated crossing that must prevent
localization, and exhausted work. The boundary-tube fallback uses this only
when global flow is unavailable, with fresh actual coedge positions and trim
validation. All numerical/exact caps stay unchanged.

Both saved bodies now have 11 unresolved pairs, 8 bounded owned-edge localities,
2 exact vertex pairs, all 66 pairs visited, and zero unproven faces. New end
localities [1,11] and [2,11] use actual shared edges. Pair [0,11] (cylinder/skin)
and ten other pairs remain unresolved. absenceProven and
 toleranceContactLocalityProven remain false; no exact-contact upgrade is
inferred from a metric tube. Final diagnostic cells are 7055 each, domain cells
29622/29652, additional exact work 445062/446359.

Evidence: `output/qualification/constant-variable-collar-2026-10-19/`.
Historical `boundary-diagnostics.json` retains the 13-pair intermediate;
`boundary-diagnostics-local.json` is the final 11-pair audit. 26 focused tests
pass across contact-band bounds, variable/end collars, saved radius, general
fillet contacts, combined self-intersection and qualified-coverage regressions.
Known interior contacts and pending work cannot become positive absence claims.
Both independent saved STEP/G2/radius controls pass after the coordinate fix.
Final no-default-features check and cumulative 129-file patch apply-check pass.

Remaining work includes the 11 pairs, full thickness/embedding, general complex
corners and the rest of the thread objective; native changes remain isolated
and are not WASM/new-UI delivery evidence.


## 2026-10-19: exact original-corner hull certificates reduce pending pairs to one

`exact_vertex_hull::inspect` conditions positive-rational control hulls on exact
supporting coordinate planes. All min/max/equality comparisons select original
binary64 poles; no tolerances, rounded restrictions or sampled separators are
used. Up to four bounded reductions may restrict the possible contact set to
one world point. Admission then requires the SAME topological vertex on both
faces, exact original clamped 3D edge endpoints, exact original pcurve endpoint
ownership at a natural surface corner, and the literal surface corner pole
matching that vertex. The image-at-corner identity follows from the clamped
positive rational basis, not a global approximate edge-lift report.

This local point theorem requires no exact lift of unrelated edges. It does
not authorize an edge contact or infer shell orientation/filled-volume validity.
`inspect_qualified` recomputes these proofs before costlier vertex/tube fallbacks,
charges their pole/topology visits to the shared geometry budget, and records
`exact_hull_vertices` separately from metric localities. Exact absence still
requires all faces injective and complete classification of every pair.

Both control bodies now have 12 exact hull-vertex pair certificates (10 newly
resolved pairs plus 2 previously classified vertices), 8 metric edge localities,
all 12 faces injective, and all 66 pairs visited. The only unresolved pair is
[0,11], the original cylinder versus the variable transition skin.
absenceProven and toleranceContactLocalityProven remain false. Total geometry
cells are 6985 each, domain cells 28704/28734, additional exact work
322696/323985. No work or tolerance cap was raised.

New tests cover the genuine source-corner contact, reversed pair order, rigid
permutation/translation, exhausted work, deliberately shifted original UV
ownership, and an additional surface overlap sharing the same topology vertex.
The latter two cannot acquire a certificate. Full body tests require all
non-cylinder pairs covered and preserve incomplete flags when the cylinder pair
remains. Eleven focused tests pass, including qualified coverage/interior-contact
and combined self-intersection regressions; no-default-features check passes.

Evidence: `output/qualification/vertex-owned-collar-2026-10-19/`.
Saved models are unchanged from the prior constant-coordinate fix; two
independent STEP/G2/radius checks pass again. File hashes bind the copied model,
STEP and final native diagnostics. Cumulative 132-file native patch passes
apply-check against the unchanged frozen baseline.

The remaining cylinder/skin case needs original trimmed-surface contact
classification; nominal circular measurements or topological adjacency alone
cannot close it. Global thickness/embedding, complex corners, WASM/new UI,
part acceptance, P0 and P2/P3 remain open within the full thread objective.


## 2026-10-19 — original ruled-support contact locality (isolated native)

The final cylinder/variable-skin pair [0,11] now has a fresh metric locality
certificate on both unequal-radius controls (.03/.05 mm and reversed). It uses
original rational XY ray coefficients and a source-material affine height
inequality over actual trim-pcurve hulls. Strict source-ray monotonicity bounds
parameter drift. Original Bernstein height residuals exclude the lower skin
band; bounded original derivative jets localize any remaining contact to the
V-max rim. Fresh lifts of both actual coedges to their owned 3D edge and fresh
trim audits are required. No nominal circle, sampled absence, topology-only
admission, tolerance snapping, geometric gap, or budget increase is used.

Both bodies: 12 injective faces, 66/66 visited pairs, zero unresolved pairs,
toleranceContactLocalityProven=true, absenceProven=false. The new pair lies
within 2.437e-8 mm of its actual owned edge (model tolerance 1e-7 mm).
Total geometry cells: 7009 each; domain cells: 28736/28766; new coefficient work:
1384323 each, below the unchanged 2M cap. These bounds permit near-edge contacts;
they do not certify exact absence, global embedding, or whole-body thickness.

Twelve focused native tests pass: conditional ray/height controls and rejection
of insufficient work, nonmonotone ray and reversed height flow; both complete
body/radius-law controls including reversed pair order and displaced interior
edge poles; exact vertex controls; qualified coverage/interior-contact and
combined self-intersection regressions. No-default-features check passes.
Two independent STEP/G2/radius controls pass on unchanged saved models.

Evidence: output/qualification/ruled-contact-locality-2026-10-19/ with full
reports, logs, SHA256 hashes and a cumulative 135-file unapplied native patch.
Patch apply-check passes against the unchanged frozen baseline. No shared
native source, WASM, worker or UI promotion was performed.

Arbitrary B-rep automatic trimming, general NURBS fillet admission, multi-edge
unequal-radius corners, exact contact classification, global thickness and
part acceptance remain open. This closes only the bounded contact-locality
milestone for the current plane/cylinder transition controls.

## 2026-10-19 — atomic independent variable laws at both ends (isolated native)

Added replace_pair_variable: both endpoint-radius laws are anchored before
angular padding, both immutable/private edits must pass, and the FINAL body
freshly rechecks all four original contact G2 seams, both complete rolling-center
laws, skin injectivity and all saved root coordinates/identities. Any second-end
or final gate failure returns no partial body. Existing constant replace_pair
uses the same assembly path with absent radius-law receipts.

The upper variable collar now contracts its source transverse frame toward the
original chart's V-max when its requested axis opposes the source rail. The
lower end retains V-min contraction. These remain proposals; original-coefficient
G2 inspection admits them. The operation preserves original support slots and
literal surface definitions; trimmed faces receive modify-lineage identities,
so final support roles do not search obsolete pre-trim IDs.

Control: lower .03 -> .05 mm, upper .04 -> .02 mm, opposite end axes. Final body:
1 solid, 13 faces, 27 edges, 16 vertices. Four continuously admitted saved G2
seams and both full saved rolling-center laws. Per-stage geometry cells
34251/34350; exact work 999995 each. Separately bounded final inspection:
65757 cells, 410226 exact operations, 461175 coefficient work. Existing caps
100k / 1M / 2M were retained. Input remains equal after success, invalid upper
radius, exhausted upper work, and serialization roundtrip.

The ray-height diagnostic now supports exact height reflection and exact
single-clamped-Bezier source-clock reversal. Original coedge lifts use the
untransformed surfaces. A 2x2 original derivative cover replaces the former
4x4 cover; outward bounds may widen, never become unjustifiably tighter. The
upper control needs both reflected height and reversed source U. A reflected
lower-body adversarial/control regression tests this independently.

Added trimmed_hull_disjoint: original positive rational trim-pcurve pole hulls,
fresh bounded trim audits, and outward original-surface image enclosures over
ALL intersecting knot spans certify a STRICT coordinate gap. It accepts no
nominal-circle or mesh evidence and refuses insufficient work, overlapping
hulls and out-of-chart trim hulls. On the new body [6,10] has z gap lower bound
0.019999998807906646 mm. Four additional plane/skin pairs have tiny positive
original-coefficient gaps; these are exact separation certificates, distinct
from approximate seam closure.

Final diagnostics: all 13 faces injective, all 78 pairs visited, zero unresolved
pairs, no detected interior contacts. toleranceContactLocalityProven=true;
absenceProven=false. Geometry/domain cells 9907/46930, additional coefficient
work 1392015. Metric cylinders/skins remain within 3.763e-8 mm of their actual
owned edges. Locality does NOT prove absence of microscopic intersections or
global solid embedding/thickness.

Independent OCCT STEP check passes: valid solid, matching topology/bounds,
volume 0.4708386030900333 mm3; 260 original saved-seam samples, max position
error 1.193e-9 mm, shape tensor difference 3.992e-14, rolling-center gap
1.686e-9 mm, endpoint radius error 5.690e-16 mm. Sampling complements the native
continuous gates; it is not whole-body G2/embedding evidence.

Evidence: output/qualification/two-variable-ends-2026-10-19/. Full current native
regressions, source manifest and cumulative unapplied patch are retained. This
is still isolated native work. Arbitrary B-rep trimming, general multi-edge
corner construction, global thickness/embedding, WASM/worker/new UI, three-part
acceptance, P0 and P2/P3 remain open under the full goal.

Current qualification: 18 focused tests pass, no-default-features check passes,
and the 137-file cumulative native patch passes apply-check against the
unchanged frozen baseline. No patch was applied to that baseline.

## 2026-10-19 — multiple original cutter roots on one shared rim (isolated native)

Added edge_partition::partition_at_face_roots. Each requested crossing is
freshly isolated against an actual trimmed cutter on the IMMUTABLE original
edge before authoring. Strictly disjoint original parameter bands are required;
repeated/overlapping bands refuse. Requests retain caller indices, independent
of natural-parameter sorting. Right-to-left cuts author shared vertices and
update every incident pcurve. Every FINAL child is checked against the original
3D carrier; no cumulative rounded-parent agreement substitutes for this check.
Each root records the adjacent FINAL children, so two roots share the correct
middle child. Each actually saved vertex is also freshly bounded against its
original root position box, including after exact constant-coordinate retention.

New partition_g2_at_face_roots rechecks parent and EACH final child using fresh
original-surface position, regularity and continuous G2 inspection. Supplied
receipts cannot authorize geometry. Existing 100k cells / 1M exact / 2M G2
coefficient caps, 256 curve controls and 4096 surface controls remain in force.
Failures return no partial candidate. Material-side choice, radius construction
and solid embedding are not inferred by the partition API.

replace_g2_rooted_sides now groups multiple cutter requests by original rim
edge, propagates the complete child chain into mapped side ownership, and
preserves caller root order and final root/child identities. Automatic region
selection still runs from immutable original rim ownership. The new corner
control has five roots on four sides, including two roots on one original edge,
and nine freshly admitted fragment seams. The final closed body has 1 solid,
30 faces, 65 edges and 37 vertices; original support definitions/points and
root-owned child geometry remain intact. Work: 16144 total geometry cells,
184 root-partition cells. General radius-law and global-embedding flags remain
false.

Controls also cover a deformed quintic NURBS carrier, two rational G2 rims with
independent nonunit 3D/pcurve clocks and two curved trimmed cutters per rim,
reversed request order, exact cuboid lifts, duplicate root bands, insufficient
work and invalid cutter data. The existing twenty-cut closed-incidence test
and one-root APIs remain covered. Both final-cut G2 controls pass independent
STEP validity/topology and original-carrier/seam checks: 195 samples each,
max carrier error 1.153e-16 mm, max shape-tensor difference 1.959e-13, max
source-to-partition volume error 1.199e-12 mm3. These are sampled independent
checks supplementing native continuous proofs.

Independent mapped-region STEP check: volume 30.375 mm3 (zero measured change),
297 samples across 9 unique seams, max position error 5.853e-15 mm and tensor
difference 3.320e-14. The verifier additionally checks separated original root
bands and the actual shared middle child for same-edge root groups.

Evidence: output/qualification/multi-root-ends-2026-10-19/. The cumulative native
patch remains unapplied; frozen and shared native/WASM sources are preserved.
This advances general owned trimming and corner assembly. Arbitrary NURBS
fillet/radius construction, multi-valent unequal-radius junctions, whole-body
thickness/intersections, WASM/worker/new UI, three-part acceptance, P0 and P2/P3
remain open under the unchanged full objective.

Final current-source validation: 24 focused regressions, no-default-features
check and three independent STEP controls pass. SHA256 checks bind actual
models, verifiers and current isolated sources. The 139-file cumulative patch
passes apply-check against the frozen baseline and was not applied there.

## 2026-10-19 — affine transverse fields in original-support corner construction

Added surface_collar::construct_in_affine_frame. The rational pcurve is retained
at rho=0 and maps to M*p+offset at rho=1. All original chart-control hulls must
remain inside the immutable natural surface rectangle. Outward homogeneous
Bernstein arithmetic bounds the WHOLE stored rounding error. This construction
receipt only concerns positions; it does not grant G2, radius or solid admission.
The existing constant-direction constructor delegates with the identity M.

Added inspect_g2_in_affine_frame: compose the original source in the affine
frame and compare its coupled jets against the ACTUAL stored unit-Bernstein
candidate controls. The candidate is not recomposed into an unnecessarily
inflated degree. Continuous Bernstein chart-Jacobian sign is mandatory; a folded
transverse field cannot qualify from its endpoint chord. Original support and
candidate remain immutable, work is charged to the unchanged 2M coefficient
cap, and exhausted work leaves Unresolved with no upper bound.

Added corner_blend::mapped::AffineSupport, g2_in_affine_frames and fresh
inspect_candidate_in_affine_frames. All four varying transverse source collars,
original-source G2 seams, candidate seams and full patch injectivity are checked.
Old constant APIs remain available through exact identity-frame conversion.
A curved trapezoid initially exhausted the direct-comparison budget because
candidate recomposition inflated its degree; reading the actual candidate
coefficients directly reduces admitted work to 130688 coefficients / 13 cells,
with no tolerance or cap changes. Maximum direct normal/tensor bounds on that
control are 4.713e-11 / 7.848e-10; the planar counterpart uses 92856 / 13.

B-rep close_mapped_sides_in_affine_frames uses actual original owned side chains
and the existing fresh saved-fragment, trim, closure and naming gates. The
control closes a trapezoid in a planar ring on an open cube shell. Four frames
vary along their sides; no constant endpoint direction replaces them. Final
body: 1 solid, 10 faces, 20 edges, 12 vertices. Every original 3D edge and vertex
(including persistent identities) is unchanged. Body work: 137 cells, 4280 exact
operations, 92856 coefficients. The candidate serializes/roundtrips and refuses
an out-of-chart affine frame atomically.

Tests cover planar and curved original supports, known bilinear trapezoid
correspondence, fresh refusal of changed candidate curvature/position, folded
fields, out-of-chart frames and exhausted coefficient work. Existing curved,
rational, reversed and nonunit mapped-corner controls, root-bound region
replacement, constant/variable two-end bodies and G2 child partitions pass.
The direct-inspector improvement does not change the geometry proposal or
silently average incompatible corner jets.

Independent OCCT checks cover 260 sampled seam sites each on the planar and
curved original-support controls plus 260 sites on the ACTUAL imported STEP
body. The STEP body is valid with the expected topology, unit volume and cube
bounds. Source edges/vertices are checked literally. Sampling supplements the
native continuous proofs. PrescribedRadiusLaw and globalEmbedding flags remain
false: a variable transverse field is not a prescribed rolling-ball radius law.

Evidence: output/qualification/affine-corner-2026-10-19/. Changes remain isolated
native sources plus a cumulative unapplied patch; no shared native, WASM,
worker or new UI promotion. General prescribed-radius NURBS junctions,
multi-valent corners, global thickness/intersections, three-part acceptance,
P0 and P2/P3 remain open under the full objective.

Final current validation: 26 focused native tests, no-default-features check,
two independent original-surface controls and one independent STEP solid
control pass. Hashes bind models and current isolated sources. The 142-file
cumulative patch passes apply-check against the unchanged frozen baseline;
it was not applied there.

## 2026-10-19 — atomic contact chains with fresh radius and G2 admission

Added fillet_radius::inspect_chains_model for actual saved contact fragments.
Each side must use one original support face. Natural domains are sorted and
must meet literally, share actual endpoint vertices, cover the requested
retained interval and align between sides. All skin boundary edges incident to
the selected support are mandatory; a caller cannot admit a hand-picked subset
by shrinking the requested domain. Duplicate edges, omitted pieces, changed
support ownership and unmatched clocks refuse/unresolve. The immutable original
law_domain is retained for EVERY piece; radii are never restarted per fragment.
Constant/linear rolling-contact center laws retain existing support; spline laws
and unmatched partition alignment remain outside current admission.

inspect_qualified_chains_model recomputes complete radius coverage, positions,
regularity and G2 on the same immutable saved model. No external receipt grants
authority. Automatic or original affine transverse G2 frames are available per
support. Radius and saved-fragment G2 share the original 100k-cell / 1M-exact
allowances, with at most 2M additional affine-G2 coefficient work. Each failing
fragment retains its edge index and actual proof in the returned diagnostics;
admitted remains false on any incomplete or exceeded gate.

Added fillet_chain_partition::partition as an actual atomic operation. Both
contact edges use one original natural clock. Sorted distinct interior numeric
cuts author shared vertices and update every incident pcurve privately. Every
FINAL child is freshly checked against its immutable original carrier, then
whole saved-chain radius and all-fragment G2 gates run before a model is
returned. Original source contact identities, final child chains, cut vertices
and work ledgers are retained. Failure returns no partial model; source remains
unchanged. These NUMERIC cut parameters are not cutter-intersection root
certificates and the operation does not choose/remove material sectors.

Control: the .03 -> .05 mm variable plane/cylinder transition is subdivided at
original parameters .35 and .65 on both contacts. Six actual saved fragments
cover three paired intervals. The padded original radius law remains
.0275 -> .0525 on [0,1], while retained coverage stays
[0.10000000000000142,0.8999999999999773]. Combined atomic work: 49247 cells
(24 subdivision, 6 final original-carrier checks, 49217 joint admission),
746281 exact operations and 489465 affine-G2 coefficients. Continuous full-chain
contact-center gap upper bound: 7.804495628817945e-8 mm, below unchanged 1e-7 mm.
All six actual saved G2 fragments are freshly admitted. Serialization reload
runs fresh combined admission again; prior flags/snapshots are not imported.

Adversarial controls reject a wrong constant radius, restarting the original
law clock, omitted/repeated fragments, a partial declared interval, changed
piece clocks, damaged edge geometry, exhausted work and invalid numeric cuts.
A stronger counterexample preserves contact positions and first transverse
jets but changes second curvature jets in the rational skin: homogeneous
perturbation vanishes at actual trim-end parameters and both contact positions.
The ball-center law still passes, but the combined saved G2 gate refuses it.
Thus a radius-center certificate is never treated as a curvature/G2 certificate.
No constant circular skin cross-section claim is made for the G2 quintic.

Contact diagnostics initially regressed to one unresolved cylinder/skin pair
after subdivision because the ray/height metric certificate expected one edge.
It now freshly lifts the UNION of actual shared V-max fragments, bounds all
inter-fragment gaps/end overhangs outwards using original boundary speed, and
records every owned edge in the certificate. A missing shared middle edge
cannot gain admission from a separate geometrically identical copy on an open
diagnostic shell. Final complete boundary diagnostics: 12 injective faces,
66/66 pairs visited, zero unresolved pairs; metric locality true, exact absence
false. The [0,11] certificate owns all three cylinder/skin fragments and bounds
contact distance to their union by 3.245116963708363e-8 mm. Diagnostics use
7335 cells, 33445 domain cells and 462723 additional coefficient operations.

Independent OCCT STEP check: valid solid, 12 faces / 28 edges / 18 vertices,
unchanged original surfaces and unedited edges/vertices, volume change
1.132e-11 mm3. Across 390 actual saved-seam samples: original-carrier deviation
4.997e-16 mm, position error 1.193e-9 mm, shape-tensor difference 4.010e-14,
rolling-center discrepancy 1.686e-9 mm. Per-piece samples also lie below their
native continuous center-gap bounds. STEP validity and samples do not grant
exact global embedding, whole-body G2 or thickness.

Evidence: output/qualification/radius-chains-2026-10-19/. Work remains isolated
native plus an unapplied cumulative patch. General cutter-clock synchronization,
prescribed-radius NURBS junction synthesis, multi-valent corners, full thickness
and exact intersections, WASM/worker/new UI, three-part acceptance, P0 and P2/P3
remain open under the full unchanged goal.

Final current-source evidence: 21 focused native regressions, no-default-features
check, fresh complete boundary diagnostics and independent STEP acceptance pass.
Hashes bind actual models, verifier and all current isolated source differences.
The cumulative 146-file patch passes apply-check against the frozen baseline
and remains unapplied. No full-goal or delivery completion is claimed.

## 2026-10-19 — paired original contact roots on actual trimmed cutters

Added fillet_chain_partition::partition_at_faces. Each cutter request recomputes
complete original curve/surface root coverage and actual trimmed-face membership
for BOTH owned contact curves before any authoring. No supplied root/receipt is
accepted. Caller-selected retained crossings must exist, preserve their order
on both contacts and have strictly interior, separated original parameter bands.
Bands touching an existing endpoint refuse with an explicit ownership-reuse
requirement. Reusing such endpoints remains a separate incomplete path.

The original curves give DISJOINT root bands even on a radial plane: their
saved representations differ slightly within the existing authoring tolerance.
The adapter therefore retains each actual band's own representative. It never
widens root bands, snaps original weights or forces a common parameter. Final
actual vertices are bounded independently against both original root-position
boxes. The immutable original law map is retained on each contact's own clock.
The private paired-parameter subdivision path then freshly qualifies every
final carrier, actual incident use, radius interval and G2 fragment before
returning a model. Queries, subdivision and final admission share existing
100k geometry cells / 1M exact / 2M affine-G2 coefficient limits.

Whole-chain admission now accepts independently partitioned piece clocks only
through complete original center-law bounds. Piece reports record BOTH actual
domains. Missing pieces, changing support ownership, reordered roots and
uncovered intervals remain inadmissible; no correspondence is inferred from
spatial proximity. The adaptive normal proof evaluates each original radius
map on its own actual curve interval.

For compatible original Bernstein bases, a proof-only correlated difference
retains the first original weights and adds an explicit whole-interval bound
for the second curve's weight change. With p_i=wa_i B_i/Wa and r_i=wb_i/wa_i,
B_wb-B_wa is bounded by (range(r)/min(r))*max_i||b_i-b_0||. The bound is
translation invariant, uses outward ORIGINAL ratios, and is added to all upper
bounds and subtracted from witnesses. No original curve or weight changes.
Large bounds fall back to independent original enclosures. A dedicated control
checks independent knot clocks, unequal weights, dense original evaluations,
source immutability, large perturbation refusal and exhausted work.

One common rolling sphere must also use ONE radius. Independently trimmed
linear clocks add the outward radius mismatch |dr/dt|*max_endpoint|ta-tb| to
the center error; a mismatch beyond tolerance remains unresolved. The extra
unit-normal correction bounds the second contact's center using the first
contact's original radius. A counterexample with coincident centers of unequal
prescribed spheres cannot pass. General nonlinear clock synchronization and
spline radius laws remain outside current admission.

Positive control: two actual finite radial cutter faces on the .03 -> .05 mm
plane/cylinder transition. One pair of authored parameters is
[0.35000000000002274,0.3500000000925638]; each stays inside its OWN original
certified band. Maximum clock difference is 9.255e-11, never an enlarged root
box. Saved vertex error against original root-position boxes is <=1.482e-13 mm.
The final body has six fresh radius/G2 contact fragments and unchanged original
surfaces; fresh combined admission passes again after serialization reload.
Total work: 49600 cells, including 227 original root-query cells; 682513 exact
operations and 489465 affine-G2 coefficients. No tolerance/cap was increased.

Negative controls reject cutters whose roots are inside actual trim holes,
existing-endpoint root bands and incompatible original radius-clock pairings.
Source and cutter models remain equal after success and refusal. Boundary
inspection still covers all 66/66 pairs on 12 injective faces with zero
unresolved pairs; metric contact locality is true, exact absence remains false.

Independent OCCT STEP/original-data acceptance checks the actual final solid,
12 faces / 28 edges / 18 vertices, original carriers, every saved fragment,
common-radius sphere centers, original root bands/boxes and actual finite
cutter UV bounds. Across 390 seam samples, maximum center discrepancy is
1.686e-9 mm; radius-clock mismatch is 2.314e-12 mm. Volume change is
1.132e-11 mm3. The saved vertices also satisfy their actual cutter plane and
surface coordinates independently. These checks supplement native continuous
proofs; they do not prove global embedding or whole-body thickness.

Evidence: output/qualification/paired-contact-roots-2026-10-19/. Isolated native
work and its unapplied patch remain separate from shared native/WASM/UI sources.
The full objective is unchanged: general NURBS unequal-radius junction synthesis,
multi-valent corners, existing endpoint reuse, general clock maps, global
thickness/exact intersections, WASM/worker/new UI, three-part acceptance, P0 and
P2/P3 remain open.

Current evidence: 24 focused regressions and the endpoint guard pass;
no-default-features compilation, complete boundary diagnostics and independent
STEP/root/center checks pass. Hashes bind every current isolated source diff,
actual saved model/cutters and verifier. The 148-file cumulative patch passes
apply-check against the unchanged frozen baseline and remains unapplied.


## 2026-10-19 — existing endpoint reuse preparation

Native isolated checkout only. Paired root partitioning now distinguishes existing
owned endpoints from interior cuts. It requires both original root bands to contain
the same natural endpoint, retains the original vertex coordinate and TopoId, and
bounds the actual final vertex against both freshly isolated root-position boxes.
Mixed endpoint/interior correspondence still refuses. No endpoint snapping or root
band inflation is used. Empty numeric cuts now freshly qualify all unchanged actual
contacts with the original radius law and G2 gates.

Validation: 9 focused tests pass; no-default-features check passes. The unchanged
contact test explicitly verifies all vertex coordinates, vertex identities, edge
count and fresh admission. The endpoint cutter control still refuses earlier with
`Every original root and actual cutter membership must be classified`: positive
endpoint reuse is NOT qualified. This does not establish arbitrary B-rep material
trimming, general unequal-radius corners, global thickness or exact absence of
self-intersections. WASM/UI not promoted.

Evidence: `output/qualification/endpoint-reuse-2026-10-19/`. Cumulative isolated
native patch covers 148 changed files and passes apply-check against the untouched
frozen baseline; it was not applied there or to shared main.


## 2026-10-19 — exact authored endpoint incidence and contraction uniqueness

Isolated native implementation removes the strict-interior-existence restriction
for a proved class of original endpoint intersections. Fully clamped single-span
curve/surface corners use literal original pole equality. Bilinear rational
surfaces additionally admit original parameter fractions 0, 1/2, 1 on each axis:
the common nonzero tensor Bernstein factor cancels, and at most eight exact
binary64 products per coordinate establish the weighted point equation. The
fixed prepass is bounded by 432 product terms and reports its actual count.
It neither samples residuals nor modifies original poles, weights or root bands.

Existence alone never admits a root. Original interval homogeneous Jacobians
still require the unchanged norm bound ||I-RJ|| < 0.5 throughout a convex cell
and an invertible preconditioner with proved orientation. The segment-integral
argument proves uniqueness even on a cell boundary. Reports distinguish this
proof from strict Krawczyk interior inclusion. Outward local and natural
parameter maps retain the exact site, including nonunit knot domains.

Validation: 11 core tests plus 7 B-rep regressions pass; no-default-features
check passes. Eight weighted original corner controls cover both curve
orientations and all four surface corners on nonunit domains. A weighted
bilinear interior endpoint control passes; a smallest-subnormal displacement
does not acquire the exact endpoint proof. Tangency, coincidence and exhausted
work remain unresolved. An actual owned cuboid edge endpoint is certified
inside a freshly audited cutter trim, with both complete Models unchanged.

Scope remains incomplete: arbitrary interior UV incidence on higher-degree
NURBS, the original variable-collar endpoint cutter fixture, general material
region trimming and unequal-radius complex corners. No WASM/UI promotion,
STEP acceptance, global thickness or exact self-intersection-absence claim.
Evidence: `output/qualification/exact-endpoint-roots-2026-10-19/`. The cumulative
152-file native patch passes apply-check against the untouched frozen baseline
and remains unapplied to shared main and the baseline.


## 2026-10-19 — owned endpoint and interior chains with fresh G2 admission

Isolated native `edge_partition::bind_face_roots` supports a batch containing
existing exact endpoint roots and interior roots on one immutable original
shared edge. Endpoint incidence is explicit (one final child); interior
incidence names both final children. No duplicate sentinel child or new vertex
is created at the existing endpoint. Requests retain caller order; overlap and
repeat bands refuse. Every original cutter is freshly queried against its actual
trim. Endpoint reuse requires `ExactAuthoredEndpointWithContraction`, not a
strict-interior box which merely touches the natural endpoint. The same guard
was added to paired fillet contact root binding.

Interior subdivision uses the existing atomic all-face operation under the
remaining shared cell cap. Final original-carrier and every actual incident
child pcurve lift are rechecked. All stored root vertices must remain within
their original root-position boxes under the unchanged model tolerance. Source
endpoint coordinates and TopoId are retained literally.

`bind_g2_face_roots` additionally requires fresh parent and every final child
original-surface position, regularity and G2 inspection under unchanged shared
cell/exact/coefficient caps. Controls cover unchanged endpoint-only ownership,
mixed interior/end requests on a cuboid, duplicate refusal, exhausted budget,
invalid support refusal, and an actual smooth transition rim with both an
endpoint root and a separate interior cutter. Saved-model roundtrip and native
watertight inspection pass. Eight B-rep regressions and no-default-features
check pass.

Independent OCCT inspection of source/final STEP: valid single solids, 11 faces
each, edges 21 to 22, vertices 12 to 13. Volume difference
1.7680301667155618e-14 mm^3. Across 130 samples, original-carrier error
3.1031676915590914e-17 mm, surface position error 4.2207643840401616e-17 mm,
normal error 1.5700924586837752e-16, world curvature-tensor error
5.329070518200751e-15. Samples supplement native continuous proofs; they do
not prove global embedding, thickness or whole-body G2.

Remaining: original variable-collar endpoint fixture, general NURBS endpoint
incidence, paired contacts with unequal subdivision correspondence, general
material-sector trimming, unequal-radius complex junctions, WASM/UI and full
three-part acceptance. Evidence: `output/qualification/owned-endpoint-chains-2026-10-19/`;
independent checker `scripts/verify-owned-root-chains.py`. Cumulative 153-file
native patch passes apply-check against frozen baseline and remains unapplied
to shared main, baseline and WASM.


## 2026-10-19 — independent saved contact windows and common-radius admission

Isolated native `partition_independent` accepts separately authored cuts on the
two actual contact chains. Complete saved chain ownership and original natural
domain coverage remain mandatory. Unequal subdivisions are overlaid in proof
space only: contiguous normalized windows cover each unchanged saved edge. The
original law domain is never restarted. Same-count corresponding pieces retain
the existing direct proof; a failed correspondence can retry a full window
overlay, charging all previous cell/exact work against the same original caps.

Window radius bounds evaluate original curves, actual support pcurves and
original surface normal traces. Each adaptive leaf bounds both radius clocks
and includes their maximum difference to prove a single common rolling-sphere
radius. Leaf windows form continuous complete covers; they do not author
matching geometry. Every unique actual full seam receives fresh position,
regularity and G2 inspection once, even when it participates in multiple proof
windows. This is a rolling-contact-center law, not constant circular curvature
of the G2 skin cross-section.

For bounded single-Bezier windows, outward homogeneous de Casteljau coefficients
construct proof-only rational proxies. Positive-weight residual and weight-ratio
covariance bounds include all proxy rounding. Adaptive endpoint mapping error
is bounded by 8*epsilon times an original rational derivative bound
4*p*wmax/wmin*max|pi-p0|. The correlated difference bound includes both source
proxy errors; original model poles, knots and weights remain unchanged. Other
original curves retain the direct conservative trace fallback and can remain
unresolved at the existing budget.

A regression exposed a necessary distinction: a whole unchanged body may pass
an alternative overlay while a caller's requested cutter-root pairing does not.
Paired cutter-root binding therefore still requires its own full-piece
correspondence and explicitly rejects an unrelated successful overlay. The
previous incompatible x=0.8 cutter control remains a refusal with this reason.

Validation: 22 integration tests and 5 radius unit tests pass; no-default-features
check passes. Actual variable-collar controls cover two pieces against one and
two pieces against two with different cut locations. Incorrect laws remain
unaccepted. The two-against-one operation uses 49,640 cells, 415,598 exact work
and 219,336 affine G2 coefficients under the unchanged 100k/1M/2M caps.

Independent OCCT STEP control: valid single solid, unchanged 12 faces, edges
24 to 25 and vertices 14 to 15. Volume difference 1.914246539058695e-12 mm^3.
Across 130 common-sphere samples, original-carrier error
4.599733051526907e-16 mm, surface-position error 1.19209309090945e-9 mm,
normal and curvature-tensor errors about 3.9914e-14, center gap
1.6858739027535423e-9 mm, radius-clock drift 6.938893903907228e-18 mm.
Independent samples supplement native continuous proofs, not global embedding.

Remaining: general NURBS qualification, unresolved original variable-collar
endpoint cutter, requested nonmatching root-end assembly, arbitrary material
trimming, unequal-radius complex corners, thickness/self-intersection completion,
WASM/UI and the full three-part/P0 acceptance. Evidence:
`output/qualification/independent-contact-windows-2026-10-19/`; checker:
`scripts/verify-independent-contact-chains.py`. The cumulative 154-file native
patch passes apply-check against the frozen baseline and remains unapplied
to shared main, frozen sources and WASM.


## 2026-10-19 — original shared corner endpoints survive region replacement

Isolated native `replace_g2_owned_rooted_sides` carries explicit endpoint and
interior root incidence from actual cutter queries through full mapped-region
replacement. Every saved root is remapped using the replacement's source
vertex/edge tables. Original vertex coordinates and TopoId, retained child curve
definitions and identities, and endpoint orientation must survive literally.
No two-child sentinel represents an endpoint. Legacy interior-only result
fields remain available through `replace_g2_rooted_sides`.

`replace_g2_owned_rooted_region` first recomputes the removable topological
component from the declared retained side chains, then performs the same atomic
replacement. This is topological region discovery, not arbitrary material-side
classification. Any root, trim, original-carrier, saved G2 or closure failure
returns no partial candidate. Original work caps remain unchanged.

Control: five independently isolated interior roots on four curved sides, plus
two exact endpoint roots on adjacent original edges sharing one original corner.
The two endpoint queries use separate transverse cutters and retain one shared
source vertex identity; no duplicate vertex is created. Explicit and automatic
region replacement produce the exact same saved Model as the five-interior-cut
control. Seven bindings, five new vertices, nine freshly checked G2 seams.
Saved Model roundtrip passes. Operation uses 16,364 cells, including 404 root
partition cells; topological discovery visits 241 records. Nine integration
regressions and no-default-features check pass.

Independent OCCT STEP result: valid single solid with 30 faces, 65 edges and
37 vertices. Volume 30.375 mm^3, zero reported difference from the analytic
control volume. Across 297 seam samples: maximum surface position error
5.852215564642961e-15 mm, normal error 6.495753533316291e-15 and curvature
tensor error 3.319566843629218e-14. The checker verifies both endpoint bindings'
original coordinates and identities and counts only five new interior vertices.
The older interior-only evidence also passes the updated checker on a temporary
copy; its original evidence remains unchanged. Samples supplement native
continuous G2 proofs, not global embedding or a general radius-law certificate.

Remaining: general exact NURBS/cutter endpoint incidence, nonmatching root-end
transition geometry, prescribed unequal-radius complex corners, material-sector
classification, thickness/self-intersection completion, WASM/UI and full
three-part/P0 acceptance. Evidence: `output/qualification/owned-root-region-2026-10-19/`;
checker: `scripts/verify-rooted-region.py --automatic`. The cumulative 154-file
isolated native patch passes apply-check against the frozen baseline and
remains unapplied to shared main, frozen sources and WASM.


## 2026-10-19 — unequal circular-radius transition retains split contact chains

Current isolated native owned-band tests first revalidated the 0.1 mm transition
and both original-support circular G2 joins; the older unresolved result is not
the current state of this bounded operator. `replace_owned_band` now accepts
independently split complete circular sides. Canonical affine UV fragments are
ordered by increasing U, cover [0,1] literally, and meet at the same owned
vertices. No curve merging, weight replacement, approximate vertex matching or
tolerance expansion supplies coverage. Original radial sides still require one
edge each. Existing representative `smooth_edges` remain; `smooth_chains`
records every retained circular fragment.

Each complete original support receives its transverse G2 check, and EVERY
actual saved fragment separately receives original 3D curve / both pcurves /
actual support and skin position, regularity and G2 admission. Actual curves,
vertices and their identities are retained. Original trims and closure are
freshly checked before returning the private candidate. Only the two radial
profile edges and three selected face charts change. Circular joins claim G2;
radial/end joins remain sharp. This is a coaxial circular-radius transition,
not a general NURBS rolling-ball fillet or complex corner junction.

Validation: nine integration tests and no-default-features check pass. New
controls split one circle twice and the other once (three versus two fragments),
cover radius orders 0.1 to 0.3 and 0.3 to 0.1 mm, and a rotated/translated frame.
All five saved contact fragments, three existing cut vertices and their IDs
survive replacement and Model roundtrip. A 0.1001 mm request for a fixed 0.1 mm
owned band still refuses. A 5e-10 UV gap/overlap below Model::validate's 1e-9
preliminary UV chain tolerance does not acquire exact complete-chain admission.
No caller tolerance was increased. Fragmented control uses 307 cells, 204,897
exact work and 538,689 G2 coefficients under the original 100k/1M/2M caps.

Independent default-frame STEP control: valid single solid, 11 faces, 24 edges,
15 vertices. Volume 0.011440973240253834 mm^3 versus analytic quintic-law volume
0.011440973246027764 mm^3 (error 5.773929945274148e-12 mm^3). 507 station/radius
samples and 50 complete-surface seam samples pass. An additional 325 actual
fragment/pcurve seam samples have position error 2.9925865470192585e-16 mm,
normal error 8.93858920912155e-15 and curvature tensor error
2.1439372004630252e-13. Material probes independently change inside-to-outside
and outside-to-inside at the expected locations: this edits the transition
geometry, not only its metadata. The unsplit STEP control and a legacy manifest
without the new optional chain fields also pass updated verifiers. Independent
samples supplement the native original-coefficient proofs.

Remaining: partial angular material selection, split radial/end ownership with
new end geometry, general NURBS fillets, prescribed unequal-radius complex
corners, thickness/self-intersection completion, WASM/UI and full three-part/P0
acceptance. Evidence: `output/qualification/fragmented-owned-radius-2026-10-19/`;
verifiers: `scripts/verify-owned-radius-band.py --actual-owned` and
`scripts/verify-radius-transition-occt.py`. The cumulative 154-file isolated
native patch passes apply-check against frozen baseline and remains unapplied
to shared main, frozen sources and WASM.


## 2026-10-19 — lower shared contact tube via exact skin V reversal

Fresh whole-boundary diagnostics of the fragmented unequal-radius control found
11 injective faces and one unresolved pair [0,3] among 55 visited pairs. The
ray/height certificate previously localized contacts only to skin V-max; this
actual three-fragment circular boundary is at V-min.

`ruled_contact_tube::inspect` now also tries exact normalized V reversal of a
nonperiodic single clamped Bezier skin. Only original control columns and weight
columns are permuted in a proof-local clone. Knots remain unchanged, including
nonunit domains; no rounded a+b-v map, fitted geometry or widened root band is
authoritative. Shared coedges are selected on the corresponding ORIGINAL V-min
side, and all lifts and trim audits still use the actual untransformed surfaces
and pcurves. Height reflection remains an exact sign change. Certificates
record `skin_v_reversed`; legacy V-max trials retain their original order.

Original union coverage and every actual shared fragment lift remain required.
A geometrically identical but separately owned middle edge on an open diagnostic
shell cannot replace the missing shared interval. A [2,5] skin V domain passes
without modifying the input model. Zero proof budgets yield no certificate.
No limits or tolerances were increased.

After: all 55 pairs are visited, unresolved pairs empty, all 11 faces injective
and tolerance contact locality proved. The lower certificate owns actual edges
[2,22,21], uses height sign -1 and exact skin V reversal, and bounds possible
contacts to within 1.2500002200720008e-8 mm of the owned edge union under the
unchanged 1e-7 mm model tolerance. Work: 16,633 geometry cells, 84,314 domain
cells, 562,810 fallback coefficients and 27,644 additional exact work. Ten
integration regressions and no-default-features check pass, including the
existing detected-interior-contact and incomplete-budget distinctions.

This remains a METRIC locality certificate: exact absence stays false and no
global embedding, thickness or complete general NURBS diagnosis is inferred.
The control Model is byte-identical to the previous independently STEP-checked
fragmented transition. Before/after reports and model hash are retained in
`output/qualification/lower-contact-diagnostics-2026-10-19/`. Remaining scope
includes exact boundary-contact classification, global thickness, general
NURBS/unequal-radius complex corners, WASM/UI and full three-part/P0 acceptance.
The cumulative 155-file isolated native patch passes apply-check against the
frozen baseline and remains unapplied to shared main, frozen sources and WASM.


## 2026-10-19 — exact composed contact-chain classification

New isolated native `shared_boundary_chain::inspect` proves contacts of two
positive, single-clamped original NURBS charts restricted to a complete owned
natural boundary union. Original Cartesian poles strictly separate the two
interiors by a constant-coordinate plane. Each natural side has complete literal
UV coverage, no overlap/gap and shared endpoint ownership. Every actual stored
3D fragment must exactly equal BOTH actual original surface/pcurve compositions:
`curve_surface_agreement::verify_exact` outcomes must be Equal under the caller's
remaining exact-work budget. Positional tolerance never supplies this identity.
Fresh bounded trim audits and positive pcurve control hulls keep all material
parameters inside the original natural rectangles. Shared lookup is indexed,
with 1024 bounded uses and 64 boundary fragments under existing caps.

Qualified self-intersection diagnostics now retain exact contact-chain proofs
separately from metric localities, and use them for exact boundary-chart absence
only after every original face is injective and every pair is covered. Failed
exact attempts are fully charged before existing vertex/tube fallbacks. Raw
initial pair reports remain initial reports; the qualified ledger names the
additional exact boundary-chain classification. No caller certificates enter
this path.

Control: two curved rational profile strips (original weights [1,0.5,1]) share
a boundary cut into two exact halves. Four actual curve/surface lift identities,
strict interior separation and both chart injectivity checks yield exact absence
of forbidden boundary-chart intersections for this OPEN two-face control. This
is not a filled-solid or general NURBS claim. Nonunit [2,6] clocks and incident
U/V axis transposition also pass. One-ULP and smallest-subnormal changes to a
stored edge remain sampled-valid but cannot acquire an exact identity or global
exact absence. A geometrically coincident separately owned half-edge likewise
cannot replace shared topology. Zero/exhausted work cannot supply a proof.

Thirteen unique integration tests across the focused regression and added axis
control pass; no-default-features check passes. Full qualified exact control work:
1,138 geometry cells, 9,489 domain cells and 12,848 exact work
under the unchanged 100k/1M/1M caps. `scripts/verify-exact-boundary-chain.py`
independently reconstructs original binary64 values as exact fractions, verifies
four rational composition polynomial identities, positive Bernstein derivative
coefficients for injectivity and strict halfspaces, without samples or epsilons.
It separately rejects the one-ULP and subnormal controls.

The rounded circular closed-body control remains METRIC-only: all 55 pairs are
covered with no unresolved pairs, but exact absence stays false and no exact
chain is fabricated from rounded edge restrictions. Remaining scope includes
arbitrary supporting planes, nonplanar shared boundaries, exact representation
of general trimmed curves, global thickness, general unequal-radius corners,
WASM/UI and full three-part/P0 acceptance. Evidence:
`output/qualification/exact-boundary-chains-2026-10-19/`. The cumulative 157-file
isolated native patch passes apply-check against frozen baseline and remains
unapplied to shared main, frozen sources and WASM.


## 2026-10-19: original-anchor oblique shared-boundary certificates

The isolated native classifier now admits an additional sufficient class: two
positive rational Bezier charts whose complete shared natural boundary lies in
an oblique supporting plane. Three noncollinear ORIGINAL boundary poles define
the plane. Exact orient2d/orient3d predicates establish coplanarity and strictly
opposite off-boundary poles; no rounded normal or fitted plane supplies proof.
All actual owned 3D fragments must still equal both original pcurve/surface
compositions exactly, cover the natural boundary fully, and share original
endpoint vertex ownership. Failed candidates consume the same bounded work.

The new dyadic-shear control has two injective faces, two shared fragments and
four exact rational lift identities. Full qualified inspection proves absence
on these boundary charts using 1,074 cells, 10,072 domain cells and 28,273 exact
work. The independent Fraction verifier confirms original binary64 polynomial
identities and halfspaces without samples. A one-ULP stored edge change, an
off-boundary pole crossing the plane and exhausted work refuse admission.
Reverse face order and the earlier coordinate-plane verifier remain covered.
This is an open two-chart control, NOT a filled-solid certificate.

The rounded closed-body regression still visits all 55 pairs over 11 injective
faces with no unresolved pairs and metric locality true; exact absence remains
false. Fresh counters: 16,986 cells, 84,347 domain cells and 63,128 additional
exact work. The new fallback does not fabricate exact identities for rounded
circular restrictions. Existing numerical and work tolerances are unchanged.

Scope still excludes collinear anchor boundaries, nonplanar shared boundaries,
arbitrary B-rep automatic contour discovery, general unequal-radius corner
construction and whole-body thickness. WASM/UI and full part/P0 acceptance
remain outstanding. Evidence: `output/qualification/oblique-boundary-chains-2026-10-19/`.
The cumulative 158-native-file patch passes apply-check against the frozen
baseline and remains UNAPPLIED to shared main, frozen sources and WASM.

Fresh final regression: all 14 integration tests pass across shared boundary
chains, qualified self-intersection, owned radius transitions, variable collar
closure and fillet radius chains; no-default-features compilation passes.


## 2026-10-19: automatic roots on authored side chains

A new native transaction removes the caller's crossing-ordinal hints. The caller
still authors four rim-side chains and each side's actual trimmed cutter faces.
`discover_side_roots` inspects EVERY saved edge against EVERY cutter of its side,
retains all complete query reports, and publishes a request list only when all
queries are complete. It never merges roots by position. Duplicate edge
ownership and duplicate cutter definitions refuse. Bounds remain 16 fragments
and 16 cutters per side, 60 total roots, 100k shared geometry cells, and the
existing construction/exact/coefficient limits. Queries consume shared work;
root binding repeats actual checks and charges that repeated work separately.

`replace_g2_discovered_region` joins discovery to component selection, original
shared endpoint binding, full side-chain propagation and closed G2 region
replacement. Incomplete discovery publishes no candidate Model. The positive
control automatically finds seven requests: five interior roots and two uses of
one ORIGINAL shared endpoint. The resulting Model equals the independently
requested explicit-root reference, including saved coordinates and identities.
Discovery uses 193 cells; the complete transaction uses 16,557 cells. The new
split-chain test visits all four combinations of two original saved fragments
and two cutters, finds both retained roots, and excludes both other queries.
An additional coincident cutter leaves coverage unresolved after a successful
query: no partial root list escapes. Exhausted work similarly publishes no model.

Nine native integration tests pass across region replacement, multi-face roots,
exact boundary chains and split-chain/ambiguity discovery; no-default-features
compilation passes. STEP is exported from the ACTUAL new transaction result.
Independent OCP validates one solid, 30 faces, 65 edges and 37 vertices, volume
30.375 mm3 (zero measured error for this control), and 297 samples over nine
actual saved seams. Maximum position/normal/shape-tensor differences are
5.8523e-15 mm, 6.4958e-15 and 3.3196e-14 respectively. Samples supplement the
native full-interval proofs and do not replace them. The legacy STEP verifier
also passes on a COPY of historical evidence; original evidence is preserved.

This is automatic root discovery on AUTHORED side/cutter assignments, not
arbitrary B-rep contour discovery or general radius-law construction. The
control remains a mapped paraboloid replacement; general unequal-radius corner
bodies, nonplanar trimming coverage, global embedding/thickness and WASM/UI
acceptance remain outstanding. Evidence:
`output/qualification/discovered-side-roots-2026-10-19/`. The cumulative
160-native-file patch passes apply-check against frozen baseline and stays
UNAPPLIED to shared main, frozen native sources and WASM.


## 2026-10-19: infer complete original region rims from retained supports

The new native `discover_region_boundary` removes manual edge enumeration for
an EXISTING partitioned source region. Given an original face seed and retained
support faces, it discovers the full connected component in original shell
adjacency with those supports removed. It reconstructs every boundary arc from
opposite two-face incidence and original vertex identity. Every declared support
must touch the resulting rim. Complete loops, including multiple loops, are
returned with selected/retained face ownership and original oriented vertex
ports. Forks, open ports, duplicate supports, ambiguous shell ownership and
exhausted topology work refuse. No proximity welding or geometry samples choose
the boundary. Caps remain 100k topology visits, 256 selected faces and 64 original
boundary fragments. This theorem is topological, not a geometry embedding proof.

The mapped adapter accepts only one simple loop with four distinct supports,
each owning one contiguous side of at most 16 fragments. `replace_g2_from_supports`
integrates this discovery with automatic root discovery, component revalidation,
shared endpoint binding and final closed G2 region admission. Every repeated
component query is charged to the SAME remaining topology allowance. No edge
list or root ordinal is supplied to this entry point. Source-face selection is
still explicit: the seed chooses which material component is intended.

The actual control infers four original arcs from seed face 29 and retained
faces [3,14,7,10], then discovers all seven root requests and produces the same
saved Model as the explicit-rim/reference construction. Discovery uses 309
topology visits, full transaction 550 topology visits and 16,557 geometry cells.
Seven fresh integration tests pass, including the closed transaction, original
split-edge ports, whole-shell orientation reversal, multiple-loop preservation,
root ambiguity/refusal and multi-face root regressions. No-default-features
compilation passes. An independent checker reconstructs complete original
oriented incidence and component connectivity; missing-edge and wrong-owner
controls both refuse. A two-loop region remains two loops and cannot be admitted
as a single mapped cap.

STEP is exported from the ACTUAL inferred-rim transaction. Independent OCP
validates one solid, 30 faces, 65 edges and 37 vertices, volume 30.375 mm3 and 297
samples across nine saved seams. Samples supplement native seam proofs; they do
not prove general G2/embedding. Evidence:
`output/qualification/inferred-region-boundary-2026-10-19/`. The cumulative
162-native-file patch remains isolated and unapplied. Remaining work includes
creating previously absent intersection contours on arbitrary B-rep supports,
general unequal-radius corner geometry, whole-body diagnostics, WASM/UI and
full three-part/P0 acceptance. Existing-loop topology discovery does not close
those requirements.


## 2026-10-19: automatic open intersection contours without chart-end hints

`face_cut::partition_automatic_open` now builds NEW saved intersection curves
from the original pair of surfaces without caller-authored chart boxes,
end-chart indices, end-side flags, root ordinals or trace coefficients. It
requires fresh whole-original-product discovery and separated component proofs,
then enumerates natural-boundary end-chart candidates. Every candidate must
pass internal root ports, distinct natural endpoint enclosures and coherent
original arc construction. Failed candidates consume the same allowance.
All components must succeed: no partially completed branch set escapes.

A fresh whole-product census checks the proposed arcs. Actual original B-rep
roots are then bound, actual world curves/pcurves are constructed, and a SECOND
whole-product refinement checks every saved root-cover target. Only after that
coverage succeeds does the actual source face partition occur. Every source
sector remains; this is not material removal or a fillet/radius construction.
Stages report cells, exact work, evaluations and traversal visits. Opaque root
binding/partition refusals reserve their entire offered work rather than making
that spent allowance available again. Original 100k cells/1M exact work/4096
evaluations/100k visits are unchanged. The code handles 1..128 open components,
with the existing 2..256-chart port support per component. Interior trim endpoints,
closed/singular components and incomplete products remain explicit refusals.

Eight native integration tests pass, including all of the following actual
controls: symmetric and asymmetric polynomial cutters, a rational cutter,
a source chart with transposed parameters, and TWO curved original supports.
The three body controls find two components and partition the original top
face into three sectors. Full cells/exact work are respectively
36,031/29,728; 39,133/29,080; and 28,873/29,272. The curved/curved sheet finds one
component and partitions into two faces, using 11,107 cells and 999,994 exact
work; its exact budget is nearly exhausted and no performance claim follows.
Original source/cutter snapshots, original vertices/identities and codec
roundtrips are checked. Exhausted search/discovery and a closed component
publish neither traces nor a partial partition. No-default-features check passes.

Independent OCP checks all four saved controls. The three closed STEP bodies
are valid single solids retaining 0.25 mm3. Each actual exported contour curve
is matched against the actual saved curve and checked along its interval;
maximum sampled saved-to-STEP difference is 1.111e-16 mm. Original source lifts
and actual cutter distances are sampled independently (544/544/442 samples).
The curved/curved OPEN sheet is checked directly from original/saved geometry
at 1,037 points; maximum actual cutter distance is 3.440e-12 mm. It is NOT a STEP
body: the exporter explicitly refuses its open shell. An altered actual saved
pcurve fails independent support checks. Samples supplement native continuous
and complete-product proofs; they do not establish global G2/thickness.

Evidence and actual outputs: `output/qualification/automatic-open-contours-2026-10-19/`.
The cumulative 164-native-file patch passes apply-check against frozen baseline
and stays UNAPPLIED to shared native sources, frozen sources and WASM. Remaining
work includes contours ending on arbitrary interior trims, mixed open/closed
and singular branches, general unequal-radius corner bodies, global result
qualification, WASM/UI and full part/P0 acceptance. The full goal remains active.


## 2026-10-19: automatic actual interior-trim endpoints with fresh port reanchoring

`partition_automatic_trimmed_open` now prepares actual original boundary roots,
rechecks the complete ordinary boundary census, discovers complete original open
components, orders all actual trim roots on separated branch carriers and binds
adjacent root pairs WITHOUT caller endpoint/root/chart hints. Every pair still
passes the existing independent original-source binding theorem. The plan alone
is never authority to omit material: every actual saved segment must pass fresh
construction, junction/endpoint bands and whole trim membership, followed by
complete original root-family coverage INCLUDING outside-trim witnesses and the
full ordinary boundary-root census. Only then is the actual face partitioned.
Stored vertices remain tolerance-bounded authored representatives of interval
roots; no algebraic root is snapped or promoted to an exact coordinate claim.

The first real control exposed a previous search artifact: exact surface
parameter 3/8 lay on a fixed 3/8 subdivision plane, leaving valid transverse
roots unresolved. Curve/surface search now uses its available Krawczyk image to
PROPOSE a subdivision away from that image. Both closed children still cover
the original cell, actual rounded subdivision ratios remain outward enclosed,
and all original strict Krawczyk / exact authored-endpoint acceptance gates are
unchanged. All 12 native curve/surface tests pass, including the new 3/8, 5/8,
1/4, 1/2 and 3/4 controls, both transverse roots/signs and low-work refusal.
Tangency and original boundary exclusions retain their existing proof criteria.

A second real refusal placed trim endpoints at internal computational ports.
New `reanchor_open_arcs` moves ONLY numerical internal-port proposals inside
original certified bridge domains and freshly proves internal ports, original
natural ends and ordered arcs. The chosen fraction is stored and used on full
independent replay. Actual source definitions, original root ownership and
geometry tolerances do not change. The automatic driver tries the original
ports followed by bounded 3/8 and 5/8 proposals. Failed opaque pair attempts
reserve their offered traversal work rather than reusing spent allowance.
Duplicate/missing branches, overlapping root clocks, odd root counts and low
work refuse without exposing a partial pair/trace/Model set.

The actual body control expands only the top support chart while its physical
outer trim stays strictly inside that chart. The native transaction discovers
and binds FOUR actual interior-trim endpoints, constructs two saved chains,
and splits the top into three retained sectors. The initial port proposal
really refuses; fresh reanchoring is exercised. Full work is 44,757 cells,
36,720 exact work and 25,322 CHARGED visits (including a reserved 25,000-visit
failed offer; this is not a measured runtime count). Original 100k/1M/4096 work
caps and model tolerance are unchanged. Fourteen B-rep tests pass across this
new path, the prior automatic open path, multi-face shared roots and exact
boundary chains. Low cells/exact work/visits expose neither traces nor partial
partition. New ports independently replay; duplicate or missing branches refuse.
No-default-features compilation also passes.

Actual-result STEP independently validates as one solid, 8 faces, 48 edges,
42 vertices and volume 0.25 mm3. All 32 exported saved contour edges are matched
to actual source coefficients and sampled at 544 points: maximum saved-to-STEP
error 7.851e-17 mm, original-face lift error 4.441e-16 mm and actual cutter
projection distance 2.522e-10 mm. The independent check also verifies actual
endpoint UVs are strictly inside original natural chart bounds. An altered
saved pcurve fails. The earlier natural-boundary control passes on a COPY of
its evidence; historical evidence remains intact. These samples supplement
native continuous and complete coverage gates, and imply no global G2/thickness.

Evidence: `output/qualification/automatic-trimmed-open-2026-10-19/`. Cumulative
167-native-file patch passes apply-check and stays UNAPPLIED to shared main,
frozen sources and WASM. This qualifies separated interior-chart roots on the
actual outer trim. Joining an outer loop to a hole still requires general loop
assembly; closed/singular/mixed branches, fully general rational surfaces,
unequal-radius corner bodies, global result diagnostics, WASM/UI and full part/P0
acceptance remain outstanding. The full thread goal remains active.


## 2026-10-19: atomic outer-to-hole networks with winding conservation

`partition_loop_network` performs ALL saved cuts in one private transaction,
including roots on hole boundaries. The previous sequential path could only
accept two outer-loop endpoints; its first outer-to-hole cut creates a
weakly-simple slit that cannot pass the existing simple-region gates. The new
path builds a complete successor permutation: at each uniquely owned cut end,
original boundary and cut traversals exchange successors. Final cycles are
extracted only after ALL paths are installed. A repeated cut edge in one cycle
refuses as an unpaired slit. No intermediate weak boundary is published.

Every original boundary use is retained exactly once. Each new cut uses its
same actual 3D edge with opposite pcurves. Those pcurves must be single clamped
positive Beziers whose original Cartesian controls and weights reverse exactly.
Thus cut winding cancels identically, independent of parameter clock. Source
and final loops undergo bounded continuous simplicity/winding audits; negative
cycles must have exactly one proved positive outer owner. Every final region
is positive with disjoint holes. Their indicator sum equals the original
region off boundaries by exact oriented-curve cancellation; this supplies UV
partition conservation without sampled angle routing or material-center seeds.
Actual saved forward AND reverse 3D/surface lifts are separately checked under
the original world tolerance. This does not prove global surface embedding.

The typed saved-trace path dispatches to the atomic network when a root belongs
to a hole. Actual original root identities and support definitions are checked.
Existing boundary 3D edges/vertices/identities survive verbatim; only new curve
pieces and descendant face loops are added. There are no abandoned loop slots.
Caps remain 256 original boundary uses, 128 paths, 256 pieces per path, 1024
combined uses, 256 controls per curve, 4096 surface controls, 100k cells, 1M
exact work and 10k trim pairs. Unsupported/ambiguous networks return no Model.

The real through-hole control exposed closed membership touching an outer or
hole boundary: generic Boundary-band classification cannot certify the whole
curve. `FaceDomain` now proves a query rectangle lies in a literal rectangular
outer closure AND outside every OPEN Cartesian hole control-hull box. Positive
rational basis and closed-loop winding confine the entire hole interior to that
box. Equality at its supporting line is permitted for closure; one ULP into
the hole is refused. No tolerance band grows. Hole-free literal-rectangle
fast-path identity remains unchanged; holed faces are never presented as a
whole rectangle. Nonrectangular/overlapping hull cases still need other proofs.

Twenty-two integration tests pass: the real automatic hole network, earlier
automatic open/trimmed paths, original face-cut regressions, direct loop-network
controls and three cap-with-hole regressions. The direct test cuts an annular
region into four sectors, then repeats with an untouched second hole; a 256-point
interior occupancy check supplements the native winding proof. Original and
prepared edge/vertex definitions and identities remain unchanged. Slits,
duplicate ends, exhausted work, hole-interior/outer-exterior one-ULP queries and
incorrect saved pcurves refuse. No-default-features compilation passes.

The actual extrusion-with-through-hole transaction automatically finds TWO
original intersection components but FOUR retained chains (the middle portions
are inside the hole). It finds all EIGHT real trim endpoints, builds 24 saved
curve edges and produces four top sectors. Work: 42,420 cells, 33,972 exact work,
25,546 charged visits and 300 evaluations, under unchanged limits. It preserves
the original body and all original/prepared root coordinates and identities;
codec roundtrip equals the actual result.

Independent STEP validates one solid, 13 faces, 56 edges and 44 vertices. Each
of the eight actual root vertices is shared by THREE edges and THREE faces,
including neighboring walls; no proximity-created duplicate is accepted.
Source STEP volume is 0.189453125 mm3, result 0.18945312500000047 mm3, measured
change 4.71845e-16 mm3. All 24 actual exported contour curves are matched with
saved geometry and checked at 408 points: maximum saved-to-STEP difference
7.851e-17 mm; original-face lift 3.886e-16 mm; actual cutter distance 2.522e-10 mm.
These samples supplement native whole-interval and ordinary boundary-root
coverage, and do not establish global G2/thickness/self-intersection absence.
The prior hole-free interior-trim control also independently passes on a COPY.

Evidence: `output/qualification/atomic-hole-network-2026-10-19/`. Cumulative
171-native-file patch passes apply-check and remains UNAPPLIED to shared main,
frozen sources and WASM. Qualification still needs connected multiple-hole
networks, general curved/nonrectangular trim membership, shared/tangent root
nodes and singular/closed mixed components. General unequal-radius corners,
whole-body diagnostics, WASM/UI and full part/P0 acceptance remain outstanding.
The full goal is active.


## 2026-10-19: connected multiple-hole bodies and rational open-component cover

Fresh native end-to-end controls now connect TWO and THREE actual through holes
in closed extrusions; a third control uses two holes and an original rational
cutter (original positive weight 1.001). No root ordinal, trace coefficient,
manual edge list or selected-sector seed chooses the cuts. Complete original
boundary/root/trim coverage precedes the actual atomic loop network. The two-hole
cases produce SIX retained chains, TWELVE shared endpoints and FIVE top regions;
three holes produce EIGHT chains, SIXTEEN endpoints and SIX top regions. The
root segments between holes are retained; intervals inside holes are excluded
by native original trim witnesses. Original and prepared edge/vertex definitions
and identities survive; full model codec roundtrip is equal to the result.

The rational control reproduced a real refusal: original charts touched in
one parameter but their component connection was unproved. The final discovery
fix freshly tightens only unknown coordinates on the SAME complete original
free interval. Every successful tightening is a nested uniform original root
graph, hence names the same family by uniqueness. Extension proposals then
contain that tighter chart and require another fresh uniform proof. Failed
proposals spend the same original ledger. No root range, geometry tolerance,
Krawczyk acceptance threshold or work cap is loosened. Final original-product
coverage remains mandatory, so neither tightening nor an extension proposal is
itself authority to omit a source root.

That complete rational graph had redundant charts with no coherent all-chart
construction order. `simplify_open_component_cover` proposes short graph paths
between natural end charts, then performs a NEW COMPLETE ORIGINAL-PRODUCT census
on the retained actual fibers. Only a successful cover and native component
proof can remove a chart. Existing original transitions transfer with exact
source/domain/report matching, and independent replay reconstructs the cover
and every transition. Failed subset proof retains the original cover and charges
its spent work. Missing coverage or low work never becomes a reduced proof.
The actual rational graph has 30 charts; six are removed after a fresh 3,306-
rectangle proof, retaining both original components. Low traversal work refuses.
The old component connector's orientation/acceptance algorithm remains intact.

Thirty-one tests pass: 17 B-rep and 14 core coverage/component tests. They include
all three new actual bodies, independent graph/reduced-cover replay, earlier
open/interior-trim and cap-with-hole regressions, loop conservation and low-work
controls. No-default-features compilation passes. Native cells/exact work/charged
visits are: two holes 44,666 / 40,864 / 25,836; three holes 47,778 / 48,720 / 26,060;
two holes with rational cutter 65,616 / 35,440 / 25,640. Charged visits include
reserved failed root-pair offers, rather than being wall-clock measurements.
All existing 100k/1M/4096 caps are unchanged.

Independent STEP validates all THREE actual results and their source STEP
bodies, matching actual exported contour curves to saved coefficients. Shared
root counts are 12/16/12, each one vertex on three edges and three faces including
neighboring walls. Actual STEP surface Euler characteristics are -2/-4/-2,
confirming two/three/two through holes rather than silently capping them. Native
source hole counts, retained regions and complete prepared identity prefixes
are checked separately. Measured volume changes are 1.055e-15, -2.776e-17 and
-5.552e-17 mm3 respectively. Maximum actual cutter projection distances stay
below 2.548e-10 mm. Samples supplement native continuous original-root/trim
coverage and do not prove general global G2, thickness or self-intersections.
A modified rational saved pcurve refuses independent validation; the earlier
single-hole control still passes on a COPY of its evidence.

Evidence: `output/qualification/connected-hole-networks-2026-10-19/`. The cumulative
172-native-file patch passes apply-check against frozen baseline and remains
UNAPPLIED to shared main, frozen sources and WASM. Qualification covers these
rectangular connected-hole controls, not arbitrary curved/slanted trims,
tangencies, shared branch nodes or mixed closed/open singular components.
General unequal-radius corners, whole-body diagnostics, WASM/UI and full part/P0
acceptance remain outstanding. The full thread goal remains active.


## 2026-10-19 — atomic automatic trimmed-face batch (native, bounded)

Added `face_cut::partition_automatic_trimmed_batch` in the isolated native candidate.
Up to 16 distinct selected original faces share 100,000 cells/visits and
1,000,000 exact work. Each successive contour discovery uses the current owned
model, including earlier shared-edge splits. Only the complete candidate escapes;
refusal leaves diagnostics without intermediate topology. Caller models remain
immutable. Original chart and persistent identity are checked before each face.
This keeps all material sectors. It is not arbitrary-solid Boolean removal or a
constructor for general unequal-radius corners.

New regressions cover a successful one-face transaction, first-face success
followed by refusal (no published model), invalid/duplicate selections and source
immutability. A successful adjacent multi-face control is still required before
claiming shared-edge batch qualification. Existing closed/singular intersection
refusals remain. No new general-corner, global self-intersection, WASM or UI claim.

Evidence: `output/qualification/atomic-trim-batch-2026-10-19/`.
The cumulative candidate patch is checked against the frozen baseline without
applying it; native work remains isolated from the main runtime.

Validation: 23 tests passed across automatic hole/open/trimmed contour, end-collar and owned-radius transition suites. `brep-core --no-default-features` passed. Cumulative 173-file candidate patch passes frozen-baseline apply-check; it remains unapplied.


## 2026-10-19 — shared original roots and an atomic closed graph trim

This supersedes the preceding sequential batch design. All selected faces now
use ONE registry of original boundary roots, prepared before any face partition.
Roots are not rediscovered on rounded child edges. A deterministic face schedule
makes reversed selection produce the same saved geometry and identities.

`partition_automatic_trimmed_batch` prepares complete original root-family traces
per face and admits their common partition atomically. The new
`trim_automatic_graph_region` also selects the material side and admits the closed
original-cutter cap within the SAME 100,000-cell / 1,000,000-exact-work allowance.
A missing rim face, unresolved classification or insufficient work publishes no
intermediate model. Material diagnostics refer to original source face identities.

Three proof improvements are included:

- One-sided original-fiber extension can bridge a turning region without forcing
  its opposite end outside the natural root family. The existing successful
  symmetric/tightened atlas remains the first choice. Every extension and final
  complete original-product cover is freshly proved.
- Original rational-curve trim membership uses a complete adaptive interval
  cover. A control polygon leaving the trim does not alone reject a curve wholly
  inside it. Hole crossings, a one-ULP excursion outside and exhausted work still
  refuse. No curve coefficients or trim tolerances are changed.
- A near-unit parent contraction is tightened on the WHOLE unchanged free
  interval before trace construction. Fresh nested unique-root proofs preserve
  the same original family. The rational regression with contraction
  0.999967262236405 now constructs and reverifies its complete trace using 70
  cells and 12 numerical evaluations. Local pieces retain the original world
  tolerance and fresh source proofs.

Trim coverage reuses native complete parent images when already covered or
proved wholly outside the actual trim. The final original-product refinement
remains mandatory. Graph material extrema use a conservative enclosure of the
entire actual outer-loop control hull on the ORIGINAL source surface. Strict
membership in the actual cutter trim and original model tolerance remain required.

### Current evidence

- 40 B-rep regressions, 19 core regressions and 2 original-curve membership tests
  passed (61 unique tests). The no-default-features check passed.
- Adjacent top/side faces use the same two original root vertices. Both selection
  orders pass independent STEP checks, preserve volume and produce equal saved
  geometry and identities.
- The three-face public atomic operation uses 92,750 geometry cells, 60,728 exact
  work, and 216 domain cells. It discovers the complete contour, selects the
  positive material sectors and admits one closed body: 8 faces, 50 edges and
  44 vertices. All four shared original roots survive as single STEP vertices.
- Independent STEP volume is 0.12435159362333702 mm3; the integral of the specified
  polynomial cut gives 0.12435159362789563 mm3 (difference 4.56e-12 mm3). Across
  612 boundary samples, maximum original-lift error is 2.98e-11 mm. Samples
  supplement the native whole-interval proofs.
- Fresh STEP controls also pass for two and three through holes and a rational
  cutter through two holes. The six positive STEP controls, a legacy verifier
  compatibility control and two deliberately corrupted evidence controls pass
  their expected checks. Altered saved geometry and false root ownership refuse.

Evidence: `output/qualification/shared-original-trim-batch-2026-10-19/`.
The cumulative 174-file native candidate patch passes apply-check against the
unchanged frozen baseline and remains UNAPPLIED. Changed source hashes were
reverified after packaging. Main native code, WASM and the UI are not promoted.

Scope remains bounded: the closed material-removal control uses affine selected
source faces and a polynomial coordinate-graph cutter. Rational cutter partition
coverage is qualified separately. Common dispatch for closed/singular components,
general NURBS material classification, unequal-radius corner surfaces, whole-body
G2/thickness/self-intersections, WASM/worker/UI and the three-part history/STEP
acceptance remain part of the active objective.


## 2026-10-19 — preserve and requalify all six circular G2 seams

Added `end_collar_body::replace_pair_preserving` and
`replace_pair_variable_preserving`. Callers name original protected seam edges,
incident source faces and transverse frames. The final edited body must retain
those edge/endpoint identities, actual curve coefficients, original surface and
pcurve definitions, and shell orientations. Every protected seam then undergoes
fresh owned-position, regularity and full-interval G2 inspection. No earlier G2
flag is transferred. Protected inspections share the existing FINAL work receipt
with the four new end contacts; per-edit and final caps remain unchanged.

The 0.1 mm band control exposed a fixed half-chart transverse step leaving the
original cylinder chart. Constant collars now use their actual available chart
margin. A transverse gain proposed from the actual stored tangents aligns the
proof frames; the original-coefficient inspector remains the admission authority.
The four numerical proposal queries are charged to the existing cell allowance.
Saved geometry and position/normal/curvature tolerances are unchanged.

Current evidence:

- Eleven native regressions passed: six end-body, four owned radius transition,
  and one variable fillet body test. The no-default-features check passed.
- The source body has cylindrical radii 0.1 / 0.3 mm and central width 0.1 mm.
  Both end edits preserve two middle G2 seams and freshly qualify four end
  contacts. The FINAL six-seam audit uses 251 cells, zero exact work and 443,643
  coefficient work. Each construction stage retains its existing separate caps.
  The actual unchanged central radius law is also independently requalified
  under a separate native radius-transition allowance.
- Independent STEP: one body, 13 faces, 27 edges, 16 vertices. All six seam fields
  pass 774 sampled comparisons supplementing native continuous proofs. Maximum
  position error is 1.20e-9 mm, normal chord error 1.06e-14, and world curvature
  tensor error 2.15e-13 /mm. Meridian-volume reference difference is 3.15e-11 mm3.
- A second control has independent nominal rolling-center laws [0.03,0.05] and
  [0.04,0.02] mm at its two ends. The protected result preserves the baseline
  body exactly, both final saved laws pass native checks, and independent STEP
  checks cover all six G2 seams (390 samples including both radius laws).
- Native admission refuses an unchanged sharp seam requested as protected G2,
  duplicate roles, and the existing wrong-role / exhausted-work cases. The
  sharp negative reaches the final G2 guard, rather than merely failing input
  validation. Independent deliberately forged protection of that sharp seam
  fails with a normal difference of sqrt(2); changed protected curve coefficients
  also fail. Source models remain immutable.

Evidence: `output/qualification/protected-six-seam-body-2026-10-19/`.
The cumulative 175-file native candidate passes frozen-baseline apply-check and
remains unapplied. Packaged changed-source hashes were reverified. This is native
qualification, not WASM/worker/UI delivery.

The body retains sharp angular walls. These controls do not establish general
NURBS fillet construction, mixed-radius multi-edge corners, whole-body G2,
minimum wall thickness or global absence of self-intersections. The full active
objective, including UI/history/STEP acceptance, P0 and P2-P3, remains open.

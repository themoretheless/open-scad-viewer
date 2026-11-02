# Source-backed algebraic removal with independent floors and side walls

Rust API under `minkowski3d::curved`:
`certify_model_pocket(model, source_face, floor, domain, factors, window, selection, depth)`
and `verify_model_pocket(model, floor, certificate)`.

The result is a checked **AlgebraicModelPocket**, a new native algebraic B-rep
backed by the original Model. This is a geometrical removal operation: the edited
face excludes the chosen region, an independent original NURBS floor closes it,
and new ruled wall faces connect every exposed CAD boundary arc. Rational queries
refuse the deleted original-face interior and evaluate the new floor and walls.

It is NOT an ordinary legacy Model. The old Model type has rational curve/surface
payloads and cannot generally encode these algebraic wall charts exactly. The
`source_model()` getter is explicitly the INPUT geometry for retained face/edge
references, not an unchanged Model presented as the result. Consumers must use
`faces()` and the checked native predicates. `into_inputs()` permits editing and
recertification. No approximate legacy conversion is supplied.

## Domain selection and exact wall geometry

The source polynomial D and two exact window guard polynomials participate in a
complete CAD partition. The guard signs prove that every selected open cell is
inside the requested window, and its closure is in the closed window. Explicit
selection may include ANY sign of D, including individual cells, not only whole
positive components. Default selection chooses positive-D cells inside the window.
The operation uses regularized unions of cell closures, avoiding spurious
zero-dimensional material left by a strict inequality at an isolated zero.

Unlike the earlier direct cap-pair operation, D does NOT need to be the height
difference of the two surfaces, and does NOT need a globally nonzero gradient.
The CAD proof isolates all critical and singular events. Open root sections and
critical-fiber intervals are exact embedded arcs; selected boundary cycles must
be closed and nonbranching. Singular input polynomials can therefore define valid
cornered cuts. Diagonally touching selections that produce a nonmanifold boundary
are refused by the resulting topology checks.

A wall over an arc gamma is exactly
(1-t)*upper(gamma)+t*floor(gamma), t in [0,1]. The arc remains an algebraic root
selector or a critical-fiber segment. In general this ruled carrier is not a
rational NURBS surface. Top/bottom edge selectors and vertical edges at algebraic
vertices are explicit native entities. No fitted control polygon or rounded
algebraic vertex is used. A positive original/floor height gap over the whole
window makes each wall noncollapsed.

Retained face components are reconstructed from all unselected CAD cells. One
component extends to the original face exterior and retains its original trim
loops; bounded islands become separate retained faces. Each floor component
retains its full oriented boundary cycles, including holes. Every original edge
is preserved. Each new or original edge must have exactly two opposite uses;
every face cycle also checks exact abstract endpoint connectivity and closure.
Native face loops are outward shell traversals, with reversal already applied.

## Geometric embedding and material ownership

The original Model must receive a complete checked source atlas. It currently
must have one connected outward closed outer shell and no inner shells. Genus
and nonconvexity are unrestricted by a convex-support test; source-atlas proof
and existing arithmetic budgets still apply. Nominal signed volume is used only
to identify orientation, not claimed as exact curved mass properties.

Only the EDITED carrier and floor currently require affine XY graph projections.
Their independent UV charts are aligned by exact affine coefficient identities.
Other original faces may be nongraph NURBS carriers. The tests retain all four
rational cylindrical wall patches unchanged. The common quadratic deformation
is not required.

The closed guard window is proved to lie inside the original AUTHORED face:
all original pcurve knot cells must avoid it by exact positive-weight control
hulls and bounded subdivision, and a rational centre point receives a checked
original-curve winding certificate (inside outer wire, outside each hole).
Incomplete pcurve trim domains and nonzero periodic lifts are explicitly refused.
This argument covers curved trim boundaries, not polygon parity.

Exact common-degree homogeneous Bernstein controls bound the upper and floor
over the window. Positive denominator controls give a rational spatial box for
the complete swept volume. Every other original NURBS carrier must be strictly
separated from that box; its original knot cells are checked by bounded exact
subdivision and control hulls. This is conservative: untrimmed supports that
cannot be separated may cause an unproved result even if their trimmed face is
actually disjoint.

The upper face must face outward along +Z. Each vertical segment just below it
starts in the original material. The swept pocket interior meets no original
boundary: the edited graph has only one height per projected point, and all
other carriers are strictly excluded. Connectedness then places the entire
swept interior in the original material. The result boundary is precisely the
retained original boundary, the lower cap, and the ruled side walls.

In the common domain, affine XY projection is injective and the height gap is
strictly positive. The swept map is thus an explicit embedding of each selected
region times [0,1]. Exact CAD cycle topology, opposite edge uses and source
separation establish the new closed embedded boundary without assuming a
sampled genus or an approximate Boolean. Irrational strata remain symbolic.

The certificate binds the full original Model, original floor, selected face,
domain, exact factor witnesses, window, original source atlas, trim winding,
normalized UV correspondences, spatial bounds, complete partition and native
result faces/edge uses. Verification reconstructs all of them. It shares the
Rust solver with the producer; it is replayable checking, not a separate theorem
prover. A legacy source-atlas certificate is NOT claimed for the implicit result.

## Remaining requested generality

Nongraph EDITED/floor carriers, nonlinear inverse UV correspondences, arbitrary
multi-cell edited carriers, simultaneous multi-face cuts, cuts crossing original
trim edges, multiple outer shells or existing cavity shells, and unconstrained
general NURBS Boolean removal remain open. This is a local +Z pocket operation,
not a universal arbitrary-body Boolean. The source-backed native type is not yet
integrated into legacy Model operations, UI/MCP, STEP or WASM.

All prior bounds remain: degree/bit/work/isolation limits, original source-atlas
depth 0..3, finite source controls, at most 11 supplied domain factors, and at most
12 levels for trim and other-carrier separation. Unknown geometry is refused.
Cylindrical source qualification uses depth 1; the initial depth-0 refusal is
retained as a diagnostic and was not bypassed by relaxing the predicates.

## Validation

Focused regressions cover actual deleted-face queries and curved floor/wall
queries; source preservation; forged wall orientation and changed-floor refusal;
a nonconvex L-shaped original Model; retained nongraph cylinder patches; an
annular cut with a retained top island; negative-sign cell selection; a valid
single quadrant at a singular crossing; refused diagonal pinches, empty cuts,
invalid guard windows and floors crossing the original bottom boundary.

See result.json, native-tests.log and branch-tests.log. The same copied native
executable is used for the full regression checks. Scoped source hashes are not
a frozen manifest of every transitive file in this shared workspace.

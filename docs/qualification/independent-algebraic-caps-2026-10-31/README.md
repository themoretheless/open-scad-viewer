# Independent NURBS carriers with native algebraic cap boundaries

This milestone adds `AlgebraicRegionBrep` and the Rust APIs
`certify_graph_region_brep`, `certify_graph_region_brep_factored`, and
`verify_graph_region_brep` under `minkowski3d::curved`.

It constructs closed solids bounded by regions of two independent original
rational NURBS graph surfaces. Their height functions need not share a quadratic
or any common polynomial deformation of polyhedral source planes. Original UV
charts may differ: an exact affine correspondence aligns the two projected
charts. Its full coefficient identity and inclusion in the lower original cell
are checked. Transposed UV orientation is supported explicitly.

This is a NEW native algebraic B-rep type. It does not pretend that an implicit
intersection is a rational NURBS Curve, and does not convert to the legacy Model
by fitting, tessellating or rounding an algebraic seam. `carriers()` retains the
original NURBS surfaces; `into_carriers()` permits source editing followed by
recertification. Cell selection is also an input to recertification. Existing
Model editing/rendering/serialization is not automatically integrated.

## Geometric certificate

Each original source is one nonperiodic tensor knot cell with positive homogeneous
weight controls. Its XY projection must equal an affine map coefficientwise,
with a nonzero determinant. The original carrier is therefore a global graph
on its parameter square; the common domain is a normalized closed triangle.
The inverse affine correspondence maps this triangle into the other original
cell. Stored source powers, source hashes and UV maps bind the original surfaces.

Let upper and lower heights be Zu/Wu and Zl/Wl in common coordinates. The source
intersection equation is F=Zu*Wl-Zl*Wu, computed in exact rational arithmetic.
Positive denominators make F's sign the actual height ordering. The complete
existing cylindrical algebraic decomposition (CAD) provides symbolic critical
vertices, root sections and two-dimensional sign-invariant cells. Its isolating
intervals are witnesses, never replacement coordinates.

The caller supplies a polynomial M. The checker verifies strict positivity of
Fu²+Fv²+M*F on the whole unit square using exact Bernstein bounds and bounded
subdivision. Hence grad F is nonzero wherever F=0. It also verifies F<0 on all
three triangle sides. Consequently positive regions are compactly contained in
the domain, and top/bottom carriers intersect transversely along their boundary.
No claim about unsupported singular seams is inferred from a few sample points.

Selected cells must have strictly positive F signs. Adjacent selected cells
cancel their oriented internal edges. Each exposed edge must have a selected
positive cell and an unselected negative cell: continuity then proves F=0 on the
COMPLETE common arc. Selection cutting through a positive component is refused,
because a separate side cap would be needed. Boundary vertices must have one
incoming and one outgoing arc, giving closed nonbranching oriented cycles.

Each connected selected region becomes a body with an upper and lower cap face.
Their boundary coedges use the same exact CAD edge selector in opposite directions.
Holes remain additional boundary cycles. Positive height ordering separates cap
interiors; common affine projection gives injectivity of each carrier. Regular
F=0 and exact shared boundary cycles give a closed embedded cap pair. Different
positive components have disjoint closures under the same regularity proof.
Source-chart orientation and affine UV orientation control outward face reversal.

Rational point queries run exact CAD stratum location and evaluate the original
homogeneous powers in the corresponding carrier. Queries outside the selected
face fail. The API does not supply rounded algebraic-vertex coordinates or claim
that rational point queries cover irrational parameters.

## Factor witnesses and checking

An optional factor list must multiply to the COMPLETE original F, coefficient
for coefficient, before it can affect any decision. The triangle boundary
factors are included as well. Each factor is made primitive and square-free;
common factors are removed by exact polynomial division. Projection includes
factor leading coefficients, discriminants, pairwise resultants, vertical
contents and domain boundary events. This is the exact discriminant product
identity. No root, resultant or branch is dropped due to a numerical estimate.
At each critical fiber the new path uses the smallest proved projection factor
with one root in the globally isolated interval; rational singleton roots use
their exact linear defining polynomial. Every such factor was included in the
global projection by exact polynomial union, so this changes the arithmetic
field rather than the root census. This avoids unrelated algebraic factors
inflating intermediate fractions.

The old monolithic branch recipe retains its original arithmetic and output;
only the common final isolation/incidence routine is extracted unchanged.

The certificate contains both source hashes, exact UV correspondences, height
equation, regularity multiplier, optional factor witness, full branch/cell
partition, selected cells, body components, oriented cap loops and seam uses.
Verification reconstructs these from the original surfaces and compares them.
It shares the Rust solver with the producer; it is replayable verification,
not an independently implemented theorem prover or a universal geometry proof.

## Explicit remaining scope

This is NOT arbitrary deletion from an existing closed B-rep. It constructs the
closed cap-pair solid over selected positive components. Regions that reach the
triangle boundary, arbitrary partial cells requiring independent side caps,
non-graph carriers, nonlinear projection inverse maps, multi-knot-cell source
carriers, singular seams, global Boolean classification against arbitrary
nonconvex container walls, and legacy Model/UI/MCP/STEP/WASM integration remain
outside this result. The earlier common-quadratic API is preserved.

Bounds remain explicit: source degree 1..8, at most 1024 controls, effective
source cut degree at most 8 in each parameter, joint polynomial degree budgets,
at most 13 supplied factors, 8192-bit rational coefficient guards, bounded CAD
arithmetic/isolation, and at most 12 Bernstein subdivision levels. Unknown or
exhausted work is refused. A multiplier or factor witness is not an unchecked
user assertion.

See result.json and logs for measured native validation. Hashes cover the scoped
files; they do not freeze all concurrent changes in this shared workspace.

# Original material graph neighborhoods at ordinary vertices

The Rust implementation is in `source_seam_tangency::vertices`, with separate
original-corner, material-collar and exact-fan modules. Its frozen native
qualification passed at f046834ca; see QUALIFICATION.md and result.json.
This document states the proof conditions, not a full-objective completion claim.

The intended owner borrows two private certificates of the **same** original
material body: regular Ck face interiors and material-oriented exact edge jets.
The body separately owns closed embedded source geometry, complete incidence,
material orientation and nonzero volume. A pointer identity check rejects
independently admitted bodies. No caller diagnostic report supplies authority.

For every original vertex, the complete owned vertex-link cycle determines its
declared order. Any explicitly C0 incident edge leaves that vertex explicitly
C0. A smooth candidate is never downgraded after a failed proof. Its requested
order is at most the face-interior order and the exact incident edge classes.

For each smooth candidate the proposed construction requires:

1. Every original incident corner is enumerated from the immutable shell and
   matched bijectively to one adjacent pair of the owned link cycle.
2. Both incident original UV traces are complete natural sides of the same
   original clamped chart. Endpoint enclosures must be exact singletons at its
   natural endpoints; a rounded algebraic root is insufficient.
3. A positive-width parameter rectangle is anchored at that exact corner.
   Every other original UV contour is proved disjoint from it using the
   positive rational control hull. Holes participate in the same census. A
   certified interior winding witness then identifies the entire connected
   boundary-free open rectangle as material. Unsupported overlapping hulls
   refuse; they do not imply an actual invalid region.
4. Fresh original interval normal bounds separate one common world-axis
   component from zero throughout each material rectangle. Failed trials may
   shrink the rectangles; every normal span stays charged. The material-signed
   normal components must have a coherent sign.
5. Endpoint tangent directions use adjacent ORIGINAL Cartesian controls.
   Positive clamped weights and knot widths multiply these differences by
   positive scalars. Native predicates receive the original points, not rounded
   tangent differences. The two appearances of every owned ray must be exactly
   collinear with the same direction in the common projection.
6. Fresh exact angular predicates prove strict successive cones and exactly
   one complete turn of the original link. Duplicate winding, backtracking,
   zero cones, incomplete corners and exhausted work yield no neighborhood.

Why these premises give a physical graph neighborhood:

The nonzero projected Jacobian and original Ck face jets give a local projected
inverse at each natural corner, including a one-sided material quadrant. Its
two boundary meridians are regular Ck curves with the certified outgoing rays.
The exact single-turn fan has finitely many distinct rays and positive angular
separations. After choosing a sufficiently small neighborhood, the projected
meridians stay in their distinct angular cones and meet each small centered
circle once. Each projected material quadrant occupies the sector between its
two meridians. Shared carriers identify these boundary curves exactly. One
complete turn therefore partitions a sufficiently small disk exactly once.

The admitted body's embedding excludes other incident contacts and remote
source portions. Compactness of the finite original charts permits a smaller
world neighborhood containing only this owned star. Thus this is a neighborhood
of the body's physical boundary, not merely a collection of projected patches.

Exact cooriented G1 identities give equal graph gradients on every meridian.
Exact G2 identities additionally give equal graph Hessians in the common
projection. These assertions are invariant under the projection proposals used
by the original seam factories, since rank and coorientation are retained.
Finite Ck face jets have common limits at the central vertex by connectivity of
the complete link. Integrating the continuous limiting gradient and Hessian on
radial segments removes the central point and proves a C1 or C2 graph there.

This is an existence certificate for an open physical graph neighborhood. The
parameter rectangles bound the proof; their whole union is not asserted to be
one projected disk, and no numerical metric radius is promised. Explicit C0
strata remain C0. Polar stars keep their separate owned graph/jet proof route.
Non-natural source corners and cases not certified by the stated hull/ray
conditions remain open. This does not complete general ideal-law-to-shell
binding or the full loft/sweep objective.

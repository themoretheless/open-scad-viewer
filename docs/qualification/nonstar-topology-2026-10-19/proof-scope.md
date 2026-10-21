# Proof obligations and scope

This is a Rust checker for sufficient exact certificates, not a formal proof
assistant development. Every witness is derived again from the bound original
source; changing its forest, coefficients, polynomial, erosion cores or census
must fail equality in the independent verifier. Rational arithmetic represents
all input binary64 values exactly. Work and degree limits may refuse a proof.

## Signed prismatic atlas

A full original face identity proves each shell is the Cartesian product of a
planar region and a height interval. The caps are exact affine charts and their
original rational trim profiles agree. Side charts are exact Cartesian ruled
surfaces, not sampled fits. Seam/preflight obligations precede this proof.

A simultaneous original-profile certificate establishes disjoint labeled Jordan
curves and their entire containment forest. The actual nominal polygon family
is simple/disjoint with the same labeled parent forest. All curves are normalized
to the same orientation. The planar Jordan/Schoenflies isotopy extension theorem
therefore supplies one orientation-preserving ambient homeomorphism of the plane
carrying every labeled bounded interior to the corresponding polygon interior.
Its product with the identity on height preserves every shell membership and
hence the complete positive-winding material, its boundary and its complement.
This applies to nonstar regions, multiple handles, nested negative shells,
sealed cavities and disconnected bodies admitted by the checked atlas.

The checker reconstructs the signed product material and compares its actual
canonical filled boundary with the actual nominal material. Matching a genus,
Euler number or nesting depth alone would not establish this obligation.

## Polynomial ambient atlas

For one coordinate axis the global map is F(x,y,z)=(x,y,z+f(x,y)). Its explicit
inverse subtracts f, and F_t adds t*f; its Jacobian determinant is exactly one.
There is no star-center hypothesis and no restriction on genus or cavities.
Every full original NURBS knot cell must equal F applied to a rank-two affine
chart as an exact polynomial identity. Current implementation requires constant
weights and exact straight UV trim images for the inverse polyhedral source.

The reference mesh has the actual nominal mesh's abstract triangulation. A
producer guesses positions on inverse source planes; those guesses have no
proof authority. Exact positive-filled material equality and closed manifold
vertex links are checked. Then all disjoint and incident moving triangle pairs
are checked through a piecewise linear isotopy to the actual nominal mesh,
including the shared-edge and shared-vertex stars. The PL isotopy extension
theorem and the explicit ambient inverse establish preservation of material and
complement. A small coordinate error alone is never a topology certificate.

For a proved nonradial source atlas (signed prism or polynomial), certificate v3
uses the exact axis-aligned
box of original Cartesian surface controls as the outer source enclosure and
an empty inner source enclosure. Positive rational tensor weights put the whole
source boundary in that box; bounded positive-winding material is also inside.
This avoids a facet-plane explosion in tight metric tube arrangements. It is
conservative: the filled Hausdorff bound is explicitly absent. The ambient
source topology proof and per-face continuous deviation bound remain checked.

## Sum topology is a separate assertion

Source topology is not generally preserved by Minkowski addition. For a checked
single prism with convex outer region O and disjoint strictly interior convex
holes H_i, and a convex connected prism kernel K, connectedness of x-K gives

    (O minus union interior(H_i)) + K
      = (O+K) minus union {x : x-K is contained in interior(H_i)}.

Every surviving hole is the interior of a convex erosion H_i minus (-K).
Independent rational Bernstein inner/outer convex hulls sandwich each erosion.
A positive-area inner core proves an open hole; a zero-area/empty outer core
proves its absence. Ambiguous critical transitions refuse. Disjointness follows
from that of the source holes; erosions remain strictly inside the outer sum.
The actual nominal sum must itself be an exact prism and have the same hole
count. Both are products of connected planar regions with disjoint disk holes,
so this establishes a homeomorphism of the complete 3D material. It does not
assert a particular geometric sum isotopy for arbitrary NURBS operands.

Other supported full-sum proofs remain the verified radial construction and
zero-error identity of both sources. Two valid source atlases never suffice by
themselves to enable the full-sum claim in the worker protocol or interface.

Unsupported general rational deformations, non-product arbitrary NURBS handles,
critical/degenerate contacts and unsupported trim atlases retain an explicit
unproved topology status. Legacy version 1/2 proofs retain their old recipes.

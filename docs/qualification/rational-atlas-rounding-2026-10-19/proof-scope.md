# Rational source atlas and mesh rounding: proof obligations

This is an exact-arithmetic sufficient certificate checker implemented in Rust,
not a proof-assistant formalization or a complete decision procedure for all
NURBS inputs. No numerical tolerance admits a topology claim.

## Source homotopy

For every full rectangular source face, split at all active knots and the
chosen dyadic stations. Convert the original positive homogeneous tensor net
exactly to triangular Bernstein form on the same diagonal used by the actual
nominal source mesh. This supports nonconstant rational weights and degrees
1 through 8 in each tensor direction, without recognizing an analytic surface,
requiring a global star centre, or prescribing a polynomial deformation.

For each triangle, write S=N/W, W>0, and let L interpolate its three original
corners. The checked map is H_t=(1-t)S+tL, 0<=t<=1. Its homogeneous numerator
is (1-t)N+tWL; both endpoints use the same elevated W. All swept convex-hull
bounds therefore cover complete triangles and complete closed time intervals.

Every geometric edge has exactly two oppositely oriented incidents. Original
rational edge functions must agree pointwise, after orientation reversal, by
exact homogeneous cross multiplication. Equality of endpoints or of sampled
points is insufficient. All source corners remain fixed throughout H.

For each vertex the oriented triangle link is one circle. A common projection
is chosen, and each incident patch's projected rational derivative is expressed
in its actual PL triangle basis. Either strict positive row diagonal dominance, or positive determinant and
positive trace, is checked over the whole triangle in exact Bernstein
arithmetic (interval bounds may prove the determinant before multiplication).
For the second criterion det((1-t)J+tI)=(1-t)^2 det(J)+t(1-t)tr(J)+t^2>0.
Thus every patch has a nonzero orientation-preserving projected derivative
throughout the homotopy. The disk degree proof below supplies global injectivity.
The projected link boundary is a Jordan curve for every t: each boundary edge
is strictly monotone in its chord direction; nonadjacent edge swept hulls are
strictly separated; adjacent edge cones meet only at their fixed common end.

The disk degree argument then applies. All interior preimages have positive
local degree, while the simple oriented boundary has degree one on its inside
and zero outside. Thus the projected whole vertex star is a disk embedding;
local seam regularity and link degree rule out identifications at seams and at
its central vertex. Each star is locally flat. All pairs of patches without a
shared abstract vertex have disjoint swept control hulls, with exact spatial
subdivision if necessary. Pairs sharing a vertex are covered by that vertex's
proved disk. Hence H_t embeds the entire closed oriented boundary at all times.

Local flatness is required jointly in space and time, not merely for each
individual surface. On a smaller interior disk of each proved vertex star,
(x,t) -> (projection(H_t(x)),t) is a continuous injective map into R² x I and
hence a local homeomorphism. The remaining normal coordinate is a continuous
graph, giving a level-preserving local flattening of the whole track. This
supplies the locally flat proper isotopy hypothesis of Edwards and Kirby,
[Corollary 1.4](https://www.maths.gla.ac.uk/~mpowell/Edwards%20Kirby.pdf),
*Deformations of Spaces of Imbeddings*, Annals of Mathematics 93 (1971),
[publication record](https://annals.math.princeton.edu/1971/93-1/p03).
Compactness then gives ambient isotopy by that theorem. This preserves positive-winding material,
all shell nesting, and every component of its complement, irrespective of genus
or connectedness. No assertion about the topology of a Minkowski sum follows
from two source isotopies alone.

The independent source certificate binds the complete serialized source, exact
PL coordinates, connectivity, subdivision depth and recomputed report. For
Minkowski enclosure version 4, the proved endpoint must additionally equal the
actual oriented nominal triangle chain, with multiplicities. Versions 1, 2 and
3 keep their previous deterministic source-proof recipes. In particular, the
polynomial-atlas checker freezes its original moving-simplex predicates and
work accounting in `rounding_legacy.rs`: a stronger display checker must not
change the serialized choice of homotopy segment count or pair census.

## Coordinate rounding

Rounding checks the complete linear vertex homotopy and every triangle pair.
Absolute swept hulls are tested first, preserving every previous separator.
Common moving translation gives a second set of bounds; it cannot discard an
absolute witness, including a sub-ulp exact gap in the rational homotopy checker. Each time slab is charged once. Pairs that cannot be
separated over a full time interval are subdivided into closed time intervals;
accepted children jointly cover the entire interval. Endpoint failures,
triangle collapse, edge/star ambiguity, and budget exhaustion remain failures.

For coplanar hulls, exact in-plane separating axes are required. Ordinary 3D
face normals and crossed edge directions alone miss coplanar disjoint triangles.
The new branch first proves coplanarity exactly, then checks edge normals in
that plane. Collinear hulls use the line direction. All axes are accepted only
by strict separation of every input hull vertex. A triangle may rotate beyond its initial normal: positivity of its exact
squared area proves nondegeneracy without fixing a projection direction.
Polynomial sign checks use
exact Bernstein coefficients and closed-interval subdivision, with positive
endpoint values. No samples or epsilon signs replace a continuous proof.

## Deliberate refusals and remaining scope

- General curvilinear trims, holes within a single UV chart, and singular chart
  poles need a compatible embedded parameter triangulation and additional local
  charts. They are not admitted by this rectangular-atlas recipe. Existing
  prismatic/polynomial recipes still cover their previously supported trims.
- Different nonlinear clocks on a shared curve require an explicitly verified
  parameter transport. Pointwise seam failure remains unproved here.
- Some embedded stars have no projection satisfying this sufficient condition;
  they require another local certificate. Finite degree, coefficient-bit,
  cell, subdivision and exact-work budgets also cause explicit refusals.
- Genuine f32 collapse cannot be certified as topology preserving. A coordinate
  error bound alone does not make a collapsed feature acceptable.
- The independent source certificate describes an exact rational endpoint mesh.
  Any subsequent conversion to binary64/f32 requires the separate rounding
  checker; the independent source certificate alone makes no display claim.

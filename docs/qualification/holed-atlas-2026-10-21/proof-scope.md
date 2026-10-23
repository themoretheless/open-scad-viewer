# Sufficient exact predicates

All accepted equalities concern the exact rational interpretation of authored
binary64 data. No coordinate tolerance repairs a source gap. The existing
source-star Jacobian, Jordan-link, nonincident-hull separation and exact source
identity checks remain mandatory.

## Regions with holes

Every complete rational UV boundary edge has a monotone chord coordinate.
Exact swept-control-hull separators and endpoint cones prove that all wires
remain disjoint Jordan curves throughout their homotopy to the chord wires.
Exact signed areas and winding tests prove outer/inner orientation, containment
and nonnesting of holes at that endpoint. Hence original material winding is
0 or1 throughout its complementary regions.

A deterministic rational advancing-front algorithm proposes triangles of the
authored boundary vertices. It uses exact ordering, segment predicates and
winding, without floating distances, epsilons or platform-dependent tie breaks.
Its proposal has no authority to establish coverage.

For each proposed positive triangle, substitute its original curved boundary
edges and straight internal diagonals. The existing triangular UV disk checker
proves positive Jacobian and Jordan boundary for the whole disk homotopy.
Check every index, edge incidence, orientation and exact boundary cancellation.
Each such disk has nonnegative winding0/1. Cancellation makes their winding sum
equal the original0/1 region, so no hole is filled and disk interiors cannot
overlap. Compose every disk exactly with its original NURBS surface. Full world
atlas checks still exclude any nonincident contacts or bad vertex stars.

Current limits:128 boundary vertices per face,64 edges per wire,1024 triangles,
one tensor knot cell per composed source face, and existing arithmetic budgets.
Complex nonmonotone boundaries may require a subdivision not generated here.

## Stationary poles

Rotate a collapsed tensor side to v=0, and also try angular reflection. Require
all nonzero homogeneous power coefficients u^i v^j to have i divisible by a,
j divisible by r, and j/r >= i/a. Then x=v^r, y=u^a v^r gives an exact polynomial
homogeneous triangular chart. On v>0, its inverse uses the unique nonnegative
real roots on [0,1]. The collapsed side is one already-checked source point.
No algebraic mesh coordinate or root approximation is introduced.

Positive target weights and the full atlas embedding checks remain necessary.
This handles higher-order angular and radial stationary coordinates, including
their reflected versions. It is not a universal replacement for conical or
other non-monomial singular rational charts.

## Nonlinear seam correspondences

On straight seams, prove the same oriented segment endpoints, collinear controls
and strictly monotone complete rational clocks. Each source-to-chord homotopy
has the same segment image. The edge-clock homeomorphisms vary continuously
in time and extend over each parameter disk. Thus projected-star and joint-track
arguments still apply after this time-dependent gluing; speed alone is not a gap.

For curved graph carriers, find a spatial linear observable that is exactly a
fractional-linear function of one curve's parameter. Recover the other curve's
rational parameter clock from it. Require its positive denominator, domain,
endpoints and monotonicity, then independently check the full homogeneous
composition identity in all three spatial coordinates. Matching a projection
alone is insufficient. A changed third coordinate is a tested refusal.

Prescribe the proved clock on its source triangle edge and identity clocks on
unassigned edges. The UV disk checker proves an embedded rational extension.
Exact triangular composition reparameterizes the whole source patch, preserving
its image and vertices. All final seams, source derivatives, stars and disjoint
pairs are checked again. This is not an image-only relaxation for curved seams.

Limits:128 transported charts; composed chart degree at most16; seam identity
degree at most64; strictly positive sufficient Bernstein predicates. Curves
without a recoverable observable, unresolved inverse branches, stationary clocks
that fail disk regularity, and failed parameter extensions still refuse.

## Compatibility and delivery

Old successful projective transport is attempted on a copy first. Failed attempts
cannot leave changed charts behind. Each bounded attempt has its own predicate
budget; the final admitted recipe still passes its complete checker.

The v4 enclosure source-atlas recipe is frozen in atlas_legacy from the four
2026-10-20 pinned source files. Only test modules are removed. New standalone
source certificates do not bypass the actual Minkowski nominal-chain binding,
and do not prove topology of a complete Minkowski sum. Shared WASM and dist are
not promoted by native qualification.

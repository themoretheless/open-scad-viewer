# Filled planar cap error transfer

This is a mathematical transfer lemma, not a runtime certificate. The runtime
must supply every premise below from original geometry and shared budgets.

Let A and B be compact planar material regions in planes PA and PB. Each is
represented by finitely many simple oriented boundary curves, with outer and
hole ownership such that the summed winding number is exactly its material
indicator off the boundary. Boundaries are paired parameterwise, respecting
this ownership convention, with Euclidean distance at most epsilon everywhere.
Assume orthogonal projection pi from PA to PB is nonsingular (the planes'
unit normals have nonzero dot product). Orient the projected curves so their
winding number represents pi(A); reflection of the plane reverses all boundary
orientations together.

Then the Euclidean Hausdorff distance between A and B is at most
sqrt(2) * epsilon.

## Proof

1. At every boundary point a of A, its paired point b lies in PB, so
   distance(a, PB) <= |a-b| <= epsilon. Signed height over PB is affine on PA.
   Its maximum and minimum over the compact region A occur on its boundary:
   if height is nonconstant, an interior extremum would admit a sufficiently
   small displacement in PA increasing/decreasing it; if constant, its value
   equals its boundary value. Thus every point of A has absolute height at
   most epsilon, including regions with holes.
2. Projection is nonexpansive and fixes points of B. Paired projected boundary
   points satisfy |pi(a)-b| <= epsilon. Because pi is an affine homeomorphism
   between the two planes, pi(A) is a compact material region with the paired
   outer/hole ownership. The planar winding transfer lemma gives
   Hausdorff(pi(A), B) <= epsilon. For completeness, a point more than epsilon
   from one endpoint boundary cannot be reached by the linear homotopy of
   paired boundary curves. Its winding sum cannot change. A material point
   lying outside the other region therefore cannot be farther than epsilon
   from that region. Apply this argument in both directions. Intermediate
   homotopy boundaries need not be simple.
3. For a in A choose b in B with |pi(a)-b| <= epsilon. Height a-pi(a) is
   orthogonal to PB, hence |a-b|^2 <= epsilon^2 + epsilon^2. Conversely, for
   b in B choose y in pi(A) with |b-y| <= epsilon and use the unique a in A
   with pi(a)=y. Step 1 bounds its height and the same identity applies.
   Compactness guarantees nearest points exist. Both directed distances are
   at most sqrt(2)*epsilon.

## Runtime obligations

- Certify the ideal endpoint cap is a nonsingular affine image of the authored
  local material profile. Profile projection/planarity tolerance alone does
  not certify ideal outer/hole ownership.
- Certify both endpoint material ownership conventions and all-domain boundary
  pairing; sampling and retained-only cap validity are insufficient.
- Certify a strictly positive lower bound on the absolute normal dot product
  using the ideal and retained planes. Near-perpendicular uncertainty refuses.
- Compose endpoint contour rounding and explicit correction displacement
  outward before applying the lemma. Multiply/square-root outward, or use the
  conservative 2*epsilon bound with outward addition.
- Require exact retained cap-region correspondence and exact retained wall
  decomposition/correspondence separately, then take the maximum of cap and
  wall bounds under the requested total budget.

For coincident planes, height is zero and the sharper epsilon bound applies.
For parallel translated planes the sharper epsilon bound also applies, by the
constant-height argument below. No result here licenses continuousBound until
all runtime obligations are implemented and verified.

## Sharper transfer for exactly parallel planes

Let the planes be parallel and let their constant separation be h. Every
paired boundary displacement decomposes into a constant orthogonal component
of magnitude h and a tangential component. Since the total displacement is
at most epsilon, h <= epsilon and the tangential component is at most
delta = sqrt(epsilon^2 - h^2). Projection is now a translation isometry.
The same per-loop winding transfer gives Hausdorff(pi(A), B) <= delta.
Every interior point of A has the same height h. Both directed distances
therefore satisfy distance^2 <= delta^2 + h^2 = epsilon^2. The conclusion
also covers h = epsilon (identical projected material regions) and h = 0.
The runtime need not subtract nearly equal squares or evaluate delta: once
exact parallelism is certified, it may use the existing outward epsilon bound.

An interval dot product close to one is insufficient to certify parallelism.
An exact certificate must identify the original ideal endpoint axis (before
normalization) and prove the actual retained cap plane has a normal parallel
to it. Guide modes retain the original path endpoint tangent as their axis;
authored frame modes use the original longitudinal law's exact endpoint
value, not the stored normalized frame. Constant laws can supply that value
directly; general rational endpoint values require exact evaluation or a
construction identity. The native certificate now checks exact retained-plane
planarity/nondegeneracy and two exact direction dot products against each
original endpoint axis. Clamped rational authored laws use their end control
vectors. Identical original poles additionally prove an exactly constant
rational axis under any valid knot domain/positive weight encoding, without
clamping or rounded evaluation. Other authored endpoint encodings refuse this
sharper proof instead of using rounded evaluation. Ordinary/guide axes use
differences of original path endpoints.
Both endpoint frames and a shared exact-work budget are required, and no
partial parallel pair is returned on exhausted/indeterminate work. Native JSON
transport and TypeScript body composition now carry this certificate separately
from projection/ownership; each parallel endpoint uses epsilon, while missing
or negative parallel proof keeps the general sqrt(2)*epsilon transfer. Both
paths retain all material-ownership, boundary-pairing and correction premises.
Packaged WASM/public qualification is tracked in the coverage audit.

The general runtime currently multiplies epsilon by the binary64 rational
6369051672525773 / 2^52 and then takes one floating-point successor. The
rational's exact square exceeds 2, so it bounds sqrt(2) from above; the
successor encloses product rounding, including subnormal products. Invalid
input, overflow or a nonfinite successor refuses. Integer dyadic oracle tests
independently verify output^2 >= 2*input^2 across normal/subnormal inputs.

## Source profile ownership prerequisite

The ideal miter profile is defined by projection onto the initial local
normal/binormal coordinates, even when source controls pass only the
constructor's planarity tolerance. Therefore a retained endpoint contour
audit cannot replace a source-to-local material certificate. A valid transfer
route is: certify an exact common source profile plane; certify a nonzero dot
product between its normal and the ideal initial path tangent; then certify
source contour simplicity, disjointness and material nesting. Projection into
the initial local plane is then an affine homeomorphism. Positive transverse
scale/axis factors and a certified orthonormal endpoint frame give another
affine homeomorphism into the endpoint plane; center translation preserves
material ownership. Reflections must change the entire boundary winding
convention together. Profiles passing only the planarity tolerance require a
separate interval projected-contour topology certificate and cannot use this
exact-source-plane route. The existing contour audit supports exact coordinate
planes; extension to arbitrary exact planes must preserve original controls
and avoid a rounded UV fit.

## Per-loop proof without a shared boundary orientation

For the supported one-outer/disjoint-hole material convention, the planar
transfer lemma also follows from each simple loop's winding magnitude. Pair
each outer boundary with the other outer boundary and each hole boundary with
its corresponding hole boundary. Reverse a paired curve only when necessary
to express the same parameter pairing; no coupling of independently stored
coedge orientations is assumed. A point at distance greater than epsilon from
the boundary of one endpoint region cannot be reached by any linear paired
curve homotopy, so every individual winding number is unchanged. For a simple
loop, its winding magnitude is one inside and zero outside, irrespective of
orientation. Material membership (inside the outer loop and outside every hole)
is therefore unchanged at that point. Every outer/hole boundary belongs to
the closed material region, so a point farther than epsilon from that material
region is farther than epsilon from all its boundaries. A material point of
the other region at that distance would contradict the unchanged per-loop
membership. Apply the argument in both directions. The projection and height
steps of the distinct-plane proof then remain unchanged.

This argument preserves explicit outer/hole role pairing, simple endpoint
loops, strictly contained disjoint holes and a nonsingular plane projection.
It does not prove shell orientation, nested-shell containment, curve
regularity, or intermediate homotopy simplicity. These remain separate
certificates. It permits the cap error transfer to use geometric material
ownership without treating stored coedge orientation as a global shell proof.

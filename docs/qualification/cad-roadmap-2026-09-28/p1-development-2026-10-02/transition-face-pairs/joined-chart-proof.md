# Joined projected chart proof

Scope: two single-span rational tensor Bézier charts, a blend and a ruled
wall. Their original control points and weights on blend v=1 and wall v=1
must match exactly. Both U domains are normalized to [0,1]; reversing U is
shared by both charts. The joined coordinate t is blend v for 0≤t≤1 and
2−wall v for 1≤t≤2. Matching seam definitions make the map continuous.

The blend has one complete U endpoint identified to its owned pole. The
wall's first projected coordinate is independent of its V coordinate.
Its positive, unweighted second-coordinate V derivative makes the wall
leave the pole immediately after the seam. No other point of the wall
U endpoint is identified to the pole.

## Sufficient inequalities

For the same two linear projection rows on both source charts, conservative
bounds over the entire normalized domains establish

- f_u ≥ a > 0;
- g_t ≥ b u² with b > 0;
- |f_t| ≤ e u;
- |g_u| ≤ c u;
- a b − c e > 0.

For two parameter points, integrate the piecewise Jacobian along the
straight segment between them. Let A be its average. Then

A_11 ≥ a, A_22 ≥ b ∫u², |A_12| ≤ e ∫u, |A_21| ≤ c ∫u.

Cauchy–Schwarz gives (∫u)² ≤ ∫u² on this unit integration interval, so

det A ≥ (a b − c e) ∫u² > 0

whenever the segment is not entirely on U=0. The fundamental theorem of
calculus gives Δ(f,g)=A Δ(u,t); invertibility excludes equal projected
positions. Equality of three-dimensional positions would imply equality
of their projections, so it too is excluded. At U=0 the blend remains at
the pole, while the wall's strictly positive g_t separates its remaining
points. A segment lying on the seam uses the common exact U curve and
has the same directional derivative from either side. The seam kink in
the transverse derivative does not affect the integral identity.

## Source checks and interval work

The blend bounds come from original homogeneous controls, exact known U
factors, and outward interval Bernstein restriction. Conditional bounds
that prove the blend alone are insufficient: the joined result explicitly
requires the global product inequality above.

The ruled wall must have degree one in V and equal V weights per U row.
Its first projected coordinate must be exactly invariant between its two
V controls. Original coordinates used by the second projection must match
between the first two U rows, for each V endpoint. Those source equalities
establish g_u(0,t)=0 independently of differing U weights; they justify the
known zero first Bernstein row before dividing by U. Other coefficients
retain outward interval enclosures. A positive unweighted wall g_t bound
also bounds b u² because U lies in [0,1]. Each chart is restricted over a
complete 16×16 grid, consuming 512 reported proof cells per accepted pair.

## Binding and current qualification

B-rep admission requires the same owned edge, complete natural boundaries
on both charts, and exact rational equality with the authored edge. The
embedding audit additionally requires every edge/lift use and join to be
exact, every trim region to be valid and positively wound, and every face
to be injective. Missing prerequisites admit no contact certificate.

For the canonical quarter specimen R=20, inner radius=5, height=6 and
blend radius=1.25, both remaining pairs pass. Their global margins exceed
1.63753. All 351 pairs are classified: 240 disjoint and 111 shared contacts.
The 27-face boundary is proven embedded for this source model.

This does not qualify endpoint G1, wall thickness, shell orientation,
material volume, arbitrary placements, all radii or the broader roadmap.
WASM and application qualification of this change remain pending.

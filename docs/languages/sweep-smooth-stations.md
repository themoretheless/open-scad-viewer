# Bounded station reconstruction

`smooth_station_walls` proposes degree-5 walls between retained rational sections.
It accepts canonical nine-pole, degree-2 profiles represented by a centre and
two generators on a common binary lattice. Each profile retains its positive
weights and exact four-piece decomposition. Incompatible weights, changed
decomposition, out-of-range lattice coordinates and exhausted work are refused.

For a station generator G and its rounded lattice increment T, the adjacent
quintic controls are G, G+T, G+2T. At the opposite end they are H-2U, H-U, H.
Soft stations share T across their two walls, giving identical first derivatives
and zero second derivatives in the normalized span parameter. Closed paths use
the same rule across the duplicate endpoint. Sharp stations use independent
one-sided increments and retain C0; marking either copy of the closed endpoint
marks the same geometric station.

The displacement certificate encloses every control difference against the
degree-elevated source linear wall with outward interval arithmetic. Common
positive weights make the pointwise difference a convex combination of these
differences. Its maximum norm therefore bounds wall displacement over the
whole domain. Exceeding the requested displacement tolerance returns no walls.
The constructor alone makes no claim about regularity, intersections or Solid.

`section_loft_surfaces` retains the supplied walls on every span and extracts
their longitudinal edges from the actual surfaces. `proposeSmoothStationWalls`
and `createBrepSectionLoftSurfaces` expose these native operations through JSON
and TypeScript. A valid B-rep topology alone is insufficient for Solid admission.

`smoothCertifiedMiterBody` requires an unchanged constructor-owned complete
boundary certificate and sections that exactly reproduce its source B-rep.
It adds the wall displacement bound to the source wall bound, checks the complete
boundary budget, and verifies that the caps, trim loops, boundary edges and
vertices are unchanged. It then recomputes profile/station smoothness, wall
injectivity, global embedding, nesting and material orientation on the new body.
Unproved material geometry is refused; source certificates are not transferred
to changed walls. `reconstructCertifiedMiterStations` uses privately retained final corrected
sections; exposed approximation samples cannot replace them. Original polyline
vertices retain independent one-sided jets.

Rush exposes `.brep_smooth_miter_stations(wall_tolerance: ..., quantum: ...,
max_work: ..., max_deviation: ...)` after a progressive miter body. Tolerances
and quantum are lengths; mismatched dimensions refuse during compilation.
The complete boundary budget includes both the original approximation and
reconstruction displacement. It is independent of the wall reconstruction
tolerance. See `examples/rush/progressive-miter-reconstructed-stations.r` for
a curved G2 Solid and `progressive-miter-reconstructed-sharp.r` for a preserved
C0 corner. Unsupported or unproved combinations still refuse.

The station normal-scale proposal is computed by Rust
`propose_station_normal_scale`, exposed through JSON/WASM; the TypeScript
adapter forwards the retained surfaces and boundaries. Clamped Bezier spans
with constant longitudinal weights use stored endpoint poles in normalized
parameters, avoiding independent rational-basis rounding. Other supported
spans use native rational derivatives. A proposal cannot certify continuity;
exact strip identities and regularity are still mandatory.

Profile and station audits share a finite default budget of 2,000,000 exact
operations. Each individual native predicate is limited to 1,000,000. Work
already spent on profile G2/G1 is deducted before station G2/G1, and viewport
admission validates the combined accounting. An explicitly smaller budget can
withhold smoothness even when the represented body is a certified Solid.
The combined guide/frame/affine hollow examples are
`progressive-miter-reconstructed-guide-frame-affine-hollow.r` and
`progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r`.

Closed circle correction does not require filled-cap proofs because the body
has no caps; retained wall correspondence, regularity, the complete boundary
budget and Solid obligations still apply. See
`closed-miter-frame-guide-affine-hollow-corrected.r` and
`closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r`.

For quadratic cross strips with uniform weights `[1,w,1]`, positive `w` and
unit transverse scale, Rust checks a sufficient exact conic relation on every
along-seam coefficient. If E is the shared boundary pole, Q the adjacent pole
and R the opposite pole, G1 requires `Q_a + Q_b = 2E`; G2 additionally requires
`R_a - R_b = Q_a - Q_b`. These identities imply constant projective jet
relations. They use exact authored arithmetic, not rounded pole comparisons,
and still require independent seam regularity. Noncanonical strips fall back
to the general predicate with only the remaining work budget.

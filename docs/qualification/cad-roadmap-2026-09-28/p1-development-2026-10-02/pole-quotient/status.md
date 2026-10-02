# Within-face absence on a collapsed transition boundary

2026-10-02. Native implementation and explicit source fixtures. This new
proof is not yet integrated into combined B-rep diagnostics or the packaged
WASM runtime. Preview application remains disabled.

## Geometry correction

The zero-radius endpoint and the adjacent radius-law control both have zero
radius. Their Euclidean control points must be independent of meridian v.
Previously, repeated multiplication and division through different meridian
weights introduced one-ulp differences in the adjacent row. The constructor
now authors the complete repeated row from its common center control, sets
height directly, and preserves coordinates shared by adjacent angular controls
directly. This introduces no tolerance snapping of external geometry.

21 circular-blend tests pass, including radius law, contact normals, boundary
agreement, source identities and placements. The corrected 18-member STEP
matrix passes independent OCCT solid validity, bounds and integrated-volume
checks. Maximum volume error is 5.548135959543288e-7 mm³; maximum bounds error
is 1.0000002248489182e-7 mm. Tolerances are unchanged. OCCT validity alone is
not used as an absence-of-intersection certificate.

## Sufficient quotient-domain proof

Normalize and, for an exit, reverse u so the entire u=0 boundary is one
declared point. For a linear projection F=(f,g), interval bounds establish

`f_u >= a > 0`, `g_v >= b*u²`, `|f_v| <= e*u`, `|g_u| <= c*u`.

The input must be one clamped nonperiodic Bezier patch, degrees at most eight.
Every point of the collapsed row must be exactly identical. Derivative
numerators are formed as tensor Bernstein polynomials from the original
controls and positive weights. Removing factors u and u² requires complete
leading coefficient rows to be exact zeros; a small residual is rejected.
Every remaining coefficient and denominator has outward interval bounds.
No sampled derivative admits a patch.

Consider equal projected images at parameters with u2>=u1 and u2>0. Write
each difference as one horizontal segment at v1 and one vertical segment
at u2. Equality of f implies

`a*(u2-u1) <= e*u2*|v2-v1|`.

Equality of g implies

`b*u2²*|v2-v1| <= c*u2*(u2-u1)`.

If `a*b > c*e`, these inequalities force v1=v2, then u1=u2. An interior point
cannot equal the pole because f_u>0 and f=0 at the pole. Only the already
declared whole u=0 boundary is identified. This proves within-face absence
on that quotient domain; it does not provide a regular tangent plane there.

For larger radii, a global c can be unnecessarily wide. The first inequality
also imposes `u1 >= u2*(1-e/a)` because |v2-v1|<=1. In each covered u2 band,
the verifier bounds g_u only over this necessary horizontal range, with
outward range endpoints. It then verifies dominance for every band. These
are conditions on any hypothetical equal-image pair, not independent local
injectivity certificates. The complete chart and all possible pairs remain
covered by the argument.

## Evidence and remaining work

`native.json` proves all 36 entry/exit faces: three scales, three radii and
both directions. Twelve use conditional band dominance. The smallest
certified dominance margin is 0.3582135898381414. Two nurbs-core tests cover
polynomial and rational triangular charts, reversed poles, partial budgets,
exact-order refusal at a 1e-12 mutation and a folded transverse parameter.

These fixtures use specified projections on the constructed axis-aligned
geometry. General placement/frame selection and B-rep pole ownership must
be integrated and validated before updating the UI aggregate. Distinct-face
contacts, endpoint G1 and the complete volume certificate remain open.

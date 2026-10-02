# Within-face absence on a collapsed transition boundary

2026-10-02. Native implementation and explicit source fixtures. The proof is integrated into native combined B-rep diagnostics and source-bound
frontend validation. The packaged WASM and production browser validation pass
with both mouse and keyboard input. Preview application remains disabled.

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

The source-frame projection uses exact expressions in original binary64
control points, with outward interval bounds. Its orthogonality identity is
preserved algebraically. `source-frame-native.json` covers 108 entry/exit
cases across three scales, three radii, both directions and three placements;
all pass without changing their source models.

`combined-native.json` proves within-face absence for all 27 faces: 25 regular
charts and two quotient charts. Pole ownership is checked against actual
coedges, vertices, complete UV traversal and the collapsed surface boundary.
The frontend binds the returned proof to all original surface controls,
weights, knots and periodicity as well as its frame and pole identities.
Fourteen protocol tests pass, including mutations outside the projection
frame, altered reply controls and weights. TypeScript checking passes.

Distinct-face contacts, endpoint G1 and the complete volume certificate
remain open. The aggregate `absenceProven` remains false. No preview commit
is enabled by this within-face certificate.

## Packaged runtime evidence

15 WASM/protocol tests pass. `wasm/report.json` records six placement/budget
cases and two worker handler cases, with unchanged source models. Both
`browser-mouse/result.json` and `browser-keyboard/result.json` pass against
the same production WASM; the mouse screenshot was visually inspected.
The UI distinguishes within-face coverage from unresolved distinct-face
contacts and endpoint smoothness. Exported documents remain unchanged.

Geometry WASM: 9734183 bytes; SHA256 `3f158c6c1624a32b213761f9f763633df1f3ca8b3553cdd8b45e105c8deb323d`.
Production verification: 140 artifacts, 7,270,619 asset bytes and 11,790,303
raw WASM bytes, totaling 19,060,922 bytes. Size ceilings retain the prior
measured headroom after accounting for this feature. Preview commit remains
disabled.

# Exact rational-weight affine ellipsoids

`nurbs_core::primitives::exact_quadrics::ellipsoid(center, axes, max_work)`
returns the whole affine image of the unit sphere under three authored 3D axes.
Unlike the older axis-aligned revolution constructor, this API supports oblique
and orientation-reversing frames and proves the geometry of the literal output.
The older constructors remain available with their existing contracts.

The degree2×2 surface uses a 9×5 positive-weight tensor net, domain `[0,1]²`,
clamped nonperiodic storage, a repeated longitude seam and two exactly collapsed
latitude boundaries. At these poles the parameterization is singular while the
image is a regular ellipsoid. This geometric coverage theorem does not create
trim loops, a vertex fan, oriented B-rep incidence, volume or a certified solid.

The algorithm freshly obtains the proved exact unit circle from NURBS-0236.
Its first half gives the upper semicircle, used as radial/height latitude.
For longitude `(x,y)` and latitude `(r,-z)`, the local sphere coordinates are
`(x*r,y*r,-z)`. Every proposed affine pole sum and weight must round-trip through
binary64 exactly. Finite but nonrepresentable output is an explicit refusal.
Angles have variable rational clocks, not the legacy trigonometric clocks.

Proof then starts from the actual stored degrees, knot vectors, dimensions,
positive weights and pole values. Exact Cramer projections using the authored
frame cofactors prove every homogeneous control coefficient. All actual weights
must share one global gauge relative to the reference tensor. Thus the complete
homogeneous tensor is proportional to the reference factors:

`X=X_long*R_lat`, `Y=Y_long*R_lat`, `Z=W_long*Z_lat`, `D=W_long*W_lat`.

The two proved circle identities imply `X²+Y²+Z²=D²` on all eight tensor cells.
The full oriented longitude and upper-semicircle latitude prove full sphere
coverage, seam agreement, and both collapsed poles. Exact nonzero determinant
transfers this result to the affine ellipsoid. No epsilon rank decision or
floating determinant is used; even the smallest positive binary64 axis scale
can be proved without a floating determinant underflow.
Very small or nearly dependent frames can still make floating-point derivative
and normal evaluation ill-conditioned; this proof does not upgrade evaluator
statuses or supply numeric normal-error bounds.

One shared ledger includes fresh reference proof, construction and verification.
`max_work` remains `1..=1_000_000`; rational intermediates retain the existing
4096-bit guard. `Report.surface` is `None` on every refusal. No rounded fallback,
partial surface, caller-owned proof, evaluator-bit equality or prior artifact
certificate is accepted. Invalid inputs/budgets are errors; resource exhaustion,
nonrepresentable storage, dependent axes or unproved output are explicit refusals.

Qualification: [receipt](../qualification/nurbs-exact-quadrics-2026-11-03/receipt.json).
Native Rust only. JSON/WASM/Rush/editor/STEP and current-source distribution
remain separate qualification work.

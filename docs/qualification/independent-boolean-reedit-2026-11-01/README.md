# Independent Boolean and repeated editing — 2026-11-01

Native Rust qualification; bounded scope, not completion of arbitrary NURBS Boolean.

## Independent operands and crossing cuts

New `certify_quadratic_boolean` and `verify_quadratic_boolean` support union, intersection, difference and xor between independent Models whose complete original carriers and trim inverse images satisfy the common global map `F(x,y,z)=(x,y,z+k*x*x)`. Its explicit inverse is `z-k*x*x`, so set operations and topology transfer exactly. This includes `k=0` and admitted nonconvex/cavity/disconnected material; it does not require the cutter to be contained in the source.

The exact source-plane arrangement contains both operands' planes. Cells are classified with exact winding, and Boolean truth tables retain material. Union/xor use a box enclosing both sources (using only the first box would discard material). Shared boundary subdivisions are assembled jointly; manifold edge and vertex links are mandatory. The ordinary Model author creates exact curved caps, pcurves, loops, shells and cavities. Output controls that cannot be represented exactly in binary64 refuse. Point/edge-contact nonmanifold boundaries refuse.

The operation is bound in a new certificate envelope around the existing exact arrangement/source-atlas geometry receipt. Verification recomputes the exact construction and compares the full Model and receipt. Existing `certify_quadratic_difference` remains available.

The standard `operations::boolean` now attempts this exact path before existing analytic/numerical curved routes. Coefficient inference is only a proposal from exact polynomial coefficients. Both operands must pass complete carrier identities and exact trim inverse-image checks before dispatch. The existing numerical routes for other families are not qualified by this report.

## Circular pocket admission

The UV triangulation proposer can retry exact homogeneous bisection of curved authored boundary spans (up to two subdivision levels). Original rational curves are retained exactly. Every retry must pass the existing boundary isotopy, Jordan, winding, positive UV disk Jacobian, complete area-chain and incidence proofs. Existing work/vertex/triangle budgets and rejection predicates remain in force. This does not substitute chords for curves or relax tolerance.

The initial circular-pocket regression passed the former UV disk Jacobian barrier, then refused global projected vertex-disk regularity at depth 1. This diagnostic is preserved. The positive regression now constructs a circular pocket with a flat floor on the top of a unit cube, then another circular pocket with a flat floor on the front face. The second request uses the existing atlas depth 2 option. It validates continuous edge/pcurve agreement and replays the entire batch certificate successfully. It does not silently retry a depth-1 request with a raised depth. This qualifies this repeated-edit scenario; it does not prove every circular-pocket output or overlapping successive pockets. The full curved suite, including this expensive replay, took 385.14 seconds in the current concurrent workspace.

## Open scope

- Arbitrary independent NURBS carriers without a common proved map.
- Arbitrary cross-face algebraic intersections, tangencies and overlap strata outside the admitted family.
- Exact non-rational algebraic-edge payloads in ordinary Model and certified nonzero-error fitting.
- WASM/browser/STEP/dist qualification.

## Evidence

Logs, source hashes and the copied test executable hash are recorded alongside this file. Scoped source hashes are not a frozen transitive checkout or qualification of unrelated concurrent work.


## Qualified results

One native test executable was copied before additional suite runs. No source implementation changes followed the source hash manifest.

- `minkowski3d::curved::`: **113 passed**, 0 failed, 385.14 s (`curved.log`). This includes both new independent-Boolean tests and the new circular-pocket re-edit test.
- Remaining `minkowski3d::` tests with `--skip minkowski3d::curved::`: **26 passed**, 0 failed, 5.03 s (`minkowski-other.log`). Thus all **139** Minkowski tests pass across the disjoint filters.
- `operations::`: **25 passed**, 0 failed, 1.68 s (`operations.log`).
- `profile_certificate::branches::`: **7 passed**, 0 failed, 0.08 s (`branches.log`).
- Total distinct tests: **171 passed**, 0 failed.
- SHA-256 verification of scoped sources and copied executable passed; scoped formatting and diff-whitespace checks passed.

The new independent-model regressions check exact reference volumes 15/1/7 for union/intersection/difference of overlapping deformed boxes, xor retaining an exterior disconnected component (volume 9), forged operation rejection, and standard Boolean dispatch agreement. A crossing cutter intersects multiple source supports; another cut edits that result, with exact retained volumes 40 and 32. The deformation has unit Jacobian, so these reference volumes are also world volumes.

API entry points: `crates/brep-core/src/minkowski3d/curved/difference/boolean.rs`; integration: `crates/brep-core/src/operations/boolean.rs`. Certificate replay, not numerical fitting, authorizes the new path. This report does not certify arbitrary NURBS Boolean or the pre-existing numerical fallback.

# Compiled original chart polynomials for G2 traversal

This is a partial native step toward trimming original supports and sewing a G2 closed body under the 10/mm curvature limit. It does not admit a body or complete the wider CAD/P0/P2-P3 goal. Earlier receipts are preserved.

## Reuse without changing the original functions

The immutable original support pair owns a bounded cache of homogeneous derivative polynomials. Entries bind both complete pcurves by exact binary64 bits, both original chart-span pairs and the normalized parameter interval. A child can reuse only a containing entry with exactly the same curves and spans. The cache stores polynomial enclosures, never position/G2/solid receipts. Fresh whole-edge 3D position ownership remains mandatory in the outer traversal.

Original polynomials are restricted by outward de Casteljau subdivision. Child clock ratios are computed as exact rationals from the original binary64 endpoints, then independently enclosed; no rounded knot ratio or refitted curve substitutes for that clock. Both polynomial and rational pcurves retain their original common homogeneous denominator powers. Every restriction and curve-identity comparison uses the caller's existing coefficient ledger. A failed operation retains consumed work; there is no reset or free retry.

The cache has at most 16 entries and 65536 interval coefficients. At deeper than a 1/16 subinterval of the stored range, composition is rebuilt from original coefficients instead of inheriting arbitrarily deep interval uncertainty. Cache eviction or lack of budget to store an optimization never changes the source geometry.

## Rational chart correction

A unit-weight candidate SURFACE can still have a rational pcurve. Its composed homogeneous derivatives then contain the pcurve's common denominator power. Local Cartesian Jacobian/Hessian hulls now divide that factor out before the chart correction. Exact zero coordinates of original homogeneous derivative nets remain zero for rational pcurves too; nonzero constants still retain their required denominator factor. No near-zero coefficient is snapped.

An initial test demanding one broad cell certify the entire curved rational graph remained inconclusive. That demand is not the cell verifier's contract: actual seam admission must subdivide until every cell is proved or refuse. The final tests require a narrow curved rational cell to qualify and reject changed curvature, and separately require a COMPLETE rational planar seam to pass the normal fresh-position traversal. General curved rational whole-seam admission is not claimed.

## Tests and measured control

The final 23 composition regressions include exact rational-reference checks of cached child jets for polynomial and varying-weight pcurves, nonbinary clock restriction, one-bit identity changes, span/clock mismatch, bounded eviction, zero-work refusal, source immutability, independent transverse/mixed derivative checks and changed-curvature rejection. Five original-position/traversal regressions pass, including the complete varying-weight rational planar seam.

Before the rational chart correction (which changes the rational path), the same unit-weight expanded runout diagnostic improved from 16 to 22 accepted cells at 100 million coefficient work. At one billion it improved from 174 to 246 accepted cells (519 visited, 2056 exact position work). The pending normalized interval is [0.1676025390625, 0.16766357421875]; the preceding range [0.16748046875, 0.167724609375] has normal upper 1.238327633478218e-6 and tensor upper 0.00011824982739713143. These are inconclusive enclosures above the 1e-4 tensor tolerance, not certified geometric defects or physical-length coverage percentages. The first full contact remains unresolved; the second is unvisited.

## Still required

Complete both actual contact seams, bind original saved coedges and reversals with a shared construction coefficient ledger, then perform fresh saved-edge G2, whole-face curvature and global embedding/solid admission. Current candidate construction remains the same 20-span control with variable-width runouts and central R1 law only on x=0.5..1.5. No STEP, document history, WASM/worker/UI result or arbitrary-body/unequal-radius corner completion follows from these diagnostics. Thickness, part acceptance, P0 and P2-P3 remain open.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo` and after the subcommand: `--offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`. Use `test -p nurbs-core --lib curve_surface_composition:: -- --nocapture`, `test -p brep-core --test expanded_contact_runout --test face_curvature -- --nocapture`, and `run -p brep-core --example expanded_contact_runout -- --graph-seams`. An optional diagnostic `--graph-coefficients=1000000000` consumes a larger explicit coefficient ledger under the existing API cap; it never changes body admission limits. Scoped source hashes are not a full shared-worktree build receipt.

## Final gates on the current code

All 36 focused tests pass (23 composition, 5 ownership/traversal, 8 control/field). The expanded candidate still has whole-field curvature upper 9.997898233305802/mm, under the unchanged 10/mm cap, with 5243 cover and 8962 shape cells. Its central R1 contact-center gap remains at most 9.996074188456394e-8 mm; source immutability is asserted.

The final post-correction diagnostic reproduces 22 accepted / 70 visited cells at 100 million work, pending [0.09375, 0.09765625]. At one billion it reproduces 246 accepted / 519 visited, pending [0.1676025390625, 0.16766357421875]. Both terminate with exit 6 after coefficient exhaustion, with 2056 exact position work. First-contact G2 remains unproven and the second contact is unvisited. Neither increases the construction/body admission budget or publishes a result.

A remaining optimization is a separately typed constant-weight graph derivative path: position XYZ numerators are unused in those derivative formulas once fresh 3D position ownership is established. Such a path must omit those fields explicitly, not feed fabricated zeros to the generic homogeneous composition API; general rational traces must retain their complete denominator semantics. This is not implemented here.

# Original spline G2 traces and expanded runout

This is a new qualification slice, not a replacement for the earlier expanded-contact-runout receipt. No closed body, global embedding, STEP, or WASM/UI admission is claimed.

## Implemented

- Exact rational extraction of a single original span of a clamped, continuous multispan pcurve. Original coefficients and weights remain unchanged. Crossing a knot refuses until traversal subdivides; arbitrary nondyadic crossings are not claimed complete. Unsupported/discontinuous curves and exhausted budgets refuse.
- Exact constant homogeneous derivative-net recognition and exact unit/zero polynomial algebra, preserving uncertainty and degree elevation. No near-zero or near-unit coefficient is snapped.
- Non-affine candidate-chart corrections now use whole-cell hulls of the original composed derivative jets on the actual seam interval, instead of hulls of the entire untrimmed surface. Subdivision can refine this enclosure without dropping seam points.
- The expanded polynomial construction now stores unit weights as a NEW proposal, followed by a fresh whole-surface deviation certificate and original-support/radius/curvature proofs. This does not modify an original B-rep support or inherit the old rational-refit certificate.

## Verified scope

The 16 composition tests cover rational original traces, bit-preserving extraction, budget refusal, nonlinear charts, uncertain arithmetic and changed-curvature rejection. Four fresh graph ownership tests cover complete traversal, displaced edges, orientation and zero-work refusal. Terminal logs accompany this receipt.

The candidate uses 20 spans and 1015 poles under the existing row limit. Whole-surface storage deviation is bounded by 3.6291212560982753e-13 mm. The central x=0.5..1.5 R1 law has upper contact-center gap 9.996074188456394e-8 mm under its 1e-7 mm tolerance. Runouts outside that central interval have variable width and do not carry a constant-R1 certificate. The candidate whole-field curvature bound is 9.997898233305802/mm (limit 10/mm), with 5243 cover and 8962 shape cells. Eight control/field tests passed.

## Assembly boundary

Diagnostic graph traversal is distinct from saved-coedge admission. Any integration must bind actual saved pcurves and traversal reversals, preserve literal source supports, account for coefficient work separately, and then pass whole-body curvature and global embedding. No independent seam diagnostic may replace that transaction. Arbitrary rational support cutting, general unequal-radius corners, thickness, full part/history/P0 acceptance and P2-P3 remain open.

Reproduce with the manifest and profile flags shown in the earlier receipt; use `test -p nurbs-core --lib curve_surface_composition:: -- --nocapture` and `run -p brep-core --example expanded_contact_runout -- --graph-seams`. The latter is diagnostic and explicitly reports `closed_body_admitted=false` even if both seam proofs pass.

## Measured proof strategies

At 100 million coefficient work, exact constant-jet optimization alone left the first contact unresolved after 52 cells with zero accepted cells. Adding local candidate derivative hulls accepted 12 cells at the same total work. An experimental direct graph-quotient comparison passed its small regressions but accepted zero cells on this control at the same work, so it was discarded; production retains the measured local-hull path. The diagnostic accepts an explicit `--graph-coefficients=N` within the existing graph API cap; this never changes body admission budgets.

The fresh strict saved-body transaction on the polynomial proposal exits 2: actual edge 17, faces 0 and 7, has proven positions but unresolved G2 after 100000/100000 cells and 131938/1000000 exact work. Its opposite-orientation upper bounds are normal 0.002217199034153839 and tensor 0.004828392257009004, above the admission limits; these remain inconclusive bounds, not certified violations. Later declared seams are unvisited. No global-body result follows from this refusal.

Scoped source hashes identify this slice only; they are not a complete shared-worktree build receipt. The discarded direct-comparison log is retained as negative experimental evidence, and that algorithm is absent from the final code.

The final explicit one-billion-coefficient diagnostic terminates with exit 6: 310 cells visited, 141 accepted, 2056 exact position work; the first full contact remains unresolved when the coefficient ledger exhausts, and the second contact is unvisited. Accepted cells are not a coverage percentage and do not establish a whole seam. Raising work from 100 million to one billion does not upgrade this to admission. Further work must reduce composition cost and/or tighten the correlated enclosure, then bind the result to actual saved coedges before the full body transaction can pass.

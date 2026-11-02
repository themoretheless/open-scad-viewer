# Exact original boundary jets and adaptive G2 proof

This receipt continues the whole-body trim/sew goal. It does not claim an admitted closed solid, arbitrary NURBS fillets, saved-coedge integration, global embedding, STEP or WASM/UI completion. Earlier qualification receipts remain unchanged.

## Native changes

Exact homogeneous composition now restricts a literal unit-weight boundary pcurve to one original tensor row/column before composing each derivative jet. The test is exact: every normalized Bernstein coordinate must be point zero or point one, with a point-one denominator. No tolerance, fit or coefficient snapping is used. Rational denominators and uncertain near-boundary coordinates keep the full tensor path. Derivatives, including transverse and mixed second derivatives, are extracted before boundary restriction. Both normal directions and both endpoints are supported.

The composition ledger charges boundary recognition/copy work and the actual reduced-degree convolution bound; exact preprocessing retains its own charges even when later composition refuses. Original support nets remain immutable.

A cache scoped to ONE composition interval shares original normalized Bernstein powers and basis rows across the six homogeneous derivative nets. It is reconstructed for every call, not shared across cells or curves. Every uncached convolution, scale, accumulation and cached-row copy is charged. Exact constant homogeneous coordinates are omitted from the convolution only for a unit pcurve denominator, then restored from their exact original-net enclosure; general rational traces retain all coordinates. The normalized coordinates already came from exact rational preprocessing, so a redundant interval subtraction of zero/division by one is omitted without changing the original mathematical quantity.

Graph admission first tries the cheaper local-chart enclosure. If that does not meet the requested normal/tensor tolerances, an alternative direct rational graph-jet difference reuses the SAME original compositions, positions scope and coefficient ledger. Exhaustion is never reset. Each alternative encloses the same metric; componentwise minima are used only with the same proven orientation. The earlier failed experiment replaced the local method entirely; this implementation preserves local success and tries the alternative only on unresolved cells.

Owned graph diagnostics retain a pending normalized interval and the last geometric bound failure separately from later budget errors. These are diagnostic locations, not certificates or coverage percentages.

## Verification

Nineteen composition regressions passed. New tests independently compare all six original Cartesian surface jets against an analytic polynomial surface on all four boundaries; nonzero transverse and mixed curvature must survive. Further tests reject rational/uncertain boundary shortcuts, missing work and changed curvature, and verify the full correlated non-affine chart. Four original-position ownership/traversal regressions passed.

Eight expanded-runout/control-field regressions passed afresh. Original source immutability, actual contact roots 0 and 2, central R1 domain x=0.5..1.5 and the whole constructed field remain checked. Storage deviation is bounded by 3.6291212560982753e-13 mm; central contact-center gap by 9.996074188456394e-8 mm; whole-field absolute curvature by 9.997898233305802/mm under the unchanged limit 10/mm (5243 cover / 8962 shape cells). Variable-width ends do not carry a constant-R1 claim.

Before sharing the per-cell basis, at the default 100-million coefficient diagnostic budget, the first contact remains unresolved: 52 visited cells, 12 accepted, 2056 exact position work. The pending range is [0.078125, 0.0791015625]. The preceding wider range [0.078125, 0.08203125] has normal upper 2.657264892530393e-6 and tensor upper 0.00014226418866734026 (limits 1e-5 and 1e-4). This is an inconclusive upper bound, not a certified geometric excess. The second full contact is not visited after the first refuses.

## Open admission

Complete both contact traversals, bind actual saved coedge pcurves and their reversals with a separate shared coefficient ledger, then run fresh saved-seam G2/whole-field/global-body admission. The prior strict body result remains unresolved at saved edge 17; this slice does not override it. Whole-body thickness, global intersections, arbitrary cuts, unequal-radius corners, part/STEP/history acceptance, P0 and P2-P3 remain open.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo` plus `--offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` after the subcommand. Use `test -p nurbs-core --lib curve_surface_composition:: -- --nocapture`, `test -p brep-core --test expanded_contact_runout --test face_curvature -- --nocapture`, and `run -p brep-core --example expanded_contact_runout -- --graph-seams`. A diagnostic success alone never publishes a body. Scoped hashes identify these files, not a complete shared-worktree build receipt.

## Final shared-basis comparison

At the SAME 100-million coefficient budget, shared basis powers and exact constant-coordinate omission increase accepted cells from 12 to 16 (59 visited, 2056 exact position work). The pending normalized range moves from [0.078125, 0.0791015625] to [0.0859375, 0.0869140625]. The last wider bound failure is range [0.0859375, 0.08984375], normal upper 3.597614516985716e-6, tensor upper 0.00018896444603954716. The first full contact still exhausts the coefficient ledger, and the second is not visited. This is measured progress, not whole-seam admission.

Further optimization must preserve full original-source semantics. For constant homogeneous source weights, graph derivative formulas do not need the XYZ position numerators, but the generic composition API must not be fed fabricated zero position jets. A separately typed derivative-only graph path could eliminate that unused composition while retaining the general rational path and fresh whole-edge 3D ownership. This remains future work, alongside actual saved-coedge reversal handling and shared construction coefficient accounting.

The final one-billion-coefficient run terminates with exit 6: 377 cells visited, 174 accepted, versus 312/142 before sharing the basis. Pending range [0.1572265625, 0.157470703125]; last wider failing enclosure [0.1572265625, 0.158203125], normal upper 4.3260937025131006e-6, tensor upper 0.0003853960843805379. All work is charged to the unchanged explicit ledger; the first contact remains unresolved and the second unvisited. These cell counts are not physical-length coverage percentages.

A larger remaining cost is reconstructing the same original polynomial compositions on each child interval. An original-chart compilation cache followed by outward Bernstein restriction could remove this repetition. Any such cache needs immutable source/pcurve/span identity, exact original clock endpoints (no rounded knot ratios), bounded coefficient storage, shared work accounting, and refusal/fallback when its enclosure cannot qualify. It must preserve original knot-branch coverage and saved-edge position ownership. This is a next implementation direction, not an existing certificate.

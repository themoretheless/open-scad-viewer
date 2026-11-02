# Typed original graph derivative traces

This is a native step toward complete G2 seam and closed-body admission, not a completed fillet operation or a narrower replacement for the seven-stage goal. Earlier receipts remain unchanged.

## Explicit position omission

Graph composition now returns a distinct internal `GraphTrace`: optional XYZ position numerators, the actual common homogeneous weight polynomial, and the five original homogeneous surface derivative jets. The generic full composition API still returns all six jets. No fabricated zero XYZ position is passed to the rational quotient formulas.

Position numerators can be absent only when exact original homogeneous-net inspection proves a positive constant surface weight and all five surface-weight derivatives exactly zero. This applies to unit or nonunit constant surface weights, polynomial or rational pcurves. For a rational pcurve, the trace explicitly computes the original common factor Wp^(degree_u+degree_v) and multiplies it by the original constant surface weight; that factor is not treated as one. Actual derivative-weight zeros are checked before constructing the typed result.

Variable original surface weights retain all position numerators and the general quotient formulas. The graph formulas branch on the typed presence of position: all position-dependent quotient terms are omitted only in the proven zero-weight-derivative case. Original-support surfaces, pcurves, coefficient limits, tolerances and fresh whole-edge 3D position ownership are unchanged.

## Cache and metrics

The existing compiled original-chart cache preserves the typed optional position, restricts every actual weight/derivative polynomial outward, and counts only coefficients it stores. It continues to bind full pcurve bits, chart spans and parameter clocks, with the same 16-entry / 65536-coefficient bounds, deep-interval refresh and shared work ledger. Local chart Jacobian/Hessian indexing now uses the five derivative entries directly; the existing rational-pcurve weight normalization remains mandatory.

## Verification

Twenty-five composition regressions pass, including exact reversed clocks. The new nonunit constant-source-weight test compares physically identical surfaces with homogeneous gauges 2 and 1 for polynomial and varying-weight pcurves. The cached-child rational-reference test now checks the explicitly omitted position and the preserved common weight/derivative values. A variable-surface-weight test asserts that position remains present and checks its graph jets against an independent chain rule. Existing original-bit, uncertainty, boundary, budget, changed-curvature and cache-binding regressions pass.

Six fresh original-position/traversal tests pass, including the complete rational planar seam and displaced-edge refusal. These are scoped checks, not arbitrary curved-rational whole-seam qualification.

## Open scope

Both proposal contacts now pass. The expanded control also passes local saved-model sewing: eight faces, six actual saved G2 seams, maximum face curvature 9.997898233305802/mm. Saved proofs bind original coedge direction without rounded reversal, use a shared coefficient ledger, and preserve source supports. This is local closure, not globally admitted solid validity. Global embedding/solid admission remains open. Whole-body thickness, arbitrary contact/trim/corner construction, STEP/history part acceptance, WASM/worker/UI, P0 and P2-P3 remain open. A successful seam diagnostic would still not publish a body.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo` and after the subcommand `--offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`. Use `test -p nurbs-core --lib curve_surface_composition:: -- --nocapture`, `test -p brep-core --test expanded_contact_runout --test face_curvature -- --nocapture`, and `run -p brep-core --example expanded_contact_runout -- --graph-seams`. Scoped hashes identify these files only, not the entire dirty shared worktree.

## Saved-body and original-boundary qualification

`sew_g2_patch_set_with_graph_work` is additive; existing callers retain their API. The saved-model adapter charges shared coefficient, cell and exact work, reserving work for the generic fallback. Partial authored coedge intervals and periodic lifts are explicitly refused rather than inheriting a full-edge G2 proof.

The exact partial natural-boundary predicate reads original control points, weights and knots. It proves monotone restricted traversal, including reverse traversal, without claiming full source-boundary coverage. Existing full-cover predicate semantics are preserved. All 117 CAD predicate tests and all 26 curve/surface agreement tests pass. Three saved-body integration tests pass, including preservation and refusal of mismatched trim clocks.

The expanded R1 control's central interval is x=0.5..1.5 with contact-center deviation at most 9.996074188456394e-8 mm. End runouts have variable width and are not constant-radius R1. Latest completed global audit visits all 28 face pairs: exact joins, trim checks and all face injectivity checks pass; exact lifts remain unsupported on face 0 edge 17 and face 1 edge 20. Pairs [0,1], [0,7] and [1,7] remain unresolved. The body is deliberately not globally admitted. Construction uses 415158946 coefficient work, 5951 cells and 236982 exact work in this control.

Reproduce the saved local/global paths with the common Cargo flags above and `run -p brep-core --example expanded_contact_runout -- --saved-graph-seams` or `--saved-global-audit`. The latter returns nonzero while global proof is unresolved. There is no WASM, worker/UI, STEP or arbitrary B-rep completion claim.

Final repeat of `--saved-global-audit` reproduces the frontier above. Final saved geometric ledger tests: 3 passed. Logs and structured result are stored alongside this receipt.

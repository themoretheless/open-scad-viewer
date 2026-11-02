# Exact source-owned panel curvature

The full seven-stage CAD goal remains active. This step adds a fresh complete-field curvature proof to the previously constructed exact source-owned graph panels. Earlier coordinate-body refusals and prior receipts remain unchanged.

## Exact field, not rounded display

`Patch.inspect_curvature` differentiates the authoritative rational tensor coefficients exactly, then encloses those derivatives outward in binary64 intervals. Adaptive Bernstein subdivision restricts these derivative nets over a complete normalized [0,1]^2 cover. The proof never calls `preview` or reuses an approximate Surface certificate. The same nonsingular graph-to-world shape operator is used as in the existing kernel.

The conservative symmetric world-tensor spectrum implementation was moved unchanged from B-rep `face_curvature` to NURBS `surface_shape_operator::absolute_spectral_bounds`; the existing B-rep function delegates to it. A proposed floating Jacobi frame is still checked through interval transformed tensors and a positive Gram-matrix lower bound. Floating rotations are not assumed exact or orthonormal.

Construction, previews, joins and field derivative/subdivision coefficients share the same Budget. A field report is Within only after every cell is covered. Cell/work exhaustion, arithmetic uncertainty or a parameter-resolution frontier returns Unresolved without a whole-field upper bound. A certified cell lower bound above the threshold yields Exceeds and its region. Regions are normalized patch parameters. This ledger counts construction/generated coefficients, not PredicateContext arithmetic operations.

## Current evidence

All 20/20 exact source-owned panels pass the unchanged 10/mm limit. The largest certified upper bound is 8.402466975075457/mm. The same run confirms 40 whole-polynomial source contact two-jets and 19/19 exact internal positional boundaries. Total shared charged coefficients: 166120 under the explicit 10000000 budget. The original sources are unchanged. See result.json for per-panel work, complete cover counts and bounds.

Six exact graph construction/field tests pass, including the independent analytic geometry check, complete coverage, insufficient cell/work budgets and a certified curvature excess. All seven existing B-rep face-curvature tests pass after the shared spectrum move. No test threshold or geometric tolerance was relaxed.

Run with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. For B-rep checks use the same target and `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, then `test -p brep-core --test face_curvature` or `run -p brep-core --example expanded_contact_runout -- --exact-source-curvature`.

## Open scope

The supported construction is still the explicit polynomial graph class described in the previous receipt, not arbitrary rational NURBS. Internal panel joins are proved C0 only; their G2, original multispan width ownership, exact representation persistence and B-rep face/shared-edge transport remain open. The 20-panel field proof does not certify retained faces, caps/corners, global inter-panel/body intersections, thickness, radius or solid volume. The assembled coordinate body is not admitted, and WASM/worker/UI, STEP/history, full part acceptance, P0 and P2-P3 remain open. Source hashes cover listed files in a shared dirty worktree, not an isolated delivery revision.

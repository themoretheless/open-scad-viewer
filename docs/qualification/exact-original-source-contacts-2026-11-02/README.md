# Exact source-chart contact bindings

The full seven-stage CAD goal remains active. This step constructs exact original-source UV contact curves and binds them to the actual saved longitudinal edges of the native exact transition strip. It does not yet create trimmed retained source faces or a closed solid. Prior receipts and coordinate-model refusals are unchanged.

## Full-interval original-source lift

`Patch.source_contact` constructs a source-chart polynomial pcurve from the original width definition. Physical U Bernstein coefficients are retained as exact rationals, including non-binary64 values such as 1/3; no ordinary Curve fit or floating pcurve re-encoding is used. The source snapshot is the actual immutable source Surface owned by the qualified graph Patch.

A private SourceContact owner is created only after a fresh exact polynomial check. The verifier checks actual U-clock linearity and source-chart membership, restricts the actual original source q/h coefficients to that U interval, composes q-h*V, and compares every XYZ coefficient with the canonical world edge. This is a complete interval identity, not point sampling. Immutable original world/UV roots and rational range are retained. Child restrictions map back to those roots and recheck the actual source lift.

This verifier relies on the existing private Patch constructor's explicit polynomial graph-class preconditions; it is not a verifier for an arbitrary independently supplied NURBS surface. Source geometry and the 10/mm threshold are unchanged.

## Saved B-rep binding and invalidation

`Strip.source_contacts` examines the actual saved longitudinal perimeter uses, including their coedge direction and rational child intervals. Every SourceBinding compares the source-contact coefficients and root interval with the actual saved canonical edge before publication. `is_current` binds the actual panel Arc, original edge-root Arc, current range, world coefficients and source snapshot. A changed edge interval invalidates the old binding, while an unrelated unchanged binding can remain valid. These are native geometry-scope guards, not a completed worker/UI request-generation protocol.

## Evidence

Ten native construction/field/join/root/source-contact tests and five B-rep integration tests pass. The new native test checks an independent analytic non-dyadic contact endpoint and refusal after changing an actual UV coefficient; zero-budget restriction refuses. The B-rep test verifies exact saved-world/source agreement, binding invalidation after an interval edit, reconstruction of all child bindings and zero-budget refusal.

The final expanded split model has 41 exact original-source contact bindings, all current, alongside 85 actual face-boundary uses, 20 faces, 64 edges and 45 exact vertices. All 19 internal surface-pair G2 joins remain admitted and the maximum complete-panel curvature upper remains 8.402466975075457/mm below 10/mm. Shared charged coefficients: 612351. The 43 outer edges are still unsewn; shell.closed=false and bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. For B-rep use the same target plus `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, then `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining body work

Create retained original-source face loops with these exact UV curves, construct the end-rim source uses and sew the remaining perimeter. Source-contact curves alone do not certify trim-region validity, end G2, external intersections, thickness, radius or solid volume. General rational NURBS/algebraic endpoints, mixed-radius complex corners, persisted document identities/restore, coordinate Model integration, WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 remain open. Scoped hashes identify files in a shared dirty worktree, not a delivery revision.

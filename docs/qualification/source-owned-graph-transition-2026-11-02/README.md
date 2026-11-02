# Source-owned exact graph transition

The full seven-stage CAD objective remains active. Previous receipts and the previously refused global coordinate-body admission are unchanged. This step constructs exact transition geometry from the actual original source graphs, rather than only diagnosing a rounded contact mismatch.

## Native geometry

`nurbs_core::original_graph_transition::build` creates an immutable patch that owns its original source surfaces, original width segment and exact rational tensor coefficients. It restricts the original source Bezier coefficients in exact arithmetic, forms the source q/h graph and width products, and constructs the transversal quartic. No rounded Cartesian contact curve, fitted transition or sampled source point supplies its contact authority.

The currently qualified class is explicit: opposite nonperiodic polynomial graph supports, Bezier U degree 1..12, linear V with physical X=U and Y=+/-V, matching q(U)-h(U)V heights and constant positive weight gauge. The scalar width is an original positive polynomial Bezier, degree at most 8, inside the source U chart and with poles in (0,1]. Other source classes refuse. This is not arbitrary NURBS support completion.

Both source contacts pass whole-polynomial positional two-jet identities. Bernstein endpoint derivative formulas check C0, first transversal derivative and second transversal derivative. Their identities over the entire U polynomial also bind the along-U and mixed derivatives. Monotone X and strictly positive width give a regular graph parameterization. These facts do not grant a curvature bound, global embedding or solid admission.

Adjacent exact panels can freshly prove the full transverse positional boundary identity with `joins_position_exact` under the same construction budget. This is C0 closure only, not an internal G2 certificate. The exact source restriction currently uses authored width pieces; cross-piece higher-order qualification and original multispan width ownership remain separate work.

`preview` returns an ordinary binary64 Surface plus a conservative whole-patch positional error derived from exact coefficient differences. It is display/storage approximation only: neither that rounded surface nor its error receipt inherits exact contact authority. No existing Model, exact boundary gate or curvature threshold was weakened. Source surfaces remain unchanged. The construction has shared coefficient and bit limits; it is not the production PredicateContext operation ledger.

## Verification

Five native tests pass: non-dyadic exact coefficients and conservative preview error; an independent closed-form x^2 graph/width/quartic check; complete adjoining transverse boundaries including moved-boundary refusal; unsupported source/zero-width refusal; and a shared budget that is not reset for preview. Input sources and widths are preserved.

The actual expanded control produces 20 source-owned exact panels. All 40 longitudinal source-contact two-jet identities and all 19/19 internal positional boundaries pass. Construction, previews and join checks share 32953 charged coefficients under a 1000000 limit. Maximum whole-panel preview position error is 1.1453004973069396e-16 mm. Both `curvature_admitted` and `closed_body_admitted` remain false.

Reproduce tests with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. Reproduce the expanded source-backed construction with the same target and `cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example expanded_contact_runout -- --exact-source-panels`.

## Remaining integration

Carry this exact surface/edge representation through B-rep faces and shared edge ownership, storage/restore, trimmed-source sewing and cap/corner construction. Requalify internal G2, the complete exact surface field against the unchanged 10/mm cap, radius, thickness and global intersections before admitting a body. Old rounded-model certificates do not transfer to this new exact geometry. Native construction alone is not WASM/worker/UI, STEP/history, part acceptance, P0 or P2-P3 completion. Hashes cover only listed files in the shared dirty worktree, not a delivery revision.

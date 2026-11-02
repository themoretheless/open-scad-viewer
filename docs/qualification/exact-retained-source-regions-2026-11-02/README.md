# Exact retained original-source trim regions

The full seven-stage CAD objective remains active. This step constructs two retained regions with exact UV and world boundary loops on the original graph supports. They are NOT yet appended to the transition shell; there is no closed solid. Prior receipts remain unchanged.

## Original-source trim construction

`Strip.retained_regions` consumes fresh exact source-contact and source-end bindings. For each original support it orders the contact chain in increasing U, chooses the proper end-use direction, and completes the original rectangular chart boundary with exact original-source lines. All loop joins must agree EXACTLY in BOTH UV and world coordinates. Natural source lines are constructed from actual source coefficients, including decreasing-U and fixed-U cases; no rounded trim, sampled world vertex or spatial welding supplies authority.

The accepted trim class is a single graph notch strictly inside the original U rectangle: contact U clocks are monotone, exact adjacent ends coincide, and all V Bernstein poles lie strictly between 0 and 1. The source ends supply monotone vertical connectors. Together with the original rectangle this proves a simple connected retained region without holes or self-touching contour segments. Arbitrary trim arrangements, touching an outer chart boundary and general NURBS regions remain unsupported.

Exact projected/UV area is rectangle area minus the integral of the contact-width graph. Each curve uses its actual linear U clock and exact Bernstein mean. Area coefficients have an explicit 8192-bit guard; this structural area arithmetic is separate from the native generated-coefficient Budget. The returned area is PARAMETER AREA, not 3D surface area or volume.

The immutable RetainedRegion owns the original source, actual contact/end bindings and natural line definitions. `is_current` rejects a changed geometry scope. Updating one original contact invalidates the affected region while leaving an unrelated region current. This is native scope validation, not a completed worker/UI revision protocol.

## Evidence

Twelve native construction/join/field/source-line tests and eight B-rep tests pass. New native tests compare diagonal, reverse-U and fixed-U source lines against an independent analytic graph, including nonunit source-weight gauge and refusal cases. The retained-region integration test checks exact closed regions, known UV area 7/2 for the constant-width fixture, subdivision-invariant area, invalidation/reconstruction and insufficient budget.

The actual expanded split control produces two current exact source regions with 28 and 27 contour segments. Both UV areas equal 4619845248697188285017/1180591620717411303424. Its complete 45/45 transition perimeter remains bound to original sources. Transition topology remains 20 faces, 66 edges, 47 vertices and 87 exact uses, with 19 within-tolerance G2 joins and complete-panel curvature upper 8.402466975075457/mm below the unchanged 10/mm limit. Shared charged generated coefficients: 894861. The new source regions are standalone owners; shell.closed=false and bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. With the same target plus `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, use `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining sewing and admission

Attach the retained regions as source-face payloads, bind their natural outer boundaries to original source edge identities and append the remaining original faces. Then qualify original-face fields, all body interfaces, global intersections, orientation, radius, thickness and solid volume. Closed UV loops and positive area alone do not certify these. General rational NURBS/algebraic endpoints, complex mixed-radius corners, document identities/persistence, coordinate Model integration, WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 remain open. Scoped hashes describe files in a shared dirty worktree, not an isolated delivery revision.

# Exact original-root boundary splitting

The full seven-stage CAD objective remains active. This step extends the native exact graph B-rep strip with rational endpoint restrictions shared by neighboring faces. It is not arbitrary rational NURBS trimming or a completed solid.

## Exact root and topology ownership

`BoundaryDefinition.restricted` applies exact rational de Casteljau restriction to the original polynomial coefficients and maps the original UV line ends with the same exact clock. A child curve uses a fresh normalized [0,1] clock. Original interval endpoints need not be representable in binary64. No Curve::trim, float endpoint, Cartesian point fitting or spatial welding supplies identity.

Every saved edge now owns an immutable original root-curve snapshot, a strip-local root identifier and a rational interval on that original curve. `inspect_geometry` reconstructs that root restriction freshly, compares all saved coefficients, checks exact vertex endpoints and independently verifies the actual face-local UV restriction against the original full surface boundary. Coedge reversal is handled separately from parameter range direction. Arbitrary CoedgeTrim metadata still does not inherit this proof.

`Strip.split_edge` changes a cloned candidate topology transactionally. It creates one shared exact vertex, preserves the original edge index for the left child, appends the right child, maps both children back to the original root, and updates ALL one/two incident coedges in their actual traversal order. Only after generic topology and full root/lift/vertex checks pass is the candidate committed. A failure spends bounded work but leaves the Strip unchanged. Surfaces are unchanged immutable Arc owners, so their full-field and whole-surface-pair G2 reports remain bound to the identical geometry and cover the resulting subarcs.

Input rational denominators must be positive, including rejection of manually constructed zero-denominator values. Existing bit/coefficient limits remain active. The same shared Budget covers construction, restrictions and fresh validation.

## Evidence

Nine native construction/join/field/restriction tests and four B-rep integration tests pass. An independent analytic boundary formula checks non-dyadic 1/3 and 2/3 ends. B-rep tests verify repeated splitting of a shared child edge, immutable original root retention, identical root ends for both faces, opposite coedge directions, unchanged surface ownership and field/G2 evidence, an open-rim split, and rollback under zero budget. Wrong saved vertices/UV uses, unsupported trim metadata, unjustified closure and invalid denominators refuse.

The final expanded control splits one shared edge at original parameters 1/3 and 2/3 (both neighboring uses), plus one outer-rim edge at 1/3. It finishes with 20 faces, 64 edges, 45 exact vertices and 85 fresh boundary-use checks. All 19 surface-pair G2 joins remain within tolerance and the complete-panel curvature upper remains 8.402466975075457/mm under the unchanged 10/mm cap. Shared charged coefficients: 482109. The 43 outer edges are the still-unsewn perimeter after subdivision; this is not a newly created gap. Shell.closed=false and bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. With the same target and `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, use `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining scope

Use these exact partial boundary carriers to bind the end rims to trimmed original source faces, then sew and qualify the complete body. The root identifiers here are strip-local, not persisted document TopoIds. General rational NURBS curves, algebraic root-equation endpoints, source-face payloads, end/corner construction, persistence, global radius/thickness/intersection/volume checks and solid admission remain open. Existing coordinate Model and WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 are not qualified by this result. Hashes describe listed files in a shared dirty worktree, not a delivery revision. Prior receipts are unchanged.

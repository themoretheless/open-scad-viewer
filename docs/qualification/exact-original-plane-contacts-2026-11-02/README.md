# Exact original-plane contacts: native control-shell embedding

The full seven-stage CAD goal remains active. This closes the fifteen remaining ORIGINAL-source face-pair contacts and proves global embedding for the present native polynomial graph control shell. It does not qualify general rational/multi-span NURBS, fillet radius/thickness, material volume, a published body or the application/STEP pipeline. Earlier receipts remain unchanged.

## Supporting-plane isolation

`polynomial_supporting_trace` checks exact original clamped polynomial Bezier support coefficients. Either the entire support lies in the selected physical coordinate plane, or its complete intersection with that plane is one natural parameter boundary and EVERY coefficient away from that boundary has one strict sign. A zero off-boundary coefficient or mixed signs refuses the certificate. No epsilon, sampled normals, endpoint-only test or assumed planar adjacency is used.

`ClosedShell.inspect_original_pairs` considers actual shared world-edge coefficient planes. Both original supports must lie in opposite halfspaces, or one must lie entirely in the plane and the other have a qualified boundary trace. Two coplanar supports or supports on the same nonzero side are not accepted. Consequently any intersection must be on the selected plane.

## Complete actual trimmed trace, including isolated touches

For at least one boundary-trace face, `polynomial_trim_trace_covered` checks the complete actual directed UV loop. All UV controls must lie on the allowed side of the natural chart boundary. A curve with all controls on the boundary must be a designated actual shared edge, with an exact affine nonconstant remaining parameter clock. A non-boundary curve with one strictly interior control cannot touch the boundary in its open parameter interval because all Bernstein basis functions are positive there. Its endpoints can still touch, and EVERY such endpoint must lie in the union of designated shared-edge intervals.

Thus neither missing boundary intervals nor isolated vertex touches elsewhere can be hidden by an existing shared edge. The two retained original-crease intervals are handled as two distinct closed intervals: the source-notch middle is removed, and the global notch end touches must coincide with their covered endpoints. Source UV containment and domain simplicity are prerequisites supplied by the immutable graph-notch owner or the previously qualified convex original imported face. Fresh saved native topology/UV/world lifts are rechecked before qualification.

A supporting-plane certificate together with complete covered trace proves the entire pair intersection is confined to the actual shared edges. Original G0 ownership proves those edges belong to both faces. Global embedding then combines every pair certificate with complete support-chart injectivity and the native closed, connected, oriented manifold topology checks. The result is restricted to this exact native representation; no approximate coordinate Model is substituted.

## Terminal evidence

27 focused NURBS unit tests, one separate isolated-touch regression and 13 B-rep integration tests pass: 41 tests total. New native tests exercise both natural parameter directions, the whole-plane case, non-boundary point contact refusal, a mixed halfspace coefficient of -2^-100, exhausted work, both retained original-crease intervals, missing interval coverage and falsely claiming a non-boundary end curve as a covered edge. The separate regression has a genuine covered boundary segment plus an isolated boundary touch elsewhere; the coverage proof rejects it. The full two-panel B-rep control qualifies all fifteen original pairs, preserves the two distinct crease edges, proves all nine face charts and refuses insufficient pair/geometry budgets. Its independent exact volume check remains green.

The expanded control keeps 27 faces, 82 edges, 57 exact vertices and zero open edges. All 351 face pairs are accounted for: 277 have strict hull separation, 19 meet only on admitted transverse boundaries, 40 meet only on admitted transition/source-notch boundaries and 15 meet only on admitted original-source boundaries. There are ZERO unresolved or omitted pairs. All 27 whole support charts are injective. `global_embedding=true`, but `solid_admitted=false` and bodies.len=0.

Nineteen internal G2 joins remain valid and maximum whole-support curvature upper is 9.867331764661289/mm under the unchanged 10/mm cap. No surface, source-document geometry or oriented flux was changed. Shared charged coefficient work after the full diagnostic sequence is 2507291.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib` and the same flags with `--test exact_support_plane_trace` instead of the lib filter. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and run `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining full goal

Finish explicit material/volume admission, radius qualification and wall thickness before exposing the result as an admitted application body. The exact positive oriented flux from the preceding receipt is available, but no body/volume admission owner is created here. General rational/multi-span trimming, arbitrary contact branches, unequal-radius complex nodes and 5+ corners remain open. WASM/worker/UI, STEP/independent measurements/Undo/Redo/reload, three-part acceptance, P0 and P2-P3 remain open. Source hashes cover selected files in a shared dirty checkout, not an isolated delivery revision.

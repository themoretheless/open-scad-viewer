# Exact transition faces on shared B-rep topology

The full seven-stage goal remains active. This is an OPEN transition-strip owner, not the finished trimmed-source fillet solid. Prior coordinate-body refusals and earlier receipts are unchanged.

## Native topology transport

`brep_core::exact_graph_strip::assemble` puts original-source-owned exact graph patches into the existing generic `brep_topology::Model`. Faces own immutable Arc<Patch> surfaces. Canonical polynomial 3D edges and vertices retain exact rational coordinates; face-local parameter lines retain exact rational UV coordinates. No ordinary Surface/Curve rounding supplies topology geometry or sharing authority.

The constructor freshly requires original contact two-jets, every internal G2 join, and every whole-panel field below the unchanged 10/mm limit under the caller's shared Budget. Shared cross edges have one index and opposite coedge traversal on neighboring faces. Boundary identities are construction identities, not nearest-point welding.

`inspect_geometry` reruns generic topology validation and actual saved face-use geometry. It matches the saved UV direction against the immutable actual patch boundary, compares every canonical edge coefficient with the full surface lift in the proper direction, and requires exact edge/vertex endpoint ownership. Partial CoedgeTrim metadata and unsupported holes are explicitly refused rather than inheriting a full-boundary proof. This is local incidence/lift validation, not an arbitrary-model embedding or volume certificate.

The Strip owner only exposes immutable topology and fresh reports. Edited clones do not inherit admission. The constructor deliberately creates an open shell with no Body, and the generic validator rejects marking its remaining boundary closed.

## Evidence

The expanded control now has 20 exact B-rep faces, 61 canonical edges, 42 exact vertices and 80 fresh full-boundary lift checks. All 19 internal joins pass G2, all 20 panel fields pass 10/mm with maximum certified upper 8.402466975075457/mm. Shared construction/preview/join/field/topology coefficient work is 211047 under the 10000000 budget. The 42 outer perimeter edges remain open; shell.closed=false and bodies.len=0. The original source model is unchanged.

Two integration tests pass. A two-panel strip proves shared edge identity and opposite directions, full saved-boundary checks and field/G2 gates. Negative cases reject a 2^-100 vertex displacement, a wrong UV use, an unjustified closed-shell flag, partial trim metadata and insufficient budget. Inputs remain available through the immutable owner; there is no coordinate-model solid publication.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test exact_graph_strip`, or the same flags and `run -p brep-core --example expanded_contact_runout -- --exact-source-brep`.

## Remaining body construction

Bind the open exact perimeter to trimmed original source faces, retain source/root/topology identities and construct the end/corner interfaces. General root-valued partial boundaries are not covered by this full-rectangle boundary transport. Source-face/edge variants, persistence, general rational NURBS, unequal-radius complex nodes and full global body/radius/thickness/intersection/volume qualification remain open. The existing coordinate Model used by worker/UI was not replaced. WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 remain unqualified. Hashes cover listed files in the shared dirty worktree, not an isolated delivery revision.

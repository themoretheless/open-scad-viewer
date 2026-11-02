# Frozen native graph body and original identities

Added a codec-gated Rust transport for the native convex degree-six graph body. A frozen artifact contains the original B-rep source, ordered support indices, original width curves, exact rational root cuts, volume bounds and the versioned profile recipe. It also contains exact output geometry: rational vertices/edge coefficients/root intervals/UV trims, transition coefficients, retained/imported supports, incidence and signed volume. Original edge, vertex and face identity links are explicit and compared during replay. Original binary64 support inputs are recorded by their exact bit patterns in the geometry snapshot.

Restore does not deserialize a native success flag. It validates the bounded schema and canonical rational strings, rebuilds exact transitions from original source inputs, reapplies root cuts using local parameters derived from their root intervals, trims/sews the original supports, and reruns volume/orientation/global embedding/G2/whole-support curvature admission. The newly proved geometry and identity links must equal the stored snapshot. Changed geometry, identity mappings, profile versions, duplicate cuts, noncanonical fractions and exhausted limits refuse restoration. Degree-six profile coefficients are pinned to this format version; a future constructor change cannot silently reinterpret a saved recipe.

The artifact preserves a native geometrically admitted body, not a fully qualified constant-radius fillet or application mutation. Radius-law/application admission flags remain false. Thickness certificates are not cached or transferred by this format. Retained-source/natural-rim seams are not all claimed as full-boundary G2; the existing native graph construction keeps its 19 internal G2 seams and source-contact jets distinct from original body edges.

## Validation

The resumed small-body suite passes 19 tests (16 existing exact-strip regressions and 3 transport tests). Binary and JSON replay retain shared non-dyadic root partitions [0,1/3], [1/3,2/3], [2/3,1], exact signed volume and original identities. Tampered geometry and source identity claims refuse. Work, pair and byte exhaustion cannot borrow success from a previous artifact. Paused runs were stopped and are not counted as successful evidence.

The expanded 27-face replay is qualified separately in the final result/log. Existing checks cover all 351 distinct face pairs, the explicit source-derived radius law and aligned-material thickness separately; serializing geometry does not promote those radius/thickness scopes. WASM/worker/UI routing for this closed-body format, STEP and part/history acceptance remain open.

## Completed resumed expanded replay

The final artifact is 585259 binary bytes. Fresh restore gives 27 faces, 82 edges, 57 vertices and one body, with exact equality of volume, all geometry and original identity links. All 351 distinct face products are re-proven. The combined original qualification, frozen construction and restored-body qualification consume 9074749 of a single 10,000,000 coefficient budget, without resetting it. Maximum whole-support curvature remains 9.867331764661003 mm^-1. Radius-law and application admission remain false.

Reproduction: `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example expanded_contact_runout -- --exact-source-brep-split --convex-g2 --frozen-graph-body`. The byte count describes the actual in-process binary roundtrip; a standalone large-body artifact file is not included in this receipt.

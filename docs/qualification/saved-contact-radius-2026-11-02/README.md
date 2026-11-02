# Saved-topology contact radius laws

`fillet_radius::inspect_model_skins` accepts one actual saved transition face for each contact edge. It derives both source-support and transition pcurves from real saved coedges, then reuses the whole-interval contact-center law proof with both original surfaces and fresh contact lifts. The previous single-face API delegates with repeated skin face and retains its behavior. Original law domains remain explicit, without trim renormalization.

A real planar-face diagonal cut produces two independent transition-face descendants. R1 contact centers are admitted for exactly one of the four support-normal sign choices; swapped face roles are refused. Model codec reload retains the result; zero work returns Unresolved. This is a contact-law fixture, not a circular or G2 skin certificate.

The compatible closed curved-NURBS G2 smoothing body remains G2/curvature admitted but its nominal R1 contact law is definitively rejected: normalized parameter 0 has center discrepancy >=0.49999999999999856 mm. The smoothing is not a prescribed R1 fillet.

Native tests passed: compatible_contact_body (1), radius_transition_body (2), saved_skin_radius (3), fillet_contact (8), saved_contact_radius (1). Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test NAME -- --nocapture` for each named target.

Still open: construction meeting both the requested radius/shape constraints and G2/curvature admission for general NURBS, arbitrary end transitions and unequal-radius complex nodes; whole thickness qualification, WASM/UI, three-part STEP/history acceptance, full P0 and P2-P3.

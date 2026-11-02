# Contact sewing bindings

Both complete original contact curves are now bound through actual face-cut and sewing maps to saved edges. Original support-face identities determine side ordering; cut-request order is not trusted. Admission refuses missing or ambiguous contacts, removed contacts, collapsed sides, or an interface that does not have exactly one retained-support use and one replacement-face use. Existing fresh G2, whole-face curvature <=10/mm, closure and global embedding admission remain mandatory.

Verified: compatible_contact_body (1 test), convex_contact_branch (15 tests). The compatible curved-NURBS control remains an 8-face closed body, with 6 G2 seams and curvature upper bound 9.867331764661289/mm; all 28 face pairs classified. This is a bounded smoothing control, not a proof of arbitrary NURBS fillets, prescribed radius, unequal-radius corners, thickness or WASM/UI delivery.

Command: `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test compatible_contact_body --test convex_contact_branch -- --nocapture`

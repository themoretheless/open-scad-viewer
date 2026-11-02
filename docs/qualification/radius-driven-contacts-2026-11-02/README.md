# Radius-driven contacts on original curved NURBS

The compatible curved-NURBS control previously used constant-width contacts that do not satisfy R1. This work constructs new contacts from the offset-support intersection for the requested radius instead. The source contains degree-eight curved NURBS supports and a genuinely curved common edge. The selected numerical search window is x=0.5..1.5 in original support coordinates; no source model surfaces are replaced.

## Construction changes

- Constant-radius open G2 proposals on curved supports now use the same matched section/UV Hermite interpolation as variable-radius curved proposals. Constant radius does not imply straight contact coordinate fields. Affine pairs retain the previous simpler route. Natural loft stays bounded at 11 sections; local curved G2 interpolation has a separate 33-section proposal cap. Existing contact/G2/radius proof limits are unchanged.
- `fillet_contact::build_g2_on_domains` restricts the numerical search using temporary trimmed carriers, then freshly requalifies complete contacts against the immutable ORIGINAL support coefficients. The two contact passes share the existing 100k-cell / 1M-exact-work ledger. Temporary trim rounding is never original-source authority. Radius, branch ownership and G2 still require separate checks.
- C2 storage conversion refits once into the final knot multiplicities, then certifies the returned surface directly against the ORIGINAL surface. `remove_surface_knots_report` accepts only a smaller multiplicity-subset and unchanged clamped ends. A failed error bound returns the original geometry. The contact builder retains its 2M coefficient-work ceiling. This replaces repeated whole-surface proofs after individual knot removals; no sampled or accumulated fitted-position claim substitutes for the final bound.

## Evidence and boundaries

See result.json and compressed logs for final numerical results. The automatically chosen 21-section R1 branch has contacts on the original curved supports and a complete contact-center discrepancy below 1e-7 mm. Both complete seams are also independently checked against the original supports with position 1e-7 mm, normal 1e-5 and shape-tensor 1e-4 tolerances. Contact, radius and seam phases have explicit separate ledgers.

The earlier 8-face closed smoothing control remains globally admitted, G2 and curvature bounded by 9.867331764661289/mm; it is NOT the new radius-driven transition and is still NOT an R1 fillet. No new closed R1 body is claimed here. Whole-field curvature of the new transition, root/material ownership, trimmed source-face assembly, end transitions, complex unequal-radius corners, thickness, global embedding and STEP acceptance remain required. A contact-center law is not an interior circular-skin-radius certificate. WASM, UI/history, full P0 and P2-P3 remain open.

The diagnostic example exits 0 only when a contact-center law is proven; printed G2 results and whole-body admission remain separate. Reproduce using `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example radius_driven_contact`. CONTACT_SECTIONS optionally controls the bounded interpolation proposal.

A one-character incidental compile repair removed an extraneous reference in the rational comparison in retained_boolean/propose_arcs.rs. Its arithmetic was not changed. No full Boolean qualification is inferred.

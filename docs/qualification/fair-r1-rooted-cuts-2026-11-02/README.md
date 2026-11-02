# Fair R1 transition and root-owned source-face cuts

The previously constructed automatic R1 transition passed contacts and G2 seams but failed its whole-field curvature gate: a certified witness exceeded 22.214768/mm against the required 10/mm. The active `section_geometry` implementation used tangent handles of 0.35 chord. In the shallow-wedge limit this reduces midpoint speed to 0.34375 of mean speed and amplifies curvature to about 22.2149/R. Handles of 0.2 chord preserve the endpoint position/tangent/curvature formulas and give about 1.5/R in that limit. This is a changed geometric proposal, not a tolerance increase or a universal curvature guarantee.

The original curved-NURBS R1 control now passes full natural-patch coverage with absolute curvature upper bound 3.4888470400359064/mm. It retains the proven original-support contact-center law and both whole G2 seams. The natural-boundary model used for field checks is an actual open sheet with extracted iso edges, actual corner vertices, directed pcurves, and no bodies; it is never presented as a closed solid.

## Source ownership and cuts

Contact traversal can now oppose original-edge parameter direction. `convex_branch::Ownership.contact_reversed` records this; outward source endpoint brackets stay ordered along the ORIGINAL edge. The original curves/pcurves are freshly checked unchanged. Nonlinear and rational inverse clocks, opposite coedge uses and independently reversed sides are covered by regression.

Material planning follows the contact traversal when assigning root endpoint roles, preserving literal contact coefficients without reversing knots or fitting. Unequal complete contact spans are supported by fresh ORIGINAL-support projection and actual UV partition qualification. They must enclose a guaranteed common source interval; the unique discarded sector must touch the shared middle edge, while BOTH outside source-edge pieces must stay retained. Material is not deleted at this stage. No endpoint proximity, rounded equality or caller flag replaces these tests.

The automatic R1 control yields two root-owned partitions (9 total faces) with unchanged source-support coefficients. Stored shared vertices are bounded to original source root images within 2.6483e-14 mm. The full-control regression checks all these stages together, original source immutability and deterministic root/cut reconstruction.

## Open end region

This is NOT a new closed R1 body. Both unexpanded ends require additional source material to be included: normal gaps 0.000722013702526506 and 0.0006909837481697888 exceed the existing finite-fan bound 0.0006500000000000002. High-level admission returns EndRegion BEFORE calling the body constructor; the complete regression proves its constructor callback was not called.

Still required: constructive end-region expansion, endpoint transitions and closed sewing, then fresh whole-body G2/curvature, radius support scope, gaps, overlap, thickness and global embedding. General rational NURBS, unequal-radius complex corners, STEP/three-part/history acceptance, WASM/UI, full P0 and P2-P3 remain open. A contact-center R1 law is not proof of a circular interior skin.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101` and `cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` with the package and target names in result.json. Native diagnostic: same Cargo options with `cargo run -p brep-core --example radius_transition_field`. Its zero exit qualifies roots/material/field only, not closure. Logs include all counted test targets; repeated runs are counted once.

## Next construction idea - unimplemented

For this polynomial control, the source crease angle vanishes at x=0 and x=2. A possible runout is to retain the central R1 contact law on its verified interval and expand width smoothly to a positive value at these zero-angle stations, where original source 2-jets match. A positive piecewise-quartic width field can keep the product with the degree-eight source below the current degree-12 field gate. Use a short final runout span to keep its endpoint-jet control coefficients positive, and C2 knot multiplicities between spans. This is a design proposal, not admitted geometry; the runout radius scope, seam jets, full curvature, material sectors, closure and embedding all need fresh proofs.

# Retained sectors on original NURBS faces

## Implemented

`brep_core::face_cut::partition` authors two face sectors along a supplied open
3D cut and original-chart pcurves. Existing outer-loop endpoints are shared
vertex identities; intermediate segments receive shared new vertices. Both
faces keep the complete original surface definition. Original boundary edges
remain unchanged; the path has two oppositely directed owned uses. Face Split
lineage and complete persistent naming are retained.

The source region and both sectors require full trim simplicity, orientation
and containment audits. Holes are assigned to exactly one proven sector by
whole-loop separation and containment checks, not by an unqualified sample.
The actual forward and reversed cut definitions each require exact coefficient
identity or whole-interval lift agreement. One shared cell budget, a 10,000-pair
trim limit and a 1,000,000 exact-work limit apply. Failure returns no model and
leaves the original model unchanged.

Two-curve lens regions now have a separate sufficient proof: positive rational
Bezier convex hulls and outward chord-side signs establish open-half-plane
separation, with independent injectivity and exact joins. Retraced arcs, flat
zero-area retracing and an arc crossing its chord remain unqualified.

The correlated curve/surface verifier now uses its existing composed-degree
ceiling of 32 and source-degree ceiling of 16 instead of separately excluding
surface total degrees above 8 or pcurve degrees above 4. Cross-multiplied
polynomials remain degree at most 48, within the existing exact binomial range.
No geometric tolerance, cell budget or exact-work budget was raised.

## Current evidence

- `brep-core/tests/face_cut.rs`: five tests passed. Rational cubic cut; multiple
  segments with one intermediate vertex; automatic hole assignment; crossing a
  hole, an exterior sector, a corrupted lift, bad endpoints and insufficient
  work refuse; original degree-6-by-6 nonplanar surface splits after both shared
  boundary edges are subdivided. Positive controls pass continuous closure.
- Related edge_partition, face_crossings, face_window, pcurve_geometric_g2 and
  watertight suites: 28 tests passed.
- `nurbs-core/tests/trim_lens.rs` and `curve_surface_composed_degree.rs`: two
  tests passed, including high-degree surface and pcurve whole-domain admission
  within eight cells and an incorrect lift that remains a demonstrated mismatch.
- Existing trim_simplicity unit regressions: four tests passed.

## Remaining requested work

This is retained-sector authoring. It still needs automatic surface/surface
contact-curve generation, material-side selection and removal, replacement
transition surfaces, end transitions and unequal-radius complex corners.
Periodic charts and poles, cut endpoints on holes, and inconclusive trim proofs
are refused. A candidate is not a radius, thickness or global solid certificate.
No independent STEP acceptance, WASM build, worker/UI behavior, three-part/history
acceptance, full P0 matrix or remaining P2/P3 completion is claimed here.

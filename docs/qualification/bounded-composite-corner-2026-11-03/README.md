# Bounded composite corner — native qualification, 2026-11-03

Status: partial. This is not admission of a new closed CAD body.

## Implemented

- A three-part planar proposal: polynomial zero-curvature end runouts and a rational middle arc. Nominal radii remain 0.1, 0.2 and 0.35 mm; original contact tolerances are unchanged.
- Absolute curvature of actual original positive-weight XYZ Bezier coefficients: exact regularity and a Bernstein polynomial inequality, with an outward interval polynomial filter before exact expansion products. Positive results cover the entire closed parameter interval. Point signs can reject, never accept. Resource exhaustion refuses a certificate.
- A binary-grid-aligned linear ordinate prevents cancellation in micro-runout coordinates from introducing actual excessive curvature.
- A newly authored cubic composite is independently rechecked; degree elevation does not transfer the old certificate. Both complete extruded joins are checked against normal tolerance 1e-5 and tensor tolerance 1e-4/mm. Positive balanced clocks condition derivatives consistently without changing stored geometry.
- An explicit alternative strip factory retains original-contact, rolling-center radius, G2, injectivity and separation gates. The existing default factory retains its original profile and proof path.
- Separation can use a complete UV boundary-control hull evaluated on the original nonlinear support. Its coordinate projection gap is an outward lower Euclidean-distance bound. Holes remain filled in this conservative enclosure. All spent hull work is charged to the shared cell budget; inside upper witnesses remain independent.
- Whole original knot cells and short closed endpoint bands retain correlated original affine-pcurve comparisons; incomplete bands cannot grant acceptance. Existing periodic/trimmed agreement regressions remain separate.
- Central layers retain clamped multispan rims, their internal clocks and a common control basis. Six fresh boundary comparisons follow composition; this is not a G2/curvature/embedding admission.

## Checked

89 native tests passed: 4 predicate, 4 profile, 6 original G2-join, 39 agreement and 36 region-replacement tests. Final summaries and source hashes are recorded in `review.json` and `native-results.txt`. The profile tests cover 9 original cells, 9 newly authored composite cells and 6 complete extruded joins. All 18 curve fields qualify at <=10/mm. A separate production diagnostic covers 36 cells across contact tolerances 1e-7 and 2.5e-8 mm. Before the grid correction the 0.1-mm original runout exceeded the cap at the production tolerance; the corrected actual stored coefficients pass. No tolerance or cap was relaxed. See `native-results.txt` for work counts, budgets, negative tests and exact test names.

The region tests include the new multispan boundary and trim-hull tests, plus the previous saved closed-corner, material/intersection/flux and curvature regressions. The previous closed body remains a negative curvature fixture; its successful regression does not mean that body meets the curvature cap.

## Body investigation

The first alternative-profile run passed all three strip gates but exhausted generic separation. An original-support UV hull then established strip separation, reaching the central proposal. The old central implementation refused multispan rims. It has now been extended and its six-boundary regression passes. After equal piece clocks avoided a 1e-9 whole-piece parameter span, all six central boundary comparisons passed. The final probe reaches sewing but explicitly refuses with `NURBS_INVALID_INPUT: Source seam must remain on one original knot cell`. The original-support extrusion/G2 source comparator still accepts only one original knot cell. It must be partitioned into complete, source-owned branches before this new rational composite can be sewn. The exact result is recorded in `review.json`; no new closed-body admission is inferred.

## Still open

First: extend the original-source seam comparator across all original knot cells without inheriting a fitted/sampled proof. Then: a newly admitted complete closed body with all seams G2, full-face curvature <=10/mm, material/volume, gaps, thickness and global self-intersections. Arbitrary NURBS trimming and 5+ unequal-radius nodes remain outside this bounded planar corner evidence. No WASM, worker, UI or STEP acceptance is claimed.

Run the probe with `cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature -- --bounded-profile 0.00390625`. A zero process exit after a printed refused stage is diagnostic completion, not body acceptance.

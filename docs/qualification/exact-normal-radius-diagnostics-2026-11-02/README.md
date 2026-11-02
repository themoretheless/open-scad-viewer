# Exact normal-radius diagnostic: current profile does not satisfy declared R1

The full seven-stage CAD goal remains active. This adds a rigorous declared-direction radius diagnostic to the actual native body. It exposes a shape-quality failure; it does NOT fix that shape or qualify a full fillet. Earlier geometric volume/embedding receipts remain valid and unchanged.

## What is measured

For the declared transverse parameter direction Sv, let C=Su cross Sv, T=C dot Svv and E=Sv dot Sv. Squared normal-curvature radius is |C|^2*E^2/T^2. All derivative Bernstein nets are taken from actual exact rational Patch coefficients. They are restricted in exact arithmetic to the EXPLICIT normalized UV domain; no rounded preview Surface is used.

Outward interval arithmetic checks the polynomial inequalities D-(R-tol)^2*T^2>=0 and (R+tol)^2*T^2-D>=0, with D=|C|^2*E^2 and a regularity enclosure. This avoids dividing by a possibly vanishing curvature. A complete field enclosure can certify a cell within the target. Exact de Casteljau midpoint jets with outward enclosures can provide a counterexample, but a passing midpoint NEVER certifies a cell. Uncertain cells subdivide until fully covered or a budget/bit/numeric limit leaves explicit unresolved regions. Only exhaustion of the entire pending partition produces Within.

Exact rational UV subdivision is retained, and float witness boxes outwardly enclose the actual exact witness. This avoids reporting a rounded coordinate as if it were the exact point. The same shared exact coefficient budget covers construction/proofs. Work or cell exhaustion never appears as a qualified result. Caller-selected G2 rims/runouts are never silently removed from the requested domain.

This is NORMAL radius in the specified Sv direction. It is not automatically a principal radius or a circular-section/moving-center radius under arbitrary surface parameterization. Passing it is not a rolling-ball/canal certificate. Full requested fillet admission still needs the appropriate characteristic frame and center/radius law. The diagnostic deliberately grants no fillet/application status.

## Independent controls and terminal tests

Thirty focused native tests and fourteen B-rep integration tests pass, including all prior embedding/volume checks. The independent flat-X quartic control has x=u, y=(2v-1)/4, z=-phi(v)/16. At v=1/2, z_v=0, z_vv=-3/8 and y_v=1/2, giving normal radius EXACTLY 2/3 mm. The new interval diagnostic encloses 2/3 and rejects R1 +/-0.01 with a certified midpoint witness. A narrow explicit central band passes the declared 2/3 +/-0.01 constraint through complete field coverage, while the full domain fails due to its nonconstant-radius G2 runouts. Zero work/cell budgets remain unresolved, and invalid laws refuse.

The B-rep wrapper rechecks the actual native body geometry and inspects every transition. Its integration control verifies explicit violation witnesses on both panels and refuses insufficient geometry work. Source document, exact volume, body role and geometry are unchanged.

## Expanded actual body: certified failure, not successful radius admission

The expanded native body still has 27 faces, 82 edges, 57 vertices, one body, zero unresolved global embedding pairs, nineteen internal G2 joins and whole-support curvature upper 9.867331764661289/mm under the unchanged 10/mm cap.

For the explicitly configured control constraint R=1 mm +/-0.001 mm on U=[0,1], V=[0.25,0.75] of EVERY transition, all twenty panels have certified violation witnesses. The sixteen middle panels give normal radii about 0.666662..0.666675 mm at their symmetric centers; the four longitudinal runout panels give different, larger values. All witnesses, face indices and outward UV/radius enclosures are in witnesses.json. These are counterexamples to the declared direction constraint, not a sampled pass or a complete classification of every direction in a general canal surface. Work for the full preceding admission and this diagnostic sequence is 3667492 charged coefficients.

Thus geometric body admission is true, but requested fillet radius, wall thickness and application admission remain FALSE. Do not reinterpret the old positive-volume or G2 checks as radius qualification.

## Reproduce and next actual correction

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib`. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and run `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

Rebuild the central transition profile toward the requested characteristic radius while preserving original contact position/tangents/two-jets, G2 runout zones, global embedding, the 10/mm cap and allowed volume. The current quartic profile cannot be passed by increasing tolerance, hiding bands or relabeling curvature as radius. Full constant finite radius at a G2 boundary against a ruled support is incompatible with its zero transverse normal curvature; central radius and explicitly designed runout laws must be qualified separately. A candidate higher-degree or piecewise/rational profile must re-pass the same complete geometry/volume gates and a proper center/characteristic-radius law; this diagnostic alone is not enough.

General rational/multi-span edges, unequal-radius complex nodes and 5+ corners, thickness, WASM/worker/UI, STEP/independent measurements/history, three-part acceptance, P0 and P2-P3 remain open. Listed hashes cover selected files in the shared dirty checkout, not an isolated delivery revision.

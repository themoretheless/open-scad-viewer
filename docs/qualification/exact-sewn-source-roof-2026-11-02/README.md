# Exact transition sewn to retained original source faces

The full seven-stage CAD goal remains active. This step produces an OPEN sewn roof with transition and retained source faces on the shared generic B-rep topology. It does not append the remaining original side/floor faces or publish a solid. Earlier receipts remain unchanged.

## Sewing and ownership

`Strip.sew_retained` reconstructs fresh retained source regions, verifies their current geometry scope, and uses their already-bound canonical transition edge indices. The new RoofTopology has transition-panel and retained-source face payloads and matching parameter-use variants. All transition perimeter edges are shared with their neighboring original-source faces. The two remaining original crease intervals outside the cut are shared by exact original-chart interval identity.

Vertices are assigned by named original source-chart endpoints. The common V=0 source crease has an explicit shared chart key; other source endpoints retain the source side in their key. Existing bound transition vertex identities are installed first. A conflicting identity or exact world value refuses. No Cartesian nearest-point matching or tolerance welding is performed. Retained source face orientation accounts for the opposite original chart handedness.

`inspect_roof_geometry` freshly validates generic incidence, immutable root restrictions, exact edge/vertex endpoints and every actual saved face use. Retained loop order/directions, bound transition edge IDs and actual root Arc/interval ownership are checked. A geometrically identical edge transplanted from a different owner is rejected. Mutable topology clones do not inherit the Roof owner or solid admission.

## Curvature

The existing full-panel exact field certificates remain bound to identical immutable transition geometry. Both retained ORIGINAL supports now receive fresh full-natural-domain shape-operator/spectrum coverage under explicit per-support 20000-cell / 100000-shape-cell limits. This sufficient bound covers a superset of each retained trim domain. Unresolved/exhausted queries refuse Roof publication. A certified support excess above 10/mm is rejected. The ordinary native coefficient budget and source shape-query work are reported separately; no work or tolerance is silently reset.

## Evidence

Ten B-rep integration tests pass, including retained-face sewing, full saved geometry reinspection, Euler disk topology, unjustified closure refusal, wrong source UV order, foreign-root transplant refusal, and positive/negative source field cases. Existing split/contact/trim tests remain green.

The final expanded control has 22 faces, 74 exact canonical edges, 53 exact vertices and 142 checked boundary uses. Every former transition perimeter edge now has two uses. Only SIX original outer roof edges remain open. Complete transition curvature upper is 8.402466975075457/mm. Complete original-support bounds are 9.867331764661289/mm and 9.867331764661287/mm, each covering 45 cells / 45 shape cells, below the unchanged 10/mm cap. Shared charged generated coefficients: 1104773. Original sharp crease intervals are retained; new transition/source interfaces keep their previously proved source G2 and all 19 internal transition joins remain qualified. Shell.closed=false, bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test exact_graph_strip`, or the same flags and `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining whole-body admission

Bind the six original outer edges to original document edge/vertex identities and append the remaining original side/floor faces. Then verify complete closure/orientation, global embedding/intersections, radius, thickness and volume. Local roof topology and face curvature do not certify those. The qualified graph-class trim and direct end construction do not complete arbitrary rational NURBS/algebraic endpoint or mixed-radius corner support. Persistence, coordinate Model integration, WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 remain open. Hashes describe listed files in the shared dirty worktree, not an isolated delivery revision.

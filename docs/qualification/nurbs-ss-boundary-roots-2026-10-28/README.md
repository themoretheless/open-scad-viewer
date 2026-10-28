# Original SS equation roots on rational trim boundaries

`qualify_graph_boundary_roots` isolates a complete census of simple roots of the original qualified polynomial graph difference along an actual native-UV rational Bézier trim definition. The boundary can have varying positive weights and degree one through eight. Its entire image must lie in the chosen original native rectangle; a control pole may lie outside if the existing continuous domain proof admits the image. A unit clock changes the knot domain only and retains every original pole and weight. Original boundary parameter results are outward enclosures, not snapped source parameters.

The equation is derived from the complete original height coefficients with a fresh exact-difference check. Homogeneous polynomial composition retains the actual rational boundary numerator/denominator. The complete graph owner already proves the source difference is quadratic and the component census over its rectangle. Outward Bernstein coefficients, derivative ranges and interval Newton establish original root existence/uniqueness and exclude all remaining clocks. Original height, both independent native domains and world position are enclosed at every accepted root. Localization requires both explicit clock and spatial bounds. The derivative sign of the numerator at a root equals the original graph-difference traversal sign because the rational denominator is positive.

The private root owner retains the original SS owner, original query curve, selected side, limits, roots and exact operation count. Fresh replay reconstructs the roots and their bounds from these definitions. Root records alone do not grant authority. Distinct root clocks must be strictly separated; world proximity never merges them. Failure, insufficient work, unresolved multiple/endpoint roots or a coincident event cannot return a partial complete census. Coincident original surfaces are explicitly refused as a separate overlap-region problem. General rational or multispan original support surfaces are still outside the underlying graph contract.

The shared coefficient/interval operation allowance covers domain proof, polynomial construction, evaluation, Newton steps, subdivision, original root-position enclosure and parameter transport. Depth and tolerances are explicit and validated. Original-owner replay uses its separate original-source contract. No hidden work allowance or tolerance growth is used.

This is a source-bound root census on a query boundary. It does not yet associate each root with an owned SS branch clock, construct a cut representative, split actual face edges, assign material roles, imprint faces, sew a shell or validate a Solid. Existing B-rep trim membership and these root proofs must be composed through a separate constructor before topology can change. No WASM/editor/STEP qualification is implied.

## Verification

The five focused integration tests pass. A private replay test rejects changed root bounds, boundary weights, clocks and rebound original supports. The final wider run passed **3046 tests, zero failed, one ignored**, across 169 result summaries including doctests. Fresh B-rep checks passed **9/9** actual-face membership tests and **53/53** SS/Boolean tests. [matrix.json](matrix.json) records source and log hashes. The original rational line clock has two analytically checked roots; a nonmonotone quadratic trim with an exterior control pole has four independently checked roots. Both native support domains, complete empty exclusions, unresolved tangencies/endpoints, outside-domain input and exact shared work exhaustion are tested.

Rustfmt, diff whitespace and API mapping pass. The whole task snapshot still has 72 architecture diagnostics after the separately recorded stale-pin cleanup and wire-mapper extraction; no numerical limit or architecture size ceiling was raised. Distribution size and full application qualification remain open.

## Qualification procedure

The qualification commands run against the preserved task worktree, which also contains the separately documented continuous-offset work. Final receipts identify the additional modified Rust source hashes as well as this extension; numerical test totals belong to that actual source snapshot. They are not a claim that an isolated milestone commit is the complete deliverable.

```sh
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml --no-fail-fast --offline --locked -p nurbs-core -p cad-predicates -p nurbs-intersect
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml -p brep-core --lib ss_face_membership --offline --locked
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml -p brep-core --lib nurbs_ss --offline --locked
node scripts/map-nurbs-api.mjs --check
```

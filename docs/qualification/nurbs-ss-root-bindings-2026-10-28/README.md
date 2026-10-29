# Original SS root-to-branch and shared-vertex bindings

`bind_graph_boundary_roots` consumes the private complete boundary-root census and binds every original query event to either a strict interior clock of one owned original SS edge or an exact original vertex with all its edge ports. The private result retains the complete census, bindings and bounded operation count. Replay recomputes both original root ownership and the branch bindings. Mutable cloned records cannot authorize a binding.

## Original edge interiors

For closed ellipses, the inverse of the exact original source-coordinate basis recovers enclosed diagonal coordinates. Positive radical radius bounds and strict quadrant signs select one original conic chart. Its clock is `sin/(1+cos)` in that quarter. The original complete conic identity justifies this inverse, not numerical nearest-curve distance. The full original height polynomial and both native domains remain owned by the root census.

For clipped factor graphs, interval carrier residuals and projected segment clocks exclude other original segments. The complete source graph census proves that the original root belongs to one of them; only one remaining candidate with a strictly interior clock is admitted. A box intersection alone does not establish carrier membership. Unresolved segments, endpoints or quadrants fail instead of being snapped.

## Exact shared vertices

For conic vertices, exact affine identities of every actual rational boundary control place the entire query on an original diagonal carrier. The source conic equation has only two cardinal vertices on that carrier. A strict sign of the remaining diagonal coordinate identifies one algebraic original vertex without computing a rounded vertex or radius. The query need not be linear or injective: a zero-chord rational trace can visit the same source vertex more than once, and every separate original root event is retained.

For source factor endpoints, literal native endpoint controls prove that the complete query lies on an original natural rectangle rail. The complete clipped factor graph restricts finite simple roots on that rail to the owned endpoint vertices. Original normalized root bounds distinguish these exact vertices. Endpoint normalization facts remain exact even when subtraction of the native domain endpoints is not representable in binary64. No proximity tolerance proves an incidence.

Every incident original edge port is copied from the original graph, including both adjacent ends at a closed-loop vertex. No different query event is merged by world position or a shared vertex ID. Each binding retains its original root ordinal. Overlapping edge-clock boxes are not an ordering certificate for later cuts.

## Continuous domain proof repair

An original regression exposed an unnecessary refusal for a rational curve with exterior control poles, natural-boundary endpoints and a complete image inside the original rectangle. The shared saved-pcurve domain checker now proves homogeneous inequalities `N-lower*W >= 0` and `upper*W-N >= 0`, with original positive weights supplying `W>0`. Exact checked dyadic arithmetic retains exact zeros when available; all other arithmetic uses outward intervals. Bounded de Casteljau subdivision proves the remaining coefficient inequalities. This admits actual closed-domain contacts while rejecting a real excursion smaller than the requested world tolerance. It changes neither geometry nor tolerance. Both polynomial and unequal-weight rational traces are covered by regressions.

All coefficient construction and subdivision work shares the explicit operation allowance. The existing domain depth limit remains 16. Root-to-branch binding has its own validated allowance, while replay of the prerequisite original root census retains its separate bounded source contract. Insufficient proof or budget returns no partial qualified owner.

## Scope still open

The underlying original SS support family and simple query-root census are unchanged. General rational/multispan original supports, unresolved multiple or query-endpoint roots, arbitrary vertex incidences without the exact carrier proof, coincident regions, actual model/coedge ownership, cut representatives, face imprinting, material roles, sewing and final Solid validity remain separate requirements. This native result is not new WASM/editor/STEP qualification. The other four directions of the full plan and its release gates remain open.

## Verification

Final focused tests passed **27/27** (18 boundary/root/binding tests and nine compact-curve tests). The private tamper test passes: changing edge clocks, normalized root bounds or dropping a shared-vertex port cannot replay. The wider three-crate native run passed **3060 tests, zero failed, one ignored**, across 169 result summaries. The complete B-rep library passed **1893 tests, zero failed, five explicitly ignored**. Source/log hashes and the original domain regression are in [matrix.json](matrix.json).

The regressions independently check analytic original quarter clocks, mixed source bases, a common quartic world height, irrational cardinal vertices, four events on repeated vertices despite zero query chord, sub-tolerance near-cardinal interiors, distinct factor carriers, natural-rail endpoints, support swapping, independent native domains, nonrepresentable native spans and exact shared work exhaustion. Positive unequal rational weights and exact interior domain contacts are tested; excursions smaller than the world tolerance still fail.

Rustfmt, diff whitespace and API mapping pass. The separate SVG file-exchange extraction passes 400 editor/SVG/edited-STEP tests and Vue types. Architecture still reports 71 violations. The current production Vite build succeeds, and packed/streamed geometry WASM match the saved c8 artifact byte-for-byte. The distribution size gate still fails at 5,330,908 geometry chunk bytes against 4,680,000. That artifact does not include this new native extension. The earlier 14/15 focused run exposed the domain refusal and was corrected without changing its source geometry or tolerance.

```sh
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml --no-fail-fast --offline --locked -p nurbs-core -p cad-predicates -p nurbs-intersect
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml --no-fail-fast --offline --locked -p brep-core --lib
node scripts/map-nurbs-api.mjs --check
```

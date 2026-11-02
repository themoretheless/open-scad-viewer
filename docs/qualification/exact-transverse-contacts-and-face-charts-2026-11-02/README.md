# Exact transverse contacts and whole-face chart embedding

The full seven-stage CAD goal remains active. This advances the 74 unresolved face pairs from the previous conservative hull audit and adds whole-face injectivity evidence. It does not publish a solid, qualify arbitrary NURBS or complete the application pipeline. Earlier receipts remain unchanged.

## Entire transverse contact, not sampled agreement

`Patch.transverse_contact_only` freshly proves physical X equals the authored affine U clock throughout both complete exact tensor nets. Adjacent U intervals meet only in their common endpoint plane. The entire end/start boundary coefficient arrays must match in the same parameter clock. Its Y clock is checked to be affine and strictly increasing throughout the transverse boundary, so the common curve has no fold. Thus the complete surface images meet only on that boundary; this is separate from the previously proved G2 and field curvature.

`ClosedShell.inspect_transverse_pairs` first rechecks native topology and saved UV/world lifts through the full hull audit. For each qualifying transition pair it then checks the ACTUAL shared edge uses on BOTH faces: their constant-U lines must partition [0,1] exactly once, with no gap or overlap. Sharing a topological edge by itself proves nothing. Non-dyadic root splits at 1/3 and 2/3 are retained; the first expanded seam has three owned edge pieces and all are covered.

No neighboring transition pair is exempted solely because it is adjacent, has G2 metadata or passes an endpoint test. All unsupported pairs remain explicit unresolved entries.

## Whole-chart injectivity

`PolynomialChartEmbedding` proves that one physical coordinate is an exact nonconstant affine parameter clock and a second physical coordinate is strictly monotone in the transverse parameter. Every transverse derivative Bernstein coefficient must have the same strict sign. The proof covers the COMPLETE closed parameter chart; it gives injectivity and regularity of its 2D projection and therefore of the 3D surface. The remaining physical coordinate can be nonlinear. Parameter axes and physical coordinate axes may be interchanged; either monotonic direction is accepted.

Original supports require the existing clamped, polynomial Bezier, constant positive weight class. Transition patches use exact native coefficients, never preview poles. Budget exhaustion is an error, and a chart for which this certificate cannot be established stays unresolved. Complete-original-support injectivity also covers every retained trimmed subset. `ClosedShell.inspect_face_charts` freshly checks saved geometry before collecting these proofs. This is per-face injectivity, not a proof that different faces do not intersect.

## Terminal evidence

23 focused native NURBS tests and 13 B-rep integration tests pass. New negative native cases alter an interior X coefficient by 2^-100, perturb a whole-boundary interior Z coefficient and construct matching but folded transverse boundaries; none is admitted. Whole-chart tests cover a nonlinear height, interchanged physical axes, reversed monotonicity, a genuinely folded surface, rational-support refusal and incomplete budgets. The B-rep test checks the three-piece root seam, full accounting of every pair, insufficient budgets, whole-face injectivity and the earlier independent volume formula.

The expanded shell remains 27 faces, 82 edges, 57 vertices, zero open edges and no published bodies. Of 351 face pairs, 277 have strict hull separation and 19 now have proved contact limited to the actual complete shared transverse boundary. The remaining 55 comprise 40 transition-to-retained-source contacts and 15 contacts between original-source faces. Zero pairs are omitted. Their exact face indices are in unresolved-pairs.json.

All 27 complete support charts are injective; zero face charts remain unresolved. Nineteen internal G2 joins and maximum whole-support curvature bound 9.867331764661289/mm remain under the unchanged 10/mm cap. Shared charged coefficient work is 1910236 after transverse contacts and 2021929 after face charts.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib`. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and run `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining admission and full goal

Prove the 40 transition/retained-domain contact pairs using their actual original-chart notch partitions and source-bound contact owners; then prove the 15 remaining original-face contacts are limited to admitted shared boundaries. Original-chart/source ownership and full edge coverage must survive every trim/split. Radius, thickness, material/volume admission, general rational/multi-span trimming, unequal-radius complex nodes and 5+ corners still require work. WASM/worker/UI, STEP/independent measurements/history, three-part acceptance, P0 and P2-P3 remain open. Listed hashes describe selected files in the shared dirty checkout, not an isolated delivery revision.

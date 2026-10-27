# Compact original SS curves with continuous error bounds

`construct_graph_curves` builds saved world NURBS curves and two independent original-native-UV pcurves from a private `QualifiedQuadraticGraphSS` owner. The admitted original surfaces remain the existing complete single-span polynomial graph family with an exactly classified quadratic difference. Rational output curves do not imply support for general rational original surfaces, multiple spans, clipped conics or general Boolean operations.

Each complete original edge gets one clamped rational Bézier curve per representation. Native pcurves have degree one for line factors or two for conics. The world degree follows the complete nonzero original height polynomial: a constant-height support gives a quadratic conic with three poles; a quadratic height can give degree four; a full supported biquadratic height can require degree eight and nine poles. Straight UV factors can have curved quartic world lifts. No nonzero height term is discarded. When either original support has constant height, that exact height is retained in every saved world pole and vertex.

The constructor proposes actual binary64 poles and positive weights, then proves their error against the original branch using outward-rounded Bernstein coefficients over the whole clock. Cross-multiplied homogeneous residual bounds and positive denominator lower bounds replace sample-based acceptance. Both actual saved pcurves are separately composed with their own original support and checked against the saved world curve. This includes isolated point vertices: tiny UV error alone cannot admit an incorrect world lift through a very narrow original knot domain.

Shared original vertex identities supply bit-identical end poles in adjacent curves. Proven original rectangle endpoint incidences retain exact original native endpoints and world coordinate rails; there is no distance-based snapping. Native-domain membership is established from positive-weight convex hulls or bounded homogeneous subdivision, so an exterior control pole does not by itself reject a curve whose complete image lies inside. Domain membership concerns the original rectangular support, not a retained face's trim holes.

One explicit operation allowance covers interval arithmetic, coefficient construction, proposed coordinates, vertex stations, actual curve checks and domain subdivision. Insufficient work, unresolved positivity or unmet tolerance returns an error without a partial qualified owner or an increased tolerance. Original source classification and original-owner replay retain their separate bounded work contracts. `reverify()` replays the original owner, constructs and proves fresh actual curves, then compares all saved poles, weights, clocks, endpoints, bounds and operation counts. A report or mutated curve cannot authorize reuse.

## Qualification

The final wider native run passed **3040 tests, zero failed, one ignored**, across 168 result summaries including doctests. The B-rep SS/Boolean compatibility run passed **53/53** tests (1836 unrelated tests filtered out). The preceding focused integration run passed **39/39** tests; the final wider run additionally includes the private replay regression and final clock guard. Rustfmt, diff whitespace and API mapping checks passed. The unchanged architecture gate still fails with 83 diagnostics; none names a new curve module.

Measured results and exact source/log hashes are recorded in [matrix.json](matrix.json). Focused regressions include independent analytic original-branch oracles, both actual support lifts, shared endpoints, preserved quartic height, minimum conic degree, support swapping, nonrepresentable native widths, outside control poles with inside images, exact boundary incidence, coincident/point/empty/line strata and exact shared work exhaustion. A private replay regression rejects changed world/pcurve poles, weights, clocks, forged error reports and rebound original supports.

Reproduce from the worktree root with the pinned Rust toolchain and lockfile:

```sh
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml --no-fail-fast --offline --locked -p nurbs-core -p cad-predicates -p nurbs-intersect
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml --no-fail-fast --offline --locked -p brep-core --lib nurbs_ss
node scripts/map-nurbs-api.mjs --check
```

## Remaining obligations

This is native curve approximation qualification. It does not prove immersion, global embedding, trim-hole membership, orientation/material roles, original-face subdivision, sewing or a final Solid. No new WASM/editor/STEP qualification is claimed. Existing distribution size and architecture gates remain open with their limits unchanged. All five directions of the original plan remain in scope.

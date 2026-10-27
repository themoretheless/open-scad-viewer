# Continuous original SS branch ranges

This extends the qualified closed/degenerate graph owner from the mixed quadratic milestone. It is a step toward original-face trim construction, not a completed Boolean or Solid operation.

`QualifiedQuadraticGraphSS::edge_enclosure(edge, [start,end], max_operations)` encloses every point on one owned edge over the closed clock interval. It returns normalized UV, both independent original native UV domains and the original world-space curve enclosure. Rational quarter-conic clocks, the exact source-coordinate basis and the complete original height polynomial participate in outward-rounded interval evaluation. It does not construct a box from sampled stations or only the two endpoints.

The complete owner already proves ellipse containment or exact line clipping against the original rectangle. The interval overestimate is intersected with that proven domain before evaluating world coordinates. This does not admit a clipped ellipse that the constructor refused. A componentwise box includes off-branch points: its midpoint is not an asserted intersection, and a public mutable range report cannot authorize trim topology. Consumers must retain the private original-source owner and the edge/clock identity.

Clock ordering, finite parameters, edge existence and explicit operation bounds are validated. An exhausted allowance returns an error without a partial range. Degenerate point intersections and empty sets do not acquire a fictitious edge. The point-station API remains available and uses the same quarter-conic evaluation. Native UV spans are outward-rounded intervals rather than a required exact binary64 subtraction. This admits ordinary domains such as `[0.000001,1]` without replacing the true span by its rounded value. Both point and range mappings use the original endpoints.

## Verification

The final focused run passed **30/30** tests (the original 13 plus 17 mixed/range regressions). Continuous-range regressions independently check analytic conic points throughout full and partial clocks, both native domains, original-coordinate clamping, support swapping, subdivision, nonrepresentable native span widths and exact work exhaustion. They include a tilted arc whose interior exceeds its endpoint hull, and a nonmonotone original spatial height whose maximum is missed by both endpoints.

The wider native regression run passed: **3030 passed, zero failed, one ignored**, across 167 result summaries including doctests (`nurbs-core`, `cad-predicates`, `nurbs-intersect`). SS/Boolean compatibility checks passed **53/53** B-rep tests (1836 unrelated tests filtered out). Rustfmt and diff whitespace checks passed. No fresh WASM, editor or Boolean construction qualification is claimed for this extension. The existing distribution-size and architecture gates remain open with unchanged limits.

## Next construction obligation

Use these continuous bounds, together with original derivative or coefficient bounds, to qualify a compact NURBS proxy against the original branch with an explicit tolerance. Complete branch coverage, world/pcurve correspondence, original face imprinting, winding/material roles, sewing and final Solid verification remain separate requirements. Dense samples or a proxy's own identity must not replace the original-branch error proof.

## API ownership repair

The API-map check revealed 12 previously public, unassigned modules. The existing flat paths remain intact. The task-oriented facade now assigns interval Bernstein interpolation to geometry, the section bridge to joining, differential/secant queries to queries and original-profile/normal certificates to validation. This changes neither numerical algorithms nor catalog capability counts. `map-nurbs-api --check` passes with 239 public modules and 200 catalog entries; `cargo check -p nurbs-core --offline --locked` separately validates the added aliases. Numerical regressions above ran before this alias-only update; the SS implementation hashes are unchanged.

Reproduce the runtime checks with the repository toolchain, `CARGO_BUILD_JOBS=3`, an isolated `CARGO_TARGET_DIR`, and `CARGO_PROFILE_TEST_OPT_LEVEL=1`:

- `cargo test --manifest-path crates/Cargo.toml -p nurbs-core --test ss_closed_degenerate --test ss_mixed_quadratic_graph --offline --locked`
- `cargo test --manifest-path crates/Cargo.toml -p nurbs-core -p cad-predicates -p nurbs-intersect --no-fail-fast --offline --locked`
- `cargo test --manifest-path crates/Cargo.toml -p brep-core --lib nurbs_ss --no-fail-fast --offline --locked`

Checks apply to the ongoing plan worktree, including the separate offset/application work. They do not claim clean-branch release qualification.

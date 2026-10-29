# Original-source mixed quadratic surface intersections

This native milestone advances priority 1 of the five-direction NURBS plan. It expands the existing qualified intersection contract; coefficient choices are not counted as new catalog features. It does not complete general Boolean support.

## Geometry and ownership

The original degree-one/two polynomial graph coefficient proof now accepts a quadratic mixed `u*v` difference term. Exact completion of squares constructs an invertible shear, with a coordinate swap or a sum/difference basis when the first quadratic pivot vanishes. Every sum, product and required quotient is checked as a mathematical binary64 identity. No sampled primitive fit, approximate eigenvector or enlarged tolerance authorizes the result.

The existing private qualified owner retains both complete original supports, their independent native UV domains, polynomial heights, diagonal form and the original-coordinate basis. `reverify()` rebuilds all of that data. The basis participates in replay; mutating a private test copy invalidates verification.

Supported mixed strata now include a fully contained closed ellipse, an isolated tangent point, a tangent double line (multiplicity two), two exact linear factors with their common singular node, and simple parallel factors. Empty original intersections retain no fake curve. A tangent double line touching the rectangle only at a corner becomes an isolated tangent point instead of a zero-length edge.

## Original coordinates and complete coverage

Ellipse extents are checked AFTER mapping the diagonal coordinates into the original source rectangle. Each original coordinate has amplitude `sqrt(A*A+B*B)` for its cosine/sine coefficients. Independent diagonal-radius checks would falsely admit clipped tilted ellipses; using an absolute sum would unnecessarily reject valid ones. Outward-rounded interval operations bound both these amplitudes and each algebraic station. The four conic vertices are canonical clock anchors, not necessarily extrema in original UV.

All linear factors are clipped against the original rectangle. Active parameter constraints are selected by exact cross-products of fractions before division. Only actual endpoints are constructed, preserving the existing case where an unused boundary quotient is non-dyadic. Crossings retain one true singular node, four halfedges and two branches. Trims keep both original support identities; the original height polynomial is retained when lifting UV points into world coordinates. A separate regression checks an actual quartic world curve against the shared `u²*v²` source contribution and rejects replacement by its endpoint chord.

Only a complete derivation returns a qualified owner. The single exact-attempt work limit includes basis construction, radius checks and original clipping. Exhaustion remains a resource error and does not buy a new, larger fallback attempt.

## Verification

Focused native checks: the existing 13 closed/degenerate SS tests and 12 new mixed-form tests pass. The new tests independently evaluate the original surfaces and analytic equations, cover both UV domains, swapped supports, unequal and irrational radii, a thin ellipse, zero pivots, oblique factors, small factor separation, multiplicity, actual clipped dimension and exact work exhaustion.

The wider native regression run passed: **3024 passed, 0 failed, 1 ignored** across 167 test-result summaries, including doctests, for `nurbs-core`, `cad-predicates` and `nurbs-intersect`. One final regression for the shared biquadratic world height was then added; the final focused run passed **25/25**. The production Rust implementation is unchanged between those runs. B-rep compatibility checks passed **53/53** SS/Boolean tests (1836 unrelated unit tests filtered out). Rustfmt and diff whitespace checks passed. These tests qualify native source behavior. The previously built WASM/browser artifact has not been rebuilt for this native extension.

## Remaining scope

Varying rational weights, multiple original knot spans, clipped ellipse arcs, nonzero-level hyperbolas, parabolic strata and certain nonrepresentable exact constructions still require further classifiers. A simple parallel factor meeting only a rectangle corner is explicitly refused. Complete graph/stratum ownership does not implement original-face imprinting, material/winding classification, sewing or final Solid verification; those Boolean stages remain required. Central G2 filling, global offset cleanup, simplification and edited-body STEP export also remain open.

## Reproduction

Use the repository toolchain and an isolated target directory. The tests ran offline with the locked dependency graph, `CARGO_BUILD_JOBS=3` and `CARGO_PROFILE_TEST_OPT_LEVEL=1`:

- `cargo test --manifest-path crates/Cargo.toml -p nurbs-core --test ss_closed_degenerate --test ss_mixed_quadratic_graph --offline --locked`
- `cargo test --manifest-path crates/Cargo.toml -p nurbs-core -p cad-predicates -p nurbs-intersect --no-fail-fast --offline --locked`
- `cargo test --manifest-path crates/Cargo.toml -p brep-core --lib nurbs_ss --no-fail-fast --offline --locked`

Validation applies to the ongoing plan worktree, which also contains the separate offset/application milestone. Other uncommitted work is retained; this result is not a clean-branch full-release qualification. Existing architecture and distribution-size failures remain open and their limits are unchanged.

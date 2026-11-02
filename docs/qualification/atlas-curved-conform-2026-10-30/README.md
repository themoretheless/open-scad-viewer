# Curved knot pullbacks and conforming source charts — 2026-10-30

Bounded Rust extension to the exact source atlas. This does not claim arbitrary
algebraic pullbacks, unbounded computation or complete Minkowski-sum topology.

## Curved trims through surface knots

The existing single-cell and affine-triangle paths are preserved. A nonlinear
UV disk can cross knot cells when each crossed coordinate is exactly affine in
its parameter triangle. The remaining UV coordinate and the actual boundary can
be nonlinear rational curves. Affineness is proved by a complete triangular
Bernstein identity H_axis = W*(a+b*u+c*v), not by fitting samples. A non-affine
coordinate must have its full control hull in one axis interval.

Knot halfplanes are pulled back to the original parameter triangle and clipped
using exact rational intersections. Oriented areas sum exactly to the standard
triangle area. Every child is obtained by exact Bernstein restriction of the
original rational UV map, then composed with its original tensor surface cell.
No curved boundary is replaced by a chord. The original UV disk proof is still
mandatory. A genuinely nonlinear algebraic knot pullback remains unproved.

## Neighbor conformity

Before clock synchronization, an atlas with unmatched edge pieces attempts to
propagate known spatial vertices across the neighboring charts. It never
invents a nearest point. The three homogeneous equations H_k-P_k*W are converted
to exact rational polynomials; their common factor and its square-free part
must identify one rational interior parameter. All three homogeneous coordinates
are re-evaluated exactly at that parameter. A nonrational or ambiguous preimage
refuses. Children are exact restrictions of the parent source chart. No new
unbound spatial vertex is permitted. A chart count cap and predicate work/bit
limits remain in force.

The final atlas must still pass edge incidence/orientation, complete seam
identities, source-star regularity, global separation and source binding.
Conformity alone is not an embedding or a complete closed-body certificate.
Legacy v4 files are unchanged. WASM/dist and browser checks are outside this run.

New tests cover a bowed rational UV boundary crossing a surface knot, retention
of its exact knot-intersection point, refusal of an unsupported nonlinear
pullback, propagation across an open curved seam while retaining open-boundary
refusal, and a closed curved tetrahedron with matching paired edges and Euler
characteristic after propagation.

Validation: **88 native Minkowski tests passed, 0 failed**. The closed curved
tetrahedron, curved knot crossing and open-boundary refusal regressions pass.
Observed test wall time 67.64 s is not a performance benchmark. Scoped rustfmt
and git diff whitespace checks pass. The initial diagnostic log retains the
superseded overly broad open-boundary test assertion; it is not final evidence.

Reproduction:

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

`source-files.json` binds nine atlas files including previous-turn extensions;
it is not an isolated full workspace snapshot. `native-tests.log` and
`result.json` record this run. No standalone external checker replay or runtime
artifact rebuild was performed.

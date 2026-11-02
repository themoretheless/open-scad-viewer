# Nonplanar source graphs and implicit algebraic pullbacks — 2026-10-30

This is an exact bounded Rust extension. It is not a completed universal NURBS
solid modeller and does not assert complete Minkowski-sum topology.

## Nonplanar charts in the actual conformity path

A missing rational edge parameter can trigger an exact source-point membership
proof: the common factor of all three homogeneous coordinate equations must
have an actual root in the open active interval, with a positive source weight.
No root midpoint is used. The source chart can then be reconstructed as a
nonplanar graph over a proved triangular projection. The projection is proved
homeomorphic to its corner triangle; source height is fitted only as a proposal.
Every coefficient of Z*D(X,Y,W)-W*N(X,Y,W) is independently checked. D must have
strictly positive Bernstein controls over the whole target triangle. Rebuilding
the exact height and both projected coordinates preserves the entire original
source image and all three corner positions. The normal atlas incidence, seam,
vertex-star and separation proofs remain mandatory.

Polynomial and rational graph degree 1..3 is searched, subject to the existing
triangular arithmetic cap. A graph is never replaced by its corner plane.
The rational seam-parameter finder now removes common homogeneous base factors
before isolating its actual spatial preimage; unrelated nonreal base roots no
longer hide a unique rational parameter.

Tests cover polynomial and rational nonplanar graphs, actual curved-neighbor
subdivision at an originally irrational seam parameter, perturbation refusal,
nonreal homogeneous base factors and a complete curved tetrahedral atlas.

## Implicit algebraic pullbacks

Public Rust APIs under `minkowski3d::curved`:

* `certify_algebraic_surface_pullback` / `verify_algebraic_surface_pullback`
* `certify_algebraic_triangle_pullback` / `verify_algebraic_triangle_pullback`
* `AtlasAlgebraicPullback`, `AtlasExactTriangle`

The first input is one original tensor NURBS knot cell; the second retains exact
rational homogeneous triangular controls without F64 conversion. The equation
H_axis-level*W is converted to a full bivariate power polynomial exactly. Degree
is bounded by eight in each tensor direction or total triangular degree eight.
The existing exact cylindrical branch engine reconstructs the critical projection,
algebraic fibers, ordered open-slab roots, vertical components and one-sided
incidences. A verifier reruns these proofs from the ORIGINAL source equation,
then checks every supplied branch and every source-side sign. No sampled
continuation establishes connectivity.

All square-domain branch sections are retained. The diagonal is the triangle
boundary t=1-s: `diagonal_side=1` marks open sections below it, -1 above, and 0 on
it. In this API `excluded_nodes` marks nodes on that boundary; it must NOT be
interpreted as deletion of source events. Critical-fiber and boundary nodes are
retained in the original branch decomposition. Homogeneous source-equation signs
and multiplicity are checked in each open slab. Inside the source triangle,
positive W makes them the actual coordinate-cut signs. A relation vanishing
identically has an explicit two-dimensional coincidence certificate.

Tests cover closed curved components, irrational critical/boundary parameters,
an even-multiplicity parabolic level set, intersecting vertical/horizontal
components, two-dimensional coincidence, exact triangular input and forgeries
of source geometry, roots, incidences and side labels. Source bindings include
the query and every authored surface bit, including periodic flags, or every
canonical exact triangular coefficient.

## Open scope

This adds a verified branch representation; arbitrary algebraic regions are NOT
yet assembled into the existing rational-control-net 3D atlas. General non-graph
nonplanar charts, graph fits outside the supported degree/budget, algebraic
restriction control fields and the assembled topology of arbitrary clipped
regions remain unproved/refused. Multi-cell source surfaces must be partitioned
before the single-cell pullback API. Legacy v4 and shared WASM/dist are unchanged.
No standalone checker process or browser qualification has been performed here.

## Validation

**103 native Minkowski tests passed, 0 failed**, including the closed nonplanar
atlas, all new graph/implicit-branch cases, the earlier source-certificate
regressions, cone, sphere and torus. The closed nonplanar regression checks all
source vertex stars, paired seams and Euler characteristic after conformity;
it is a synthetic source-chart test, not an external B-rep fixture replay.
Observed test wall time 87.84 s is not a performance benchmark. Formatting and
scoped `git diff --check` pass.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

`native-tests.log`, `closed-nonplanar-tests.log` and `result.json` retain results.
`source-files.json` binds twelve scoped files; it is not a frozen full workspace
snapshot. Concurrent unrelated edits were preserved. Nothing was committed,
pushed or deployed by this geometry milestone.

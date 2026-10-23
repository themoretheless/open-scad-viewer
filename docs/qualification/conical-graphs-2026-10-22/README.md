# Conical poles, rational graph lifts and spatial seam inverses

Native qualification complete: **78 Minkowski/atlas tests passed**, including
**7 new regressions**. The final separate Rust checker verified **18 certificates**
(4 new sources and 14 archived proofs), rejected **44 forged certificates**, and
refused **4 B-reps with a broken declared pole edge**. All recorded refusals are
structured and panic-free.

New public fixtures: ordinary, inverted and oblique rational cones, plus a
non-polynomial rational graph. All new certificates use depth 0. Cones produce
32 exact source-chart triangles; the graph fixture produces four.

No WASM/runtime/dist promotion is implied.
The legacy v4 enclosure recipe remains frozen. These additions concern the
standalone source-atlas certificate, not topology of a full Minkowski sum.

New sufficient predicates:

- A complete positive-weight tensor side collapsed to one point is transformed
  by u=y/(x+y), v=x+y, multiplying homogeneous coordinates by (x+y)^n.
  The exact Bernstein basis identity covers the entire punctured source cell.
  The resulting triangular weights are nonnegative. Every zero-weight control
  must have zero numerator. A complete strictly positive first weight layer with
  one common Euclidean point proves a uniform finite limit at each singular
  vertex. Distinct directional limits, missing leading-layer evidence, negative
  weights and infinite limits refuse. The total conical chart degree is bounded
  by eight to retain existing derivative arithmetic budgets.
- Exact common endpoint factors are removed from all four homogeneous edge
  coordinates. No approximate cancellation or endpoint deletion is used.
  Hull bounds retain positive-weight controls and proved corner limits. The
  singular chart is retained in every homotopy/separation check.
- For singular charts, nonnegative determinant Bernstein coefficients with at
  least one positive coefficient prove a strictly positive interior Jacobian.
  A nonnegative trace proves the complete source-to-chord homotopy. Each chart
  must separately prove its projected boundary Jordan and monotone throughout
  that homotopy, including internal star edges. All global star/link/separation
  checks still apply. This is a topological disk proof, not a smooth-apex claim.
- Rational graph lifts reconstruct N/D with homogeneous numerator and denominator
  of degree at most three. The full tensor identity Z D(X,Y,W)=W N(X,Y,W) is
  checked coefficient by coefficient. Strictly positive triangular denominator
  coefficients prove it has no zeros on the complete target disk. Sampling and
  elimination proposals cannot authorize a geometry result.
- Spatial seam inverse candidates now include homogeneous monomials through
  degree four and bounded pairwise nullspace combinations. The source denominator
  sign, recovered clock range/endpoints/monotonicity, full three-coordinate
  composition identity and embedded parameter extension remain mandatory.

These are bounded sufficient classes. General algebraic inverse branches,
denominator-zero inverses requiring other charts, missing sign certificates,
higher-degree pole charts and unpartitioned knot-cell trims can still return
unproved. No universal completeness or proof-assistant theorem is claimed.

Production computation and verification remain Rust. The Python fixture builder
constructs exact regression data only and checks every binary64 conversion.

Round-cap fallback: when the boundary-vertex triangulation cannot prove its UV
maps, a hole-free face may propose a fan around the exact rational mean of its
boundary vertices. Winding must put that point inside. The original complete
boundary-cancellation, positive-disk and winding checks certify the proposal;
an interior point is never trusted merely because it is a centroid. Hole-bearing
faces retain their existing checked construction.

Projection fallback: the original area-sum direction is always attempted first.
On failure, deduplicated L1-normalized corner-face normals propose another
projection. A bounded exact three-variable halfspace feasibility search can
propose up to 12 more directions. It checks positive dot products with all
corner-face normals; this condition alone never authorizes a result. Every
candidate must reprove the full star, monotone/Jordan boundary and separation
conditions. Feasibility search is limited to 32 distinct normals and boxes of
radius 2, 8 and 32.

The spatial inverse search can recover quartic observable identities, but the
existing full triangular parameter composition limit remains degree 16. An
isolated seam identity is not proof that a high-degree full surface patch or a
complete solid will pass its subsequent embedding checks.

Before proving singular vertex stars, every source triangle is subdivided
exactly into four triangles. This conforming split is admitted only after every
paired seam proves pointwise parameter agreement, including straight seams.
Rational midpoint coordinates are retained exactly. The refined charts are
rechecked through the complete atlas proof; subdivision itself authorizes no
embedding claim. Old nonsingular successful recipes retain their old endpoints.

## Reproduction

Use the frozen source ledger in source-files.json and owned-files.json. Build
`brep-core` library tests with filter `minkowski3d::` and its
`verify_profile_certificate` example from `crates/Cargo.toml`, opt-level 1,
debug 0. Production code is entirely Rust.

Run `scripts/check-conical-graphs-native.py CHECKER FIXTURES OLD_SOURCE OLD_ENCLOSURE OUTPUT`.
OLD_SOURCE is `../spatial-poles-2026-10-21/native-replay.json`; OLD_ENCLOSURE is
`../rational-atlas-rounding-2026-10-19/wasm-results.json`. Archived requests are
replayed unchanged. New certificates use depth 0, including all three cones.
The separate Rust processes recompute the same exact checker; this is not a
second geometry implementation or a proof-assistant kernel.

`make-cone-fixtures.rs` constructs up/down cones and an exact affine shear.
`make-rational-graph-fixture.py` constructs an exact projective image of a
qualified curved tetrahedron and checks every binary64 conversion. These are
fixture generators; neither replaces production Rust geometry.

Receipts: [manifest](manifest.json), [native tests](native-tests.log),
[replay](native-replay.json), [source ledger](source-files.json).

Checker SHA256: `f1e40d7e569b912f4826481583809f6c3cec255b98c29bdaff7b7fb088584db2`.

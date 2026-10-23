# Spatial rational seam clocks and non-monomial poles

Native qualification complete: **71 Minkowski/atlas tests passed**, including
**7 new regressions**. The focused run passed **28 tests**. A separate native
Rust checker verified **14 new and archived certificates**, rejected **60 forged
certificates**, and refused **2 invalid source interiors** through certification.
The final rebuild produced exactly the same checker bytes.

This milestone expands sufficient exact predicates; it does not establish
support for every rational surface singularity or seam. WASM, browser and dist
were not built or promoted.

## New checks

- Spatial seam inverse candidates use homogeneous linear and quadratic observables
  in X,Y,Z,W. Exact rational elimination proves N(A(t))=t D(A(t)). Denominators
  must have a strict sign on the complete carrier. The recovered clock must
  preserve endpoints and orientation and stay inside [0,1]. Its complete spatial
  composition A(clock(s))=B(s) is checked in all three coordinates. The existing
  embedded parameter extension and full atlas checks remain mandatory.
- Rational planar poles may have non-monomial angular and radial clocks,
  u-dependent homogeneous weights and stationary boundary derivatives. A positive
  interior rational Jacobian, a collapsed side, three monotone segment boundaries
  and planar degree one prove exactly one disk image. A linear triangular chart
  represents this same image. Folded interiors and nonplanar perturbations refuse.
- Nonplanar pole charts can be reconstructed as polynomial graphs of degree up
  to three over a proved triangular projection. Each coordinate-axis projection
  is tried. Exact homogeneous tensor identities check the complete graph, not
  just samples. The projected pole disk is proved first; lifting a continuous
  graph preserves its embedding. The reconstructed chart still passes all atlas
  stars, links, seam, derivative and separation checks.

## Proof details and boundaries

For planar poles, all rational weights are positive. The determinant numerator
has nonnegative tensor Bernstein coefficients and at least one positive one.
Every Bernstein basis function is positive in the open square, so the interior
Jacobian is strictly positive even if boundary derivatives vanish. Three
noncollapsed sides map monotonically onto a triangle, while the fourth side is
one point. Properness and oriented planar degree one then prove uniqueness of
every interior preimage and absence of exterior preimages. This is a chart
change of the complete source image, not omission of singular cells.

For nonplanar graph poles, write S=A+e*x+f*y+d*z, using two barycentric projected
coordinates and one coordinate-axis lift direction. A source cell is accepted
only if Z W^(q-1)=sum(c_ij X^i Y^j W^(q-i-j)) holds coefficient by coefficient.
The source weights are positive and the projected source is the proved disk;
therefore the image is exactly the reconstructed graph on the whole triangle.
The final global atlas proof is retained.

Limits: inverse observables above degree two, inverse branches with denominator
zeros, candidates needing combinations absent from the generated nullspace basis,
non-polynomial graph lifts, non-triangular projected pole boundaries and failed
sufficient sign predicates may still refuse. Genuine nonsmooth conical poles
need a separate topological chart proof. Degree, coefficient and work budgets
remain bounded. No arbitrary-case completeness claim is made.

Legacy v4 enclosure admission remains frozen. This work expands the standalone
source-atlas checker. Native tests and checker replay do not qualify WASM,
browser transport or dist.

## Reproduction

The source snapshot and complete SHA256 ledger are pinned in manifest.json and
source-files.json. The five changed Rust files were compared with that snapshot.
Build the native `brep-core` library tests and `verify_profile_certificate`
example from its `crates/Cargo.toml`, with opt-level=1 and debug=0. The test filter
is `minkowski3d::`; the focused filter is `minkowski3d::curved::atlas::`, excluding
only the long-running sphere and torus tests from that focused run.

Run `scripts/check-spatial-poles-native.py CHECKER FIXTURES OLD_SOURCE OLD_ENCLOSURE OUTPUT`.
OLD_SOURCE is `../atlas-charts-2026-10-20/native-replay.json`; OLD_ENCLOSURE is
`../rational-atlas-rounding-2026-10-19/wasm-results.json`. Requests in those archived
receipts are replayed unchanged. The checker runs in separate native processes;
it recomputes the same exact Rust predicates, rather than using a second geometry
algorithm. New requests are normalized for signed JSON zero before certification.

`make-graph-fixture.py` constructs only exact rational regression data. Every
conversion to binary64 is checked for exact equality. Production construction and
verification remain in Rust.

Receipts: [manifest](manifest.json), [native tests](native-tests.log),
[focused tests](focused-tests.log), [replay](native-replay.json).

Checker SHA256: `358fe3015972db2316166553549f7b4672600574343f1d03fecc2b9226d0c429`.

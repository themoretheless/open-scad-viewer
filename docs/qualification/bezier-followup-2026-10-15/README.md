# Bézier follow-up qualification — 2026-10-15

This milestone expands four bounded domains. Geometry construction and certificate
checking are owned by Rust; TypeScript only transports the ABI and types.

## Implemented and qualified domains

- A normal-offset **curve** approximation API for regular retained positive-weight
  Bézier/NURBS of degrees 1–8. The independent checker reconstructs exact rational
  Bernstein bounds and dyadic integer-square-root enclosures, proving pointwise
  Euclidean deviation over the complete active domain, including rounded output
  endpoints. Output is explicitly `bounded_approximation`, not an exact offset.
  Corners and singular normals refuse for nonzero distance. The release-WASM
  rational-quartic fixture used 28 segments and a 0.05 mm bound.
- Exact coincidence under a supplied nonlinear Möbius correspondence
  `phi(u)=scale*u/(1+(scale-1)*u)`, including common knot partitions. Both
  homogeneous coordinate identities and the source/map binding are rechecked.
- Complete endpoint-intersection certificates for two clamped spans. A genuine
  two-arc lens with two shared endpoints now has exact Green orientation and is
  admitted to production extrusion, with a closed nondegenerate cap mesh.
- Degree 4–8 admission in production profile normalization, Booleans and
  extrusion. Scalar line/common-coordinate and stationary-root exact reductions
  extend to degree eight. Transverse Boolean fixtures retain the high-degree
  edges; a rational quartic retains its nonuniform weights.

## Validation

- 87 selected native Rust tests passed: 23 profile-certificate tests, 40 existing
  Bézier profile regressions, 3 new modeling fixtures, 13 scalar predicate tests,
  7 bridge tests and 1 degree-eight stationary/contact test. The latter checks a
  sixfold stationary root and a seventh-order line contact. Filtered-out tests
  are not counted as executed tests.
- 107 host tests passed across 10 files, including five new ABI/modeling tests.
  Vue and MCP type checks passed.
- Direct release-WASM checks covered the three new certificate scopes and actual
  quartic/eighth-degree transverse Boolean extrusion plus the two-arc lens.
  All three resulting meshes are closed, with zero degenerate triangles,
  nonmanifold edges and orientation conflicts; volume discrepancy is below 0.3%.
- Existing release-WASM measure/affine-agreement checks (three profiles and one
  agreement) and cusp/rational topology checks (four profiles) passed again.
- The separately compiled Rust checker verified the three WASM-produced new
  certificates and rejected a moved offset vertex, changed map and false contact
  census. See `standalone-checker.json`.
- A fresh in-app browser tab imported the actual degree-eight solid, displayed
  its sketch/body with smooth B-rep preview disabled, and reported no console
  warnings/errors. This is a browser integration check of that fixture, not
  interactive UI coverage of every new certificate operation or latency proof.

## Delivered package

`public`, generated module and `dist` contain identical 11,207,482-byte geometry
WASM, SHA-256 `d14c1518798c6f2f614e0e554eabfd51747cc6f22f15ef3ca4dc64c7a18e55a4`.
The production build and packed/raw identity verification passed: 173 artifacts,
8,094,808 asset bytes + 14,235,763 raw WASM bytes = 22,330,571 total bytes.
The aggregate budget was explicitly increased from 8,080,000 to 8,100,000 bytes,
leaving 5,192 bytes. The geometry chunk budget remains unchanged. Relative to
prior qualified delivery, the actual asset increase is 22,526 bytes.

Compression block/mode/window and whole-module trials did not recover enough
space. Repeated optimization saved only about 5 kB of the packed package; it and
the similar-function merge trial were not adopted. The optimizer and compression
recipes remain unchanged; source identity and runtime evidence refer to the
qualified module above, not any of those temporary experiments.

## Explicit remaining limits

The normal-offset certificate does not prove a filled-region morphological
operation, corner joins, global offset self-intersection cleanup or solid validity.
The projective certificate covers this Möbius family, not arbitrary nonlinear
correspondences. Endpoint proofs do not turn a touching compound or touching
hole into a manifold solid. The general paired implicit reducer remains cubic;
higher-degree pairs/self-intersections can still exhaust the conservative
intersector and return unproved. No refusal is treated as a proof of invalidity.

These are independently checkable exact-arithmetic Rust certificates under the
trusted compiler/runtime/arithmetic libraries, not proof-assistant verification
or third-party formal certification of the entire CAD kernel.

No commit or push was performed. Parallel work was preserved. The parallel thread
module needed one compile fix (`FaceUse` is Clone, not Copy). `source-sha256.json`
records the inspected task-related source snapshot; the shared checkout contains
other ongoing work. See `docs/design/bezier-profile-certificates.md` for contracts.

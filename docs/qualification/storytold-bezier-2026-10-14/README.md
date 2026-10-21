# Planar Bézier singularities, curved coincidence and cusp meshes

Qualification completed on 2026-10-15. Predicate/profile source fingerprints are
in `source.sha256`; the mesher fingerprints are in `cusp-mesh-source.sha256`.
Authored inputs are preserved. Profiles retain the existing
`numerical_uncertified` status and explicit resource/conditioning refusals.

## Predicate and profile coverage

`cargo test --offline --locked --manifest-path crates/Cargo.toml -p cad-predicates --lib --test planar_singularities`

- 56 unit tests and both independent singularity integration tests passed under the unchanged default predicate work limit.
- The degree-nine eliminant `27v^6(4-(1+v)^3)` retains its sixth-order cusp and separate simple contact. Both original parameter boxes are checked against independent closed-form roots, before and after an affine change of coordinates.
- The degree-nine eliminant `-27s^7(3+3s+s²)` retains only the seventh-order endpoint contact in the finite domain.
- The exceptional fiber distinguishes a node's two preimages from a cusp's single preimage. Other checks cover nonlinear parameter maps, finite-domain filtering, quartic double contact, rational weights and explicit exhausted-resource outcomes.

The final source diagnostic passed 54 cases. It links current profile/planar
modules and a separately compiled current predicate library to cached native
kernels. One extrusion fixture was excluded because its cached external prism
predates the unordered-control admission. This diagnostic is distinct from the
final packaged application checks below.

The standard BRep profile checkpoint passed all 37 tests (903 unrelated tests
filtered), including retained extrusion and mixed cubic/rational Boolean
operations. It predates the final parameter-transport bounds; those guards pass
the current 54-case diagnostic and final packaged WASM tests. The optional full
NURBS lib-test build was cancelled while waiting for its target lock and is not
counted as a pass. The redundant native mesher Cargo rebuild was also cancelled;
its superseded failures are not counted as results for the final source.

Root transport compares outward images of the original unit-domain carrier and
the represented parameter observation. Meaningful interior events cannot silently
become endpoints on coarse knot domains. Endpoint coalescence requires the
explicit roundoff geometry bound; ordinary transverse Boolean joins remain
admitted.

## Curved cusp display mesh

Uniform parameter grids can give distinct cubic carriers coincident first
chords. The shared-edge schedule now refines the appropriate carrier before any
positions are allocated, preserving every source control point. Interval bounds
prove departure order, including anisotropic cap UV coordinates: a rounded
positive determinant alone is insufficient. Opposite ruled-face edges receive
the same refined schedule. Boundary and triangle budgets remain unchanged.

The current mesher source diagnostic passed all 15 registry tests. It covers
both cusp coordinate frames at subdivisions 1, 2, 3, 4, 8, 16 and 32; closed
incidence; zero degenerate/non-manifold/orientation defects; unchanged source
models; and volume error below 0.3% at detail 32. It also covers dense ruled
hollow bodies, periodic poles/seams, curved holes, separate contacting bodies,
and exact collinear-triangle repair. It links current mesher source to cached
native kernel libraries; final WASM coverage follows below.

## Final application artifact

The standard WASM compilation succeeded after correcting a shared-checkout
import to the public `nurbs_core::interval_eval::Interval` re-export (the identical
interval type). The checked-in build recipe was used, including the shared
checkout's per-package size optimization settings.

- Raw WASM passed the profile/root smoke cases, cusp extrusion and all seven mesh subdivision levels, original nodal carrier normalization and coarse-knot-domain refusal.
- Optimized geometry WASM: 10,874,400 bytes; SHA-256 `6ce47a0cf18b241ba3a39d09f9fa0cabf8371a0228aca89e90534f0bdb347f0f`.
- All 95 tests in seven profile/drawing/specification/worker suites passed on the final package. These include both cusp coordinate frames through the application mesh API.
- The production API fixture has area 12.6 mm², exact volume 37.8 mm³, five valid BRep faces, retained degrees [3,1,3], and 608 display triangles at detail 32.
- Fresh Vue application and MCP TypeScript checks passed. Scoped whitespace and mesher formatting checks passed.
- Production bundling and the unchanged distribution verifier passed: 171 artifacts. Geometry chunk: 3,699,795 bytes; application entry: 234,016 bytes. No size budgets were raised. Generated, public and production WASM hashes match.

The real production browser imported `cusp-solid.json`, displayed its retained
sketch and extruded body, and rendered the actual triangle mesh with smooth BRep
preview disabled. The cap and side faces were inspected by rotating the view.
WebGPU remained active at approximately 118–120 FPS. The captured browser
warning/error log is empty. The temporary browser tab and preview server were
closed after preserving the screenshot and fixture.

Evidence: `cusp-mesh-native-diagnostic.log`, `cusp-mesh-raw-wasm.log`,
`cusp-mesh-packaged-host-tests.log`, `cusp-mesh-production-verification.log`,
`cusp-mesh-fixture-metrics.json`, `cusp-mesh-browser-console.json`,
`cusp-mesh-browser.png`, and `cusp-solid.json`.

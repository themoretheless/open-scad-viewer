# manifold-core

Single public home for mesh manifoldness in `open-scad-viewer`.
See the [user guide](../../docs/manifold-user-guide.md) for check, repair,
metrics, boolean, TypeScript examples, and geometric limitations. The
[developer guide](../../docs/manifold-developer-guide.md) defines layer
ownership and the change/review workflow. The
[benchmarking guide](../../docs/manifold-benchmarking.md) documents rbench,
regression comparisons, and the external `manifold-3d` sidecars. The
[changelog](../../docs/manifold-changelog.md) records crate and ABI contracts.

A triangle mesh is a 2-manifold ("watertight") surface when every edge is
shared by exactly two consistently oriented triangles, every vertex link is
one connected cycle, and no degenerate triangles exist. Boolean BSP kernels
and slicers require this property.

## API

- `check(positions, indices) -> ManifoldReport` — full diagnostic:
  boundary edges, non-manifold edges (3+ faces), orientation mismatches,
  degenerate triangles, pinched (bowtie) vertices, isolated vertices,
  connected component count.
- `is_manifold(positions, indices) -> bool` — strict closed 2-manifold.
- `repair(positions, indices, epsilon) -> RepairOutcome` — conservative
  repair: vertex weld (exact or ε grid hash), degenerate removal,
  orientation unification via dual-graph BFS. It never drops faces to hide
  non-manifold edges or pinches; what it cannot fix is reported in
  `RepairReport.residual` (a `ManifoldReport` on the repaired mesh).
- `validate_vertex_links(vertex_count, indices)` — strict solid vertex-link
  validation (degree-2 link cycle, single fan). Canonical implementation
  used by the `polygon-core` boolean kernels.
- `metrics(positions, indices) -> MeshMetrics` — edge count, Euler
  characteristic, per-component breakdown (boundary edges, genus for closed
  components), signed volume (winding sign exposes inward orientation),
  surface area, watertight flag.

Edge incidence statistics are computed by the canonical packed-edge kernel
in `mesh-topology` (`EdgeUses`); vertex links and repair live here.

`mesh-topology` is the only runtime dependency; there is no polygon, host,
bridge, or WASM dependency. `positions` are f64 xyz triples, `indices` are
triangle corners.

## WASM / TypeScript

Exposed through `geometry-bridge` (`abi_manifold_check`,
`abi_manifold_repair`, `abi_manifold_metrics`, results via `abi_array_field`)
and re-exported by `geometry-wasm`. TypeScript wrappers live in
`src/services/geometry/manifoldAnalysis.ts`.

Previously this logic was scattered: private `vertex_manifold` in
`polygon-core` boolean, `close_topology` in `brep-core`, and the TS
`meshTopology` service.

## Required green checks

From the repository root, build the packaged WASM once with
`npm run build:geometry`, then run `npm run test:manifold`. This is the same
required contract used by CI: all `manifold-core`/`manifold-ops` tests, the
native manifold ABI smoke suite, fixed-seed property tests, and the focused
TypeScript/worker integration suite. The individual `test:manifold:core`,
`test:manifold:abi`, `test:manifold:property`, and `test:manifold:ts` commands
are reusable without duplicating their test selections in CI.

Required manifold tests must not be skipped, focused, marked as todo, or
ignored as pre-existing failures. `npm run test:architecture` audits this
policy and verifies that all four commands remain wired into CI. CI also runs
property and transport fuzz tests with a logged random seed, differential
tests, the native/WASM precision matrix, and the complete native test
partition. After a native-`wasm-opt` build, `npm run test:manifold:artifact`
verifies the packaged byte identity and manifold exports, then exercises
check, Full repair, metrics, and boolean on tetrahedra.

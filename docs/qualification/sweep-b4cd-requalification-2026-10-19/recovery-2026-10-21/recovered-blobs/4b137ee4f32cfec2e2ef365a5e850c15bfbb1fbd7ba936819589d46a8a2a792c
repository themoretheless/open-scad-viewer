# manifold-core

Single public home for mesh manifoldness in `open-scad-viewer`.

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

Zero dependencies. `positions` are f64 xyz triples, `indices` are triangle
corners.

## WASM / TypeScript

Exposed through `geometry-bridge` (`abi_manifold_check`,
`abi_manifold_repair`, results via `abi_array_field`) and re-exported by
`geometry-wasm`. TypeScript wrappers live in
`src/services/geometry/meshAnalysis.ts` (`checkManifoldInKernel`,
`repairManifoldInKernel`).

Previously this logic was scattered: private `vertex_manifold` in
`polygon-core` boolean, `close_topology` in `brep-core`, and the TS
`meshTopology` service.

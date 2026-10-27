# Manifold library user guide

`manifold-core` checks, repairs, and measures indexed triangle meshes.
`manifold-ops` adds union, intersection, and difference with a strictly
manifold output contract. The same native implementations are exposed to
TypeScript through the packaged geometry WASM; the TypeScript layer does not
reimplement the topology algorithms.

## Mesh representation and units

- `positions` is a flat xyz buffer: `[x0, y0, z0, x1, y1, z1, ...]`.
- `indices` contains three zero-based vertex indices per triangle.
- Rust uses `&[f64]` and `&[usize]`. TypeScript accepts `Float32Array` or
  `Float64Array` for positions and requires `Uint32Array` for indices.
- Mesh units are chosen by the caller. The TypeScript option name
  `weldEpsilonMm` reflects the application's millimetre convention.
- Winding matters: outward-oriented closed solids have positive signed
  volume. Consistent winding alone does not guarantee outward winding.

For untrusted Rust input, call `validate_indexed_mesh` before other APIs. It
rejects incomplete xyz/triangle triples, non-finite coordinates, and indices
outside the vertex array. The WASM wrappers perform this transport validation
and report failures as `GeometryKernelError`.

## Runnable Rust example

The complete check → Full repair → metrics → union example is
`crates/manifold-ops/examples/user_guide.rs`. From the repository root:

```sh
cargo run --locked --manifest-path crates/Cargo.toml \
  -p manifold-ops --example user_guide
```

Its central calls are:

```rust
use manifold_core::{RepairMode, check, metrics, repair_with_mode};
use manifold_ops::{BooleanOp, boolean_manifold};

let report = check(&positions, &indices);
let repaired = repair_with_mode(&positions, &indices, 0.0, RepairMode::Full);
let measured = metrics(&repaired.positions, &repaired.indices);
let union = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Union)?;
```

## `check`: topological diagnosis

```rust
let report = manifold_core::check(&positions, &indices);

if report.is_manifold() {
    // Closed, consistently oriented, non-degenerate 2-manifold topology.
} else if report.is_manifold_with_boundary() {
    // Valid manifold surface except for open boundary edges.
}
```

`is_manifold()` is the strict solid-boundary contract. It requires:

- every undirected edge to have exactly two incident triangles;
- opposite directed edge traversal in adjacent triangles;
- one connected cycle/fan around every used vertex;
- no repeated-corner triangles.

`is_manifold_with_boundary()` permits boundary edges but still rejects
non-manifold edges, orientation conflicts, pinched vertices, and degenerates.

The report contains counts plus defect locations:

| Field | Meaning |
| --- | --- |
| `boundary_edges` | Flat conceptual `(a, b)` pairs used by one triangle. |
| `non_manifold_edges` | Edges used by three or more triangles. |
| `orientation_edges` | Shared edges traversed in the same direction. |
| `degenerate_triangles` | Triangle ordinals with repeated corners. |
| `non_manifold_vertices` | Vertices with disconnected incident fans. |
| `isolated_vertices` | Vertices referenced by no triangle. |
| `component_count` | Connected components of the face-adjacency graph. |

In TypeScript, edge lists are flat `Uint32Array` values `[a0, b0, a1, b1,
...]`; triangle and vertex lists contain ordinals directly.

## `repair`: explicit best-effort changes

### Conservative mode

```rust
let out = manifold_core::repair(&positions, &indices, 0.0);
// Equivalent to repair_with_mode(..., RepairMode::Conservative).
```

Conservative repair performs only:

1. vertex welding;
2. removal of triangles whose corners collapse after welding;
3. orientation unification across shared edges.

It does not split pinched/non-manifold topology or fill holes.

### Full mode

```rust
let out = manifold_core::repair_with_mode(
    &positions,
    &indices,
    0.0,
    manifold_core::RepairMode::Full,
);

if !out.is_manifold() {
    eprintln!("residual defects: {:?}", out.report.residual);
}
```

Full repair additionally splits 3+-use edge shells and disconnected vertex
fans, then triangulates fillable simple boundary loops. Inspect
`out.report.residual` (or `out.is_manifold()`), not just the change counters.
A pass with `filled_holes == 0` can still succeed on an already closed mesh;
a pass that fills a hole does not guarantee that every other defect was fixed.

Important report fields are `welded_vertices`,
`removed_degenerate_triangles`, `flipped_triangles`, `split_vertices`,
`filled_holes`, `filled_triangles`, and `residual`.

`epsilon == 0.0` performs an exact bitwise weld. A positive epsilon uses a
conservative grid-cell search and Euclidean distance check; it can change
geometry and can collapse small triangles. Core repair clamps a negative
epsilon to zero. Boolean options are stricter; see below. The complete
selection and requalification procedure is in
`docs/manifold-tolerance-policy.md`.

## `metrics`: geometry and topology

```rust
let m = manifold_core::metrics(&positions, &indices);
println!("V={} E={} F={}", m.vertex_count, m.edge_count, m.triangle_count);
println!("chi={} volume={} area={}",
    m.euler_characteristic, m.signed_volume, m.surface_area);
```

Whole-mesh results include vertex/edge/triangle counts, Euler characteristic,
signed volume, surface area, a `watertight` flag, and per-component metrics.

- `euler_characteristic` is `V - E + F`, summed over components.
- `signed_volume` follows winding. Negative volume on a closed solid means
  inward orientation; repair unifies orientation but does not promise to make
  it outward.
- Whole-mesh volume includes closed components only. A component's own
  `signed_volume` is still exposed for diagnosis.
- `surface_area` is the sum of triangle areas, including open components.
- Per-component `genus` is `Some(g)` only for a closed component with a clean
  orientable-surface Euler relation; otherwise it is `None` (`null` in
  TypeScript).
- `watertight` means no boundary edges and a non-empty mesh. It is not a
  replacement for the full `check(...).is_manifold()` predicate.

## `boolean`: union, intersection, difference

```rust
use manifold_ops::{BooleanOp, boolean_manifold_with_options, BooleanOptions};

let out = boolean_manifold_with_options(
    &ap, &ai, &bp, &bi,
    BooleanOp::Difference, // a - b; operand order matters
    BooleanOptions {
        weld_epsilon: 0.0,
        relative_tolerance: 1e-9,
    },
)?;
assert!(out.output_report.is_manifold());
```

Available operations are `Union`, `Intersection`, and ordered `Difference`.
The pipeline is:

1. check each operand and apply Full pre-repair if needed;
2. reject an operand that remains non-manifold;
3. run the native BSP kernel with its work budget;
4. check the result and allow one conservative seam-repair pass;
5. reject any result that is still non-manifold.

After admission, the BSP requires closed, consistently outward-oriented
solids with positive signed volume; flip inward shells before the operation.

Consequently, a successful `BooleanOutcome` always has strictly manifold
output. `operand_repairs` records whether each input needed pre-repair;
`kernel_report` exposes tolerance, work, fragment, and input-triangle
diagnostics. Convenience methods include `volume()`, `surface_area()`,
`center_of_mass()`, `inertia_tensor()`, and `to_mesh()`.

Defaults are `weld_epsilon = 0.0` and `relative_tolerance = 1e-9`. Explicit
boolean options require weld epsilon in `0..=1` and relative tolerance in
`1e-12..=1e-5`. Relative tolerance is scaled by the longest combined input
bound side. A larger weld epsilon can merge intended detail; it is not a
general fix for self-intersections.

## TypeScript / packaged WASM

Import from `src/services/geometry/manifoldAnalysis.ts` after the geometry
kernel has been initialized (the application's CAD workers do this as part of
worker startup):

```ts
import {
  booleanManifoldInKernel,
  checkManifoldInKernel,
  metricsManifoldInKernel,
  repairManifoldInKernel,
} from './services/geometry/manifoldAnalysis'

const report = checkManifoldInKernel(positions, indices)
const repaired = repairManifoldInKernel(positions, indices, 0, true)
const measured = metricsManifoldInKernel(repaired.positions, repaired.indices)
const union = booleanManifoldInKernel(
  aPositions, aIndices, bPositions, bIndices, 'union',
  { weldEpsilonMm: 0, relativeTolerance: 1e-9 },
)
```

The runnable one-file sandbox is `examples/manifold-sandbox.mts`. After
`npm run build:geometry`, run it from the repository root:

```sh
npm run example:manifold
# Or load {"positions": [x,y,z, ...], "indices": [a,b,c, ...]}:
npm run example:manifold -- path/to/mesh.json
```

With no argument it loads an open tetrahedron, checks it, fills the boundary
with Full repair, computes its volume, and unions the repaired solid with a
disjoint translated copy. A file argument runs the same pipeline on that mesh
and throws if Full repair leaves residual non-manifold defects.

| Operation | TypeScript function |
| --- | --- |
| Check | `checkManifoldInKernel` |
| Conservative / Full repair | `repairManifoldInKernel(..., full = false / true)` |
| Metrics | `metricsManifoldInKernel` |
| Boolean | `booleanManifoldInKernel(..., 'union' / 'intersection' / 'difference')` |

Returned mesh buffers own copies and remain valid after the call. Repair and
boolean results always return `Float64Array` positions and `Uint32Array`
indices. These synchronous functions throw `GeometryKernelError` on failure;
a boolean never returns a successful non-manifold result.

## Limitations and required follow-up

- **Self-intersections are outside the manifold guarantee.** Edge incidence,
  vertex links, and winding are topological checks. Two triangles can cross
  in space while the indexed topology passes `is_manifold()`. Full repair
  does not detect or remove such crossings. Run geometric self-intersection
  analysis separately when the input is not known to be intersection-free.
- **Non-simple boundary loops are not filled.** Self-touching, repeated-
  vertex, repeated-edge, branching, degenerate, or numerically unclippable
  loops are left unfilled and remain visible in the residual boundary report.
  Fix the boundary topology first instead of increasing epsilon blindly.
- **Hole filling is not arbitrary surface reconstruction.** It triangulates a
  projected simple loop; it does not infer intended curved caps or recover
  missing interior geometry.
- **Topological manifoldness is not physical validity.** Zero-thickness,
  overlapping shells, inward shells, and other geometric intent issues need
  their own validation.
- **A failed boolean is explicit.** Unrepairable operands, kernel budget or
  classification failures, and non-manifold kernel output must be handled;
  they are not converted into an approximate successful mesh.

See `docs/manifold-error-catalog.md` for every structured failure and its
recommended recovery strategy.

For the required local verification contract, run `npm run build:geometry`
once and then `npm run test:manifold` from the repository root.

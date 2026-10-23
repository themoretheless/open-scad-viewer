# mesh-topology

Indexed edge incidence, directed boundary loops, binary64 mesh inspection and
float32 display edge extraction. Uses `osv-math`; no polygon or host dependency.

`MeshView` borrows xyz triples, triangle indices and optional UVs. Its validation
limit is 100,000 triangles and 300,000 vertices. Inspection preserves the original
polygon kernel's relative area test and compensated signed volume calculation.
`Inspection::closed` checks edge closure, winding and degeneracy. It does not
check pinched vertex links, self intersections or shell nesting. Use
`manifold-core` for full vertex-link manifoldness and conservative mesh repair.

`measure::SurfaceArea` accumulates affine world-space triangle area in binary64
with compensated summation across batches. It takes flat triangle point triples
and a row-major affine matrix; translation cancels before edge arithmetic. The
caller controls batch size and cooperative scheduling.

```rust
use mesh_topology::MeshView;
let positions = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
let indices = [0, 1, 2];
let mesh = MeshView::new(&positions, &indices);
assert_eq!(mesh.inspect()?.boundary_edges, 3);
assert_eq!(mesh.boundary_loops()?, vec![vec![0, 1, 2, 0]]);
# Ok::<(), mesh_topology::Error>(())
```

`EdgeUses::new` accepts triangle triples with endpoints below 2^31 and panics
when these structural conditions are violated. It retains exact indexed
identity. `edges::extract_semantic_edges` instead accepts float32 xyz/normal
records (stride 6), optional merge pairs, exact-position welding and a crease
threshold. Its input index ranges, paired merge arrays and finite threshold must
be validated by the caller. Both synchronous and cooperative versions preserve
float32 rounding, edge ordering and coplanar diagonal removal.

The cooperative future is manually repolled by its owner after a checkpoint;
it deliberately does not wake an executor. Dropping it releases partial state.

## Internal organization

Buffer views and validation live in `view`, mesh diagnostics in `inspection`,
boundary contour walks in `boundary`, packed edge incidence in `indexed`, and
float32 display edge extraction in `edges`. The crate root reexports the public
types, so consumers keep using `mesh_topology::MeshView` and `Inspection`.

## Coplanar display topology

`planar::topology(MeshView)` returns typed planar faces and display seams in
first-seen order, including triangle and vertex membership. Decimal seam keys
retain the legacy seven-place rounding. Coplanar disconnected triangles may
share a face group; these groups are for selection and sampled operations.
`planar::face_plane` validates face vertices and returns an orthonormal workplane.

```rust
use mesh_topology::{MeshView, planar};
let positions = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
let indices = [0, 1, 2];
let mesh = MeshView::new(&positions, &indices);
let topology = planar::topology(mesh)?;
let face = &topology.faces[0];
let plane = planar::face_plane(mesh, &face.vertices, face.normal)?;
assert_eq!(plane.origin, [0., 0., 0.]);
# Ok::<(), mesh_topology::Error>(())
```

## Exact welding

`weld::exact` validates a borrowed `MeshView` and returns owned position/index
buffers in first-use order. Exactly equal coordinates share identity, including
signed zeros; nearby coordinates stay distinct. Unused vertices and UVs are
removed, after validating the original UV buffers. Welding does not repair or
reject degenerate triangles.

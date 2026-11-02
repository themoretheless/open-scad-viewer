# subdivision-core

Own Rust Catmull–Clark implementation, without C/C++ or OpenSubdiv runtime.
`Cage { vertices, faces }` stores an indexed, consistently oriented polygon control
mesh. `subdivide(levels)` returns a new cage and original polygon IDs.
`Refined::triangulate()` returns a neutral `geometry_ops::Triangles` buffer and one
original face ID per triangle. The source cage remains the editable representation.

Boundary vertices use `(6P + previous + next)/8`; boundary edge points are midpoints.
Interior face, edge and vertex points follow Catmull–Clark rules. Disconnected
vertex fans, edge orientation conflicts, nonmanifold edges and unused vertices
are rejected. Limits: 0–5 levels, 25000 faces, 100000 corners, vertex valence 256.

`Cage::subdivide` performs finite uniform refinement. `QuadSurface` additionally
retains regular bicubic limit patches and extraordinary/boundary limit stencils;
finite patch coverage near extraordinary points remains bounded. Separate crease
operations support semi-sharp refinement. Triangle cages also expose Loop
refinement; face-varying UVs remain unsupported. Fan
triangulation assumes convex faces; folded/concave geometric faces and
self-intersections are not certified. Topological validation does not establish
solid validity. Refined patches retain original face IDs for selection.

Reference: https://graphics.pixar.com/people/derose/publications/Geri/paper.pdf

`Cage::from_mesh` imports welded triangle topology. `reconstruct(mesh,iterations)`
fits the control positions to original-vertex samples at one subdivision step,
with backtracking and a separate sampled geometric-deviation report. This does
not infer a coarse quad layout. See docs/design/mesh-reconstruction.md.

## Native dependency boundary

Use `default-features = false` for native `Cage`, `Refined` and triangle-buffer
operations. Runtime dependencies are `osv-math`, `geometry-ops`, `cad-predicates` and `rustc-hash`;
`geometry-ops` also disables its default codec. The default `codec` feature
retains the existing value transport implementations in a separate module.

```rust
use subdivision_core::Cage;
let cage = Cage::from_faces(
    vec![[0.,0.,0.], [2.,0.,0.], [2.,2.,0.], [0.,2.,0.]],
    vec![vec![0,1,2,3]],
)?;
let refined = cage.subdivide(2)?;
let (triangles, source_faces) = refined.triangulate()?;
assert_eq!(source_faces.len(), triangles.indices.len()/3);
assert!(source_faces.iter().all(|&id| id == 0));
# Ok::<(), subdivision_core::Error>(())
```

## Quad control deletion

`QuadSurface::delete_vertex` and `delete_vertices` replace one connected disk of
original vertex stars with quads. Only selected controls are removed; all other
coordinates remain bit-identical. The result owns source/result snapshots,
vertex/edge/face maps, replacement-face ownership and independent `reverify`.
Exact predicates use original XYZ coordinates and a chosen projection normal,
without rounding projected points before deciding orientation or intersections.

Boundary parity, simple projected fill, retained-edge incidence, manifold fans,
Euler characteristic, components and boundary-loop counts gate publication.
Original unselected interior controls are retained by exact-checked quad
splits, followed by independent replay of the actual active disk. Selections
requiring new controls, multiple holes, unresolved projected insertion or
exhausted construction/proof work return errors. This changes control topology; preserving the old limit shape, global embedding and
solid validity are separate unresolved requirements.

# subdivision-kernel

Own Rust Catmull–Clark implementation, without C/C++ or OpenSubdiv runtime.
`Cage { vertices, faces }` stores an indexed, consistently oriented polygon control
mesh. `subdivide(levels)` returns a new cage and original polygon IDs.
`Refined::triangulate()` returns the shared `polygon_kernel::Mesh` and one original
face ID per triangle. The source cage remains the editable representation.

Boundary vertices use `(6P + previous + next)/8`; boundary edge points are midpoints.
Interior face, edge and vertex points follow Catmull–Clark rules. Disconnected
vertex fans, edge orientation conflicts, nonmanifold edges and unused vertices
are rejected. Limits: 0–5 levels, 25000 faces, 100000 corners, vertex valence 256.

This is finite uniform refinement, not exact limit-surface evaluation. No crease
weights, face-varying UVs, adaptive subdivision or Loop scheme yet. Fan
triangulation assumes convex faces; folded/concave geometric faces and
self-intersections are not certified. Topological validation does not establish
solid validity. Refined patches retain original face IDs for selection.

Reference: https://graphics.pixar.com/people/derose/publications/Geri/paper.pdf

`Cage::from_mesh` imports welded triangle topology. `reconstruct(mesh,iterations)`
fits the control positions to original-vertex samples at one subdivision step,
with backtracking and a separate sampled geometric-deviation report. This does
not infer a coarse quad layout. See docs/design/mesh-reconstruction.md.

# sdf-core

Own Rust negative-inside implicit fields and marching-tetrahedra mesh extraction.
No native/C++ runtime. `Field` supports sphere, axis-aligned box, Z-axis torus,
union, intersection, difference, polynomial smooth union, offset and translation.
`evaluate(point)` samples a validated tree; `polygonize(field, grid)` returns a
neutral `geometry_ops::Triangles` buffer. Mesh inspect lives in the bridge.
`polygonize_with(closure, grid)` also supports arbitrary user-supplied scalar
fields in native Rust.

Primitive fields are signed distances. Boolean combinations and offsets of those
combinations are implicit fields, not generally exact distances. Offsetting such
a field does not promise a geometrically exact constant-distance surface.

Extraction uses a consistent six-tetrahedron cube decomposition with shared edge
vertices and outward winding. Exact-zero samples share grid vertices; near-zero
samples snap within 32 machine epsilons times the largest grid extent. Limits:
256 field nodes, depth 32, 1–64 cells per axis, 100000 output triangles. Bounds
must have strictly positive boundary samples: enlarge bounds if rejected.
Sub-cell features can be missed, including ones between boundary samples. Empty
meshes are valid. The grid spacing is not a certified global surface-error bound;
inspect the mesh report before solid operations/export. No automatic repair,
adaptive octree or recovery of an unknown original procedural field.

`Field::from_triangles(mesh,signed)` constructs triangle distance fields; closed oriented
meshes support signed distance, open meshes unsigned distance. Solid-angle winding
provides the sign. Sources have at most 4096 triangles; extraction is bounded to
8 million triangle/primitive samples. See docs/design/mesh-reconstruction.md.

Reference: https://paulbourke.net/geometry/polygonise/

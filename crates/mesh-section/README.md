# mesh-section

Indexed horizontal mesh sections and planar projection. Runtime dependencies:
`osv-math`, `mesh-topology`, `planar-geometry`. Inputs are borrowed binary64 buffers;
the reusable section index owns exact vertex identities and a height index.

```rust
use mesh_topology::MeshView;
use mesh_section::MeshSectionIndex;
let positions = [0.,0.,0., 1.,0.,0., 0.,1.,0., 0.,0.,1.];
let indices = [0,2,1, 0,1,3, 1,2,3, 2,0,3];
let index = MeshSectionIndex::new(&MeshView::new(&positions, &indices))?;
let section = index.section(0.5)?;
assert_eq!(section.contours.len(), 1);
assert_eq!(section.contours[0].points.len(), 3);
# Ok::<(), mesh_section::Error>(())
```

`section(z)` uses `min_z <= z < max_z`: bottom sections are included and top
sections excluded. Exactly coincident vertices, including signed zero, share
identity; nearby vertices are not welded. Contours retain winding and the source
triangle of every outgoing segment. Open or branching boundaries and unresolved
interpolation return typed errors. `section_for_display` preserves collapsed
coordinates and reports their source triangles; these contours are display data.

The index cuts horizontal planes. For an arbitrary plane, the caller must first
express vertices in that plane's coordinate frame. Contours require subsequent
arrangement/material validation before becoming printable regions. `slice` is
the legacy tolerance-based planar arrangement path; `project` unions triangle
projections. Toolpath generation remains in `slicer-core`.

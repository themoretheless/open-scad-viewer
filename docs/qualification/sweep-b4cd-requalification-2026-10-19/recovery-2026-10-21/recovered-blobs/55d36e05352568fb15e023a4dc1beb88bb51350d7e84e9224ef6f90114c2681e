# mesh-io

Mesh serialization and deterministic 3MF OPC packaging, borrowing `MeshView`.
Runtime dependencies: `osv-math`, `mesh-topology`, `crc32fast`, `flate2`.

```rust
use mesh_io::{MeshView, export};
let positions = [0.,0.,0., 1.,0.,0., 0.,1.,0.];
let indices = [0,1,2];
let bytes = export(&MeshView::new(&positions, &indices), "obj")?;
assert!(String::from_utf8(bytes)?.ends_with("f 1 2 3\n"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

- `export`: ASCII/binary STL, OBJ, PLY, OFF and AMF, with a 4 MiB artifact budget.
  STL and AMF require closed, consistently oriented positive-volume meshes.
  Binary STL rejects float32 rounding that collapses source triangles.
- `export_print_mesh`: STL, OBJ or 3MF.
- `model_3mf::export`: preserves individual objects, validates exported part totals
  (100,000 triangles / 300,000 vertices), bounds expanded XML at 64 MiB.
- `package_3mf::export`: fixed OPC paths, CRC32, stored or raw-DEFLATE ZIP,
  deterministic headers, 4 MiB final artifact budget.
- `export_prepare::prepare`: places float64 xyz/normal records with an affine
  matrix, reverses mirrored winding, optionally rounds to float32, filters
  collapsed triangles and compacts referenced vertices.
- `export_file::Builder`: transactional scene binary STL or OBJ; up to 750,000
  source triangles and 256 MiB. Any append failure poisons the session and
  prevents committing a partial file.

Units and legacy file headers are preserved from the product. Topological
admission checks edge closure and winding, not self intersections or vertex-link
manifoldness. Full manifold analysis belongs to `manifold-core`. Polygon adapters
retain the existing product error codes; standalone calls expose mesh error codes.

## Import

`import::{obj, ply, stl, off}` return typed raw meshes. `import::finalize` applies
`Weld::{None, DisplayExact, Grid}` and reports source vertices and collapsed
triangles. DisplayExact preserves the previous float32-coordinate welding
contract; Grid rounds ties toward positive infinity. Inputs retain their first
source coordinate after welding. Limits: 250,000 triangles and 750,000 vertices.

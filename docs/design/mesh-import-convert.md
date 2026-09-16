# Mesh import and format conversion

Companion to [mesh-export-formats.md](mesh-export-formats.md). The importer and
converter add no dependency: decoding is TypeScript in
`src/services/meshImport.ts`, serialization is the Rust kernel through
`src/services/meshExportFormats.ts`, and `src/services/meshConvert.ts` joins the
two.

## Import formats

| Format | Encodings | Decoder | Notes |
|---|---|---|---|
| stl | ASCII, binary | shared OpenSCAD `import()` decoder | Binary detected by exact size; file normals ignored |
| obj | text | `meshImport` | `v` (with optional `w`), `f` with `v/vt/vn` slots, negative indices, polygon fans; `vn`/`vt`/`o`/`g`/`s`/`mtllib`/`usemtl`/`l`/`p` ignored |
| ply | ASCII, binary little/big endian | `meshImport` | Needs `vertex` element with scalar `x y z` and `face` element with a `vertex_indices`/`vertex_index` list; other elements and properties are read and discarded |
| off | text | shared OpenSCAD `import()` decoder | Polygon faces triangulated |
| amf | XML, gzip | shared OpenSCAD `import()` decoder | Units scaled to millimeters |
| 3mf | OPC ZIP | shared OpenSCAD `import()` decoder | Build items and transforms applied; objects flattened |

Detection uses the file extension first (`.amf.gz` included), then content
sniffing (`ply`, `OFF`, `solid`, `<amf`, OBJ statements, ZIP magic, binary STL
size). Limits: 20 MB per file, 250,000 triangles, 750,000 vertices, plus the
XML/ZIP budgets of the shared decoder.

### Output mesh

Every decoder yields one welded, indexed triangle mesh: positions and triangle
connectivity only. Colours, textures, UVs, units metadata, per-object names and
assembly transforms are dropped. Welding merges bit-identical positions by
default (enough to close well-formed STL/PLY triangle soups without moving any
vertex); callers may pass an absolute tolerance (the Mesh and Solid workbenches
use `1e-4`) or `weld: false` to keep the file's own indexing. Triangles that
collapse after welding are removed and counted.

## Conversion paths

| Surface | Entry point |
|---|---|
| Browser toolbar | **Convert…** button (or drop a mesh file onto the app): converts to the format chosen in the export selector and downloads it |
| Command palette | `convert-mesh` |
| Mesh workbench | Import STL/OBJ/PLY/OFF/AMF/3MF as an object; download the selected object in any export format |
| Solid workbench | Import any mesh format as a body; download the selected body in any export format |
| MCP | `mesh_convert` — base64 file in, embedded base64 resource out, with source statistics |
| CLI | `node --import tsx scripts/convert-mesh.ts input.stl output.3mf` (after `npm run build:geometry`) or `--to <format>`, `--out <dir>`, `--weld <tol>\|off`, `--source-format`, `--no-compress` |

Output formats and their geometry requirements are listed in
[mesh-export-formats.md](mesh-export-formats.md). STL, 3MF and AMF writers
require a closed, consistently oriented mesh with positive volume; OBJ, PLY
and OFF accept open surfaces. A `.stl` output extension on the CLI selects
binary STL; `--to stl` selects ASCII.

## Not covered

glTF/GLB, STEP, IGES, DXF/SVG (2D) and NEF3 are outside this converter. STEP
`FACETED_BREP` import/export remains in the CAD workbench, and SVG/DXF remain
2D `import()` sources.

## Tests

`tests/meshImportConvert.test.ts` covers detection, OBJ/PLY parsing (ASCII and
both binary byte orders), STL welding, round trips of every kernel writer
through the importer, the full input × output conversion matrix, open-mesh
refusals and typed errors. `tests/mcpServer.test.ts` exercises `mesh_convert`
end to end (OBJ → 3MF → PLY) including error codes.

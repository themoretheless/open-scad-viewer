# Mesh export formats

The shared exporter has no added dependency. Browser export uses current eligible scene geometry; modelgraph_export builds ModelGraph/1, and modelgraph_nurbs_export uses the own NURBS process. NURBS JSON export retains the source graph; mesh exports are explicitly derived geometry.

| Format key | Encoding | Geometry requirement |
|---|---|---|
| stl | ASCII | Closed, consistently oriented, positive volume |
| stl_binary | Little-endian float32; .stl filename | Same; reject precision-induced degenerate triangles |
| 3mf | Deterministic ZIP/OPC + core XML | Closed, consistently oriented, positive volume |
| amf | Uncompressed XML | Closed, consistently oriented, positive volume |
| obj | Text vertices and 1-based triangular faces | Open surfaces allowed |
| ply | ASCII, double coordinates and indexed faces | Open surfaces allowed |
| off | Text vertices and 0-based indexed faces | Open surfaces allowed |

The print-mesh trio for production artifacts is STL, OBJ, and mesh 3MF via
`mesh_export::export_print_mesh`. See [print-export-formats.md](print-export-formats.md)
for the five-format surface (those three plus machine `.gcode` and `.gcode.3mf`).

3MF writes [Content_Types].xml, _rels/.rels and 3D/3dmodel.model. Model units are millimeters. The ZIP writer uses stored entries, UTF-8 names, CRC32 and deterministic timestamps. Package layout follows the [3MF Core specification](https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md). This is a geometry package, not a slicer project: no printer settings, textures, colors or assembly identities are exported. STL/OBJ/PLY/OFF coordinates use millimeters even when the file format does not encode units formally.

Maximum 100000 triangles and 4 MiB per new-format artifact. Existing browser STL/OBJ exporters retain their previous policies. Exports validate indices, finite coordinates and mesh topology; they do not certify self-intersection freedom or printability. No STEP or general B-rep export is implied.

The reverse direction (importing any of these formats and converting between them) is described in [mesh-import-convert.md](mesh-import-convert.md).

Tests independently reimport ASCII/binary STL, 3MF, AMF and OFF with the existing import subsystem and compare triangle counts and bounds. OBJ indexing, PLY declarations, determinism, transformed/mirrored geometry, open-surface refusals and MCP binary transport have regression coverage. Browser 3MF download is checked through the actual UI.

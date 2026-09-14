# Print export formats (five)

Production artifact surface for mesh + toolpath export. No Worker/UI and no
live printer transport in this contract.

## Formats

| # | Format | Crate API |
|---|---|---|
| 1 | STL | `polygon_core::mesh_export::export_print_mesh(mesh, "stl")` |
| 2 | OBJ | `export_print_mesh(mesh, "obj")` |
| 3 | mesh 3MF | `export_print_mesh(mesh, "3mf")` → stored OPC |
| 4 | machine `.gcode` | `gcode_core::emit_job` / `parse_job` |
| 5 | `.gcode.3mf` | `gcode_core::emit_gcode_3mf_job` (+ optional `MeshBody`) |

Happy path after slicing:

`ToolpathLayer` → `slicer_core::emit_optimized_gcode` (preview) or
`emit_job_gcode` / `emit_job_gcode_3mf` (machine job).

Host ops: `mesh_gcode` (preview) and `mesh_gcode_job` (job text +
`gcode3mfBase64`). Mesh path: `Mesh` → `export_print_mesh` for STL / OBJ / mesh 3MF.

## Dialects

- **Preview** (`open-scad-viewer/print-preview 2`): model-space motion only.
  Used by UI preview and `emit` / thin `emit_3mf`.
- **Job** (`open-scad-viewer/print-job 1`): heat, optional home, fan, absolute-E
  retract, cooldown. Packaged `.gcode.3mf` may include a real mesh body and
  `Metadata/plate_1.json` with nozzle/bed targets and G-code MD5.

## Non-goals

AMS UI, multi-plate UI, “true Bambu slicer” parity, and expanding the print-mesh
set beyond STL/OBJ/3MF (AMF/PLY/OFF remain available on `mesh_export::export`
but are not part of this five-format surface). Live LAN FTPS/MQTT lives in
`printer-core` (`network` feature), not in these export APIs.

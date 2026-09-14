# gcode-core

Bounded serialization and independent parsing of model-space toolpath previews
and a separate machine **job** dialect. This crate does not section meshes, plan
walls/infill, or communicate with a printer.

## Five print/export formats

| Format | Owner | Role |
|---|---|---|
| STL | `polygon-core::mesh_export::export_print_mesh` | Closed mesh |
| OBJ | same | Mesh interchange (open surfaces allowed) |
| mesh 3MF | same (`"3mf"`) | Closed mesh OPC package |
| `.gcode` job | `emit_job` / `parse_job` | Heat, retract, start/end |
| `.gcode.3mf` | `emit_gcode_3mf_job` / `package_job_3mf` | Job G-code + plate JSON + optional mesh |

Preview `.gcode` / thin `emit_3mf` remain available for UI previews and are not
printer jobs.

## Preview vs job

| | Preview (`print-preview 2`) | Job (`print-job 1`) |
|---|---|---|
| First line | `; open-scad-viewer/print-preview 2` | `; open-scad-viewer/print-job 1` |
| Heat / home / fan | Forbidden | `M140`/`M104`/`M190`/`M109`, optional `G28`, `M106`/`M107` |
| Retract | Forbidden (E never decreases) | Absolute-E `G1 E…` retract / unretract |
| Prologue | `G21` `G90` `M82` `M200 D0` `G92 E0` | Same after heat |
| Package | Empty `3D/3dmodel.model` | Optional mesh body + `Metadata/plate_1.json` (MD5) |

## Preview contract

The exact first line is `; open-scad-viewer/print-preview 2`. A single
`;FILAMENT_DIAMETER_MM:<positive diameter>` annotation is required before layers.
The command prologue is, in order:

```gcode
G21
G90
M82
M200 D0
G92 E0
```

Coordinates are millimeters, XYZ and E are absolute, and E measures filament
length. Feedrate is supplied as mm/s in `MachineProfile` and stored as `F` in
mm/min. `M200 D0` explicitly disables volumetric extrusion, following the
[Marlin configuration documentation](https://marlinfw.org/docs/configuration/configuration.html).
This declaration does not make the preview a portable machine job.

Legacy version 1 previews lack required extrusion-mode and filament metadata
and may contain printer shutdown commands. They are rejected with a diagnostic
requesting regeneration through the current exporter.

Each layer starts with a consecutive, zero-based `;LAYER:<index>` marker and a
move establishing its Z coordinate. Layers increase in Z. Optional `;Z:` metadata
must match that coordinate; further Z changes require another layer marker.
The emitter rounds XYZ to five decimal places before computing segment lengths.
It writes E to seven decimal places and F to three. Zero-length rounded segments
are omitted. A positive-length segment whose E cannot advance at the exported
precision is rejected instead of silently losing deposition. Closed paths need
at least three vertices, and their closing segment is included without copying
the point buffer.

The parser accepts only this explicit preview dialect. After the prologue, only
whitespace-separated `G0`/`G1` commands with X/Y/Z/E/F words are supported. Omitted
axes and F retain their previous values. E may stay constant or increase; only a
positive E delta on a nonzero, fixed-Z `G1` XY segment counts as deposition.
Retraction, extrusion-only motion, arcs, mode changes, G92 resets, heater/home
commands, unknown commands, duplicate words, invalid numbers, and contradictory
layer metadata fail with a typed `GCODE_*` error. In-file errors include their
one-based line number. Blank lines, semicolon comments, Unicode inside ordinary
comments, and CRLF are supported. Malformed Unicode command words return errors.

## Job contract

`JobProfile` wraps `MachineProfile` plus nozzle/bed temperatures, retract
length/speeds, fan PWM, and optional homing. `emit_job` writes heat, optional
`G28`, the same units/modes prologue, layer motion with retract on long travels,
then fan off, optional `G28 X Y`, and `M104`/`M140` cooldown. `parse_job` accepts
that dialect for round-trip checks and returns the same `GcodePreview` totals
shape (peak absolute E for volume).

## Preview data

`parse` returns `GcodePreview`:

- `moves`: full XYZ destinations, absolute filament `e` in mm,
  `feedrate_mm_s`, zero-based `layer_index`, and `extruded` from the E delta.
- `bounds: Option<GcodeBounds>`: minimum/maximum XYZ of known destinations.
- `layers`, `extrusion_mm`, and `deposited_volume_mm3`. Volume uses the file's
  filament diameter and actual exported E.
- `travel_distance_mm`, `print_distance_mm`, and `estimated_time_s`.

XYZ starts unknown: initial Z-only setup does not create a drawable move. The
first fully specified XYZ destination establishes the start with zero distance
and time. Bounds and totals exclude any assumed machine origin. Subsequent
travel includes vertical moves between layers. Nominal time is segment length
divided by modal commanded speed; it excludes acceleration, heating, homing,
retraction, and other printer behavior. It is a preview estimate.

The rectangular bead approximation is `line_width * layer_height`.
`deposited_volume_mm3(layers, machine)` is an analytic helper for an unquantized
plan; callers validate its inputs. Use parsed volume to describe an exported
file, because coordinate and extrusion rounding can change its volume.

## Resource and numeric limits

Emission and parsing enforce 2,048 layers, 100,000 motion commands, and 4 MiB
output/input. Emission preflights the planned vertices and paths and checks the
byte budget on every append, including within a single layer. Empty paths count
toward a 100,000-path budget. The conservative planned-move budget includes
vertices that may collapse after rounding. Parsing limits each line to 1,024
bytes and counts motion commands even before XYZ is fully known.

All coordinates must be finite and within +/-1,000,000 mm. Profile dimensions
(layer height, line width, filament diameter) must be 0.00001..1,000,000 mm;
feedrates must be 0.001/60..1,000,000 mm/s. Derived settings, extrusion
accumulation, and preview totals must remain finite. Consecutive layer heights
must remain distinct at the 0.00001 mm export resolution. Public constants
expose these shared limits for callers.

## G-code 3MF

`package_gcode_3mf` / `emit_3mf` write a stored OPC ZIP with
`Metadata/plate_1.gcode` plus an empty millimeter `3D/3dmodel.model`.
`package_job_3mf` / `emit_gcode_3mf_job` add `Metadata/plate_1.json` (temps,
filament placeholder, G-code MD5) and may embed a non-empty model from
`MeshBody`. `extract_gcode_3mf` / `parse_3mf` read the plate G-code back
(`parse_3mf` dispatches preview vs job by dialect line). This is a file
container, not LAN upload and not a Bambu machine-job certificate.

## Non-goals

Live FTPS/MQTT, AMS, multi-plate UI, Worker download buttons, and full Bambu
slicer parity are out of scope for this crate.

```sh
cargo test --locked --manifest-path crates/Cargo.toml -p gcode-core
```

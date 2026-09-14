# gcode-core

FDM G-code library: serialize a completed print plan and independently parse
Marlin/RepRap-style files back into motion, extrusion, and process state.
It does not section meshes, plan walls/infill, or send jobs to a printer.

Coordinates default to millimeters. Internal feedrate is mm/s and is written
as `F` in mm/min. The writer dialect is `open-scad-viewer/print-preview 1`.
Bead volume for planned paths is the engineering rectangle `width * height *
length`, not a physical melt certificate.

## Parse and interpret

`parse` requires the preview dialect header. `parse_fdm` / `parse_fdm_with`
read the same IR without that header. Typed commands:

- Motion: `G0`/`G1`. `G2`/`G3` parse into the IR; interpretation returns
  `GCODE_UNSUPPORTED_ARC` unless `MachineProfile.linearize_arcs_mm` is set.
- Modal: `G20`/`G21`, `G90`/`G91`, `M82`/`M83`, `G92`, `G28`, `G17`
- Extrusion: filament or volumetric `M200`, flow `M221`
- Temperature: `M104`/`M109` hotend, `M140`/`M190` bed, `M141`/`M191` chamber
  (tracked; they do not change deposited volume)
- Fan `M106`/`M107`, tool `T`, dwell `G4`, firmware retract `G10`/`G11`
- Other `M`/`G` kept in the program; they increment `other_commands`

Positive E on an XY move is deposited filament. E-only retract/unretract is
not deposited volume. Cutter compensation and canned cycles (`G41`/`G81`…)
return `GCODE_UNSUPPORTED`.

## Emit

`MachineProfile` selects units, XYZ/E modes, temps, fan, retract, z-hop,
optional arc linearization, and start/end templates (lexed, size-limited).
Templates and generation never open a serial port.

Limits: 16 MiB input, 4 MiB output, 2048 layers, 500k blocks/moves, 4096-byte
lines.

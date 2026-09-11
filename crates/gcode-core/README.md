# gcode-core

Machine-output library for a completed print plan. It serializes planned
layer paths to G-code and independently parses that file back into moves.
It does not section meshes, plan walls/infill, or send jobs to a printer.

Coordinates are millimeters. Internal feedrate is mm/s and is written as
`F` in mm/min. The current dialect is `open-scad-viewer/print-preview 1`:
absolute XYZ, absolute E as filament length (`G90`, `M82`), rectangular
bead area `width * height`. This is an engineering approximation, not a
physical melt certificate.

Limits: finite positive machine settings, 2048 layers, 4 MiB output.
Unsupported here: other dialects, relative/volumetric E, retract, z-hop,
arcs, temperature inserts, start/end templates, and printer I/O.

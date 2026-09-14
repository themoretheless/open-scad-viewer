# slicer-core

Plans bounded, model-space toolpaths from horizontal `LayerSection` contours and delegates preview G-code encoding and parsing to `gcode-core`. The host supplies mesh sections; this crate does not own a mesh kernel or a printer connection.

## Geometry contract

- Coordinates and lengths are millimeters. Each nonempty contour has at least three vertices. Rings are implicitly closed; an explicit repeated closing vertex is accepted by normalization.
- Contours use the nonzero winding fill rule. Outer boundaries and holes have opposite orientations; reversing every contour preserves the region. Overlapping contours are normalized before planning.
- `plan_layer` preserves the section's Z. Outline centerlines lie half a line width inside the material. Further wall centerlines are spaced one line width apart. Infill is horizontal, with alternating row direction, clipped to the region inset by `wall_count * line_width_mm`.
- Insets subtract a boundary stroke from the source region. A consumed wall or narrow neck becomes empty or separates into islands. It cannot reappear as an inverted offset contour. Stroke joins use the geometry kernel's miter join with a miter limit of four.
- Empty material produces an empty plan. The planner does not invent centerlines for features narrower than a line width.

## Height scheduling

`schedule_layers(section_at, z_min, z_max, settings)` samples
`z_min + i * layer_height_mm` in the half-open interval `[z_min, z_max)`.
It computes each sample from its integer index to avoid accumulated step error.
For example, `[0, 10)` at 0.2 mm yields 50 samples, from 0 through 9.8 mm.

The callback must return the exact requested `z_mm`, including for empty sections.
All requested planes count toward the layer limit. The entire sample count is
validated before the callback runs; a range requiring too many empty sections is
rejected immediately. Layers with no resulting paths are omitted from the output.

Z values are model coordinates and may be negative or zero. This contract describes
preview sample planes. It does not select a printer's first-layer nozzle height,
translate a model to a build plate, or model a partial final layer. Volume uses the
nominal layer height for every retained path.

## Admission and work limits

Limits apply to a single `plan_layer` call or, cumulatively where stated, an entire
`schedule_layers` call. Exceeding a limit returns an error instead of a partial plan.

| Resource | Limit |
| --- | --- |
| Requested layer planes, including empty sections | 2,048 |
| Source contour vertices per section | 4,096 |
| Source contours per section | 1,365 |
| Hatch scan lines per layer, including empty rows | 65,536 |
| Source, intermediate and output vertices across a plan | 1,000,000 |
| Conservative geometry and hatch work across a plan | 64,000,000 units |
| Coordinate magnitude, including Z | 1,000,000 mm |
| Layer height, line width and filament diameter | 0.00001–1,000,000 mm |
| Infill spacing | 0.00001–1,000,000 mm |
| Wall count | 1–8 |
| Total requested wall inset | At most 1,000,000 mm |

Geometry work charges squared arrangement sizes and hatch edge/sort estimates
before the corresponding operations. The conservative estimate can reject a
detailed plan even when its final output would be small. The planar kernel also
applies its own intersection and arrangement limits. Host sectioning work requires
a separate host-side budget.

Machine settings additionally pass `gcode_core::MachineProfile::validate`, including
feedrate precision and finite derived extrusion calculations. The encoder applies
its own path, move, coordinate precision and 4 MiB output limits. Caller-supplied
`ToolpathLayer` collections are bounded before copying them into encoder structures.
`deposited_volume_mm3` returns NaN for invalid settings or oversized plans; callers
must supply geometrically valid paths for this analytic estimate.

## Preview and job encoding

`emit_gcode` still writes the raw preview dialect without reordering paths.
`emit_optimized_gcode` runs `gcode-optimize` then preview emit.
`emit_job_gcode` / `emit_job_gcode_3mf` run the same optimize pass into the
machine job dialect (and thick `.gcode.3mf` with optional `MeshBody`).
`parse_gcode_preview` / `parse_gcode_job` validate those dialects.
`GCODE_DIALECT`, `GCODE_JOB_DIALECT`, `JobProfile`, `OptimizeSettings`,
`GcodeBounds`, `GcodePreview` and `GcodeMove` are reexported for consumers.

The planner itself has no top/bottom solid-layer strategy, supports, adaptive
layers, pressure advance, or printer-profile scheduling beyond what `JobProfile`
encodes at emit time.

Run the regression suite with:

```sh
cargo test --locked --manifest-path crates/Cargo.toml -p slicer-core
```

# G-code export and preview

The CAD workbench generates a bounded, deterministic preview from one scene
body. It uses the displayed body's placement, exposes the layer, line width,
wall count, infill spacing, print/travel speeds and filament diameter, and
displays the exported file's movements one layer at a time. Travel and
deposition have separate colors. An existing file in the same preview dialect
can also be opened.

## Execution and file contract

Slicing and parsing run in a dedicated browser worker. Cancel, a changed input,
a changed model or a closed panel invalidates the result and terminates the
worker. A completed result is downloadable only while its inputs remain current.
Errors return without publishing a partial file.

The native pipeline is `polygon-core::MeshSectionIndex` → `slicer-core` →
`gcode-optimize` → `gcode-core`. `geometry-bridge` exposes `mesh_toolpaths`,
`mesh_gcode` (preview dialect), `mesh_gcode_job` (job + `.gcode.3mf`), and
`gcode_preview`; their typed host functions live in
`src/services/geometry/polygon.ts`. `mesh_gcode` independently parses its output
before returning the text, layer count, dialect and movement statistics.
Explicit invalid options are errors; only omitted options use defaults.

```ts
import { emitPolygonMeshGcode, parseGcodePreview } from './src/services/geometry/polygon'

// mesh contains flat XYZ positions and triangle indices, already in model space.
const result = emitPolygonMeshGcode(mesh, 0, 10, {
  layerHeightMm: 0.2,
  lineWidthMm: 0.4,
  wallCount: 2,
  infillSpacingMm: 2,
  feedrateMmS: 50,
  travelFeedrateMmS: 120,
  filamentDiameterMm: 1.75,
})
const preview = parseGcodePreview(result.gcode)
// result.preview is the same validated movement/statistics result.
```

These direct host calls are synchronous. Browser UI callers use
`createGcodePreviewWorker()` from `src/services/gcodePreviewWorker.ts`, with
`run`, `cancel` and `dispose`, as demonstrated by `GcodePanel.vue`.

The supported dialect is `open-scad-viewer/print-preview 2`. Coordinates are
millimeters; XYZ and filament E are absolute. Feedrate is mm/s in the host API
and mm/min in the file. The header explicitly establishes the coordinate,
extrusion and unit modes. The parser rejects unsupported commands and modes,
malformed words, invalid numbers and invalid layer ordering. It is not a
general parser for arbitrary printer/slicer files.

Version 1 files from the earlier prototype did not declare the filament
diameter or establish all required modes. They are explicitly rejected; regenerate
the preview from the source model. The parser never guesses their extrusion volume.

The layer schedule samples `zMin + i * layerHeightMm < zMax` in model space.
It starts at `zMin`, uses indexed arithmetic and checks the count even when all
sections are empty. The range is a geometric preview range, not a build-plate
placement or first-layer process setting. A range with no generated paths
cannot be exported.

Material volume uses rectangular bead area `lineWidthMm * layerHeightMm`;
filament length uses the circular filament area. Preview statistics come from
the rounded G-code, including its filament diameter metadata. Time is a
constant-speed estimate starting at the first known XYZ point; it excludes
initial machine positioning, acceleration, heating, cooling and firmware timing.

## Bounds and supported geometry

The browser admits up to 100,000 triangles and 16 MiB of scene mesh data.
The native adapter accepts up to 100,000 mesh triangles and limits cumulative section
triangle visits to 10,000,000. The planner additionally bounds section geometry,
total plan geometry, scan lines, geometric work and the layer count. Both file
input and output are bounded; see the current constants and numeric range in
[`gcode-core`](../../crates/gcode-core/README.md) and
[`slicer-core`](../../crates/slicer-core/README.md).

The geometry path supports horizontal sections, inset walls and sparse linear
hatch infill with holes. It does not implement top/bottom solid skins, supports,
bridges, variable-width extrusion, retractions, z-hop or printer-specific start
and end programs. It does not establish printer homing, heating, build-volume
limits or a qualified material/process profile. Files are movement previews,
not ready-to-run physical print jobs. No printer connection or job submission
is performed.

## Verification

Native regression tests cover invalid coordinates and settings, overflow,
rounding, parser mode and token errors, resource limits, layer schedules,
collapsed thin regions, holes and extrusion-volume agreement. Bridge tests
exercise the same public operations used by WASM. Host tests read exported
files back through the actual WASM boundary; worker and Vue tests exercise
cancellation, stale results and download behavior.

Run `npm run test:gcode` for the focused suite, and `npm run typecheck` plus
`npm run build` for type and production distribution checks.

Command semantics follow the primary firmware references:
[Marlin linear moves](https://marlinfw.org/docs/gcode/G000-G001.html) and
[Marlin volumetric extrusion mode](https://marlinfw.org/docs/gcode/M200.html).
Supporting these commands in a preview is not firmware or machine qualification.

# G-code export and preview

The CAD workbench generates a bounded, deterministic preview from one scene
body. It uses the displayed body's placement, exposes the layer, line width,
wall count, infill spacing, print/travel speeds and filament diameter, and
displays the exported file's movements one layer at a time. Travel and
deposition have separate colors. An existing file can also be opened: the
app's own preview/job dialects are validated strictly, while files from other
slicers (PrusaSlicer, Orca/Bambu Studio, Cura, SuperSlicer, Klipper or
RepRapFirmware targets, …) are read by a tolerant preview parser that reports
the detected generator and firmware flavor.

## Execution and file contract

Slicing and parsing run in a dedicated browser worker. Cancel, a changed input,
a changed model or a closed panel invalidates the result and terminates the
worker. A completed result is downloadable only while its inputs remain current.
Errors return without publishing a partial file.

The native pipeline is `polygon-core::MeshSectionIndex` → `slicer-core` →
`gcode-optimize` → `gcode-core`. `geometry-bridge` exposes `mesh_toolpaths`,
`mesh_gcode` (preview dialect), `mesh_gcode_job` (job + `.gcode.3mf`, with an
optional `flavor`), `gcode_preview` (flat movement result) and `gcode_parse`
(movements plus detected dialect/generator/flavor); their typed host functions
live in `src/services/geometry/polygon.ts`. `mesh_gcode` independently parses its output
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

## Dialects and firmware flavors

Three parsing modes exist:

| Input | Parser | Behaviour |
|---|---|---|
| `; open-scad-viewer/print-preview 2` | strict preview | exact prologue, no heat/retract |
| `; open-scad-viewer/print-job 1` + `;FLAVOR:` | strict job | flavor-specific heat/prologue/shutdown |
| anything else | tolerant foreign reader | unknown commands skipped, statistics approximate |

Print jobs target one of three firmware flavors selected in the panel
(`JobSettingsInput.flavor`): `marlin` (default; Creality, Prusa Buddy, Ender,
most Marlin forks), `klipper` (native `SET_HEATER_TEMPERATURE`,
`TEMPERATURE_WAIT`, `SET_PRINT_STATS_INFO`, `TURN_OFF_HEATERS`; no `M200`), and
`reprapfirmware` (Duet: `G10 P0 S R`, `T0`, `M116`). The exported result and
the file header report the flavor; the strict job parser requires the body to
match the header. See the command tables in
[`gcode-core`](../../crates/gcode-core/README.md).

The tolerant reader understands `G0`–`G3` (XY arcs chorded), `G90`/`G91`,
`M82`/`M83`, `G92`, `G20`/`G21`, line numbers/checksums, compact words and the
layer markers of the common slicers; without markers, each Z level with
extrusion counts as a layer. Volume uses the filament diameter declared in the
file or 1.75 mm. The UI labels such results as foreign files and offers the
original bytes for download; it does not convert between flavors.

The strict preview dialect is `open-scad-viewer/print-preview 2`. Coordinates are
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
collapsed thin regions, holes and extrusion-volume agreement, per-flavor job
round trips and cross-flavor rejection, and tolerant parsing of PrusaSlicer,
Cura, relative/inch, arc and checksummed inputs. Bridge tests
exercise the same public operations used by WASM. Host tests read exported
files back through the actual WASM boundary; worker and Vue tests exercise
cancellation, stale results and download behavior.

Run `npm run test:gcode` for the focused suite, and `npm run typecheck` plus
`npm run build` for type and production distribution checks.

Command semantics follow the primary firmware references:
[Marlin linear moves](https://marlinfw.org/docs/gcode/G000-G001.html) and
[Marlin volumetric extrusion mode](https://marlinfw.org/docs/gcode/M200.html).
Supporting these commands in a preview is not firmware or machine qualification.

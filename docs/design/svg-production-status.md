# SVG production status

The requested workflow is viewing artwork, importing it into a 3D model, and
exporting SVG that can be imported again. The implementation supports that
cycle for bounded static SVG. This document does not declare universal SVG 2
conformance or replace the application's release qualification.

## Implementation and verification scope

The current code and integrated tests cover the following behavior, including
CSS geometry and non-scaling strokes. The component evidence records an exact
source and WASM revision; subsequent edits require another qualification run.

| Requirement | Implementation and verification |
| --- | --- |
| View SVG artwork | Normalization retains CSS styling, references, nested viewports, gradients, clipping, masks, filters and embedded images. Native tests cover `use` and `symbol`, supplied fonts and malformed resources. |
| CSS geometry | `x`, `y`, `width`, `height`, `cx`, `cy`, `r`, `rx`, `ry` and `d: path(...)` participate in the supported selector/inline cascade, `!important` and explicit inheritance, including separate `use` instances. Tests cover physical and font units, percentages, root sizing and nested symbol viewports. |
| Non-scaling strokes | `vector-effect="non-scaling-stroke"` preserves intrinsic physical stroke width through affine transforms and nested viewports. Tests cover dashes, caps, joins, holes, text, markers, clipping, paint order and instance-dependent pattern/mask coordinates. Normalized artwork freezes the stroke into filled outlines at the document's intrinsic size. |
| Import into 3D | Physical dimensions, compound holes, stroked paths and curve tolerance reach full solid builds. The panel appends editable-height SCAD to an OpenSCAD document and requests a full build. Complex rendering effects have an explicit raster silhouette mode. A real-worker test combines CSS-defined geometry and non-scaling strokes with extrusion and projection reimport. |
| Export and reimport | Artwork and fabrication contours are separate exports. Model projections and planar faces produce millimeter SVG; holes survive reimport. Projection does not overwrite the current artwork. |
| Editable model output | MCP can additionally produce a validated ModelGraph document with an editable height parameter and independently checked bounds, volume and topology. |
| Responsive editing | A dedicated worker has cancellation, a hard deadline and recovery. Source, settings, scene and destination changes invalidate stale results. |
| Draft recovery | Source, unfinished numeric settings and exact font bytes persist through IndexedDB, with bounded storage failure and cross-tab conflict handling. |
| Resource admission | Source, fonts, decoded images, reference/marker expansion, CSS selector matching, non-scaling paint-resource expansion, geometry, raster work, generated code and transport have explicit limits. Rejected input does not produce a partial model. |

Run `node scripts/record-svg-evidence.mjs` to reproduce the component evidence.
The successful run at 2026-09-13T11:10:01Z executed 72 native SVG tests, 169 native
planar geometry tests and 122 application/worker/MCP/vector workflow tests
(363 checks). The
generated record in `docs/qualification/svg-static-cycle-v1.json` binds the
source and test inputs, build configuration, geometry WASM, packed geometry
module and Brotli decoder. It refuses to record success if they change during
the run. Re-run after later source changes; the record describes its recorded
revision, not arbitrary subsequent worktree edits. This run used geometry WASM
SHA-256 `375f878b7998b13c4cb2b76286f1116ea4565a1f3cfe6c4f1a62d547cf61a65f`.

An earlier production browser build was also exercised through the actual controls:
a 40 × 30 mm profile with a 20 × 10 mm hole, extruded by 5 mm, produced one
body with volume 5000 mm³ and surface area 3000 mm². Exporting the Z projection
and importing it into an empty model reproduced those values, with integer
coordinates and no additional coordinate transform. Browser checks also
covered cancellation, draft restoration and a 390 × 844 viewport. These are
manual browser observations, separate from the automated qualification record.

The CSS/non-scaling browser check used a 100 × 100 mm viewport supplied by CSS
overriding 4 mm XML attributes, `d: path(...)`, a 2 mm non-scaling stroke and a
nonuniform transform. Extrusion by 3 mm produced one body, 12 triangles,
180 mm³ volume and 312 mm² surface area. Z projection exported a 30 × 2 mm
profile; loading that SVG and extruding into an empty document reproduced both
measurements. The browser console reported no errors. The same cycle was repeated on the
final distribution, including draft restoration and a fresh full model build.
The final integration record is [svg-application-check-v1.json](../qualification/svg-application-check-v1.json).

The packed kernel's per-artifact budget is 2.35 MB; the complete distribution
retains its 4.70 MB budget. CSS geometry and instance-dependent non-scaling
strokes increased the kernel to approximately 2.32 MB. Two isolated size-profile
experiments did not justify changing the compiler profile: `usvg`/`resvg` at
`opt-level=z` saved only 4704 bytes of base64 JS and slowed text, while applying
that profile to image/font decoders increased size by 5060 bytes. Both trials
preserved 141 compared SVG ABI responses, but the original profile was retained.

## Remaining boundaries

Scripts, event handlers, animation, external resources/stylesheets, CSS font
loading, DTDs and `foreignObject` are rejected. The static CSS profile does not
provide arbitrary browser CSS: math/variables, conditional or layered rules,
viewport-relative and unsupported font-relative units, min/max sizing and
unsupported geometry selectors require resolved values. Vector effects other
than `none` and `non-scaling-stroke` are rejected. Ordinary outline TTF/OTF/TTC
fonts are supported; SVG/COLR/bitmap glyph tables are rejected. Matching text
metrics requires the intended supplied outline font. Embedded GIF/WebP images
are normalized to the first static frame with a diagnostic.

Masks, filters, patterns, transparent gradient stops and embedded images require
explicit rendered-silhouette conversion for geometry. It thresholds a bitmap
of 128–2048 pixels on the longest edge and clips it to the SVG viewport; this
is an approximation. Colors do not create different extrusion heights.
Standalone SVG defaults to 96 DPI, while project `import()` retains its 72 DPI
compatibility convention; explicit physical units preserve scale.

Panel extrusion requires an OpenSCAD document. An editable native ModelGraph
result is available through the MCP option, with its tighter profile/node
limits. Projection uses all model bodies, including hidden ones, and ignores
the section view. Selected-face export requires a planar face. Export crops
to contour bounds, preserving size and holes on reimport rather than the
original world-space origin; curved-surface unwrapping is not implemented.

The panel/MCP allow 4 MiB SVG input/output and 20000 contour points; the shared
import engine allows 500000 points. Projection admits 20000 source triangles,
counting only the selected face for face export. SCAD output must fit the
250000-character workspace budget. Font, decoded-image, expansion, raster and
transport limits also apply; conservative expansion estimates can count unused
definitions. See the [README SVG section](../../README.md#svg-creation-and-conversion) for details.

## Application release gates

The final coordinated run passed 2642 application tests, with no
failures. Five optional official OpenSCAD runtime checks were skipped because
the external runtime is not installed. Vue and MCP typechecks passed, and the
production distribution passed all artifact/decoded-WASM checks: 68 files,
4699211 bytes, within the unchanged 4700000-byte total budget. The final
source and raw/packed/decoder fingerprints matched the recorded SVG evidence
after the entire run.

G0 v3 and G1 v20 bind the coordinated sources while preserving all earlier
artifacts. The shared-executor export after v19 required a separate v20; v19
was retained unchanged. These ordinary regression checks do not execute the
separate G1 qualification program: G0 remains open, qualificationClaim remains
none, and all 4740 required clean work units/u07 remain outstanding.

The active own-Rust CAD oracle v2 passed its binary and independent semantic
regressions. Its [reviewed historical replay](../qualification/own-rust-cad-oracle-v2-review.md)
explains the intentional primitive/normalization changes; v1 is retained
byte-for-byte. Historical Manifold constants remain archival. This record
qualifies the stated SVG workflow at its exact source snapshot and does not
claim a separate semantic-engine production cutover.

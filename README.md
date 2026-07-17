# OpenSCAD Viewer

A client-side OpenSCAD workspace built with Vue, WebGPU, and the
[Manifold](https://github.com/elalish/manifold) geometry kernel. It is designed
as a fast, independent subset viewer: supported operations produce
real geometry, while unsupported OpenSCAD syntax returns a line/column error
instead of a misleading preview.

## Highlights

- Real manifold `union()`, `difference()`, `intersection()`, and `hull()`.
- Variables, expressions, ranges, `for`, `if`, `let`, user modules, and
  `children()`.
- 3D primitives, 2D shapes, `linear_extrude()`, `rotate_extrude()`,
  `projection()`, transforms, colors, and polyhedra.
- Cancellable preview → full geometry compilation in a dedicated Worker with
  stale-result protection, source/AST/tessellation budgets, and transferable
  mesh, semantic-edge, provenance, and BVH buffers.
- Z-up WebGPU viewport with BVH-accelerated preselection/picking, point/face/
  body selection modes, focus/isolate/hide, shaded/semantic-edge/x-ray display,
  an interactive view cube, fit/reset, standard views, grid control,
  perspective/orthographic projection, and event-driven redraw.
- Plasticity-inspired searchable command palette, Scene Outliner, Inspect
  workspace, source ↔ CSG cross-highlighting, two-point measurements, virtual
  section analysis, and viewport-scoped shortcuts.
- Lightweight OpenSCAD Customizer controls for top-level literal variables,
  including `// [min:step:max]` sliders and choice lists.
- Binary STL and OBJ export with object transforms baked into the result.
- Open/save/drag-and-drop `.scad` files, shareable source links, local draft
  persistence, RU/EN UI, light/dark themes, and a resizable workspace.
- Strict diagnostics and tests for booleans, transforms, modules, loops,
  extrusion, projection math, and all bundled examples.

## Run locally

Requirements: Node.js 20.19+ and a browser with WebGPU.

```bash
npm ci
npm run dev
```

Quality gate:

```bash
npm run check
```

Production preview:

```bash
npm run build
npm run preview
```

## Controls

- Left drag: orbit.
- Right drag or Shift + left drag: pan.
- Mouse wheel: zoom.
- Hover: preselect the nearest surface through the mesh BVH.
- Click without dragging: select the exact surface and its source operation.
- `1`, `3`, `4`: point, face, and body selection modes.
- `Shift + M`: cycle selection modes (`Tab` keeps normal keyboard focus traversal).
- `F` or `/`: focus the selection, or fit the model when nothing is selected.
- `.`: isolate/unisolate the selected object.
- `H`: hide the selected object; use the Outliner to show it again.
- `Esc`: clear selection.
- `E`, `X`, `G`: toggle mesh edges, x-ray, and grid.
- `5`: toggle perspective/orthographic projection.
- Numpad `0`, `1`, `3`, `7`: isometric, front, right, and top views.
- `Ctrl/⌘ + =`: start/cancel a two-point measurement.
- `Shift + F`: flip an active section plane.
- `Ctrl/⌘ + K`: open the command palette.
- `Ctrl/⌘ + Shift + B`: toggle the Scene/Inspect/Parameters dock.
- `Ctrl/⌘ + Enter`: render.

## Supported subset and limits

This project does **not** bundle the official OpenSCAD compiler. The official
WASM runtime is the best route to full OpenSCAD compatibility, but bundling it
introduces GPL-2.0+ licensing requirements. This viewer instead uses the
Apache-2.0 `manifold-3d` package and implements a strict language subset.

Currently unsupported features include `include`/`use`, user functions,
`import()`, `surface()`, `text()`, Minkowski operations, and advanced OpenSCAD
Customizer annotations. These fail explicitly. Complexity is bounded to protect
the browser: source length, AST size, recursion, range size, object count,
`$fn`, and final triangle count all have limits.

## Architecture

- `src/services/openscadParser.ts`: lexer, expression/statement parser,
  evaluator, Manifold geometry conversion, diagnostics, and budgets.
- `src/workers/geometry.worker.ts`: asynchronous compilation and transferable
  mesh response protocol.
- `src/services/webgpuRenderer.ts`: WebGPU resource lifecycle, lighting,
  camera, BVH picking/preselection, measurement and section overlays, grid,
  input, resize, and event-driven rendering.
- `src/services/meshBvh.ts`: compact transferable triangle BVH and raycast.
- `src/services/meshTopology.ts`: boundary/crease/non-manifold edge extraction.
- `src/services/meshInspection.ts`: provenance, bounds, hit, and measurement
  helpers.
- `src/components/`: command palette, ViewCube, Scene Outliner, Inspect, and
  Customizer panels.
- `src/App.vue`: workspace UI, file actions, settings, stale/error state, and
  viewer controls.
- `tests/`: mathematical and geometry golden tests.

The implementation priorities came from two reproducible, non-overlapping
comparisons covering 200 OpenSCAD, CAD, mesh, and 3D-viewer repositories. See
[the benchmark](docs/research/top-100-repositories.md).
The follow-up UI pass is documented in
[Plasticity interaction patterns](docs/research/plasticity-patterns.md).
The second independent benchmark adds
[100 non-overlapping repositories](docs/research/top-100-repositories-second.md),
while the [geometry literature review](docs/research/geometry-pipeline-literature.md),
[CAD/HCI review](docs/research/cad-hci-literature.md), and
[prioritized roadmap](docs/research/second-pass-recommendations.md) document the
next evidence-backed implementation candidates.

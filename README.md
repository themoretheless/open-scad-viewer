# OpenSCAD Viewer

A browser-based **OpenSCAD editor and 3D viewer** powered by **WebGPU**. Write a subset of OpenSCAD in the editor and see it rendered in real time — no server, no native OpenSCAD install required.

> Vue 3 · Vite · TypeScript · WebGPU · zero runtime dependencies (only Vue)

---

## Features

The app has grown to ~250 features. Highlights:

### Editor
- Syntax highlighting, line numbers, minimap, code folding
- Autocomplete, hover docs, bracket matching, occurrence highlighting
- Find & replace, go-to-line, command palette (`Ctrl+Shift+P`), quick switcher (`Ctrl+P`)
- Multi-tab editing with pinning, colors, drag-reorder, per-tab version history
- Auto-close brackets, auto-indent, smart Home, comment toggle, sort/join/case commands
- Custom + native undo/redo, snippets, customizable keyboard shortcuts (VS Code / Sublime / Emacs presets)
- 6 editor themes + custom theme editor, adjustable UI density

### 3D Viewport (WebGPU)
- Orbit / fly camera, view presets, orientation gizmo, axis labels, compass
- Render modes: solid, solid+edges, wireframe, x-ray, hidden-line
- Shading: smooth/flat, toon/cel, Gooch, SSAO, normal smoothing
- Lighting presets, skybox gradients, color grading, vignette, bloom, MSAA 4×
- Clipping plane (multi-axis), section box, exploded view, ground shadow, reflection
- Auto-rotate, turntable export, FOV slider, orbit inertia, measurement tool, snapshots

### OpenSCAD Language Support
A custom parser/evaluator supporting **80+ primitives and operations**:
- **Primitives:** `cube`, `sphere`, `cylinder`, `polyhedron`, `polygon`, `circle`, `square`, `text`, `surface`
- **Extended primitives:** `torus`, `helix`, `spring`, `bezier`, `sweep`, `star`, `thread`, `pipe`, `tube`, `ring`, `wedge`, `gear`, `capsule`, `prism`, `cone`, `pyramid`, `trapezoid`, `rounded_cube`, `chamfer_cube`, `arrow`, `donut`, `ogive`, `teardrop`, `ellipsoid`, `hemisphere`, `slot`, `cross`, `honeycomb`, `knurl`, `lattice`, `maze`, `fibonacci_sphere`
- **Transforms:** `translate`, `rotate`, `scale`, `mirror`, `resize`, `multmatrix`, `color`, `mirror_copy`, `radial_array`, `linear_array`, `distribute`, `grid`
- **CSG:** `union`, `difference`, `intersection`, `hull`, `minkowski`
- **Extrusion:** `linear_extrude`, `rotate_extrude`, `spiral_extrude`, `loft`, `offset`, `projection`
- **Language:** `module`, `function`, `for`, `let`, `if`/`else`, list comprehensions, `each`, `assert`, `echo`, `children`, `use`/`include`, full expression evaluator with 20+ math functions

### Export / Import
- Export: **STL** (binary), **OBJ**, **3MF**, **PNG** (1×/2×/4×), turntable ZIP, all-tabs ZIP
- Import: binary **STL**
- Share via URL hash, copy image to clipboard, spec sheet, weight & cost estimator

### Internationalization
- 4 languages: Russian, English, German, Chinese (`ru` / `en` / `de` / `zh`)

---

## Getting Started

### Prerequisites
- Node.js 18+
- A **WebGPU-capable browser**: Chrome 113+, Edge 113+, or Firefox Nightly

### Install & Run

```bash
npm install
npm run dev      # start dev server at http://localhost:5173
```

### Build

```bash
npm run build    # type-check (vue-tsc) + production build to dist/
npm run preview  # preview the production build
```

---

## Usage

1. Write OpenSCAD code in the left editor panel.
2. The model renders automatically (debounced) — or press `Ctrl+Enter`.
3. Orbit with the left mouse button, pan with right/shift-drag, zoom with the wheel.
4. Pick an example from the gallery to get started.

### Example

```openscad
difference() {
    cube([30, 30, 30], center = true);
    sphere(r = 19, $fn = 32);
}

translate([50, 0, 0])
    gear(teeth = 16, mod = 2, thickness = 6);
```

### Keyboard Shortcuts (selection)

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Render |
| `Ctrl+Shift+P` / `F1` | Command palette |
| `Ctrl+P` | Quick tab switcher |
| `Ctrl+F` / `Ctrl+H` | Find / Find & Replace |
| `Ctrl+/` | Toggle comment |
| `Ctrl+G` | Go to line |
| `Numpad 1-9` | Camera views (Blender-style) |
| `?` | Keyboard shortcuts help |

---

## Project Structure

```
src/
├── App.vue                      # main application component (being decomposed)
├── main.ts                      # entry point
├── i18n/                        # locale dictionaries (ru/en/de/zh), t(), lang state
├── config/                      # editor themes, viewer presets, shortcut presets
├── parser/
│   ├── geometry.ts              # 41 pure mesh generators, earClip, extrude, hull
│   └── limits.ts                # shared tessellation bounds ($fn cap)
├── renderer/
│   └── shaders.ts               # WGSL sources; Scene struct single-sourced
└── services/
    ├── openscadParser.ts        # tokenizer, parser, evaluator (public API)
    ├── webgpuRenderer.ts        # WebGPU pipelines, camera, render passes
    ├── math3d.ts                # mat4 / vec3 math
    ├── safeStorage.ts           # validated + quota-safe localStorage access
    ├── stlExport.ts / stlImport.ts
    ├── objExport.ts
    ├── threemfExport.ts
    └── zipExport.ts
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for a deeper dive and the module map, [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) for the phased roadmap with checklists, [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) for the severity-ranked shortlist, [recommendation.md](./recommendation.md) for the live 500-item improvement catalog (v2), and [ISSUES.md](./ISSUES.md) for the older full defect snapshot.

---

## Browser Support

WebGPU is required. If unsupported, the app shows a notice. Status by engine:
- **Chromium (Chrome / Edge) 113+** — full support
- **Firefox** — Nightly with `dom.webgpu.enabled`
- **Safari** — Technology Preview / recent versions

---

## License

MIT
